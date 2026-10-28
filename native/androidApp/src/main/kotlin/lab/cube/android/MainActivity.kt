package lab.cube.android

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.compose.BackHandler
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.background
import androidx.compose.ui.graphics.Color
import androidx.core.view.WindowInsetsControllerCompat
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import kotlinx.coroutines.CompletableDeferred
import lab.cube.core.*
import lab.cube.ui.CubeApp
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class MainActivity:ComponentActivity() {
    private lateinit var controller:LabController
    private var permissionResult:CompletableDeferred<Boolean>?=null
    private var exportText=""
    private var importKind="backup"
    private val permissionLauncher=registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()){result->permissionResult?.complete(result.values.all{it});permissionResult=null}
    private val exportLauncher=registerForActivityResult(ActivityResultContracts.CreateDocument("application/json")){uri->if(uri!=null)runCatching{contentResolver.openOutputStream(uri)?.bufferedWriter()?.use{it.write(exportText)}}.onFailure{controller.message="Export failed: ${it.message}";controller.errorMessage=controller.message}}
    private val importLauncher=registerForActivityResult(ActivityResultContracts.OpenDocument()){uri->if(uri!=null)runCatching{val raw=contentResolver.openInputStream(uri)?.bufferedReader()?.use{it.readText()}?:error("Could not read file");require(raw.length<=25*1024*1024){"File exceeds 25 MB"};if(importKind=="config")controller.saveSettings(WireJson.decodeFromString<ClientSettings>(raw)) else controller.importBackup(raw)}.onFailure{controller.message="Import failed: ${it.message}";controller.errorMessage=controller.message}}
    override fun onCreate(savedInstanceState:Bundle?){
        super.onCreate(savedInstanceState)
        val storage=EncryptedStorage(this)
        controller=LabController(AndroidBle(this){requestPermissions()},storage)
        controller.openBluetoothSettings={startActivity(android.content.Intent(android.provider.Settings.ACTION_BLUETOOTH_SETTINGS))}
        setContent {
            BackHandler(enabled=controller.review!=null||controller.showDevices||controller.page!="Practice") {
                when {
                    controller.review!=null -> controller.review=null
                    controller.showDevices -> controller.showDevices=false
                    controller.page=="Case" -> controller.page="Cases"
                    else -> controller.page="Practice"
                }
            }
            SideEffect {WindowInsetsControllerCompat(window,window.decorView).isAppearanceLightStatusBars=controller.settings.theme=="paper";WindowInsetsControllerCompat(window,window.decorView).isAppearanceLightNavigationBars=controller.settings.theme=="paper";if(controller.active!=null)window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
            Box(Modifier.fillMaxSize().background(if(controller.settings.theme=="paper")Color(0xfffcfcfd)else Color(0xff17191d)).systemBarsPadding()){
                CubeApp(controller,onExport={text->exportText=text;exportLauncher.launch("cstimer-native-backup.json")},onImport={kind->importKind=kind;importLauncher.launch(arrayOf("application/json","text/plain","application/octet-stream"))})
            }
        }
    }
    private suspend fun requestPermissions():Boolean {
        val required=if(Build.VERSION.SDK_INT>=31)arrayOf(Manifest.permission.BLUETOOTH_SCAN,Manifest.permission.BLUETOOTH_CONNECT) else arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
        if(required.all{ContextCompat.checkSelfPermission(this,it)==PackageManager.PERMISSION_GRANTED})return true
        val pending=CompletableDeferred<Boolean>();permissionResult=pending;permissionLauncher.launch(required);return pending.await()
    }
    override fun onStart(){super.onStart();if(::controller.isInitialized)controller.foreground=true}
    override fun onStop(){super.onStop();if(::controller.isInitialized)controller.foreground=false;if(::controller.isInitialized&&controller.active!=null)controller.cancel("app-backgrounded")}
    override fun onDestroy(){if(::controller.isInitialized)controller.close();super.onDestroy()}
}

private class EncryptedStorage(context:android.content.Context):LocalStorage {
    private val preferences=context.getSharedPreferences("cstimer-native",android.content.Context.MODE_PRIVATE)
    private val key:SecretKey by lazy {
        val store=KeyStore.getInstance("AndroidKeyStore").apply{load(null)}
        (store.getKey("cstimer-native-local",null) as? SecretKey)?:KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore").apply{init(KeyGenParameterSpec.Builder("cstimer-native-local",KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())}.generateKey()
    }
    override fun read(key:String):String? {val raw=preferences.getString(key,null)?:return null;return runCatching{val data=Base64.decode(raw,Base64.NO_WRAP);val cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.DECRYPT_MODE,this.key,GCMParameterSpec(128,data.copyOfRange(0,12)));cipher.doFinal(data.copyOfRange(12,data.size)).toString(Charsets.UTF_8)}.getOrNull()}
    override fun write(key:String,value:String){val cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.ENCRYPT_MODE,this.key);val data=cipher.iv+cipher.doFinal(value.toByteArray(Charsets.UTF_8));preferences.edit().putString(key,Base64.encodeToString(data,Base64.NO_WRAP)).apply()}
}
