package lab.cube.ui

import android.content.Context
import android.graphics.SurfaceTexture
import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLSurface
import android.opengl.GLES30
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.view.Surface
import android.view.TextureView
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.util.zip.GZIPInputStream
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.min

@Composable internal actual fun NativeCubeSurface(frame:CubeRenderFrame,modifier:Modifier) {
    val context=LocalContext.current
    val assets by produceState<ApprovedCubeAssets?>(null){value=approvedCubeAssets()}
    val ready=assets?:return
    val view=remember(context,ready){CubeTextureView(context,ready)}
    DisposableEffect(view){onDispose{view.close()}}
    AndroidView(factory={view},modifier=modifier,update={it.submit(frame)})
}

/** TextureView participates in normal Compose clipping/alpha, unlike a topmost SurfaceView. */
private class CubeTextureView(context:Context,assets:ApprovedCubeAssets):TextureView(context),TextureView.SurfaceTextureListener {
    private val thread=HandlerThread("Cube drawing").apply{start()}
    private val handler=Handler(thread.looper)
    private val queued=AtomicBoolean(false)
    private val closed=AtomicBoolean(false)
    @Volatile private var latest:CubeRenderFrame?=null
    @Volatile private var drawingVisible=true
    private val graphics=CubeGl(assets)
    init {isOpaque=false;surfaceTextureListener=this}
    fun submit(frame:CubeRenderFrame){if(latest==frame)return;latest=frame;redraw()}
    private fun redraw(){
        if(closed.get()||!drawingVisible||!queued.compareAndSet(false,true))return
        handler.post{queued.set(false);if(!closed.get()&&drawingVisible)runCatching{latest?.let(graphics::draw)}.onFailure{Log.e("CubeLab","Cube drawing failed",it)}}
    }
    override fun onWindowVisibilityChanged(visibility:Int){super.onWindowVisibilityChanged(visibility);drawingVisible=visibility==VISIBLE;if(drawingVisible)redraw()}
    override fun onSurfaceTextureAvailable(texture:SurfaceTexture,width:Int,height:Int){
        handler.post{if(closed.get())return@post;runCatching{graphics.open(texture,width,height);latest?.let(graphics::draw)}.onFailure{Log.e("CubeLab","Cube graphics unavailable",it);graphics.close()}}
    }
    override fun onSurfaceTextureSizeChanged(texture:SurfaceTexture,width:Int,height:Int){handler.post{if(!closed.get())runCatching{graphics.resize(texture,width,height);latest?.let(graphics::draw)}.onFailure{Log.e("CubeLab","Cube resize failed",it)}}}
    override fun onSurfaceTextureUpdated(texture:SurfaceTexture)=Unit
    override fun onSurfaceTextureDestroyed(texture:SurfaceTexture):Boolean {
        // Retain the texture until in-flight GL commands and its EGL window have been released.
        handler.post{graphics.close();texture.release()};return false
    }
    fun close(){
        if(!closed.compareAndSet(false,true))return
        surfaceTextureListener=null
        // With no listener installed TextureView owns/reclaims its SurfaceTexture on detach.
        handler.post{graphics.close();thread.quitSafely()}
    }
}

private class CubeGl(private val assets:ApprovedCubeAssets) {
    private var display:EGLDisplay=EGL14.EGL_NO_DISPLAY
    private var context:EGLContext=EGL14.EGL_NO_CONTEXT
    private var surface:EGLSurface=EGL14.EGL_NO_SURFACE
    private var window:Surface?=null
    private var program=0;private var buffer=0
    private var width=1;private var height=1
    private var indexBuffer=0
    private var count=0
    private var lastState:String?=null
    private fun uniform(name:String)=GLES30.glGetUniformLocation(program,name)
    fun open(texture:SurfaceTexture,w:Int,h:Int){
        close();display=EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
        check(display!=EGL14.EGL_NO_DISPLAY&&EGL14.eglInitialize(display,IntArray(2),0,IntArray(2),0)){"Cannot initialize EGL"}
        fun choose(samples:Int,depth:Int):EGLConfig? {
            val attributes=intArrayOf(EGL14.EGL_SURFACE_TYPE,EGL14.EGL_WINDOW_BIT,EGL14.EGL_RENDERABLE_TYPE,0x40,EGL14.EGL_RED_SIZE,8,EGL14.EGL_GREEN_SIZE,8,EGL14.EGL_BLUE_SIZE,8,EGL14.EGL_ALPHA_SIZE,8,EGL14.EGL_DEPTH_SIZE,depth,EGL14.EGL_SAMPLE_BUFFERS,if(samples>0)1 else 0,EGL14.EGL_SAMPLES,samples,EGL14.EGL_NONE)
            val choices=arrayOfNulls<EGLConfig>(1);val count=IntArray(1)
            return if(EGL14.eglChooseConfig(display,attributes,0,choices,0,1,count,0)&&count[0]>0)choices[0]else null
        }
        val config=choose(4,24)?:choose(0,24)?:choose(0,16)?:error("No depth-capable EGL configuration")
        context=EGL14.eglCreateContext(display,config,EGL14.EGL_NO_CONTEXT,intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION,3,EGL14.EGL_NONE),0)
        check(context!=EGL14.EGL_NO_CONTEXT){"Cannot create cube graphics context"}
        resize(texture,w,h);window=Surface(texture)
        surface=EGL14.eglCreateWindowSurface(display,config,window,intArrayOf(EGL14.EGL_NONE),0)
        check(surface!=EGL14.EGL_NO_SURFACE&&EGL14.eglMakeCurrent(display,surface,surface,context)){"Cannot create cube graphics surface"}
        fun shader(type:Int,source:String):Int {
            val id=GLES30.glCreateShader(type);GLES30.glShaderSource(id,source);GLES30.glCompileShader(id)
            val status=IntArray(1);GLES30.glGetShaderiv(id,GLES30.GL_COMPILE_STATUS,status,0)
            check(status[0]!=0){GLES30.glGetShaderInfoLog(id)};return id
        }
        val vertex=shader(GLES30.GL_VERTEX_SHADER,"#version 300 es\nprecision highp float;precision highp int;\n"+assets.vertex)
        val fragment=shader(GLES30.GL_FRAGMENT_SHADER,"#version 300 es\nprecision highp float;precision highp int;\n"+assets.fragment)
        program=GLES30.glCreateProgram();GLES30.glAttachShader(program,vertex);GLES30.glAttachShader(program,fragment)
        for((i,name)in listOf("position","normal","home","face").withIndex())GLES30.glBindAttribLocation(program,i,name)
        GLES30.glLinkProgram(program);GLES30.glDeleteShader(vertex);GLES30.glDeleteShader(fragment)
        val status=IntArray(1);GLES30.glGetProgramiv(program,GLES30.GL_LINK_STATUS,status,0);check(status[0]!=0){GLES30.glGetProgramInfoLog(program)}
        val raw=GZIPInputStream(assets.scene.inputStream()).use{it.readBytes()}
        val scene=ByteBuffer.wrap(raw).order(ByteOrder.LITTLE_ENDIAN)
        val vertices=scene.int;count=scene.int
        fun upload(target:Int,offset:Int,bytes:Int):Int {val names=IntArray(1);GLES30.glGenBuffers(1,names,0);GLES30.glBindBuffer(target,names[0]);val data=ByteBuffer.allocateDirect(bytes).order(ByteOrder.nativeOrder());data.put(raw,offset,bytes).position(0);GLES30.glBufferData(target,bytes,data,GLES30.GL_STATIC_DRAW);return names[0]}
        buffer=upload(GLES30.GL_ARRAY_BUFFER,8,vertices*40)
        indexBuffer=upload(GLES30.GL_ELEMENT_ARRAY_BUFFER,8+vertices*40,count*4)
        GLES30.glClearColor(0f,0f,0f,0f);GLES30.glEnable(GLES30.GL_DEPTH_TEST);GLES30.glDepthFunc(GLES30.GL_LESS)
        GLES30.glEnable(GLES30.GL_CULL_FACE);GLES30.glCullFace(GLES30.GL_BACK);GLES30.glFrontFace(GLES30.GL_CCW)
        GLES30.glDisable(GLES30.GL_BLEND)
    }
    fun resize(texture:SurfaceTexture,w:Int,h:Int){
        width=w.coerceAtLeast(1);height=h.coerceAtLeast(1)
        texture.setDefaultBufferSize(width,height)
    }
    fun draw(frame:CubeRenderFrame){
        if(surface==EGL14.EGL_NO_SURFACE||program==0)return
        if(!EGL14.eglMakeCurrent(display,surface,surface,context))return
        GLES30.glViewport(0,0,width,height);GLES30.glClear(GLES30.GL_COLOR_BUFFER_BIT or GLES30.GL_DEPTH_BUFFER_BIT)
        GLES30.glUseProgram(program);GLES30.glBindBuffer(GLES30.GL_ARRAY_BUFFER,buffer);GLES30.glBindBuffer(GLES30.GL_ELEMENT_ARRAY_BUFFER,indexBuffer)
        for((attribute,size)in listOf(3,3,3,1).withIndex()){GLES30.glEnableVertexAttribArray(attribute);GLES30.glVertexAttribPointer(attribute,size,GLES30.GL_FLOAT,false,40,attribute*12)}
        GLES30.glUniform2f(uniform("aspect"),height.toFloat()/width,1f);GLES30.glUniform2f(uniform("orbit"),frame.yaw,frame.pitch);GLES30.glUniform1f(uniform("logoAngle"),frame.logoAngle)
        val q=frame.orientation;GLES30.glUniform4f(uniform("orientation"),q.x,q.y,q.z,q.w)
        val turns=FloatArray(8)
        for((i,pair)in listOf(frame.move to frame.angle,frame.partnerMove to frame.partnerAngle).withIndex()){
            val spec=pair.first?.firstOrNull()?.let{cubeTurnSpecs[it]}?:continue
            turns[i*4]=spec.first.toFloat();turns[i*4+1]=spec.second.toFloat();turns[i*4+2]=pair.second;turns[i*4+3]=1f
        }
        GLES30.glUniform4fv(uniform("turns[0]"),2,turns,0)
        if(frame.state!=lastState){GLES30.glUniform1iv(uniform("stickers[0]"),54,IntArray(54){"URFDLB".indexOf(frame.state[it]).coerceAtLeast(0)},0);lastState=frame.state}
        GLES30.glDrawElements(GLES30.GL_TRIANGLES,count,GLES30.GL_UNSIGNED_INT,0)
        EGL14.eglSwapBuffers(display,surface)
    }
    fun close(){
        if(display!=EGL14.EGL_NO_DISPLAY){
            if(context!=EGL14.EGL_NO_CONTEXT&&surface!=EGL14.EGL_NO_SURFACE){
                EGL14.eglMakeCurrent(display,surface,surface,context)
                if(buffer!=0)GLES30.glDeleteBuffers(2,intArrayOf(buffer,indexBuffer),0)
                if(program!=0)GLES30.glDeleteProgram(program)
            }
            EGL14.eglMakeCurrent(display,EGL14.EGL_NO_SURFACE,EGL14.EGL_NO_SURFACE,EGL14.EGL_NO_CONTEXT)
            if(surface!=EGL14.EGL_NO_SURFACE)EGL14.eglDestroySurface(display,surface)
            if(context!=EGL14.EGL_NO_CONTEXT)EGL14.eglDestroyContext(display,context)
            EGL14.eglTerminate(display);EGL14.eglReleaseThread()
        }
        window?.release();window=null;display=EGL14.EGL_NO_DISPLAY;context=EGL14.EGL_NO_CONTEXT;surface=EGL14.EGL_NO_SURFACE
        program=0;buffer=0;indexBuffer=0;lastState=null
    }
}
