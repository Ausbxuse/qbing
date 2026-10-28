package lab.cube.ui

import java.io.File
import javax.imageio.ImageIO
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlin.math.sqrt
import kotlin.math.PI
import kotlin.math.sin
import kotlin.math.cos
import kotlin.test.*
import lab.cube.core.Cube

class CubeRendererTest {
    private fun axisRotation(axis:Int,degrees:Float):CubeQuaternion {
        val radians=degrees*PI.toFloat()/360f;val s=sin(radians)
        return CubeQuaternion(if(axis==0)s else 0f,if(axis==1)s else 0f,if(axis==2)s else 0f,cos(radians))
    }
    @Test fun gyroSnapContainsAllTwentyFourProperCubeOrientations(){
        val poses=CubeOrientationSnap.poses
        assertEquals(24,poses.size)
        for((i,pose)in poses.withIndex()){
            assertTrue(poses.take(i).none{it.closeTo(pose)})
            assertTrue(CubeOrientationSnap().update(pose).closeTo(pose))
            for(axis in 0..2){
                val p=FloatArray(3){if(it==axis)1f else 0f}
                val tx=2*(pose.y*p[2]-pose.z*p[1]);val ty=2*(pose.z*p[0]-pose.x*p[2]);val tz=2*(pose.x*p[1]-pose.y*p[0])
                val rotated=floatArrayOf(p[0]+pose.w*tx+pose.y*tz-pose.z*ty,p[1]+pose.w*ty+pose.z*tx-pose.x*tz,p[2]+pose.w*tz+pose.x*ty-pose.y*tx)
                assertEquals(1,rotated.count{abs(it)>.9999f},"Snapped poses must align faces to proper signed axes")
            }
        }
    }
    @Test fun gyroSnapHysteresisPreventsChatterAtHalfQuarterTurns(){
        for(axis in 0..2){
            val snap=CubeOrientationSnap()
            for(degrees in listOf(44f,46f,48.9f,45f,47f))assertTrue(snap.update(axisRotation(axis,degrees)).closeTo(CubeQuaternion()))
            assertTrue(snap.update(axisRotation(axis,50f)).closeTo(axisRotation(axis,90f)))
            for(degrees in listOf(46f,44f,41.1f,47f,43f))assertTrue(snap.update(axisRotation(axis,degrees)).closeTo(axisRotation(axis,90f)))
            assertTrue(snap.update(axisRotation(axis,40f)).closeTo(CubeQuaternion()))
        }
    }
    @Test fun gyroSnapResetForgetsHysteresisWithoutChangingLogicalTurns(){
        val snap=CubeOrientationSnap();val animator=CubeTurnAnimator(Cube.solved)
        animator.enqueue(Cube.turn(Cube.solved,"R"),"R");animator.advance(0)
        snap.update(axisRotation(1,90f));snap.reset()
        assertTrue(snap.selected.closeTo(CubeQuaternion()))
        assertTrue(snap.update(axisRotation(1,46f)).closeTo(CubeQuaternion()))
        animator.advance(1_000_000_000)
        assertEquals(Cube.turn(Cube.solved,"R"),animator.state)
        val pose=axisRotation(2,90f);val antipodal=CubeQuaternion(-pose.x,-pose.y,-pose.z,-pose.w)
        assertTrue(snap.update(antipodal).closeTo(pose))
    }
    @Test fun animationCarriesElapsedTimeAcrossQueuedTurns(){
        val animator=CubeTurnAnimator(Cube.solved);var state=Cube.solved
        val moves=listOf("R","U","R'","U'")
        for(move in moves){state=Cube.turn(state,move);animator.enqueue(state,move)}
        animator.advance(0)
        val moving=animator.advance(200_000_000)
        assertEquals(Cube.apply(Cube.solved,listOf("R","U")),animator.state)
        assertEquals("R'",moving.move)
        assertTrue(abs(moving.angle)>0f)
        animator.advance(1_000_000_000)
        assertEquals(state,animator.state);assertFalse(animator.busy)
    }
    @Test fun trustedOppositeInverseTurnsAnimateTogetherOnAllAxesAndOrders(){
        for(moves in listOf(listOf("R","L'"),listOf("L'","R"),listOf("U'","D"),listOf("D","U'"),listOf("F","B'"),listOf("B'","F"))){
            val animator=CubeTurnAnimator(Cube.solved);var state=Cube.solved
            for((i,move)in moves.withIndex()){state=Cube.turn(state,move);animator.enqueue(state,move,1000.0+i*35)}
            animator.advance(0)
            val pose=animator.advance(65_000_000)
            assertEquals(moves[0],pose.move);assertEquals(moves[1],pose.partnerMove)
            assertTrue(abs(pose.angle)>.1f);assertEquals(pose.angle,pose.partnerAngle,.00001f)
            assertEquals(Cube.solved,animator.state,"Logical endpoint is committed only after both visual layers finish")
            animator.advance(130_000_000)
            assertEquals(state,animator.state);assertFalse(animator.busy)
        }
    }
    @Test fun slowRecoveredMissingClockAndSameDirectionTurnsStaySequential(){
        data class Case(val second:String="L'",val gap:Double?=101.0,val firstRecovered:Boolean=false,val secondRecovered:Boolean=false)
        for(case in listOf(Case(),Case(gap=null),Case(gap=30.0,firstRecovered=true),Case(gap=30.0,secondRecovered=true),Case(gap=-1.0),Case(gap=Double.NaN),Case(second="L",gap=30.0),Case(second="U'",gap=30.0))){
            val animator=CubeTurnAnimator(Cube.solved)
            val first=Cube.turn(Cube.solved,"R");val final=Cube.turn(first,case.second)
            animator.enqueue(first,"R",1000.0,case.firstRecovered)
            animator.enqueue(final,case.second,case.gap?.let{1000+it},case.secondRecovered)
            assertNull(animator.advance(0).partnerMove)
            assertNull(animator.advance(50_000_000).partnerMove)
            animator.advance(1_000_000_000)
            assertEquals(final,animator.state)
        }
    }
    @Test fun latePartnerJoinsWithoutJumpingEitherLayerAndSharesTheEndpoint(){
        val animator=CubeTurnAnimator(Cube.solved)
        val first=Cube.turn(Cube.solved,"R");val final=Cube.turn(first,"L'")
        animator.enqueue(first,"R",1000.0);animator.advance(0)
        val before=animator.advance(40_000_000)
        animator.enqueue(final,"L'",1040.0)
        val joined=animator.advance(40_000_000)
        assertEquals(before.angle,joined.angle,.000001f)
        assertEquals("L'",joined.partnerMove);assertEquals(0f,joined.partnerAngle,.000001f)
        val middle=animator.advance(85_000_000)
        assertTrue(abs(middle.angle)>abs(joined.angle));assertTrue(abs(middle.partnerAngle)>0f)
        val nearEnd=animator.advance(129_000_000)
        assertEquals(nearEnd.angle,nearEnd.partnerAngle,.00005f)
        animator.advance(130_000_000)
        assertEquals(final,animator.state);assertFalse(animator.busy)
    }
    @Test fun repeatedPairedQuarterTurnsKeepRawM2EndpointAndFrameOvershoot(){
        val animator=CubeTurnAnimator(Cube.solved);var state=Cube.solved
        val moves=listOf("R","L'","R","L'")
        for((i,move)in moves.withIndex()){state=Cube.turn(state,move);animator.enqueue(state,move,1000.0+i*20)}
        assertEquals("L'",animator.advance(0).partnerMove)
        val second=animator.advance(100_000_000)
        assertEquals(Cube.apply(Cube.solved,moves.take(2)),animator.state)
        assertEquals("R",second.move);assertEquals("L'",second.partnerMove);assertTrue(abs(second.angle)>0f)
        animator.advance(1_000_000_000)
        assertEquals(Cube.apply(Cube.solved,moves),animator.state);assertFalse(animator.busy)
    }
    @Test fun noninterleavedM2BurstUsesTwoPairedMotionsAndEachRawTurnOnce(){
        for(moves in listOf(listOf("R","R","L'","L'"),listOf("L'","L'","R","R"),listOf("U'","U'","D","D"),listOf("F","F","B'","B'"))){
            val animator=CubeTurnAnimator(Cube.solved);var rawState=Cube.solved
            for((i,move)in moves.withIndex()){rawState=Cube.turn(rawState,move);animator.enqueue(rawState,move,1000.0+i*20)}
            val first=animator.advance(0)
            assertEquals(moves[0],first.move);assertEquals(moves[2],first.partnerMove)
            val second=animator.advance(100_000_000)
            assertEquals(Cube.apply(Cube.solved,listOf(moves[0],moves[2])),animator.state,"Visual endpoint must exclude the still-queued second raw turn")
            assertEquals(moves[1],second.move);assertEquals(moves[3],second.partnerMove)
            animator.advance(1_000_000_000)
            assertEquals(rawState,animator.state);assertFalse(animator.busy)
        }
    }
    @Test fun activeTurnFindsLatePartnerAcrossOnlyItsSameAxisQueue(){
        val animator=CubeTurnAnimator(Cube.solved);var rawState=Cube.turn(Cube.solved,"R")
        animator.enqueue(rawState,"R",1000.0);animator.advance(0)
        val before=animator.advance(35_000_000)
        for((i,move)in listOf("R","L'","L'").withIndex()){rawState=Cube.turn(rawState,move);animator.enqueue(rawState,move,1020.0+i*20)}
        val joined=animator.advance(35_000_000)
        assertEquals(before.angle,joined.angle,.000001f);assertEquals("L'",joined.partnerMove);assertEquals(0f,joined.partnerAngle,.000001f)
        val second=animator.advance(145_000_000)
        assertEquals(Cube.apply(Cube.solved,listOf("R","L'")),animator.state)
        assertEquals("R",second.move);assertEquals("L'",second.partnerMove)
        animator.advance(1_000_000_000)
        assertEquals(rawState,animator.state);assertFalse(animator.busy)
    }
    @Test fun pairedSearchCannotCrossAxisClockRecoveryOrSlowBarriers(){
        data class Barrier(val move:String="R",val time:Double?=1020.0,val recovered:Boolean=false,val lastTime:Double=1040.0)
        for(barrier in listOf(Barrier(move="U"),Barrier(time=null),Barrier(recovered=true),Barrier(time=999.0),Barrier(time=1120.0,lastTime=1140.0))){
            val animator=CubeTurnAnimator(Cube.solved);var state=Cube.turn(Cube.solved,"R")
            animator.enqueue(state,"R",1000.0);animator.advance(0)
            state=Cube.turn(state,barrier.move);animator.enqueue(state,barrier.move,barrier.time,barrier.recovered)
            state=Cube.turn(state,"L'");animator.enqueue(state,"L'",barrier.lastTime)
            assertNull(animator.advance(35_000_000).partnerMove)
            animator.advance(1_000_000_000)
            assertEquals(state,animator.state)
        }
    }
    @Test fun pairedGeometryKeepsGyroIndependentAndFixedCenterFrame(){
        val base=CubeRenderFrame(Cube.solved,"R",-.7f,yaw=0f,pitch=0f,partnerMove="L'",partnerAngle=-.7f)
        val vertices=CubeMesh.vertices(base)
        val rotated=CubeMesh.vertices(base.copy(orientation=axisRotation(0,90f)))
        for(i in vertices.indices step CubeMesh.STRIDE){
            assertEquals(vertices[i],rotated[i],.00001f)
            assertEquals(-vertices[i+2],rotated[i+1],.00001f)
            assertEquals(vertices[i+1],rotated[i+2],.00001f)
            for(color in 9..11)assertEquals(vertices[i+color],rotated[i+color])
        }
        val resting=CubeMesh.vertices(base.copy(move=null,angle=0f,partnerMove=null,partnerAngle=0f))
        val verticesPerXBlock=vertices.size/3
        for(i in verticesPerXBlock until verticesPerXBlock*2)assertEquals(resting[i],vertices[i],.00001f,"Center frame is not replaced with a literal slice turn")
    }
    @Test fun authoritativeStateClearsStaleAnimation(){
        val animator=CubeTurnAnimator(Cube.solved)
        animator.enqueue(Cube.turn(Cube.solved,"R"),"R");animator.advance(0)
        val corrected=Cube.turn(Cube.solved,"U2");animator.enqueue(corrected,null)
        assertNull(animator.advance(50_000_000).move);assertEquals(corrected,animator.state)
    }
    @Test fun oppositeQuaternionSignsDoNotSpin(){
        val q=CubeQuaternion(.2f,.3f,.4f,.7f).normalized()
        val opposite=CubeQuaternion(-q.x,-q.y,-q.z,-q.w)
        assertTrue(q.interpolate(opposite,.5f).closeTo(q))
        assertTrue((q*q.inverse()).closeTo(CubeQuaternion()))
    }
    @Test fun gyroUsesTheInitialCubeFrameAfterAnAngledConnection(){
        val s=sqrt(.5f)
        // Hold the cube 90 degrees around Z, then rotate 90 degrees around its local X.
        // These absolute sensor orientations are specified independently of our multiplication.
        val initial=CubeQuaternion(0f,0f,s,s)
        val current=CubeQuaternion(.5f,.5f,.5f,.5f)
        val relative=current.relativeTo(initial)
        assertTrue(relative.closeTo(CubeQuaternion(s,0f,0f,s)))
        // The calibrated up vector must move to front, rather than remain on the Y axis.
        val rotatedUp=listOf(2*(relative.x*relative.y-relative.w*relative.z),1-2*(relative.x*relative.x+relative.z*relative.z),2*(relative.y*relative.z+relative.w*relative.x))
        for(i in 0..2)assertEquals(listOf(0f,0f,1f)[i],rotatedUp[i],.00001f)

        // A different initial pose must preserve a different local turning axis too.
        val xHeld=CubeQuaternion(s,0f,0f,s)
        val zTurn=CubeQuaternion(.5f,-.5f,.5f,.5f).relativeTo(xHeld)
        assertTrue(zTurn.closeTo(CubeQuaternion(0f,0f,s,s)))
        val rotatedRight=listOf(1-2*(zTurn.y*zTurn.y+zTurn.z*zTurn.z),2*(zTurn.x*zTurn.y+zTurn.w*zTurn.z),2*(zTurn.x*zTurn.z-zTurn.w*zTurn.y))
        for(i in 0..2)assertEquals(listOf(0f,1f,0f)[i],rotatedRight[i],.00001f)
    }
    @Test fun gyroRecentersWithoutChangingPhysicalAxesOrQuaternionSign(){
        val s=sqrt(.5f)
        val recentered=CubeQuaternion(0f,0f,s,s)
        assertTrue(recentered.relativeTo(recentered).closeTo(CubeQuaternion()))
        val next=CubeQuaternion(.5f,.5f,.5f,.5f)
        val antipodal=CubeQuaternion(-.5f,-.5f,-.5f,-.5f)
        assertTrue(next.relativeTo(recentered).closeTo(CubeQuaternion(s,0f,0f,s)))
        assertTrue(antipodal.relativeTo(recentered).closeTo(next.relativeTo(recentered)))
    }
    @Test fun cubiesAreClosedWithoutDegenerateTriangles(){
        val vertices=CubeMesh.vertices(CubeRenderFrame(Cube.solved,yaw=0f,pitch=0f))
        assertTrue(vertices.all{it.isFinite()})
        fun point(i:Int)=(0..2).map{(vertices[i+it]*100000).roundToInt()}.joinToString(",")
        val edges=mutableMapOf<String,Int>()
        for(start in vertices.indices step CubeMesh.STRIDE*3){
            val points=listOf(point(start),point(start+CubeMesh.STRIDE),point(start+CubeMesh.STRIDE*2))
            assertEquals(3,points.toSet().size)
            for(i in 0..2){val edge=listOf(points[i],points[(i+1)%3]).sorted().joinToString("|");edges[edge]=(edges[edge]?:0)+1}
        }
        assertTrue(edges.values.all{it==2},"Every rounded-cubie edge must be shared by exactly two triangles")
        System.getenv("CUBE_LAB_RENDERER_PREVIEW")?.let{directory->
            File(directory).mkdirs()
            val frames=mapOf("home" to CubeRenderFrame(Cube.solved),"turn" to CubeRenderFrame(Cube.solved,move="R",angle=-.785398f),"shallow" to CubeRenderFrame(Cube.solved,pitch=.06f),"top" to CubeRenderFrame(Cube.solved,pitch=1.22f))
            for((name,frame)in frames)ImageIO.write(CubeRaster.image(CubeMesh.vertices(frame),640,640),"png",File(directory,"kotlin-cube-$name.png"))
        }
    }
    @Test fun nearerSurfaceWinsIndependentOfDrawOrder(){
        fun triangle(z:Float,r:Float,b:Float)=floatArrayOf(
            -1f,-1f,z,0f,0f,1f,0f,0f,.496f,r,0f,b,
            1f,-1f,z,0f,0f,1f,0f,0f,.496f,r,0f,b,
            0f,1f,z,0f,0f,1f,0f,0f,.496f,r,0f,b)
        val near=triangle(1f,1f,0f);val far=triangle(-1f,0f,1f)
        val a=CubeRaster.image(near+far,100,100);val b=CubeRaster.image(far+near,100,100)
        val pixel=a.getRGB(50,50)
        assertTrue((pixel shr 16 and 255)>200&&(pixel and 255)<100,"The nearer red surface must obscure the blue surface")
        assertEquals(a.getRGB(50,50),b.getRGB(50,50))
        assertEquals(0,a.getRGB(0,0),"Unused pixels remain transparent")
    }
    @Test fun wholeCubeFitsViewportThroughTurnsAndOrbit(){
        for(yaw in listOf(-2.4f,-.58f,0f,1.7f))for(pitch in listOf(-1.3f,.06f,.48f,1.3f)){
            val vertices=CubeMesh.vertices(CubeRenderFrame(Cube.solved,move="R",angle=-.785398f,yaw=yaw,pitch=pitch))
            for(i in vertices.indices step CubeMesh.STRIDE){
                val factor=CubeMesh.PROJECTION_SCALE*CubeMesh.CAMERA_DISTANCE/(CubeMesh.CAMERA_DISTANCE-vertices[i+2])
                assertTrue(abs(vertices[i]*factor)<.5f&&abs(vertices[i+1]*factor)<.5f,"Turning pieces must stay inside the drawing surface")
            }
        }
    }
    @Test fun assembledProfileRoundsOnlyOutsideEdgesAndPreservesInternalPlanes(){
        val face=floatArrayOf(1.496f,.4f,-.4f)
        assertContentEquals(face,CubeMesh.outerProfile(face))
        val center=floatArrayOf(.496f,.3f,-.2f)
        assertContentEquals(center,CubeMesh.outerProfile(center))
        val corner=CubeMesh.outerProfile(floatArrayOf(1.496f,1.496f,1.496f))
        assertTrue(corner.all{it>1.196f&&it<1.4f},"The whole silhouette should round more than an individual cubie bevel")
        val internal=CubeMesh.outerProfile(floatArrayOf(.496f,1.496f,1.496f))
        assertEquals(.496f,internal[0],.000001f,"Internal layer planes must remain flat")
        val near=CubeMesh.outerProfile(floatArrayOf(1.48f,1.48f,1.48f))
        assertTrue((0..2).all{near[it]<corner[it]},"Profile deformation must preserve distinct surfaces")
    }
    @Test fun undersideMaterialIsDarkerWithoutChangingItsFaceColor(){
        val top=floatArrayOf(0f,0f,0f,0f,1f,0f,0f,0f,.496f,1f,.847f,.118f)
        val bottom=top.copyOf().apply{this[4]=-1f}
        val a=CubeMaterial.argb(top);val b=CubeMaterial.argb(bottom)
        assertTrue((b shr 16 and 255)<(a shr 16 and 255))
        assertTrue((b shr 8 and 255)>(b and 255),"Yellow remains yellow on the underside")
    }
}
