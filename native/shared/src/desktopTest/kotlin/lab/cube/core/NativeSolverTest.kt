package lab.cube.core
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.*
import kotlin.test.*
class NativeSolverTest {
 @Test fun randomStateAndRecoveryAreVerifiedWithoutNetwork()=runBlocking {
  val generated=WireJson.parseToJsonElement(localSolver("scramble")).jsonObject
  val moves=generated.getValue("scramble").jsonPrimitive.content.trim().split(Regex("\\s+"))
  val state=Cube.apply(Cube.solved,moves)
  assertEquals(generated.getValue("state").jsonPrimitive.content,state)
  val target=Cube.apply(Cube.solved,listOf("R","U","F2","D","L'"))
  val recovered=WireJson.parseToJsonElement(localSolver("recovery",buildJsonObject{put("state",state);put("target",target)}.toString())).jsonObject
  assertEquals(target,Cube.apply(state,recovered.getValue("moves").jsonArray.map{it.jsonPrimitive.content}))
 }
}
