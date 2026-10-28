package lab.cube.core

import java.io.File
import kotlinx.coroutines.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*
import kotlin.test.*

class CrossDetectionTest {
    private val catalog by lazy {
        WireJson.decodeFromString<Catalog>(File("src/commonMain/composeResources/files/catalog.json").readText())
    }
    private val solution="D R U R' U' R U R' U R U2 R' R U R' U' R' F R2 U' R' U' R U R' F'".split(' ')
    private val initial by lazy { Cube.apply(Cube.solved,Cube.inverse(solution)) }
    private fun record(moves:List<String> = solution,state:String = initial,cross:String = "U") = AttemptRecord(
        "cross-fixture","2026-10-26T00:00:00Z","solve",duration=(moves.size-1).coerceAtLeast(0)*100.0,
        initialState=state,cross=cross,moves=moves.mapIndexed { i,m -> MoveSample(m,i*100.0) })

    @Test fun detectsYellowCrossAndKeepsOriginalConfigurationAndCaseRecognition() {
        val input=record()
        val result=analyze(input,catalog)
        assertEquals("U",input.cross)
        assertEquals(4,result.version)
        assertEquals("D",result.cross)
        assertEquals("detected",result.crossSource)
        assertEquals("f2l",result.crossEvidence)
        assertEquals(listOf("D"),result.crossCandidates)
        assertEquals(listOf(1,3,8,14),result.splits.map { it.moves })
        assertEquals("oll-27",result.splits.first { it.stage=="OLL" }.caseId)
        assertEquals("pll-t",result.splits.first { it.stage=="PLL" }.caseId)
        assertEquals(Cube.solved,result.finalState)
    }

    @Test fun detectionAndPhaseCountsAreInvariantAcrossAllSixCrossColors() {
        val covered=mutableSetOf<String>()
        val possibleMoves=Cube.faces.flatMap { face -> listOf("$face","${face}'","${face}2") }
        for(rotation in listOf(emptyList(),listOf("x"),listOf("x2"),listOf("x'"),listOf("z"),listOf("z'"))) {
            val colors=Cube.centers(Cube.apply(Cube.solved,rotation))
            val expected=Cube.faces[colors.indexOf('D')].toString()
            val remapped=solution.map { move ->
                val transformed=Cube.reframe(Cube.turn(Cube.solved,move),rotation)
                possibleMoves.single { Cube.turn(Cube.solved,it)==transformed }
            }
            val state=Cube.reframe(initial,rotation)
            assertTrue(Cube.valid(state))
            assertEquals(Cube.solved,Cube.apply(state,remapped))
            val result=analyze(record(remapped,state,if(expected=="U")"D"else "U"),catalog)
            covered.add(result.cross)
            assertEquals(expected,result.cross,"rotation $rotation")
            assertEquals(listOf(1,3,8,14),result.splits.map { it.moves },"rotation $rotation")
            assertEquals("oll-27",result.splits.first { it.stage=="OLL" }.caseId)
            assertEquals("pll-t",result.splits.first { it.stage=="PLL" }.caseId)
        }
        assertEquals(Cube.faces.map { it.toString() }.toSet(),covered)
    }

    @Test fun initialCrossAndF2lSkipsDoNotAcquireArtificialMoves() {
        val crossReady=analyze(record(solution.drop(1),Cube.apply(initial,solution.take(1))),catalog)
        assertEquals("D",crossReady.cross)
        assertEquals(listOf(0,3,8,14),crossReady.splits.map { it.moves })
        assertEquals(0.0,crossReady.splits.first().time)

        val f2lReady=analyze(record(solution.drop(4),Cube.apply(initial,solution.take(4))),catalog)
        assertEquals("D",f2lReady.cross)
        assertEquals(listOf(0,0,8,14),f2lReady.splits.map { it.moves })
        assertEquals(0.0,f2lReady.splits.first { it.stage=="F2L" }.time)

        val auf=analyze(record(listOf("U'"),Cube.turn(Cube.solved,"U")),catalog)
        assertEquals("D",auf.cross)
        assertEquals(listOf(0,0,0,1),auf.splits.map { it.moves })
    }

    @Test fun equalRecoveredTimestampsCannotTieEarlierAndLaterCubeStates() {
        val result=analyze(record().copy(moves=record().moves.map { it.copy(t=0.0,recovered=true) },quality=listOf("estimated-timing")),catalog)
        assertEquals("D",result.cross)
        assertEquals(listOf("D"),result.crossCandidates)
        assertEquals(listOf(1,3,8,14),result.splits.map { it.moves })
    }

    @Test fun provisionalCrossEvidenceHandlesTiesAndNoEvidenceWithoutPretendingCertainty() {
        val none=analyze(record(emptyList(),cross="B"),catalog)
        assertEquals("B",none.cross)
        assertEquals("configured",none.crossSource)
        assertEquals("configured",none.crossEvidence)

        // D and L both first complete at state 1; U is not among the tied candidates.
        val provisional=analyze(record(solution.take(2)),catalog)
        assertEquals("D",provisional.cross)
        assertEquals(listOf("D","L"),provisional.crossCandidates)
        assertEquals("ambiguous",provisional.crossSource)
        assertEquals("cross",provisional.crossEvidence)
        val preferred=analyze(record(solution.take(2),cross="L"),catalog)
        assertEquals("L",preferred.cross)
        assertEquals("ambiguous",preferred.crossSource)

        val solved=analyze(record(emptyList(),Cube.solved,"F"),catalog)
        assertEquals("F",solved.cross)
        assertEquals("ambiguous",solved.crossSource)
        assertEquals(Cube.faces.map { it.toString() },solved.crossCandidates)
    }

    @Test fun drillsRemainOnTheirChosenCrossAndCompletedMilestonesDoNotReset() {
        val drill=analyze(record().copy(kind="drill",stage="PLL"),catalog)
        assertEquals("U",drill.cross)
        assertEquals("configured",drill.crossSource)
        assertEquals("configured",drill.crossEvidence)

        assertTrue(Cube.progress(Cube.apply(initial,solution.take(1)),'D').cross)
        assertFalse(Cube.progress(Cube.apply(initial,solution.take(2)),'D').cross)
        assertTrue(Cube.progress(Cube.apply(initial,solution.take(4)),'D').f2l)
        assertFalse(Cube.progress(Cube.apply(initial,solution.take(6)),'D').f2l)
        val result=analyze(record(),catalog)
        assertEquals(0.0,result.splits.first { it.stage=="Cross" }.end)
        assertEquals(300.0,result.splits.first { it.stage=="F2L" }.end)
        assertEquals(listOf(1,3,8,14),result.splits.map { it.moves })
    }

    @Test fun oldAnalysisDefaultsToVersionOneAndRefreshInvalidatesWrongCrossSolution() {
        val legacy=analyze(record().copy(kind="drill"),catalog)
        val raw=WireJson.parseToJsonElement(WireJson.encodeToString(legacy)).jsonObject.toMutableMap()
        for(key in listOf("version","cross","crossSource","crossEvidence","crossCandidates"))raw.remove(key)
        val decoded=WireJson.decodeFromString<Analysis>(JsonObject(raw).toString())
        assertEquals(1,decoded.version)
        val old=record().copy(analysis=decoded,crossOptimal=CrossSolution(1,listOf("U")))
        val repaired=refreshAnalysis(old,catalog)
        assertEquals("U",repaired.cross)
        assertEquals(4,repaired.analysis?.version)
        assertEquals("D",repaired.analysis?.cross)
        assertNull(repaired.crossOptimal)

        val correctOptimal=CrossSolution(1,listOf("D"),cross="D")
        val consistent=refreshAnalysis(repaired.copy(crossOptimal=correctOptimal),catalog)
        assertEquals(correctOptimal,consistent.crossOptimal)
        assertEquals(repaired.analysis,consistent.analysis)
        val previousVersion=consistent.copy(analysis=consistent.analysis!!.copy(version=2))
        val updatedVersion=refreshAnalysis(previousVersion,catalog)
        assertEquals(4,updatedVersion.analysis?.version)
        assertEquals(previousVersion.moves,updatedVersion.moves)
        assertEquals(correctOptimal,updatedVersion.crossOptimal)
    }

    @Test fun controllerMigratesCachedAndPendingAnalysisOfflineWithoutChangingRawRecords() = runBlocking {
        val legacy=analyze(record().copy(kind="drill"),catalog).copy(version=1,cross="",crossCandidates=emptyList())
        val cached=record().copy(id="cached",analysis=legacy,crossOptimal=CrossSolution(1,listOf("U")))
        val pending=cached.copy(id="pending",notes="unsynced edit")
        val values=mutableMapOf(
            "settings" to WireJson.encodeToString(ClientSettings(token="")),
            "cache" to WireJson.encodeToString(Bootstrap(records=listOf(cached,pending.copy(notes="older cached edit")))),
            "pending" to WireJson.encodeToString(listOf(pending)),
        )
        val storage=object:LocalStorage {
            override fun read(key:String)=values[key]
            override fun write(key:String,value:String){values[key]=value}
        }
        val transport=object:BleTransport {
            override var onBytes:(ByteArray)->Unit={}
            override var onDisconnected:(String)->Unit={}
            override suspend fun scan():List<CubeDevice> = error("Migration must not scan Bluetooth")
            override suspend fun connect(device:CubeDevice){error("Migration must not connect Bluetooth")}
            override suspend fun write(bytes:ByteArray){error("Migration must not write Bluetooth")}
            override suspend fun disconnect(){}
        }
        val scope=CoroutineScope(coroutineContext+SupervisorJob(coroutineContext[Job]))
        try {
            val controller=LabController(transport,storage,scope)
            withTimeout(10000) {
                while(controller.records.size!=2||controller.records.any{it.analysis?.version!=4}||controller.syncStatus!="Local")delay(10)
            }
            assertEquals(1,controller.queueCount)
            assertEquals("unsynced edit",controller.records.first{it.id=="pending"}.notes)
            val persistedCache=WireJson.decodeFromString<Bootstrap>(values.getValue("cache"))
            val persistedPending=WireJson.decodeFromString<List<AttemptRecord>>(values.getValue("pending"))
            assertEquals(2,persistedCache.records.size)
            assertEquals(1,persistedPending.size)
            for(updated in controller.records+persistedCache.records+persistedPending) {
                assertEquals("U",updated.cross)
                assertEquals(cached.initialState,updated.initialState)
                assertEquals(cached.moves,updated.moves)
                assertEquals(cached.duration,updated.duration)
                assertEquals(4,updated.analysis?.version)
                assertEquals("D",updated.analysis?.cross)
                assertEquals(listOf(1,3,8,14),updated.analysis?.splits?.map{it.moves})
                assertNull(updated.crossOptimal)
            }
        }finally{scope.cancel()}
    }
}
