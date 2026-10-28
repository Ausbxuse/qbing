package lab.cube.core
import kotlin.test.*
class NativeImportTest {
 @Test fun cstimerImportPreservesRawSourceAndNeverInventsDeviceClocks(){
  val raw="""{"session1":[[[2000,3456],"R","Legacy",1700000000,["R'@3456","333"],{"localReview":{"favorite":true,"color":"blue"}}]]}"""
  val r=importNativeRecords(raw).single()
  assertEquals(3456.0,r.duration);assertEquals("+2",r.penalty);assertEquals("Legacy",r.notes);assertEquals("blue",r.localReview.color);assertNull(r.moves.single().raw.cubeTimestamp);assertNotNull(r.importedSource)
 }
}
