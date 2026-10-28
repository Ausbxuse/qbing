package lab.cube.android

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.*
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.BroadcastReceiver
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import lab.cube.core.*
import java.util.UUID

@SuppressLint("MissingPermission")
class AndroidBle(private val context:Context,private val permissionRequest:suspend()->Boolean):BleTransport {
    override var onBytes:(ByteArray)->Unit={}
    override var onDisconnected:(String)->Unit={}
    override var onAdapter:(Boolean,String)->Unit={_,_->}
        set(value) { field=value; runCatching{adapter?.isEnabled}.getOrNull()?.let{value(it,if(it)"Bluetooth ready" else "Bluetooth is off")} }
    private val adapter get()=(context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager).adapter
    private var gatt:BluetoothGatt?=null
    private var command:BluetoothGattCharacteristic?=null
    private var ready:CompletableDeferred<Unit>?=null
    private var writeResult:CompletableDeferred<Unit>?=null
    private val writeMutex=Mutex()
    private var intentionalDisconnect=false
    private val powerReceiver=object:BroadcastReceiver(){
        override fun onReceive(context:Context,intent:Intent){
            val state=intent.getIntExtra(BluetoothAdapter.EXTRA_STATE,BluetoothAdapter.ERROR)
            if(state==BluetoothAdapter.STATE_OFF||state==BluetoothAdapter.STATE_TURNING_OFF){
                ready?.completeExceptionally(IllegalStateException("Bluetooth is off"))
                writeResult?.completeExceptionally(IllegalStateException("Bluetooth is off"))
                gatt?.close();gatt=null;command=null
                onAdapter(false,"Bluetooth is off")
            } else if(state==BluetoothAdapter.STATE_ON)onAdapter(true,"Bluetooth ready")
        }
    }
    init {
        if(Build.VERSION.SDK_INT>=33)context.registerReceiver(powerReceiver,IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED),Context.RECEIVER_EXPORTED)
        else context.registerReceiver(powerReceiver,IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED))
    }
    override fun close(){runCatching{context.unregisterReceiver(powerReceiver)}}
    private suspend fun permission(){check(permissionRequest()){ "Bluetooth permission is required to connect your cube." };check(adapter?.isEnabled==true){"Turn on Bluetooth in Android settings."}}
    override suspend fun scan():List<CubeDevice> {
        permission();val found=java.util.concurrent.ConcurrentHashMap<String,CubeDevice>();val scanner=adapter.bluetoothLeScanner?:error("Bluetooth scanning is unavailable.");var failure:String?=null
        val callback=object:ScanCallback(){
            override fun onScanResult(callbackType:Int,result:ScanResult){val name=result.scanRecord?.deviceName?:result.device.name?:"";if(name.startsWith("GAN")||name.startsWith("MG")||name.startsWith("AiCube"))found[result.device.address]=CubeDevice(result.device.address,name)}
            override fun onScanFailed(errorCode:Int){failure="Bluetooth scan failed ($errorCode). Try again in a moment."}
        }
        try{scanner.startScan(null,ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build(),callback);delay(4500);failure?.let{error(it)}}finally{scanner.stopScan(callback)}
        return found.values.sortedBy{it.name}
    }
    override suspend fun connect(device:CubeDevice){
        permission();disconnect();intentionalDisconnect=false;val completion=CompletableDeferred<Unit>();ready=completion
        val remote=adapter.getRemoteDevice(device.address)
        gatt=remote.connectGatt(context,false,callback,BluetoothDevice.TRANSPORT_LE)
        try{withTimeout(15000){completion.await()}}catch(e:Exception){disconnect();throw e}
    }
    private val callback=object:BluetoothGattCallback(){
        override fun onConnectionStateChange(connection:BluetoothGatt,status:Int,newState:Int){
            if(connection!==gatt)return
            if(status==BluetoothGatt.GATT_SUCCESS&&newState==BluetoothProfile.STATE_CONNECTED){connection.requestConnectionPriority(BluetoothGatt.CONNECTION_PRIORITY_HIGH);if(!connection.discoverServices())ready?.completeExceptionally(IllegalStateException("Could not discover cube services."))}
            else if(newState==BluetoothProfile.STATE_DISCONNECTED||status!=BluetoothGatt.GATT_SUCCESS){ready?.completeExceptionally(IllegalStateException("Bluetooth connection ended ($status)."));writeResult?.completeExceptionally(IllegalStateException("Cube disconnected."));command=null;connection.close();if(!intentionalDisconnect)onDisconnected("Cube disconnected. Reconnect before the next attempt.")}
        }
        override fun onServicesDiscovered(connection:BluetoothGatt,status:Int){
            if(connection!==gatt)return
            if(status!=BluetoothGatt.GATT_SUCCESS){ready?.completeExceptionally(IllegalStateException("Service discovery failed ($status)."));return}
            val service=connection.getService(UUID.fromString(GanUuids.service))
            val notify=service?.getCharacteristic(UUID.fromString(GanUuids.notify))
            command=service?.getCharacteristic(UUID.fromString(GanUuids.command))
            if(notify==null||command==null){ready?.completeExceptionally(IllegalStateException("This device does not expose GAN Gen4 services."));return}
            if(!connection.setCharacteristicNotification(notify,true)){ready?.completeExceptionally(IllegalStateException("Could not enable cube notifications."));return}
            val descriptor=notify.getDescriptor(UUID.fromString(GanUuids.cccd))
            if(descriptor==null){ready?.completeExceptionally(IllegalStateException("Cube notification descriptor is missing."));return}
            val value=if(notify.properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY!=0)BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE else BluetoothGattDescriptor.ENABLE_INDICATION_VALUE
            val accepted=if(Build.VERSION.SDK_INT>=33)connection.writeDescriptor(descriptor,value)==BluetoothStatusCodes.SUCCESS else {descriptor.value=value;connection.writeDescriptor(descriptor)}
            if(!accepted)ready?.completeExceptionally(IllegalStateException("Could not subscribe to cube notifications."))
        }
        override fun onDescriptorWrite(connection:BluetoothGatt,descriptor:BluetoothGattDescriptor,status:Int){if(connection!==gatt)return;if(descriptor.uuid==UUID.fromString(GanUuids.cccd)){if(status==BluetoothGatt.GATT_SUCCESS)ready?.complete(Unit) else ready?.completeExceptionally(IllegalStateException("Notification subscription failed ($status)."))}}
        override fun onCharacteristicChanged(connection:BluetoothGatt,characteristic:BluetoothGattCharacteristic,value:ByteArray){if(connection===gatt&&characteristic.uuid==UUID.fromString(GanUuids.notify))onBytes(value.copyOf())}
        @Deprecated("Legacy callback for Android 12 and earlier")
        override fun onCharacteristicChanged(connection:BluetoothGatt,characteristic:BluetoothGattCharacteristic){if(connection===gatt&&Build.VERSION.SDK_INT<33&&characteristic.uuid==UUID.fromString(GanUuids.notify))onBytes(characteristic.value.copyOf())}
        override fun onCharacteristicWrite(connection:BluetoothGatt,characteristic:BluetoothGattCharacteristic,status:Int){if(connection!==gatt)return;if(status==BluetoothGatt.GATT_SUCCESS)writeResult?.complete(Unit) else writeResult?.completeExceptionally(IllegalStateException("Cube command failed ($status)."))}
    }
    override suspend fun write(bytes:ByteArray)=writeMutex.withLock {
        val connection=gatt?:error("Connect the cube first.");val characteristic=command?:error("Cube services are not ready.");val result=CompletableDeferred<Unit>();writeResult=result
        val accepted=if(Build.VERSION.SDK_INT>=33)connection.writeCharacteristic(characteristic,bytes,BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT)==BluetoothStatusCodes.SUCCESS else {characteristic.writeType=BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT;characteristic.value=bytes;connection.writeCharacteristic(characteristic)}
        check(accepted){"Bluetooth is busy. Reconnect the cube."}
        try{withTimeout(5000){result.await()}}finally{writeResult=null}
    }
    override suspend fun disconnect(){intentionalDisconnect=true;gatt?.disconnect();gatt?.close();gatt=null;command=null;ready?.cancel();ready=null;writeResult?.cancel();writeResult=null}
}
