package lab.cube.core

/** Face turns: one half turn counts once, without cancelling mistakes or reverse turns. */
fun moveWeights(moves:List<String>):List<Int> {
    val weights=MutableList(moves.size){0}
    val pending=mutableMapOf<Char,Pair<Int,String>>()
    var axis:Char?=null
    for((i,move)in moves.withIndex()){
        if(!Regex("[URFDLB](2|')?").matches(move)){pending.clear();axis=null;continue}
        val face=move.first()
        val nextAxis=when(face){'U','D'->'U';'R','L'->'R';'F','B'->'F';else->null}
        if(nextAxis==null||nextAxis!=axis)pending.clear()
        axis=nextAxis
        weights[i]=1
        if(move.endsWith('2')){pending.remove(face);continue}
        val previous=pending[face]
        if(previous!=null&&previous.second==move){weights[previous.first]=0;pending.remove(face)}
        else pending[face]=i to move
    }
    return weights
}
fun countMoves(moves:List<MoveSample>):Int=moveWeights(moves.map{it.move}).sum()
