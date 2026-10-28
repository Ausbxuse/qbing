package lab.cube.core

/** Origin plus every displayed turn/gyro event, preserving raw spans and equal timestamps. */
internal data class ReplayPosition(val rawIndex:Int,val time:Double)
internal fun replayPositions(record:AttemptRecord,raw:Boolean=false):List<ReplayPosition> =
 listOf(ReplayPosition(0,0.0))+reconstructionWithRotations(record,raw,viewFrame=true).map{
  ReplayPosition(it.rawEnd,it.elapsed?.takeIf{t->t.isFinite()&&t>=0}?:record.moves.getOrNull(it.rawEnd-1)?.t?.takeIf{t->t.isFinite()&&t>=0}?:0.0)
 }
internal fun adjacentReplayPosition(positions:List<ReplayPosition>,current:ReplayPosition,cursor:Int?,forward:Boolean):Int {
 val exact=cursor?.takeIf{positions.getOrNull(it)==current}
 if(exact!=null)return if(forward)(exact+1).coerceAtMost(positions.lastIndex)else(exact-1).coerceAtLeast(0)
 return if(forward)positions.indexOfFirst{it.time>current.time||it.time==current.time&&it.rawIndex>current.rawIndex}.takeIf{it>=0}?:positions.lastIndex
 else positions.indexOfLast{it.time<current.time||it.time==current.time&&it.rawIndex<current.rawIndex}.coerceAtLeast(0)
}
