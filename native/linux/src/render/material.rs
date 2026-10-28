//! The cube's studio lighting, shared by the native GPU and software rasterizers.
//! Albedo enters as sRGB; all lighting happens in linear light before one encoding.

pub const FRAGMENT: &str = r#"
in vec3 tint;
in vec3 surfaceNormal;
in vec3 surfacePosition;
in vec3 surfaceLocal;
uniform float cameraDistance;
out vec4 fragColor;

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
    // A nearby softbox produces a broad gradient across the whole object, not
    // nine individually highlighted tiles. The ceiling remains the bright side.
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
    // Keep white plastic's shape in the strongest light instead of clipping
    // its broad faces to a uniform display white.
    if (diffuse > 0.90) {
        diffuse = 0.90 + 0.08 * (1.0 - exp(-(diffuse - 0.90) / 0.08));
    }
    vec3 eye = normalize(vec3(-surfacePosition.xy, cameraDistance - surfacePosition.z));
    vec3 albedo = linearColor(tint);
    // Colored molded plastic has a broad satin reflection; dark internal
    // mechanics remain charcoal instead of becoming silver under the softbox.
    float plastic = smoothstep(0.015, 0.060, max(albedo.r, max(albedo.g, albedo.b)));
    float specular = mix(0.012, 0.037, plastic)
        * pow(max(dot(n, normalize(key + eye)), 0.0), 6.0) * keyGain
        * (1.0 - 0.35 * underside);
    fragColor = vec4(srgbColor(clamp(albedo * diffuse + specular, 0.0, 1.0)), 1.0);
}
"#;

type V3 = [f32; 3];

fn dot(a: V3, b: V3) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn unit(v: V3) -> V3 {
    let length = dot(v, v).max(1e-12).sqrt();
    v.map(|v| v / length)
}

/// Mirror `FRAGMENT` exactly so captures and fallback rendering share a material.
pub fn shade(base: V3, normal: V3, position: V3, local: V3) -> V3 {
    use eframe::egui::ecolor::{gamma_from_linear, linear_from_gamma};

    let n = unit(normal);
    let to_key = std::array::from_fn(|i| [-3.5, 6.5, 10.0][i] - position[i]);
    let key = unit(to_key);
    let fill = unit(std::array::from_fn(|i| [7.0, -1.0, 6.0][i] - position[i]));
    let key_gain = (154.5 / dot(to_key, to_key)).clamp(0.78, 1.20);
    // The floor contributes less light than the ceiling. This follows the
    // surface orientation, so any face regains its full color when turned up.
    let t = ((-n[1] - 0.55) / 0.30).clamp(0., 1.);
    let underside = t * t * (3. - 2. * t);
    let room_gain = 1. - 0.30 * underside;
    let ambient = 0.41 + 0.12 * (0.5 + 0.5 * n[1]);
    let mut axes = local.map(f32::abs);
    axes.sort_by(f32::total_cmp);
    let edge = (0.496 - axes[1]).max(0.);
    let contact = 1. - 0.045 * (-edge / 0.018).exp();
    let mut diffuse = ((ambient + 0.16 * dot(n, fill).max(0.)) * room_gain
        + 0.49 * dot(n, key).max(0.) * key_gain)
        * contact;
    if diffuse > 0.90 {
        diffuse = 0.90 + 0.08 * (1. - (-(diffuse - 0.90) / 0.08).exp());
    }
    let eye = unit([
        -position[0],
        -position[1],
        super::mesh::CAMERA_DISTANCE - position[2],
    ]);
    let albedo = base.map(linear_from_gamma);
    let t = ((albedo.into_iter().fold(0., f32::max) - 0.015) / 0.045).clamp(0., 1.);
    let plastic = t * t * (3. - 2. * t);
    let half = unit(std::array::from_fn(|i| key[i] + eye[i]));
    let specular = (0.012 + 0.025 * plastic)
        * dot(n, half).max(0.).powi(6)
        * key_gain
        * (1. - 0.35 * underside);
    albedo.map(|c| gamma_from_linear((c * diffuse + specular).clamp(0., 1.)))
}
