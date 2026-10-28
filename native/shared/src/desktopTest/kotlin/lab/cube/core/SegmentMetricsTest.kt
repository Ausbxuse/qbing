package lab.cube.core
import kotlin.test.*
class SegmentMetricsTest {
 @Test fun windowsUseWholeSolveHtmWeightsAndSeparateEntryGap(){
  val moves=listOf("R","R","U","F").zip(listOf(0.0,30.0,100.0,200.0)).map{(m,t)->MoveSample(m,t,raw=RawMove(cubeTimestamp=1000+t))}
  val r=AttemptRecord("segment","2026-10-27T00:00:00Z","solve",duration=200.0,initialState=Cube.solved,moves=moves)
  assertEquals(1,segmentMetrics(r,0,2)!!.moves)
  val b=segmentMetrics(r,2,4)!!;assertEquals(70.0,b.gap);assertEquals(100.0,b.execution);assertEquals(2,b.moves)
  val recovered=r.copy(moves=moves.mapIndexed{i,m->if(i==1)m.copy(recovered=true)else m},quality=listOf("estimated-timing"))
  assertFalse(segmentMetrics(recovered,0,4)!!.estimated)
  assertTrue(segmentMetrics(recovered,2,4)!!.estimated)
  assertNull(segmentMetrics(r,3,8))
 }
}
