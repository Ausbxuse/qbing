//! One closed rounded solid per cubie. Color is part of the shell, not a sticker overlay.
use crate::cube::{Cube, Move, geometry, rotate};
use std::sync::OnceLock;

pub type V3 = [f32; 3];
pub type Quaternion = [f32; 4]; // x, y, z, w
pub const IDENTITY: Quaternion = [0., 0., 0., 1.];
// A longer lens keeps the near row from appearing oversized or stretched.
pub const CAMERA_DISTANCE: f32 = 16.;
pub const PROJECTION_SCALE: f32 = 0.195;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: V3,
    pub normal: V3,
    pub local: V3,
    // Unlit sRGB albedo. Both rasterizers shade in linear light, then encode once.
    pub color: V3,
}
#[derive(Clone, Copy)]
struct SurfaceVertex {
    position: V3,
    normal: V3,
    shell_normal: V3,
    crown: f32,
    face: usize,
}

pub fn dot(a: V3, b: V3) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn mul(a: V3, b: f32) -> V3 {
    a.map(|v| v * b)
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normal(a: V3) -> V3 {
    mul(a, 1. / dot(a, a).sqrt().max(1e-8))
}
pub fn normalize(q: Quaternion) -> Option<Quaternion> {
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    (length.is_finite() && length > 1e-6).then(|| q.map(|v| v / length))
}
pub fn conjugate(q: Quaternion) -> Quaternion {
    [-q[0], -q[1], -q[2], q[3]]
}
pub fn product(a: Quaternion, b: Quaternion) -> Quaternion {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}
pub fn orient(v: V3, q: Quaternion) -> V3 {
    let xyz = [q[0], q[1], q[2]];
    let t = mul(cross(xyz, v), 2.);
    add(v, add(mul(t, q[3]), cross(xyz, t)))
}
pub fn slerp(a: Quaternion, mut b: Quaternion, t: f32) -> Quaternion {
    let mut d = a.iter().zip(b).map(|(a, b)| a * b).sum::<f32>();
    if d < 0. {
        b = b.map(|v| -v);
        d = -d;
    }
    if d > 0.9995 {
        return normalize(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)).unwrap_or(a);
    }
    let angle = d.clamp(-1., 1.).acos();
    let (u, v) = (
        ((1. - t) * angle).sin() / angle.sin(),
        (t * angle).sin() / angle.sin(),
    );
    std::array::from_fn(|i| a[i] * u + b[i] * v)
}

const NORMALS: [V3; 6] = [
    [0., 1., 0.],
    [1., 0., 0.],
    [0., 0., 1.],
    [0., -1., 0.],
    [-1., 0., 0.],
    [0., 0., -1.],
];
fn shell() -> &'static [SurfaceVertex] {
    static SHELL: OnceLock<Vec<SurfaceVertex>> = OnceLock::new();
    SHELL.get_or_init(|| {
        let half = 0.496;
        let radius = 0.075;
        let inner = half - radius;
        let samples = [
            -half,
            -inner - radius * 0.7,
            -inner - radius * 0.3,
            -inner,
            -0.35,
            -0.20,
            0.,
            0.20,
            0.35,
            inner,
            inner + radius * 0.3,
            inner + radius * 0.7,
            half,
        ];
        let mut vertices = Vec::new();
        for (face, n) in NORMALS.iter().copied().enumerate() {
            let u = if n[0] != 0. {
                [0., 1., 0.]
            } else {
                [1., 0., 0.]
            };
            let v = cross(n, u);
            let point = |a: f32, b: f32| {
                let p = add(mul(n, half), add(mul(u, a), mul(v, b)));
                let center = p.map(|x| x.clamp(-inner, inner));
                let direction = normal(std::array::from_fn(|i| p[i] - center[i]));
                // The colored surface is part of the shell. A very shallow
                // crown catches a broad studio reflection like molded plastic;
                // it and its slope vanish at shared edges, so the solid stays closed.
                let x = a / half;
                let y = b / half;
                let sx = (1. - x * x).max(0.);
                let sy = (1. - y * y).max(0.);
                let crown = 0.004 * sx * sx * sy * sy;
                let da = -4. * 0.004 / half * x * sx * sy * sy;
                let db = -4. * 0.004 / half * y * sy * sx * sx;
                SurfaceVertex {
                    position: add(center, mul(direction, radius)),
                    normal: normal(add(add(direction, mul(u, -da)), mul(v, -db))),
                    shell_normal: direction,
                    crown,
                    face,
                }
            };
            for row in samples.windows(2) {
                for col in samples.windows(2) {
                    let a = point(col[0], row[0]);
                    let b = point(col[1], row[0]);
                    let c = point(col[1], row[1]);
                    let d = point(col[0], row[1]);
                    vertices.extend([a, b, c, a, c, d]);
                }
            }
        }
        vertices
    })
}

fn surface_position(vertex: &SurfaceVertex, crowned: bool) -> V3 {
    if crowned {
        add(vertex.position, mul(NORMALS[vertex.face], vertex.crown))
    } else {
        vertex.position
    }
}

// Only coordinates near the outside of the assembled cube are rounded. The
// interior slicing planes remain flat, so moving layers never overlap. Unlike
// projecting all outlying points onto one surface, this radial map is injective
// within every corner: it preserves the closed mechanical faces beneath it.
fn outer_profile(point: V3) -> V3 {
    const INNER: f32 = 1.196; // 0.30 radius around the assembled 1.496 halfextent.
    let center = point.map(|v| v.clamp(-INNER, INNER));
    let delta: V3 = std::array::from_fn(|i| point[i] - center[i]);
    let extent = delta.iter().map(|v| v.abs()).fold(0f32, f32::max);
    let length = dot(delta, delta).sqrt();
    if length < 1e-7 {
        point
    } else {
        add(center, mul(delta, extent / length))
    }
}
fn profile_normal(point: V3, n: V3) -> V3 {
    // Transform tangents, not the radial envelope normal. This also preserves
    // the orientation of the dark internal slicing faces at a rounded corner.
    let helper = if n[1].abs() < 0.9 {
        [0., 1., 0.]
    } else {
        [1., 0., 0.]
    };
    let u = normal(cross(helper, n));
    let v = cross(n, u);
    let differential = |t: V3| {
        let a = outer_profile(add(point, mul(t, 0.0005)));
        let b = outer_profile(add(point, mul(t, -0.0005)));
        std::array::from_fn(|i| a[i] - b[i])
    };
    normal(cross(differential(u), differential(v)))
}
#[derive(Clone, Copy)]
struct ProfileVertex {
    position: V3,
    normal: V3,
    local: V3,
    face: usize,
}
fn shaped_shell(index: usize) -> &'static [ProfileVertex] {
    static SHAPES: OnceLock<[Vec<ProfileVertex>; 27]> = OnceLock::new();
    &SHAPES.get_or_init(|| {
        std::array::from_fn(|index| {
            let center = [
                (index / 9) as f32 - 1.,
                ((index / 3) % 3) as f32 - 1.,
                (index % 3) as f32 - 1.,
            ];
            shell()
                .iter()
                .map(|vertex| {
                    let exterior = dot(center, NORMALS[vertex.face]) == 1.;
                    let point = add(center, surface_position(vertex, exterior));
                    let n = if exterior {
                        vertex.normal
                    } else {
                        vertex.shell_normal
                    };
                    ProfileVertex {
                        position: outer_profile(point),
                        normal: profile_normal(point, n),
                        local: vertex.position,
                        face: vertex.face,
                    }
                })
                .collect()
        })
    })[index]
}

fn shade(base: V3, n: V3, position: V3, local: V3) -> V3 {
    super::material::shade(base, n, position, local)
}

pub fn scene(
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<(Move, f32)>,
    orientation: Quaternion,
) -> Vec<Vertex> {
    scene_motion(cube, yaw, pitch, turn.map(super::Motion::from), orientation)
}

pub fn scene_motion(
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<super::Motion>,
    orientation: Quaternion,
) -> Vec<Vertex> {
    let view = |p| rotate(rotate(orient(p, orientation), 1, yaw), 0, pitch);
    let mut output = Vec::with_capacity(shell().len() * 27);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let p = [x as f32, y as f32, z as f32];
                let active = turn.and_then(|motion| {
                    [
                        Some((motion.first, motion.progress)),
                        motion.second.map(|m| (m, motion.partner_progress)),
                    ]
                    .into_iter()
                    .flatten()
                    .find(|(m, _)| {
                        let (axis, layer, _) = m.axis();
                        p[axis] == layer
                    })
                });
                let transform = |v| {
                    if let Some((m, t)) = active {
                        let (axis, _, direction) = m.axis();
                        rotate(
                            v,
                            axis,
                            direction * m.turns as f32 * std::f32::consts::FRAC_PI_2 * t,
                        )
                    } else {
                        v
                    }
                };
                let colored: [Option<eframe::egui::Color32>; 6] = std::array::from_fn(|face| {
                    geometry()
                        .iter()
                        .position(|g| g.p == p && g.n == NORMALS[face])
                        .map(|i| super::face_color(cube.0[i]))
                });
                let colors: [V3; 6] = std::array::from_fn(|face| {
                    let color =
                        colored[face].unwrap_or(eframe::egui::Color32::from_rgb(19, 22, 29));
                    [
                        color.r() as f32 / 255.,
                        color.g() as f32 / 255.,
                        color.b() as f32 / 255.,
                    ]
                });
                let index = ((x + 1) * 9 + (y + 1) * 3 + (z + 1)) as usize;
                for tri in shaped_shell(index).chunks_exact(3) {
                    let points: [V3; 3] = std::array::from_fn(|i| view(transform(tri[i].position)));
                    let surface_normal = cross(
                        std::array::from_fn(|i| points[1][i] - points[0][i]),
                        std::array::from_fn(|i| points[2][i] - points[0][i]),
                    );
                    // Perspective culling: normal.z alone is wrong near a silhouette.
                    if dot(
                        surface_normal,
                        [-points[0][0], -points[0][1], CAMERA_DISTANCE - points[0][2]],
                    ) <= 0.
                    {
                        continue;
                    }
                    for i in 0..3 {
                        let n = view(transform(tri[i].normal));
                        output.push(Vertex {
                            position: points[i],
                            normal: n,
                            local: tri[i].local,
                            color: colors[tri[i].face],
                        });
                    }
                }
            }
        }
    }
    output
}

/// Same perspective and depth convention used by the GPU shader.
pub fn projected(v: V3, width: usize, height: usize) -> [f32; 3] {
    let size = width.min(height) as f32;
    let inverse_depth = 1. / (CAMERA_DISTANCE - v[2]);
    [
        width as f32 * 0.5 + v[0] * size * PROJECTION_SCALE * CAMERA_DISTANCE * inverse_depth,
        height as f32 * 0.5 - v[1] * size * PROJECTION_SCALE * CAMERA_DISTANCE * inverse_depth,
        inverse_depth,
    ]
}

pub fn rasterize(vertices: &[Vertex], width: usize, height: usize) -> eframe::egui::ColorImage {
    use eframe::egui::{Color32, ColorImage};
    let mut pixels = vec![Color32::TRANSPARENT; width * height];
    let mut depth = vec![0f32; width * height];
    for tri in vertices.chunks_exact(3) {
        let p: [[f32; 3]; 3] = std::array::from_fn(|i| projected(tri[i].position, width, height));
        let min_x = p
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize;
        let max_x = p
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.)
            .min(width as f32) as usize;
        let min_y = p
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize;
        let max_y = p
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.)
            .min(height as f32) as usize;
        let area =
            (p[1][1] - p[2][1]) * (p[0][0] - p[2][0]) + (p[2][0] - p[1][0]) * (p[0][1] - p[2][1]);
        if area.abs() < 1e-8 {
            continue;
        }
        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let a = ((p[1][1] - p[2][1]) * (px - p[2][0])
                    + (p[2][0] - p[1][0]) * (py - p[2][1]))
                    / area;
                let b = ((p[2][1] - p[0][1]) * (px - p[2][0])
                    + (p[0][0] - p[2][0]) * (py - p[2][1]))
                    / area;
                let c = 1. - a - b;
                if a < 0. || b < 0. || c < 0. {
                    continue;
                }
                let q = a * p[0][2] + b * p[1][2] + c * p[2][2];
                let index = y * width + x;
                if q <= depth[index] {
                    continue;
                }
                depth[index] = q;
                let interpolate = |attribute: fn(&Vertex) -> V3| {
                    std::array::from_fn(|i| {
                        (a * attribute(&tri[0])[i] * p[0][2]
                            + b * attribute(&tri[1])[i] * p[1][2]
                            + c * attribute(&tri[2])[i] * p[2][2])
                            / q
                    })
                };
                let color = shade(
                    interpolate(|v| v.color),
                    interpolate(|v| v.normal),
                    interpolate(|v| v.position),
                    interpolate(|v| v.local),
                )
                .map(|c| (c * 255.).round().clamp(0., 255.) as u8);
                pixels[index] = Color32::from_rgb(color[0], color[1], color[2]);
            }
        }
    }
    ColorImage::new([width, height], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_shell_is_closed_and_outward() {
        use std::collections::HashMap;
        let key = |v: V3| v.map(|x| (x * 1_000_000.).round() as i32);
        let mut edges = HashMap::new();
        for tri in shell().chunks_exact(3) {
            let a = tri[0].position;
            let b = tri[1].position;
            let c = tri[2].position;
            let n = cross(
                std::array::from_fn(|i| b[i] - a[i]),
                std::array::from_fn(|i| c[i] - a[i]),
            );
            assert!(dot(n, a) > 0.);
            for i in 0..3 {
                let mut edge = [key(tri[i].position), key(tri[(i + 1) % 3].position)];
                edge.sort();
                *edges.entry(edge).or_insert(0) += 1;
            }
        }
        assert!(
            edges.values().all(|count| *count == 2),
            "Every edge belongs to two triangles"
        );
    }
    #[test]
    fn crowned_exteriors_are_closed_and_internal_slabs_never_overlap() {
        use std::collections::HashMap;
        let key = |v: V3| v.map(|x| (x * 1_000_000.).round() as i32);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let p = [x as f32, y as f32, z as f32];
                    let mut edges = HashMap::new();
                    for tri in shell().chunks_exact(3) {
                        let n = NORMALS[tri[0].face];
                        let crowned = dot(p, n) == 1.;
                        let positions: [V3; 3] =
                            std::array::from_fn(|i| surface_position(&tri[i], crowned));
                        let face_normal = cross(
                            std::array::from_fn(|i| positions[1][i] - positions[0][i]),
                            std::array::from_fn(|i| positions[2][i] - positions[0][i]),
                        );
                        assert!(dot(face_normal, positions[0]) > 0.);
                        for point in positions {
                            for axis in 0..3 {
                                if p[axis] < 1. {
                                    assert!(
                                        point[axis] <= 0.496001,
                                        "Internal positive slab protrudes"
                                    );
                                }
                                if p[axis] > -1. {
                                    assert!(
                                        point[axis] >= -0.496001,
                                        "Internal negative slab protrudes"
                                    );
                                }
                            }
                        }
                        for i in 0..3 {
                            let mut edge = [key(positions[i]), key(positions[(i + 1) % 3])];
                            edge.sort();
                            *edges.entry(edge).or_insert(0) += 1;
                        }
                    }
                    assert!(edges.values().all(|count| *count == 2));
                }
            }
        }
    }
    #[test]
    fn shaped_cubies_are_closed_outward_and_have_finite_unit_normals() {
        use std::collections::HashMap;
        let key = |v: V3| v.map(|x| (x * 1_000_000.).round() as i32);
        for index in 0..27 {
            let center = [
                (index / 9) as f32 - 1.,
                ((index / 3) % 3) as f32 - 1.,
                (index % 3) as f32 - 1.,
            ];
            let mut edges = HashMap::new();
            for tri in shaped_shell(index).chunks_exact(3) {
                let a = tri[0].position;
                let b = tri[1].position;
                let c = tri[2].position;
                let geometric_normal = cross(
                    std::array::from_fn(|i| b[i] - a[i]),
                    std::array::from_fn(|i| c[i] - a[i]),
                );
                let outward = std::array::from_fn(|i| a[i] - center[i]);
                assert!(
                    dot(geometric_normal, outward) > 0.,
                    "Cubie {index} contains a collapsed or inverted triangle"
                );
                for vertex in tri {
                    assert!(vertex.position.into_iter().all(f32::is_finite));
                    assert!(vertex.normal.into_iter().all(f32::is_finite));
                    assert!((dot(vertex.normal, vertex.normal) - 1.).abs() < 0.0001);
                    assert!(
                        dot(vertex.normal, geometric_normal) > 0.,
                        "Cubie {index} has a lighting normal pointing into its surface"
                    );
                }
                for i in 0..3 {
                    let mut edge = [key(tri[i].position), key(tri[(i + 1) % 3].position)];
                    assert_ne!(edge[0], edge[1], "Cubie {index} has a collapsed edge");
                    let direction = if edge[0] < edge[1] { 1 } else { -1 };
                    edge.sort();
                    let (count, balance) = edges.entry(edge).or_insert((0, 0));
                    *count += 1;
                    *balance += direction;
                }
            }
            assert!(
                edges
                    .values()
                    .all(|&(count, balance)| count == 2 && balance == 0),
                "Cubie {index} has an open edge or inconsistent adjacent winding"
            );
        }
    }

    #[test]
    fn shaped_cubies_preserve_internal_slabs_and_flat_mechanical_faces() {
        for index in 0..27 {
            let center = [
                (index / 9) as f32 - 1.,
                ((index / 3) % 3) as f32 - 1.,
                (index % 3) as f32 - 1.,
            ];
            for (source, shaped) in shell().iter().zip(shaped_shell(index)) {
                let face_normal = NORMALS[source.face];
                let exterior = dot(center, face_normal) == 1.;
                let original = add(center, surface_position(source, exterior));
                for axis in 0..3 {
                    if center[axis] < 1. {
                        assert!(shaped.position[axis] <= center[axis] + 0.496001);
                    }
                    if center[axis] > -1. {
                        assert!(shaped.position[axis] >= center[axis] - 0.496001);
                    }
                    // Both sides of the two layer gaps are unchanged, even
                    // where their other coordinates reach the outer silhouette.
                    if original[axis].abs() <= 0.504001 {
                        assert!((shaped.position[axis] - original[axis]).abs() < 0.000001);
                    }
                }
                if !exterior && dot(source.shell_normal, face_normal) > 0.999999 {
                    assert!(
                        dot(shaped.normal, face_normal) > 0.99999,
                        "Cubie {index} lost the normal of an internal slicing face"
                    );
                }
            }
        }
    }

    #[test]
    fn outer_silhouette_is_rounded_and_symmetric_under_logical_quarter_turns() {
        let corner = outer_profile([1.496, 1.496, 1.496]);
        assert!(corner.into_iter().all(|v| v > 1.3 && v < 1.44));
        assert_eq!(outer_profile([1.496, 0., 0.]), [1.496, 0., 0.]);
        for point in [
            [1.496, 1.496, 1.496],
            [-1.496, 1.27, 0.504],
            [1.50, -1.35, -1.21],
            [-0.496, 0.504, -1.496],
            [1.1961, -1.1959, 1.4],
        ] {
            for axis in 0..3 {
                for turns in 1..=3 {
                    let angle = turns as f32 * std::f32::consts::FRAC_PI_2;
                    let turned = outer_profile(rotate(point, axis, angle));
                    let shaped_then_turned = rotate(outer_profile(point), axis, angle);
                    assert!(
                        turned
                            .into_iter()
                            .zip(shaped_then_turned)
                            .all(|(a, b)| (a - b).abs() < 0.000001),
                        "Outer shape changes at a logical layer endpoint"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "Run explicitly in release mode to measure live cube mesh preparation"]
    fn scene_generation_profile() {
        let start = std::time::Instant::now();
        let mut count = 0;
        for frame in 0..120 {
            let vertices = scene(
                Cube::default(),
                crate::render::HOME_YAW,
                crate::render::HOME_PITCH,
                Some((Move::parse("R").unwrap(), frame as f32 / 120.)),
                IDENTITY,
            );
            count += std::hint::black_box(vertices).len();
        }
        println!(
            "Scene generation: {:.3} ms/frame, {} mean visible vertices",
            start.elapsed().as_secs_f64() * 1000. / 120.,
            count / 120
        );
    }
    #[test]
    fn gyro_sign_change_takes_the_short_arc() {
        let q = normalize([0.1, 0.2, 0.3, 0.9]).unwrap();
        let r = slerp(q, q.map(|x| -x), 0.5);
        assert!(q.iter().zip(r).all(|(a, b)| (a - b).abs() < 1e-6));
    }
    #[test]
    fn moving_geometry_fits_during_arbitrary_orbits() {
        for step in 0..24 {
            let angle = step as f32 * 0.27;
            let axis = normal([0.31, 0.73, 0.46]);
            let q = [
                axis[0] * angle.sin(),
                axis[1] * angle.sin(),
                axis[2] * angle.sin(),
                angle.cos(),
            ];
            let vertices = scene(
                Cube::default(),
                crate::render::HOME_YAW,
                crate::render::HOME_PITCH,
                Some((Move::parse("R").unwrap(), 0.5)),
                q,
            );
            for vertex in vertices {
                let p = projected(vertex.position, 240, 240);
                assert!(
                    p[0] > 1. && p[0] < 239. && p[1] > 1. && p[1] < 239.,
                    "Orbit clips at {p:?}"
                );
            }
        }
    }
    #[test]
    fn turn_endpoint_matches_committed_state() {
        let cube = Cube::default();
        let mv = Move::parse("R").unwrap();
        let a = rasterize(
            &scene(
                cube,
                crate::render::HOME_YAW,
                crate::render::HOME_PITCH,
                Some((mv, 1.)),
                IDENTITY,
            ),
            160,
            160,
        );
        let b = rasterize(
            &scene(
                cube.turn(mv),
                crate::render::HOME_YAW,
                crate::render::HOME_PITCH,
                None,
                IDENTITY,
            ),
            160,
            160,
        );
        let differences = a
            .pixels
            .iter()
            .zip(&b.pixels)
            .filter(|(a, b)| {
                a.to_array()
                    .iter()
                    .zip(b.to_array())
                    .any(|(x, y)| x.abs_diff(y) > 2)
            })
            .count();
        assert!(
            differences < 12,
            "Turn endpoint has {differences} differing pixels"
        );
    }

    #[test]
    fn paired_outer_endpoints_match_committed_raw_turns_on_all_axes() {
        let cube = Cube::default().apply(&crate::cube::sequence("R U F2 L D B'").unwrap());
        let orientation = normalize([0.1, 0.3, -0.2, 0.9]).unwrap();
        for notation in ["R L'", "L R'", "U D'", "D U'", "F B'", "B F'"] {
            let moves = crate::cube::sequence(notation).unwrap();
            let motion = super::super::Motion {
                first: moves[0],
                second: Some(moves[1]),
                progress: 1.,
                partner_progress: 1.,
            };
            let a = rasterize(
                &scene_motion(cube, -0.66, 0.38, Some(motion), orientation),
                144,
                144,
            );
            let b = rasterize(
                &scene_motion(cube.apply(&moves), -0.66, 0.38, None, orientation),
                144,
                144,
            );
            let differences = a
                .pixels
                .iter()
                .zip(&b.pixels)
                .filter(|(a, b)| {
                    a.to_array()
                        .iter()
                        .zip(b.to_array())
                        .any(|(x, y)| x.abs_diff(y) > 2)
                })
                .count();
            assert!(
                differences < 12,
                "{notation} endpoint has {differences} differing pixels"
            );
        }
    }
}
