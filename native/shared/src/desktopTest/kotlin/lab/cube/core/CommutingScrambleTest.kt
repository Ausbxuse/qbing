package lab.cube.core

import kotlin.test.*

class CommutingScrambleTest {
    @Test fun oppositeFacesAcceptEveryOrderAndDirection() {
        fun visit(state:String,setup:List<String>,a:String,b:String,leftA:Int,leftB:Int) {
            val progress=scrambleProgress(state,setup)
            assertTrue(progress.matched,"Legal interleaving rejected: $setup; $leftA, $leftB")
            if(leftA>0){
                val next=Cube.turn(state,a)
                assertEquals(emptyList(),scrambleCorrection(state,next,a,setup,emptyList()))
                visit(next,setup,a,b,leftA-1,leftB)
            }
            if(leftB>0){
                val next=Cube.turn(state,b)
                assertEquals(emptyList(),scrambleCorrection(state,next,b,setup,emptyList()))
                visit(next,setup,a,b,leftA,leftB-1)
            }
            if(leftA==0&&leftB==0){
                assertEquals(setOf(0,1,2),progress.done)
                assertEquals(3,progress.completed)
                assertFalse(progress.partial)
            }
        }
        for((a,b,prefix) in listOf(Triple("U","D","F"),Triple("R","L","U"),Triple("F","B","U"))){
            for(sa in listOf("","'","2"))for(sb in listOf("","'","2")){
                val setup=listOf(prefix,a+sa,b+sb)
                for(da in listOf("","'"))for(db in listOf("","'")){
                    visit(Cube.turn(Cube.solved,prefix),setup,
                        if(sa=="2")a+da else a+sa,if(sb=="2")b+db else b+sb,
                        if(sa=="2")2 else 1,if(sb=="2")2 else 1)
                }
            }
        }
    }

    @Test fun interleavedHalfTurnsReportIndependentStatusAndExactRecovery() {
        val setup="R U2 D2 F".split(' ')
        val state=Cube.apply(Cube.solved,"R D U' D".split(' '))
        val p=scrambleProgress(state,setup)
        assertTrue(p.matched)
        assertEquals(1,p.completed)
        assertEquals(setOf(0,2),p.done)
        assertEquals(setOf(1),p.partialMoves)
        assertEquals(setOf(1),p.nextMoves)
        assertEquals(listOf("U'"),p.finish)
        val wrong=Cube.turn(state,"L")
        val recovery=scrambleCorrection(state,wrong,"L",setup,emptyList())!!
        assertEquals(listOf("L'"),recovery)
        assertEquals(state,Cube.apply(wrong,recovery))
        val both=scrambleProgress(Cube.apply(Cube.solved,"R D U'".split(' ')),setup)
        assertEquals(setOf(1,2),both.partialMoves)
        assertEquals(setOf(1,2),both.nextMoves)
        assertEquals(listOf("U'","D"),both.finish)
    }

    @Test fun oppositeFaceProgressDoesNotCrossNoncommutingTurn() {
        val setup="R U D F B L".split(' ')
        val state=Cube.apply(Cube.solved,listOf("R","D"))
        val p=scrambleProgress(state,setup)
        assertEquals(setOf(0,2),p.done)
        assertEquals(setOf(1),p.nextMoves)
        val wrong=Cube.turn(state,"F")
        assertFalse(scrambleProgress(wrong,setup).matched)
        assertEquals(listOf("F'"),scrambleCorrection(state,wrong,"F",setup,emptyList()))
        val resumed=Cube.apply(wrong,listOf("F'","U"))
        assertEquals(setOf(3,4),scrambleProgress(resumed,setup).nextMoves)
        assertFalse(scrambleProgress(Cube.turn(Cube.solved,"U"),setup).matched)
    }
}
