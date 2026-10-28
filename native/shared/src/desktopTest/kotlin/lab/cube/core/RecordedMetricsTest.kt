package lab.cube.core

import kotlin.test.*

class RecordedMetricsTest {
    private fun record()=AttemptRecord("sample","2026-10-25T12:00:00Z","solve",duration=1000.0,
        initialState=Cube.solved,moves=listOf(MoveSample("R",0.0),MoveSample("R'",1000.0)))

    @Test fun missingAnalysisUsesRawExecutionAndIgnoresScoringPenalties(){
        for(penalty in listOf("","+2","DNF")){
            val metric=recordedExecutionMetric(record().copy(penalty=penalty))!!
            assertEquals(2.0,metric.tps);assertFalse(metric.estimated)
        }
    }
    @Test fun recoveredBleTimingIsExplicitlyEstimated(){
        val recovered=record().copy(quality=listOf("estimated-timing"))
        assertEquals(RecordedExecutionMetric(2.0,true),recordedExecutionMetric(recovered))
        assertTrue(recordedExecutionMetric(record().copy(moves=record().moves.map{it.copy(recovered=true)}))!!.estimated)
    }
    @Test fun durationEstimateRemainsVisibleWhenStatisticsRejectTheRecording(){
        val recovered=record().copy(quality=listOf("estimated-timing","unverified-final-state"),
            moves=record().moves.mapIndexed{i,m->m.copy(recovered=i==1)})
        assertNull(recordedExecutionMetric(recovered))
        assertTrue(executionDurationEstimated(recovered))
        assertTrue(executionDurationEstimated(record().copy(quality=listOf("clock-reset"))))
        assertFalse(executionDurationEstimated(record().copy(source="manual",moves=emptyList())))
    }
    @Test fun unflaggedInvalidDeviceClocksCannotClaimPreciseDuration(){
        fun withClocks(clocks:List<Double>)=record().copy(moves=clocks.mapIndexed{i,clock->
            MoveSample(listOf("R","U","F","D")[i],i*1000.0/(clocks.size-1),raw=RawMove(cubeTimestamp=clock))})
        val precise=withClocks(listOf(1000.0,1500.0,1500.0,2000.0))
        assertFalse(executionDurationEstimated(precise))
        for(clock in listOf(1400.0,-1.0,Double.NaN,Double.POSITIVE_INFINITY)){
            val damaged=withClocks(listOf(1000.0,1500.0,clock,2000.0))
            assertTrue(damaged.quality.isEmpty())
            assertTrue(executionDurationEstimated(damaged),"Unflagged direct clock $clock")
            assertTrue(recordedExecutionMetric(damaged)!!.estimated)
            val recovered=damaged.copy(moves=damaged.moves.mapIndexed{i,m->if(i==2)m.copy(recovered=true)else m})
            assertFalse(executionDurationEstimated(recovered))
        }
        assertFalse(executionDurationEstimated(precise.copy(source="manual",quality=listOf("clock-reset"))))
    }
    @Test fun manualUnverifiedAndDamagedRecordingsHaveNoExecutionMetric(){
        assertNull(recordedExecutionMetric(record().copy(source="manual")))
        assertNull(recordedExecutionMetric(record().copy(status="interrupted")))
        for(flag in listOf("unverified-final-state","missing-moves","state-mismatch","clock-reset","connection-lost"))
            assertNull(recordedExecutionMetric(record().copy(quality=listOf(flag))),flag)
        for(duration in listOf(0.0,-1.0,Double.NaN,Double.POSITIVE_INFINITY,500.0))
            assertNull(recordedExecutionMetric(record().copy(duration=duration)))
        assertNull(recordedExecutionMetric(record().copy(moves=listOf(MoveSample("R",800.0),MoveSample("R'",200.0)))))
    }
    @Test fun rawTimelineRetainsAllTurnsAndMarksUnusableGaps(){
        val damaged=record().copy(quality=listOf("clock-reset"),moves=listOf(
            MoveSample("R",0.0),MoveSample("U",900.0,recovered=true),MoveSample("F",800.0),MoveSample("L",Double.NaN)))
        val rows=recordedReconstruction(damaged)
        assertEquals(listOf("R","U","F","L"),rows.map{it.move})
        assertEquals(900.0,rows[1].gap);assertTrue(rows[1].recovered)
        assertNull(rows[2].gap);assertNull(rows[3].elapsed);assertNull(rows[3].gap)
        assertEquals(listOf(1,2,3,4),rows.map{it.number})
    }
    @Test fun preciseEndpointsKeepTotalTpsPreciseWhileRecoveredPhaseBoundaryRemainsEstimated(){
        val moves=listOf(
            MoveSample("R",0.0,raw=RawMove(cubeTimestamp=1000.0)),
            MoveSample("U",400.0,recovered=true),
            MoveSample("F",1000.0,raw=RawMove(cubeTimestamp=2000.0)))
        val r=record().copy(moves=moves,quality=listOf("estimated-timing"))
        assertEquals(RecordedExecutionMetric(3.0,false),recordedExecutionMetric(r))
        assertFalse(executionDurationEstimated(r))
        assertFalse(executionDurationEstimated(r.copy(quality=r.quality+"unverified-final-state")))
        assertTrue(hasEstimatedTurnTiming(r))
        assertTrue(recordedExecutionMetric(r.copy(moves=moves.mapIndexed{i,m->if(i==2)m.copy(recovered=true)else m}))!!.estimated)
        assertTrue(recordedExecutionMetric(r.copy(duration=1100.0))!!.estimated)
        val split=Split("Cross",0.0,400.0,400.0,moves=2,rawMoves=2)
        val analysis=Analysis(listOf(split),emptyList(),emptyList(),emptyList(),3,1000.0,0.0,0.0,3.0,Cube.solved,version=4,cross="U")
        assertTrue(splitTimingEstimated(r.copy(analysis=analysis),split))
    }
    @Test fun averagesUseRawAvailableSamplesFromTheSameTrimmedWindow(){
        val records=(1..5).map{i->record().copy(id=i.toString(),duration=i*1000.0,
            moves=listOf(MoveSample("R",0.0),MoveSample("R'",i*1000.0)))}.toMutableList()
        records[2]=records[2].copy(source="manual",moves=emptyList())
        val average=retainedAverage(records,5)
        assertEquals(3000.0,average.time)
        assertEquals(.75,average.tps)
        assertEquals(2.0,average.moves)
        assertEquals(2,average.tpsSamples);assertEquals(2,average.moveSamples);assertEquals(3,average.retained)
        assertFalse(average.estimated)
        records[1]=records[1].copy(quality=listOf("estimated-timing"))
        assertTrue(retainedAverage(records,5).estimated)
        records[1]=records[1].copy(quality=listOf("clock-reset"))
        val clock=retainedAverage(records,5)
        assertEquals(1,clock.tpsSamples);assertEquals(2,clock.moveSamples)
        assertNull(retainedAverage(records,12).tps)
    }
    @Test fun averageTimeCertaintyUsesTheRetainedTimeWindowIndependentlyOfTpsEligibility(){
        val precise=(1..5).map{i->record().copy(id=i.toString(),duration=i*1000.0,moves=listOf(
            MoveSample("R",0.0,raw=RawMove(cubeTimestamp=1000.0)),
            MoveSample("U",i*500.0,recovered=true),
            MoveSample("F",i*1000.0,raw=RawMove(cubeTimestamp=1000.0+i*1000.0))),quality=listOf("estimated-timing"))}
        assertFalse(retainedAverage(precise,5).timeEstimated)
        fun recoveredEnd(r:AttemptRecord)=r.copy(moves=r.moves.mapIndexed{i,m->if(i==2)m.copy(recovered=true)else m},quality=r.quality+"unverified-final-state")
        assertFalse(retainedAverage(precise.mapIndexed{i,r->if(i==0||i==4)recoveredEnd(r)else r},5).timeEstimated)
        val uncertain=retainedAverage(precise.mapIndexed{i,r->if(i==2)recoveredEnd(r)else r},5)
        assertTrue(uncertain.timeEstimated)
        assertFalse(uncertain.estimated) // The uncertain sample is excluded from TPS, but retained in time.
        assertEquals(2,uncertain.tpsSamples)
        assertFalse(retainedAverage(precise.take(4),5).timeEstimated)
        assertFalse(retainedAverage(precise.map{recoveredEnd(it).copy(penalty="DNF")},5).timeEstimated)
    }
    @Test fun moveCountDoesNotDependOnTimingButRejectsMissingAndUnverifiedMoves(){
        assertEquals(2,recordedMoveCount(record().copy(duration=Double.NaN,quality=listOf("clock-reset"))))
        for(flag in listOf("missing-moves","unverified-final-state","state-mismatch"))
            assertNull(recordedMoveCount(record().copy(quality=listOf(flag))))
    }

}
