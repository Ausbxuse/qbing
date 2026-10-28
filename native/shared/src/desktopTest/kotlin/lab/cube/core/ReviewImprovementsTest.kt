package lab.cube.core
import kotlin.test.*
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.*

class ReviewImprovementsTest {
 @Test fun reconstructionUsesDetectedCrossAndRetainsLegacyFallback(){
  val r=AttemptRecord("fixture","2026-10-27T00:00:00Z","solve",duration=1000.0,initialState=Cube.solved,cross="U")
  val a=Analysis(emptyList(),emptyList(),emptyList(),emptyList(),0,1000.0,0.0,0.0,finalState=Cube.solved,cross="D")
  assertEquals('D',reconstructionCross(r.copy(analysis=a)));assertEquals('U',reconstructionCross(r.copy(analysis=a.copy(cross=""))))
  assertTrue(crossReplayPose('U').closeTo(CubeQuaternion(0f,0f,1f,0f)));assertTrue(crossReplayPose('D').closeTo(CubeQuaternion()))
 }
 @Test fun orientedNotationChangesTheViewWithoutRewritingSensorTurns(){
  val r=AttemptRecord("fixture","2026-10-27T00:00:00Z","solve",duration=200.0,initialState=Cube.solved,cross="U",moves=listOf(MoveSample("R",0.0),MoveSample("U",200.0)))
  assertEquals(listOf("L","D"),reconstructionWithRotations(r,viewFrame=true).map{it.notation})
  assertEquals(listOf("R","U"),reconstruct(r,true).map{it.notation})
 }
 @Test fun adaptiveHistogramPreservesCountsAndStatisticsAcrossWidths(){
  val records=listOf(10000.0,10100.0,10200.0,10500.0,11000.0,90000.0).mapIndexed{i,t->AttemptRecord("$i","2026-10-27T00:00:00Z","solve",duration=t,initialState=Cube.solved)}
  val pair=nativeHistogram(records.take(1)+records.last(),240f);assertTrue(pair.bins.all{it>0})
  for(width in listOf(240f,400f,1200f)){
   val h=nativeHistogram(records,width);assertEquals(records.size,h.bins.sum());assertEquals(nativeDistribution(records).median,h.median)
   var gap=0;for(n in h.bins){gap=if(n==0)gap+1 else 0;assertTrue(gap<=1)}
  }
 }
 @Test fun currentCrossAndXcrossRoutesAreRealVerifiedMoves()=runBlocking {
  val scramble="R U F2 D L' B2 R2 U' F R2 D2 L2"
  val state=Cube.apply(Cube.solved,scramble.split(' '))
  val result=WireJson.parseToJsonElement(localSolver("native-cross-dashboard",buildJsonObject{put("state",state);put("faces","UD")}.toString())).jsonObject
  val routes=result.getValue("routes").jsonArray;assertEquals(4,routes.size)
  for(value in routes){val r=value.jsonObject;assertTrue(r.getValue("verified").jsonPrimitive.boolean);assertTrue(r.getValue("length").jsonPrimitive.int>0)
   val end=Cube.apply(state,r.getValue("fixedMoves").jsonArray.map{it.jsonPrimitive.content})
   assertTrue(Cube.goal(end,"Cross",r.getValue("face").jsonPrimitive.content[0]))
  }
 }
}
