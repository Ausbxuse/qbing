package lab.cube.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject

val WireJson = Json { ignoreUnknownKeys = true; encodeDefaults = true; explicitNulls = false }

@Serializable data class CasePosition(val state: String, val solution: List<String>, val setup: List<String>)
@Serializable data class AlgorithmVariant(val id: String, val text: String, val positions: Map<String, CasePosition>)
@Serializable data class CubeCase(val id: String, val stage: String, val name: String, val variants: List<AlgorithmVariant>)
@Serializable data class Catalog(val cases: List<CubeCase>, val recognition: Map<String, Map<String, String>>)
@Serializable data class RawMove(val serial: Int? = null, val localTimestamp: Double? = null, val cubeTimestamp: Double? = null, val timestamp: Double? = null, val receivedAt: Double? = null)
@Serializable data class RotationSample(val notation:String,val t:Double,val pose:List<Float>,val timing:String="local-gyro")
@Serializable data class MoveSample(val move: String, val t: Double, val recovered: Boolean = false, val raw: RawMove = RawMove())
@Serializable data class Split(val stage: String, val start: Double, val end: Double? = null, val time: Double? = null, val moves: Int = 0, val pause: Double = 0.0, val caseId: String? = null, val rawMoves:Int?=null)
@Serializable data class Pause(val index: Int, val t: Double, val duration: Double, val stage: String, val estimated: Boolean = false)
@Serializable data class ReviewSignal(val type: String, val index: Int, val stage: String, val text: String, val t: Double)
@Serializable data class PairEvent(val slot: String, val t: Double, val index: Int, val candidates: List<String> = emptyList())
@Serializable data class Analysis(val splits: List<Split>, val pauses: List<Pause>, val signals: List<ReviewSignal>, val pairEvents: List<PairEvent>, val moves: Int, val duration: Double, val pauseTime: Double, val pauseFraction: Double, val tps: Double? = null, val finalState: String,
    // Missing metadata is a legacy analysis, not an already migrated current record.
    val version:Int=1,val cross:String="",val crossSource:String="configured",val crossCandidates:List<String> = emptyList(),val crossEvidence:String="configured")
@Serializable data class CrossSolution(val distance: Int, val moves: List<String>,val cross:String?=null)
@Serializable data class ScrambleStats(val duration:Double=0.0,val moves:Int=0,val tps:Double?=null)
@Serializable data class SolveReview(val favorite:Boolean=false,val color:String="",val group:String="")
@Serializable data class AttemptRecord(
    val id: String, val createdAt: String, val kind: String, val source: String = "bluetooth",
    val status: String = "complete", val penalty: String = "", val duration: Double,
    val initialState: String, val cross: String = "U", val session: String = "Practice",
    val stage: String = "solve", val caseId: String? = null, val scramble: String = "",
    val moves: List<MoveSample> = emptyList(), val quality: List<String> = emptyList(),
    val inspection: Double? = null, val recognitionMs: Double? = null, val recognitionSource: String? = null,
    val selectedAlgorithmId: String? = null, val notes: String = "", val analysis: Analysis? = null,
    val crossOptimal: CrossSolution? = null,
    val scrambleMoves:List<MoveSample> = emptyList(), val scrambleQuality:List<String> = emptyList(), val scrambleStats:ScrambleStats?=null, val localReview:SolveReview=SolveReview(), val importedSource:kotlinx.serialization.json.JsonElement?=null, val rotations:List<RotationSample> = emptyList(), val rotationFrame:String?=null, val initialOrientation:List<Float>?=null
)
@Serializable data class Preferences(val favorite: Boolean = false, val status: String = "new", val variant: Int = 0, val algorithm: String? = null)
@Serializable data class PracticeSettings(val cross: String = "U", val session: String = "Practice", val mac: String = "", val theme: String = "paper", val gyro: Boolean = false, val pauseThreshold: Int = 500)
@Serializable data class Bootstrap(val schema: Int = 1, val records: List<AttemptRecord> = emptyList(), val settings: PracticeSettings = PracticeSettings(), val preferences: Map<String, Preferences> = emptyMap())
@Serializable data class AlgorithmSample(val recordId: String, val createdAt: String, val execution: Double, val recognition: Double? = null, val recognitionKind: String)
@Serializable data class AlgorithmPerformance(val id: String, val caseId: String, val name: String, val variant: Int? = null, val algorithm: String, val count: Int, val meanExecution: Double, val medianExecution: Double, val meanRecognition: Double? = null, val recognitionCount: Int = 0, val meanTransition: Double? = null, val transitionCount: Int = 0, val samples: List<AlgorithmSample> = emptyList())
@Serializable data class AlgorithmSuggestion(val kind: String, val algorithmId: String, val title: String, val reason: String, val gain: Double? = null)
@Serializable data class CasePerformance(val caseId: String, val name: String, val stage: String, val count: Int, val algorithms: List<AlgorithmPerformance>, val suggestion: AlgorithmSuggestion? = null)
@Serializable data class ClientSettings(val endpoint: String = "https://zhenyuzhao.com/cube-api", val token: String = "", val mac: String = "", val cross: String = "U", val session: String = "Practice", val theme: String = "paper")

expect fun monotonicMs(): Double
expect fun nowIso(): String
expect fun newId(): String
expect fun aesBlock(data: ByteArray, key: ByteArray, iv: ByteArray, encrypt: Boolean): ByteArray
expect suspend fun httpRequest(url: String, method: String, token: String, body: String? = null): String

interface LocalStorage { fun read(key: String): String?; fun write(key: String, value: String) }
data class CubeDevice(val address: String, val name: String)
interface BleTransport {
    var onAdapter: (Boolean, String) -> Unit
        get() = { _, _ -> }
        set(value) {}
    fun close() {}
    var onBytes: (ByteArray) -> Unit
    var onDisconnected: (String) -> Unit
    suspend fun scan(): List<CubeDevice>
    suspend fun connect(device: CubeDevice)
    suspend fun write(bytes: ByteArray)
    suspend fun disconnect()
}

expect suspend fun localSolver(route:String,body:String?=null):String
