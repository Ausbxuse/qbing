package lab.cube.core

data class ReconstructionRange(val stage:String,val rawStart:Int,val rawEnd:Int)
fun reconstructionRanges(record:AttemptRecord):List<ReconstructionRange>{
 var cursor=0
 val ranges=mutableListOf<ReconstructionRange>()
 for(split in record.analysis?.splits.orEmpty()){
  val n=split.rawMoves?:return listOf(ReconstructionRange("Recorded",0,record.moves.size))
  if(n<0||cursor+n>record.moves.size)return listOf(ReconstructionRange("Recorded",0,record.moves.size))
  ranges.add(ReconstructionRange(split.stage,cursor,cursor+n));cursor+=n
 }
 return if(cursor!=record.moves.size||ranges.isEmpty())listOf(ReconstructionRange("Recorded",0,record.moves.size))else ranges
}
