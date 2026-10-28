in vec3 tint;
in vec3 worldNormal;
flat in int material;
out vec4 fragColor;
vec3 linearColor(vec3 c){return mix(c/12.92,pow((c+.055)/1.055,vec3(2.4)),step(vec3(.04045),c));}
vec3 srgbColor(vec3 c){return mix(c*12.92,1.055*pow(c,vec3(1./2.4))-.055,step(vec3(.0031308),c));}
float ramp(vec3 n,vec3 direction){float d=dot(n,normalize(direction))*.5+.5;return d<1./3.?125./255.:d<2./3.?185./255.:235./255.;}
void main(){if(material==-2){fragColor=vec4(vec3(6.,77.,196.)/255.,1.);return;}vec3 n=normalize(worldNormal);float light=(.98+1.30*ramp(n,vec3(4.,6.,7.))+.25*ramp(n,vec3(-4.,1.,2.)))/3.14159265359;fragColor=vec4(srgbColor(linearColor(tint)*light),1.);}
