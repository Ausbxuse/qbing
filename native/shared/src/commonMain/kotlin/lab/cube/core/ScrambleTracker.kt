package lab.cube.core

class ScrambleTracker {
    private var frozen:ScrambleStats?=null
    private var first:Double?=null
    private var clock:Double?=null
    private var lastAt:Double?=null
    val moves=mutableListOf<MoveSample>()
    val quality=mutableSetOf<String>()
    fun reset(){frozen=null;first=null;clock=null;lastAt=null;moves.clear();quality.clear()}
    fun observe(before:String?,after:String,packet:CubePacket.Move,now:Double=monotonicMs()){
        if(before==Cube.solved){reset();first=now;clock=packet.cubeMs}
        frozen=null
        val start=first?:return
        if(clock==null&&!packet.recovered)clock=estimatedOrigin(moves,packet.cubeMs)
        var time=when {
            packet.recovered&&packet.cubeMs==null->estimatedRecoveredElapsed(moves)
            clock!=null&&packet.cubeMs!=null->packet.cubeMs-clock!!
            else->now-start
        }
        if(time<0||!packet.recovered&&deviceClockRegressed(moves,packet.cubeMs)){quality.add("clock-reset");time=now-start}
        if(!packet.recovered)reflowRecoveredTimes(moves,clock,packet.cubeMs,"clock-reset" in quality)
        if(packet.recovered)quality.add("estimated-timing")
        moves.add(MoveSample(packet.move,maxOf(time,moves.lastOrNull()?.t?:0.0),packet.recovered,RawMove(packet.serial,now-start,packet.cubeMs)))
        lastAt=now
        if(after==Cube.solved)reset()
    }
    fun freeze(){if(frozen==null)frozen=scrambleStats(moves,quality.toList())}
    fun stats()=frozen?:scrambleStats(moves,quality.toList())
    fun liveStats(now:Double=monotonicMs()):ScrambleStats {
        frozen?.let{return it};val saved=stats();val received=lastAt?:return saved
        val duration=saved.duration+(now-received).coerceAtLeast(0.0)
        return saved.copy(duration=duration,tps=if(duration>0&&quality.isEmpty())saved.moves*1000.0/duration else null)
    }
}
fun scrambleStats(moves:List<MoveSample>,quality:List<String>):ScrambleStats {
    val duration=moves.lastOrNull()?.t?:0.0
    val count=countMoves(moves)
    return ScrambleStats(duration,count,if(duration>0&&quality.isEmpty())count*1000/duration else null)
}
data class ScrambleProgress(
    /** First unfinished notation token, not a count of all completed tokens. */
    val completed:Int,
    val partial:Boolean=false,
    val matched:Boolean=true,
    val done:Set<Int> = emptySet(),
    val partialMoves:Set<Int> = emptySet(),
    val nextMoves:Set<Int> = emptySet(),
    /** Quarter turns needed to finish only the half turns already in progress. */
    val finish:List<String> = emptyList(),
)
private data class FaceProgress(val turns:Int=0,val done:Set<Int> = emptySet(),val partial:Pair<Int,String>?=null)
private fun turnAmount(move:String)=if(move.endsWith('2'))2 else if(move.endsWith('\''))3 else 1
private fun turnNotation(face:Char,amount:Int)=face.toString()+when(amount.mod(4)){2->"2";3->"'";else->""}
private fun scrambleAxis(face:Char)=when(face){'U','D'->'U';'R','L'->'R';'F','B'->'F';else->face}
private fun faceProgress(sequence:List<String>,start:Int,end:Int,face:Char):List<FaceProgress> {
    val options=mutableListOf(FaceProgress())
    var turns=0
    val done=mutableSetOf<Int>()
    for(i in start until end){
        val move=sequence[i]
        if(move.first()!=face)continue
        if(move.endsWith('2'))for(quarter in listOf(1,-1)){
            options.add(FaceProgress((turns+quarter).mod(4),done.toSet(),i to turnNotation(face,quarter)))
        }
        turns=(turns+turnAmount(move)).mod(4)
        done.add(i)
        options.add(FaceProgress(turns,done.toSet()))
    }
    return options
}

/** Opposite faces commute within an uninterrupted axis block; each face keeps its own order. */
fun scrambleProgress(state:String,sequence:List<String>):ScrambleProgress {
    var found=ScrambleProgress(0,matched=state==Cube.solved)
    var position=Cube.solved
    var start=0
    var bestScore=0
    while(start<sequence.size){
        val face=scrambleAxis(sequence[start].first())
        val end=(start until sequence.size).firstOrNull{scrambleAxis(sequence[it].first())!=face}?:sequence.size
        val opposite=Cube.opposite[face]
        val firstOptions=faceProgress(sequence,start,end,face)
        val secondOptions=opposite?.let{faceProgress(sequence,start,end,it)}?:listOf(FaceProgress())
        for(first in firstOptions){
            val left=if(first.turns==0)position else Cube.turn(position,turnNotation(face,first.turns))
            for(second in secondOptions){
                val candidate=if(second.turns==0)left else Cube.turn(left,turnNotation(opposite!!,second.turns))
                if(candidate!=state)continue
                val score=(start+first.done.size+second.done.size)*2+(if(first.partial!=null)1 else 0)+(if(second.partial!=null)1 else 0)
                if(found.matched&&score<bestScore)continue
                bestScore=score
                val partials=listOfNotNull(first.partial,second.partial).sortedBy{it.first}
                found=ScrambleProgress(0,partial=partials.isNotEmpty(),done=(0 until start).toSet()+first.done+second.done,partialMoves=partials.map{it.first}.toSet(),finish=partials.map{it.second})
            }
        }
        position=Cube.apply(position,sequence.subList(start,end))
        start=end
    }
    val completed=sequence.indices.firstOrNull{it !in found.done}?:sequence.size
    val next=mutableSetOf<Int>()
    if(found.matched&&completed<sequence.size){
        val axis=scrambleAxis(sequence[completed].first())
        val faces=mutableSetOf<Char>()
        for(i in completed until sequence.size){
            val face=sequence[i].first()
            if(scrambleAxis(face)!=axis)break
            if(i !in found.done&&faces.add(face))next.add(i)
        }
    }
    return found.copy(completed=completed,nextMoves=next)
}

fun simplifyTurns(sequence:List<String>):List<String> {
    fun amount(move:String)=if(move.endsWith('2'))2 else if(move.endsWith('\''))3 else 1
    val result=mutableListOf<String>()
    for(move in sequence){
        if(result.lastOrNull()?.firstOrNull()==move.firstOrNull()){
            val total=(amount(result.removeAt(result.lastIndex))+amount(move))%4
            if(total!=0)result.add(move.take(1)+when(total){2->"2";3->"'";else->""})
        }else result.add(move)
    }
    return result
}

/** Undo only the mistaken turns; the original row still owns an unfinished half turn. */
fun scrambleCorrection(before:String?,after:String,move:String,setup:List<String>,current:List<String>):List<String>? {
    if(scrambleProgress(after,setup).matched)return emptyList()
    if(before==null)return null
    if(current.isNotEmpty())return simplifyTurns(Cube.inverse(listOf(move))+current)
    val progress=scrambleProgress(before,setup)
    if(!progress.matched)return null
    return Cube.inverse(listOf(move))
}


/** Progress stays at the last valid preparation state while inline corrections are being followed. */
fun checkpointProgress(state:String?,checkpoint:String?,setup:List<String>):ScrambleProgress {
    state?.let{scrambleProgress(it,setup).takeIf{it.matched}?.let{return it}}
    checkpoint?.let{scrambleProgress(it,setup).takeIf{it.matched}?.let{return it}}
    return scrambleProgress(Cube.solved,setup)
}
