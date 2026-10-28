package lab.cube.core
import kotlin.math.*

enum class GraphMetric(val label:String,val unit:String,val minimum:Double){
 Time("Time","s",100.0),Tps("TPS","tps",.1),Moves("Moves","moves",1.0);
 fun value(r:AttemptRecord):Double? {
  if(!adjusted(r).isFinite()||adjusted(r)<0)return null
  return when(this){Time->adjusted(r);Tps->recordedExecutionMetric(r)?.tps;Moves->recordedMoveCount(r)?.toDouble()}
 }
 fun format(n:Double):String=when(this){Time->formatTime(n);Tps->decimal(n,2);Moves->decimal(n,if(abs(n-round(n))<.001)0 else 1)}
 fun axis(n:Double)=if(this==Time)decimal(n/1000,1)else format(n)
 private fun decimal(n:Double,digits:Int):String {
  val scale=10.0.pow(digits).toLong();val rounded=round(n*scale).toLong()
  return if(digits==0)rounded.toString()else "${rounded/scale}.${(rounded%scale).toString().padStart(digits,'0')}"
 }
}
internal data class GraphSeries(val slots:List<Double?>,val mean:Double?,val best:Double?){
 fun histogram(metric:GraphMetric,width:Float)=nativeHistogramValues(slots.filterNotNull(),slots.size,width,metric.minimum)
}
internal fun graphSeries(records:List<AttemptRecord>,metric:GraphMetric):GraphSeries {
 val slots=records.map(metric::value);val valid=slots.filterNotNull()
 return GraphSeries(slots,valid.takeIf{it.isNotEmpty()}?.average(),if(metric==GraphMetric.Tps)valid.maxOrNull()else valid.minOrNull())
}
