package lab.cube.core

import kotlin.test.*

class MoveTimingTest {
    private fun record(packets:List<CubePacket.Move>,arrivals:List<Double>):AttemptRecord {
        val attempt=ActiveAttempt(Cube.solved,"solve","solve","U","test",readyAt=0.0)
        var state=Cube.solved
        for((i,packet)in packets.withIndex()){
            state=Cube.turn(state,packet.move)
            attempt.move(packet,state,arrivals[i])
        }
        return attempt.record(Catalog(emptyList(),emptyMap()))
    }
    @Test fun recoveredFinishExcludesHistoryDeliveryDelay(){
        val packets=listOf(
            CubePacket.Move(0,"R",1000.0,0.0),
            CubePacket.Move(1,"U",1200.0,200.0),
            CubePacket.Move(2,"F",null,null,true),
            CubePacket.Move(3,"D",null,null,true))
        for(arrival in listOf(300.0,1200.0)){
            val r=record(packets,listOf(0.0,200.0,arrival,arrival))
            assertEquals(listOf(0.0,200.0,250.0,300.0),r.moves.map{it.t})
            assertEquals(300.0,r.duration)
            assertTrue(executionDurationEstimated(r))
            assertTrue(recordedExecutionMetric(r)!!.estimated)
            assertTrue("estimated-timing" in r.quality)
            for(move in r.moves.drop(2)){
                assertTrue(move.recovered)
                assertNull(move.raw.cubeTimestamp)
                assertEquals(arrival,move.raw.receivedAt)
            }
        }
    }
    @Test fun aLaterMeasuredClockReplacesEvenAnOvershootingTailEstimate(){
        val r=record(listOf(
            CubePacket.Move(0,"R",1000.0,0.0),
            CubePacket.Move(1,"U",null,null,true),
            CubePacket.Move(2,"F",null,null,true),
            CubePacket.Move(3,"D",1060.0,1000.0)),listOf(0.0,800.0,800.0,1000.0))
        assertEquals(listOf(0.0,20.0,40.0,60.0),r.moves.map{it.t})
        assertEquals(60.0,r.duration)
        assertFalse(executionDurationEstimated(r))
        assertFalse(recordedExecutionMetric(r)!!.estimated)
        assertEquals(800.0,r.moves[1].raw.receivedAt)
    }
    @Test fun aRecoveredStartUsesAnEstimatedOriginWithoutInventingRawClocks(){
        for(arrival in listOf(100.0,1000.0)){
            val r=record(listOf(
                CubePacket.Move(0,"R",null,null,true),
                CubePacket.Move(1,"U",null,null,true),
                CubePacket.Move(2,"F",1200.0,arrival),
                CubePacket.Move(3,"D",1400.0,arrival+200)),listOf(0.0,0.0,arrival,arrival+200))
            assertEquals(listOf(0.0,50.0,100.0,300.0),r.moves.map{it.t})
            assertEquals(300.0,r.duration)
            assertTrue(executionDurationEstimated(r))
            assertTrue(recordedExecutionMetric(r)!!.estimated)
            assertTrue(r.moves.take(2).all{it.recovered&&it.raw.cubeTimestamp==null})
            assertEquals(1200.0,r.moves[2].raw.cubeTimestamp)
            assertEquals(arrival,r.moves[2].raw.receivedAt)
        }
        assertNull(estimatedOrigin(listOf(MoveSample("R",0.0)),1200.0))
        assertNull(estimatedOrigin(listOf(MoveSample("R",0.0,true,RawMove(cubeTimestamp=1100.0))),1200.0))
    }
    @Test fun aWholeRecoveredBurstRemainsEstimatedAndIndependentOfDelivery(){
        val r=record(listOf(
            CubePacket.Move(0,"R",null,null,true),
            CubePacket.Move(1,"U",null,null,true),
            CubePacket.Move(2,"F",null,null,true)),listOf(1000.0,1000.0,1200.0))
        assertEquals(listOf(0.0,50.0,100.0),r.moves.map{it.t})
        assertTrue(executionDurationEstimated(r))
        assertEquals(listOf(1000.0,1000.0,1200.0),r.moves.map{it.raw.receivedAt})
    }
    @Test fun lateHistoryCannotStretchTheNextMeasuredDeviceTime(){
        val attempt=ActiveAttempt(Cube.solved,"solve","solve","U","test",readyAt=0.0)
        val packets=listOf(
            CubePacket.Move(0,"R",1000.0,0.0),
            CubePacket.Move(1,"U",1200.0,200.0),
            CubePacket.Move(2,"F",null,null,true),
            CubePacket.Move(3,"D",1400.0,560.0))
        var state=Cube.solved
        for((i,packet)in packets.withIndex()){
            state=Cube.turn(state,packet.move)
            attempt.move(packet,state,listOf(0.0,200.0,550.0,560.0)[i])
        }
        assertEquals(listOf(0.0,200.0,300.0,400.0),attempt.moves.map{it.t})
        val record=attempt.record(Catalog(emptyList(),emptyMap()))
        assertEquals(400.0,record.duration)
        assertTrue("estimated-timing" in record.quality)
        assertTrue(record.moves[2].recovered)
        assertNull(record.moves[2].raw.cubeTimestamp)
        assertEquals(550.0,record.moves[2].raw.receivedAt)
        assertEquals(RecordedExecutionMetric(10.0,false),recordedExecutionMetric(record))
    }
    @Test fun multipleMissingMovesInterpolateOnlyBetweenTrustedAnchors(){
        val moves=mutableListOf(
            MoveSample("R",0.0,raw=RawMove(cubeTimestamp=1000.0)),
            MoveSample("U",700.0,true,RawMove(receivedAt=700.0)),
            MoveSample("F",710.0,true,RawMove(receivedAt=710.0)))
        val raw=moves.map{it.raw}
        reflowRecoveredTimes(moves,1000.0,1300.0,false)
        assertEquals(listOf(0.0,100.0,200.0),moves.map{it.t})
        assertEquals(raw,moves.map{it.raw})
        assertTrue(moves.drop(1).all{it.recovered})
        for((origin,next,reset)in listOf(Triple(null,1400.0,false),Triple(1000.0,900.0,false),Triple(1000.0,1400.0,true))){
            val copy=moves.toMutableList();reflowRecoveredTimes(copy,origin,next,reset);assertEquals(moves,copy)
        }
        val unanchored=mutableListOf(MoveSample("R",0.0,true),MoveSample("U",100.0,true))
        val original=unanchored.toList();reflowRecoveredTimes(unanchored,1000.0,1400.0,false);assertEquals(original,unanchored)
    }
    @Test fun clockMovingBackwardsWithinTheSolveRemainsAnExplicitFault(){
        val attempt=ActiveAttempt(Cube.solved,"solve","solve","U","test",readyAt=0.0)
        var state=Cube.solved
        for((i,clock)in listOf(1000.0,1400.0,1200.0).withIndex()){
            state=Cube.turn(state,"R")
            attempt.move(CubePacket.Move(i,"R",clock,i*500.0),state,i*500.0)
        }
        assertTrue("clock-reset" in attempt.quality)
        assertNull(recordedExecutionMetric(attempt.record(Catalog(emptyList(),emptyMap()))))
    }
    @Test fun liveScrambleTpsKeepsTickingBetweenPacketsWithoutChangingTheSavedTiming(){
        val tracker=ScrambleTracker()
        val first=Cube.turn(Cube.solved,"R")
        tracker.observe(Cube.solved,first,CubePacket.Move(0,"R",1000.0,0.0),100.0)
        tracker.observe(first,Cube.turn(first,"U"),CubePacket.Move(1,"U",1200.0,0.0),300.0)
        assertEquals(10.0,tracker.stats().tps)
        assertEquals(5.0,tracker.liveStats(500.0).tps)
        assertEquals(200.0,tracker.stats().duration)
        assertEquals(10.0,tracker.stats().tps)
        tracker.reset();assertNull(tracker.liveStats(1000.0).tps)
    }
    @Test fun scramblingUsesTheSameRecoveryAnchors(){
        val tracker=ScrambleTracker();var state=Cube.solved
        for((i,packet)in listOf(
            CubePacket.Move(0,"R",1000.0,0.0),
            CubePacket.Move(1,"U",1200.0,200.0),
            CubePacket.Move(2,"F",null,null,true),
            CubePacket.Move(3,"D",1400.0,560.0)).withIndex()){
            val next=Cube.turn(state,packet.move)
            tracker.observe(state,next,packet,listOf(0.0,200.0,550.0,560.0)[i]);state=next
        }
        assertEquals(listOf(0.0,200.0,300.0,400.0),tracker.moves.map{it.t})
        assertEquals(400.0,tracker.stats().duration)
        assertTrue("estimated-timing" in tracker.quality)
    }
    @Test fun scramblingAlsoExcludesTrailingHistoryDeliveryDelay(){
        for(arrival in listOf(300.0,1200.0)){
            val tracker=ScrambleTracker();var state=Cube.solved
            val packets=listOf(
                CubePacket.Move(0,"R",1000.0,0.0),
                CubePacket.Move(1,"U",1200.0,200.0),
                CubePacket.Move(2,"F",null,null,true))
            for((i,packet)in packets.withIndex()){
                val next=Cube.turn(state,packet.move)
                tracker.observe(state,next,packet,listOf(0.0,200.0,arrival)[i]);state=next
            }
            assertEquals(listOf(0.0,200.0,250.0),tracker.moves.map{it.t})
            assertEquals(250.0,tracker.stats().duration)
            assertEquals(arrival,tracker.moves.last().raw.localTimestamp)
            assertNull(tracker.moves.last().raw.cubeTimestamp)
        }
    }
}
