package lab.cube.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.*
import kotlin.math.floor

/** One consistent hierarchy in the timer dock and solve review. */
@Composable internal fun PhaseTime(total:Double?,pause:Double,cross:Boolean,estimated:Boolean=false,size:Int=16){
 val displayedTotal=total?.let{floor(it.coerceAtLeast(0.0)/10+0.5)*10}
 val displayedPause=if(pause.isFinite())floor(pause.coerceIn(0.0,displayedTotal?:0.0)/10+0.5)*10 else 0.0
 val gap=if(LocalColors.current.light)Color(0xff487bc6)else Color(0xff86b9ef)
 Column(horizontalAlignment=Alignment.End){
  Text((if(estimated&&total!=null)"≈"else "")+(displayedTotal?.let(::webTime)?:"—"),fontFamily=LocalMono.current,fontSize=size.sp,lineHeight=(size+2).sp,fontWeight=FontWeight.SemiBold,color=if(total==null)Faint else Foreground,maxLines=1,modifier=Modifier.height((size+2).dp).semantics{contentDescription=total?.let{"Total ${webTime(displayedTotal?:it)} seconds"}?:"No stage time"})
  if(total!=null&&!cross){
   val execution=(displayedTotal!!-displayedPause).coerceAtLeast(0.0)
   Text(buildAnnotatedString{withStyle(SpanStyle(color=gap)){append(webTime(displayedPause))};withStyle(SpanStyle(color=Faint)){append("+")};withStyle(SpanStyle(color=Muted)){append(webTime(execution))}},fontFamily=LocalMono.current,fontSize=12.sp,lineHeight=14.sp,maxLines=1,modifier=Modifier.height(14.dp).semantics{contentDescription="Gap ${webTime(displayedPause)} seconds, execution ${webTime(execution)} seconds"})
  }else Spacer(Modifier.height(14.dp))
 }
}
