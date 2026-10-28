package lab.cube.core

data class NativeLiveMetrics(val moves:Int?,val tps:Double?,val estimatedStart:Boolean)
/** One elapsed snapshot supplies clock, total rate and the unfinished phase. */
private val nativeSensorMove=Regex("[URFDLB](2|')?")
fun nativeLiveMetrics(moves:List<MoveSample>,quality:Collection<String>,elapsed:Double):NativeLiveMetrics{
 val count=if(moves.isNotEmpty()&&quality.all{it=="estimated-timing"||it=="clock-reset"}&&moves.all{nativeSensorMove.matches(it.move)})countMoves(moves)else null
 val rate=if(count!=null&&quality.all{it=="estimated-timing"}&&elapsed.isFinite()&&elapsed>0&&moves.last().t<=elapsed+1)count*1000.0/elapsed else null
 val first=moves.firstOrNull();val estimated=first?.let{it.recovered||it.raw.cubeTimestamp?.isFinite()!=true}?:false
 return NativeLiveMetrics(count,rate,estimated)
}
