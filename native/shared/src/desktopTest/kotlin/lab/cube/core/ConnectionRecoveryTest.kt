package lab.cube.core

import kotlinx.coroutines.*
import kotlin.test.*

class ConnectionRecoveryTest {
    @Test fun deletingASolveRemovesItsPendingUploadAndPersistsTheRemainingHistory(){
        val scope=CoroutineScope(SupervisorJob()+Dispatchers.Unconfined)
        try {
            val memory=Memory();val c=LabController(Transport(),memory,scope)
            val a=AttemptRecord("delete-a","2026-10-27T00:00:00Z","solve",source="manual",duration=12000.0,initialState=Cube.solved)
            val b=a.copy(id="delete-b",duration=14000.0)
            c.save(a);c.save(b);c.review=a
            c.deleteRecord(a)
            assertEquals(listOf(b.id),c.records.map{it.id});assertNull(c.review)
            assertEquals(listOf(b.id),WireJson.decodeFromString<List<AttemptRecord>>(memory.read("pending")!!).map{it.id})
            assertEquals(listOf(b.id),WireJson.decodeFromString<Bootstrap>(memory.read("cache")!!).records.map{it.id})
            c.deleteRecord(b)
            assertTrue(c.records.isEmpty());assertEquals(0,c.queueCount)
        } finally {scope.cancel()}
    }

    @Test fun stateRetriesAreBoundedAndSuccessCancelsTheDeadline(){
        val schedule=StateRequestSchedule();schedule.begin(100.0)
        assertEquals(StateRequestSchedule.Action.NONE,schedule.poll(1099.0))
        assertEquals(StateRequestSchedule.Action.RETRY,schedule.poll(1100.0))
        assertEquals(StateRequestSchedule.Action.NONE,schedule.poll(1101.0))
        assertEquals(StateRequestSchedule.Action.RETRY,schedule.poll(3100.0))
        schedule.begin(4000.0) // More malformed replies must not extend the original deadline.
        assertEquals(StateRequestSchedule.Action.TIMEOUT,schedule.poll(6100.0))
        assertEquals(StateRequestSchedule.Action.NONE,schedule.poll(9000.0))
        schedule.begin(10000.0);schedule.clear()
        assertEquals(StateRequestSchedule.Action.NONE,schedule.poll(20000.0))
    }
    private class Memory:LocalStorage {
        private val values=mutableMapOf<String,String>()
        override fun read(key:String)=values[key]
        override fun write(key:String,value:String){values[key]=value}
    }
    private class Transport:BleTransport {
        override var onBytes:(ByteArray)->Unit={}
        override var onDisconnected:(String)->Unit={}
        override var onAdapter:(Boolean,String)->Unit={_,_->}
        var blocked:CompletableDeferred<Unit>?=null
        var writes=mutableListOf<ByteArray>()
        var disconnects=0
        override suspend fun scan()=emptyList<CubeDevice>()
        override suspend fun connect(device:CubeDevice){blocked?.await()}
        override suspend fun write(bytes:ByteArray){writes.add(bytes)}
        override suspend fun disconnect(){disconnects++}
    }
    private fun solved(serial:Int=0):ByteArray=ByteArray(20).apply {
        this[0]=0xed.toByte();this[1]=14;this[2]=serial.toByte()
        fun put(offset:Int,width:Int,value:Int){for(i in 0 until width){val pos=offset+i;if(value and (1 shl(width-1-i))!=0)this[pos/8]=(this[pos/8].toInt()or(1 shl(7-pos%8))).toByte()}}
        for(i in 0..6)put(32+3*i,3,i);for(i in 0..10)put(69+4*i,4,i)
    }
    private val mac="12:34:56:78:9A:BC"
    @Test fun disconnectedClientIgnoresLatePacketsAndFailedHandshakeReleasesTheLink(){
        val scope=CoroutineScope(SupervisorJob()+Dispatchers.Unconfined)
        try {
            val transport=Transport();val controller=LabController(transport,Memory(),scope)
            controller.connect(CubeDevice(mac,"Test cube"));assertTrue(controller.connected)
            transport.onBytes(GanCipher(mac).encrypt(solved()))
            assertEquals(Cube.solved,controller.cubeState)
            controller.requestState();controller.pollStateRequest(monotonicMs()+7000)
            assertTrue(controller.connected,"An optional state read must not create a failed initial-state deadline")
            controller.disconnect();assertFalse(controller.connected)
            transport.onBytes(GanCipher(mac).encrypt(solved(1)))
            assertNull(controller.cubeState)
            controller.connect(CubeDevice(mac,"Test cube"));assertTrue(controller.connected)
            controller.pollStateRequest(monotonicMs()+7000)
            assertFalse(controller.connected);assertFalse(controller.connectionBusy)
            assertTrue(transport.disconnects>=2)
        } finally {scope.cancel()}
    }
    @Test fun adapterLossCancelsPendingConnectAndMalformedPacketRequestsFreshState(){
        val scope=CoroutineScope(SupervisorJob()+Dispatchers.Unconfined)
        try {
            val transport=Transport();val controller=LabController(transport,Memory(),scope)
            transport.blocked=CompletableDeferred()
            controller.connect(CubeDevice(mac,"Test cube"));assertTrue(controller.connectionBusy)
            transport.onAdapter(false,"Bluetooth is off")
            transport.blocked!!.complete(Unit)
            assertFalse(controller.connected);assertFalse(controller.connectionBusy)
            transport.blocked=null
            controller.connect(CubeDevice(mac,"Test cube"))
            transport.onBytes(GanCipher(mac).encrypt(solved()))
            assertEquals(Cube.solved,controller.cubeState)
            val gyro=ByteArray(20).apply{this[0]=0xec.toByte();this[1]=8;this[2]=0x7f;this[3]=0xff.toByte()}
            transport.onBytes(GanCipher(mac).encrypt(gyro))
            val orientation=controller.orientation
            assertNotNull(orientation)
            val before=transport.writes.size
            transport.onBytes(byteArrayOf(1,2,3))
            assertNull(controller.cubeState)
            assertEquals(orientation,controller.orientation,"A logical packet fault must not recenter the physical gyro frame")
            assertTrue(transport.writes.size>before)
            transport.onBytes(GanCipher(mac).encrypt(solved()))
            assertEquals(Cube.solved,controller.cubeState)
        } finally {scope.cancel()}
    }
}
