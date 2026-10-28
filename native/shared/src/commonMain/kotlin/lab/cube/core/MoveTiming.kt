package lab.cube.core

import kotlin.math.abs

// GAN history packets contain the turns but not their clocks. Match csTimer's
// missing-timestamp cadence; Bluetooth delivery time is never the turn time.
private const val RECOVERED_TURN_INTERVAL_MS=50.0

internal fun estimatedRecoveredElapsed(moves:List<MoveSample>):Double =
    moves.lastOrNull{it.t.isFinite()&&it.t>=0}?.t?.plus(RECOVERED_TURN_INTERVAL_MS)?:0.0

/** A missing start remains estimated, even after a later device clock anchors the timeline. */
internal fun estimatedOrigin(moves:List<MoveSample>,nextClock:Double?):Double? {
    if(nextClock?.isFinite()!=true||moves.isEmpty()||moves.any{!it.recovered||it.raw.cubeTimestamp!=null})return null
    return nextClock-moves.size*RECOVERED_TURN_INTERVAL_MS
}

/** Repair only provisional derived timestamps, never original packet evidence or recovery flags. */
internal fun reflowRecoveredTimes(moves:MutableList<MoveSample>,origin:Double?,nextClock:Double?,clockReset:Boolean){
    if(clockReset||origin?.isFinite()!=true||nextClock?.isFinite()!=true||moves.lastOrNull()?.recovered!=true)return
    val anchorIndex=moves.indexOfLast{!it.recovered}
    if(anchorIndex<0)return
    val anchor=moves[anchorIndex]
    val anchorClock=anchor.raw.cubeTimestamp?.takeIf{it.isFinite()}?:return
    val start=anchorClock-origin;val end=nextClock-origin
    if(start<0||end<start||!anchor.t.isFinite()||abs(anchor.t-start)>1.0)return
    val intervals=moves.size-anchorIndex
    for(i in anchorIndex+1 until moves.size){
        val fraction=(i-anchorIndex).toDouble()/intervals
        moves[i]=moves[i].copy(t=start+(end-start)*fraction)
    }
}

internal fun deviceClockRegressed(moves:List<MoveSample>,clock:Double?):Boolean {
    if(clock?.isFinite()!=true)return false
    val previous=moves.lastOrNull{!it.recovered&&it.raw.cubeTimestamp?.isFinite()==true}?.raw?.cubeTimestamp?:return false
    return clock<previous
}
