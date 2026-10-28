import * as THREE from 'three';

export function capCrownHeight(x, y, bounds, crown = .012) {
  const [left, right, bottom, top] = bounds;
  const u = (x - (right - left) / 2) / ((left + right) / 2);
  const v = (y - (top - bottom) / 2) / ((top + bottom) / 2);
  return crown * (1 - u * u) ** 2 * (1 - v * v) ** 2;
}

// A closed molded cap: curved face, rolled shoulder, skirt and recessed back.
// Indexed rings share vertices so normals remain continuous across the face
// and bevel. The curvature also participates in real shadows and silhouettes.
export function moldedCap(radii, bounds, { crown = .012 } = {}) {
  const [bl, br, tr, tl] = radii;
  const [left, right, bottom, top] = bounds;
  const shape = new THREE.Shape();
  shape.moveTo(-left + bl, -bottom);
  shape.lineTo(right - br, -bottom); shape.quadraticCurveTo(right, -bottom, right, -bottom + br);
  shape.lineTo(right, top - tr); shape.quadraticCurveTo(right, top, right - tr, top);
  shape.lineTo(-left + tl, top); shape.quadraticCurveTo(-left, top, -left, top - tl);
  shape.lineTo(-left, -bottom + bl); shape.quadraticCurveTo(-left, -bottom, -left + bl, -bottom);

  const coarse = shape.getPoints(8), boundary = [];
  for (let i = 0; i < coarse.length - 1; i++) {
    const a = coarse[i], b = coarse[i + 1];
    const steps = Math.max(1, Math.ceil(a.distanceTo(b) / .14));
    for (let j = 0; j < steps; j++) boundary.push(a.clone().lerp(b, j / steps));
  }
  const count = boundary.length, cx = (right - left) / 2, cy = (top - bottom) / 2;
  const shoulder = .024;
  const crownHeight = (x, y) => capCrownHeight(x, y, bounds, crown);
  const positions = [cx, cy, .028 + crown], indices = [];
  const outward = boundary.map((point, i) => {
    const before = boundary[(i + count - 1) % count], after = boundary[(i + 1) % count];
    return new THREE.Vector2(after.y - before.y, before.x - after.x).normalize();
  });
  let previous;
  const ring = vertices => {
    const start = positions.length / 3;
    for (const vertex of vertices) positions.push(...vertex);
    for (let j = 0; j < count; j++) {
      const next = (j + 1) % count;
      if (previous == null) indices.push(0, start + j, start + next);
      else indices.push(previous + j, start + j, start + next, previous + j, start + next, previous + next);
    }
    previous = start;
  };
  const faceRings = 3;
  for (let i = 1; i <= faceRings; i++) {
    const r = i / faceRings;
    ring(boundary.map(p => {
      const x = cx + (p.x - cx) * r, y = cy + (p.y - cy) * r;
      return [x, y, .028 + crownHeight(x, y)];
    }));
  }
  const frontIndexCount = indices.length;
  for (let i = 1; i <= 4; i++) {
    const angle = i / 4 * Math.PI / 2;
    ring(boundary.map((p, j) => [p.x + outward[j].x * shoulder * Math.sin(angle),
      p.y + outward[j].y * shoulder * Math.sin(angle), .028 + crownHeight(p.x, p.y) - shoulder * (1 - Math.cos(angle))]));
  }
  ring(boundary.map((p, j) => [p.x + outward[j].x * shoulder, p.y + outward[j].y * shoulder, -.012]));
  const back = positions.length / 3;
  positions.push(cx, cy, -.012);
  for (let j = 0; j < count; j++) indices.push(back, previous + (j + 1) % count, previous + j);

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  // Separate the colored face from its shoulder/skirt for illustrated seams.
  geometry.addGroup(0, frontIndexCount, 0);
  geometry.addGroup(frontIndexCount, indices.length - frontIndexCount, 1);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}
