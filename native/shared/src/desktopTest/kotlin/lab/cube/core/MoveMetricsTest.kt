package lab.cube.core

import kotlin.test.*

class MoveMetricsTest {
    @Test fun halfTurnsAreOneMoveWithoutCancellingMistakesOrCrossingAxes(){
        val cases=mapOf("R R" to 1,"R' R'" to 1,"R2" to 1,"R R'" to 2,"R R R" to 2,
            "R L R L" to 2,"R R L L" to 2,"R U R" to 3,"R R2 R" to 3,"R L'" to 2)
        for((notation,count)in cases)assertEquals(count,moveWeights(notation.split(' ')).sum(),notation)
        assertEquals(listOf(0,0,1,1),moveWeights(listOf("R","L","R","L")))
        val slow=listOf(MoveSample("R",0.0),MoveSample("R",10000.0))
        assertEquals(1,countMoves(slow),"Move notation does not depend on turning speed")
    }
    @Test fun rotationsHaveZeroWeightAndNeverChangeRecordedMoveTotalsOrTps(){
        val rotations=listOf("x","x'","x2","y","y'","y2","z","z'","z2")
        assertEquals(List(9){0},moveWeights(rotations))
        assertEquals(listOf(1,0,1,0,1),moveWeights(listOf("R","x","U","y2","F2")))
        val moves=listOf(MoveSample("R",0.0),MoveSample("U",1000.0))
        val base=AttemptRecord("rotation-count","","solve",duration=1000.0,initialState=Cube.solved,moves=moves)
        val rotated=base.copy(rotations=rotations.mapIndexed{i,m->RotationSample(m,i*100.0,listOf(0f,0f,0f,1f))})
        assertEquals(2,recordedMoveCount(rotated));assertEquals(recordedMoveCount(base),recordedMoveCount(rotated))
        assertEquals(recordedExecutionMetric(base),recordedExecutionMetric(rotated));assertEquals(2.0,recordedExecutionMetric(rotated)!!.tps)
        assertEquals(11,reconstructionWithRotations(rotated).size)
        assertEquals(2,countMoves(rotated.moves))
    }
    @Test fun solvePhaseAndScrambleUseTheSameHalfTurnMetricAndPreserveRawSamples(){
        val moves=listOf(MoveSample("R",0.0),MoveSample("R",1000.0))
        val raw=AttemptRecord("half","2026-10-25T00:00:00Z","solve",duration=1000.0,
            initialState=Cube.turn(Cube.solved,"R2"),moves=moves)
        val catalog=Catalog(emptyList(),emptyMap())
        val result=refreshAnalysis(raw,catalog)
        assertEquals(moves,result.moves)
        assertEquals(1,result.analysis!!.moves)
        assertEquals(1,result.analysis!!.splits.sumOf{it.moves})
        assertEquals(2,result.analysis!!.splits.sumOf{it.rawMoves!!})
        assertEquals(1.0,result.analysis!!.tps)
        assertEquals(RecordedExecutionMetric(1.0,false),recordedExecutionMetric(result))
        assertEquals(ScrambleStats(1000.0,1,1.0),scrambleStats(moves,emptyList()))
    }
}
