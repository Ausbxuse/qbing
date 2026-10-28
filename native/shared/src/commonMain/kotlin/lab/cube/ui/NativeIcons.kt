package lab.cube.ui
import androidx.compose.runtime.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.*
import androidx.compose.ui.graphics.drawscope.*
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*
import lab.cube.core.WireJson
import lab.cube.resources.Res
import org.jetbrains.compose.resources.ExperimentalResourceApi
internal data class NativeIconContour(val path:Path)
internal object NativeIcons{var values by mutableStateOf<Map<String,List<NativeIconContour>>>(emptyMap())}
@OptIn(ExperimentalResourceApi::class)
internal suspend fun loadNativeIcons(){if(NativeIcons.values.isNotEmpty())return;val raw=Res.readBytes("files/icons.json").decodeToString();NativeIcons.values=withContext(Dispatchers.Default){WireJson.parseToJsonElement(raw).jsonObject.mapValues{(_,value)->value.jsonArray.map{v->val item=v.jsonObject;val points=item.getValue("points").jsonArray;NativeIconContour(Path().apply{for((i,point)in points.withIndex()){val p=point.jsonArray;val x=p[0].jsonPrimitive.float;val y=p[1].jsonPrimitive.float;if(i==0)moveTo(x,y)else lineTo(x,y)};if(item.getValue("closed").jsonPrimitive.boolean)close()})}}}}
@Composable internal fun ApprovedIcon(name:String,color:Color,size:Int):Boolean{
 val key=when(name){"Cube"->"cube";"Bluetooth"->"bluetooth";"Settings"->"settings";"Copy"->"copy";"Favorite"->"star";"Tag"->"tag";"Refresh"->"reset";"Play"->"play";"History"->"navHistory";"Practice"->"navTimer";"Cases"->"navPractice";"More"->"more";else->return false}
 val contours=NativeIcons.values[key]?:return false
 Canvas(Modifier.size(size.dp)){withTransform({scale(this@Canvas.size.width/24f,this@Canvas.size.height/24f,Offset.Zero)}){for(contour in contours)drawPath(contour.path,color,style=Stroke(1.5f,cap=StrokeCap.Round,join=StrokeJoin.Round))}}
 return true
}
