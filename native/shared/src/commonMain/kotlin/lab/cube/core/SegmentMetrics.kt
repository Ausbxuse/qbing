package lab.cube.core

internal data class SegmentMetrics(val start:Int,val end:Int,val duration:Double,val gap:Double,val execution:Double,val moves:Int,val tps:Double?,val estimated:Boolean)

/** Exact raw boundaries, with the same HTM weights as the complete solve. */
internal fun segmentMetrics(r:AttemptRecord,start:Int,end:Int):SegmentMetrics? {
 if(start<0||end<=start||end>r.moves.size)return null
 val before=if(start==0)0.0 else r.moves[start-1].t
 val first=r.moves[start].t;val last=r.moves[end-1].t
 if(!before.isFinite()||!first.isFinite()||!last.isFinite()||before<0||first<before||last<first)return null
 val duration=last-before;val gap=first-before
 val moves=moveWeights(r.moves.map{it.move}).subList(start,end).sum()
 fun estimatedAt(i:Int):Boolean {
  val event=r.moves[i];if(event.recovered||"clock-reset" in r.quality)return true
  if(!hasEstimatedTurnTiming(r))return false
  val first=r.moves.first();val a=first.raw.cubeTimestamp;val b=event.raw.cubeTimestamp
  return first.recovered||a==null||b==null||!a.isFinite()||!b.isFinite()||kotlin.math.abs(b-a-event.t)>1.0
 }
 val estimated=listOf((start-1).coerceAtLeast(0),start,end-1).any(::estimatedAt)
 return SegmentMetrics(start,end,duration,gap,last-first,moves,if(recordedExecutionMetric(r)!=null&&duration>0)moves*1000/duration else null,estimated)
}
