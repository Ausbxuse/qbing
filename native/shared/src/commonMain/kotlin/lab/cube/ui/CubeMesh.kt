package lab.cube.ui

import lab.cube.core.Cube
import kotlin.math.*

/** Rounded assembled silhouette, with closed mechanical faces beneath each moving layer. */
internal object CubeMesh {
    const val CAMERA_DISTANCE=16f
    const val PROJECTION_SCALE=.195f
    const val STRIDE=12 // view position, view normal, local shell position, unlit sRGB albedo
    private const val HALF=.496f
    private const val RADIUS=.075f
    private val normals=arrayOf(floatArrayOf(0f,1f,0f),floatArrayOf(1f,0f,0f),floatArrayOf(0f,0f,1f),floatArrayOf(0f,-1f,0f),floatArrayOf(-1f,0f,0f),floatArrayOf(0f,0f,-1f))
    private fun dot(a:FloatArray,b:FloatArray)=a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
    private fun add(a:FloatArray,b:FloatArray)=FloatArray(3){a[it]+b[it]}
    private fun mul(a:FloatArray,b:Float)=FloatArray(3){a[it]*b}
    private fun cross(a:FloatArray,b:FloatArray)=floatArrayOf(a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
    private fun unit(a:FloatArray)=mul(a,1/sqrt(dot(a,a)).coerceAtLeast(.00000001f))
    private data class ShellVertex(val position:FloatArray,val normal:FloatArray,val shellNormal:FloatArray,val crown:Float,val face:Int)
    private val shell:List<ShellVertex> by lazy {
        val inner=HALF-RADIUS
        val grid=floatArrayOf(-HALF,-inner-RADIUS*.7f,-inner-RADIUS*.3f,-inner,-.35f,-.20f,0f,.20f,.35f,inner,inner+RADIUS*.3f,inner+RADIUS*.7f,HALF)
        buildList {
            normals.forEachIndexed {face,n->
                val u=if(n[0]!=0f)floatArrayOf(0f,1f,0f)else floatArrayOf(1f,0f,0f)
                val v=cross(n,u)
                fun vertex(a:Float,b:Float):ShellVertex {
                    val p=add(mul(n,HALF),add(mul(u,a),mul(v,b)))
                    val center=FloatArray(3){p[it].coerceIn(-inner,inner)}
                    val direction=unit(FloatArray(3){p[it]-center[it]})
                    val x=a/HALF;val y=b/HALF;val sx=max(1-x*x,0f);val sy=max(1-y*y,0f)
                    val crown=.004f*sx*sx*sy*sy
                    val da=-4*.004f/HALF*x*sx*sy*sy;val db=-4*.004f/HALF*y*sy*sx*sx
                    return ShellVertex(add(center,mul(direction,RADIUS)),unit(add(add(direction,mul(u,-da)),mul(v,-db))),direction,crown,face)
                }
                for(row in 0 until grid.lastIndex)for(col in 0 until grid.lastIndex){
                    val a=vertex(grid[col],grid[row]);val b=vertex(grid[col+1],grid[row])
                    val c=vertex(grid[col+1],grid[row+1]);val d=vertex(grid[col],grid[row+1])
                    addAll(listOf(a,b,c,a,c,d))
                }
            }
        }
    }
    /** Injective radial deformation only near the outside of the assembled cube. */
    internal fun outerProfile(point:FloatArray):FloatArray {
        val center=FloatArray(3){point[it].coerceIn(-1.196f,1.196f)}
        val delta=FloatArray(3){point[it]-center[it]}
        val extent=max(abs(delta[0]),max(abs(delta[1]),abs(delta[2])))
        val length=sqrt(dot(delta,delta))
        return if(length<.0000001f)point.copyOf()else add(center,mul(delta,extent/length))
    }
    private fun profileNormal(point:FloatArray,n:FloatArray):FloatArray {
        val helper=if(abs(n[1])<.9f)floatArrayOf(0f,1f,0f)else floatArrayOf(1f,0f,0f)
        val u=unit(cross(helper,n));val v=cross(n,u)
        fun differential(t:FloatArray):FloatArray {
            val a=outerProfile(add(point,mul(t,.0005f)));val b=outerProfile(add(point,mul(t,-.0005f)))
            return FloatArray(3){a[it]-b[it]}
        }
        return unit(cross(differential(u),differential(v)))
    }
    // Position, actual deformed normal, original shell position, face index.
    // Cache all 27 shapes: the outer profile is computed once, never per animation frame.
    private val shapes:Array<FloatArray> by lazy {
        Array(27){index->
            val center=floatArrayOf((index/9-1).toFloat(),((index/3)%3-1).toFloat(),(index%3-1).toFloat())
            val result=FloatArray(shell.size*10);var at=0
            for(vertex in shell){
                val exterior=dot(center,normals[vertex.face])==1f
                val point=add(center,if(exterior)add(vertex.position,mul(normals[vertex.face],vertex.crown))else vertex.position)
                val position=outerProfile(point);val normal=profileNormal(point,if(exterior)vertex.normal else vertex.shellNormal)
                for(i in 0..2)result[at++]=position[i]
                for(i in 0..2)result[at++]=normal[i]
                for(i in 0..2)result[at++]=vertex.position[i]
                result[at++]=vertex.face.toFloat()
            };result
        }
    }
    private val indices=buildList<IntArray>{for(x in -1..1)for(y in -1..1)for(z in -1..1)add(IntArray(6){face->Cube.geometry.indexOfFirst{it.p==listOf(x,y,z)&&it.n==normals[face].map(Float::toInt)}})}
    private fun rgb(face:Char)=when(face){'U'->0xf4f7fa;'D'->0xffd81e;'F'->0x10ca64;'B'->0x1a6ff1;'R'->0xef3041;'L'->0xff7f14;else->0x13161d}
    private fun rotate(x:Float,y:Float,z:Float,axis:Int,angle:Float):FloatArray {val c=cos(angle);val s=sin(angle);return when(axis){0->floatArrayOf(x,y*c-z*s,y*s+z*c);1->floatArrayOf(x*c+z*s,y,-x*s+z*c);else->floatArrayOf(x*c-y*s,x*s+y*c,z)}}
    private fun matrix(frame:CubeRenderFrame,axis:Int?,angle:Float):FloatArray {
        val q=frame.orientation;val out=FloatArray(9)
        for(i in 0..2){
            var p=FloatArray(3){if(it==i)1f else 0f};if(axis!=null)p=rotate(p[0],p[1],p[2],axis,angle)
            val tx=2*(q.y*p[2]-q.z*p[1]);val ty=2*(q.z*p[0]-q.x*p[2]);val tz=2*(q.x*p[1]-q.y*p[0])
            p=floatArrayOf(p[0]+q.w*tx+q.y*tz-q.z*ty,p[1]+q.w*ty+q.z*tx-q.x*tz,p[2]+q.w*tz+q.x*ty-q.y*tx)
            p=rotate(p[0],p[1],p[2],1,frame.yaw);p=rotate(p[0],p[1],p[2],0,frame.pitch)
            for(j in 0..2)out[j*3+i]=p[j]
        };return out
    }
    /** Counterclockwise closed triangles; reuse avoids large allocations on Android's render thread. */
    fun vertices(frame:CubeRenderFrame,reuse:FloatArray?=null):FloatArray {
        val count=27*shell.size*STRIDE
        val output=reuse?.takeIf{it.size==count}?:FloatArray(count);var cursor=0;var piece=0
        val resting=matrix(frame,null,0f);val turn=cubeTurnSpecs[frame.move?.firstOrNull()];val moving=if(turn!=null)matrix(frame,turn.first,frame.angle)else resting
        val partner=cubeTurnSpecs[frame.partnerMove?.firstOrNull()];val partnered=if(partner!=null)matrix(frame,partner.first,frame.partnerAngle)else resting
        for(x in -1..1)for(y in -1..1)for(z in -1..1){
            val position=intArrayOf(x,y,z);val m=when{turn!=null&&position[turn.first]==turn.second->moving;partner!=null&&position[partner.first]==partner.second->partnered;else->resting}
            val colors=IntArray(6){f->rgb(indices[piece][f].takeIf{it>=0}?.let{frame.state.getOrNull(it)}?:' ')}
            val shape=shapes[piece++];var i=0
            while(i<shape.size){
                for(segment in 0..1){
                    val offset=segment*3
                    val px=shape[i+offset];val py=shape[i+offset+1];val pz=shape[i+offset+2]
                    output[cursor++]=m[0]*px+m[1]*py+m[2]*pz;output[cursor++]=m[3]*px+m[4]*py+m[5]*pz;output[cursor++]=m[6]*px+m[7]*py+m[8]*pz
                }
                output[cursor++]=shape[i+6];output[cursor++]=shape[i+7];output[cursor++]=shape[i+8]
                val color=colors[shape[i+9].toInt()]
                output[cursor++]=((color shr 16)and 255)/255f;output[cursor++]=((color shr 8)and 255)/255f;output[cursor++]=(color and 255)/255f
                i+=10
            }
        };return output
    }
}

/** Matches Linux material.rs: linear-light satin plastic under one broad softbox. */
internal object CubeMaterial {
    private fun linear(c:Float)=if(c<=.04045f)c/12.92f else ((c+.055f)/1.055f).pow(2.4f)
    private fun srgb(c:Float)=if(c<=.0031308f)c*12.92f else 1.055f*c.pow(1/2.4f)-.055f
    fun argb(surface:FloatArray):Int {
        val px=surface[0];val py=surface[1];val pz=surface[2]
        val nl=sqrt(surface[3]*surface[3]+surface[4]*surface[4]+surface[5]*surface[5]).coerceAtLeast(.000001f)
        val nx=surface[3]/nl;val ny=surface[4]/nl;val nz=surface[5]/nl
        val kx=-3.5f-px;val ky=6.5f-py;val kz=10f-pz;val kl2=kx*kx+ky*ky+kz*kz;val kl=sqrt(kl2)
        val fx=7-px;val fy=-1-py;val fz=6-pz;val fl=sqrt(fx*fx+fy*fy+fz*fz)
        val keyGain=(154.5f/kl2).coerceIn(.78f,1.20f)
        val t=((-ny-.55f)/.30f).coerceIn(0f,1f);val underside=t*t*(3-2*t)
        val roomGain=1-.30f*underside;val ambient=.41f+.12f*(.5f+.5f*ny)
        val ax=abs(surface[6]);val ay=abs(surface[7]);val az=abs(surface[8]);val middle=max(min(ax,ay),min(max(ax,ay),az))
        val contact=1-.045f*exp(-max(.496f-middle,0f)/.018f)
        var diffuse=((ambient+.16f*max((nx*fx+ny*fy+nz*fz)/fl,0f))*roomGain+.49f*max((nx*kx+ny*ky+nz*kz)/kl,0f)*keyGain)*contact
        if(diffuse>.90f)diffuse=.90f+.08f*(1-exp(-(diffuse-.90f)/.08f))
        val ez=CubeMesh.CAMERA_DISTANCE-pz;val el=sqrt(px*px+py*py+ez*ez)
        val hx=kx/kl-px/el;val hy=ky/kl-py/el;val hz=kz/kl+ez/el;val hl=sqrt(hx*hx+hy*hy+hz*hz)
        val r=linear(surface[9]);val g=linear(surface[10]);val b=linear(surface[11])
        val pt=((max(r,max(g,b))-.015f)/.045f).coerceIn(0f,1f);val plastic=pt*pt*(3-2*pt)
        val specular=(.012f+.025f*plastic)*max((nx*hx+ny*hy+nz*hz)/hl,0f).pow(6)*keyGain*(1-.35f*underside)
        fun channel(c:Float)=(srgb((c*diffuse+specular).coerceIn(0f,1f))*255).roundToInt().coerceIn(0,255)
        return 0xff000000.toInt() or (channel(r) shl 16) or (channel(g) shl 8) or channel(b)
    }
    val fragment="""
        #ifdef GL_FRAGMENT_PRECISION_HIGH
        precision highp float;
        #else
        precision mediump float;
        #endif
        varying vec3 tint;
        varying vec3 surfaceNormal;
        varying vec3 surfacePosition;
        varying vec3 surfaceLocal;
        uniform float cameraDistance;
        vec3 linearColor(vec3 c) {
            return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), c));
        }
        vec3 srgbColor(vec3 c) {
            return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c));
        }
        void main() {
            vec3 n = normalize(surfaceNormal);
            vec3 toKey = vec3(-3.5, 6.5, 10.0) - surfacePosition;
            vec3 key = normalize(toKey);
            vec3 fill = normalize(vec3(7.0, -1.0, 6.0) - surfacePosition);
            float keyGain = clamp(154.5 / dot(toKey, toKey), 0.78, 1.20);
            float underside = smoothstep(0.55, 0.85, -n.y);
            float roomGain = 1.0 - 0.30 * underside;
            float ambient = 0.41 + 0.12 * (0.5 + 0.5 * n.y);
            vec3 a = abs(surfaceLocal);
            float middle = max(min(a.x, a.y), min(max(a.x, a.y), a.z));
            float edge = max(0.496 - middle, 0.0);
            float contact = 1.0 - 0.045 * exp(-edge / 0.018);
            float diffuse = ((ambient + 0.16 * max(dot(n, fill), 0.0)) * roomGain
                + 0.49 * max(dot(n, key), 0.0) * keyGain) * contact;
            if (diffuse > 0.90) {
                diffuse = 0.90 + 0.08 * (1.0 - exp(-(diffuse - 0.90) / 0.08));
            }
            vec3 eye = normalize(vec3(-surfacePosition.xy, cameraDistance - surfacePosition.z));
            vec3 albedo = linearColor(tint);
            float plastic = smoothstep(0.015, 0.060, max(albedo.r, max(albedo.g, albedo.b)));
            float specular = mix(0.012, 0.037, plastic)
                * pow(max(dot(n, normalize(key + eye)), 0.0), 6.0) * keyGain
                * (1.0 - 0.35 * underside);
            gl_FragColor = vec4(srgbColor(clamp(albedo * diffuse + specular, 0.0, 1.0)), 1.0);
        }
    """.trimIndent()
}
