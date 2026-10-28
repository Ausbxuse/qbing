package lab.cube.core
import kotlin.test.*
class TimerGeometryTest{
 @Test fun tiledLayoutsKeepScrambleCubeClockAndDockApart(){
  for(w in listOf(360f,390f,450f,640f,760f,800f,960f,1080f,1180f,1280f,1440f,1920f))for(h in listOf(400f,480f,540f,600f,720f,800f,1000f,1200f)){
   val mobile=w<=760;val body=h-(if(mobile)136f else 56f);val rails=!mobile&&w>=1180&&body>=580
   val center=w-if(rails)(w*.16f).coerceIn(208f,232f)*2 else 0f
   val g=timerGeometry(center,body,mobile);val phaseTop=body-(if(g.dense)8 else if(mobile)4 else 24)-g.phaseHeight
   assertTrue(g.cubeTop>=g.scrambleTop+g.scrambleHeight,"cube/scramble $w × $h")
   assertTrue(g.cy-g.clockHeight/2>=g.cubeTop+g.cubeHeight,"cube/clock $w × $h")
   assertTrue(g.cy+g.clockHeight/2+(if(g.actions)66 else 26)<=phaseTop,"clock/dock $w × $h")
  }
 }
}
