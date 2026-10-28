package lab.cube.core

import kotlin.test.*

class GoalSettleTest {
    private val catalog=Catalog(emptyList(),emptyMap())
    private fun packet(serial:Int,move:String,time:Double,recovered:Boolean=false)=CubePacket.Move(serial,move,time,time,recovered)

    @Test fun transientSolvedHalfSliceContinuesTheSameRawAttempt(){
        val initial=Cube.turn(Cube.solved,"R'")
        val attempt=ActiveAttempt(initial,"solve","solve","U","test",readyAt=0.0)
        val gate=GoalSettle();val first=packet(254,"R",1000.0)
        assertTrue(attempt.move(first,Cube.solved,100.0))
        gate.offer(first,attempt.moves.last().t,100.0)
        assertFalse(gate.ready(219.9))
        val partner=packet(255,"L'",1080.0)
        assertTrue(gate.continuesWith(partner,180.0))
        gate.clear()
        val state=Cube.turn(Cube.solved,"L'")
        assertFalse(attempt.move(partner,state,180.0))
        assertNull(gate.candidate)
        assertEquals(listOf("R","L'"),attempt.moves.map{it.move})
        assertEquals(80.0,attempt.record(catalog,"interrupted").duration)
        assertTrue(attempt.move(packet(0,"L",1500.0),Cube.solved,600.0))
        gate.offer(packet(0,"L",1500.0),attempt.moves.last().t,600.0)
        assertTrue(gate.ready(720.0))
        assertEquals(500.0,attempt.record(catalog).duration,"The quiet settle interval must never enter solve duration")
    }
    @Test fun earlySnapshotVerificationDoesNotBypassQuietSettle(){
        val gate=GoalSettle();val final=packet(255,"R",1000.0)
        gate.offer(final,1234.0,10.0)
        gate.verify(254);assertFalse(gate.candidate!!.verified)
        gate.verify(255);assertTrue(gate.candidate!!.verified)
        assertFalse(gate.ready(129.9));assertTrue(gate.ready(130.0))
        assertEquals(1234.0,gate.candidate!!.elapsed)
        // Even a verified transient goal must be discarded if the slice partner follows.
        assertTrue(gate.continuesWith(packet(0,"L'",1020.0),30.0))
        gate.clear();assertNull(gate.candidate)
    }
    @Test fun unrelatedOrSlowNextTurnFinalizesTheCandidateFirst(){
        val gate=GoalSettle();val final=packet(12,"R",1000.0)
        val cases=listOf(
            packet(13,"U",1030.0),packet(13,"F",1030.0),packet(13,"L'",1101.0),
            packet(14,"L'",1030.0),packet(13,"L'",999.0))
        for(next in cases){
            gate.offer(final,250.0,0.0)
            assertFalse(gate.continuesWith(next,30.0))
            assertEquals(final,gate.take()!!.packet)
            assertNull(gate.candidate)
        }
        gate.offer(final,250.0,0.0)
        assertFalse(gate.continuesWith(packet(13,"L'",1030.0),120.0),"Already-settled goals cannot be reopened by delivery delays")
    }
    @Test fun allAxesAndReverseOrderUseTrustedDeviceTime(){
        for((first,next)in listOf("R" to "L'","L'" to "R","U'" to "D","D" to "U'","F" to "B'","B'" to "F")){
            val gate=GoalSettle();gate.offer(packet(255,first,1000.0),80.0,5000.0)
            assertTrue(gate.continuesWith(packet(0,next,1100.0),5100.0))
        }
        val recovered=GoalSettle();recovered.offer(packet(0,"R",1000.0,true),0.0,0.0)
        assertTrue(recovered.continuesWith(packet(1,"L'",1020.0),20.0))
        assertFalse(recovered.continuesWith(packet(1,"L'",1020.0),101.0))
    }
    @Test fun cancellationClearsCandidateWhileKeepingInterruptedRawTurns(){
        val initial=Cube.turn(Cube.solved,"R'")
        val attempt=ActiveAttempt(initial,"solve","solve","U","test",readyAt=0.0)
        val final=packet(2,"R",200.0);attempt.move(final,Cube.solved,200.0)
        val gate=GoalSettle();gate.offer(final,0.0,200.0);gate.clear()
        val interrupted=attempt.record(catalog,"interrupted","connection-lost")
        assertEquals("interrupted",interrupted.status)
        assertEquals(listOf("R"),interrupted.moves.map{it.move})
        assertTrue("connection-lost" in interrupted.quality)
        assertFalse(gate.ready(1000.0))
    }
    @Test fun sameFaceFirstM2BurstDoesNotFinishAtItsTemporarySolvedState(){
        val initial=Cube.turn(Cube.solved,"R'")
        val attempt=ActiveAttempt(initial,"solve","solve","U","test",readyAt=0.0)
        val gate=GoalSettle();var state=initial
        for((i,move)in listOf("R","R","L'","L'").withIndex()){
            val packet=packet(i,move,1000.0+i*25)
            if(gate.candidate!=null){assertTrue(gate.continuesWith(packet,100.0+i*25));gate.clear()}
            state=Cube.turn(state,move)
            if(attempt.move(packet,state,100.0+i*25))gate.offer(packet,attempt.moves.last().t,100.0+i*25)
        }
        assertNull(gate.candidate)
        assertNotEquals(Cube.solved,state)
        assertEquals(listOf("R","R","L'","L'"),attempt.moves.map{it.move})
        assertEquals(75.0,attempt.record(catalog,"interrupted").duration)
    }
    @Test fun doubleROvershootKeepsTheAttemptRunningUntilTheActualFinalTurn(){
        val initial=Cube.turn(Cube.solved,"R'")
        val attempt=ActiveAttempt(initial,"solve","solve","U","test",readyAt=0.0)
        val gate=GoalSettle();val first=packet(20,"R",1000.0)
        assertTrue(attempt.move(first,Cube.solved,100.0));gate.offer(first,0.0,100.0)
        val second=packet(21,"R",1050.0)
        assertTrue(gate.continuesWith(second,150.0));gate.clear()
        assertFalse(attempt.move(second,Cube.turn(Cube.solved,"R"),150.0))
        assertFalse(gate.ready(500.0))
        val final=packet(22,"R'",1500.0)
        assertTrue(attempt.move(final,Cube.solved,600.0));gate.offer(final,attempt.moves.last().t,600.0)
        assertTrue(gate.ready(720.0))
        assertEquals(listOf("R","R","R'"),attempt.moves.map{it.move})
        assertEquals(500.0,attempt.record(catalog).duration)
    }
    @Test fun recoveredBurstUsesLocalGuardWithoutLosingEstimatedTiming(){
        val initial=Cube.turn(Cube.solved,"R'")
        val attempt=ActiveAttempt(initial,"solve","solve","U","test",readyAt=0.0)
        val first=CubePacket.Move(8,"R",null,null,true)
        val partner=CubePacket.Move(9,"L'",null,null,true)
        assertTrue(attempt.move(first,Cube.solved,100.0))
        val gate=GoalSettle();gate.offer(first,attempt.moves.last().t,100.0)
        assertTrue(gate.continuesWith(partner,110.0));gate.clear()
        assertFalse(attempt.move(partner,Cube.turn(Cube.solved,"L'"),110.0))
        val record=attempt.record(catalog,"interrupted")
        assertEquals(listOf("R","L'"),record.moves.map{it.move})
        assertTrue(record.moves.all{it.recovered})
        assertTrue("estimated-timing" in record.quality)
        val unknown=GoalSettle();unknown.offer(CubePacket.Move(20,"R",null,100.0),0.0,100.0)
        assertTrue(unknown.continuesWith(CubePacket.Move(21,"R",null,130.0),130.0))
        assertFalse(unknown.continuesWith(CubePacket.Move(21,"R",null,230.0),230.0))
    }
}
