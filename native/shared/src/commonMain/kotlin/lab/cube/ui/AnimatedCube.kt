package lab.cube.ui

import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.channels.Channel
import lab.cube.core.Cube
import lab.cube.core.VisualTurn
import kotlin.math.*

internal typealias CubeQuaternion=lab.cube.core.CubeQuaternion
internal typealias CubeOrientationSnap=lab.cube.core.CubeOrientationSnap
internal const val CUBE_HOME_YAW=-.5980603f
internal const val CUBE_HOME_PITCH=.411252f

internal data class CubeRenderFrame(val state:String,val move:String?=null,val angle:Float=0f,val yaw:Float=CUBE_HOME_YAW,val pitch:Float=CUBE_HOME_PITCH,val orientation:CubeQuaternion=CubeQuaternion(),val partnerMove:String?=null,val partnerAngle:Float=0f,val logoAngle:Float=0f)
internal data class CubeTurnPose(val move:String?=null,val angle:Float=0f,val partnerMove:String?=null,val partnerAngle:Float=0f)
private data class TurnFrame(val state:String,val move:String,val cubeMs:Double?,val recovered:Boolean)
private data class ActiveVisualTurn(val first:TurnFrame,var partner:TurnFrame?=null,var partnerStart:Long?=null)
internal val cubeTurnSpecs=mapOf('R' to (0 to 1),'L' to (0 to -1),'U' to (1 to 1),'D' to (1 to -1),'F' to (2 to 1),'B' to (2 to -1))

/** Event states remain authoritative; elapsed time carries across queued turns. */
internal class CubeTurnAnimator(initial:String) {
    private val queue=ArrayDeque<TurnFrame>()
    private var active:ActiveVisualTurn?=null
    private var started=0L
    private var duration=0L
    private var incomingState=initial
    var state=initial;private set
    var markTurns:Int=0;private set
    val busy get()=active!=null||queue.isNotEmpty()
    fun enqueue(state:String,move:String?,cubeMs:Double?=null,recovered:Boolean=false) {
        // Validate the untouched raw event chain before any visual-only commuting.
        // Absolute raw snapshots cannot be used as reordered intermediate endpoints.
        if(move==null||cubeTurnSpecs[move.firstOrNull()]==null||runCatching{Cube.turn(incomingState,move)}.getOrNull()!=state){
            queue.clear();active=null;this.state=state
        }else if(queue.size>=8){queue.clear();active=null;this.state=state}else queue.addLast(TurnFrame(state,move,cubeMs,recovered))
        incomingState=state
    }
    private fun pairs(first:TurnFrame,second:TurnFrame):Boolean {
        if(first.recovered||second.recovered)return false
        val timeA=first.cubeMs?.takeIf{it.isFinite()}?:return false
        val timeB=second.cubeMs?.takeIf{it.isFinite()}?:return false
        if(timeB-timeA !in 0.0..100.0)return false
        val a=cubeTurnSpecs[first.move.firstOrNull()]?:return false
        val b=cubeTurnSpecs[second.move.firstOrNull()]?:return false
        // Opposite faces with inverse notation rotate in the same physical direction.
        // GAN's gyro provides the frame rotation; never insert M/E/S into logical state.
        if(a.first!=b.first||a.second==b.second)return false
        fun quarter(move:String)=when(move.drop(1)){""->1;"'"->-1;else->0}
        val q=quarter(first.move)
        return q!=0&&q == -quarter(second.move)
    }
    private fun takePartner(first:TurnFrame):TurnFrame? {
        if(first.recovered)return null
        val firstTime=first.cubeMs?.takeIf{it.isFinite()}?:return null
        val axis=cubeTurnSpecs[first.move.firstOrNull()]?.first?:return null
        var previousTime=firstTime
        for(i in queue.indices){
            val candidate=queue[i]
            val time=candidate.cubeMs?.takeIf{it.isFinite()}?:break
            if(candidate.recovered||cubeTurnSpecs[candidate.move.firstOrNull()]?.first!=axis||time<previousTime||time-firstTime>100.0)break
            if(pairs(first,candidate))return queue.removeAt(i)
            previousTime=time
        }
        return null
    }
    private fun angle(move:String,t:Float):Float {
        val side=cubeTurnSpecs.getValue(move[0]).second
        val turns=if(move.endsWith('2'))2 else if(move.endsWith('\''))-1 else 1
        val eased=t*t*t*(t*(t*6-15)+10)
        return -side*turns*PI.toFloat()/2*eased
    }
    fun advance(now:Long):CubeTurnPose {
        var nextStart=now
        while(true){
            val old=active
            if(old!=null){
                if(now-started<duration){
                    if(old.partner==null){takePartner(old.first)?.let{old.partner=it;old.partnerStart=now}}
                    val t=((now-started).toFloat()/duration).coerceIn(0f,1f)
                    // A partner delivered after the first layer started joins from zero,
                    // then catches up at the same endpoint; neither layer jumps backwards.
                    val partnerT=old.partnerStart?.let{((now-it).toFloat()/(started+duration-it).coerceAtLeast(1)).coerceIn(0f,1f)}?:t
                    return CubeTurnPose(old.first.move,angle(old.first.move,t),old.partner?.move,old.partner?.let{angle(it.move,partnerT)}?:0f)
                }
                fun mark(move:String){if(move.startsWith("U"))markTurns=(markTurns+if(move.endsWith("2"))2 else if(move.endsWith("'"))-1 else 1).mod(4)}
                mark(old.first.move);old.partner?.let{mark(it.move)}
                state=Cube.turn(state,old.first.move)
                old.partner?.let{state=Cube.turn(state,it.move)}
                nextStart=started+duration;active=null
            }
            if(queue.isEmpty())return CubeTurnPose()
            val next=queue.removeFirst()
            val partner=takePartner(next)
            active=ActiveVisualTurn(next,partner);started=nextStart;duration=(130_000_000.0/(1+queue.size*.45)).toLong().coerceAtLeast(24_000_000L)
        }
    }
}

@Composable fun CubeCanvas(state:String,modifier:Modifier=Modifier,turns:List<VisualTurn> = emptyList(),orientation:List<Float>?=null,resetGyro:Int=0,crossDown:Char?=null,replayPose:List<Float>?=null,onGyroFrame:((List<Float>,List<Float>)->Unit)?=null) {
    var yaw by remember{mutableStateOf(CUBE_HOME_YAW)};var pitch by remember{mutableStateOf(CUBE_HOME_PITCH)}
    val animator=remember{CubeTurnAnimator(state)}
    var displayed by remember{mutableStateOf(CubeRenderFrame(state))}
    var incoming by remember{mutableStateOf(state)}
    var lastId by remember{mutableStateOf(turns.lastOrNull()?.id?:0L)}
    var basis by remember{mutableStateOf<CubeQuaternion?>(null)}
    var gyro by remember{mutableStateOf(CubeQuaternion())};var gyroTarget by remember{mutableStateOf(CubeQuaternion())}
    val snap=remember{CubeOrientationSnap()}
    val currentOrientation=CubeQuaternion.of(orientation)
    val latestOrientation by rememberUpdatedState(currentOrientation)
    val reportGyro by rememberUpdatedState(onGyroFrame)
    val latestReplay by rememberUpdatedState(replayPose)
    val wake=remember{Channel<Unit>(Channel.CONFLATED)}
    LaunchedEffect(state,turns.lastOrNull()?.id) {
        val fresh=turns.filter{it.id>lastId}
        if(fresh.isNotEmpty()){
            for(event in fresh)animator.enqueue(event.state,event.move,event.cubeMs,event.recovered)
            lastId=fresh.last().id;incoming=state
        }else if(state!=incoming){
            val move=cubeTurnSpecs.keys.flatMap{f->listOf("$f","$f'","${f}2")}.firstOrNull{Cube.turn(incoming,it)==state}
            incoming=state;animator.enqueue(state,move)
        };wake.trySend(Unit)
    }
    LaunchedEffect(currentOrientation,basis,replayPose){
        if(replayPose!=null){gyroTarget=CubeQuaternion.of(replayPose)?:CubeQuaternion()}
        else
        if(currentOrientation==null){basis=null;snap.reset();gyroTarget=CubeQuaternion()}
        else {if(basis==null){basis=currentOrientation;snap.reset()};gyroTarget=snap.update(currentOrientation.relativeTo(basis!!))}
        if(replayPose==null&&currentOrientation!=null)basis?.let{reportGyro?.invoke(it.values(),gyroTarget.values())}
        wake.trySend(Unit)
    }
    val recenter:()->Unit={
        yaw=CUBE_HOME_YAW;pitch=CUBE_HOME_PITCH
        if(latestReplay!=null){gyroTarget=CubeQuaternion.of(latestReplay)?:CubeQuaternion()}
        else{basis=latestOrientation;snap.reset();gyroTarget=CubeQuaternion();basis?.let{reportGyro?.invoke(it.values(),gyroTarget.values())}}
        wake.trySend(Unit)
    }
    val latestRecenter by rememberUpdatedState(recenter)
    LaunchedEffect(resetGyro){latestRecenter()}
    LaunchedEffect(Unit){
        for(signal in wake){
            var previous=0L
            do {
                withFrameNanos {now->
                    val dt=if(previous==0L)1/60f else ((now-previous)/1e9f).coerceIn(0f,.1f);previous=now
                    val pose=animator.advance(now)
                    gyro=if(gyro.closeTo(gyroTarget))gyroTarget else gyro.interpolate(gyroTarget,1-exp(-24f*dt))
                    displayed=CubeRenderFrame(animator.state,pose.move,pose.angle,orientation=gyro,partnerMove=pose.partnerMove,partnerAngle=pose.partnerAngle,logoAngle=-animator.markTurns*PI.toFloat()/2)
                }
            }while(animator.busy||!gyro.closeTo(gyroTarget))
        }
    }
    val gestures=modifier.semantics{contentDescription="Interactive cube. Drag to orbit; double-tap to recenter."}
        .pointerInput(Unit){detectTapGestures(onDoubleTap={latestRecenter()})}
        .pointerInput(Unit){detectDragGestures{change,delta->change.consume();yaw+=delta.x*.008f;pitch=(pitch+delta.y*.008f).coerceIn(-1.4f,1.4f)}}
    NativeCubeSurface(displayed.copy(yaw=yaw,pitch=pitch,orientation=lab.cube.core.crossReplayPose(crossDown?:'D')*displayed.orientation),gestures)
}

@Composable internal expect fun NativeCubeSurface(frame:CubeRenderFrame,modifier:Modifier)
