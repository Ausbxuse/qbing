package lab.cube.ui

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.unit.IntSize
import java.awt.image.BufferedImage
import kotlin.math.*

/** Desktop reference renderer has the same mesh and real per-pixel occlusion as Android. */
@Composable internal actual fun NativeCubeSurface(frame:CubeRenderFrame,modifier:Modifier) {
    val vertices=remember(frame){CubeMesh.vertices(frame)}
    Canvas(modifier){
        val width=size.width.toInt().coerceAtLeast(1);val height=size.height.toInt().coerceAtLeast(1)
        val raster=CubeRaster.image(vertices,width*2,height*2)
        drawImage(raster.toComposeImageBitmap(),dstSize=IntSize(width,height))
    }
}

internal object CubeRaster {
    fun image(vertices:FloatArray,width:Int,height:Int):BufferedImage {
        val pixels=IntArray(width*height);val zbuffer=FloatArray(pixels.size){Float.NEGATIVE_INFINITY}
        val scale=min(width,height)*CubeMesh.PROJECTION_SCALE
        var start=0
        val x=FloatArray(3);val y=FloatArray(3);val inverse=FloatArray(3)
        val surface=FloatArray(CubeMesh.STRIDE)
        while(start<vertices.size){
            for(i in 0..2){val v=start+i*CubeMesh.STRIDE;inverse[i]=1/(CubeMesh.CAMERA_DISTANCE-vertices[v+2]);x[i]=width/2f+vertices[v]*scale*CubeMesh.CAMERA_DISTANCE*inverse[i];y[i]=height/2f-vertices[v+1]*scale*CubeMesh.CAMERA_DISTANCE*inverse[i]}
            val area=(x[1]-x[0])*(y[2]-y[0])-(y[1]-y[0])*(x[2]-x[0])
            if(area>=-0.00001f){start+=CubeMesh.STRIDE*3;continue} // CCW becomes clockwise after the screen Y flip.
            val left=floor(x.min()).toInt().coerceAtLeast(0);val right=ceil(x.max()).toInt().coerceAtMost(width-1)
            val top=floor(y.min()).toInt().coerceAtLeast(0);val bottom=ceil(y.max()).toInt().coerceAtMost(height-1)
            for(py in top..bottom)for(px in left..right){
                val sx=px+.5f;val sy=py+.5f
                val a=((x[1]-sx)*(y[2]-sy)-(y[1]-sy)*(x[2]-sx))/area
                val b=((x[2]-sx)*(y[0]-sy)-(y[2]-sy)*(x[0]-sx))/area;val c=1-a-b
                if(a<-.00001f||b<-.00001f||c<-.00001f)continue
                val depth=a*inverse[0]+b*inverse[1]+c*inverse[2];val index=py*width+px
                if(depth<=zbuffer[index])continue;zbuffer[index]=depth
                for(attribute in surface.indices){
                    surface[attribute]=(a*vertices[start+attribute]*inverse[0]+b*vertices[start+CubeMesh.STRIDE+attribute]*inverse[1]+c*vertices[start+CubeMesh.STRIDE*2+attribute]*inverse[2])/depth
                };pixels[index]=CubeMaterial.argb(surface)
            };start+=CubeMesh.STRIDE*3
        }
        return BufferedImage(width,height,BufferedImage.TYPE_INT_ARGB).apply{setRGB(0,0,width,height,pixels,0,width)}
    }
}
