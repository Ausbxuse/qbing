package lab.cube.core
import java.io.File
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.*
import kotlin.test.*

class ReconstructionReviewTest {
 @Test fun rawEventCountsKeepGroupedTurnsInTheirStageWithEqualTimestamps(){
  val moves=listOf(MoveSample("R",0.0),MoveSample("R",0.0),MoveSample("U",0.0))
  val a=Analysis(listOf(Split("Cross",0.0,moves=1,rawMoves=2),Split("F2L",0.0,moves=1,rawMoves=1),Split("OLL",0.0,moves=0,rawMoves=0)),emptyList(),emptyList(),emptyList(),2,0.0,0.0,0.0,finalState=Cube.solved)
  val r=AttemptRecord("r","","solve",duration=0.0,initialState=Cube.solved,moves=moves,analysis=a)
  assertEquals(listOf(ReconstructionRange("Cross",0,2),ReconstructionRange("F2L",2,3),ReconstructionRange("OLL",3,3)),reconstructionRanges(r))
  assertEquals(listOf(ReconstructionRange("Recorded",0,3)),reconstructionRanges(r.copy(analysis=null)))
 }
 @Test fun insertionsDoNotMergePairsTemporarilyDisplacedByAnOpenFace()=runBlocking{
  val catalog=WireJson.decodeFromString<Catalog>(File("src/commonMain/composeResources/files/catalog.json").readText())
  val fixture=WireJson.decodeFromString<AttemptRecord>(File("../androidApp/src/debug/assets/analysis-fixture.json").readText())
  val r=refreshAnalysis(fixture,catalog)
  val result=WireJson.parseToJsonElement(localSolver("native-review",buildJsonObject{put("record",WireJson.encodeToJsonElement(AttemptRecord.serializer(),r))}.toString())).jsonObject
  val pairs=result.getValue("groups").jsonArray.map{it.jsonObject}.filter{it["stage"]?.jsonPrimitive?.content=="F2L"}
  assertEquals(listOf(8,11,14,17),pairs.map{it.getValue("rawEnd").jsonPrimitive.int})
  assertTrue(recordedExecutionMetric(r)!!.tps>=10)
  for(g in pairs){val m=segmentMetrics(r,g.getValue("rawStart").jsonPrimitive.int,g.getValue("rawEnd").jsonPrimitive.int)!!;assertTrue(m.duration>0);assertNotNull(m.tps)}
 }
 @Test fun actualF2lAndOllCasesAreMatchedByTheAndroidSolverEngine()=runBlocking{
  val catalog=WireJson.decodeFromString<Catalog>(File("src/commonMain/composeResources/files/catalog.json").readText())
  for(id in listOf("f2l-1","oll-27")){
   val case=catalog.cases.first{it.id==id};val p=case.variants.first().positions.getValue("D")
   val moves=p.solution.mapIndexed{i,m->MoveSample(m,i*100.0)}
   val split=Split(case.stage,0.0,rawMoves=moves.size,moves=moves.size)
   val a=Analysis(listOf(split),emptyList(),emptyList(),emptyList(),moves.size,1000.0,0.0,0.0,finalState=Cube.solved,cross="D")
   val r=AttemptRecord(id,"","solve",duration=1000.0,initialState=p.state,moves=moves,analysis=a,cross="D")
   val result=WireJson.parseToJsonElement(localSolver("native-review",buildJsonObject{put("record",WireJson.encodeToJsonElement(AttemptRecord.serializer(),r))}.toString())).jsonObject
   val group=result.getValue("groups").jsonArray.first{it.jsonObject["stage"]?.jsonPrimitive?.content==case.stage}.jsonObject
   assertTrue(group.getValue("caseIds").jsonArray.any{it.jsonPrimitive.content==id})
   val variants=group.getValue("variants").jsonArray;assertTrue(variants.isNotEmpty())
   for(v in variants){
    val alg=v.jsonObject.getValue("algorithm").jsonPrimitive.content
    var actual=p.state
    for(token in alg.split(' ')){
     val base=token.takeWhile{it!='2'&&it!='\''};val count=if(token.contains('2'))2 else if(token.endsWith("'"))3 else 1
     val expanded=when(base){"Rw"->listOf("x","L");"Lw"->listOf("x'","R");"Uw"->listOf("y","D");"Dw"->listOf("y'","U");"Fw"->listOf("z","B");"Bw"->listOf("z'","F");"M"->listOf("x'","R","L'");"E"->listOf("y'","U","D'");"S"->listOf("z","F'","B");else->listOf(base)}
     repeat(count){actual=Cube.apply(actual,expanded)}
    }
    val mapped=Cube.faces[Cube.centers(actual).indexOf('D')];val normalized=Cube.reframe(actual,emptyList())
    assertTrue(Cube.goal(normalized,case.stage,mapped),alg)
   }
  }
 }
}
