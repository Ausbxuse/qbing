package lab.cube.ui

import lab.cube.core.*
import kotlin.test.*

class ScrambleNotationTest {
 @Test fun cursorFollowsRecoveryThenReturnsToTheOriginalScramble(){
  val setup=listOf("R","U","F");val checkpoint=Cube.turn(Cube.solved,"R")
  val progress=scrambleProgress(checkpoint,setup);val repairs=listOf("B'","L'")
  var state=Cube.apply(checkpoint,listOf("L","B"))
  for(i in repairs.indices){
   val tokens=scrambleDisplayTokens(setup,progress,repairs.drop(i))
   assertEquals(listOf(repairs[i]),tokens.filter{it.second==2}.map{it.first})
   assertEquals(1,tokens.first{it.first=="R"}.second)
   assertEquals(0,tokens.first{it.first=="U"}.second)
   state=Cube.turn(state,repairs[i])
  }
  assertEquals(checkpoint,state)
  assertEquals("U",scrambleDisplayTokens(setup,progress,emptyList()).first{it.second==2}.first)
 }
 @Test fun earlyOppositeTurnDimsWhileTheFirstOutstandingMoveKeepsTheOnlyCursor(){
  val setup="R U2 D2 F".split(' ')
  for((raw,expected)in listOf("R" to listOf(1,2,0,0),"R D" to listOf(1,2,0,0),"R D2" to listOf(1,2,1,0),"R D2 U" to listOf(1,4,1,0),"R D2 U2" to listOf(1,1,1,2))){
   val progress=scrambleProgress(Cube.apply(Cube.solved,raw.split(' ')),setup)
   assertTrue(progress.matched,raw)
   val tokens=scrambleDisplayTokens(setup,progress,emptyList())
   assertEquals(expected,tokens.map{it.second},raw)
   assertEquals(1,tokens.count{it.second==2||it.second==4})
  }
 }
 @Test fun partialAndOppositeFaceHighlightsResumeAfterRecovery(){
  val setup="R U2 D2 F".split(' ');val progress=scrambleProgress(Cube.apply(Cube.solved,listOf("R","U")),setup)
  val repairing=scrambleDisplayTokens(setup,progress,listOf("L'"))
  assertEquals(listOf("L'" to 2),repairing.filter{it.second in listOf(2,4)})
  val resumed=scrambleDisplayTokens(setup,progress,emptyList())
  assertEquals(4,resumed.first{it.first=="U2"}.second)
  assertEquals(0,resumed.first{it.first=="D2"}.second)
 }
}
