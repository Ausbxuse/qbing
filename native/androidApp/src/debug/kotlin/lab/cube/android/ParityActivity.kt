package lab.cube.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import kotlinx.coroutines.delay
import kotlinx.serialization.json.*
import lab.cube.core.*
import lab.cube.ui.CubeApp
import lab.cube.ui.ObserveNativeLayout

/** Debug-only, in-memory fixtures. Never opens Bluetooth or reads user storage. */
class ParityActivity:ComponentActivity(){
 private lateinit var controller:LabController
 override fun onCreate(savedInstanceState:Bundle?){
  super.onCreate(savedInstanceState)
  filesDir.resolve("parity-ready").delete()
  WindowCompat.setDecorFitsSystemWindows(window,false)
  WindowInsetsControllerCompat(window,window.decorView).hide(WindowInsetsCompat.Type.systemBars())
  val theme=intent.getStringExtra("theme")?:"paper"
  val fixture=WireJson.parseToJsonElement(assets.open("parity-fixture.json").bufferedReader().use{it.readText()}).jsonObject
  val data=mutableMapOf("cache" to fixture.toString(),"settings" to WireJson.encodeToString(ClientSettings.serializer(),ClientSettings(theme=theme,session="Session 1")))
  savedInstanceState?.getString("graph-metric")?.let{data["graph-metric"]=it}
  val transport=object:BleTransport{
   override var onBytes:(ByteArray)->Unit={};override var onDisconnected:(String)->Unit={}
   override suspend fun scan()=emptyList<CubeDevice>();override suspend fun connect(device:CubeDevice){};override suspend fun write(bytes:ByteArray){};override suspend fun disconnect(){}
  }
  controller=LabController(transport,object:LocalStorage{override fun read(key:String)=data[key];override fun write(key:String,value:String){data[key]=value}})
  setContent {
   LaunchedEffect(Unit){while(controller.catalog.cases.isEmpty()||controller.target==null)delay(20);controller.prepareSequence("D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2".split(" "));val view=intent.getStringExtra("view");if(view?.startsWith("case-analysis-")==true){
    val id=when(view){"case-analysis-f2l"->"f2l-1";"case-analysis-oll"->"oll-27";else->"pll-t"};val case=controller.catalog.cases.first{it.id==id}
    for(i in 0 until 12){val v=case.variants[(if(i<8)0 else 1).coerceAtMost(case.variants.lastIndex)];val p=v.positions.getValue("D");val duration=1800.0-i*60
     controller.save(AttemptRecord("case-fixture-$id-$i","2026-10-27T10:${i.toString().padStart(2,'0')}:00Z","drill",duration=duration,initialState=p.state,cross="D",caseId=id,selectedAlgorithmId=v.id,recognitionSource="cue",recognitionMs=500.0-i*20,moves=p.solution.mapIndexed{j,m->MoveSample(m,j*duration/(p.solution.size-1))}))
    };controller.selectedCase=id;controller.selectedVariant=0;controller.page="Case"
   }else when(view){"history"->controller.page="History";"settings"->controller.page="Settings";"review-analysis"->{val record=WireJson.decodeFromString<AttemptRecord>(assets.open("analysis-fixture.json").bufferedReader().use{it.readText()});controller.save(record);controller.review=controller.records.first{it.id==record.id}};"review"->controller.review=controller.records.lastOrNull{it.kind=="solve"};"review-live-frame"->{val s=kotlin.math.sqrt(.5f);controller.review=controller.records.lastOrNull{it.kind=="solve"}?.copy(rotationFrame="live-v1",initialOrientation=listOf(0f,s,0f,s),rotations=listOf(RotationSample("y'",250.0,listOf(0f,1f,0f,0f)),RotationSample("x'",750.0,listOf(0f,s,-s,0f))))}};filesDir.resolve("parity-ready").writeText("true")}
   val measured=remember{mutableMapOf<String,JsonElement>()}
   ObserveNativeLayout({name,bounds->
    measured[name]=buildJsonArray{add(bounds.left);add(bounds.top);add(bounds.width);add(bounds.height)}
    filesDir.resolve("parity-bounds.json").writeText(JsonObject(measured).toString())
   }){CubeApp(controller)}
  }
 }
 override fun onSaveInstanceState(outState:Bundle){if(::controller.isInitialized)outState.putString("graph-metric",controller.graphMetric.name);super.onSaveInstanceState(outState)}
 override fun onDestroy(){if(::controller.isInitialized)controller.close();super.onDestroy()}
}
