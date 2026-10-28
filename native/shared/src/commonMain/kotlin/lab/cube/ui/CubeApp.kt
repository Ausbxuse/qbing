package lab.cube.ui

import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.input.key.*
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlinx.serialization.json.*
import kotlinx.coroutines.delay
import lab.cube.core.*
import lab.cube.resources.*
import org.jetbrains.compose.resources.Font
import kotlin.math.*

internal data class LabColors(
    val background: Color, val surface: Color, val raised: Color, val line: Color,
    val text: Color, val muted: Color, val accent: Color, val onAccent: Color,
    val success: Color, val warning: Color, val error: Color, val light: Boolean = false,
)
internal val Graphite = LabColors(Color(0xff17191d),Color(0xff1d2026),Color(0xff242831),Color(0x17ffffff),Color(0xfff0f2f7),Color(0xffbac2cf),Color(0xff86a9ff),Color(0xff111827),Color(0xff9ed2b2),Color(0xffdbb978),Color(0xfff0a59d))
internal val Snappy = Graphite
internal val Paper = LabColors(Color(0xfffcfcfd),Color(0xfff4f5f7),Color.White,Color(0x17202633),Color(0xff202633),Color(0xff566174),Color(0xff2b5de5),Color.White,Color(0xff25764d),Color(0xff966414),Color(0xffb44e45),true)
internal val LocalColors = staticCompositionLocalOf { Snappy }
internal val LocalMono = staticCompositionLocalOf<FontFamily> { FontFamily.Monospace }
internal val LocalDisplay = staticCompositionLocalOf<FontFamily> { FontFamily.SansSerif }
internal val Ink: Color @Composable get() = LocalColors.current.background
internal val Panel: Color @Composable get() = LocalColors.current.surface
internal val Raised: Color @Composable get() = LocalColors.current.raised
internal val Line: Color @Composable get() = LocalColors.current.line
internal val Muted: Color @Composable get() = LocalColors.current.muted
internal val Accent: Color @Composable get() = LocalColors.current.accent
internal val Foreground: Color @Composable get() = LocalColors.current.text
internal val Green: Color @Composable get() = LocalColors.current.success
internal val Amber: Color @Composable get() = LocalColors.current.warning
internal val Red: Color @Composable get() = LocalColors.current.error
internal val faces = mapOf('U' to Color(0xfff5f7fc),'R' to Color(0xfff44e65),'F' to Color(0xff28c992),'D' to Color(0xffffda43),'L' to Color(0xffff9846),'B' to Color(0xff4599f4))
internal fun palette(theme: String) = when(theme) { "graphite", "carbon", "dark" -> Graphite; else -> Paper }
internal fun pageName(page: String) = when(page) { "Practice" -> "timer"; "Cases", "Case" -> "train"; "Progress" -> "statistics"; "History" -> "solves"; else -> page.lowercase() }

@Composable fun CubeApp(c: LabController, onExport: (String)->Unit = {}, onImport: (String)->Unit = {}) {
    LaunchedEffect(Unit){runCatching{loadNativeIcons()}}
    val sans = FontFamily(Font(Res.font.geist_regular,FontWeight.Normal),Font(Res.font.geist_medium,FontWeight.Medium),Font(Res.font.geist_semibold,FontWeight.SemiBold))
    val mono = FontFamily(Font(Res.font.maple_regular),Font(Res.font.maple_semibold,FontWeight.SemiBold))
    val display = mono
    val colors = palette(c.settings.theme)
    val base = TextStyle(fontFamily=sans,letterSpacing=0.sp)
    val typography = Typography(
        bodyLarge=base.copy(fontSize=15.sp,lineHeight=23.sp),bodyMedium=base.copy(fontSize=14.sp,lineHeight=21.sp),bodySmall=base.copy(fontSize=12.sp,lineHeight=18.sp),
        labelLarge=base.copy(fontSize=14.sp,fontWeight=FontWeight.Medium),labelMedium=base.copy(fontSize=12.sp,fontWeight=FontWeight.Medium),labelSmall=base.copy(fontSize=12.sp),
        titleLarge=base.copy(fontSize=25.sp,fontWeight=FontWeight.SemiBold,letterSpacing=(-0.6).sp),titleMedium=base.copy(fontSize=17.sp,fontWeight=FontWeight.Medium),titleSmall=base.copy(fontSize=14.sp,fontWeight=FontWeight.Medium),
        headlineSmall=base.copy(fontSize=25.sp,fontWeight=FontWeight.SemiBold),headlineMedium=base.copy(fontSize=30.sp,fontWeight=FontWeight.Medium),headlineLarge=base.copy(fontSize=36.sp),
        displaySmall=base.copy(fontSize=48.sp),displayMedium=base.copy(fontSize=64.sp),displayLarge=base.copy(fontSize=84.sp),
    )
    val scheme = if(colors.light) lightColorScheme(primary=colors.accent,onPrimary=colors.onAccent,secondary=colors.accent,onSecondary=colors.onAccent,secondaryContainer=colors.raised,onSecondaryContainer=colors.text,tertiary=colors.accent,onTertiary=colors.onAccent,background=colors.background,onBackground=colors.text,surface=colors.surface,surfaceVariant=colors.raised,onSurface=colors.text,onSurfaceVariant=colors.muted,outline=colors.line,error=colors.error)
                 else darkColorScheme(primary=colors.accent,onPrimary=colors.onAccent,secondary=colors.accent,onSecondary=colors.onAccent,secondaryContainer=colors.raised,onSecondaryContainer=colors.text,tertiary=colors.accent,onTertiary=colors.onAccent,background=colors.background,onBackground=colors.text,surface=colors.surface,surfaceVariant=colors.raised,onSurface=colors.text,onSurfaceVariant=colors.muted,outline=colors.line,error=colors.error)
    CompositionLocalProvider(LocalColors provides colors,LocalMono provides mono,LocalDisplay provides display,LocalContentColor provides colors.text) {
        MaterialTheme(colorScheme=scheme,typography=typography) {
            NativeWorkspace(c,onExport,onImport)
            if(c.showDevices) DeviceDialog(c)
            c.review?.let { RecordDialog(c,it) }
        }
    }
}

@Composable internal fun AppHeader(c:LabController,wide:Boolean) {
    val quiet=c.page=="Practice"&&c.active?.startedAt!=null
    BoxWithConstraints(Modifier.fillMaxWidth().height(if(wide)88.dp else 72.dp),contentAlignment=Alignment.Center){
        Row(Modifier.widthIn(max=1040.dp).fillMaxWidth().padding(horizontal=if(wide)if(maxWidth<1120.dp)32.dp else 0.dp else 20.dp),verticalAlignment=Alignment.CenterVertically){
            Text("Qbing",fontSize=if(wide)20.sp else 18.sp,fontWeight=FontWeight.Medium,letterSpacing=(-.6).sp,
                color=if(quiet)Muted.copy(alpha=.3f)else Foreground,modifier=if(quiet)Modifier else Modifier.heightIn(min=44.dp).wrapContentHeight().clickable{c.page="Practice"})
            if(quiet){
                Spacer(Modifier.weight(1f))
                IconButton({c.cancel()},modifier=Modifier.size(44.dp).semantics{contentDescription="Cancel attempt"}){Glyph("Close",Muted.copy(alpha=.55f),17)}
            }else{
                if(wide){
                    Spacer(Modifier.width(48.dp))
                    for(page in listOf("Practice","Cases","History","Progress")){
                        val selected=c.page==page||(page=="Cases"&&c.page=="Case")
                        Box(Modifier.height(44.dp).clickable{c.page=page}.padding(horizontal=13.dp),contentAlignment=Alignment.Center){Text(pageName(page),fontSize=13.sp,color=if(selected)Foreground else Muted.copy(alpha=.66f))}
                    }
                }
                Spacer(Modifier.weight(1f))
                Row(Modifier.height(44.dp).clickable(enabled=!c.connectionBusy){if(c.connected)c.page="Settings"else c.scan()}.padding(horizontal=5.dp),verticalAlignment=Alignment.CenterVertically){
                    if(c.connectionBusy)CircularProgressIndicator(Modifier.size(10.dp),color=Muted,strokeWidth=1.5.dp)else if(c.connected)StatusDot(Green)else Glyph("Bluetooth",Muted,14)
                    Spacer(Modifier.width(8.dp));Text(if(c.connected){(if(wide)c.deviceName else "GAN16 UI")+c.battery?.let{"  $it%"}.orEmpty()}else if(c.connectionBusy)"Connecting"else "Connect",fontSize=11.sp,color=Muted)
                }
                GlyphButton("Settings","Settings"){c.page="Settings"}
            }
        }
    }
}
@Composable internal fun BottomNavigation(c:LabController) {
    val quiet=c.page=="Practice"&&c.active?.startedAt!=null
    Box(Modifier.fillMaxWidth().height(58.dp).padding(horizontal=20.dp)){
        if(!quiet){
            DividerLine()
            Row(Modifier.fillMaxSize(),horizontalArrangement=Arrangement.SpaceBetween,verticalAlignment=Alignment.CenterVertically){
                for(page in listOf("Practice","Cases","History","Progress")){
                    val selected=c.page==page||(page=="Cases"&&c.page=="Case")
                    Box(Modifier.weight(1f).fillMaxHeight().clickable{c.page=page},contentAlignment=Alignment.Center){Text(pageName(page),fontSize=11.sp,color=if(selected)Accent else Muted.copy(alpha=.7f))}
                }
            }
        }
    }
}
@Composable internal fun MessageBar(c:LabController) {
    Row(Modifier.fillMaxWidth().background(Panel).padding(start=20.dp,end=6.dp),verticalAlignment=Alignment.CenterVertically) {
        Text(c.message,fontSize=12.sp,lineHeight=18.sp,color=Muted,modifier=Modifier.weight(1f))
        GlyphButton("Close","Dismiss message"){c.message=""}
    }
}
@Composable internal fun BrandMark(size:Int) {
    val color=Accent
    Canvas(Modifier.size(size.dp)){
        val unit=this.size.width/24f
        drawRoundRect(color,Offset(4*unit,4*unit),Size(15*unit,15*unit),androidx.compose.ui.geometry.CornerRadius(3*unit),style=Stroke(2.4f*unit))
        drawLine(color,Offset(14*unit,14*unit),Offset(22*unit,22*unit),2.4f*unit)

    }
}
@Composable internal fun Glyph(name:String,color:Color=Muted,size:Int=20) {
    if(ApprovedIcon(name,color,size))return
    val panel=Panel
    Canvas(Modifier.size(size.dp)) {
        val u=this.size.width/24f;val stroke=1.6f*u
        fun line(x:Float,y:Float,a:Float,b:Float)=drawLine(color,Offset(x*u,y*u),Offset(a*u,b*u),stroke,StrokeCap.Round)
        when(name){
            "Cube" -> {drawRoundRect(color,Offset(4*u,4*u),Size(16*u,16*u),androidx.compose.ui.geometry.CornerRadius(3*u),style=Stroke(1.5f*u));for(v in listOf(10f,15f)){line(4f,v,20f,v);line(v,4f,v,20f)}}
            "Favorite" -> {val p=Path();for(i in 0..9){val a=-PI.toFloat()/2+i*PI.toFloat()/5;val r=if(i%2==0)9f else 4.2f;val x=12f+cos(a)*r;val y=12f+sin(a)*r;if(i==0)p.moveTo(x*u,y*u)else p.lineTo(x*u,y*u)};p.close();drawPath(p,color,style=Stroke(stroke))}
            "Tag" -> {val p=Path().apply{moveTo(3*u,4*u);lineTo(11*u,4*u);lineTo(21*u,14*u);lineTo(13*u,22*u);lineTo(3*u,12*u);close()};drawPath(p,color,style=Stroke(stroke));drawCircle(color,1.1f*u,Offset(8*u,9*u),style=Stroke(stroke))}
            "Practice" -> {drawCircle(color,8*u,Offset(12*u,14*u),style=Stroke(stroke));line(9f,2f,15f,2f);line(12f,6f,12f,2f);line(12f,14f,15f,10f)}
            "Cases" -> for(x in listOf(3f,14f))for(y in listOf(3f,14f))drawRoundRect(color,Offset(x*u,y*u),Size(7*u,7*u),androidx.compose.ui.geometry.CornerRadius(1.5f*u),style=Stroke(stroke))
            "Progress" -> {line(3f,4f,3f,21f);line(3f,21f,22f,21f);line(6f,16f,11f,11f);line(11f,11f,15f,14f);line(15f,14f,21f,5f)}
            "History" -> {for(y in listOf(5f,12f,19f)){drawCircle(color,u,Offset(4*u,y*u));line(9f,y,21f,y)}}
            "Settings" -> {line(4f,5f,20f,5f);line(4f,12f,20f,12f);line(4f,19f,20f,19f);drawCircle(panel,2.5f*u,Offset(9*u,5*u));drawCircle(color,2.5f*u,Offset(9*u,5*u),style=Stroke(stroke));drawCircle(panel,2.5f*u,Offset(16*u,12*u));drawCircle(color,2.5f*u,Offset(16*u,12*u),style=Stroke(stroke));drawCircle(panel,2.5f*u,Offset(10*u,19*u));drawCircle(color,2.5f*u,Offset(10*u,19*u),style=Stroke(stroke))}
            "Bluetooth" -> {line(11f,2f,11f,22f);line(11f,2f,18f,8f);line(18f,8f,5f,18f);line(5f,6f,18f,16f);line(18f,16f,11f,22f)}
            "Refresh" -> {drawArc(color,45f,290f,false,Offset(4*u,4*u),Size(16*u,16*u),style=Stroke(stroke,cap=StrokeCap.Round));line(21f,5f,20f,11f);line(20f,11f,15f,8f)}
            "Copy" -> {drawRoundRect(color,Offset(8*u,7*u),Size(12*u,14*u),androidx.compose.ui.geometry.CornerRadius(2*u),style=Stroke(stroke));line(4f,16f,4f,3f);line(4f,3f,15f,3f)}
            "Play" -> {val p=Path().apply{moveTo(8*u,5*u);lineTo(19*u,12*u);lineTo(8*u,19*u);close()};drawPath(p,color,style=Stroke(stroke))}
            "Next" -> {line(4f,12f,20f,12f);line(14f,6f,20f,12f);line(14f,18f,20f,12f)}
            "Check" -> {line(5f,12f,10f,17f);line(10f,17f,20f,6f)}
            "Record" -> drawCircle(color,4*u,Offset(12*u,12*u))
            "Undo" -> {drawArc(color,180f,250f,false,Offset(5*u,6*u),Size(14*u,12*u),style=Stroke(stroke,cap=StrokeCap.Round));line(3f,8f,7f,5f);line(3f,8f,7f,11f)}
            "Back" -> {line(19f,12f,5f,12f);line(5f,12f,11f,6f);line(5f,12f,11f,18f)}
            "Chevron" -> {line(9f,6f,15f,12f);line(15f,12f,9f,18f)}
            "StepBack", "StepForward" -> {
                val forward=name=="StepForward"
                fun px(x:Float)=(if(forward)x else 24f-x)*u
                val path=Path().apply{moveTo(px(5f),5*u);lineTo(px(5f),19*u);lineTo(px(15f),12*u);close()}
                drawPath(path,color);line(if(forward)19f else 5f,5f,if(forward)19f else 5f,19f)
            }
            "Pause" -> {drawRect(color,Offset(7*u,5*u),Size(3*u,14*u));drawRect(color,Offset(14*u,5*u),Size(3*u,14*u))}
            "Delete" -> {line(4f,6f,20f,6f);line(9f,3f,15f,3f);line(6f,6f,7f,21f);line(7f,21f,17f,21f);line(17f,21f,18f,6f);line(10f,10f,10f,17f);line(14f,10f,14f,17f)}
            "Close" -> {line(6f,6f,18f,18f);line(18f,6f,6f,18f)}
            else -> for(x in listOf(5f,12f,19f))drawCircle(color,1.4f*u,Offset(x*u,12*u))
        }
    }
}
@Composable internal fun GlyphButton(icon:String,description:String,enabled:Boolean=true,onClick:()->Unit) {
    IconButton(onClick,enabled=enabled,modifier=Modifier.size(44.dp).semantics{contentDescription=description}){Glyph(icon,if(enabled)Muted else Muted.copy(alpha=.35f),19)}
}
@Composable internal fun SmallButton(text:String,enabled:Boolean=true,primary:Boolean=false,onClick:()->Unit) {
    Button(onClick,enabled=enabled,modifier=Modifier.heightIn(min=44.dp),shape=RoundedCornerShape(3.dp),contentPadding=PaddingValues(horizontal=14.dp,vertical=10.dp),colors=ButtonDefaults.buttonColors(containerColor=if(primary)Accent else Raised,contentColor=if(primary)LocalColors.current.onAccent else Foreground,disabledContainerColor=Panel,disabledContentColor=Muted.copy(alpha=.45f)),elevation=null){Text(text,fontSize=13.sp,fontWeight=FontWeight.Medium)}
}
@Composable internal fun QuietButton(text:String,enabled:Boolean=true,onClick:()->Unit){TextButton(onClick,enabled=enabled,modifier=Modifier.heightIn(min=44.dp),contentPadding=PaddingValues(horizontal=10.dp)){Text(text,fontSize=13.sp,color=if(enabled)Muted else Muted.copy(alpha=.4f))}}
@Composable internal fun Segment(text:String,selected:Boolean,onClick:()->Unit){val fill by animateColorAsState(if(selected)Raised else Color.Transparent,tween(150));Surface(onClick,shape=RoundedCornerShape(3.dp),color=fill){Box(Modifier.height(40.dp).padding(horizontal=15.dp),contentAlignment=Alignment.Center){Text(text,fontSize=13.sp,fontWeight=if(selected)FontWeight.Medium else FontWeight.Normal,color=if(selected)Foreground else Muted)}}}
@Composable internal fun StatusDot(color:Color){Canvas(Modifier.size(6.dp)){drawCircle(color)}}
@Composable internal fun Label(text:String,size:Int=12){Text(text,fontSize=size.sp,color=Muted,lineHeight=(size+6).sp)}
@Composable internal fun DividerLine(){HorizontalDivider(color=Line,thickness=1.dp)}
@Composable internal fun Stat(label:String,value:String,detail:String="",modifier:Modifier=Modifier){Column(modifier){Label(label);Spacer(Modifier.height(7.dp));Text(value,fontFamily=LocalMono.current,fontSize=22.sp,fontWeight=FontWeight.Medium,letterSpacing=0.sp);if(detail.isNotBlank()){Spacer(Modifier.height(4.dp));Label(detail,12)}}}
@Composable internal fun SectionTitle(text:String,aside:String=""){Row(Modifier.fillMaxWidth().padding(top=24.dp,bottom=12.dp),verticalAlignment=Alignment.CenterVertically){Text(text,fontSize=14.sp,fontWeight=FontWeight.Medium,modifier=Modifier.weight(1f));if(aside.isNotBlank())Label(aside)}}
@Composable internal fun Empty(title:String,detail:String=""){Column(Modifier.fillMaxWidth().padding(vertical=25.dp)){Text(title,fontSize=14.sp,color=Foreground);if(detail.isNotBlank()){Spacer(Modifier.height(7.dp));Label(detail,13)}}}
@Composable internal fun PageTitle(title:String,subtitle:String=""){Text(title,fontSize=18.sp,fontWeight=FontWeight.SemiBold,letterSpacing=0.sp);if(subtitle.isNotBlank()){Spacer(Modifier.height(6.dp));Label(subtitle,13)}}
internal fun crossColorName(face:String?):String?=when(face){"U"->"White";"D"->"Yellow";"F"->"Green";"B"->"Blue";"R"->"Red";"L"->"Orange";else->null}
internal fun crossDescription(analysis:Analysis?):String? {
    val color=crossColorName(analysis?.cross)?:return null
    val evidence=when {analysis?.crossSource=="ambiguous"->"tied evidence";analysis?.crossSource=="configured"->"configured";analysis?.crossEvidence=="cross"->"provisional";else->"detected from F2L"}
    return "$color cross · $evidence"
}
internal fun tps(value:Double?):String=if(value!=null&&value.isFinite())"${round(value*100)/100} TPS" else "— TPS"

@Composable internal fun StableTimer(text:String,size:Float,modifier:Modifier=Modifier){
    val decimal=text.lastIndexOf('.')
    val run=buildAnnotatedString {
        if(decimal<0)append(text)else{
            append(text.substring(0,decimal))
            withStyle(SpanStyle(fontSize=(size*.87f).sp)){append(text.substring(decimal))}
        }
    }
    Text(run,modifier=modifier,fontFamily=LocalDisplay.current,fontWeight=FontWeight.Light,
        fontSize=size.sp,lineHeight=size.sp,letterSpacing=(-1).sp,maxLines=1,softWrap=false,
        style=TextStyle(fontFeatureSettings="tnum"))
}

internal fun metric(value:Double?,digits:Int=2):String {
    if(value==null||!value.isFinite())return "—"
    val factor=10.0.pow(digits);val parts=(round(value*factor)/factor).toString().split('.')
    return parts[0]+if(digits>0)"."+parts.getOrElse(1){""}.padEnd(digits,'0').take(digits) else ""
}
internal fun scoredTime(record:AttemptRecord):String=(if(adjusted(record).isFinite()&&executionDurationEstimated(record))"≈"else "")+formatTime(adjusted(record))
@Composable internal fun AverageSummary(records:List<AttemptRecord>,size:Int,compact:Boolean,modifier:Modifier=Modifier){
    val average=remember(records,size){retainedAverage(records,size)}
    var details by remember{mutableStateOf(false)}
    Box(modifier){
        Column(Modifier.heightIn(min=52.dp).clickable(enabled=average.retained>0){details=true}.semantics{contentDescription="Average of $size: ${formatTime(average.time)} seconds. TPS from ${average.tpsSamples}/${average.retained}; moves from ${average.moveSamples}/${average.retained} retained solves."},verticalArrangement=Arrangement.Center){
            Row(verticalAlignment=Alignment.CenterVertically){
                Text("ao$size",fontSize=11.sp,color=Muted);Spacer(Modifier.width(8.dp))
                Text((if(average.timeEstimated)"≈"else "")+formatTime(average.time),fontSize=if(compact)16.sp else 18.sp,style=TextStyle(fontFeatureSettings="tnum"))
            }
            Spacer(Modifier.height(5.dp))
            Text("${if(average.estimated)"≈"else ""}${metric(average.tps)} tps · ${metric(average.moves,1)} moves",fontSize=if(compact)10.sp else 11.sp,lineHeight=15.sp,color=Muted)
        }
        DropdownMenu(details,{details=false},containerColor=Panel){DropdownMenuItem(text={Text("TPS ${average.tpsSamples}/${average.retained} · moves ${average.moveSamples}/${average.retained} retained solves",fontSize=12.sp)},onClick={details=false})}
    }
}
@Composable internal fun MoveToken(move:String,done:Boolean,current:Boolean,half:Boolean=false,correction:Boolean=false,small:Boolean=false){
    val color=if(correction)Red else if(current)Accent else if(done)Muted.copy(alpha=.32f)else Foreground
    Column(Modifier.width(IntrinsicSize.Min).clip(RoundedCornerShape(3.dp))
        .background(if(current)color.copy(alpha=.10f)else Color.Transparent)
        .then(if(done)Modifier.blur(.7.dp)else Modifier)
        .semantics{contentDescription=move+if(correction)" · correction"else "";if(half)stateDescription="Half complete"},horizontalAlignment=Alignment.CenterHorizontally){
        Text(move,fontFamily=LocalMono.current,fontSize=if(small)16.sp else 23.sp,lineHeight=if(small)24.sp else 33.sp,color=color,softWrap=false,modifier=Modifier.padding(horizontal=3.dp,vertical=2.dp))
        Box(Modifier.fillMaxWidth().height(2.dp)){
            if(current||half){Box(Modifier.fillMaxWidth().height(1.dp).background(if(half)Muted.copy(alpha=.35f)else color));if(half)Box(Modifier.fillMaxWidth(.5f).height(2.dp).background(color))}
        }
    }
}
@Composable internal fun PhaseSummary(analysis:Analysis?,liveElapsed:Double?=null,modifier:Modifier=Modifier,compact:Boolean=false,record:AttemptRecord?=null){
    val trusted=record==null||record.quality.all{it=="estimated-timing"}
    val current=analysis?.splits?.firstOrNull{it.end==null}?.stage
    Row(modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(if(compact)12.dp else 32.dp)){
        for(stage in listOf("Cross","F2L","OLL","PLL")){
            val split=analysis?.splits?.firstOrNull{it.stage==stage};val live=liveElapsed!=null&&current==stage
            val duration=split?.time?:if(live)maxOf(0.0,liveElapsed!!-split!!.start)else null
            val estimated=record!=null&&split!=null&&splitTimingEstimated(record,split)&&duration!=null&&duration>0
            val ink=if(live)Accent else if(liveElapsed!=null)Muted else Foreground
            val phaseMoves=split?.moves?.takeIf{duration!=null}
            Column(Modifier.weight(1f).semantics{contentDescription="$stage: ${formatTime(duration)} seconds, ${split?.moves?:0} moves"}){
                Row(Modifier.height(25.dp),verticalAlignment=Alignment.CenterVertically){
                    Text(stage,fontSize=11.sp,color=if(live)Accent else Muted)
                    if(stage=="Cross"&&analysis!=null){Spacer(Modifier.width(7.dp));StatusDot(faces[analysis.cross.firstOrNull()]?:Muted)}
                }
                Text((if(estimated)"≈"else "")+formatTime(duration),fontSize=if(compact)23.sp else 28.sp,lineHeight=34.sp,color=ink,letterSpacing=(-.7).sp,style=TextStyle(fontFeatureSettings="tnum"))
                val speed=(if(estimated)"≈"else "")+metric(duration?.takeIf{it>0&&trusted}?.let{split!!.moves*1000/it})
                if(compact){
                    Text("$speed tps",fontSize=11.sp,lineHeight=18.sp,color=Muted)
                    Text("${phaseMoves?:"—"} moves",fontSize=11.sp,lineHeight=18.sp,color=Muted)
                }else Text("$speed tps · ${phaseMoves?:"—"} moves",fontSize=12.sp,lineHeight=25.sp,color=Muted)
                Box(Modifier.padding(top=8.dp).fillMaxWidth().height(2.dp).background(if(live)Accent else Color.Transparent))
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun Practice(c:LabController,wide:Boolean,noticeHeight:Dp=0.dp){
    val active=c.active;val target=c.target;val latest=c.latest;val running=active?.startedAt!=null
    val latestExecution=remember(latest){latest?.let(::recordedExecutionMetric)}
    val phaseRecord=remember(c.moves.size,latest,running){if(running&&c.moves.isNotEmpty())active!!.record(c.catalog)else latest}
    val analysis=phaseRecord?.analysis
    val liveMeasured=active?.quality?.all{it=="estimated-timing"}==true
    val liveEstimated=c.moves.firstOrNull()?.let{it.recovered||it.raw.cubeTimestamp?.isFinite()!=true}==true
    var manual by remember{mutableStateOf(false)};var sessionEditor by remember{mutableStateOf(false)};var more by remember{mutableStateOf(false)}
    var resetGyro by remember{mutableStateOf(0)};var confirmResetCube by remember{mutableStateOf(false)}
    val solves=c.records.filter{it.kind=="solve"&&it.session==c.settings.session};val clipboard=LocalClipboardManager.current
    val progress=c.cubeState?.takeIf{!running}?.let{state->target?.let{scrambleProgress(state,it.position.setup)}}
    val recovery=c.recoveryMoves
    val correcting=!running&&progress?.matched==false&&recovery.isNotEmpty()
    val displayedMoves=target?.position?.setup.orEmpty()
    val marks=target?.let{checkpointProgress(c.cubeState,c.scrambleCheckpoint,it.position.setup)}
    val ready=target!=null&&c.cubeState==target.position.state&&recovery.isEmpty()&&!running
    val clock=if(c.cueCountdown!=null)c.cueCountdown.toString()else formatTime(if(running)c.elapsed else latest?.let(::adjusted)?:0.0)
    val estimatedTime=c.cueCountdown==null&&if(running)liveEstimated else latest?.let{adjusted(it).isFinite()&&executionDurationEstimated(it)}==true
    BoxWithConstraints(Modifier.fillMaxSize()){
        val short=maxHeight<620.dp
        val timerSize=if(wide)if(short)124f else 152f else if(clock.length>5)92f else 106f
        // The header and mobile navigation reserve the same space in focus mode.
        // Anchor the numeral itself to the viewport, independent of surrounding rows.
        val centerY=maxHeight/2+(if(wide)(-88).dp else (-14).dp)/2-noticeHeight/2
        val resultTop=centerY+if(wide)if(short)110.dp else 196.dp else if(short)128.dp else 142.dp
        val activityTop=if(wide)if(short)4.dp else 32.dp else 20.dp
        Row(Modifier.align(Alignment.TopCenter).offset(y=activityTop).widthIn(max=1040.dp).fillMaxWidth().padding(horizontal=if(wide)if(maxWidth<1120.dp)32.dp else 0.dp else 20.dp),horizontalArrangement=Arrangement.spacedBy(if(wide)42.dp else 12.dp)){
            Column(Modifier.weight(1f)){
                if(!running){
                    Row(Modifier.fillMaxWidth().height(40.dp),verticalAlignment=Alignment.CenterVertically){
                        Text("3 × 3",fontSize=11.sp,color=Muted);Spacer(Modifier.width(if(wide)20.dp else 8.dp))
                        Text(c.settings.session+" ⌄",fontSize=11.sp,color=Muted,maxLines=1,overflow=TextOverflow.Ellipsis,modifier=Modifier.weight(1f).height(40.dp).wrapContentHeight().clickable(enabled=active==null){sessionEditor=true})
                        IconButton({c.prepareScramble()},enabled=active==null&&!c.scrambleLoading,modifier=Modifier.size(40.dp).semantics{contentDescription="Next scramble"}){Glyph("Next",Muted,16)}
                        IconButton({clipboard.setText(AnnotatedString(displayedMoves.joinToString(" ")))},enabled=displayedMoves.isNotEmpty(),modifier=Modifier.size(40.dp).semantics{contentDescription="Copy scramble"}){Glyph("Copy",Muted,16)}
                    }
                    val speed=if(target!=null||active!=null)c.scrambleSpeed else latest?.scrambleStats
                    FlowRow(horizontalArrangement=Arrangement.spacedBy(if(wide)9.dp else 4.dp),verticalArrangement=Arrangement.spacedBy(3.dp),modifier=Modifier.fillMaxWidth().padding(top=4.dp)){
                        for((i,move)in displayedMoves.withIndex()){
                            if(correcting&&i==(marks?.completed?:0)){
                                Box(Modifier.height(if(wide)39.dp else 30.dp),contentAlignment=Alignment.Center){Glyph("Undo",Red,11)}
                                for((j,repair)in recovery.withIndex())MoveToken(repair,false,j==0,correction=true,small=!wide)
                            }
                            MoveToken(move,marks?.done?.contains(i)==true,!correcting&&marks!=null&&i==marks.completed&&marks.nextMoves.contains(i),!correcting&&marks!=null&&i==marks.completed&&marks.partialMoves.contains(i),small=!wide)
                        }
                        if(displayedMoves.isNotEmpty())Row(Modifier.height(if(wide)39.dp else 30.dp).padding(start=5.dp),verticalAlignment=Alignment.CenterVertically){
                            if(ready){Glyph("Check",Green,12);Spacer(Modifier.width(6.dp))}
                            Text(metric(speed?.tps)+" tps",fontSize=if(wide)12.sp else 10.sp,color=Muted,modifier=Modifier.semantics{contentDescription="Scramble speed: ${metric(speed?.tps)} turns per second"})
                        }
                    }
                    if(c.recoveryBusy||c.scrambleLoading)CircularProgressIndicator(Modifier.padding(top=8.dp).size(12.dp).semantics{contentDescription=if(c.recoveryBusy)"Finding corrective moves"else "Preparing scramble"},strokeWidth=1.5.dp,color=Muted)
                    c.recoveryError?.let{Text(it,fontSize=11.sp,lineHeight=16.sp,color=Red,modifier=Modifier.padding(top=6.dp))}
                }
            }
            // Keep this exact geometry when solving; only notation and controls disappear.
            Column(Modifier.width(if(wide)130.dp else 78.dp).padding(top=if(wide)22.dp else 26.dp),horizontalAlignment=Alignment.CenterHorizontally){
                CubeCanvas(c.cubeState?:Cube.solved,Modifier.size(if(wide)118.dp else 78.dp).alpha(if(running).55f else 1f),c.visualTurns,orientation=c.orientation,resetGyro=resetGyro,onGyroFrame=c::captureGyroFrame)
                if(!running)Row(Modifier.height(44.dp),horizontalArrangement=Arrangement.Center){
                    IconButton({resetGyro++},modifier=Modifier.size(if(wide)44.dp else 39.dp).semantics{contentDescription="Reset gyro"}){Glyph("Refresh",Muted,15)}
                    IconButton({confirmResetCube=true},enabled=c.connected,modifier=Modifier.size(if(wide)44.dp else 39.dp).semantics{contentDescription="Reset cube"}){Glyph("Cases",if(c.connected)Muted else Muted.copy(alpha=.4f),15)}
                }
            }
        }
        Box(Modifier.align(Alignment.TopCenter).offset(y=centerY-(timerSize/2).dp-(timerSize*.145f).dp).height(timerSize.dp),contentAlignment=Alignment.Center){
            StableTimer(clock,timerSize,Modifier.semantics{contentDescription=(if(estimatedTime)"Estimated "else "")+clock+" seconds"}.clickable(enabled=latest!=null&&!running){c.review=latest})
            if(estimatedTime)Text("≈",fontSize=20.sp,color=Muted,modifier=Modifier.align(Alignment.CenterStart).offset(x=(-25).dp))
        }
        Row(Modifier.align(Alignment.TopCenter).offset(y=centerY+(if(wide)if(short)80.dp else 102.dp else 69.dp)).height(28.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(17.dp)){
            Row(verticalAlignment=Alignment.CenterVertically){Text((if(if(running)liveEstimated else latestExecution?.estimated==true)"≈"else "")+metric(if(running)if(liveMeasured&&c.elapsed>0)countMoves(c.moves)*1000/c.elapsed else null else latestExecution?.tps),fontSize=17.sp,style=TextStyle(fontFeatureSettings="tnum"));Text(" tps",fontSize=11.sp,color=Muted,modifier=Modifier.padding(start=3.dp))}
            Text("·",fontSize=10.sp,color=Muted.copy(alpha=.6f))
            Row(verticalAlignment=Alignment.CenterVertically){Text(if(running)countMoves(c.moves).toString()else latest?.let(::recordedMoveCount)?.toString()?:"—",fontSize=17.sp);Text(" moves",fontSize=11.sp,color=Muted,modifier=Modifier.padding(start=3.dp))}
        }
        Column(Modifier.align(Alignment.TopCenter).offset(y=resultTop).widthIn(max=900.dp).fillMaxWidth().padding(horizontal=if(wide)if(maxWidth<964.dp)32.dp else 0.dp else 22.dp)){
            DividerLine()
            if(!running){
                Row(Modifier.fillMaxWidth().height(if(wide)if(short)56.dp else 76.dp else 74.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(if(wide)34.dp else 8.dp)){
                    AverageSummary(solves,5,!wide,if(wide)Modifier.width(184.dp)else Modifier.weight(1f))
                    AverageSummary(solves,12,!wide,if(wide)Modifier.width(184.dp)else Modifier.weight(1f))
                    if(wide)Spacer(Modifier.weight(1f))
                    Row(verticalAlignment=Alignment.CenterVertically){
                        when{
                            c.awaitingCue->IconButton({c.startCue()},modifier=Modifier.size(40.dp).semantics{contentDescription="Start cue"}){Glyph("Play",Accent,18)}
                            active!=null->IconButton({c.cancel()},modifier=Modifier.size(40.dp).semantics{contentDescription="Cancel attempt"}){Glyph("Close",Muted,18)}
                            latest!=null->{
                                if(wide)TextButton({c.review=latest},contentPadding=PaddingValues(horizontal=8.dp)){Text("▷ replay",fontSize=12.sp,color=Accent)}
                                else IconButton({c.review=latest},modifier=Modifier.size(36.dp).semantics{contentDescription="Replay last solve"}){Glyph("Play",Accent,17)}
                                if(wide){
                                    TextButton({c.updateRecord(latest.copy(penalty=if(latest.penalty=="+2")""else "+2"));c.review=null},contentPadding=PaddingValues(horizontal=6.dp)){Text("+2",fontSize=12.sp,color=if(latest.penalty=="+2")Accent else Muted)}
                                    TextButton({c.updateRecord(latest.copy(penalty=if(latest.penalty=="DNF")""else "DNF"));c.review=null},contentPadding=PaddingValues(horizontal=6.dp)){Text("DNF",fontSize=12.sp,color=if(latest.penalty=="DNF")Red else Muted)}
                                }
                            }
                        }
                        Box{
                            IconButton({more=true},modifier=Modifier.size(if(wide)40.dp else 36.dp).semantics{contentDescription="Result and practice controls"}){Glyph("More",Muted,16)}
                            DropdownMenu(more,{more=false},containerColor=Panel){
                                if(latest!=null&&active==null){
                                    DropdownMenuItem(text={Text(if(latest.penalty=="+2")"Remove +2"else "Add +2")},onClick={c.updateRecord(latest.copy(penalty=if(latest.penalty=="+2")""else "+2"));c.review=null;more=false})
                                    DropdownMenuItem(text={Text(if(latest.penalty=="DNF")"Remove DNF"else "Mark DNF")},onClick={c.updateRecord(latest.copy(penalty=if(latest.penalty=="DNF")""else "DNF"));c.review=null;more=false})
                                    if(latest.caseId!=null)DropdownMenuItem(text={Text("Repeat case")},onClick={c.prepareCase(latest.caseId,c.selectedVariant);more=false})
                                }
                                DropdownMenuItem(text={Text("Enter a time")},enabled=active==null,onClick={manual=true;more=false})
                                DropdownMenuItem(text={Text("New scramble")},enabled=active==null,onClick={c.prepareScramble();more=false})
                                DropdownMenuItem(text={Text("Use current cube state")},enabled=c.connected&&active==null,onClick={c.armCurrent();more=false})
                                DropdownMenuItem(text={Text("Open this session")},onClick={c.page="History";more=false})
                                if(target!=null)DropdownMenuItem(text={Text("Cancel setup")},enabled=active==null,onClick={c.cancel();more=false})
                            }
                        }
                    }
                }
            }else Spacer(Modifier.height(if(wide)28.dp else 24.dp))
            PhaseSummary(analysis,if(running)c.elapsed else null,compact=!wide,record=phaseRecord)
        }
    }
    if(manual)ManualDialog(c){manual=false}
    if(confirmResetCube)ResetCubeDialog(c,{confirmResetCube=false}){resetGyro++}
    if(sessionEditor){var value by remember{mutableStateOf(c.settings.session)};AlertDialog(onDismissRequest={sessionEditor=false},containerColor=Panel,title={Text("Practice session")},text={Field("Session name",value,{value=it})},confirmButton={TextButton(onClick={runCatching{c.saveSettings(c.settings.copy(session=value.trim().ifBlank{"Practice"}))}.onSuccess{sessionEditor=false}.onFailure{c.message=it.message?:"Could not update session"}}){Text("Save")}},dismissButton={TextButton(onClick={sessionEditor=false}){Text("Cancel")}})}
}
@Composable internal fun ResetCubeDialog(c:LabController,onDismiss:()->Unit,onReset:()->Unit={}){
    AlertDialog(onDismissRequest=onDismiss,containerColor=Panel,title={Text("Is the physical cube solved?")},
        text={Column{Text("All six faces must be physically solved. Reset cube replaces the cube’s internal state with solved; it does not solve the physical cube.",fontSize=14.sp);if(c.active!=null){Spacer(Modifier.height(10.dp));Text(if(c.moves.isNotEmpty())"The current attempt will be saved as interrupted."else "The armed attempt will be cancelled.",fontSize=13.sp,color=Muted)};if(!c.connected){Spacer(Modifier.height(10.dp));Text("Reconnect your cube before resetting.",fontSize=13.sp,color=Muted)}}},
        confirmButton={TextButton(onClick={if(c.connected){c.cancel("cube-reset");c.calibrate();onReset();onDismiss()}},enabled=c.connected){Text("Cube is solved · reset")}},
        dismissButton={TextButton(onClick=onDismiss){Text("Cancel")}})
}
@Composable internal fun ManualDialog(c:LabController,onDismiss:()->Unit){var value by remember{mutableStateOf("")};val seconds=value.toDoubleOrNull();AlertDialog(onDismissRequest=onDismiss,containerColor=Panel,title={Text("Enter a time")},text={Column{TextField(value,{value=it},label={Text("Seconds")},keyboardOptions=KeyboardOptions(keyboardType=KeyboardType.Decimal),singleLine=true,shape=RoundedCornerShape(10.dp));Spacer(Modifier.height(12.dp));Label("Manual times have no turn-speed or phase measurements.")}},confirmButton={TextButton(enabled=seconds!=null&&seconds>0&&seconds<=86400,onClick={seconds?.let{c.manualTime(it);onDismiss()}}){Text("Save time")}},dismissButton={TextButton(onClick=onDismiss){Text("Cancel")}})}

@Composable internal fun Cases(c:LabController){
    val cases=c.catalog.cases.filter{(c.caseStage=="All"||it.stage==c.caseStage)&&(c.search.isBlank()||it.name.contains(c.search,true)||it.variants.any{v->v.text.contains(c.search,true)})}
    Column(Modifier.fillMaxSize().padding(horizontal=22.dp)){
        Spacer(Modifier.height(8.dp));PageTitle("Algorithm library","41 F2L · 57 OLL · 21 PLL · ${c.catalog.cases.sumOf{it.variants.size}} variants")
        Spacer(Modifier.height(20.dp));SearchField(c.search,{c.search=it},"Find a case or algorithm")
        Spacer(Modifier.height(12.dp));Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),horizontalArrangement=Arrangement.spacedBy(3.dp)){for(stage in listOf("All","Cross","F2L","OLL","PLL"))Segment(stage,c.caseStage==stage){c.caseStage=stage}}
        Spacer(Modifier.height(16.dp));Label("${cases.size} cases · cross-down diagrams",12);Spacer(Modifier.height(8.dp))
        LazyColumn(contentPadding=PaddingValues(bottom=20.dp)){
            items(cases,key={it.id}){case->
                val performance=c.performances.firstOrNull{it.caseId==case.id};val usual=performance?.algorithms?.maxByOrNull{it.count}
                Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).clickable{c.selectedCase=case.id;c.selectedVariant=c.preferences[case.id]?.variant?:0;c.page="Case"}.padding(vertical=14.dp,horizontal=2.dp),verticalAlignment=Alignment.CenterVertically){
                    CasePattern(case.variants.first().positions.getValue("D").state,case.stage,Modifier.size(62.dp));Spacer(Modifier.width(16.dp))
                    Column(Modifier.weight(1f)){Text(case.name,fontSize=16.sp,fontWeight=FontWeight.Medium);Spacer(Modifier.height(5.dp));Label("${case.variants.size} variants"+if(c.preferences[case.id]?.favorite==true)" · favorite"else "",12)}
                    Column(horizontalAlignment=Alignment.End){Text(formatTime(usual?.meanExecution),fontSize=22.sp,letterSpacing=(-.5).sp);Spacer(Modifier.height(4.dp));Label(if(performance==null)"Not practiced"else "${performance.count} attempts",12)}
                    Spacer(Modifier.width(12.dp));Glyph("Chevron",Muted.copy(alpha=.55f),15)
                };DividerLine()
            }
            if(cases.isEmpty())item{Empty("No matching cases","Try a case name such as T perm or a move sequence.")}
        }
    }
}
@Composable internal fun SearchField(value:String,onChange:(String)->Unit,placeholder:String){TextField(value,onChange,placeholder={Text(placeholder,fontSize=14.sp,color=Muted)},singleLine=true,modifier=Modifier.fillMaxWidth(),shape=RoundedCornerShape(12.dp),textStyle=LocalTextStyle.current.copy(fontSize=14.sp),colors=TextFieldDefaults.colors(focusedContainerColor=Panel,unfocusedContainerColor=Panel,focusedIndicatorColor=Color.Transparent,unfocusedIndicatorColor=Color.Transparent,cursorColor=Accent))}

@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun CaseDetail(c:LabController){
    val case=c.catalog.cases.firstOrNull{it.id==c.selectedCase}?:return;val index=c.selectedVariant.coerceIn(case.variants.indices);val alg=case.variants[index]
    val local=remember(c.records,case){localCasePerformance(c.records,case)}
    val performance=c.performances.firstOrNull{it.caseId==case.id}?:local;val stats=remember(performance){performance?.let(::caseSummary)}
    var analysis by remember(case.id){mutableStateOf(false)}
    LazyColumn(Modifier.fillMaxSize().padding(horizontal=22.dp),contentPadding=PaddingValues(bottom=28.dp)){
        item{
            Row(verticalAlignment=Alignment.CenterVertically){GlyphButton("Back","Back to library"){c.page="Cases"};Label("Library / ${case.stage}")}
            Row(Modifier.fillMaxWidth().padding(top=10.dp,bottom=22.dp),verticalAlignment=Alignment.CenterVertically){CasePattern(alg.positions.getValue("D").state,case.stage,Modifier.size(93.dp));Spacer(Modifier.width(22.dp));Column(Modifier.weight(1f)){Label(case.stage);Spacer(Modifier.height(6.dp));Text(case.name,fontSize=18.sp,fontWeight=FontWeight.SemiBold,letterSpacing=0.sp);QuietButton(if(c.preferences[case.id]?.favorite==true)"Favorited"else "Add to favorites"){c.toggleFavorite(case)}}}
            Row(Modifier.fillMaxWidth().height(36.dp),horizontalArrangement=Arrangement.spacedBy(8.dp)){NativeTab("Algorithms",!analysis,Modifier.fillMaxHeight()){analysis=false};NativeTab("Analysis",analysis,Modifier.fillMaxHeight()){analysis=true}}
            Spacer(Modifier.height(16.dp))
            if(!analysis){
            Surface(color=Panel,shape=RoundedCornerShape(14.dp)){
                Column(Modifier.fillMaxWidth().padding(18.dp)){
                    Row(Modifier.fillMaxWidth()){Label("ALGORITHM",12);Spacer(Modifier.weight(1f));Label("${index+1} of ${case.variants.size}",12)}
                    Spacer(Modifier.height(14.dp));Text(alg.text,fontFamily=LocalMono.current,fontSize=19.sp,lineHeight=30.sp)
                    if(case.variants.size>1){Spacer(Modifier.height(18.dp));FlowRow(horizontalArrangement=Arrangement.spacedBy(4.dp),verticalArrangement=Arrangement.spacedBy(4.dp)){for(i in case.variants.indices)Segment("${i+1}",i==index){c.selectVariant(case,i)}}}
                }
            }
            Spacer(Modifier.height(18.dp));SmallButton("Practice this algorithm",primary=true){c.prepareCase(case.id,index)}
            }else{
            SectionTitle("Your results",if(stats!=null)"${stats.count} executions"else "Build a baseline")
            Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(18.dp)){Stat("Recognition",formatTime(stats?.meanRecognition),"${stats?.recognitionCount?:0} cue samples",Modifier.weight(1f));Stat("Execution",formatTime(stats?.meanExecution),"first to last turn",Modifier.weight(1f))}
            Spacer(Modifier.height(20.dp));TraceChart(stats?.samples?.map{it.execution}?:emptyList(),Modifier.fillMaxWidth().height(170.dp))
            stats?.meanTransition?.let{Spacer(Modifier.height(14.dp));Label("Full-solve transition: ${formatTime(it)} s · ${stats.transitionCount} samples")}
            SectionTitle("Algorithms you use")
            if(performance==null)Empty("No measured attempts","Practice this case to build a baseline.")else AlgorithmTable(performance.algorithms)
            performance?.suggestion?.let{suggestion->
                SectionTitle(if(suggestion.kind=="measured")"A faster option"else "Next experiment")
                Surface(color=Panel,shape=RoundedCornerShape(14.dp)){Column(Modifier.fillMaxWidth().padding(18.dp)){Text(suggestion.title,fontSize=15.sp,fontWeight=FontWeight.Medium);Spacer(Modifier.height(8.dp));Label(suggestion.reason,13);val candidate=case.variants.indexOfFirst{it.id==suggestion.algorithmId};if(candidate>=0){Spacer(Modifier.height(15.dp));SmallButton("Try this variant"){c.prepareCase(case.id,candidate)}}}}
            }
            SectionTitle("How timing works")
            Label("Recognition runs from the cue to your first turn, including reaction. Execution ends at the case goal. Recovered timing is excluded from these averages.",12)
            }
        }
    }
}
@Composable internal fun AlgorithmTable(algorithms:List<AlgorithmPerformance>){
    for(algorithm in algorithms){
        Column(Modifier.fillMaxWidth().padding(vertical=15.dp)){
            Row(Modifier.fillMaxWidth()){Text(algorithm.name,fontSize=14.sp,fontWeight=FontWeight.Medium,modifier=Modifier.weight(1f));Label("${algorithm.count} uses")}
            if(algorithm.variant==null){Spacer(Modifier.height(6.dp));Text(algorithm.algorithm,fontFamily=LocalMono.current,fontSize=12.sp,lineHeight=20.sp,color=Muted,maxLines=3,overflow=TextOverflow.Ellipsis)}
            Spacer(Modifier.height(12.dp));Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(18.dp)){Column(Modifier.weight(1f)){Label("Recognition",12);Text(formatTime(algorithm.meanRecognition),fontSize=19.sp);Label("${algorithm.recognitionCount} samples",12)};Column(Modifier.weight(1f)){Label("Execution",12);Text(formatTime(algorithm.meanExecution),fontSize=19.sp);Label("${algorithm.count} samples",12)}}
        };DividerLine()
    }
}

@Composable internal fun ProgressPage(c:LabController){
    var kind by remember{mutableStateOf("solve")};val records=c.records.filter{it.kind==kind};val complete=records.filter{it.status=="complete"}
    val ranked=complete.filter{adjusted(it).isFinite()}.sortedBy(::adjusted);val times=ranked.map(::adjusted)
    val medianEstimated=ranked.isNotEmpty()&&listOf(ranked[(ranked.size-1)/2],ranked[ranked.size/2]).any(::executionDurationEstimated)
    val best=ranked.filter{it.duration>0&&(if(it.source=="manual")it.quality.isEmpty() else recordedExecutionMetric(it)?.estimated==false)}.minOfOrNull(::adjusted)
    val trend=records.takeLast(60);val estimatedPoints=trend.mapIndexedNotNull{i,r->if(adjusted(r).isFinite()&&executionDurationEstimated(r))i else null}.toSet()
    val measured=complete.filter{it.penalty!="DNF"&&it.analysis!=null&&it.quality.none{q->q in listOf("missing-moves","estimated-timing","state-mismatch","clock-reset","unverified-final-state")}}
    LazyColumn(Modifier.fillMaxSize().padding(horizontal=22.dp),contentPadding=PaddingValues(top=8.dp,bottom=28.dp)){
        item{
            PageTitle("Insights","${c.records.size} attempts in your scorebook")
            Spacer(Modifier.height(18.dp));Row(horizontalArrangement=Arrangement.spacedBy(4.dp)){Segment("Full solves",kind=="solve"){kind="solve"};Segment("Drills",kind=="drill"){kind="drill"}}
            Spacer(Modifier.height(24.dp));Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(16.dp)){Stat("Attempts",records.size.toString(),modifier=Modifier.weight(1f));Stat("Median",(if(medianEstimated)"≈"else "")+formatTime(median(times)),modifier=Modifier.weight(1f));Stat("Best",formatTime(best),modifier=Modifier.weight(1f))}
            SectionTitle("Execution trend",if(estimatedPoints.isEmpty())"seconds"else "seconds · ○ estimated");TraceChart(trend.map(::adjusted),Modifier.fillMaxWidth().height(180.dp),estimatedPoints)
            if(kind=="solve"){
                SectionTitle("Where the time goes","${measured.size} solves")
                val stages=listOf("Cross","F2L","OLL","PLL");val values=stages.map{stage->measured.mapNotNull{it.analysis?.splits?.firstOrNull{it.stage==stage}?.time}.takeIf{it.isNotEmpty()}?.average()};val max=values.filterNotNull().maxOrNull()?:1.0
                for((i,stage)in stages.withIndex()){
                    val phaseTps=measured.mapNotNull{it.analysis?.splits?.firstOrNull{s->s.stage==stage}}.filter{(it.time?:0.0)>0}.map{it.moves*1000/it.time!!}.takeIf{it.isNotEmpty()}?.average()
                    Row(Modifier.fillMaxWidth().padding(vertical=11.dp),verticalAlignment=Alignment.CenterVertically){
                        Text(stage,fontSize=13.sp,modifier=Modifier.width(51.dp));Box(Modifier.weight(1f).height(5.dp).clip(RoundedCornerShape(3.dp)).background(Line)){Box(Modifier.fillMaxWidth(((values[i]?:0.0)/max).toFloat().coerceIn(0f,1f)).fillMaxHeight().background(Accent.copy(alpha=.9f-i*.12f)))}
                        Column(Modifier.width(96.dp),horizontalAlignment=Alignment.End){Text(formatTime(values[i]),fontSize=18.sp);Label(tps(phaseTps),12)}
                    }
                }
            }
            SectionTitle("Case performance","mean execution")
            if(c.performances.isEmpty())Empty("No case measurements yet","Record five clean repetitions to establish a baseline.")
        }
        items(c.performances,key={it.caseId}){performance->val usual=performance.algorithms.maxByOrNull{it.count};Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).clickable{c.selectedCase=performance.caseId;c.selectedVariant=(usual?.variant?:1)-1;c.page="Case"}.padding(vertical=15.dp),verticalAlignment=Alignment.CenterVertically){Column(Modifier.weight(1f)){Text(performance.name,fontSize=15.sp,fontWeight=FontWeight.Medium);Spacer(Modifier.height(4.dp));Label("${performance.count} executions · ${performance.algorithms.size} sequences",12)};Column(horizontalAlignment=Alignment.End){Text(formatTime(usual?.meanExecution),fontSize=22.sp);Label("Recognition ${formatTime(usual?.meanRecognition)}",12)}};DividerLine()}
        val suggestions=c.performances.mapNotNull{it.suggestion}.take(4)
        if(suggestions.isNotEmpty())item{SectionTitle("Next practice")}
        items(suggestions){suggestion->Column(Modifier.fillMaxWidth().padding(vertical=13.dp)){Text(suggestion.title,fontSize=15.sp,fontWeight=FontWeight.Medium);Spacer(Modifier.height(7.dp));Label(suggestion.reason,13)};DividerLine()}
    }
}

@Composable internal fun History(c:LabController){
    var kind by remember{mutableStateOf("Solves")};var session by remember{mutableStateOf(c.settings.session)};var menu by remember{mutableStateOf(false)}
    val records=c.records.filter{(kind=="All"||it.kind==if(kind=="Solves")"solve"else "drill")&&(session=="All sessions"||it.session==session)}
    BoxWithConstraints(Modifier.fillMaxSize()){
        val wide=maxWidth>=700.dp
        Column(Modifier.fillMaxSize().padding(horizontal=if(wide)54.dp else 22.dp)){
            Row(Modifier.fillMaxWidth().height(52.dp),verticalAlignment=Alignment.CenterVertically){
                Text("Solves",fontSize=25.sp,fontWeight=FontWeight.Medium,letterSpacing=(-.6).sp)
                Text(records.size.toString(),fontSize=12.sp,color=Muted,modifier=Modifier.padding(start=12.dp))
                Spacer(Modifier.weight(1f))
                Box{TextButton({menu=true}){Text(session+" ⌄",fontSize=12.sp,color=Muted,maxLines=1,overflow=TextOverflow.Ellipsis,modifier=Modifier.widthIn(max=if(wide)240.dp else 150.dp))};DropdownMenu(menu,{menu=false},containerColor=Panel){for(name in listOf("All sessions")+c.records.map{it.session}.distinct())DropdownMenuItem(text={Text(name)},onClick={session=name;menu=false})}}
            }
            Row(Modifier.fillMaxWidth().padding(bottom=12.dp),verticalAlignment=Alignment.CenterVertically){
                for(value in listOf("Solves","Drills","All"))TextButton({kind=value},contentPadding=PaddingValues(horizontal=10.dp)){Text(value,fontSize=12.sp,color=if(kind==value)Accent else Muted)}
                if(wide){Spacer(Modifier.weight(1f));val a=retainedAverage(records,5);if(a.time!=null)Text("ao5 ${if(a.timeEstimated)"≈"else ""}${formatTime(a.time)} · ${if(a.estimated)"≈"else ""}${metric(a.tps)} TPS · ${metric(a.moves,1)} moves",fontSize=11.sp,color=Muted)}
            }
            Row(Modifier.fillMaxWidth().height(30.dp),verticalAlignment=Alignment.CenterVertically){
                Text("#",fontSize=10.sp,color=Muted,modifier=Modifier.width(30.dp))
                Text("recorded",fontSize=10.sp,color=Muted,modifier=if(wide)Modifier.width(100.dp)else Modifier.weight(1f))
                if(wide)Text("session",fontSize=10.sp,color=Muted,modifier=Modifier.weight(1f))
                for((label,width)in listOf("time" to 64,"TPS" to 48,(if(wide)"scramble TPS"else "prep TPS") to if(wide)86 else 62,"moves" to 40))Text(label,fontSize=10.sp,color=Muted,modifier=Modifier.width(width.dp),textAlign=androidx.compose.ui.text.style.TextAlign.End)
            }
            DividerLine()
            LazyColumn(Modifier.weight(1f),contentPadding=PaddingValues(bottom=20.dp)){
                if(records.isEmpty())item{Empty("No attempts here yet","Completed solves and drills are saved automatically.")}
                itemsIndexed(records.asReversed(),key={_,r->r.id}){index,r->HistoryRow(c,r,records.size-index,wide)}
            }
        }
    }
}
@Composable internal fun HistoryRow(c:LabController,r:AttemptRecord,number:Int,wide:Boolean){
    val execution=recordedExecutionMetric(r)
    Row(Modifier.fillMaxWidth().height(if(wide)38.dp else 44.dp).clickable{c.review=r}
        .semantics{contentDescription="Solve $number, ${scoredTime(r)} seconds, ${metric(execution?.tps)} TPS, ${recordedMoveCount(r)?:0} moves. Open reconstruction."},verticalAlignment=Alignment.CenterVertically){
        Text(number.toString(),fontSize=10.sp,color=Muted,modifier=Modifier.width(30.dp))
        Text(r.createdAt.substringAfter('T').take(5),fontSize=11.sp,color=Muted,modifier=if(wide)Modifier.width(100.dp)else Modifier.weight(1f),maxLines=1)
        if(wide)Text(if(r.kind=="drill")c.catalog.cases.firstOrNull{it.id==r.caseId}?.name?:r.session else r.session,fontSize=12.sp,color=Muted,modifier=Modifier.weight(1f),maxLines=1,overflow=TextOverflow.Ellipsis)
        Text(scoredTime(r),fontSize=15.sp,modifier=Modifier.width(64.dp),textAlign=androidx.compose.ui.text.style.TextAlign.End,style=TextStyle(fontFeatureSettings="tnum"))
        Text((if(execution?.estimated==true)"≈"else "")+metric(execution?.tps),fontSize=12.sp,color=Muted,modifier=Modifier.width(48.dp),textAlign=androidx.compose.ui.text.style.TextAlign.End)
        Text(metric(r.scrambleStats?.tps),fontSize=12.sp,color=Muted,modifier=Modifier.width(if(wide)86.dp else 62.dp),textAlign=androidx.compose.ui.text.style.TextAlign.End)
        Text(recordedMoveCount(r)?.toString()?:"—",fontSize=12.sp,color=Muted,modifier=Modifier.width(40.dp),textAlign=androidx.compose.ui.text.style.TextAlign.End)
    };DividerLine()
}

@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun Settings(c:LabController,onExport:(String)->Unit,onImport:(String)->Unit){
    var endpoint by remember(c.settings.endpoint){mutableStateOf(c.settings.endpoint)};var token by remember(c.settings.token){mutableStateOf(c.settings.token)};var mac by remember(c.settings.mac){mutableStateOf(c.settings.mac)}
    var session by remember(c.settings.session){mutableStateOf(c.settings.session)};var cross by remember(c.settings.cross){mutableStateOf(c.settings.cross)};var confirmReset by remember{mutableStateOf(false)};var advancedCube by remember{mutableStateOf(false)};var advancedServer by remember{mutableStateOf(false)}
    Box(Modifier.fillMaxSize(),contentAlignment=Alignment.TopCenter){
    Column(Modifier.widthIn(max=712.dp).fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal=16.dp).padding(top=24.dp,bottom=28.dp)){
        NativeText("Settings",18,weight=FontWeight.SemiBold)
        SettingsSection("Appearance")
        Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(9.dp)){
            for((id,name,colors)in listOf(Triple("carbon","Dark",Graphite),Triple("paper","Light",Paper))){
                val selected=palette(c.settings.theme)==colors
                Column(Modifier.weight(1f).clip(RoundedCornerShape(3.dp)).border(if(selected)1.5.dp else 1.dp,if(selected)Accent else Line,RoundedCornerShape(3.dp)).clickable(enabled=c.active==null){runCatching{c.saveSettings(c.settings.copy(theme=id))}.onFailure{c.message=it.message?:"Finish the attempt before changing theme"}}.padding(10.dp)){
                    Row(Modifier.fillMaxWidth().height(40.dp).clip(RoundedCornerShape(3.dp)).background(colors.background).padding(8.dp),verticalAlignment=Alignment.CenterVertically){Box(Modifier.width(12.dp).fillMaxHeight().clip(RoundedCornerShape(3.dp)).background(colors.surface));Spacer(Modifier.width(6.dp));Column(Modifier.weight(1f),verticalArrangement=Arrangement.spacedBy(4.dp)){Box(Modifier.fillMaxWidth(.85f).height(4.dp).background(colors.text,RoundedCornerShape(3.dp)));Box(Modifier.fillMaxWidth(.6f).height(3.dp).background(colors.muted,RoundedCornerShape(3.dp)))};Box(Modifier.size(7.dp).background(colors.accent,RoundedCornerShape(4.dp)))}
                    Spacer(Modifier.height(10.dp));NativeText(name,14,Foreground,weight=if(selected)FontWeight.SemiBold else FontWeight.Normal)
                }
            }
        }
        SettingsSection("Practice")
        Field("Session name",session,{session=it});Label("Cross color")
        Spacer(Modifier.height(8.dp))
        Column(verticalArrangement=Arrangement.spacedBy(4.dp)){
            for(colors in listOf("U" to "White","D" to "Yellow","F" to "Green","B" to "Blue","R" to "Red","L" to "Orange").chunked(3)){
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(4.dp)){
                    for((face,name)in colors)Box(Modifier.weight(1f)){Segment(name,cross==face){cross=face}}
                }
            }
        }
        SettingsSection("Cube",if(c.connected)"Connected"else c.adapterStatus)
        if(advancedCube)Field("Bluetooth address",mac,{mac=it.uppercase()})
        FlowRow(horizontalArrangement=Arrangement.spacedBy(8.dp),verticalArrangement=Arrangement.spacedBy(8.dp)){if(c.connected)SmallButton("Disconnect"){c.disconnect()}else SmallButton("Find cube"){c.scan()};SmallButton("Read state",enabled=c.connected){c.requestState()};QuietButton("Calibrate solved",enabled=c.connected){confirmReset=true}}
        QuietButton("Manual address"){advancedCube=!advancedCube}
        SettingsSection("Sync & storage")
        if(advancedServer){Field("HTTPS endpoint",endpoint,{endpoint=it});Field("Access key",token,{token=it},secret=true)}
        FlowRow(horizontalArrangement=Arrangement.spacedBy(8.dp),verticalArrangement=Arrangement.spacedBy(8.dp)){SmallButton("Import connection"){onImport("config")};QuietButton(if(c.syncBusy)"Syncing…"else "Sync now",enabled=!c.syncBusy){c.launch{c.sync()}}}
        QuietButton("Connection settings"){advancedServer=!advancedServer}
        Spacer(Modifier.height(6.dp));Label(c.syncStatus)

        SettingsSection("Backups")
        FlowRow(horizontalArrangement=Arrangement.spacedBy(8.dp),verticalArrangement=Arrangement.spacedBy(8.dp)){SmallButton("Export backup"){onExport(c.exportBackup())};QuietButton("Import backup"){onImport("backup")}}
        Spacer(Modifier.height(22.dp));SmallButton("Save changes",primary=true,enabled=c.active==null){runCatching{c.saveSettings(c.settings.copy(endpoint=endpoint.trim(),token=token.trim(),mac=mac.trim(),session=session.trim().ifBlank{"Practice"},cross=cross))}.onFailure{c.message=it.message?:"Could not save"}}
        Spacer(Modifier.height(20.dp));var about by remember{mutableStateOf(false)}
        QuietButton("About & measurement"){about=!about}
        if(about){Label("Qbing · Kotlin Android",12);Spacer(Modifier.height(8.dp));Label("HTM: R, R′ and R2 each count as one move. Recognition: cue to first turn.",12);Spacer(Modifier.height(8.dp));Label("Algorithms: Tao Yu / Alg-Trainer · MIT\nGAN protocol: gan-web-bluetooth · MIT",12)}
    }
    }
    if(confirmReset)ResetCubeDialog(c,{confirmReset=false})
}
@Composable private fun SettingsSection(title:String,aside:String=""){
    Spacer(Modifier.height(24.dp));DividerLine();Spacer(Modifier.height(16.dp))
    Row(Modifier.fillMaxWidth(),verticalAlignment=Alignment.CenterVertically){NativeText(title,14,weight=FontWeight.Medium,modifier=Modifier.weight(1f));if(aside.isNotBlank())NativeText(aside,12,Faint)}
    Spacer(Modifier.height(12.dp))
}
@Composable internal fun Field(label:String,value:String,onChange:(String)->Unit,secret:Boolean=false){Column(Modifier.fillMaxWidth().padding(bottom=16.dp)){Label(label);Spacer(Modifier.height(7.dp));TextField(value,onChange,singleLine=true,modifier=Modifier.fillMaxWidth(),shape=RoundedCornerShape(3.dp),textStyle=LocalTextStyle.current.copy(fontSize=14.sp),visualTransformation=if(secret)PasswordVisualTransformation()else androidx.compose.ui.text.input.VisualTransformation.None,colors=TextFieldDefaults.colors(focusedContainerColor=Panel,unfocusedContainerColor=Panel,focusedIndicatorColor=Accent.copy(alpha=.5f),unfocusedIndicatorColor=Color.Transparent,cursorColor=Accent))}}

@Composable internal fun DeviceDialog(c:LabController){
 val off=c.adapterStatus.contains("off",true)
 Dialog(onDismissRequest={c.showDevices=false},properties=DialogProperties(usePlatformDefaultWidth=false)){
  Surface(color=Panel,shape=RoundedCornerShape(8.dp),border=BorderStroke(1.dp,Line),modifier=Modifier.padding(24.dp).widthIn(max=408.dp).fillMaxWidth()){
   Column(Modifier.padding(24.dp)){
    Row(verticalAlignment=Alignment.CenterVertically){Text("Your cube",fontSize=20.sp,fontWeight=FontWeight.SemiBold,modifier=Modifier.weight(1f));GlyphButton("Close","Close connection"){c.showDevices=false}}
    Spacer(Modifier.height(20.dp))
    Row(verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(8.dp)){
     Box(Modifier.size(6.dp).background(if(off)Amber else if(c.connected)Green else Accent,androidx.compose.foundation.shape.CircleShape))
     Text(when{off->"Bluetooth is off";c.connected->"Connected";c.connectionBusy->"Connecting…";c.scanning->"Finding your cube…";else->"Nearby cubes"},fontSize=14.sp,fontWeight=FontWeight.Medium)
     if(c.scanning)CircularProgressIndicator(Modifier.size(14.dp),color=Accent,strokeWidth=1.5.dp)
    }
    Spacer(Modifier.height(8.dp));Text(when{off->"Enable Bluetooth to find your cube.";c.connected->"Ready for your next solve.";else->"Wake your cube with a turn. Close other cube apps."},fontSize=14.sp,lineHeight=21.sp,color=Muted)
    Spacer(Modifier.height(20.dp))
    if(off){SmallButton("Bluetooth settings"){c.openBluetoothSettings()}}
    else if(c.connected){Text(c.deviceName,fontSize=16.sp,fontWeight=FontWeight.Medium);Spacer(Modifier.height(16.dp));QuietButton("Disconnect"){c.disconnect()}}
    else {
     for(device in c.devices){Surface(onClick={if(!c.connectionBusy)c.connect(device)},color=Raised,shape=RoundedCornerShape(4.dp),border=BorderStroke(1.dp,Line),modifier=Modifier.fillMaxWidth().padding(bottom=8.dp)){
      Row(Modifier.padding(14.dp),verticalAlignment=Alignment.CenterVertically){Text(device.name,fontSize=16.sp,fontWeight=FontWeight.Medium,modifier=Modifier.weight(1f));Text("Pair  →",fontSize=14.sp,color=Accent)}
     }}
     if(c.devices.isEmpty()&&!c.scanning&&!c.connectionBusy){Text("No cubes found. Keep yours nearby and try again.",fontSize=13.sp,color=Muted);Spacer(Modifier.height(16.dp))}
     OutlinedButton(onClick={c.scan()},enabled=!c.scanning&&!c.connectionBusy,shape=RoundedCornerShape(4.dp),border=BorderStroke(1.dp,Line),modifier=Modifier.fillMaxWidth().height(40.dp)){Text("Scan again",fontSize=14.sp,color=Accent)}
    }
   }
  }
 }
}

@Composable fun TraceChart(values:List<Double>,modifier:Modifier=Modifier,estimated:Set<Int> = emptySet()){
    val valid=values.mapIndexedNotNull{i,value->if(value.isFinite())i to value else null};val colors=LocalColors.current;val measurer=rememberTextMeasurer();val labelStyle=LocalTextStyle.current.copy(fontSize=12.sp,color=colors.muted)
    if(valid.isEmpty()){Box(modifier.clip(RoundedCornerShape(14.dp)).background(colors.surface).padding(18.dp),contentAlignment=Alignment.Center){Column(horizontalAlignment=Alignment.CenterHorizontally){Text("Your trend starts with the first attempt",fontSize=12.sp,color=colors.muted);Spacer(Modifier.height(10.dp));Canvas(Modifier.fillMaxWidth(.6f).height(24.dp)){drawLine(colors.line,Offset(0f,size.height/2),Offset(size.width,size.height/2),1.dp.toPx())}}};return}
    val low=(valid.minOf{it.second}*.85).coerceAtLeast(0.0);val high=maxOf(low+1000,valid.maxOf{it.second}*1.1)
    Canvas(modifier){
        val left=36.dp.toPx();val top=12.dp.toPx();val width=size.width-left-5.dp.toPx();val height=size.height-top-22.dp.toPx()
        for(n in 0..3){val y=top+height*n/3;drawLine(colors.line,Offset(left,y),Offset(left+width,y),1f);drawText(measurer,formatTime(high-(high-low)*n/3),topLeft=Offset(0f,y-7.dp.toPx()),style=labelStyle)}
        fun point(value:Pair<Int,Double>)=Offset(left+(if(values.size>1)value.first.toFloat()/(values.size-1)*width else width*.5f),top+height-((value.second-low)/(high-low)*height).toFloat())
        val path=Path();valid.forEachIndexed{i,value->val p=point(value);if(i==0)path.moveTo(p.x,p.y)else path.lineTo(p.x,p.y)}
        drawPath(path,colors.accent,style=Stroke(1.8.dp.toPx(),cap=StrokeCap.Round));for(value in valid){if(value.first in estimated){drawCircle(colors.background,3.2.dp.toPx(),point(value));drawCircle(colors.accent,3.2.dp.toPx(),point(value),style=Stroke(1.4.dp.toPx()))}else drawCircle(colors.accent,2.3.dp.toPx(),point(value))}
        drawText(measurer,"First",topLeft=Offset(left,size.height-15.dp.toPx()),style=labelStyle);val latest=measurer.measure("Latest",style=labelStyle);drawText(latest,topLeft=Offset(size.width-latest.size.width,size.height-15.dp.toPx()))
    }
}
@Composable fun CasePattern(state:String,stage:String,modifier:Modifier=Modifier){
    if(stage=="Cross"||stage=="F2L"){CubeCanvas(state,modifier);return}
    val line=Line
    Canvas(modifier){val unit=min(size.width,size.height)/5.5f;fun tile(x:Float,y:Float,face:Char){val color=if(stage=="OLL"){if(face=='U')faces.getValue('D')else line}else if(face=='U')faces.getValue('D')else if(face=='D')faces.getValue('U')else faces.getValue(face);drawRoundRect(color,Offset(x*unit,y*unit),Size(unit*.87f,unit*.87f),androidx.compose.ui.geometry.CornerRadius(unit*.1f))};for(row in 0..2)for(col in 0..2)tile(1.2f+col,1.2f+row,state[row*3+col]);for(j in 0..2){tile(1.2f+j,.1f,state[47-j]);tile(1.2f+j,4.3f,state[18+j]);tile(.1f,1.2f+j,state[36+j]);tile(4.3f,1.2f+j,state[11-j])}}
}

@Composable internal fun ReconstructionCases(c:LabController,r:AttemptRecord,stage:String,data:JsonObject?,busy:Boolean,rawStart:Int?=null,heading:String?=null){
 if(busy)return
 val groups=data?.get("groups")?.jsonArray.orEmpty().map{it.jsonObject}.filter{it["stage"]?.jsonPrimitive?.content==stage&&(it["rawEnd"]?.jsonPrimitive?.int?:0)>(it["rawStart"]?.jsonPrimitive?.int?:0)&&(rawStart==null||it["rawStart"]?.jsonPrimitive?.int==rawStart)}
 val clipboard=LocalClipboardManager.current
 for(group in groups){key(r.id,stage,group["rawStart"].toString()){
  var expanded by remember{mutableStateOf(false)}
  val name=group["caseName"]?.jsonPrimitive?.content?:"Unclassified"
  val label=heading?:if(stage=="F2L")"${group["label"]!!.jsonPrimitive.content} · $name"else if(stage=="Cross")"Solutions"else name
  Column(Modifier.fillMaxWidth().padding(vertical=4.dp)){
   Row(Modifier.fillMaxWidth().heightIn(min=36.dp).clickable{expanded=!expanded}.padding(horizontal=8.dp),verticalAlignment=Alignment.CenterVertically){NativeText(if(expanded)"▾"else "▸",12,Faint);Spacer(Modifier.width(8.dp));NativeText(label,12,Muted,weight=FontWeight.Medium)}
   if(expanded){
    val variants=group["variants"]?.jsonArray.orEmpty()
    if(variants.isEmpty())Label("No verified alternative.",12)
    for(value in variants){val v=value.jsonObject;val alg=(v["viewAlgorithm"]?:v["algorithm"])!!.jsonPrimitive.content
     NativeText("${v["name"]!!.jsonPrimitive.content} · ${v["moves"]!!.jsonPrimitive.int} moves",12,Muted)
     SelectionContainer{Text(alg,fontFamily=LocalMono.current,fontSize=14.sp,lineHeight=23.sp,color=Foreground)}
     Row(horizontalArrangement=Arrangement.spacedBy(12.dp)){QuietButton("Copy"){clipboard.setText(AnnotatedString(alg))};v["caseId"]?.jsonPrimitive?.content?.let{id->QuietButton("Practice"){c.prepareCase(id,v["variantIndex"]?.jsonPrimitive?.int?:0);c.review=null}}}
    }
   }
  }
 }}
}
