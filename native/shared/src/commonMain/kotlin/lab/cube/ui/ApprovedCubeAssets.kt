package lab.cube.ui

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import lab.cube.resources.Res
import org.jetbrains.compose.resources.ExperimentalResourceApi

internal data class ApprovedCubeAssets(val scene:ByteArray,val vertex:String,val fragment:String)
private val assetMutex=Mutex()
private var cachedAssets:ApprovedCubeAssets?=null
@OptIn(ExperimentalResourceApi::class)
internal suspend fun approvedCubeAssets():ApprovedCubeAssets=assetMutex.withLock {
 cachedAssets?:withContext(Dispatchers.Default){ApprovedCubeAssets(Res.readBytes("files/cube-scene.mesh"),Res.readBytes("files/cube.vert").decodeToString(),Res.readBytes("files/cube.frag").decodeToString())}.also{cachedAssets=it}
}
