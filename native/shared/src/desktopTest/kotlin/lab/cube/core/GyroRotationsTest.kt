package lab.cube.core
import kotlin.test.*
import kotlin.math.*

class GyroRotationsTest {
 @Test fun observationsAreSnappedHystereticAndAntipodalSafe(){
  val r=GyroRotationRecorder();val s=sqrt(.5f)
  assertNull(r.observe(listOf(0f,0f,0f,1f),false))
  val a=48f*PI.toFloat()/360f
  assertNull(r.observe(listOf(0f,sin(a),0f,cos(a)),true))
  assertEquals("y'",r.observe(listOf(0f,s,0f,s),true)!!.first)
  assertNull(r.observe(listOf(0f,-s,0f,-s),true))
  assertEquals("y",r.observe(listOf(0f,0f,0f,1f),true)!!.first)
 }
 @Test fun allPosePairsHaveValidDeltas(){for(a in CubeOrientationSnap.poses)for(b in CubeOrientationSnap.poses)if(!a.closeTo(b))assertTrue(rotationTransition(a,b).isNotBlank())}
 @Test fun rotationTimelineRoundTripsWithoutChangingMovesOrTps(){
  val r=AttemptRecord("gyro-fixture","2026-10-27T00:00:00Z","solve",duration=1000.0,initialState=Cube.solved,moves=listOf(MoveSample("R",0.0),MoveSample("U",1000.0)),rotations=listOf(RotationSample("y'",500.0,listOf(0f,sqrt(.5f),0f,sqrt(.5f)))))
  val roundTrip=WireJson.decodeFromString<AttemptRecord>(WireJson.encodeToString(AttemptRecord.serializer(),r))
  assertEquals(r,roundTrip)
  val timeline=reconstructionWithRotations(r,true)
  assertEquals(listOf("R","y'","U"),timeline.map{it.notation})
  assertEquals(1,timeline[1].rawStart);assertEquals(1,timeline[1].rawEnd)
  assertEquals(countMoves(r.moves),countMoves(roundTrip.moves))
  assertTrue(reconstructionWithRotations(r.copy(rotations=emptyList())).none{it.rotation})
 }
 @Test fun recordedReplaySeeksDeterministicallyAcrossAllCrossFrames(){
  val s=sqrt(.5f)
  val r=AttemptRecord("replay-poses","2026-10-27T00:00:00Z","solve",duration=400.0,initialState=Cube.solved,rotations=listOf(
   RotationSample("y'",200.0,listOf(0f,s,0f,s)),
   RotationSample("x'",100.0,listOf(s,0f,0f,s)),
   RotationSample("z'",200.0,listOf(0f,0f,s,s)),
   RotationSample("x",300.0,listOf(0f,0f,0f,0f))))
  for(face in Cube.faces){
   val base=crossReplayPose(face)
   repeat(3){
    assertTrue((base*recordedRotationPose(r,200.0)).closeTo(base*CubeQuaternion(0f,0f,s,s)))
    assertTrue((base*recordedRotationPose(r,150.0)).closeTo(base*CubeQuaternion(s,0f,0f,s)))
    assertTrue((base*recordedRotationPose(r,0.0)).closeTo(base))
   }
  }
  assertTrue(recordedRotationPose(r,400.0).closeTo(CubeQuaternion(0f,0f,s,s)))
  assertTrue(recordedRotationPose(r.copy(rotations=emptyList()),400.0).closeTo(CubeQuaternion()))
 }
}

class LiveReplayFrameTest {
 @Test fun everyLiveHoldAndCrossKeepsTheSameReplayFrame(){
  val basis=CubeQuaternion(.2f,.3f,.1f,.7f).normalized()
  val s=sqrt(.5f);val tilt=CubeQuaternion(sin(12f*PI.toFloat()/360),0f,0f,cos(12f*PI.toFloat()/360))
  for(hold in CubeOrientationSnap.poses){
   val live=CubeOrientationSnap();val recorder=GyroRotationRecorder()
   val held=basis*hold*tilt
   val initial=live.update(held.relativeTo(basis))
   recorder.setViewFrame(basis.values(),initial.values(),false);recorder.observe(held.values(),false)
   assertTrue(recorder.initialPose!!.closeTo(initial))
   var record=AttemptRecord("live-frame","2026-10-27T00:00:00Z","solve",duration=4000.0,initialState=Cube.solved,rotationFrame="live-v1",initialOrientation=initial.values())
   var pose=hold
   for((i,turn)in listOf(CubeQuaternion(0f,-s,0f,s),CubeQuaternion(-s,0f,0f,s),CubeQuaternion(0f,0f,-s,s)).withIndex()){
    pose=pose*turn;val raw=basis*pose;val target=live.update(raw.relativeTo(basis));val time=(i+1)*1000.0
    recorder.observe(raw.values(),true)?.let{(notation,p)->record=record.copy(rotations=record.rotations+RotationSample(notation,time,p))}
    for(cross in Cube.faces){
     val r=record.copy(cross=cross.toString())
     assertTrue(recordedReplayPose(r,time).closeTo(target),"$hold / $cross / $i")
     assertTrue(recordedReplayPose(r,0.0).closeTo(initial))
    }
   }
  }
 }
 @Test fun activeRecordingRoundTripsTheLiveInitialPoseAndAbsoluteTargets(){
  val s=sqrt(.5f);val hold=CubeQuaternion(0f,s,0f,s)
  val a=ActiveAttempt(Cube.solved,"solve","solve","U","test",readyAt=0.0)
  a.captureViewFrame(CubeQuaternion().values(),hold.values(),0.0)
  a.orientation(hold.values(),1.0)
  a.move(CubePacket.Move(0,"R",1000.0,0.0),Cube.turn(Cube.solved,"R"),100.0)
  val pose=hold*CubeQuaternion(s,0f,0f,s);a.orientation(pose.values(),600.0)
  a.move(CubePacket.Move(1,"R'",2000.0,1000.0),Cube.solved,1100.0)
  val r=a.record(Catalog(emptyList(),emptyMap()))
  val saved=WireJson.decodeFromString<AttemptRecord>(WireJson.encodeToString(AttemptRecord.serializer(),r))
  assertEquals("live-v1",saved.rotationFrame);assertEquals(r,saved)
  assertTrue(recordedReplayPose(saved,0.0).closeTo(hold))
  assertTrue(recordedReplayPose(saved,500.0).closeTo(pose))
  assertEquals(2,countMoves(saved.moves));assertEquals(1000.0,saved.duration)
 }
 @Test fun recenterKeepsOriginalStartAndRecordsTheResetPose(){
  val s=sqrt(.5f);val start=CubeQuaternion(0f,s,0f,s);val r=GyroRotationRecorder()
  r.setViewFrame(CubeQuaternion().values(),start.values(),false)
  assertTrue(CubeQuaternion.of(r.setViewFrame(start.values(),CubeQuaternion().values(),true)!!.second)!!.closeTo(CubeQuaternion()))
  assertTrue(r.initialPose!!.closeTo(start))
 }
}

class ReplayMetadataSyncTest {
 @Test fun syncRetainsMatchingEvidenceAndRespectsRemoteChanges(){
  val local=AttemptRecord("pose","2026-10-27T00:00:00Z","solve",duration=1000.0,initialState=Cube.solved,moves=listOf(MoveSample("R",0.0),MoveSample("U",1000.0)),rotationFrame="live-v1",initialOrientation=listOf(0f,1f,0f,0f),rotations=listOf(RotationSample("x",500.0,listOf(0f,0f,0f,1f))))
  val remote=local.copy(rotationFrame=null,initialOrientation=null,rotations=emptyList(),penalty="+2")
  val merged=retainReplayMetadata(remote,local)
  assertEquals(local.initialOrientation,merged.initialOrientation);assertEquals(local.rotations,merged.rotations);assertEquals("+2",merged.penalty)
  val changed=remote.copy(moves=listOf(MoveSample("L",0.0),MoveSample("U",1000.0)))
  assertTrue(retainReplayMetadata(changed,local).rotations.isEmpty())
  assertEquals(local.rotations,retainReplayMetadata(local.copy(rotations=emptyList()),local).rotations)
  val rich=local.copy(initialOrientation=listOf(0f,0f,0f,1f))
  assertEquals(rich,retainReplayMetadata(rich,local))
 }
}
