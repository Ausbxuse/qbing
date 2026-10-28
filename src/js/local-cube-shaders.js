// Surface details stay in each cap's local space, including during a turn.
// Three's physical BRDF still handles lighting, reflections and clearcoat.
export function moldedPlastic(material) {
  material.onBeforeCompile = shader => {
    const varyings = `
      varying vec3 vPlasticPosition;
      varying float vPlasticFront;
    `;
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\n' + varyings)
      .replace('#include <beginnormal_vertex>', `
        #include <beginnormal_vertex>
        vPlasticFront = smoothstep(0.65, 0.99, objectNormal.z);
      `)
      .replace('#include <begin_vertex>', `
        #include <begin_vertex>
        vPlasticPosition = position;
      `);
    shader.fragmentShader = shader.fragmentShader
      .replace('#include <tonemapping_fragment>', `
        #ifdef TONE_MAPPING
          vec3 exposed = gl_FragColor.rgb * toneMappingExposure;
          float peak = max(exposed.r, max(exposed.g, exposed.b));
          if (peak > 0.82) {
            float shoulder = 0.82 + 0.18 * (1.0 - exp(-(peak - 0.82) / 0.18));
            exposed *= shoulder / peak;
          }
          gl_FragColor.rgb = exposed;
        #endif
      `)
      .replace('#include <common>', `
        #include <common>
        ${varyings}
        float plasticHash(vec2 p) {
          vec3 q = fract(vec3(p.xyx) * 0.1031);
          q += dot(q, q.yzx + 33.33);
          return fract((q.x + q.y) * q.z);
        }
        float plasticNoise(vec2 p) {
          vec2 cell = floor(p), f = fract(p);
          f = f * f * (3.0 - 2.0 * f);
          return mix(mix(plasticHash(cell), plasticHash(cell + vec2(1.0, 0.0)), f.x),
                     mix(plasticHash(cell + vec2(0.0, 1.0)), plasticHash(cell + vec2(1.0)), f.x), f.y);
        }
      `)
      .replace('#include <roughnessmap_fragment>', `
        #include <roughnessmap_fragment>
        vec2 plasticPoint = vPlasticPosition.xy;
        // Fade microscopic detail below pixel size instead of letting it
        // sparkle or crawl on the small timer cube.
        float footprint = max(length(dFdx(plasticPoint)), length(dFdy(plasticPoint))) * 72.0;
        float resolved = 1.0 - smoothstep(0.35, 1.1, footprint);
        float grain = (plasticNoise(plasticPoint * 72.0) - 0.5) * resolved;
        float molding = plasticNoise(plasticPoint * 4.0) - 0.5;
        roughnessFactor = clamp(roughnessFactor + vPlasticFront * (grain * 0.025 + molding * 0.012), 0.16, 0.5);
      `);

  };
  material.customProgramCacheKey = () => 'cube-design-study-v1';
  return material;
}
