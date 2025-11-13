use raylib::prelude::*;
use crate::vertex::Vertex;
use crate::fragment::Fragment;
use crate::Uniforms;

use crate::matrix::multiply_matrix_vector4;



pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
  // Convert vertex position to homogeneous coordinates (Vec4) by adding a w-component of 1.0
  let position_vec4 = Vector4::new(
    vertex.position.x,
    vertex.position.y,
    vertex.position.z,
    1.0
  );

  // Apply Model transformation
  let world_position = multiply_matrix_vector4(&uniforms.model_matrix, &position_vec4);

  // Apply View transformation (camera)
  let view_position = multiply_matrix_vector4(&uniforms.view_matrix, &world_position);

  // Apply Projection transformation (perspective)
  let clip_position = multiply_matrix_vector4(&uniforms.projection_matrix, &view_position);

  // Perform perspective division to get NDC (Normalized Device Coordinates)
  let ndc = if clip_position.w != 0.0 {
      Vector3::new(
          clip_position.x / clip_position.w,
          clip_position.y / clip_position.w,
          clip_position.z / clip_position.w,
      )
  } else {
      Vector3::new(clip_position.x, clip_position.y, clip_position.z)
  };

  // Apply Viewport transformation to get screen coordinates
  let ndc_vec4 = Vector4::new(ndc.x, ndc.y, ndc.z, 1.0);
  let screen_position = multiply_matrix_vector4(&uniforms.viewport_matrix, &ndc_vec4);

  let transformed_position = Vector3::new(
      screen_position.x,
      screen_position.y,
      screen_position.z,
  );

  // Create a new Vertex with the transformed position
  Vertex {
    position: vertex.position,
    normal: vertex.normal,
    tex_coords: vertex.tex_coords,
    color: vertex.color,
    transformed_position,
    transformed_normal: transform_normal(&vertex.normal, &uniforms.model_matrix),
  }
}

fn transform_normal(normal: &Vector3, model_matrix: &Matrix) -> Vector3 {
    // Convert normal to homogeneous coordinates (w=0 for direction vectors)
    let normal_vec4 = Vector4::new(normal.x, normal.y, normal.z, 0.0);

    // Transform the normal by the model matrix.
    // For non-uniform scaling, the inverse transpose of the model matrix should be used.
    // For uniform scaling (as in this project), transforming by the model matrix is sufficient.
    let transformed_normal_vec4 = multiply_matrix_vector4(model_matrix, &normal_vec4);

    // Convert back to Vector3 and normalize
    let mut transformed_normal = Vector3::new(
        transformed_normal_vec4.x,
        transformed_normal_vec4.y,
        transformed_normal_vec4.z,
    );
    transformed_normal.normalize();
    transformed_normal
}

pub fn fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    // Default fragment shader: just use the interpolated vertex color.
    fragment.color
}

// ------------------------
// Helper functions for fragment shaders
// ------------------------

fn clamp(x: f32, min_v: f32, max_v: f32) -> f32 {
    if x < min_v {
        min_v
    } else if x > max_v {
        max_v
    } else {
        x
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn mix_vec3(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    Vector3::new(
        mix(a.x, b.x, t),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    )
}

fn saturate_vec3(v: Vector3) -> Vector3 {
    Vector3::new(
        clamp(v.x, 0.0, 1.0),
        clamp(v.y, 0.0, 1.0),
        clamp(v.z, 0.0, 1.0),
    )
}

// ------------------------
// Planet-specific fragment shaders
// ------------------------

// 🌞 Sun / star: add a soft radial glow and slight color burn
pub fn sun_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    // Approximate screen center (adjust if your resolution changes)
    let cx = 400.0;
    let cy = 300.0;

    let dx = pos.x - cx;
    let dy = pos.y - cy;
    let r = (dx * dx + dy * dy).sqrt();

    // Glow stronger near center, fading outward
    let glow = clamp(1.0 - r / 350.0, 0.0, 1.0);
    let glow2 = glow * glow;

    let boosted = Vector3::new(
        base.x * (1.0 + 1.8 * glow2),
        base.y * (1.0 + 1.2 * glow2),
        base.z * (1.0 + 0.6 * glow2),
    );

    saturate_vec3(boosted)
}

// 🪨 Rocky planet: add gentle vignette and contrast to make terrain pop
pub fn rocky_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    // Use distance from center of the object on screen approximately
    let cx = 400.0;
    let cy = 300.0;
    let dx = pos.x - cx;
    let dy = pos.y - cy;
    let r = (dx * dx + dy * dy).sqrt();

    // Vignette: a bit darker further from center
    let vignette = mix(1.0, 0.7, clamp(r / 450.0, 0.0, 1.0));

    // Simple contrast curve
    let mut c = base;
    c.x = (c.x - 0.5) * 1.2 + 0.5;
    c.y = (c.y - 0.5) * 1.2 + 0.5;
    c.z = (c.z - 0.5) * 1.2 + 0.5;

    c = Vector3::new(c.x * vignette, c.y * vignette, c.z * vignette);

    saturate_vec3(c)
}

// 🪐 Gas giant: emphasize bands with subtle screen-space waves
pub fn gas_giant_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    // Wave pattern along y (vertical) + a small x-dependent swirl
    let wave = (pos.y / 25.0).sin() * 0.5 + 0.5;
    let swirl = ((pos.x + pos.y * 0.3) / 40.0).cos() * 0.5 + 0.5;

    let band_boost = mix(0.8, 1.3, wave);
    let swirl_mix = mix(0.9, 1.1, swirl);

    let mut c = Vector3::new(
        base.x * band_boost * swirl_mix,
        base.y * band_boost,
        base.z * band_boost * swirl_mix,
    );

    // Slight color shift towards magenta in darker areas
    let magenta_tint = Vector3::new(0.1, 0.0, 0.2);
    c = mix_vec3(c, magenta_tint, 0.15);

    saturate_vec3(c)
}

// 🌍 Earth-like planet: soft atmospheric haze and subtle glow on bright areas
pub fn earth_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    let cx = 400.0;
    let cy = 300.0;
    let dx = pos.x - cx;
    let dy = pos.y - cy;
    let r = (dx * dx + dy * dy).sqrt();

    // "Atmospheric" fade at the edge: mix with space blue
    let edge = clamp(r / 350.0, 0.0, 1.0);
    let atmosphere_color = Vector3::new(0.05, 0.12, 0.25);
    let with_atmo = mix_vec3(base, atmosphere_color, edge * 0.35);

    // Slight "bloom" on bright areas
    let brightness = (with_atmo.x + with_atmo.y + with_atmo.z) / 3.0;
    let bloom_strength = clamp((brightness - 0.5) * 2.0, 0.0, 1.0);
    let bloom_color = Vector3::new(0.9, 0.95, 1.0);
    let color_final = mix_vec3(with_atmo, bloom_color, bloom_strength * 0.3);

    saturate_vec3(color_final)
}

// 🌑 Moon: harsher contrast and subtle specular-like highlight
pub fn moon_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    // Simple directional light approximation using screen-space x
    let light_dir = (pos.x / 200.0).sin() * 0.5 + 0.5;

    // High contrast grey
    let mut c = base;
    c.x = (c.x - 0.5) * 1.4 + 0.5;
    c.y = (c.y - 0.5) * 1.4 + 0.5;
    c.z = (c.z - 0.5) * 1.4 + 0.5;

    // Specular-ish highlight
    let spec = clamp((light_dir - 0.6) * 4.0, 0.0, 1.0);
    let spec_color = Vector3::new(0.9, 0.9, 0.95);
    c = mix_vec3(c, spec_color, spec * 0.5);

    saturate_vec3(c)
}

// 💿 Ring: fade edges and add fine radial band variation
pub fn ring_fragment_shader(fragment: &Fragment, _uniforms: &Uniforms) -> Vector3 {
    let base = fragment.color;
    let pos = fragment.position;

    let cx = 400.0;
    let cy = 300.0;
    let dx = pos.x - cx;
    let dy = pos.y - cy;
    let r = (dx * dx + dy * dy).sqrt();

    // Fade at inner/outer edges
    let inner = 80.0;
    let outer = 260.0;
    let t = clamp((r - inner) / (outer - inner), 0.0, 1.0);

    // Fine radial bands
    let band1 = (r / 6.0).sin() * 0.5 + 0.5;
    let band2 = (r / 3.0).cos() * 0.5 + 0.5;
    let band_mix = 0.6 * band1 + 0.4 * band2;

    let band_color = mix_vec3(base, Vector3::new(0.9, 0.9, 0.95), band_mix * 0.3);

    // Vignette fade
    let fade = clamp(1.0 - (t - 0.5).abs() * 1.8, 0.0, 1.0);
    let color_final = Vector3::new(
        band_color.x * fade,
        band_color.y * fade,
        band_color.z * fade,
    );

    saturate_vec3(color_final)
}
