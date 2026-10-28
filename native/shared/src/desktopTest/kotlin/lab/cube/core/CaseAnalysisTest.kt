package lab.cube.core
import java.io.File
import kotlin.test.*
class CaseAnalysisTest {
 @Test fun scrambleCompletionFreezesAtTheDeviceEndpoint(){
  val t=ScrambleTracker();val first=Cube.turn(Cube.solved,"R");val end=Cube.turn(first,"U2")
  t.observe(Cube.solved,first,CubePacket.Move(1,"R",1000.0,0.0),0.0)
  t.observe(first,end,CubePacket.Move(2,"U2",1400.0,400.0),400.0)
  t.freeze();assertEquals(t.stats(),t.liveStats(15000.0));assertEquals(400.0,t.liveStats(15000.0).duration)
  t.observe(end,Cube.turn(end,"F"),CubePacket.Move(3,"F",17000.0,16000.0),16000.0)
  assertTrue(t.liveStats(18000.0).duration>t.stats().duration)
 }
 @Test fun localAnalyticsRecognizeTheObservedSequenceAndExcludeTransitionAndDnf(){
  val catalog=WireJson.decodeFromString<Catalog>(File("src/commonMain/composeResources/files/catalog.json").readText());val case=catalog.cases.first{it.id=="f2l-1"};val p=case.variants[0].positions.getValue("D")
  val r=AttemptRecord("a","2026-01-01","drill",duration=600.0,initialState=p.state,cross="D",caseId=case.id,selectedAlgorithmId="unrelated",moves=p.solution.mapIndexed{i,m->MoveSample(m,i*200.0)},recognitionSource="cue",recognitionMs=250.0)
  val perf=localCasePerformance(listOf(r,r.copy(id="b",recognitionSource="transition",recognitionMs=9999.0),r.copy(id="c",penalty="DNF")),case)!!
  val s=caseSummary(perf);assertEquals(2,s.count);assertEquals(1,s.recognitionCount);assertEquals(250.0,s.meanRecognition);assertEquals(600.0,s.meanExecution);assertEquals(1,perf.algorithms[0].variant)
 }
}
