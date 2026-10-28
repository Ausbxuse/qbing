package lab.cube.core
import kotlin.test.*
import kotlinx.coroutines.*
import kotlin.coroutines.CoroutineContext
class GraphMetricTest {
 @Test fun metricChoiceSurvivesControllerRecreationWithoutChangingSolveData(){
  val dispatcher=object:CoroutineDispatcher(){override fun dispatch(context:CoroutineContext,block:Runnable){}}
  val scope=CoroutineScope(SupervisorJob()+dispatcher)
  val values=mutableMapOf<String,String>();val storage=object:LocalStorage{override fun read(key:String)=values[key];override fun write(key:String,value:String){values[key]=value}}
  val transport=object:BleTransport{override var onBytes:(ByteArray)->Unit={};override var onDisconnected:(String)->Unit={};override suspend fun scan()=emptyList<CubeDevice>();override suspend fun connect(device:CubeDevice){};override suspend fun write(bytes:ByteArray){};override suspend fun disconnect(){}}
  try{val first=LabController(transport,storage,scope);first.selectGraphMetric(GraphMetric.Moves);val recreated=LabController(transport,storage,scope);assertEquals(GraphMetric.Moves,recreated.graphMetric);assertTrue(recreated.records.isEmpty());assertEquals(setOf("graph-metric"),values.keys)}finally{scope.cancel()}
 }
 @Test fun missingMetricsKeepTheirSolveSlotsAndTpsUsesExecutionRatherThanPenalty(){
  fun sensor(duration:Double,penalty:String="")=AttemptRecord("s","","solve",duration=duration,initialState=Cube.solved,penalty=penalty,moves=listOf(MoveSample("R",0.0),MoveSample("U2",duration)))
  val records=listOf(sensor(1000.0),sensor(1500.0).copy(source="manual",moves=emptyList()),sensor(2000.0,"+2"),sensor(1000.0,"DNF"))
  val tps=graphSeries(records,GraphMetric.Tps);assertEquals(listOf(2.0,null,1.0,null),tps.slots);assertEquals(1.5,tps.mean);assertEquals(2.0,tps.best)
  val moves=graphSeries(records,GraphMetric.Moves);assertEquals(listOf(2.0,null,2.0,null),moves.slots);assertEquals(1.0,moves.histogram(GraphMetric.Moves,300f).step)
  for(metric in GraphMetric.entries){val s=graphSeries(records,metric);assertEquals(s.slots.count{it!=null},s.histogram(metric,300f).bins.sum())}
 }
}
