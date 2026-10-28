package lab.cube.core

import kotlin.test.*
import java.io.File
import kotlinx.serialization.json.*

class CoreTest {
    @Test fun draftsHaveStableIdentityAndDoNotContaminateCompletion(){
        val a=ActiveAttempt(Cube.turn(Cube.solved,"R"),"solve","solve","U","Test")
        val draft=a.record(catalog,"interrupted","app-closed");val complete=a.record(catalog)
        assertEquals(draft.id,complete.id);assertEquals(draft.createdAt,complete.createdAt)
        assertTrue("app-closed" in draft.quality);assertTrue(complete.quality.isEmpty())
    }
    @Test fun scrambleSpeedIncludesCorrectionTurns(){
        val tracker=ScrambleTracker();var state=Cube.solved
        for((i,move) in listOf("R","F","F'","U").withIndex()){
            val next=Cube.turn(state,move);tracker.observe(state,next,CubePacket.Move(i,move,1000.0+i*200,1000.0+i*200),1000.0+i*200);state=next
        }
        assertEquals(4,tracker.stats().moves);assertEquals(600.0,tracker.stats().duration)
        assertEquals(4*1000.0/600,tracker.stats().tps)
        assertEquals(1,scrambleProgress(Cube.turn(Cube.solved,"R"),listOf("R","U2")).completed)
        assertTrue(scrambleProgress(Cube.apply(Cube.solved,listOf("R","U'")),listOf("R","U2")).partial)
    }
    private val directory=File("src/commonMain/composeResources/files")
    private val catalog by lazy {WireJson.decodeFromString<Catalog>(File(directory,"catalog.json").readText())}
    @Test fun independentMoveFixtures(){
        val fixtures=WireJson.parseToJsonElement(File(directory,"cube-oracle.json").readText()).jsonArray
        for(item in fixtures){val o=item.jsonObject;val alg=o.getValue("alg").jsonPrimitive.content.split(' ');val state=o.getValue("facelets").jsonPrimitive.content;assertEquals(state,Cube.apply(Cube.solved,alg));assertTrue(Cube.valid(state));assertEquals(Cube.solved,Cube.apply(state,Cube.inverse(alg)))}
    }
    @Test fun everyCaseAndCrossColor(){
        assertEquals(125,catalog.cases.size)
        for(item in catalog.cases)for(variant in item.variants)for((cross,p) in variant.positions){assertTrue(Cube.valid(p.state),"${item.id}/$cross");assertEquals(p.state,Cube.apply(Cube.solved,p.setup));assertEquals(Cube.solved,Cube.apply(p.state,p.solution));assertFalse(Cube.goal(p.state,item.stage,cross.first()),item.id);if(item.stage in listOf("OLL","PLL"))assertEquals(item.id,Cube.recognize(p.state,item.stage,cross.first(),catalog))}
    }
    @Test fun impossibleStates(){val s=Cube.solved.toCharArray();val t=s[5];s[5]=s[10];s[10]=t;assertFalse(Cube.valid(s.concatToString()));assertFalse(Cube.valid("U".repeat(54)));assertFalse(Cube.valid(Cube.solved.drop(1)))}
    @Test fun ganOverlappingAesVector(){
        val cipher=GanCipher("12:34:56:78:9A:BC");val plain=ByteArray(20).apply{this[0]=0xdd.toByte();this[1]=4;this[3]=0xed.toByte()}
        val expected="9294f283805a5d11623b6663e66c5ad021727f85".chunked(2).map{it.toInt(16).toByte()}.toByteArray()
        assertContentEquals(expected,cipher.encrypt(plain));assertContentEquals(plain,cipher.decrypt(expected))
        assertContentEquals(expected,GanProtocol("12:34:56:78:9A:BC"){}.command("state"))
    }
    @Test fun packedStateAndMoves(){
        val emitted=mutableListOf<CubePacket>();val decoder=GanProtocol("12:34:56:78:9A:BC"){emitted.add(it)}
        val state=ByteArray(20).apply{this[0]=0xed.toByte();this[1]=14}
        fun put(offset:Int,width:Int,value:Int){for(i in 0 until width){val pos=offset+i;if(value and (1 shl (width-1-i))!=0)state[pos/8]=(state[pos/8].toInt()or (1 shl (7-pos%8))).toByte()}}
        for(i in 0..6)put(32+3*i,3,i);for(i in 0..10)put(69+4*i,4,i)
        decoder.decode(state,1000.0);assertEquals(CubePacket.State(0,Cube.solved),emitted.last())
        val move=ByteArray(20).apply{this[0]=1;this[1]=7;this[2]=0x40;this[3]=0xe2.toByte();this[4]=1;this[6]=1;this[8]=0x20}
        decoder.decode(move,2000.0);val m=emitted.last() as CubePacket.Move;assertEquals("R",m.move);assertEquals(123456.0,m.cubeMs);assertEquals(1,m.serial)
    }
    @Test fun serialRecoveryAndSnapshots(){
        val moves=mutableListOf<String>();val faults=mutableListOf<String>();val stream=MoveStream({}, {m,_->moves.add(m.move)},{faults.add(it)},{_,_->})
        stream.accept(CubePacket.State(254,Cube.solved),0.0)
        val expected=Cube.apply(Cube.solved,listOf("R","U"))
        stream.accept(CubePacket.State(256,expected),10.0)
        stream.accept(CubePacket.Move(256,"U",20.0,20.0),20.0)
        stream.accept(CubePacket.Move(255,"R",null,null,true),30.0)
        stream.accept(CubePacket.Move(256,"U",20.0,20.0),40.0)
        assertEquals(listOf("R","U"),moves);assertEquals(expected,stream.state);assertTrue(faults.isEmpty());assertFalse(stream.expire(5000.0))
        stream.accept(CubePacket.Move(2,"F",30.0,30.0),50.0);assertTrue(stream.expire(1500.0));assertTrue(faults.contains("missing-moves"));assertNull(stream.state)
    }
    @Test fun cueTimingIsSeparateFromExecution(){
        val item=catalog.cases.first{it.id=="pll-t"};val p=item.variants.first().positions.getValue("U");val attempt=ActiveAttempt(p.state,"drill","PLL","U","Test",item.id,cue=true,readyAt=1000.0)
        var state=p.state;for((i,m) in p.solution.withIndex()){state=Cube.turn(state,m);attempt.move(CubePacket.Move(i,m,5000+i*100.0,5000+i*100.0),state,1800+i*100.0)}
        val record=attempt.record(catalog);assertEquals(800.0,record.recognitionMs);assertEquals((p.solution.size-1)*100.0,record.duration);assertEquals("cue",record.recognitionSource);assertEquals(Cube.solved,record.analysis?.finalState)
        val ordinary=ActiveAttempt(p.state,"solve","solve","U","Test",readyAt=1000.0);ordinary.move(CubePacket.Move(1,p.solution[0],10.0,1800.0),Cube.turn(p.state,p.solution[0]),1800.0);assertNull(ordinary.record(catalog,"interrupted").recognitionMs)
    }
    @Test fun dnfAverages(){val r=AttemptRecord("a","2026-10-25T00:00:00Z","solve",duration=1000.0,initialState=Cube.solved);val records=(1..5).map{r.copy(id=it.toString(),duration=it*1000.0)};assertEquals(3000.0,average(records,5));assertEquals(3000.0,average(records.dropLast(1)+records.last().copy(penalty="DNF"),5));assertEquals(Double.POSITIVE_INFINITY,average(records.take(3)+records.takeLast(2).map{it.copy(penalty="DNF")},5))}
}
