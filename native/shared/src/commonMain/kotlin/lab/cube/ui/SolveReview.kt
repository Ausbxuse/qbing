package lab.cube.ui

import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.rememberLazyListState
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

@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun RecordDialog(c:LabController,r:AttemptRecord){
    var index by remember(r.id){mutableStateOf(0)};var playing by remember(r.id){mutableStateOf(false)};var notes by remember(r.id){mutableStateOf(r.notes)};var deleting by remember{mutableStateOf(false)}
    val execution=remember(r){recordedExecutionMetric(r)}
    val derivedNotation=remember(r){reconstructionWithRotations(r,viewFrame=true)}
    val rawNotation=remember(r){reconstructionWithRotations(r,raw=true,viewFrame=true)}
    var rawTurns by remember(r.id){mutableStateOf(false)}
    val notation=if(rawTurns)rawNotation else derivedNotation
    val clipboard=LocalClipboardManager.current
    val states=remember(r.initialState,r.moves){buildList<String?>{var state:String?=r.initialState.takeIf{Cube.valid(it)};add(state);for(move in r.moves){state=state?.let{runCatching{Cube.turn(it,move.move)}.getOrNull()};add(state)}}}
    val ranges=remember(r){reconstructionRanges(r)}
    val playbackFocus=remember(r.id){FocusRequester()};var spaceHeld by remember{mutableStateOf(false)}
    var playhead by remember(r.id){mutableStateOf(0.0)}
    var replayRevision by remember(r.id){mutableStateOf(0)}
    var stepCursor by remember(r.id){mutableStateOf<Int?>(null)}
    val stepPositions=remember(r,rawTurns){replayPositions(r,rawTurns)}
    val playbackTimes=remember(r){r.moves.mapIndexed{i,m->if(execution!=null&&m.t.isFinite())m.t else i*180.0}}
    fun seek(position:Int){stepCursor=null;playing=false;index=position.coerceIn(0,r.moves.size);playhead=if(index==0)0.0 else playbackTimes[index-1];replayRevision++}
    fun step(forward:Boolean){
        val cursor=adjacentReplayPosition(stepPositions,ReplayPosition(index,playhead),stepCursor,forward)
        val target=stepPositions[cursor];seek(target.rawIndex);playhead=target.time;stepCursor=cursor
    }
    var caseReview by remember(r){mutableStateOf<JsonObject?>(null)};var caseBusy by remember(r){mutableStateOf(r.moves.isNotEmpty())}
    LaunchedEffect(r){if(r.moves.isNotEmpty()){try{caseReview=WireJson.parseToJsonElement(localSolver("native-review",buildJsonObject{put("record",WireJson.encodeToJsonElement(AttemptRecord.serializer(),r))}.toString())).jsonObject}catch(e: kotlinx.coroutines.CancellationException){throw e}catch(_:Exception){caseReview=null}finally{caseBusy=false}}}
    LaunchedEffect(playing,r.id,replayRevision){if(playing){
        if(index>=r.moves.size){index=0;playhead=0.0}
        val start=monotonicMs()-playhead
        while(index<r.moves.size&&playing){playhead=monotonicMs()-start;while(index<r.moves.size&&playbackTimes[index]<=playhead)index++;if(index>=r.moves.size){playhead=playbackTimes.lastOrNull()?:0.0;playing=false}else delay(16)}
    }}
    var tab by remember(r.id){mutableStateOf(0)}
    val moveScroll=rememberLazyListState();val analysisScroll=rememberLazyListState()
    val stages=listOf("Cross","F2L","OLL","PLL")
    val phaseColors=if(LocalColors.current.light)listOf(Color(0xff16bda2),Color(0xff5a8fff),Color(0xffedc94b),Color(0xffa282ee))else listOf(Color(0xff3bd6b6),Color(0xff79a6ff),Color(0xfff2d255),Color(0xffba9bff))
    Dialog(onDismissRequest={c.review=null},properties=DialogProperties(usePlatformDefaultWidth=false)){
      BoxWithConstraints(Modifier.padding(12.dp).widthIn(max=812.dp).fillMaxWidth()){
        val wide=maxWidth>=620.dp
        val detailHeight=(maxHeight-482.dp).coerceIn(90.dp,360.dp)
        Surface(shape=RoundedCornerShape(8.dp),color=Ink,border=BorderStroke(1.dp,Line),modifier=Modifier.onPreviewKeyEvent{event->
            if(event.key==Key.DirectionLeft||event.key==Key.DirectionRight){if(event.type==KeyEventType.KeyDown)step(event.key==Key.DirectionRight);true}
            else if(event.key==Key.Spacebar){if(event.type==KeyEventType.KeyDown){if(!spaceHeld){if(r.moves.isNotEmpty()){stepCursor=null;playing=!playing};spaceHeld=true}}else if(event.type==KeyEventType.KeyUp)spaceHeld=false;true}else false
        }.focusRequester(playbackFocus).focusable().heightIn(max=maxHeight)){
          Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp)){
            Row(Modifier.fillMaxWidth().height(44.dp),verticalAlignment=Alignment.CenterVertically){NativeText(if(r.kind=="drill")c.catalog.cases.firstOrNull{it.id==r.caseId}?.name?:"Drill"else "Solve review",14,weight=FontWeight.SemiBold,modifier=Modifier.weight(1f));GlyphButton("Delete","Delete solve"){deleting=true};GlyphButton("Close","Close review"){c.review=null}}
            Row(Modifier.fillMaxWidth().padding(top=8.dp,bottom=4.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(20.dp)){
              NativeText(scoredTime(r),32,Foreground,true,FontWeight.SemiBold)
              Column{NativeText((if(execution?.estimated==true)"≈ "else "")+metric(execution?.tps),17,Foreground,true);NativeText("tps",12,Faint)}
              Column{NativeText(recordedMoveCount(r)?.toString()?:"—",17,Foreground,true);NativeText("moves",12,Faint)}
            }
            NativeText(listOf(r.session,r.createdAt.take(16).replace('T',' ')+" UTC",crossColorName(r.analysis?.cross)?.let{"$it cross"}).filterNotNull().filter{it.isNotBlank()}.joinToString(" · "),12,Faint)
            Spacer(Modifier.height(10.dp));DividerLine()
            if(r.moves.isNotEmpty()){
              Row(Modifier.fillMaxWidth().padding(top=8.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(12.dp)){
                states.getOrNull(index)?.let{CubeCanvas(it,Modifier.width(if(wide)210.dp else 110.dp).height(116.dp),replayPose=recordedReplayPose(r,playhead).let{q->listOf(q.x,q.y,q.z,q.w)})}
                Column(Modifier.weight(1f)){
                  for((i,stage)in stages.withIndex()){
                    val split=r.analysis?.splits?.firstOrNull{it.stage==stage}
                    Row(Modifier.fillMaxWidth().height(35.dp).clickable(enabled=split!=null){seek(r.analysis!!.splits.takeWhile{it.stage!=stage}.sumOf{it.rawMoves?:it.moves})},verticalAlignment=Alignment.Top){
                      Box(Modifier.width(2.dp).height(14.dp).background(phaseColors[i],RoundedCornerShape(1.dp)))
                      Spacer(Modifier.width(8.dp));NativeText(stage,12,Muted,modifier=Modifier.width(40.dp))
                      Spacer(Modifier.weight(1f))
                      if(split?.time!=null&&execution!=null){
                        val pause=if(i==0)0.0 else phasePause(r,split)
                        PhaseTime(split.duration,pause,i==0,splitTimingEstimated(r,split),14)
                      }else PhaseTime(null,0.0,i==0,size=14)

                      if(wide){Spacer(Modifier.width(16.dp));NativeText(split?.tps?.takeIf{execution!=null}?.let{(if(splitTimingEstimated(r,split))"≈"else "")+metric(it)+" tps"}?:"— tps",12,Muted,true,modifier=Modifier.width(80.dp),align=androidx.compose.ui.text.style.TextAlign.End);NativeText(split?.moves?.let{"$it moves"}?:"— moves",12,Muted,true,modifier=Modifier.width(80.dp),align=androidx.compose.ui.text.style.TextAlign.End)}
                    }
                  }
                }
              }
              Row(Modifier.fillMaxWidth().height(44.dp),verticalAlignment=Alignment.CenterVertically){
                IconButton({step(false)},modifier=Modifier.size(32.dp).semantics{contentDescription="Previous turn"}){Glyph("StepBack",Foreground,18)}
                IconButton({stepCursor=null;playing=!playing},modifier=Modifier.size(40.dp).semantics{contentDescription=if(playing)"Pause"else "Play"}){Glyph(if(playing)"Pause"else "Play",Foreground,18)}
                IconButton({step(true)},modifier=Modifier.size(32.dp).semantics{contentDescription="Next turn"}){Glyph("StepForward",Foreground,18)}
                Slider(index.toFloat(),{seek(it.roundToInt())},valueRange=0f..r.moves.size.toFloat(),modifier=Modifier.weight(1f).padding(horizontal=8.dp).semantics{contentDescription="Replay position $index/${r.moves.size}"})
                NativeText("${webTime(playhead)} / ${webTime(r.duration)}",12,Faint,true)
              }
            }
            DividerLine()
            Row(Modifier.fillMaxWidth().height(36.dp)){for((i,name)in listOf("Moves","Analysis").withIndex())NativeTab(name,tab==i,Modifier.height(36.dp)){tab=i}}
            if(tab==1){ReviewAnalysis(c,r,caseReview,caseBusy,derivedNotation,Modifier.fillMaxWidth().height(detailHeight),analysisScroll,if(wide)2 else 1){position->seek(position);playing=true}}
            else LazyColumn(Modifier.fillMaxWidth().height(detailHeight),state=moveScroll){
              when(tab){
                0->{
                  if(r.moves.isEmpty())item{NativeText("No recorded turns for this solve.",12,Muted)}
                  else{
                    item{Row(Modifier.fillMaxWidth().height(40.dp),verticalAlignment=Alignment.CenterVertically){Segment("Notation",!rawTurns){rawTurns=false;stepCursor=null};Segment("Raw",rawTurns){rawTurns=true;stepCursor=null};Spacer(Modifier.weight(1f));if(caseBusy){CircularProgressIndicator(Modifier.size(14.dp),strokeWidth=1.5.dp,color=Accent);Spacer(Modifier.width(8.dp))};QuietButton("Copy raw"){clipboard.setText(AnnotatedString(r.moves.joinToString(" "){it.move}))}}}
                    for(range in ranges){
                      item(key="stage:${range.stage}"){
                        Row(Modifier.fillMaxWidth().height(30.dp).clickable{seek(range.rawStart)},verticalAlignment=Alignment.CenterVertically){Box(Modifier.width(2.dp).height(14.dp).background((phaseColors.getOrNull(stages.indexOf(range.stage))?:Faint)));Spacer(Modifier.width(8.dp));NativeText(range.stage,12,weight=FontWeight.SemiBold);Spacer(Modifier.width(12.dp));NativeText("${countMoves(r.moves.subList(range.rawStart,range.rawEnd))} moves",12,Faint)}
                      }
                      item(key="turns:${range.stage}"){
                        FlowRow(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(4.dp),verticalArrangement=Arrangement.spacedBy(4.dp)){
                          for(turn in notation.filter{val at=it.rawStart.coerceAtMost((r.moves.size-1).coerceAtLeast(0));at>=range.rawStart&&at<range.rawEnd}){
                            val selected=if(turn.rotation)turn.elapsed?.let{abs(it-playhead)<.5}==true&&!playing else index>turn.rawStart&&index<=turn.rawEnd
                            Column(Modifier.width(56.dp).height(40.dp).clip(RoundedCornerShape(4.dp)).background(if(selected)Accent.copy(alpha=.12f)else Color.Transparent).clickable{seek(turn.rawEnd);if(turn.rotation)playhead=turn.elapsed?:playhead}.semantics{contentDescription="${turn.notation} at ${turn.elapsed?.let{metric(it/1000,3)}?:"unknown"} seconds"},horizontalAlignment=Alignment.CenterHorizontally,verticalArrangement=Arrangement.Center){
                              NativeText((if(turn.rotation)"◈ "else "")+turn.notation+(if(turn.inferred)"*"else ""),14,if(selected||turn.rotation)Accent else Foreground,true)
                              NativeText(turn.elapsed?.let{(if(turn.rotation||turn.recovered||hasEstimatedTurnTiming(r))"≈"else "")+metric(it/1000,3)}?:"—",12,Faint,true)
                            }
                          }
                        }
                      }
                      item(key="cases:${range.stage}"){ReconstructionCases(c,r,range.stage,caseReview,caseBusy)}
                    }
                  }
                }
              }
            }
            DividerLine()
            Row(Modifier.fillMaxWidth().height(40.dp),verticalAlignment=Alignment.CenterVertically){for(penalty in listOf("","+2","DNF"))Segment(penalty.ifBlank{"OK"},r.penalty==penalty){c.updateRecord(r.copy(penalty=penalty,notes=notes))};Spacer(Modifier.weight(1f));NativeText(if(r.source=="manual")"Manual"else if(execution==null)"Unverified timing"else if(execution.estimated)"≈ Estimated timing"else "Bluetooth",12,Faint)}
          }
        }
      }
    }
    if(deleting)AlertDialog(onDismissRequest={deleting=false},containerColor=Panel,title={Text("Delete this solve?")},text={Text("${scoredTime(r)} s · This cannot be undone.",fontSize=14.sp)},confirmButton={TextButton(onClick={c.deleteRecord(r);deleting=false}){Text("Delete",color=Red)}},dismissButton={TextButton(onClick={deleting=false}){Text("Cancel")}})
    LaunchedEffect(r.id){withFrameNanos{};playbackFocus.requestFocus()}
}
