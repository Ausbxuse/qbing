package lab.cube.core
import kotlin.test.*
class NativeStatisticsTest {
 @Test fun rollingAo5UsesOriginalWindowsAndTrimsDnfsAndPenalties(){
  val r=(10..15).map{AttemptRecord("$it","","solve","manual",duration=it*1000.0,initialState=Cube.solved)}
  assertEquals(listOf(null,null,null,null,12000.0,13000.0),rollingAo5(r))
  val penalized=r.toMutableList();penalized[2]=r[2].copy(penalty="+2")
  assertEquals(38000.0/3,rollingAo5(penalized)[4])
  penalized[2]=r[2].copy(penalty="DNF");assertEquals(38000.0/3,rollingAo5(penalized)[4])
  penalized[3]=r[3].copy(penalty="DNF");assertTrue(rollingAo5(penalized)[4]!!.isInfinite())
 }

 @Test fun binsAndPopulationStatisticsMatchTheApprovedWebReference(){
  val records=(0 until 20).map{AttemptRecord("$it","","solve","manual",duration=10500.0+it*170,initialState=Cube.solved)}
  val d=nativeDistribution(records);assertEquals(500.0,d.step);assertEquals(listOf(3,3,3,3,3,3,2),d.bins);assertEquals(12115.0,d.median);assertEquals(13390.0,d.p90);assertEquals(0.0,d.dnfRate)
 }
}
