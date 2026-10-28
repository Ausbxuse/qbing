package lab.cube.core
import kotlin.math.*
data class NativeDistribution(val start:Double=0.0,val step:Double=0.0,val bins:List<Int> = emptyList(),val count:Int=0,val median:Double?=null,val spread:Double?=null,val p90:Double?=null,val dnfRate:Double?=null)
fun nativeDistribution(records:List<AttemptRecord>,targetBins:Int=8):NativeDistribution{
 return nativeDistributionValues(records.map(::adjusted).filter{it.isFinite()&&it>=0},records.size,targetBins,100.0)
}
private fun nativeDistributionValues(values:List<Double>,total:Int,targetBins:Int,minStep:Double):NativeDistribution {
 val times=values.sorted();val n=times.size
 if(n==0)return NativeDistribution(dnfRate=if(total==0)null else 1.0)
 val raw=max(minStep,(times.last()-times.first())/targetBins.coerceAtLeast(1));val power=10.0.pow(floor(log10(raw)));val step=listOf(1,2,5,10).first{it*power>=raw}*power;val start=floor(times.first()/step)*step
 val bins=MutableList(floor((times.last()-start)/step).toInt()+1){0};for(t in times)bins[floor((t-start)/step).toInt()]++
 val mean=times.average()
 return NativeDistribution(start,step,bins,n,(times[(n-1)/2]+times[n/2])/2,if(n>1)sqrt(times.sumOf{(it-mean).pow(2)}/n)else null,times[ceil(n*.9).toInt()-1],if(total==0)null else (total-n).toDouble()/total)
}
expect fun nativeCreatedLabel(iso:String):String

fun nativeHistogram(records:List<AttemptRecord>,width:Float):NativeDistribution {
 return nativeHistogramValues(records.map(::adjusted).filter{it.isFinite()&&it>=0},records.size,width,100.0)
}
internal fun nativeHistogramValues(values:List<Double>,total:Int,width:Float,minStep:Double):NativeDistribution {
 val target=minOf((width/44).toInt().coerceIn(2,12),ceil(sqrt(values.size.toDouble())).toInt().coerceAtLeast(2))
 var d=nativeDistributionValues(values,total,target,minStep)
 while(d.bins.size>=3){
  var run=0;var longest=0;for(n in d.bins){run=if(n==0)run+1 else 0;longest=maxOf(longest,run)}
  if(longest==0||longest*width/d.bins.size<=32f)break
  val step=d.step*2;val start=floor(d.start/step)*step;val last=d.start+d.step*(d.bins.size-1)
  val bins=MutableList(floor((last-start)/step).toInt()+1){0}
  for((i,n)in d.bins.withIndex())bins[floor((d.start+d.step*i-start)/step).toInt()]+=n
  d=d.copy(start=start,step=step,bins=bins)
 }
 return d
}

/** Session-order ao5; the current solve and its preceding four, before filtering. */
fun rollingAo5(records:List<AttemptRecord>):List<Double?> = records.indices.map { index ->
 if(index<4)null else records.subList(index-4,index+1).map(::adjusted).sorted().subList(1,4).average()
}
