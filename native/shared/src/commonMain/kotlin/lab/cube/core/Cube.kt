package lab.cube.core

data class Sticker(val p: List<Int>, val n: List<Int>, val face: Char)
data class PairSlot(val id: String, val corner: List<Int>, val edge: List<Int>)
data class CubeProgress(val cross: Boolean, val pairs: Map<String, Boolean>, val f2l: Boolean, val oll: Boolean, val solved: Boolean)

object Cube {
    const val faces = "URFDLB"
    val solved = faces.map { it.toString().repeat(9) }.joinToString("")
    val opposite = mapOf('U' to 'D','D' to 'U','R' to 'L','L' to 'R','F' to 'B','B' to 'F')
    val geometry = buildList {
        val normals=listOf(listOf(0,1,0),listOf(1,0,0),listOf(0,0,1),listOf(0,-1,0),listOf(-1,0,0),listOf(0,0,-1))
        for(f in 0..5) for(row in 0..2) for(col in 0..2) {
            val p=when(f){0->listOf(col-1,1,row-1);1->listOf(1,1-row,1-col);2->listOf(col-1,1-row,1);3->listOf(col-1,-1,1-row);4->listOf(-1,1-row,col-1);else->listOf(1-col,1-row,-1)}
            add(Sticker(p,normals[f],faces[f]))
        }
    }
    val edges = geometry.indices.groupBy { geometry[it].p }.values.filter { it.size==2 }
    val corners = geometry.indices.groupBy { geometry[it].p }.values.filter { it.size==3 }
    private fun rotate(v: List<Int>,axis: Int,sign: Int): List<Int> = when(axis){0->listOf(v[0],-sign*v[2],sign*v[1]);1->listOf(sign*v[2],v[1],-sign*v[0]);else->listOf(-sign*v[1],sign*v[0],v[2])}
    private val specifications = mapOf('R' to listOf(0,1,-1),'L' to listOf(0,-1,1),'U' to listOf(1,1,-1),'D' to listOf(1,-1,1),'F' to listOf(2,1,-1),'B' to listOf(2,-1,1),'x' to listOf(0,9,-1),'y' to listOf(1,9,-1),'z' to listOf(2,9,-1))
    val permutations: Map<Char, IntArray> = specifications.mapValues { (_,spec)->
        val (axis,layer,sign)=spec;val p=IntArray(54){it}
        geometry.forEachIndexed { i,g-> if(layer==9||g.p[axis]==layer){val position=rotate(g.p,axis,sign);val normal=rotate(g.n,axis,sign);val destination=geometry.indexOfFirst {it.p==position&&it.n==normal};p[destination]=i} };p
    }
    fun turn(state: String, move: String): String {
        require(Regex("[URFDLBxyz](2|')?").matches(move)){"Invalid move: $move"}
        val perm=permutations.getValue(move[0]);var s=state
        repeat(if(move.endsWith('2'))2 else if(move.endsWith('\''))3 else 1){s=perm.map {s[it]}.joinToString("")};return s
    }
    fun apply(state: String,moves: List<String>): String = moves.fold(state){s,m->turn(s,m)}
    fun inverse(moves: List<String>) = moves.asReversed().map {if(it.endsWith('2'))it else if(it.endsWith('\''))it.dropLast(1) else "$it'"}
    fun simplify(moves: List<String>): List<String> {
        val out=mutableListOf<String>();fun amount(m:String)=if(m.endsWith('2'))2 else if(m.endsWith('\''))3 else 1
        for(m in moves){val last=out.lastOrNull();if(last?.firstOrNull()!=m.first()){out.add(m);continue};val n=(amount(m)+amount(last))%4;out.removeAt(out.lastIndex);if(n!=0)out.add("${m[0]}${if(n==2)"2" else if(n==3)"'" else ""}")};return out
    }
    fun centers(s: String) = listOf(4,13,22,31,40,49).map{s[it]}.joinToString("")
    private val orientations: List<Pair<String,List<String>>> by lazy {
        val result=mutableListOf(solved to emptyList<String>());val seen=mutableSetOf(faces);var i=0
        while(i<result.size){val (s,moves)=result[i++];for(r in listOf("x","y","z")){val next=turn(s,r);if(seen.add(centers(next)))result.add(next to (moves+r))}};result
    }
    fun downFrame(state: String,cross: Char): String {
        val rotation=orientations.first {it.first[31]==cross}.second;val s=apply(state,rotation);val c=centers(s);return s.map {faces[c.indexOf(it)]}.joinToString("")
    }
    fun reframe(state: String,rotations: List<String>): String {val s=apply(state,rotations);val c=centers(s);return s.map{faces[c.indexOf(it)]}.joinToString("")}
    fun slots(cross: Char) = corners.filter {ids->ids.any {solved[it]==cross}}.map {corner->val sides=corner.map{solved[it]}.filter{it!=cross}.sorted().joinToString("");PairSlot(sides,corner,edges.first{e->e.map{solved[it]}.sorted().joinToString("")==sides})}
    fun pieceSolved(state: String,piece: List<Int>)=piece.all {state[it]==solved[it]}
    fun progress(state: String,cross: Char='U'): CubeProgress {
        val crossDone=edges.filter {e->e.any{solved[it]==cross}}.all {pieceSolved(state,it)}
        val pairs=slots(cross).associate {it.id to (pieceSolved(state,it.corner)&&pieceSolved(state,it.edge))}
        val f2l=crossDone&&pairs.values.all{it};val top=opposite.getValue(cross);val start=faces.indexOf(top)*9
        return CubeProgress(crossDone,pairs,f2l,f2l&&state.substring(start,start+9).all{it==top},state==solved)
    }
    fun goal(state: String,stage: String,cross: Char='U'): Boolean {val p=progress(state,cross);return when(stage){"Cross"->p.cross;"F2L"->p.f2l;"OLL"->p.oll;else->p.solved}}
    val cornerMap=listOf(listOf(8,9,20),listOf(6,18,38),listOf(0,36,47),listOf(2,45,11),listOf(29,26,15),listOf(27,44,24),listOf(33,53,42),listOf(35,17,51))
    val edgeMap=listOf(listOf(5,10),listOf(7,19),listOf(3,37),listOf(1,46),listOf(32,16),listOf(28,25),listOf(30,43),listOf(34,52),listOf(23,12),listOf(21,41),listOf(50,39),listOf(48,14))
    fun fromCubies(cp: List<Int>,co: List<Int>,ep: List<Int>,eo: List<Int>): String {
        require(cp.sorted()==(0..7).toList()&&ep.sorted()==(0..11).toList())
        require(co.all{it in 0..2}&&co.sum()%3==0&&eo.all{it in 0..1}&&eo.sum()%2==0)
        val state=solved.toCharArray()
        for(i in 0..7)for(p in 0..2)state[cornerMap[i][(p+co[i])%3]]=solved[cornerMap[cp[i]][p]]
        for(i in 0..11)for(p in 0..1)state[edgeMap[i][(p+eo[i])%2]]=solved[edgeMap[ep[i]][p]]
        return state.concatToString().also {require(valid(it))}
    }
    fun valid(s: String): Boolean {
        if(s.length!=54||faces.any{f->s.count{it==f}!=9}||centers(s)!=faces)return false
        val cp=mutableListOf<Int>();val co=mutableListOf<Int>();val ep=mutableListOf<Int>();val eo=mutableListOf<Int>()
        for(ids in cornerMap){val c=ids.map{s[it]};val o=c.indexOfFirst{it=='U'||it=='D'};if(o<0)return false;val p=cornerMap.indexOfFirst{home->home.indices.all{solved[home[it]]==c[(it+o)%3]}};if(p<0)return false;cp.add(p);co.add(o)}
        for(ids in edgeMap){val c=ids.map{s[it]};var p=-1;var o=0;for(j in edgeMap.indices)for(k in 0..1)if(edgeMap[j].indices.all{solved[edgeMap[j][it]]==c[(it+k)%2]}){p=j;o=k};if(p<0)return false;ep.add(p);eo.add(o)}
        fun parity(p: List<Int>)=p.indices.sumOf{i->p.drop(i+1).count{it<p[i]}}%2
        return cp.distinct().size==8&&ep.distinct().size==12&&co.sum()%3==0&&eo.sum()%2==0&&parity(cp)==parity(ep)
    }
    fun recognize(s: String,stage: String,cross: Char,catalog: Catalog): String? {
        val state=downFrame(s,cross);val p=progress(state,'D')
        return when(stage){"OLL"->if(p.f2l&&!p.oll)catalog.recognition[stage]?.get(state.map{if(it=='U')'1' else '0'}.joinToString("")) else null;"PLL"->if(p.oll&&!p.solved)catalog.recognition[stage]?.get(state) else null;else->null}
    }
}
