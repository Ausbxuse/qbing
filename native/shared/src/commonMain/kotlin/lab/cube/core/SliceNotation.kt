package lab.cube.core

/** A display-only interpretation. The original sensor events remain authoritative. */
data class NotationItem(
    val notation:String,
    val rawStart:Int,
    val rawEnd:Int,
    val start:Double?,
    val elapsed:Double?,
    val inferred:Boolean=false,
    val recovered:Boolean=false,
    val rotation:Boolean=false,
)

private data class NotationTurn(val face:Char,val amount:Int) {
    val direction:Int get()=if(amount==3)-1 else if(amount==1)1 else 0
    val quarters:Int get()=if(amount==2)2 else 1
}
private data class SliceGroup(val end:Int,val symbol:Char,val amount:Int)
private val sensorTurnPattern=Regex("[URFDLB](2|')?")
private val sliceFrames=mapOf('M' to "BRUFLD",'E' to "UFLDBR",'S' to "LUFRDB")
private fun notationTurn(text:String,frame:String):NotationTurn? {
    if(!sensorTurnPattern.matches(text))return null
    val face=Cube.faces[frame.indexOf(text[0])]
    return NotationTurn(face,if(text.endsWith('2'))2 else if(text.endsWith('\''))3 else 1)
}
private fun notationText(face:Char,amount:Int)=face.toString()+when(amount){2->"2";3->"'";else->""}
private fun axis(face:Char)=when(face){'R','L'->0;'U','D'->1;else->2}
private fun eventTime(move:MoveSample)=move.t.takeIf{it.isFinite()&&it>=0.0}

/**
 * Opposite sensor turns can represent a slice in a rotated notation frame.
 * Infer only bounded, reliable bursts; never simplify or replace the recorded moves.
 * Setting raw=true exposes the exact sensor sequence with one row per event.
 */
fun reconstruct(record:AttemptRecord,raw:Boolean=false,initialFrame:String=Cube.faces):List<NotationItem> {
    val moves=record.moves
    val grouping=!raw&&record.quality.none{it=="clock-reset"||it=="missing-moves"}
    var frame=initialFrame
    var i=0
    val result=mutableListOf<NotationItem>()

    fun trustedWindow(start:Int,end:Int):Boolean {
        if(end>moves.size||start>=end)return false
        val first=eventTime(moves[start])?:return false
        var previous=first
        for(index in start until end){
            val move=moves[index];val time=eventTime(move)?:return false
            if(move.recovered||time<previous||time-previous>100.0)return false
            previous=time
        }
        return true
    }
    fun sliceGroup(start:Int):SliceGroup? {
        val first=notationTurn(moves[start].move,frame)?:return null
        val a=axis(first.face);val firstFace="RUF"[a];val secondFace="LDB"[a]
        val counts=IntArray(2);val directions=IntArray(2)
        var best:SliceGroup?=null
        for(end in start until minOf(moves.size,start+4)){
            if(!trustedWindow(start,end+1))break
            val turn=notationTurn(moves[end].move,frame)?:break
            if(axis(turn.face)!=a)break
            val side=if(turn.face==firstFace)0 else if(turn.face==secondFace)1 else break
            if(turn.direction!=0&&directions[side]!=0&&turn.direction!=directions[side])break
            if(turn.direction!=0)directions[side]=turn.direction
            counts[side]+=turn.quarters
            if(counts[side]>2)break
            if(counts[0]==0||counts[0]!=counts[1])continue
            // Two half-turns have no handedness. Quarter-turn bursts must oppose.
            if(counts[0]==1&&directions[0]==directions[1])continue
            val amount=if(counts[0]==2)2 else {
                val baseDirection=if(a==2)-1 else 1
                if(directions[0]==baseDirection)1 else 3
            }
            best=SliceGroup(end+1,"MES"[a],amount)
        }
        return best
    }
    while(i<moves.size){
        val turn=notationTurn(moves[i].move,frame)
        val slice=if(grouping)sliceGroup(i)else null
        var end=i+1
        var text=if(raw)moves[i].move else turn?.let{notationText(it.face,it.amount)}?:moves[i].move
        if(slice!=null){
            end=slice.end;text=notationText(slice.symbol,slice.amount)
        }else if(grouping&&turn!=null&&turn.amount!=2&&trustedWindow(i,i+2)){
            val next=notationTurn(moves[i+1].move,frame)
            if(next==turn){end=i+2;text=notationText(turn.face,2)}
        }
        result.add(NotationItem(text,i,end,eventTime(moves[i]),eventTime(moves[end-1]),slice!=null,
            moves.subList(i,end).any{it.recovered}))
        if(slice!=null)repeat(slice.amount){
            frame=sliceFrames.getValue(slice.symbol).map{frame[Cube.faces.indexOf(it)]}.joinToString("")
        }
        i=end
    }
    return result
}

/** A gyro row seeks to the surrounding raw turn; it never becomes a counted move. */
fun reconstructionWithRotations(record:AttemptRecord,raw:Boolean=false,viewFrame:Boolean=false):List<NotationItem> =
 (reconstruct(record,raw,if(viewFrame&&!raw)reconstructionFrame(reconstructionCross(record))else Cube.faces)+record.rotations.filter{it.t.isFinite()&&it.t>=0&&it.notation.split(" ").all{m->m in setOf("x","x'","x2","y","y'","y2","z","z'","z2")}}.map{r->
  val index=record.moves.takeWhile{it.t<=r.t}.size
  NotationItem(r.notation,index,index,r.t,r.t,rotation=true)
 }).sortedBy{it.elapsed?:Double.POSITIVE_INFINITY}

internal fun reconstructionFrame(face:Char):String=when(face){'U'->"DLFURB";'F'->"BRUFLD";'B'->"FRDBLU";'R'->"LUFRDB";'L'->"RDFLUB";else->Cube.faces}
