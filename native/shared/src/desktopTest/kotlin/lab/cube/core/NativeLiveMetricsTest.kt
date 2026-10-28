package lab.cube.core
import kotlin.test.*
class NativeLiveMetricsTest{
 @Test fun rateUsesTheLiveSnapshotWithoutInventingUncertaintyFromInteriorRecovery(){
  val moves=listOf(MoveSample("R",0.0,raw=RawMove(cubeTimestamp=1000.0)),MoveSample("R",500.0,true),MoveSample("U",1000.0,raw=RawMove(cubeTimestamp=2000.0)))
  val a=nativeLiveMetrics(moves,listOf("estimated-timing"),1000.0);val b=nativeLiveMetrics(moves,listOf("estimated-timing"),2000.0)
  assertEquals(2,a.moves);assertEquals(2.0,a.tps);assertEquals(1.0,b.tps);assertFalse(a.estimatedStart);assertFalse(b.estimatedStart)
  assertNull(nativeLiveMetrics(moves,listOf("missing-moves"),2000.0).tps)
 }
}
