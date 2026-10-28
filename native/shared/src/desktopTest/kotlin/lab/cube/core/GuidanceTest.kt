package lab.cube.core

import kotlin.test.*
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.*

class GuidanceTest {
    @Test fun invalidatingQueuedScrambleClearsLoadingBeforeItsRequestRuns() {
        val dispatcher=object:CoroutineDispatcher(){
            override fun dispatch(context:CoroutineContext,block:Runnable) { /* Keep initialization/network jobs queued. */ }
        }
        val scope=CoroutineScope(SupervisorJob()+dispatcher)
        val storage=object:LocalStorage{val values=mutableMapOf<String,String>();override fun read(key:String)=values[key];override fun write(key:String,value:String){values[key]=value}}
        val transport=object:BleTransport{
            override var onBytes:(ByteArray)->Unit={}
            override var onDisconnected:(String)->Unit={}
            override suspend fun scan()=emptyList<CubeDevice>()
            override suspend fun connect(device:CubeDevice){}
            override suspend fun write(bytes:ByteArray){}
            override suspend fun disconnect(){}
        }
        try {
            val controller=LabController(transport,storage,scope)
            controller.prepareScramble();assertTrue(controller.scrambleLoading)
            controller.saveSettings(controller.settings);assertFalse(controller.scrambleLoading);assertNull(controller.target)
            controller.prepareScramble();assertTrue(controller.scrambleLoading)
            controller.calibrate();assertFalse(controller.scrambleLoading);assertNull(controller.target)
        }finally{scope.cancel()}
    }
    @Test fun correctionReturnsToPartialHalfTurnWithoutConsumingTheOriginalToken() {
        val setup="D2 B2 U F2 D R2 F2".split(' ')
        val anchor=Cube.turn(Cube.apply(Cube.solved,setup.take(5)),"R")
        val first=Cube.turn(anchor,"U")
        val plan=scrambleCorrection(anchor,first,"U",setup,emptyList())!!
        val second=Cube.turn(first,"B")
        val correction=scrambleCorrection(first,second,"B",setup,plan)!!
        assertEquals(listOf("B'","U'"),correction)
        assertEquals(anchor,Cube.apply(second,correction))
        val mistaken=Cube.turn(second,"F")
        val corrected=scrambleCorrection(second,mistaken,"F",setup,correction)!!
        assertEquals(listOf("F'","B'","U'"),corrected)
        assertEquals(anchor,Cube.apply(mistaken,corrected))
        assertTrue(scrambleProgress(anchor,setup).partial)
    }
    @Test fun checkpointPreservesCommutingProgressAndInsertsAtTheEarliestUnfinishedToken(){
        val setup=listOf("U2","D","R")
        val checkpoint=Cube.apply(Cube.solved,listOf("D","U"))
        val wrong=Cube.turn(checkpoint,"F")
        val marks=checkpointProgress(wrong,checkpoint,setup)
        assertEquals(setOf(1),marks.done)
        assertEquals(setOf(0),marks.partialMoves)
        assertEquals(0,marks.completed)
        val correction=scrambleCorrection(checkpoint,wrong,"F",setup,emptyList())!!
        assertEquals(listOf("F'"),correction)
        val repaired=Cube.apply(wrong,correction)
        assertEquals(checkpoint,repaired)
        assertEquals(marks,checkpointProgress(repaired,checkpoint,setup))
        assertEquals(setOf(0,1),checkpointProgress(Cube.turn(repaired,"U"),checkpoint,setup).done)
        assertEquals(emptySet(),checkpointProgress(wrong,null,setup).done)
    }
    @Test fun remoteHalfTurnCorrectionAdvancesByOnePhysicalQuarter() {
        val state=Cube.turn(Cube.solved,"F")
        val after=Cube.turn(state,"R")
        val result=scrambleCorrection(state,after,"R",listOf("B"),listOf("R2","U"))!!
        assertEquals(listOf("R","U"),result)
        assertEquals(Cube.apply(state,listOf("R2","U")),Cube.apply(after,result))
    }
    @Test fun gyroDecodeMapsAxesAndRejectsZeroQuaternion() {
        val packets=mutableListOf<CubePacket>()
        val protocol=GanProtocol("12:34:56:78:9A:BC"){packets.add(it)}
        val bytes=ByteArray(20).apply{this[0]=0xec.toByte();this[1]=10}
        listOf(0x4000,0xc000,0x4000,0xc000).forEachIndexed{i,n->bytes[2+i*2]=(n shr 8).toByte();bytes[3+i*2]=n.toByte()}
        protocol.decode(bytes,0.0)
        val q=(packets.single() as CubePacket.Orientation).quaternion
        q.zip(listOf(-.5f,-.5f,-.5f,.5f)).forEach{(actual,expected)->assertEquals(expected,actual,.00001f)}
        packets.clear();bytes.fill(0,2,10);protocol.decode(bytes,0.0);assertTrue(packets.isEmpty())
    }
    @Test fun delayedSnapshotVerifiesPreviousSolveWithoutResettingNextScramble() {
        val verified=mutableListOf<Int>();val faults=mutableListOf<String>()
        val stream=MoveStream({}, {_,_->}, {faults.add(it)}, {_,_->}, {verified.add(it)})
        stream.accept(CubePacket.State(254,Cube.turn(Cube.solved,"R")),0.0)
        stream.accept(CubePacket.Move(255,"R'",100.0,100.0),100.0)
        stream.accept(CubePacket.Move(0,"U",200.0,200.0),200.0)
        stream.accept(CubePacket.State(255,Cube.solved),250.0)
        assertEquals(listOf(255),verified)
        assertEquals(Cube.turn(Cube.solved,"U"),stream.state)
        assertTrue(faults.isEmpty())
    }
}
