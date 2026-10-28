// Presentation only: incoming facelets and timing remain upstream-authoritative.
import * as THREE from 'three';
import {RoundedBoxGeometry} from '../vendor/three/RoundedBoxGeometry.js';
import {moldedCap, capCrownHeight} from './local-cube-geometry.js';
import './local-cube-motion.js';

const canvas=document.querySelector('#cube-canvas'), stage=document.querySelector('.cube-stage');
const renderer=new THREE.WebGLRenderer({canvas,antialias:true,alpha:true,powerPreference:'low-power'});
renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.NoToneMapping;
const scene=new THREE.Scene(), camera=new THREE.PerspectiveCamera(25,1,.1,100);
camera.position.set(6.2,4.8,9.1);camera.lookAt(0,0,0);
scene.add(new THREE.AmbientLight(0xffffff,.9));
const key=new THREE.DirectionalLight(0xffffff,1.25);key.position.set(4,6,7);scene.add(key);
const fill=new THREE.DirectionalLight(0xffffff,.2);fill.position.set(-4,1,2);scene.add(fill);
const ramp=new THREE.DataTexture(new Uint8Array([125,185,235]),3,1,THREE.RedFormat);
ramp.minFilter=ramp.magFilter=THREE.NearestFilter;ramp.generateMipmaps=false;ramp.needsUpdate=true;
const shellMaterial=new THREE.MeshToonMaterial({color:0xffffff,gradientMap:ramp});
const plastic=new THREE.MeshToonMaterial({color:0x303844,gradientMap:ramp});
const colors={U:0xf1f0e8,R:0xe65c66,F:0x57ba8a,D:0xefcb59,L:0xee9c54,B:0x6299df};
for(const face of Object.keys(colors))colors[face]=new THREE.Color(colors[face]);
canvas.dataset.environment='flat';canvas.dataset.shading='toon';
const cube=new THREE.Group();scene.add(cube);
const pieces=[],pieceAt=new Map(),surfaces=[],marks=[];
for(let x=-1;x<=1;x++)for(let y=-1;y<=1;y++)for(let z=-1;z<=1;z++){
 if(!x&&!y&&!z)continue;
 const part=new THREE.Object3D();part.position.set(x,y,z);part.userData.home=part.position.clone();cube.add(part);pieces.push(part);pieceAt.set([x,y,z].join(','),part);
}
const coreGeometry=new RoundedBoxGeometry(.93,.93,.93,2,.14);
const crown=.006, bounds=[.448,.448,.448,.448], shellGeometry=moldedCap([.17,.17,.17,.17],bounds,{crown});
// Share vertex/index buffers; front colors and charcoal shoulders are separate
// instance batches so the latter never inherit a face's tint.
function geometryRange(start,count){
 const g=new THREE.BufferGeometry();for(const name of Object.keys(shellGeometry.attributes))g.setAttribute(name,shellGeometry.attributes[name]);
 g.setIndex(shellGeometry.index);g.setDrawRange(start,count);return g;
}
const frontCount=shellGeometry.groups[0].count;
const frontGeometry=geometryRange(0,frontCount), shoulderGeometry=geometryRange(frontCount,shellGeometry.index.count-frontCount);
const front=new THREE.InstancedMesh(frontGeometry,shellMaterial,54);
const shoulder=new THREE.InstancedMesh(shoulderGeometry,plastic,54);
const cores=new THREE.InstancedMesh(coreGeometry,plastic,26);
const ink=new THREE.TextureLoader().load(new URL('../assets/gan-center.svg',import.meta.url).href,()=>{canvas.dataset.centerMark='ready';invalidate();},undefined,()=>{canvas.dataset.centerMark='failed';});
ink.colorSpace=THREE.SRGBColorSpace;ink.anisotropy=Math.min(4,renderer.capabilities.getMaxAnisotropy());
const inkMaterial=new THREE.MeshBasicMaterial({map:ink,transparent:true,alphaTest:.05,depthWrite:false,polygonOffset:true,polygonOffsetFactor:-1,polygonOffsetUnits:-1});
const markGeometry=new THREE.PlaneGeometry(.50,.566,4,4), positions=markGeometry.attributes.position;
for(let i=0;i<positions.count;i++)positions.setZ(i,.028+capCrownHeight(positions.getX(i),positions.getY(i),bounds,crown)+.0006);
markGeometry.computeVertexNormals();
const logos=new THREE.InstancedMesh(markGeometry,inkMaterial,6);
for(const mesh of [cores,front,shoulder,logos]){mesh.frustumCulled=false;mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);cube.add(mesh);}
const normals=[new THREE.Vector3(0,1,0),new THREE.Vector3(1,0,0),new THREE.Vector3(0,0,1),new THREE.Vector3(0,-1,0),new THREE.Vector3(-1,0,0),new THREE.Vector3(0,0,-1)];
const forward=new THREE.Vector3(0,0,1);
for(let face=0;face<6;face++)for(let row=0;row<3;row++)for(let col=0;col<3;col++){
 const locations=[[col-1,1,row-1],[1,1-row,1-col],[col-1,1-row,1],[col-1,-1,1-row],[-1,1-row,col-1],[1-col,1-row,-1]];
 const surface=new THREE.Object3D();surface.quaternion.setFromUnitVectors(forward,normals[face]);surface.position.copy(normals[face]).multiplyScalar(.472);pieceAt.get(locations[face].join(',')).add(surface);surfaces.push(surface);
 if(row===1&&col===1){const mark=new THREE.Object3D();surface.add(mark);marks.push({mark,index:face*9+4});}
}
const solved='UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB';
function apply(state,move){
 if(!window.mathlib)return null;
 const c=new mathlib.CubieCube();if(c.fromFacelet(state)===-1)return null;c.selfMoveStr(move);return c.toFaceCube();
}
const motion=new window.LocalCubeMotion.Motion(solved,apply), reduced=matchMedia('(prefers-reduced-motion: reduce)');
let painted=solved, colorsDirty=true, matricesDirty=true, sizeDirty=true, needsDraw=true;
let frameId=0, turn=null, turnId=0, turnMembers=[], contextLost=false, suspended=false, intersects=true, visible=true;
let width=0,height=0,pixelRatio=0,pointer;
const inverse=new THREE.Matrix4(),matrix=new THREE.Matrix4(),zero=new THREE.Matrix4().makeScale(0,0,0);
const profile={frames:0,drawCalls:0,triangles:0,lastFrameMs:0};
function paint(state){if(painted!==state){painted=state;colorsDirty=true;}canvas.dataset.facelets=state;}
function spin(move,turns){const m=/^([URFDLB])([2']?)$/.exec(move);if(m){const mark=marks['URFDLB'.indexOf(m[1])].mark;mark.rotation.z=(mark.rotation.z-Math.PI/2*(turns||(m[2]==='2'?2:m[2]==="'"?-1:1)))%(Math.PI*2);}matricesDirty=true;}
function cancelTurn(){
 if(!turn)return;
 for(const part of turnMembers){turn.remove(part);cube.add(part);part.position.copy(part.userData.home);part.quaternion.identity();}
 cube.remove(turn);turn=null;turnMembers=[];turnId=0;matricesDirty=true;
}
function settle(){
 const steps=(motion.active?[motion.active]:[]).concat(motion.queue);cancelTurn();steps.forEach(step=>spin(step.move,step.turns));motion.snap(motion.target);paint(motion.display);canvas.dataset.animating='false';needsDraw=true;
}
function upload(){
 if(colorsDirty){for(let i=0;i<54;i++)front.setColorAt(i,colors[painted[i]]);front.instanceColor.needsUpdate=true;colorsDirty=false;matricesDirty=true;}
 if(!matricesDirty)return;
 cube.updateMatrixWorld(true);inverse.copy(cube.matrixWorld).invert();
 for(let i=0;i<26;i++)cores.setMatrixAt(i,matrix.multiplyMatrices(inverse,pieces[i].matrixWorld));
 for(let i=0;i<54;i++){matrix.multiplyMatrices(inverse,surfaces[i].matrixWorld);front.setMatrixAt(i,matrix);shoulder.setMatrixAt(i,matrix);}
 for(let i=0;i<6;i++)logos.setMatrixAt(i,painted[marks[i].index]==='U'?matrix.multiplyMatrices(inverse,marks[i].mark.matrixWorld):zero);
 for(const mesh of [cores,front,shoulder,logos])mesh.instanceMatrix.needsUpdate=true;
 matricesDirty=false;
}
function invalidate(){needsDraw=true;if(!frameId&&!suspended&&!document.hidden&&!contextLost)frameId=requestAnimationFrame(frame);}
function measure(){
 sizeDirty=false;const box=stage.getBoundingClientRect();visible=box.width>0&&box.height>0&&intersects;
 if(!visible)return;
 const ratio=Math.min(devicePixelRatio||1,2);
 if(box.width!==width||box.height!==height||ratio!==pixelRatio){
  width=box.width;height=box.height;pixelRatio=ratio;
  renderer.setPixelRatio(ratio);renderer.setSize(width,height,false);camera.aspect=width/height;camera.updateProjectionMatrix();needsDraw=true;
 }
}
function frame(now){
 frameId=0;if(document.hidden||suspended||contextLost){settle();return;}
 if(sizeDirty)measure();if(!visible){settle();return;}
 const step=motion.frame(now);
 for(const done of step.completed){cancelTurn();spin(done.move,done.turns);paint(done.state);needsDraw=true;}
 if(step.active){
  if(turnId!==step.active.id){
   cancelTurn();const axis=normals['URFDLB'.indexOf(step.active.move[0])];turnMembers=pieces.filter(p=>p.userData.home.dot(axis)>.9);turn=new THREE.Group();cube.add(turn);
   for(const part of turnMembers){cube.remove(part);turn.add(part);}turnId=step.active.id;
  }
  const move=step.active.move,axis=normals['URFDLB'.indexOf(move[0])],angle=-Math.PI/2*step.active.turns;
  const p=step.progress;turn.quaternion.setFromAxisAngle(axis,angle*p*p*(3-2*p));matricesDirty=true;needsDraw=true;
 }
 canvas.dataset.animating=motion.busy()?'true':'false';
 if(needsDraw){const start=performance.now();upload();renderer.render(scene,camera);profile.frames++;profile.drawCalls=renderer.info.render.calls;profile.triangles=renderer.info.render.triangles;profile.lastFrameMs=performance.now()-start;needsDraw=false;}
 if(motion.busy())invalidate();
}
function updateState(detail=window.localCubeState){
 if(!detail||!/^[URFDLB]{54}$/.test(detail.facelets||''))return;
 const before=motion.revision,oldSteps=(motion.active?[motion.active]:[]).concat(motion.queue).map(step=>({move:step.move,turns:step.turns}));
 const animate=visible&&intersects&&!document.hidden&&!suspended&&!contextLost&&!reduced.matches;
 if(!motion.accept(detail.facelets,detail.moves,performance.now(),animate))return;
 canvas.dataset.targetFacelets=motion.target;
 if(motion.revision!==before){cancelTurn();oldSteps.concat(motion.lastSteps).forEach(step=>spin(step.move,step.turns));paint(motion.display);}
 canvas.dataset.animating=motion.busy()?'true':'false';needsDraw=true;if(visible&&intersects&&!document.hidden&&!suspended&&!contextLost)invalidate();
}
canvas.addEventListener('pointerdown',event=>{pointer={x:event.clientX,y:event.clientY};canvas.setPointerCapture(event.pointerId);});
canvas.addEventListener('pointermove',event=>{if(!pointer)return;cube.rotation.y+=(event.clientX-pointer.x)*.008;cube.rotation.x=Math.max(-.8,Math.min(.8,cube.rotation.x+(event.clientY-pointer.y)*.008));pointer={x:event.clientX,y:event.clientY};invalidate();});
function release(){pointer=null;}
for(const name of ['pointerup','pointercancel','lostpointercapture'])canvas.addEventListener(name,release);
document.querySelector('#cube-reset').addEventListener('click',()=>{cube.rotation.set(0,0,0);invalidate();});
new ResizeObserver(()=>{sizeDirty=true;invalidate();}).observe(stage);
if(window.IntersectionObserver)new IntersectionObserver(entries=>{intersects=entries[0].isIntersecting;sizeDirty=true;if(!intersects){release();settle();cancelAnimationFrame(frameId);frameId=0;}else invalidate();}).observe(stage);
window.addEventListener('local-cube-state',event=>updateState(event.detail));
window.addEventListener('local-cube-redraw',()=>{sizeDirty=true;invalidate();});
window.addEventListener('resize',()=>{sizeDirty=true;invalidate();});
window.addEventListener('focus',()=>{sizeDirty=true;invalidate();});
document.addEventListener('visibilitychange',()=>{if(document.hidden){release();settle();cancelAnimationFrame(frameId);frameId=0;}else{intersects=true;sizeDirty=true;invalidate();}});
reduced.addEventListener('change',()=>{if(reduced.matches)settle();invalidate();});
canvas.addEventListener('webglcontextlost',event=>{event.preventDefault();contextLost=true;release();settle();cancelAnimationFrame(frameId);frameId=0;});
canvas.addEventListener('webglcontextrestored',()=>{contextLost=false;matricesDirty=colorsDirty=sizeDirty=true;invalidate();});
window.addEventListener('pagehide',()=>{suspended=true;release();settle();cancelAnimationFrame(frameId);frameId=0;});
window.addEventListener('pageshow',()=>{suspended=false;sizeDirty=true;invalidate();});
window.localCubeView=Object.freeze({stats:()=>({...profile,pixelRatio,visible:visible&&intersects&&!document.hidden&&!suspended&&!contextLost,queue:motion.queue.length,animating:motion.busy(),activeId:motion.active?motion.active.id:null})});
canvas.dataset.centerMark='loading';canvas.dataset.animating='false';paint(solved);updateState();invalidate();canvas.dataset.ready='true';
