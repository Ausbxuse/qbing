package lab.cube.core

import kotlin.test.*

class ProtocolRecoveryTest {
    private class Fixture(serial:Int=0) {
        val moves=mutableListOf<CubePacket.Move>()
        val requests=mutableListOf<Pair<Int,Int>>()
        val verified=mutableListOf<Int>()
        val faults=mutableListOf<String>()
        val stream=MoveStream({}, {move,_->moves.add(move)}, {faults.add(it)}, {s,n->requests.add(s to n)}, {verified.add(it)})
        init {stream.accept(CubePacket.State(serial,Cube.solved),0.0)}
        fun move(s:Int,m:String,clock:Double?,at:Double=0.0)=stream.accept(CubePacket.Move(s,m,clock,clock,clock==null),at)
    }
    @Test fun recoveryPreservesBufferedLiveClock() {
        val f=Fixture(209)
        f.move(211,"U",492877.0)
        f.move(211,"U",null)
        f.move(210,"R",null)
        assertEquals(listOf(210,211),f.moves.map{it.serial})
        assertTrue(f.moves[0].recovered)
        assertFalse(f.moves[1].recovered)
        assertEquals(492877.0,f.moves[1].cubeMs)
        assertEquals(Cube.apply(Cube.solved,listOf("R","U")),f.stream.state)
        assertFalse(f.stream.expire(2000.0))
    }
    @Test fun delayedLivePacketUpgradesBufferedRecoveredCopy() {
        val f=Fixture()
        f.move(3,"F",300.0)
        f.move(2,"U",null)
        f.move(2,"U",200.0)
        f.move(1,"R",100.0)
        assertEquals(3,f.moves.size)
        assertTrue(f.moves.all{!it.recovered&&it.cubeMs!=null})
    }
    @Test fun aheadSnapshotAllowsLiveTurnToArriveFirst() {
        val f=Fixture()
        f.stream.accept(CubePacket.State(1,Cube.turn(Cube.solved,"R")),0.0)
        f.stream.poll(499.0)
        assertTrue(f.requests.isEmpty())
        f.move(1,"R",100.0,30.0)
        f.stream.poll(1000.0)
        assertTrue(f.requests.isEmpty())
        assertEquals(listOf(1),f.verified)
    }
    @Test fun lostEvenFinalTurnUsesInclusiveAlignedHistoryWindow() {
        val f=Fixture(1)
        val target=Cube.turn(Cube.solved,"U")
        f.stream.accept(CubePacket.State(2,target),0.0)
        assertFalse(f.stream.expire(500.0))
        assertEquals(listOf(3 to 2),f.requests)
        f.move(3,"F",null,510.0)
        assertTrue(f.moves.isEmpty())
        f.move(2,"U",null,520.0)
        assertEquals(target,f.stream.state)
        assertEquals(listOf(2),f.verified)
    }
    @Test fun zeroSerialSnapshotNeverInventsWrappedMove() {
        val f=Fixture(255)
        f.stream.accept(CubePacket.State(0,Cube.turn(Cube.solved,"D")),0.0)
        f.stream.poll(600.0)
        assertTrue(f.requests.isEmpty())
        f.move(1,"D",null)
        assertTrue(f.moves.isEmpty())
        assertEquals(255,f.stream.serial)
    }
    @Test fun fullEvenSnapshotWindowRecoversInTwoBoundedRequests() {
        val f=Fixture()
        f.stream.accept(CubePacket.State(16,Cube.solved),0.0)
        f.stream.poll(500.0)
        assertEquals(16 to 16,f.requests.last())
        for(serial in 15 downTo 1)f.move(serial,"R",null,510.0)
        assertEquals(15,f.stream.serial)
        f.stream.poll(520.0)
        assertEquals(17 to 2,f.requests.last())
        f.move(16,"R",null,530.0)
        assertEquals(Cube.solved,f.stream.state)
        assertEquals(listOf(16),f.verified)
    }
    @Test fun unansweredGapRetriesWithBoundedFrequency() {
        val f=Fixture()
        f.move(2,"U",200.0)
        assertEquals(listOf(2 to 2),f.requests)
        f.stream.poll(199.0)
        assertEquals(1,f.requests.size)
        f.stream.poll(200.0)
        assertEquals(2,f.requests.size)
    }
}
