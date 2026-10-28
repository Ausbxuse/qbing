package lab.cube.core

import kotlin.test.*

class SliceNotationTest {
    private fun record(moves:List<String>,times:List<Double> = moves.indices.map{it*20.0},quality:List<String> = emptyList()) = AttemptRecord(
        "notation-fixture","2026-10-26T00:00:00Z","solve",duration=times.lastOrNull()?.takeIf{it.isFinite()}?:0.0,
        initialState=Cube.solved,moves=moves.mapIndexed{i,move->MoveSample(move,times[i])},quality=quality)

    @Test fun allThreeSliceAxesPrimesAndReversedSensorOrder() {
        for((raw,notation)in listOf(
            listOf("R","L'") to "M",listOf("R'","L") to "M'",
            listOf("U","D'") to "E",listOf("U'","D") to "E'",
            listOf("F'","B") to "S",listOf("F","B'") to "S'")) {
            for(moves in listOf(raw,raw.reversed())) {
                val group=reconstruct(record(moves)).single()
                assertEquals(notation,group.notation,moves.toString())
                assertEquals(0,group.rawStart);assertEquals(2,group.rawEnd)
                assertTrue(group.inferred);assertFalse(group.recovered)
            }
        }
    }

    @Test fun doubleSlicesUseTheLongestBalancedPrefixWithoutLosingInterleavedEvents() {
        for((moves,notation)in listOf(
            listOf("R","L'","R","L'") to "M2",
            listOf("R","R","L'","L'") to "M2",
            listOf("R","R","L","L") to "M2",
            listOf("R'","R'","L'","L'") to "M2",
            listOf("L'","L'","R","R") to "M2",
            listOf("R'","L","R'","L") to "M2",
            listOf("U","D'","U","D'") to "E2",
            listOf("F'","B","F'","B") to "S2",
            listOf("R2","L2") to "M2",
            listOf("D2","U2") to "E2",
            listOf("F2","B2") to "S2",
            listOf("R2","L'","L'") to "M2",
            listOf("R","R","L2") to "M2")) {
            val group=reconstruct(record(moves)).single()
            assertEquals(notation,group.notation,moves.toString())
            assertEquals(0,group.rawStart);assertEquals(moves.size,group.rawEnd)
            assertTrue(group.inferred)
        }
        val five=reconstruct(record(listOf("R","L'","R","L'","R")))
        assertEquals(listOf("M2","R"),five.map{it.notation})
        assertEquals(listOf(0 to 4,4 to 5),five.map{it.rawStart to it.rawEnd})
    }

    @Test fun oneHundredMillisecondsBoundsAdjacentPacketsInsteadOfTotalBurst() {
        assertEquals("M",reconstruct(record(listOf("R","L'"),listOf(40.0,140.0))).single().notation)
        assertEquals(listOf("R","L'"),reconstruct(record(listOf("R","L'"),listOf(40.0,141.0))).map{it.notation})
        // The pair gaps remain bounded even when the total burst exceeds 100 ms.
        val longGroup=reconstruct(record(listOf("R","L'","R","L'"),listOf(0.0,30.0,80.0,120.0)))
        assertEquals(listOf("M2"),longGroup.map{it.notation})
        assertEquals(listOf(0 to 4),longGroup.map{it.rawStart to it.rawEnd})
    }

    @Test fun groupedHalfSlicePacketsStillRecognizeAllThreeAxesAtNormalSpeed() {
        for((raw,expected)in listOf(listOf("R","R","L'","L'") to "M2",listOf("U","U","D'","D'") to "E2",listOf("F'","F'","B","B") to "S2")){
            val groups=reconstruct(record(raw,listOf(0.0,60.0,120.0,180.0)))
            assertEquals(expected,groups.single().notation)
            assertEquals(0 to 4,groups.single().rawStart to groups.single().rawEnd)
            assertEquals(180.0,groups.single().elapsed)
            assertEquals(raw,reconstruct(record(raw,raw.indices.map{it*101.0})).map{it.notation})
        }
    }

    @Test fun identicalQuarterTurnsMayBecomeDoublesButCancellationNeverRemovesEvents() {
        for(raw in listOf(listOf("R","R"),listOf("R'","R'"))) {
            val group=reconstruct(record(raw)).single()
            assertEquals("R2",group.notation);assertFalse(group.inferred)
            assertEquals(0 to 2,group.rawStart to group.rawEnd)
        }
        for(raw in listOf(listOf("R","R'","L'"),listOf("R","L","L'"),listOf("R","U","L'"))) {
            val groups=reconstruct(record(raw))
            assertEquals(raw,groups.map{it.notation})
            assertTrue(groups.all{it.rawEnd-it.rawStart==1&&!it.inferred})
        }
        val undo=reconstruct(record(listOf("R","L'","R'","L")))
        assertEquals(listOf("M","M'"),undo.map{it.notation})
        assertEquals(listOf(0 to 2,2 to 4),undo.map{it.rawStart to it.rawEnd})
    }

    @Test fun untrustedTimingRemainsVisibleWithoutInventingSlices() {
        for(times in listOf(listOf(50.0,40.0),listOf(Double.NaN,40.0),listOf(0.0,Double.POSITIVE_INFINITY),listOf(-1.0,40.0))) {
            val groups=reconstruct(record(listOf("R","L'"),times))
            assertEquals(listOf("R","L'"),groups.map{it.notation})
            assertTrue(groups.none{it.inferred})
        }
        for(flag in listOf("clock-reset","missing-moves")) {
            val groups=reconstruct(record(listOf("R","L'"),quality=listOf(flag)))
            assertEquals(listOf("R","L'"),groups.map{it.notation});assertTrue(groups.none{it.inferred})
        }
        val base=record(listOf("R","L'","R","L'"),listOf(0.0,20.0,200.0,220.0))
        val recovered=base.copy(moves=base.moves.mapIndexed{i,m->m.copy(recovered=i==1)},quality=listOf("estimated-timing"))
        val groups=reconstruct(recovered)
        assertEquals(listOf("R","L'","M"),groups.map{it.notation})
        assertTrue(groups[1].recovered);assertFalse(groups[1].inferred)
        assertTrue(groups[2].inferred);assertFalse(groups[2].recovered)
        val invalid=reconstruct(record(listOf("R"),listOf(Double.NaN))).single()
        assertNull(invalid.start);assertNull(invalid.elapsed)
    }

    @Test fun sliceFrameChangesRenameSubsequentRawFacesIncludingPrimesAndDoubles() {
        for((raw,expected)in listOf(
            listOf("R","L'","B") to listOf("M","U"),
            listOf("U","D'","F") to listOf("E","R"),
            listOf("F'","B","L") to listOf("S","U"),
            listOf("R'","L","F'") to listOf("M'","U'"),
            listOf("U'","D","B2") to listOf("E'","R2"),
            listOf("F","B'","R") to listOf("S'","U"),
            listOf("R2","L2","D") to listOf("M2","U"),
            listOf("U2","D2","L") to listOf("E2","R"),
            listOf("F2","B2","D") to listOf("S2","U"))) {
            val groups=reconstruct(record(raw,listOf(0.0,20.0,200.0)))
            assertEquals(expected,groups.map{it.notation},raw.toString())
            assertEquals(2 to 3,groups.last().rawStart to groups.last().rawEnd)
            assertFalse(groups.last().inferred)
        }
    }

    @Test fun sequentialSliceFramesComposeRatherThanResetToTheLastAxis() {
        // After M, raw B/F' are logical U/D'. After that E, raw L is logical F.
        val mixed=reconstruct(record(listOf("R","L'","B","F'","L"),listOf(0.0,20.0,200.0,220.0,400.0)))
        assertEquals(listOf("M","E","F"),mixed.map{it.notation})
        assertEquals(listOf(0 to 2,2 to 4,4 to 5),mixed.map{it.rawStart to it.rawEnd})
        val four=record(List(4){listOf("R","L'")}.flatten()+"U",listOf(0.0,20.0,200.0,220.0,400.0,420.0,600.0,620.0,900.0))
        assertEquals(listOf("M","M","M","M","U"),reconstruct(four).map{it.notation})
    }

    @Test fun rawSpansTimesStateAndExecutionTpsStayAuthoritative() {
        val input=record(listOf("R","L'","B","U","U"),listOf(0.0,25.0,300.0,600.0,630.0))
        val original=input.copy(moves=input.moves.toList())
        val metric=recordedExecutionMetric(input)
        val rawState=Cube.apply(input.initialState,input.moves.map{it.move})
        val groups=reconstruct(input)
        assertEquals(listOf(0 to 2,2 to 3,3 to 5),groups.map{it.rawStart to it.rawEnd})
        assertEquals(listOf(0.0,300.0,600.0),groups.map{it.start})
        assertEquals(listOf(25.0,300.0,630.0),groups.map{it.elapsed})
        assertEquals(input.moves.indices.toList(),groups.flatMap{(it.rawStart until it.rawEnd).toList()})
        assertEquals(original,input)
        assertEquals(metric,recordedExecutionMetric(input))
        assertEquals(rawState,Cube.apply(input.initialState,input.moves.map{it.move}))
        assertEquals(5,recordedReconstruction(input).size)
        assertEquals(4*1000.0/630.0,metric?.tps,"The F F half turn counts once; all five raw events remain available")
        val raw=reconstruct(input,raw=true)
        assertEquals(input.moves.map{it.move},raw.map{it.notation})
        assertEquals(input.moves.indices.map{it to it+1},raw.map{it.rawStart to it.rawEnd})
        assertTrue(raw.none{it.inferred})
        assertEquals(input.moves.map{it.t},raw.map{it.start})
        assertEquals(input.moves.map{it.t},raw.map{it.elapsed})
    }
}
