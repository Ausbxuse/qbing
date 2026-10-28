package lab.cube.core
import kotlin.test.*
class ReplayNavigationTest {
 @Test fun groupedStepsGyroAndEqualTimestampsRemainIndividuallyReversible(){
  val r=AttemptRecord("steps","","solve",duration=400.0,initialState=Cube.solved,moves=listOf("R","L'","R","L'","U").zip(listOf(0.0,60.0,120.0,180.0,400.0)).map{(m,t)->MoveSample(m,t)},rotations=listOf(RotationSample("y",250.0,listOf(0f,0f,0f,1f)),RotationSample("z",250.0,listOf(0f,0f,0f,1f))))
  val p=replayPositions(r);assertEquals(listOf(0,4,4,4,5),p.map{it.rawIndex});assertEquals(listOf(0.0,180.0,250.0,250.0,400.0),p.map{it.time})
  var cursor=0
  for(expected in 1..p.lastIndex){cursor=adjacentReplayPosition(p,p[cursor],cursor,true);assertEquals(expected,cursor)}
  for(expected in p.lastIndex-1 downTo 0){cursor=adjacentReplayPosition(p,p[cursor],cursor,false);assertEquals(expected,cursor)}
  assertEquals(8,replayPositions(r,raw=true).size)
  val zero=replayPositions(r.copy(moves=listOf(MoveSample("R",0.0),MoveSample("U",0.0)),rotations=emptyList()),raw=true)
  assertEquals(1,adjacentReplayPosition(zero,zero[0],null,true));assertEquals(0,adjacentReplayPosition(zero,zero[1],null,false))
  assertEquals(3,adjacentReplayPosition(p,ReplayPosition(4,300.0),null,false))
 }
}
