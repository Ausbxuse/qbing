package lab.cube.core

private fun normalizedCaseMoves(moves:List<String>):List<String>{
 val out=mutableListOf<Pair<Char,Int>>()
 for(move in moves){val face=move.firstOrNull()?:continue;val n=when{move.contains('2')->2;move.endsWith("'")->3;else->1}
  if(out.lastOrNull()?.first==face){val amount=(out.removeAt(out.lastIndex).second+n)%4;if(amount!=0)out.add(face to amount)}else out.add(face to n)
 }
 return out.map{(f,n)->"$f"+when(n){2->"2";3->"'";else->""}}
}
fun localCasePerformance(records:List<AttemptRecord>,case:CubeCase):CasePerformance?{
 val eligible=records.filter{it.kind=="drill"&&it.caseId==case.id&&it.source=="bluetooth"&&adjusted(it).isFinite()&&it.moves.isNotEmpty()&&recordedExecutionMetric(it)?.estimated==false}
 if(eligible.isEmpty())return null
 val groups=eligible.groupBy{record->val raw=normalizedCaseMoves(record.moves.map{it.move});case.variants.indexOfFirst{it.positions[record.cross]?.solution?.let(::normalizedCaseMoves)==raw}.takeIf{it>=0}?.let{case.variants[it].id}?:record.moves.joinToString(" "){it.move}}
 val algorithms=groups.map{(id,rows)->
  val variant=case.variants.indexOfFirst{it.id==id}.takeIf{it>=0}
  val samples=rows.map{r->AlgorithmSample(r.id,r.createdAt,r.duration,r.recognitionMs?.takeIf{r.recognitionSource=="cue"&&it.isFinite()&&it>=0},if(r.recognitionSource=="cue")"cue"else "none")}
  val recognition=samples.mapNotNull{it.recognition}
  AlgorithmPerformance(id,case.id,variant?.let{"Variant ${it+1}"}?:"Recorded sequence",variant?.plus(1),variant?.let{case.variants[it].text}?:id,samples.size,samples.map{it.execution}.average(),median(samples.map{it.execution})!!,recognition.takeIf{it.isNotEmpty()}?.average(),recognition.size,samples=samples)
 }
 return CasePerformance(case.id,case.name,case.stage,eligible.size,algorithms)
}
data class CaseSummary(val count:Int,val meanExecution:Double?,val meanRecognition:Double?,val recognitionCount:Int,val samples:List<AlgorithmSample>,val meanTransition:Double?,val transitionCount:Int)
fun caseSummary(performance:CasePerformance):CaseSummary{
 val rows=performance.algorithms;val n=rows.sumOf{it.count};val recognized=rows.sumOf{it.recognitionCount};val transitions=rows.sumOf{it.transitionCount}
 return CaseSummary(n,if(n>0)rows.sumOf{it.meanExecution*it.count}/n else null,if(recognized>0)rows.sumOf{(it.meanRecognition?:0.0)*it.recognitionCount}/recognized else null,recognized,rows.flatMap{it.samples}.sortedBy{it.createdAt},if(transitions>0)rows.sumOf{(it.meanTransition?:0.0)*it.transitionCount}/transitions else null,transitions)
}
