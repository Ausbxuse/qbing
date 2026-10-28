package lab.cube.core

/** Logical-pixel geometry for focused, tiled and short timer windows. */
data class TimerGeometry(val cy:Float,val clockHeight:Float,val clockFont:Int,val cubeWidth:Float,val cubeHeight:Float,val cubeTop:Float,val scrambleTop:Float,val scrambleHeight:Float,val phaseHeight:Float,val compact:Boolean,val dense:Boolean,val collapsed:Boolean,val actions:Boolean,val showAverages:Boolean)
fun timerGeometry(width:Float,height:Float,mobile:Boolean):TimerGeometry {
 val compact=width<600;val dense=height<640;val collapsed=height<400;val actions=height>=300
 val phaseH=if(collapsed)32f else if(dense)if(compact)112f else 84f else if(compact)172f else 130f
 val phaseTop=height-(if(dense)8f else if(mobile)4f else 24f)-phaseH
 val clockH=if(collapsed)56f else if(dense)64f else if(mobile)88f else 100f
 val cubeH=if(collapsed)40f else if(dense)if(compact)if(height>=424)64f else 52f else if(height>=460)96f else 64f else if(mobile)128f else if(height<684)104f else 164f
 val cubeW=if(dense)cubeH*1.25f else if(mobile)160f else if(height<684)144f else 200f
 val desired=height*if(mobile).457f else .47f
 val scrambleH=if(collapsed)44f else if(dense&&compact)72f else 60f
 val scrambleTop=if(dense||height<684||mobile)if(collapsed)4f else 8f else desired-303f
 val lower=scrambleTop+scrambleH+(if(dense)8f else 12f)+cubeH+(if(dense)12f else 20f)+clockH/2
 val upper=phaseTop-(if(actions)76f else 34f)-clockH/2
 val cy=if(dense)desired.coerceIn(minOf(lower,upper),upper)else desired
 val cubeTop=if(dense)cy-clockH/2-12-cubeH else cy-if(mobile)186f else if(height<=684)174f else 235f
 val averageTop=if(dense)cy+clockH/2+80 else cy-clockH/2+if(mobile)173f else 186f
 return TimerGeometry(cy,clockH,if(collapsed)44 else if(dense)56 else if(mobile)64 else 88,cubeW,cubeH,cubeTop,scrambleTop,scrambleH,phaseH,compact,dense,collapsed,actions,averageTop+(if(mobile)70 else 74)+8<=phaseTop)
}
