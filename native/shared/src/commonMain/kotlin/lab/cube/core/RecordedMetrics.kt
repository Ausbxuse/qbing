package lab.cube.core

import kotlin.math.abs
import kotlin.math.ceil

/** Execution uses the physical recording, never the score after a +2 or DNF penalty. */
data class RecordedExecutionMetric(val tps:Double,val estimated:Boolean)

fun hasEstimatedTurnTiming(record:AttemptRecord)=record.quality.any{it=="estimated-timing"||it=="clock-reset"}||record.moves.any{it.recovered}
private fun trustedInterval(record:AttemptRecord,startIndex:Int,endIndex:Int,elapsed:Double):Boolean {
    val a=record.moves.getOrNull(startIndex)?:return false
    val b=record.moves.getOrNull(endIndex)?:return false
    if(a.recovered||b.recovered)return false
    val first=a.raw.cubeTimestamp?.takeIf{it.isFinite()}?:return false
    val last=b.raw.cubeTimestamp?.takeIf{it.isFinite()}?:return false
    return last>=first&&abs((last-first)-elapsed)<=1.0&&abs((b.t-a.t)-elapsed)<=1.0
}
/** Duration certainty is independent of whether the solve is eligible for statistics. */
fun executionDurationEstimated(record:AttemptRecord):Boolean {
    if(record.source=="manual"||record.moves.isEmpty())return false
    if("clock-reset" in record.quality)return true
    var previousClock:Double?=null
    for(move in record.moves){
        if(move.recovered)continue
        val clock=move.raw.cubeTimestamp?:continue
        if(!clock.isFinite()||clock<0||previousClock?.let{clock<it}==true)return true
        previousClock=clock
    }
    if(!hasEstimatedTurnTiming(record))return false
    val first=record.moves.first()
    val last=record.moves.last()
    return !(abs(first.t)<=1.0&&abs(last.t-record.duration)<=1.0&&
        trustedInterval(record,0,record.moves.lastIndex,record.duration))
}

/** Recovered interior timestamps do not make an independently measured total duration approximate. */
fun recordedExecutionMetric(record:AttemptRecord):RecordedExecutionMetric? {
    if(recordedMoveCount(record)==null||record.quality.any{it!="estimated-timing"})return null
    if(!record.duration.isFinite()||record.duration<=0)return null
    if(record.moves.any{!it.t.isFinite()||it.t<0})return null
    if(record.moves.zipWithNext().any{(a,b)->b.t<a.t})return null
    if(record.moves.last().t>record.duration+1.0)return null
    return RecordedExecutionMetric(countMoves(record.moves)*1000.0/record.duration,executionDurationEstimated(record))
}

/** Count validity is independent of timestamp validity. Missing or unverified turns stay excluded. */
fun recordedMoveCount(record:AttemptRecord):Int? {
    if(record.source=="manual"||record.status!="complete"||record.moves.isEmpty())return null
    if(record.quality.any{it!="estimated-timing"&&it!="clock-reset"})return null
    if(record.moves.any{!Regex("[URFDLB](2|')?").matches(it.move)})return null
    return countMoves(record.moves)
}

fun splitTimingEstimated(record:AttemptRecord,split:Split):Boolean {
    if(!hasEstimatedTurnTiming(record))return false
    if("clock-reset" in record.quality)return true
    val splits=record.analysis?.splits?:return true
    val index=splits.indexOfFirst{it.stage==split.stage}
    if(index<0)return true
    val before=splits.take(index).sumOf{it.rawMoves?:it.moves}
    val end=before+(split.rawMoves?:split.moves)-1
    val time=split.time?:return record.moves.getOrNull(if(before==0)0 else before-1)?.let{it.recovered||it.raw.cubeTimestamp?.isFinite()!=true}?:true
    if(time==0.0)return false
    return !trustedInterval(record,if(before==0)0 else before-1,end,time)
}

data class RetainedAverage(val time:Double?=null,val tps:Double?=null,val moves:Double?=null,
    val estimated:Boolean=false,val tpsSamples:Int=0,val moveSamples:Int=0,val retained:Int=0,val timeEstimated:Boolean=false)
/** Sensor averages use available samples from the same time-trimmed window, never older solves. */
fun retainedAverage(records:List<AttemptRecord>,size:Int):RetainedAverage {
    if(size<=0||records.size<size)return RetainedAverage()
    val sorted=records.takeLast(size).sortedBy(::adjusted)
    val trim=if(size>=5)ceil(size*.05).toInt()else 0
    val retained=sorted.subList(trim,sorted.size-trim)
    val eligible=retained.filter{adjusted(it).isFinite()}
    val tps=eligible.mapNotNull(::recordedExecutionMetric)
    val moves=eligible.mapNotNull(::recordedMoveCount)
    return RetainedAverage(retained.map(::adjusted).average(),tps.takeIf{it.isNotEmpty()}?.map{it.tps}?.average(),
        moves.takeIf{it.isNotEmpty()}?.average(),tps.any{it.estimated},tps.size,moves.size,retained.size,retained.map(::adjusted).all{it.isFinite()}&&eligible.any(::executionDurationEstimated))
}

data class ReconstructionTurn(val number:Int,val move:String,val elapsed:Double?,val gap:Double?,val recovered:Boolean)

/** Keep every raw turn visible, even when timing or the final state cannot be trusted. */
fun recordedReconstruction(record:AttemptRecord):List<ReconstructionTurn> = record.moves.mapIndexed { i, move ->
    val elapsed=move.t.takeIf{it.isFinite()&&it>=0}
    val previous=if(i==0)0.0 else record.moves[i-1].t
    val gap=elapsed?.takeIf{previous.isFinite()&&previous>=0&&it>=previous}?.minus(previous)
    ReconstructionTurn(i+1,move.move,elapsed,gap,move.recovered)
}
