in vec3 position;
in vec3 normal;
in vec3 home;
in float face;
uniform vec4 turns[2]; // axis, layer, signed radians, enabled
uniform vec4 orientation;
uniform float logoAngle;
uniform vec2 orbit;
uniform vec2 aspect;
uniform int stickers[54];
out vec3 tint;
out vec3 worldNormal;
flat out int material;
vec3 rotateAxis(vec3 v,int axis,float angle){float c=cos(angle),s=sin(angle);if(axis==0)return vec3(v.x,c*v.y-s*v.z,s*v.y+c*v.z);if(axis==1)return vec3(c*v.x+s*v.z,v.y,-s*v.x+c*v.z);return vec3(c*v.x-s*v.y,s*v.x+c*v.y,v.z);}
vec3 quaternion(vec3 v){vec3 t=2.0*cross(orientation.xyz,v);return v+orientation.w*t+cross(orientation.xyz,t);}
vec3 color(int i){if(i==0)return vec3(245.,247.,252.)/255.;if(i==1)return vec3(244.,78.,101.)/255.;if(i==2)return vec3(40.,201.,146.)/255.;if(i==3)return vec3(255.,218.,67.)/255.;if(i==4)return vec3(255.,152.,70.)/255.;return vec3(69.,153.,244.)/255.;}
void main(){
 vec3 p=position,n=normal;
 if(face< -1.5)p=rotateAxis(p,1,logoAngle);
 for(int j=0;j<2;j++){int a=int(turns[j].x);if(turns[j].w>.5&&abs(home[a]-turns[j].y)<.1){p=rotateAxis(p,a,turns[j].z);n=rotateAxis(n,a,turns[j].z);}}
 worldNormal=quaternion(n);
 p=rotateAxis(rotateAxis(quaternion(p),1,orbit.x),0,orbit.y);
 float distance=length(vec3(6.2,4.8,9.1));float w=distance-p.z;
 gl_Position=vec4(p.x*4.5107085*aspect.x,p.y*4.5107085*aspect.y,((100.+.1)/(100.-.1))*w-2.*100.*.1/(100.-.1),w);
 material=int(face);
 tint=face<-.5?vec3(48.,56.,68.)/255.:color(stickers[int(face)]);
}
