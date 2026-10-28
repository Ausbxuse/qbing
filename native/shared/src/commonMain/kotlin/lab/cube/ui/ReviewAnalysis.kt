package lab.cube.ui

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.*
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.*
import kotlinx.serialization.json.*
import lab.cube.core.*
import kotlin.math.roundToInt

private fun pairColor(light:Boolean,i:Int)=Color((if(light)listOf(0xff4c87f2,0xff27a4b5,0xff7e82e8,0xffa171d4)else listOf(0xff79a6ff,0xff61c9d0,0xffa4a8ff,0xffc29aec))[i%4])

@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun ReviewAnalysis(c:LabController,r:AttemptRecord,data:JsonObject?,busy:Boolean,notation:List<NotationItem>,modifier:Modifier,listState:LazyListState,columns:Int,onReplay:(Int)->Unit){
 val clipboard=LocalClipboardManager.current;val light=LocalColors.current.light
 val groups=remember(data){data?.get("groups")?.jsonArray.orEmpty().map{it.jsonObject}.filter{(it["rawEnd"]?.jsonPrimitive?.int?:0)>(it["rawStart"]?.jsonPrimitive?.int?:0)}}
 val pairs=remember(r,groups){groups.filter{it["stage"]?.jsonPrimitive?.content=="F2L"}.mapNotNull{g->segmentMetrics(r,g["rawStart"]!!.jsonPrimitive.int,g["rawEnd"]!!.jsonPrimitive.int)?.let{g to it}}}
 val largest=r.analysis?.splits?.filter{it.time?.let{t->t.isFinite()&&t>0}==true}?.maxByOrNull{it.duration}
 val gap=r.analysis?.pauses?.filter{it.duration>0}?.maxByOrNull{it.duration}
 LazyColumn(modifier,state=listState,verticalArrangement=Arrangement.spacedBy(8.dp)){
  item{Surface(color=Panel,shape=RoundedCornerShape(5.dp),border=BorderStroke(1.dp,Line)){
   Column(Modifier.fillMaxWidth().padding(10.dp)){
    Row(verticalAlignment=Alignment.CenterVertically){NativeText("Scramble",12,weight=FontWeight.SemiBold);QuietButton("Copy scramble"){clipboard.setText(AnnotatedString(r.scramble))};NativeText(tps(r.scrambleStats?.tps),12,Muted,true)}
    SelectionContainer{Text(r.scramble.ifBlank{"No scramble saved."},fontFamily=LocalMono.current,fontSize=14.sp,lineHeight=23.sp,color=Foreground)}
   }
  }}
  if(largest!=null||gap!=null)item{
   FlowRow(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(12.dp),verticalArrangement=Arrangement.spacedBy(4.dp)){
    largest?.let{s->NativeText("Largest phase · ${s.stage} · ${(s.duration/r.duration.coerceAtLeast(1.0)*100).roundToInt()}%",12,Muted)}
    gap?.let{g->Row(verticalAlignment=Alignment.CenterVertically){NativeText("Longest gap · "+(if(g.estimated)"≈"else "")+webTime(g.duration)+" s",12,Accent);TextButton(onClick={onReplay(g.index)},contentPadding=PaddingValues(horizontal=6.dp,vertical=0.dp),modifier=Modifier.height(28.dp)){NativeText("Replay gap",12,Muted)}}}
   }
  }
  item{Row(Modifier.fillMaxWidth().height(28.dp),verticalAlignment=Alignment.CenterVertically){NativeText("F2L pairs",14,weight=FontWeight.Medium);Spacer(Modifier.width(12.dp));if(busy)CircularProgressIndicator(Modifier.size(14.dp),color=Accent,strokeWidth=1.5.dp)}}
  if(pairs.isEmpty()&&!busy)item{NativeText("No verified pair breakdown for this solve.",12,Muted)}
  if(pairs.sumOf{it.second.duration}>0)item{
   Row(Modifier.fillMaxWidth().height(20.dp),horizontalArrangement=Arrangement.spacedBy(4.dp),verticalAlignment=Alignment.CenterVertically){
    for((i,pair)in pairs.withIndex())Box(Modifier.weight(pair.second.duration.toFloat().coerceAtLeast(.001f)).fillMaxHeight().semantics{contentDescription="${pair.first["label"]!!.jsonPrimitive.content}, ${webTime(pair.second.duration)} seconds"}.clickable{onReplay(pair.second.start)}.padding(vertical=3.dp).clip(RoundedCornerShape(2.dp)).background(pairColor(light,i)))
   }
  }
  for((row,chunk)in pairs.chunked(columns).withIndex())item(key="pair-row:$row"){
   Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(8.dp)){
   for((column,pair)in chunk.withIndex()){
   val(g,m)=pair;val color=pairColor(light,row*columns+column)
   Surface(modifier=Modifier.weight(1f),color=color.copy(alpha=.055f),border=BorderStroke(1.dp,color.copy(alpha=.24f)),shape=RoundedCornerShape(5.dp)){
    Column(Modifier.fillMaxWidth().padding(10.dp)){
     Row(Modifier.fillMaxWidth(),verticalAlignment=Alignment.CenterVertically){NativeText(g["label"]!!.jsonPrimitive.content,14,color,weight=FontWeight.SemiBold);Spacer(Modifier.width(8.dp));NativeText(g["caseName"]?.jsonPrimitive?.content?:"Unclassified",12,Muted,modifier=Modifier.weight(1f));QuietButton("Replay"){onReplay(m.start)}}
     FlowRow(horizontalArrangement=Arrangement.spacedBy(16.dp),verticalArrangement=Arrangement.spacedBy(4.dp)){
      PhaseTime(m.duration,m.gap,false,m.estimated)

      NativeText("${m.moves} moves",12,Muted,true);NativeText(m.tps?.let{(if(m.estimated)"≈"else "")+metric(it)+" tps"}?:"— tps",12,Muted,true)
     }
     NativeText(notation.filter{it.rawStart>=m.start&&it.rawEnd<=m.end&&!it.rotation}.joinToString(" "){it.notation},14,Muted,true)
     ReconstructionCases(c,r,"F2L",data,false,rawStart=m.start,heading="Alternatives")
    }
   }
     }
   if(chunk.size<columns)Spacer(Modifier.weight(1f))
   }
  }
  for(stage in listOf("Cross","OLL","PLL"))item(key="layer:$stage"){ReconstructionCases(c,r,stage,data,busy,heading=if(stage=="Cross")"Cross solutions"else null)}
  item{
   var expanded by remember(r.id){mutableStateOf(false)}
   Column{QuietButton(if(expanded)"▾ Timing details"else "▸ Timing details"){expanded=!expanded};if(expanded){
    NativeText("Colored prefix is the entry gap. ≈ marks approximate event timing.",12,Faint)
    if(r.rotations.isNotEmpty()&&initialLivePose(r)==null)NativeText("Starting calibration was not recorded; gyro poses are relative.",12,Faint)
    if(r.quality.isNotEmpty())NativeText(r.quality.joinToString(" · "),12,Amber)
    for(s in r.analysis?.signals.orEmpty().take(5))NativeText("Possible correction · ${s.text}",12,Muted)
    if(r.notes.isNotBlank())NativeText(r.notes,14,Muted)
   }}
  }
 }
}
