package lab.cube.ui

import androidx.compose.foundation.*
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.ui.*
import androidx.compose.ui.draw.*
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.*
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.*
import androidx.compose.ui.unit.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import lab.cube.core.*
import kotlin.math.*

internal val Faint:Color @Composable get()=if(LocalColors.current.light)Color(0xff636f80)else Color(0xff939fb1)
internal val Secondary:Color @Composable get()=if(LocalColors.current.light)Color(0xff087f82)else Color(0xff74ccbf)
@Composable internal fun NativeWorkspace(c:LabController,onExport:(String)->Unit,onImport:(String)->Unit){
 var selected by remember{mutableStateOf<String?>(null)}
 LaunchedEffect(c.latest?.id){selected=null}
 var tool by remember{mutableStateOf(0)};var toolsOpen by remember{mutableStateOf(false)};var tags by remember{mutableStateOf(false)}
 val records=remember(c.records,c.settings.session){c.records.filter{it.kind=="solve"&&it.session==c.settings.session}}
 val record=records.firstOrNull{it.id==selected}?:c.latest?:records.lastOrNull()
 BoxWithConstraints(Modifier.fillMaxSize().background(Ink)){
  val mobile=maxWidth<=760.dp
  val workspaceWidth=maxWidth;val workspaceHeight=maxHeight
  val rails=!mobile&&workspaceWidth>=1180.dp&&workspaceHeight>=636.dp
  val railAverages=rails&&(workspaceWidth<=1180.dp||workspaceHeight<=740.dp)
  Column(Modifier.fillMaxSize()){
   NativeHeader(c,mobile)
   Box(Modifier.weight(1f).fillMaxWidth()){
    when(c.page){
     "Practice"->if(!rails)NativeTimer(c,record,records,mobile,Modifier.fillMaxSize(),onTools={toolsOpen=true},onTags={tags=true},showTools=true,onTool={tool=it;toolsOpen=true})else{
      val rail=(workspaceWidth.value*.16f).coerceIn(256f,272f).dp
      Row(Modifier.fillMaxSize()){
       NativeHistory(c,records,record,false,Modifier.width(rail).fillMaxHeight(),railAverages=railAverages){selected=it.id}
       NativeTimer(c,record,records,false,Modifier.weight(1f).fillMaxHeight(),railAverages=railAverages,onTools={toolsOpen=true},onTags={tags=true},onTool={tool=it})
       NativeTools(c,record,tool,{tool=it},Modifier.width(rail).fillMaxHeight().padding(16.dp,24.dp))
      }
     }
     "History"->NativeHistory(c,records,record,true,Modifier.fillMaxSize()){selected=it.id;c.page="Practice"}
     "Cases"->Cases(c)
     "Case"->CaseDetail(c)
     "Progress"->ProgressPage(c)
     else->Settings(c,onExport,onImport)
    }
   }
   if(mobile)Row(Modifier.fillMaxWidth().height(48.dp).borderTop(Line),verticalAlignment=Alignment.CenterVertically){for((page,label)in listOf("Practice" to "Timer","History" to "History","Cases" to "Practice")){
    val selected=c.page==page
    Box(Modifier.weight(1f).fillMaxHeight().clickable(role=Role.Tab){c.page=page},contentAlignment=Alignment.Center){
     if(selected)Box(Modifier.align(Alignment.TopCenter).width(56.dp).height(1.dp).background(Accent))
     Row(verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(8.dp)){Glyph(if(page=="Cases")"Cube"else page,if(selected)Accent else Faint,14);NativeText(label,12,if(selected)Accent else Muted,weight=FontWeight.Medium)}
    }
   }}
  }
  c.errorMessage?.let{error->Surface(Modifier.align(Alignment.BottomCenter).padding(horizontal=16.dp,vertical=if(mobile)60.dp else 20.dp),color=Raised,shape=RoundedCornerShape(3.dp),border=BorderStroke(1.dp,Line)){Row(Modifier.padding(start=12.dp),verticalAlignment=Alignment.CenterVertically){Text(error,fontSize=12.sp,color=Red,lineHeight=18.sp,modifier=Modifier.weight(1f,false));IconButton({c.errorMessage=null},Modifier.size(32.dp).semantics{contentDescription="Dismiss error"}){Glyph("Close",Faint,14)}}}}
  if(toolsOpen)AlertDialog(onDismissRequest={toolsOpen=false},containerColor=Raised,shape=RoundedCornerShape(4.dp),text={NativeTools(c,record,tool,{tool=it},Modifier.fillMaxWidth().heightIn(max=500.dp))},confirmButton={TextButton({toolsOpen=false}){Text("Close")}})
  if(tags&&record!=null)NativeTags(c,record){tags=false}
 }
}
internal fun Modifier.borderTop(color:Color)=drawBehind{drawLine(color,Offset.Zero,Offset(size.width,0f),1.dp.toPx())}
@Composable internal fun NativeText(value:String,size:Int=14,color:Color=Foreground,mono:Boolean=false,weight:FontWeight=FontWeight.Normal,modifier:Modifier=Modifier,align:TextAlign?=null){Text(value,modifier=modifier,fontFamily=if(mono)LocalMono.current else LocalTextStyle.current.fontFamily,fontSize=size.sp,lineHeight=(size*1.5).sp,fontWeight=weight,letterSpacing=0.sp,color=color,textAlign=align,style=TextStyle(fontFeatureSettings="tnum,lnum"),maxLines=1,overflow=TextOverflow.Clip)}
@Composable internal fun NativeTab(label:String,selected:Boolean,modifier:Modifier=Modifier,onClick:()->Unit){Box(modifier.clip(RoundedCornerShape(3.dp)).background(if(selected)Accent.copy(alpha=.063f)else Color.Transparent).clickable(role=Role.Tab,onClick=onClick).padding(horizontal=10.dp),contentAlignment=Alignment.Center){NativeText(label,color=if(selected)Accent else Muted,weight=if(selected)FontWeight.SemiBold else FontWeight.Medium)}}
@Composable internal fun NativeHeader(c:LabController,mobile:Boolean){
 val edge=Line
 BoxWithConstraints(Modifier.fillMaxWidth().height(if(mobile)88.dp else 56.dp).measureNative(".app-header").drawBehind{drawLine(edge,Offset(0f,size.height),Offset(size.width,size.height),1.dp.toPx())}){
  val showSession=maxWidth>=1180.dp
  Row(Modifier.fillMaxWidth().height(56.dp).padding(horizontal=if(mobile)16.dp else 24.dp),verticalAlignment=Alignment.CenterVertically){
   BrandMark(18);Spacer(Modifier.width(8.dp));NativeText("Qbing",weight=FontWeight.SemiBold,modifier=Modifier.clickable{c.page="Practice"})
   if(!mobile){if(showSession){Spacer(Modifier.width(26.dp));NativeText(c.settings.session,color=Muted,modifier=Modifier.clickable{c.page="Settings"})}}
   Spacer(Modifier.weight(1f))
   Box(Modifier.height(if(mobile)44.dp else 32.dp).width(if(mobile)44.dp else 120.dp).clip(RoundedCornerShape(4.dp)).background(if(c.connected||mobile)Color.Transparent else Accent).clickable(role=Role.Button,onClickLabel="Connect Bluetooth cube"){c.showDevices=true;if(!c.connected&&!c.connectionBusy&&!c.adapterStatus.contains("off",true))c.scan()}.semantics{contentDescription=if(c.connected)c.deviceName else "Connect Bluetooth cube"},contentAlignment=Alignment.Center){
    if(mobile)Glyph("Bluetooth",if(c.connected)Muted else Accent,16)else NativeText(if(c.connected)c.deviceName else if(c.connectionBusy)"Pairing…"else "Connect",color=if(c.connected)Muted else LocalColors.current.onAccent,weight=FontWeight.Medium)
    Box(Modifier.align(Alignment.TopEnd).padding(5.dp).size(5.dp).background(if(c.connected)Green else Faint,CircleShape))
   }
   Spacer(Modifier.width(12.dp));GlyphButton("Settings","Options"){c.page="Settings"}
  }
  if(!mobile)Row(Modifier.align(Alignment.Center).height(36.dp)){for((page,label)in listOf("Practice" to "Timer","History" to "History","Cases" to "Practice"))NativeTab(label,c.page==page,Modifier.fillMaxHeight()){c.page=page}}
  else Row(Modifier.align(Alignment.BottomStart).height(40.dp).padding(horizontal=16.dp),verticalAlignment=Alignment.CenterVertically){NativeText(c.settings.session,color=Muted,modifier=Modifier.clickable{c.page="Settings"})}
 }
}
@Composable internal fun NativeTimer(c:LabController,record:AttemptRecord?,records:List<AttemptRecord>,mobile:Boolean,modifier:Modifier,railAverages:Boolean=false,onTools:()->Unit,onTags:()->Unit,showTools:Boolean=false,onTool:(Int)->Unit){
 var averagesOpen by remember{mutableStateOf(false)};var phasesOpen by remember{mutableStateOf(false)}
 var reset by remember{mutableStateOf(0)};var resetCube by remember{mutableStateOf(false)}
 val running=c.active?.startedAt!=null
 val liveRecord=remember(c.moves,c.active,running){if(running)c.active?.record(c.catalog)else null}
 val shown=liveRecord?:record
 val bestTime=remember(records){records.map(::adjusted).filter{it.isFinite()&&it>0}.minOrNull()}
 val executionMetric=remember(shown){shown?.let(::recordedExecutionMetric)}
 val moves=if(running)countMoves(c.moves)else record?.let(::recordedMoveCount)
 val time=if(running)c.elapsed else record?.let(::adjusted)?:0.0
 val live=if(running)nativeLiveMetrics(c.moves,c.active?.quality.orEmpty(),time)else null
 val clock=c.cueCountdown?.toString()?:(if(if(running)live?.estimatedStart==true else record?.let(::executionDurationEstimated)==true)"≈ "else "")+webTime(time)
 val clipboard=LocalClipboardManager.current
 var manualStart by remember{mutableStateOf<Double?>(null)}
 var manualElapsed by remember{mutableStateOf(0.0)}
 LaunchedEffect(c.foreground){if(!c.foreground)manualStart=null}
 LaunchedEffect(manualStart){while(manualStart!=null){manualElapsed=monotonicMs()-manualStart!!;delay(16)}}
 BoxWithConstraints(modifier.measureNative(".timer-pane")){
  val g=timerGeometry(maxWidth.value,maxHeight.value,mobile)
  val cy=g.cy;val cubeW=g.cubeWidth;val cubeH=g.cubeHeight;val cubeY=g.cubeTop
  Box(Modifier.align(Alignment.TopCenter).offset(y=cubeY.dp).size(cubeW.dp,cubeH.dp).measureNative(".cube-stage")){
   CubeCanvas(c.cubeState?:Cube.solved,Modifier.fillMaxSize(),turns=c.visualTurns,orientation=c.orientation,resetGyro=reset,onGyroFrame=c::captureGyroFrame)
   Box(Modifier.align(Alignment.BottomEnd).offset(x=20.dp).size(26.dp).semantics{contentDescription="Reset cube view"}.clickable{reset++},contentAlignment=Alignment.Center){Glyph("Refresh",Faint,14)}
  }
  val scrambleY=g.scrambleTop
  Box(Modifier.align(Alignment.TopCenter).offset(y=scrambleY.dp).widthIn(max=620.dp).fillMaxWidth().padding(horizontal=if(mobile)16.dp else 0.dp)){
   NativeScramble(c,mobile,if(g.collapsed)12 else null)
  }
  Column(Modifier.align(Alignment.TopCenter).offset(y=(cy-g.clockHeight/2).dp).fillMaxWidth(),horizontalAlignment=Alignment.CenterHorizontally){
   Box(Modifier.height(g.clockHeight.dp).fillMaxWidth().measureNative(".clock-anchor").semantics{contentDescription="Solve time $clock"}.pointerInput(c.connected,manualStart){if(c.awaitingCue)detectTapGestures(onTap={c.startCue()}) else if(!c.connected)detectTapGestures(onPress={if(manualStart!=null){c.manualTime((monotonicMs()-manualStart!!)/1000.0);manualStart=null;tryAwaitRelease()}else{val now=monotonicMs();if(tryAwaitRelease()&&monotonicMs()-now>=500.0)manualStart=monotonicMs()}})},contentAlignment=Alignment.Center){NativeText(if(c.awaitingCue)"▷"else if(manualStart!=null)webTime(manualElapsed)else clock,g.clockFont,color=if(c.active!=null&&!running||!running&&manualStart==null&&record!=null&&record.kind=="solve"&&record.session==c.settings.session&&adjusted(record)==bestTime)Green else Foreground,mono=true,modifier=Modifier.offset(y=if(mobile)6.dp else 0.dp))}
   Spacer(Modifier.height(if(g.dense)2.dp else if(mobile)14.dp else 2.dp))
   Row(Modifier.height(if(mobile)25.dp else 26.dp),horizontalArrangement=Arrangement.spacedBy(16.dp),verticalAlignment=Alignment.CenterVertically){Row(verticalAlignment=Alignment.CenterVertically){NativeText(if(manualStart!=null)"—"else if(running)live?.tps?.let{(if(live.estimatedStart)"≈ "else "")+metric(it)}?:"—"else executionMetric?.let{(if(it.estimated)"≈ "else "")+metric(it.tps)}?:"—",if(mobile)16 else 18,Secondary,true);Spacer(Modifier.width(6.dp));NativeText("tps",12,Faint)};Row(verticalAlignment=Alignment.CenterVertically){NativeText(if(manualStart!=null)"—"else if(running)live?.moves?.toString()?:"—"else moves?.toString()?:"—",if(mobile)16 else 18,mono=true);Spacer(Modifier.width(6.dp));NativeText("moves",12,Faint)}}
   if(!running&&manualStart==null&&g.actions){
    Row(Modifier.padding(top=if(mobile)6.dp else 10.dp).height(28.dp),horizontalArrangement=Arrangement.spacedBy(4.dp),verticalAlignment=Alignment.CenterVertically){
     for(label in listOf("+2","DNF"))Box(Modifier.width(38.dp).fillMaxHeight().clickable(enabled=record!=null){record?.let{c.save(it.copy(penalty=if(it.penalty==label)""else label))}},contentAlignment=Alignment.Center){NativeText(label,14,if(record?.penalty==label)Accent else Faint)}
     Box(Modifier.width(1.dp).height(12.dp).background(Line));Box(Modifier.size(28.dp).semantics{contentDescription="Favorite solve"}.clickable(enabled=record!=null){record?.let{c.save(it.copy(localReview=it.localReview.copy(favorite=!it.localReview.favorite)))}},contentAlignment=Alignment.Center){Glyph("Favorite",if(record?.localReview?.favorite==true)Accent else Faint,16)}
     Box(Modifier.size(28.dp).semantics{contentDescription="Tag solve"}.clickable(enabled=record!=null,onClick=onTags),contentAlignment=Alignment.Center){Glyph("Tag",Faint,16)}
    }
    if(!railAverages&&g.showAverages){Spacer(Modifier.height(if(mobile)12.dp else 20.dp));NativeAverages(records,mobile)}
   }
  }
  if(shown!=null&&manualStart==null){
   val dock=Modifier.align(Alignment.BottomCenter).padding(bottom=if(g.dense)8.dp else if(mobile)4.dp else 24.dp).widthIn(max=736.dp).fillMaxWidth().padding(horizontal=16.dp)
   if(g.collapsed)Row(dock.height(32.dp),verticalAlignment=Alignment.CenterVertically){NativeText("CFOP ↗",12,Accent,modifier=Modifier.clickable{phasesOpen=true});Spacer(Modifier.width(18.dp));NativeText("Averages",12,Muted,modifier=Modifier.clickable{averagesOpen=true});Spacer(Modifier.weight(1f));NativeText("Tools",12,Muted,modifier=Modifier.clickable(onClick=onTools))}
   else NativePhases(shown,g.compact,dock,onTool,onTools,if(running)time else null,dense=g.dense,showTools=showTools,onAverages=if(!g.showAverages&&!railAverages)({averagesOpen=true})else null){record?.let{c.review=it}}
  }

 }
 if(averagesOpen)AlertDialog(onDismissRequest={averagesOpen=false},containerColor=Panel,shape=RoundedCornerShape(6.dp),title={Text("Averages")},text={NativeAverages(records,true)},confirmButton={TextButton({averagesOpen=false}){Text("Close")}})
 if(phasesOpen&&shown!=null)AlertDialog(onDismissRequest={phasesOpen=false},containerColor=Panel,shape=RoundedCornerShape(6.dp),text={NativePhases(shown,true,Modifier.fillMaxWidth(),onTool,onTools){record?.let{c.review=it}}},confirmButton={TextButton({phasesOpen=false}){Text("Close")}})
 if(resetCube)ResetCubeDialog(c,{resetCube=false}){c.calibrate();resetCube=false}
}
@Composable internal fun NativeScramble(c:LabController,mobile:Boolean,fontOverride:Int?=null){
 val setup=c.target?.position?.setup.orEmpty();val marks=c.target?.let{checkpointProgress(c.cubeState,c.scrambleCheckpoint,it.position.setup)}
 val font=fontOverride?:if(mobile)16 else 18;val mono=LocalMono.current;val clipboard=LocalClipboardManager.current;val measurer=rememberTextMeasurer()
 val items=scrambleDisplayTokens(setup,marks,c.recoveryMoves)+("" to 5)
 BoxWithConstraints(Modifier.fillMaxWidth().measureNative(".scramble-region")){
  val density=androidx.compose.ui.platform.LocalDensity.current;val gap=font*.6f+2f
  val tps=c.scrambleSpeed.tps?.let{metric(it)+" tps"}
  val widths=items.map{(m,kind)->if(kind==5)26f+(if(tps!=null)60f else 0f)else with(density){measurer.measure(m,TextStyle(fontFamily=mono,fontSize=font.sp)).size.width.toDp().value}+gap}
  val count=ceil(widths.sum()/maxWidth.value).toInt().coerceAtLeast(1);val target=widths.sum()/count
  val lines=mutableListOf(mutableListOf<Int>());var used=0f;for((i,w)in widths.withIndex()){if(used+w>target+w*.6f&&lines.size<count){lines.add(mutableListOf());used=0f};lines.last().add(i);used+=w}
  Column(Modifier.fillMaxWidth(),horizontalAlignment=Alignment.CenterHorizontally){for(indices in lines){
   Row(horizontalArrangement=Arrangement.Center,verticalAlignment=Alignment.CenterVertically){for(i in indices){val(m,kind)=items[i]
    if(kind==5){if(tps!=null)NativeText(tps,12,Faint,modifier=Modifier.padding(start=6.dp));Box(Modifier.size(26.dp).clickable{clipboard.setText(AnnotatedString(setup.joinToString(" ")))},contentAlignment=Alignment.Center){Glyph("Copy",Faint,14)}}
    else {val color=when(kind){3->Red;2->LocalColors.current.onAccent;else->Muted};val fill=Accent;val partial=Amber
     Text(m,fontFamily=mono,fontSize=font.sp,lineHeight=(font*1.65).sp,color=if(kind==1)color.copy(alpha=.35f)else color,softWrap=false,modifier=Modifier.padding(horizontal=(gap/2-4).coerceAtLeast(0f).dp).then(if(kind==1)Modifier.blur(.55.dp)else Modifier).drawBehind{
       val top=2.dp.toPx();val h=(size.height-top*2).coerceAtLeast(0f)
       if(kind==2)drawRoundRect(fill,Offset(0f,top),androidx.compose.ui.geometry.Size(size.width,h),androidx.compose.ui.geometry.CornerRadius(2.dp.toPx()))
       if(kind==4){drawRoundRect(partial.copy(alpha=.12f),Offset(0f,top),androidx.compose.ui.geometry.Size(size.width,h));drawRect(partial.copy(alpha=.35f),Offset(0f,top),androidx.compose.ui.geometry.Size(size.width/2,h));drawRoundRect(partial,Offset(0f,top),androidx.compose.ui.geometry.Size(size.width,h),style=androidx.compose.ui.graphics.drawscope.Stroke(1.dp.toPx()))}
     }.padding(horizontal=4.dp).semantics{if(kind==4)stateDescription="Half complete"})

    }
   }}
  }}
 }
}
@Composable internal fun NativeAverages(records:List<AttemptRecord>,mobile:Boolean){
 Row(Modifier.width(if(mobile)320.dp else 336.dp).measureNative(".average-table"),horizontalArrangement=Arrangement.spacedBy(28.dp)){
  for(n in listOf(5,12)){
   val a=remember(records,n){retainedAverage(records,n)}
   Column(Modifier.weight(1f),horizontalAlignment=Alignment.CenterHorizontally){
    Box(Modifier.height(if(mobile)18.dp else 20.dp),contentAlignment=Alignment.Center){NativeText("ao$n",12,Faint,weight=FontWeight.Medium)}
    Spacer(Modifier.height(2.dp))
    Box(Modifier.height(if(mobile)26.dp else 28.dp),contentAlignment=Alignment.Center){NativeText(a.time?.let{webTime(floor(it/10+.5)*10)}?:"—",18,mono=true,weight=FontWeight.SemiBold)}
    Spacer(Modifier.height(2.dp))
    Row(Modifier.height(22.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(8.dp)){NativeText(a.tps?.let{"${metric(it)} tps"}?:"— tps",12,Muted,true);NativeText(a.moves?.let{"${metric(it,1)} moves"}?:"— moves",12,Muted,true)}
   }
  }
 }
}
@Composable internal fun NativePhases(record:AttemptRecord,compact:Boolean,modifier:Modifier,onTool:(Int)->Unit,onTools:()->Unit,liveElapsed:Double?=null,dense:Boolean=false,showTools:Boolean=compact,onAverages:(()->Unit)?=null,onReplay:()->Unit){
 val edge=Line
 val splits=if(liveElapsed!=null||recordedMoveCount(record)!=null)record.analysis?.splits.orEmpty()else emptyList();val colors=if(LocalColors.current.light)listOf(Color(0xff16bda2),Color(0xff5a8fff),Color(0xffedc94b),Color(0xffa282ee))else listOf(Color(0xff3bd6b6),Color(0xff79a6ff),Color(0xfff2d255),Color(0xffba9bff))
 Column(modifier.height(if(dense)if(compact)112.dp else 84.dp else if(compact)172.dp else 130.dp).measureNative(".phase-section").borderTop(Line).padding(top=if(dense)8.dp else 14.dp)){
  Row(Modifier.fillMaxWidth().height(26.dp),verticalAlignment=Alignment.CenterVertically){NativeText("CFOP",12,Muted,weight=FontWeight.SemiBold);Spacer(Modifier.width(10.dp));onAverages?.let{NativeText("Averages",12,Muted,modifier=Modifier.clickable(onClick=it))};Spacer(Modifier.weight(1f));if(showTools)NativeText("Tools",12,Faint,modifier=Modifier.padding(end=10.dp).clickable(onClick=onTools));NativeText("▷ Replay",12,Faint,modifier=Modifier.clickable(onClick=onReplay))}
  Row(Modifier.fillMaxWidth().height(4.dp),horizontalArrangement=Arrangement.spacedBy(2.dp)){for((i,name)in listOf("Cross","F2L","OLL","PLL").withIndex()){val duration=splits.firstOrNull{it.stage==name}?.duration?:1.0;Box(Modifier.weight(duration.toFloat().coerceAtLeast(.001f)).fillMaxHeight().background(colors[i].copy(alpha=.95f)).clickable{onTool(if(i==0)0 else if(i==1)2 else 1)})}}
  Spacer(Modifier.height(if(dense)8.dp else 12.dp))
  val stages=listOf("Cross","F2L","OLL","PLL");val columns=if(compact)2 else 4
  for(row in 0 until 4/columns){Row(Modifier.fillMaxWidth()){
   for(j in 0 until columns){val i=row*columns+j;val split=splits.firstOrNull{it.stage==stages[i]}
    val span=split?.let{if(it.time!=null)it.duration else if(liveElapsed!=null&&it==splits.firstOrNull{s->s.end==null})(liveElapsed-it.start).coerceAtLeast(0.0)else null}
    val pause=if(i==0||split==null||span==null)0.0 else phasePause(record,split,span)
    Column(Modifier.weight(1f).height(if(dense)32.dp else 56.dp).drawBehind{if(j>0)drawLine(edge,Offset.Zero,Offset(0f,size.height),1.dp.toPx());if(row>0)drawLine(edge,Offset.Zero,Offset(size.width,0f),1.dp.toPx())}.clickable{onTool(if(i==0)0 else if(i==1)2 else 1)}.padding(horizontal=if(compact)10.dp else 12.dp)){
     Row(Modifier.fillMaxWidth(),verticalAlignment=Alignment.Top){NativeText(stages[i],12,Muted,modifier=Modifier.weight(1f));PhaseTime(span,pause,i==0,split?.let{splitTimingEstimated(record,it)}?:false,if(dense)14 else 16)}
     if(!dense)Row(Modifier.fillMaxWidth().padding(top=2.dp),horizontalArrangement=Arrangement.SpaceBetween){NativeText(split?.let{"${it.moves} moves"}?:"— moves",12,Muted,true);NativeText(span?.takeIf{it>0}?.let{t->"${metric((split?.moves?:0)*1000/t)} tps"}?:"— tps",12,Muted,true)}
    }
   }
  }}
 }
}
@OptIn(ExperimentalFoundationApi::class)
@Composable internal fun NativeHistory(c:LabController,records:List<AttemptRecord>,selected:AttemptRecord?,full:Boolean,modifier:Modifier,railAverages:Boolean=false,onSelect:(AttemptRecord)->Unit){
 var distributionOpen by remember{mutableStateOf(false)}
 val graphMetric=c.graphMetric
 var deleting by remember{mutableStateOf<AttemptRecord?>(null)}
 var filter by remember{mutableStateOf(0)};var search by remember{mutableStateOf("")};var tagFilter by remember{mutableStateOf("")};var tagMenu by remember{mutableStateOf(false)};var editing by remember{mutableStateOf<AttemptRecord?>(null)}
 val bestTime=remember(records){records.map(::adjusted).filter{it.isFinite()&&it>0}.minOrNull()}
 val bestRecord=remember(records){records.filter{adjusted(it).isFinite()}.minByOrNull(::adjusted)}
 val edge=Line;val accent=Accent
 val rowAo5=remember(records){rollingAo5(records)}
 val list=remember(records,filter,search,tagFilter){records.withIndex().reversed().filter{(_,r)->(when(filter){1->r.moves.isNotEmpty();2->r.penalty=="DNF";3->r.localReview.favorite;else->true})&&(tagFilter.isEmpty()||tagFilter==r.localReview.group||tagFilter==r.localReview.color)&&(search.isBlank()||"${webTime(adjusted(r))} ${r.scramble} ${r.localReview.group} ${r.localReview.color}".contains(search,true))}}
 BoxWithConstraints(modifier.background(if(full)Ink else Panel)){
  val condensed=full&&maxHeight<640.dp;val ultra=full&&maxHeight<400.dp;val twoRows=maxWidth<520.dp&&!condensed;val mobile=maxWidth<=900.dp;val inset=if(full)if(mobile)16.dp else 32.dp else 16.dp
  Column(Modifier.widthIn(max=1120.dp).fillMaxHeight().align(Alignment.TopCenter).padding(horizontal=inset)){
   Row(Modifier.fillMaxWidth().padding(top=if(full&&mobile)12.dp else if(full)20.dp else 16.dp).height(36.dp),verticalAlignment=Alignment.CenterVertically){NativeText(if(full)"History"else c.settings.session,if(full)18 else 14,weight=FontWeight.SemiBold);if(full&&!mobile){Spacer(Modifier.width(24.dp));NativeText(c.settings.session,color=Muted)};Spacer(Modifier.weight(1f));if(condensed)NativeText("Distribution ↗",12,Accent,modifier=Modifier.padding(end=12.dp).clickable{distributionOpen=true});Box(Modifier.size(28.dp).clickable{c.page="Settings"},contentAlignment=Alignment.Center){Glyph("More",Faint,14)}}
   if(full){
    if(!ultra){
    Row(Modifier.fillMaxWidth().padding(vertical=if(mobile)12.dp else 20.dp),horizontalArrangement=Arrangement.spacedBy(40.dp),verticalAlignment=Alignment.CenterVertically){
     Column(Modifier.weight(1f),verticalArrangement=Arrangement.spacedBy(14.dp)){
      val best=bestRecord
      val eligible=remember(records){records.filter{adjusted(it).isFinite()}}
      val labels=listOf("Best","Mean","ao5","ao12")
      for(row in 0 until (if(twoRows)2 else 1)){
       Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(16.dp)){
        for(i in (if(twoRows)row*2 else 0) until (if(twoRows)(row+1)*2 else 4)){
         val a=remember(records,i){if(i>=2)retainedAverage(records,if(i==2)5 else 12)else null}
         val time=if(i==0)best?.let(::adjusted)else if(i==1)eligible.map(::adjusted).takeIf{it.isNotEmpty()}?.average()else a?.time
         val tps=if(i==0)best?.let(::recordedExecutionMetric)?.tps else if(i==1)eligible.mapNotNull{recordedExecutionMetric(it)?.tps}.takeIf{it.isNotEmpty()}?.average()else a?.tps
         val moves=if(i==0)best?.let(::recordedMoveCount)?.toDouble()else if(i==1)eligible.mapNotNull(::recordedMoveCount).takeIf{it.isNotEmpty()}?.average()else a?.moves
         Column(Modifier.weight(1f).then(if(i==0&&best!=null)Modifier.clickable{c.review=best}.semantics{contentDescription="Open best solve reconstruction"}else Modifier)){
          NativeText(labels[i],12,Faint,weight=FontWeight.Medium)
          NativeText(time?.let{webTime(if(i>=2)floor(it/10+.5)*10 else it)}?:"—",if(mobile&&!twoRows)17 else 22,if(i==0)Green else Foreground,true,FontWeight.SemiBold)
          NativeText(tps?.let{"${metric(it)} tps"}?:"— tps",12,Muted,true)
          NativeText(moves?.let{"${metric(it,if(i==0)0 else 1)} moves"}?:"— moves",12,Muted,true)
         }
        }
       }
      }
     }
     if(!mobile)NativePace(records,Modifier.width(320.dp).height(84.dp),metric=graphMetric,onMetric={c.selectGraphMetric(it)})
    }
    }
    if(!condensed){
    if(mobile)Column(Modifier.fillMaxWidth().borderTop(Line).padding(top=10.dp,bottom=14.dp)){NativeHistogram(list.map{it.value},Modifier.fillMaxWidth().height(96.dp),graphMetric){c.selectGraphMetric(it)};Spacer(Modifier.height(12.dp));NativeInsights(list.map{it.value},true,Modifier.fillMaxWidth().height(42.dp),graphMetric)}
    else Row(Modifier.fillMaxWidth().borderTop(Line).padding(top=14.dp,bottom=18.dp),horizontalArrangement=Arrangement.spacedBy(40.dp),verticalAlignment=Alignment.CenterVertically){NativeHistogram(list.map{it.value},Modifier.weight(1f).height(128.dp),graphMetric){c.selectGraphMetric(it)};NativeInsights(list.map{it.value},false,Modifier.width(320.dp).height(112.dp),graphMetric)}
    }
    if(mobile){BasicTextFieldNative(search,{search=it},Modifier.fillMaxWidth().height(36.dp));Spacer(Modifier.height(8.dp))}
    Row(Modifier.fillMaxWidth().height(36.dp),verticalAlignment=Alignment.CenterVertically){
     Row(Modifier.border(1.dp,Line,RoundedCornerShape(3.dp)).padding(2.dp).height(32.dp)){for((i,label)in listOf("All","Moves","DNF",if(mobile)"★"else "★ Favorites").withIndex())Box(Modifier.clip(RoundedCornerShape(2.dp)).background(if(filter==i)Accent.copy(alpha=.07f)else Color.Transparent).clickable{filter=i}.padding(horizontal=if(mobile)8.dp else 10.dp).fillMaxHeight(),contentAlignment=Alignment.Center){NativeText(label,14,if(filter==i)Accent else Muted,weight=FontWeight.Medium)}}
     Spacer(Modifier.width(if(mobile)0.dp else 12.dp));if(mobile)Spacer(Modifier.weight(1f))
     Box{Box(Modifier.width(98.dp).fillMaxHeight().border(1.dp,Line,RoundedCornerShape(3.dp)).clickable{tagMenu=true}.padding(horizontal=8.dp),contentAlignment=Alignment.CenterStart){NativeText(tagFilter.replaceFirstChar{it.uppercase()}.ifBlank{"All tags"},14,Muted)};DropdownMenu(tagMenu,{tagMenu=false}){for(tag in listOf("","review","practice","milestone","blue","mint","amber","rose"))DropdownMenuItem(text={NativeText(tag.ifBlank{"All tags"}.replaceFirstChar{it.uppercase()})},onClick={tagFilter=tag;tagMenu=false})}}
     if(!mobile){Spacer(Modifier.weight(1f));BasicTextFieldNative(search,{search=it},Modifier.width(280.dp).fillMaxHeight())}
    }
    Spacer(Modifier.height(16.dp))
   }else{
    val values=remember(records){records.map(::adjusted).filter{it.isFinite()}}
    Row(Modifier.fillMaxWidth()){
     for((i,pair)in listOf("Best" to bestRecord?.let(::adjusted),"Mean" to values.takeIf{it.isNotEmpty()}?.average()).withIndex()){
      Row(Modifier.weight(1f).then(if(i==0&&bestRecord!=null)Modifier.clickable{c.review=bestRecord}.semantics{contentDescription="Open best solve reconstruction"}else Modifier),verticalAlignment=Alignment.CenterVertically){NativeText(pair.first,12,Faint);Spacer(Modifier.width(8.dp));NativeText(pair.second?.let(::webTime)?:"—",14,if(i==0)Green else Foreground,true,FontWeight.SemiBold)}
     }
    }
    Spacer(Modifier.height(16.dp));NativePace(records,Modifier.fillMaxWidth().height(84.dp),showMeanLabel=false,metric=graphMetric,onMetric={c.selectGraphMetric(it)});Spacer(Modifier.height(20.dp));if(railAverages){NativeRailAverages(records);Spacer(Modifier.height(14.dp))}
   }
   BoxWithConstraints(Modifier.fillMaxWidth().weight(1f)){
    val tableWidth=maxWidth.value-if(full&&!mobile)10f else 0f
    val widths=if(!full)listOf(40f,50f,46f,44f,tableWidth-180f)else if(mobile)listOf(28f,tableWidth-280f,62f,54f,46f,70f,20f)else listOf(48f,80f,64f,64f,64f)+List(4){(tableWidth-672f)/4}+listOf(64f,148f,104f,36f)
    val labels=if(!full)listOf("#","Time","ao5","TPS","Moves")else if(mobile)listOf("#","Time","ao5","TPS","Moves","Tags","")else listOf("#","Time","ao5","TPS","Moves","Cross","F2L","OLL","PLL","Pause","When","Tags","")
    Column(Modifier.fillMaxSize()){
     Row(Modifier.width(tableWidth.dp).height(if(full)32.dp else 30.dp).drawBehind{drawLine(edge,Offset(0f,size.height),Offset(size.width,size.height),1.dp.toPx())},verticalAlignment=Alignment.CenterVertically){for((i,label)in labels.withIndex())NativeText(label,12,Faint,modifier=Modifier.width(widths[i].dp).padding(start=if(i==0)10.dp else if(!full&&i==4)0.dp else 4.dp,end=if(!full&&i==4)10.dp else 4.dp),align=if(i==0||label in listOf("Tags","When"))TextAlign.Start else TextAlign.End)}
     LazyColumn(Modifier.weight(1f).fillMaxWidth()){
      itemsIndexed(list,key={_,v->v.value.id}){_,indexed->val(index,r)=indexed;val active=r.id==selected?.id
       var menu by remember(r.id){mutableStateOf(false)}
       Box {
       Row(Modifier.width(tableWidth.dp).height(if(full)32.dp else 30.dp).background(if(active)Accent.copy(alpha=.045f)else Color.Transparent).drawBehind{if(active)drawLine(accent,Offset.Zero,Offset(0f,size.height),2.dp.toPx())} .combinedClickable(onClick={onSelect(r)},onLongClick={menu=true}),verticalAlignment=Alignment.CenterVertically){
        val values=listOf((index+1).toString(),webScoredTime(r),rowAo5[index]?.let{webTime(floor(it/10+.5)*10)}?:"—",recordedExecutionMetric(r)?.let{metric(it.tps)}?:"—",recordedMoveCount(r)?.toString()?:"—")+if(full&&!mobile)listOf("Cross","F2L","OLL","PLL").map{name->r.analysis?.splits?.firstOrNull{it.stage==name}?.duration?.let(::webTime)?:"—"}+listOf(r.analysis?.splits?.sumOf{phasePause(r,it)}?.let(::webTime)?:"—",nativeCreatedLabel(r.createdAt))else emptyList()
        for((i,value)in values.withIndex())NativeText(value,if(i==0||i==10||((!full||mobile)&&i!=1))12 else 14,if(i==0)Faint else if(i==1&&adjusted(r)==bestTime)Green else if(active&&i==1)Accent else if(i==1)Foreground else Muted,true,weight=if(i==1)FontWeight.SemiBold else FontWeight.Normal,modifier=Modifier.width(widths[i].dp).padding(start=if(i==0)10.dp else 4.dp,end=if(!full&&i==4)10.dp else 4.dp),align=if(i==0||i==10)TextAlign.Start else TextAlign.End)
        if(full){Row(Modifier.width(widths[if(mobile)5 else 11].dp),verticalAlignment=Alignment.CenterVertically){Box(Modifier.size(24.dp).clickable{c.save(r.copy(localReview=r.localReview.copy(favorite=!r.localReview.favorite)))},contentAlignment=Alignment.Center){Glyph("Favorite",if(r.localReview.favorite)Accent else Faint,14)};Box(Modifier.size(24.dp).clickable{editing=r},contentAlignment=Alignment.Center){Glyph("Tag",Faint,14)}};Box(Modifier.width(widths.last().dp).fillMaxHeight().clickable{menu=true}.semantics{contentDescription="Solve actions"},contentAlignment=Alignment.Center){Glyph("More",Faint,14)}}
       }
       DropdownMenu(menu,{menu=false}){
        DropdownMenuItem(text={NativeText("Review solve")},onClick={menu=false;c.review=r})
        DropdownMenuItem(text={NativeText("Delete solve…",color=Red)},onClick={menu=false;deleting=r})
       }
       }
      }
     }
    }
   }
   Row(Modifier.fillMaxWidth().height(40.dp).borderTop(Line),verticalAlignment=Alignment.CenterVertically){NativeText("${records.size} solves",12,Faint);Spacer(Modifier.weight(1f));NativeText((if(full)"Total "else "")+nativeTotal(records.sumOf{it.duration}),12,Faint)}
  }
 }
 deleting?.let{record->AlertDialog(onDismissRequest={deleting=null},containerColor=Panel,shape=RoundedCornerShape(6.dp),title={Text("Delete this solve?")},text={Text("${scoredTime(record)} s · This cannot be undone.",fontSize=14.sp)},confirmButton={TextButton({c.deleteRecord(record);deleting=null}){Text("Delete",color=Red)}},dismissButton={TextButton({deleting=null}){Text("Cancel")}})}
 editing?.let{old->NativeTags(c,c.records.firstOrNull{it.id==old.id}?:old){editing=null}}
 if(distributionOpen)AlertDialog(onDismissRequest={distributionOpen=false},containerColor=Panel,shape=RoundedCornerShape(6.dp),title={Text("Session statistics")},text={Column{NativeHistogram(records,Modifier.fillMaxWidth().height(144.dp),graphMetric){c.selectGraphMetric(it)};Spacer(Modifier.height(12.dp));NativeInsights(records,true,Modifier.fillMaxWidth().height(60.dp),graphMetric)}},confirmButton={TextButton({distributionOpen=false}){Text("Close")}})
}
@Composable internal fun NativeInsights(records:List<AttemptRecord>,mobile:Boolean,modifier:Modifier,metric:GraphMetric=GraphMetric.Time){val d=remember(records,metric){graphSeries(records,metric).histogram(metric,320f)};val dnf=remember(records){nativeDistribution(records).dnfRate};val values=listOf("Median" to d.median?.let(metric::format),"Std dev" to d.spread?.let(metric::format),"P90" to d.p90?.let(metric::format),"DNF" to dnf?.let{"${round(it*100).toInt()}%"});Column(modifier,verticalArrangement=Arrangement.spacedBy(if(mobile)0.dp else 18.dp)){for(row in 0 until if(mobile)1 else 2)Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(if(mobile)8.dp else 24.dp)){for(j in 0 until if(mobile)4 else 2){val(label,value)=values[row*2+j];Column(Modifier.weight(1f)){NativeText(label,12,Faint);NativeText(value?:"—",if(mobile)14 else 18,Muted,true)}}}}}
@Composable internal fun BasicTextFieldNative(value:String,onValue:(String)->Unit,modifier:Modifier){
 Box(modifier.border(1.dp,Line,RoundedCornerShape(3.dp)).padding(horizontal=10.dp),contentAlignment=Alignment.CenterStart){
  androidx.compose.foundation.text.BasicTextField(value,onValue,modifier=Modifier.fillMaxWidth(),singleLine=true,textStyle=LocalTextStyle.current.copy(fontSize=14.sp,lineHeight=21.sp,color=Foreground),decorationBox={inner->Box(contentAlignment=Alignment.CenterStart){if(value.isEmpty())NativeText("Find a solve or tag",14,Faint);inner()}})
 }
}

@Composable internal fun GraphTabs(metric:GraphMetric,onChange:(GraphMetric)->Unit){
 Row(horizontalArrangement=Arrangement.spacedBy(2.dp)){
  for(item in GraphMetric.entries)Box(Modifier.height(24.dp).clip(RoundedCornerShape(3.dp)).background(if(item==metric)Accent.copy(alpha=.07f)else Color.Transparent).selectable(selected=item==metric,role=Role.Tab,onClick={onChange(item)}).semantics{contentDescription="Graph ${item.label}"}.padding(horizontal=6.dp),contentAlignment=Alignment.Center){NativeText(item.label,12,if(item==metric)Accent else Muted)}
 }
}
@Composable internal fun NativePace(records:List<AttemptRecord>,modifier:Modifier,showMeanLabel:Boolean=true,metric:GraphMetric=GraphMetric.Time,onMetric:(GraphMetric)->Unit={}){
 val data=remember(records,metric){graphSeries(records,metric)};val accent=Accent;val faint=Faint;val green=Green
 Column(modifier){
  Row(Modifier.fillMaxWidth().height(24.dp),verticalAlignment=Alignment.CenterVertically){GraphTabs(metric,onMetric);Spacer(Modifier.weight(1f));NativeText(data.mean?.let{(if(showMeanLabel||metric!=GraphMetric.Time)"Mean "else "")+metric.format(it)}?:"—",12,Faint,true,modifier=Modifier.semantics{contentDescription="Mean ${metric.label}, ${data.mean?.let(metric::format)?:"unavailable"}, ${data.slots.count{it!=null}} of ${data.slots.size} solves"})}
  if(data.mean==null){NativeText("No ${metric.label} recorded",12,Faint);return@Column}
  Canvas(Modifier.weight(1f).fillMaxWidth()){
   val points=data.slots.takeLast(40);val valid=points.filterNotNull()+data.mean;val low=valid.min();val high=valid.max();val range=(high-low).coerceAtLeast(metric.minimum);val baseline=low-(range-(high-low))/2
   val y:(Double)->Float={v->(size.height-6-((v-baseline)/range)*(size.height-10)).toFloat()}
   val average=y(data.mean);drawLine(faint.copy(alpha=.5f),Offset(0f,average),Offset(size.width,average),1f,pathEffect=PathEffect.dashPathEffect(floatArrayOf(3f,3f)))
   var previous:Offset?=null
   for((i,v)in points.withIndex()){
    if(v==null){previous=null;continue};val point=Offset(4+i*(size.width-8)/(points.size-1).coerceAtLeast(1),y(v))
    previous?.let{drawLine(accent,it,point,1.3f)};previous=point;drawCircle(if(v==data.best)green else accent,if(v==data.best)3.5f else 2f,point)
   }
  }
 }
}
@Composable internal fun NativeHistogram(records:List<AttemptRecord>,modifier:Modifier,metric:GraphMetric=GraphMetric.Time,onMetric:(GraphMetric)->Unit={}){
 BoxWithConstraints(modifier){
  val width=maxWidth.value;val series=remember(records,metric){graphSeries(records,metric)};val d=remember(series,width){series.histogram(metric,width)};val bins=d.bins
  val colors=if(LocalColors.current.light)listOf(Color(0xff567a9e),Color(0xff7092b3))else listOf(Color(0xff7895b2),Color(0xff8ca8c2))
  Column(Modifier.fillMaxSize()){
   Row(Modifier.fillMaxWidth().height(24.dp),verticalAlignment=Alignment.CenterVertically){GraphTabs(metric,onMetric);Spacer(Modifier.weight(1f));NativeText("${d.count}/${records.size}",12,Faint,true,modifier=Modifier.semantics{contentDescription="Distribution ${metric.label}, ${d.count} of ${records.size} solves"})}
   Spacer(Modifier.height(18.dp))
   if(d.count==0){NativeText(if(records.isEmpty())"No solves yet."else "No recorded values.",12,Faint);return@Column}
   BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()){
    val available=(maxHeight.value-26f).coerceAtLeast(0f);val peak=bins.max().coerceAtLeast(1)
    Row(Modifier.fillMaxSize(),verticalAlignment=Alignment.Bottom){for((i,n)in bins.withIndex()){
     val height=available*n/peak
     Column(Modifier.weight(1f),horizontalAlignment=Alignment.CenterHorizontally){Box(Modifier.fillMaxWidth().height(height.coerceAtLeast(0f).dp).background(colors[i%2]).semantics{contentDescription="${metric.axis(d.start+d.step*i)} to ${metric.axis(d.start+d.step*(i+1))} ${metric.unit}: $n solves"}){if(n>0)NativeText(n.toString(),12,Faint,true,modifier=Modifier.align(Alignment.TopCenter).wrapContentHeight(Alignment.Top,unbounded=true).offset(y=(-21).dp))};Box(Modifier.height(26.dp),contentAlignment=Alignment.Center){if(i%ceil(44f/(width/bins.size)).toInt().coerceAtLeast(1)==0)NativeText(metric.axis(d.start+d.step*i),12,Faint,true)}}
    }}
   }
  }
 }
}
@Composable internal fun NativeCrossRoutes(routes:JsonArray?,busy:Boolean,error:String?,record:AttemptRecord?=null){
 val clipboard=LocalClipboardManager.current
 if(busy)CircularProgressIndicator(Modifier.size(16.dp),strokeWidth=1.5.dp,color=Accent)
 error?.let{Text(it,fontSize=12.sp,color=Red)}
 routes?.forEach{value->val route=value.jsonObject
  val face=route["face"]!!.jsonPrimitive.content;val cross=route["kind"]!!.jsonPrimitive.content=="cross"
  val color=when(face){"U"->"White";"D"->"Yellow";"R"->"Red";"L"->"Orange";"F"->"Green";else->"Blue"}
  val count=route["length"]!!.jsonPrimitive.int
  Spacer(Modifier.height(8.dp));Row(Modifier.fillMaxWidth(),verticalAlignment=Alignment.CenterVertically){NativeText("$color · ${if(cross)"Cross"else "Xcross"}",12,Muted,weight=FontWeight.Medium);Spacer(Modifier.weight(1f));NativeText("$count moves",12,Faint,true)}
  val alg=(route["rotation"]!!.jsonPrimitive.content+" "+route["moves"]!!.jsonArray.joinToString(" "){it.jsonPrimitive.content}).trim()
  Text(alg.ifBlank{"✓"},fontFamily=LocalMono.current,fontSize=14.sp,lineHeight=22.sp,color=Foreground)
  if(record!=null&&cross)record.analysis?.splits?.firstOrNull{it.stage=="Cross"}?.let{NativeText("Played ${it.moves} → $count moves",12,if(count<it.moves)Green else Faint)}
  NativeText("Copy",12,Accent,modifier=Modifier.padding(vertical=4.dp).clickable{clipboard.setText(AnnotatedString(alg))})
 }
}
@Composable internal fun NativeTools(c:LabController,selectedRecord:AttemptRecord?,tool:Int,onTool:(Int)->Unit,modifier:Modifier){
 val record=remember(c.records,c.settings.session){c.records.lastOrNull{it.kind=="solve"&&it.session==c.settings.session}}
 val current=c.target?.position?.state
 var currentRoutes by remember(current){mutableStateOf<JsonArray?>(null)};var currentBusy by remember(current){mutableStateOf(current!=null)};var currentError by remember(current){mutableStateOf<String?>(null)}
 LaunchedEffect(current){if(current!=null){try{currentRoutes=WireJson.parseToJsonElement(localSolver("native-cross-dashboard",buildJsonObject{put("state",current);put("faces","UD")}.toString())).jsonObject["routes"]?.jsonArray}catch(e:CancellationException){throw e}catch(e:Exception){currentError=e.message}finally{currentBusy=false}}}
 var routes by remember(record?.id){mutableStateOf<JsonArray?>(null)};var busy by remember(record?.id){mutableStateOf(record!=null)};var error by remember(record?.id){mutableStateOf<String?>(null)}
 LaunchedEffect(record?.id){if(record!=null&&record.moves.isNotEmpty()&&Cube.valid(record.initialState)){try{routes=WireJson.parseToJsonElement(localSolver("native-cross-dashboard",buildJsonObject{put("state",record.initialState);put("faces",reconstructionCross(record).toString())}.toString())).jsonObject["routes"]?.jsonArray}catch(e:CancellationException){throw e}catch(e:Exception){error=e.message}finally{busy=false}}else busy=false}
 val edge=Line;val accent=Accent
 var algStage by remember{mutableStateOf("PLL")};var alternatives by remember(record?.id,algStage){mutableStateOf<JsonObject?>(null)}
 var algBusy by remember{mutableStateOf(false)}
 LaunchedEffect(record?.id,tool,algStage){if(record!=null&&tool==1){algBusy=true;try{alternatives=WireJson.parseToJsonElement(localSolver("native-algorithms",buildJsonObject{put("record",WireJson.encodeToJsonElement(AttemptRecord.serializer(),record));put("stage",algStage)}.toString())).jsonObject}catch(e:CancellationException){throw e}catch(e:Exception){alternatives=null}finally{algBusy=false}}}
 Column(modifier.verticalScroll(rememberScrollState())){
  NativeText("Current scramble",14,weight=FontWeight.SemiBold)
  NativeCrossRoutes(currentRoutes,currentBusy,currentError)
  Spacer(Modifier.height(16.dp));HorizontalDivider(color=Line);Spacer(Modifier.height(16.dp))
  NativeText(record?.let{"Last solve · #${c.records.filter{it.kind=="solve"&&it.session==c.settings.session}.indexOfFirst{r->r.id==it.id}+1}"}?:"Last solve",14,weight=FontWeight.SemiBold)
  if(record==null){NativeText("No completed solve yet.",12,Faint);return@Column}
  Spacer(Modifier.height(12.dp));Row(Modifier.fillMaxWidth().height(28.dp).drawBehind{drawLine(edge,Offset(0f,size.height),Offset(size.width,size.height),1.dp.toPx())},horizontalArrangement=Arrangement.spacedBy(12.dp)){for((i,label)in listOf("Cross","Algs","Technique").withIndex()){
    val active=tool==i
    Box(Modifier.fillMaxHeight().drawBehind{if(active)drawLine(accent,Offset(0f,size.height),Offset(size.width,size.height),1.dp.toPx())}.clickable{onTool(i)}){NativeText(label,12,if(active)Accent else Muted,weight=FontWeight.Medium)}
  }}
  Spacer(Modifier.height(12.dp));when(tool){
   0->{if(record.moves.isEmpty())NativeText("Recorded turns are needed to compare this solve.",12,Faint)else NativeCrossRoutes(routes,busy,error,record)}
   1->{
    Row(horizontalArrangement=Arrangement.spacedBy(8.dp)){for(stage in listOf("OLL","PLL"))NativeTab(stage,algStage==stage,Modifier.height(30.dp)){algStage=stage}}
    Spacer(Modifier.height(16.dp))
    if(algBusy)CircularProgressIndicator(Modifier.size(18.dp),strokeWidth=1.5.dp,color=Accent)
    else if(alternatives==null)Text("No verified variant available for this reconstruction.",fontSize=12.sp,color=Faint,lineHeight=18.sp)
    else{
     NativeText("Played",12,Faint,weight=FontWeight.Medium);Text(alternatives!!["own"]!!.jsonPrimitive.content,fontFamily=LocalMono.current,fontSize=14.sp,lineHeight=25.9.sp,color=Muted)
     Spacer(Modifier.height(12.dp));HorizontalDivider(color=Line);Spacer(Modifier.height(16.dp));NativeText("Variants to test",14,weight=FontWeight.SemiBold)
     Text("Move counts are not predictions of personal speed.",fontSize=12.sp,color=Faint,lineHeight=19.2.sp)
     val clipboard=LocalClipboardManager.current
     for(value in alternatives!!["variants"]!!.jsonArray){val v=value.jsonObject;Spacer(Modifier.height(12.dp));NativeText("${v["name"]!!.jsonPrimitive.content} · ${v["moves"]!!.jsonPrimitive.int} moves",12,Muted,weight=FontWeight.Medium);val alg=v["algorithm"]!!.jsonPrimitive.content;Text(alg,fontFamily=LocalMono.current,fontSize=14.sp,lineHeight=25.9.sp,color=Foreground);Row(horizontalArrangement=Arrangement.spacedBy(16.dp)){NativeText("Copy",12,Accent,modifier=Modifier.clickable{clipboard.setText(AnnotatedString(alg))});NativeText("Practice case",12,Accent,modifier=Modifier.clickable{c.prepareCase(v["caseId"]!!.jsonPrimitive.content)})};Spacer(Modifier.height(12.dp));HorizontalDivider(color=Line)}
    }
    Spacer(Modifier.height(12.dp));NativeText("Tao Yu / Alg-Trainer · MIT",12,Faint)
   }
   else->{NativeText("Technique",weight=FontWeight.Medium);Spacer(Modifier.height(10.dp));record.analysis?.pauses?.take(5)?.forEach{NativeText("${it.stage}  ${metric(it.duration/1000)} s",12,Muted,true)};Spacer(Modifier.height(12.dp));Text("Turn gaps are measured; finger positions are not inferred.",fontSize=12.sp,color=Faint,lineHeight=18.sp);SmallButton("Timed reconstruction"){c.review=record}}
  }
 }
}
@Composable internal fun NativeTags(c:LabController,record:AttemptRecord,onDismiss:()->Unit){AlertDialog(onDismissRequest=onDismiss,shape=RoundedCornerShape(4.dp),containerColor=Raised,title={NativeText("Tags",18,weight=FontWeight.SemiBold)},text={Column{Row(verticalAlignment=Alignment.CenterVertically){Checkbox(record.localReview.favorite,{c.save(record.copy(localReview=record.localReview.copy(favorite=it)))});NativeText("Favorite")};Row(horizontalArrangement=Arrangement.spacedBy(12.dp)){for((id,color)in listOf("" to Faint,"blue" to Accent,"mint" to Green,"amber" to Amber,"rose" to Red))Box(Modifier.size(32.dp).semantics{contentDescription="Tag color ${id.ifBlank{"none"}}"}.clickable{c.save(record.copy(localReview=record.localReview.copy(color=id)))}.padding(7.dp).background(color,CircleShape))};Spacer(Modifier.height(16.dp));for(group in listOf("","review","practice","milestone"))NativeTab(group.ifBlank{"No group"}.replaceFirstChar{it.uppercase()},record.localReview.group==group,Modifier.fillMaxWidth().height(36.dp)){c.save(record.copy(localReview=record.localReview.copy(group=group)))}}},confirmButton={TextButton(onDismiss){Text("Done")}})}

internal val Split.duration:Double get()=time?:end?.minus(start)?:0.0
internal val Split.tps:Double? get()=duration.takeIf{it>0}?.let{moves*1000/it}
internal fun phasePause(record:AttemptRecord,split:Split,duration:Double=split.duration):Double {
 val before=record.analysis?.splits.orEmpty().takeWhile{it.stage!=split.stage}.sumOf{it.rawMoves?:it.moves}
 return record.moves.getOrNull(before)?.t?.minus(split.start)?.coerceIn(0.0,duration)?:0.0
}

internal fun webTime(ms:Double?):String=lab.cube.core.formatTime(ms?.let{if(it.isFinite())floor(it/10)*10 else it})

internal fun webScoredTime(record:AttemptRecord):String=(if(adjusted(record).isFinite()&&executionDurationEstimated(record))"≈ "else "")+webTime(adjusted(record))
internal fun nativeTotal(ms:Double):String{val s=(ms/1000).toLong();return if(s>=3600)"${s/3600}h ${(s/60)%60}m"else if(s>=60)"${s/60}m ${s%60}s"else "${s}s"}

@Composable internal fun NativeRailAverages(records:List<AttemptRecord>){Column(Modifier.fillMaxWidth().measureNative(".average-table")){for(n in listOf(5,12)){val a=remember(records,n){retainedAverage(records,n)};Row(Modifier.fillMaxWidth().height(26.dp),verticalAlignment=Alignment.CenterVertically){NativeText("ao$n",12,Faint,modifier=Modifier.width(30.dp));NativeText(a.time?.let{webTime(floor(it/10+.5)*10)}?:"—",14,mono=true,weight=FontWeight.SemiBold,modifier=Modifier.weight(1f),align=TextAlign.End);NativeText(a.tps?.let{metric(it)}?:"—",12,Muted,true,modifier=Modifier.width(40.dp),align=TextAlign.End);NativeText(a.moves?.let{metric(it,1)}?:"—",12,Muted,true,modifier=Modifier.width(40.dp),align=TextAlign.End)}}}}
