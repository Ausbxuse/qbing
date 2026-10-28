package lab.cube.ui
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.*
private val LocalNativeProbe=staticCompositionLocalOf<((String,Rect)->Unit)?>{null}
/** Optional screenshot-harness observer; absent from normal application composition. */
@Composable fun ObserveNativeLayout(observer:(String,Rect)->Unit,content: @Composable () -> Unit){CompositionLocalProvider(LocalNativeProbe provides observer,content=content)}
@Composable internal fun Modifier.measureNative(name:String):Modifier {val observer=LocalNativeProbe.current?:return this;return onGloballyPositioned{observer(name,it.boundsInRoot())}}
