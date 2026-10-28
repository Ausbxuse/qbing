// Reproducible native assets from the approved web renderer, fonts and solvers.
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
const root=path.resolve(new URL('..',import.meta.url).pathname);
const read=p=>readFile(path.join(root,p),'utf8');
const prelude='if(!Array.prototype.at)Object.defineProperty(Array.prototype,"at",{value:function(i){i=Math.trunc(i)||0;return this[i<0?this.length+i:i];}});var DEBUG=false, ISCSTIMER=false; var $={isArray:Array.isArray,noop:function(){}}; var console={log:function(){}};\n';
const reviewSource=await read('src/js/local-review-tools.js'),reconsSource=await read('src/js/stats/recons.js');
const reviewFunctions=reviewSource.slice(reviewSource.indexOf(' function tokens('),reviewSource.indexOf(' function create('))+reviewSource.slice(reviewSource.indexOf('  function goal('),reviewSource.indexOf('  function catalog()'));
const counter=reconsSource.slice(reconsSource.indexOf('\tfunction MoveCounterHTM'),reconsSource.indexOf('\tfunction MoveCounterLFMC'))+reconsSource.slice(reconsSource.indexOf('\tfunction getMoveCnt('),reconsSource.indexOf('\tfunction calcRecons('));
const catalog=await read('src/assets/review-algorithms.json');
const f2l=JSON.parse(await read('native/shared/src/commonMain/composeResources/files/catalog.json')).cases.filter(c=>c.stage==='F2L').map(c=>({id:c.id,name:c.name,variants:c.variants.map(v=>v.text)}));
const bundle=prelude+(await Promise.all(['src/js/lib/isaac.js','src/js/lib/mathlib.js','src/js/lib/min2phase.js','src/js/lib/cubeutil.js','src/js/tools/cross.js'].map(read))).join('\n')+'\n'+counter+'\nvar recons={getMoveCnt:getMoveCnt};\n'+reviewFunctions+'\nvar nativeCatalog='+catalog+';\nvar nativeF2LCatalog='+JSON.stringify(f2l)+';\n'+await read('native/engine/review.js')+'\n'+await read('native/engine/bridge.js');
await mkdir(path.join(root,'native/assets'),{recursive:true});
await writeFile(path.join(root,'native/assets/solver.js'),bundle);
await copyFile(path.join(root,'native/assets/solver.js'),path.join(root,'native/shared/src/commonMain/composeResources/files/solver.js'));
if(process.env.NATIVE_SOLVER_ONLY)process.exit(0);
// The exact molded-cap and core mesh is exported, not redrawn by eye.
const THREE=await import(pathToFileURL(path.join(root,'src/vendor/three/three.module.min.js')));
const geomSource=(await read('src/js/local-cube-geometry.js')).replace("from 'three'",`from '${pathToFileURL(path.join(root,'src/vendor/three/three.module.min.js'))}'`);
const {moldedCap}=await import('data:text/javascript;base64,'+Buffer.from(geomSource).toString('base64'));
const roundSource=(await read('src/vendor/three/RoundedBoxGeometry.js')).replace(/from ['"]three['"]/,`from '${pathToFileURL(path.join(root,'src/vendor/three/three.module.min.js'))}'`);
const {RoundedBoxGeometry}=await import('data:text/javascript;base64,'+Buffer.from(roundSource).toString('base64'));
const cap=moldedCap([.17,.17,.17,.17],[.448,.448,.448,.448],{crown:.006}), core=new RoundedBoxGeometry(.93,.93,.93,2,.14);
const pack=g=>({positions:Array.from(g.attributes.position.array),normals:Array.from(g.attributes.normal.array),indices:g.index?Array.from(g.index.array):Array.from({length:g.attributes.position.count},(_,i)=>i),groups:g.groups});
const mesh={cap:pack(cap),core:pack(core),palette:[0xf5f7fc,0xf44e65,0x28c992,0xffda43,0xff9846,0x4599f4],camera:[6.2,4.8,9.1],fov:25};
for(const destination of ['native/assets/cube-mesh.json','native/shared/src/commonMain/composeResources/files/cube-mesh.json'])await writeFile(path.join(root,destination),JSON.stringify(mesh));
console.log('Native assets: upstream headless solver and exact approved cube geometry.');
// Static indexed scene: transformed entirely on GPU; zero per-frame mesh uploads.
import {gzipSync} from 'node:zlib';
const values=[],indices=[];
function append(g,matrix,home,face,start=0,count=g.index?.count||g.attributes.position.count){
 const offset=values.length/10, normal=new THREE.Matrix3().getNormalMatrix(matrix);
 for(let i=0;i<g.attributes.position.count;i++){
  const pos=new THREE.Vector3().fromBufferAttribute(g.attributes.position,i).applyMatrix4(matrix),n=new THREE.Vector3().fromBufferAttribute(g.attributes.normal,i).applyMatrix3(normal).normalize();
  values.push(...pos.toArray(),...n.toArray(),...home,face);
 }
 for(let j=start;j<start+count;j++)indices.push(offset+(g.index?g.index.getX(j):j));
}
for(let x=-1;x<=1;x++)for(let y=-1;y<=1;y++)for(let z=-1;z<=1;z++)if(x||y||z)append(core,new THREE.Matrix4().makeTranslation(x,y,z),[x,y,z],-1);
const normals=[[0,1,0],[1,0,0],[0,0,1],[0,-1,0],[-1,0,0],[0,0,-1]];
let logoPath=(await read('src/assets/gan-center.svg')).match(/ d="([^"]+)"/)[1];
const commands=logoPath.match(/[MLVHZmlvhz][^MLVHZmlvhz]*/g),polygons=[];let polygon=[],lx=0,ly=0;
for(const command of commands){const op=command[0],ns=command.slice(1).trim().split(/[\s,]+/).filter(Boolean).map(Number);if(op==='M'){if(polygon.length)polygons.push(polygon);polygon=[];lx=ns[0];ly=ns[1];}else if(op==='L'){lx=ns[0];ly=ns[1];}else if(op==='V')ly=ns[0];else if(op==='H')lx=ns[0];else if(op==='Z'){if(polygon.length)polygons.push(polygon);polygon=[];continue;}else throw Error('Unhandled brand path');polygon.push(new THREE.Vector2((lx/28.3063-.5)*.5,(.5-ly/32.002)*.566));}
const logo=new THREE.BufferGeometry(),lp=[],ln=[],li=[];
for(const poly of polygons){const off=lp.length/3;for(const p of poly){lp.push(p.x,p.y,.028+ .006*(1-(p.x/.448)**2)**2*(1-(p.y/.448)**2)**2+.0009);ln.push(0,0,1);}for(const tri of THREE.ShapeUtils.triangulateShape(poly,[]))li.push(...tri.map(i=>i+off));}
logo.setAttribute('position',new THREE.Float32BufferAttribute(lp,3));logo.setAttribute('normal',new THREE.Float32BufferAttribute(ln,3));logo.setIndex(li);
for(let f=0;f<6;f++)for(let row=0;row<3;row++)for(let col=0;col<3;col++){
 const homes=[[col-1,1,row-1],[1,1-row,1-col],[col-1,1-row,1],[col-1,-1,1-row],[-1,1-row,col-1],[1-col,1-row,-1]],home=homes[f],n=new THREE.Vector3(...normals[f]);
 const matrix=new THREE.Matrix4().compose(new THREE.Vector3(...home).add(n.clone().multiplyScalar(.472)),new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0,0,1),n),new THREE.Vector3(1,1,1));
 append(cap,matrix,home,f*9+row*3+col,0,cap.groups[0].count);append(cap,matrix,home,-1,cap.groups[1].start,cap.groups[1].count);
 if(f===0&&row===1&&col===1)append(logo,matrix,home,-2);
}
const header=Buffer.alloc(8);header.writeUInt32LE(values.length/10,0);header.writeUInt32LE(indices.length,4);
const data=Buffer.concat([header,Buffer.from(new Float32Array(values).buffer),Buffer.from(new Uint32Array(indices).buffer)]),compressed=gzipSync(data,{level:9});
for(const target of ['native/assets/cube-scene.bin.gz','native/shared/src/commonMain/composeResources/files/cube-scene.mesh'])await writeFile(path.join(root,target),compressed);
console.log(`Static native scene: ${indices.length/3} triangles, ${(compressed.length/1024).toFixed(0)} KB packaged; one draw, no mesh uploads during turns.`);

for(const file of ["cube.vert","cube.frag"])await copyFile(path.join(root,"native/assets",file),path.join(root,"native/shared/src/commonMain/composeResources/files",file));
