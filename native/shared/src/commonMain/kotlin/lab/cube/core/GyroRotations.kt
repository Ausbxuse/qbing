package lab.cube.core

import kotlin.math.*

internal data class CubeQuaternion(val x:Float=0f,val y:Float=0f,val z:Float=0f,val w:Float=1f) {
    fun normalized():CubeQuaternion {val n=sqrt(x*x+y*y+z*z+w*w);return if(n<0.00001f)CubeQuaternion() else CubeQuaternion(x/n,y/n,z/n,w/n)}
    fun inverse()=CubeQuaternion(-x,-y,-z,w)
    fun values()=listOf(x,y,z,w)
    operator fun times(b:CubeQuaternion)=CubeQuaternion(w*b.x+x*b.w+y*b.z-z*b.y,w*b.y-x*b.z+y*b.w+z*b.x,w*b.z+x*b.y-y*b.x+z*b.w,w*b.w-x*b.x-y*b.y-z*b.z).normalized()
    // Calibrate in the initial cube frame, matching gan-cube-sample's inverse(initial) * current.
    // Reversing this order rotates the motion axes when the cube was connected at an angle.
    fun relativeTo(reference:CubeQuaternion)=reference.inverse()*this
    fun interpolate(other:CubeQuaternion,t:Float):CubeQuaternion {
        var b=other;var dot=x*b.x+y*b.y+z*b.z+w*b.w
        if(dot<0){b=CubeQuaternion(-b.x,-b.y,-b.z,-b.w);dot=-dot}
        if(dot>.9995f)return CubeQuaternion(x+(b.x-x)*t,y+(b.y-y)*t,z+(b.z-z)*t,w+(b.w-w)*t).normalized()
        val angle=acos(dot.coerceIn(-1f,1f));val scale=sin(angle);val a=sin((1-t)*angle)/scale;val c=sin(t*angle)/scale
        return CubeQuaternion(x*a+b.x*c,y*a+b.y*c,z*a+b.z*c,w*a+b.w*c)
    }
    fun closeTo(b:CubeQuaternion)=abs(x*b.x+y*b.y+z*b.z+w*b.w)>.99999f
    companion object {fun of(v:List<Float>?)=v?.takeIf{it.size==4&&it.all(Float::isFinite)}?.let{CubeQuaternion(it[0],it[1],it[2],it[3]).normalized()}}
}


/** Stable whole-cube poses. Face turns remain independent of this visual orientation. */
internal class CubeOrientationSnap {
    var selected:CubeQuaternion=CubeQuaternion();private set
    fun reset(){selected=CubeQuaternion()}
    fun seed(pose:CubeQuaternion){selected=pose}
    fun update(relative:CubeQuaternion):CubeQuaternion {
        val current=relative.normalized()
        fun distance(pose:CubeQuaternion)=2*acos(abs(current.x*pose.x+current.y*pose.y+current.z*pose.z+current.w*pose.w).coerceIn(0f,1f))
        val nearest=poses.minBy(::distance)
        if(distance(selected)>distance(nearest)+8f*PI.toFloat()/180f)selected=nearest
        return selected
    }
    companion object {
        // Repeated proper 90° rotations generate the cube's 24 rotational symmetries.
        // Antipodal quaternions encode the same pose and are deduplicated.
        val poses:List<CubeQuaternion> by lazy {
            val s=sqrt(.5f)
            val rotations=listOf(CubeQuaternion(s,0f,0f,s),CubeQuaternion(0f,s,0f,s),CubeQuaternion(0f,0f,s,s))
            val result=mutableListOf(CubeQuaternion());var index=0
            while(index<result.size){
                val pose=result[index++]
                for(rotation in rotations){val candidate=(rotation*pose).normalized();if(result.none{it.closeTo(candidate)})result.add(candidate)}
            }
            check(result.size==24);result.toList()
        }
    }
}


/** Local gyro observation times; never included in HTM or turn timing statistics. */
internal class GyroRotationRecorder {
 private var reference:CubeQuaternion?=null
 private val snap=CubeOrientationSnap()
 private var previous=CubeQuaternion()
 var liveFrame=false;private set
 var initialPose:CubeQuaternion?=null;private set
 fun setViewFrame(basis:List<Float>,values:List<Float>,running:Boolean):Pair<String,List<Float>>? {
  val referencePose=CubeQuaternion.of(basis)?:return null
  val pose=CubeQuaternion.of(values)?:return null
  if(liveFrame&&reference?.closeTo(referencePose)==true)return null
  val wasLive=liveFrame;liveFrame=true;reference=referencePose;snap.seed(pose)
  if(!wasLive||!running){initialPose=pose;previous=pose;return null}
  return changed(pose)
 }
 private fun changed(pose:CubeQuaternion):Pair<String,List<Float>>? {
  if(pose.closeTo(previous))return null
  val notation=rotationTransition(previous,pose);previous=pose
  return notation to pose.values()
 }
 fun observe(values:List<Float>,running:Boolean):Pair<String,List<Float>>? {
  val q=CubeQuaternion.of(values)?:return null
  if(liveFrame){val pose=snap.update(q.relativeTo(reference!!));if(!running){initialPose=pose;previous=pose;return null};return changed(pose)}
  if(!running||reference==null){reference=q;snap.reset();previous=CubeQuaternion();return null}
  val pose=snap.update(q.relativeTo(reference!!))
  if(pose.closeTo(previous))return null
  val notation=rotationTransition(previous,pose);previous=pose
  return notation to listOf(pose.x,pose.y,pose.z,pose.w)
 }
}
internal fun rotationTransition(from:CubeQuaternion,to:CubeQuaternion):String {
 val s=sqrt(.5f)
 val generators=listOf("x" to CubeQuaternion(-s,0f,0f,s),"x'" to CubeQuaternion(s,0f,0f,s),"x2" to CubeQuaternion(1f,0f,0f,0f),"y" to CubeQuaternion(0f,-s,0f,s),"y'" to CubeQuaternion(0f,s,0f,s),"y2" to CubeQuaternion(0f,1f,0f,0f),"z" to CubeQuaternion(0f,0f,-s,s),"z'" to CubeQuaternion(0f,0f,s,s),"z2" to CubeQuaternion(0f,0f,1f,0f))
 val queue=ArrayDeque<Pair<CubeQuaternion,String>>();queue.add(from to "");val seen=mutableListOf(from)
 while(queue.isNotEmpty()){
  val (pose,path)=queue.removeFirst()
  for((name,rotation)in generators){val next=pose*rotation;val text=if(path.isEmpty())name else "$path $name";if(next.closeTo(to))return text;if(seen.none{it.closeTo(next)}){seen.add(next);queue.add(next to text)}}
 }
 error("Proper cube orientations are connected")
}

internal fun crossReplayPose(face:Char):CubeQuaternion {
 val s=sqrt(.5f)
 return when(face){'U'->CubeQuaternion(0f,0f,1f,0f);'F'->CubeQuaternion(s,0f,0f,s);'B'->CubeQuaternion(-s,0f,0f,s);'R'->CubeQuaternion(0f,0f,-s,s);'L'->CubeQuaternion(0f,0f,s,s);else->CubeQuaternion()}
}

internal fun reconstructionCross(record:AttemptRecord):Char = record.analysis?.cross?.singleOrNull()?.takeIf{it in Cube.faces}
 ?:record.cross.singleOrNull()?.takeIf{it in Cube.faces}?:'D'

/** Absolute observation lookup, independent of seek order and live calibration. */
internal fun recordedRotationPose(record:AttemptRecord,time:Double):CubeQuaternion =
 record.rotations.withIndex().filter{it.value.t.isFinite()&&it.value.t>=0&&it.value.t<=time&&CubeQuaternion.of(it.value.pose)!=null&&it.value.pose.any{v->v!=0f}}
  .maxWithOrNull(compareBy<IndexedValue<RotationSample>>{it.value.t}.thenBy{it.index})
  ?.let{CubeQuaternion.of(it.value.pose)}?:initialLivePose(record)?:CubeQuaternion()

internal fun initialLivePose(record:AttemptRecord):CubeQuaternion? =
 record.initialOrientation?.takeIf{record.rotationFrame=="live-v1"&&it.any{v->kotlin.math.abs(v)>0.000001f}}?.let(CubeQuaternion::of)

/** Live recordings already use the calibrated display frame. Never apply cross-down twice. */
internal fun recordedReplayPose(record:AttemptRecord,time:Double):CubeQuaternion {
 val pose=recordedRotationPose(record,time)
 return if(initialLivePose(record)!=null)pose else crossReplayPose(reconstructionCross(record))*pose
}
