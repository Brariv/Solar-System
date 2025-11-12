use nalgebra_glm::{Vec3, Vec4, Mat3};
use crate::vertex::Vertex;

use crate::Uniforms;
use rand::Rng;

pub fn rock_planet_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    // Work in object space first
    let original_pos = vertex.position;
    let pos_len = nalgebra_glm::length(&original_pos);
    let dir = if pos_len > 0.0 { original_pos / pos_len } else { Vec3::new(0.0, 0.0, 1.0) };

    // helper pseudo-random / noise functions (trigonometric hashing based, deterministic)
    fn hash_point(p: Vec3, seed: f32) -> f32 {
      ((p.x * 127.1 + p.y * 311.7 + p.z * 74.7 + seed).sin() * 43758.5453).fract().abs()
    }
    fn snoise(p: Vec3, freq: f32, seed: f32) -> f32 {
      // continuous-ish single octave using sin(dot)
      let s = (p.x * 12.9898 + p.y * 78.233 + p.z * 45.164) * freq + seed;
      (s.sin() * 43758.5453).fract().abs()
    }
    // fractal brownian motion (fbm) using multiple octaves of snoise
    fn fbm(p: Vec3, octaves: usize, base_freq: f32, seed: f32) -> f32 {
      let mut value = 0.0;
      let mut amp = 0.5;
      let mut freq = base_freq;
      for i in 0..octaves {
        value += amp * snoise(p, freq, seed + (i as f32) * 19.19);
        freq *= 2.0;
        amp *= 0.5;
      }
      value
    }

    // base color from vertex
    let base_color = Vec3::new(
      vertex.color.r as f32 / 255.0,
      vertex.color.g as f32 / 255.0,
      vertex.color.b as f32 / 255.0,
    );

    // Evaluate multi-scale surface detail (rocky bumps + fine grain)
    let detail_scale = 3.5; // larger -> larger features across the planet
    let coarse = fbm(dir * detail_scale, 4, 0.9, 1.0); // broad boulder shapes
    let fine = fbm(dir * detail_scale * 6.0, 3, 1.7, 7.0); // small grain
    // combine into displacement value in range approx [-0.15, 0.25]
    let displacement = coarse * 0.12 + (fine - 0.25) * 0.03;

    // radial displacement along normal (simulate bumpy rock)
    let displaced_len = (pos_len * (1.0 + displacement)).max(0.001);
    let displaced_pos = dir * displaced_len;

    // compute perturbed normal by sampling nearby displaced positions on the unit sphere
    let up = Vec3::new(0.0, 1.0, 0.0);
    let tangent = {
      // pick a stable tangent
      let t = if dir.x.abs() < 0.8 { nalgebra_glm::cross(&up, &dir) } else { nalgebra_glm::cross(&Vec3::new(1.0, 0.0, 0.0), &dir) };
      let ln = nalgebra_glm::length(&t);
      if ln > 0.0 { t / ln } else { Vec3::new(1.0, 0.0, 0.0) }
    };
    let bitangent = nalgebra_glm::cross(&dir, &tangent);

    let eps = 0.02; // sample offset amount on unit sphere
    // sample positions slightly offset on the sphere and compute their displacements
    let sample_p0 = nalgebra_glm::normalize(&(dir + tangent * eps));
    let sample_p1 = nalgebra_glm::normalize(&(dir + bitangent * eps));

    let d0 = fbm(sample_p0 * detail_scale, 4, 0.9, 1.0) * 0.12 + (fbm(sample_p0 * detail_scale * 6.0, 3, 1.7, 7.0) - 0.25) * 0.03;
    let d1 = fbm(sample_p1 * detail_scale, 4, 0.9, 1.0) * 0.12 + (fbm(sample_p1 * detail_scale * 6.0, 3, 1.7, 7.0) - 0.25) * 0.03;

    let p_center = dir * (pos_len * (1.0 + displacement));
    let p_t = sample_p0 * (pos_len * (1.0 + d0));
    let p_b = sample_p1 * (pos_len * (1.0 + d1));

    let perturbed_normal_obj = nalgebra_glm::normalize(&nalgebra_glm::cross(&(p_t - p_center), &(p_b - p_center)));

    // Transform displaced position with model matrix and perform perspective division
    let world_pos = Vec4::new(displaced_pos.x, displaced_pos.y, displaced_pos.z, 1.0);
    let transformed = uniforms.model_matrix * world_pos;
    let w = transformed.w;
    let transformed_position = Vec3::new(transformed.x / w, transformed.y / w, transformed.z / w);

    // Transform perturbed normal with normal matrix
    let model_mat3 = Mat3::new(
      uniforms.model_matrix[0], uniforms.model_matrix[1], uniforms.model_matrix[2],
      uniforms.model_matrix[4], uniforms.model_matrix[5], uniforms.model_matrix[6],
      uniforms.model_matrix[8], uniforms.model_matrix[9], uniforms.model_matrix[10]
    );
    let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());
    let transformed_normal = normal_matrix * perturbed_normal_obj;

    // Color variations: darker in crevices, lighter on protrusions, slight hue shift to look "rocky"
    let shade = 0.6 + coarse * 0.4 + fine * 0.15;
    let hue_shift = Vec3::new(1.0, 0.97, 0.92); // slight warm rock tint
    let rock_color = base_color.component_mul(&hue_shift) * shade;

    Vertex {
      position: vertex.position,
      normal: vertex.normal,
      tex_coords: vertex.tex_coords,
      color: crate::color::Color::from_vec3(rock_color),
      transformed_position,
      transformed_normal,
    }
  }



  pub fn crater_planet_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
      // Work in object space first
      let original_pos = vertex.position;
      let pos_len = nalgebra_glm::length(&original_pos);
      let dir = if pos_len > 0.0 { original_pos / pos_len } else { Vec3::new(0.0, 0.0, 1.0) };

      // Base color from vertex
      let mut base_color = Vec3::new(
        vertex.color.r as f32 / 255.0,
        vertex.color.g as f32 / 255.0,
        vertex.color.b as f32 / 255.0,
      );
      let darkened_color = base_color * 0.5;

      // Parameters for crater generation (small features)
      let crater_count = 16;
      let mut accumulated_depth = 0.0;

      // Deterministic pseudo-random crater centers and radii using trigonometric hashing
      for i in 0..crater_count {
        let seed = (i as f32) * 13.37 + dir.x * 97.3 + dir.y * 47.1 + dir.z * 83.9;
        let rnd1 = (seed.sin() * 43758.5453).fract().abs();
        let rnd2 = ((seed * 1.618).sin() * 15731.547).fract().abs();

        // crater center on unit sphere (spherical coordinates)
        let lat = (rnd1 * 2.0 - 1.0).asin();
        let lon = rnd2 * std::f32::consts::TAU;
        let center = {
          let cl = lat.cos();
          Vec3::new(cl * lon.cos(), cl * lon.sin(), lat.sin())
        };

        // angular radius in radians (small features)
        let radius_ang = 0.08 + rnd1 * 0.20; // ~0.08..0.28 rad
        // depth scaling relative to vertex distance
        let max_depth = 0.03 + rnd2 * 0.12; // fraction of radius length

        // angular distance between this vertex and crater center
        let dot = nalgebra_glm::dot(&dir, &center).max(-1.0).min(1.0);
        let ang_dist = dot.acos();

        if ang_dist < radius_ang {
          // interior of crater: k in [0,1], 1 at center, 0 at rim
          let k = 1.0 - (ang_dist / radius_ang);
          let k_smooth = k * k * (3.0 - 2.0 * k); // smoothstep-like
          // accumulate depth (multiple craters can overlap)
          let depth = max_depth * k_smooth;
          accumulated_depth = (accumulated_depth + depth).min(0.8);

          // darken crater interior
          let darken = 1.0 - 0.5 * k_smooth;
          base_color *= darken;

          // modify normal to simulate inward slope: blend toward a normal pointing slightly toward crater center
          let crater_normal = nalgebra_glm::normalize(&(dir * (1.0 - k_smooth * 0.6) - center * (k_smooth * 0.6)));
          let blend = k_smooth * 0.9;
        }
      }

      fn snoise(p: Vec3, freq: f32, seed: f32) -> f32 {
        // continuous-ish single octave using sin(dot)
        let s = (p.x * 12.9898 + p.y * 78.233 + p.z * 45.164) * freq + seed;
        (s.sin() * 43758.5453).fract().abs()
      }

      // fractal brownian motion (fbm) using multiple octaves of snoise
      fn fbm(p: Vec3, octaves: usize, base_freq: f32, seed: f32) -> f32 {
        let mut value = 0.0;
        let mut amp = 0.5;
        let mut freq = base_freq;
        for i in 0..octaves {
          value += amp * snoise(p, freq, seed + (i as f32) * 19.19);
          freq *= 2.0;
          amp *= 0.5;
        }
        value
      }

      let detail_scale = 1.0; // larger -> larger features across the planet

      let final_scale = 1.0 - accumulated_depth;
      let displaced_len = (pos_len * final_scale).max(0.001);
      let displaced_pos = dir * displaced_len;

      // Transform displaced position with model matrix and perform perspective division
      let world_pos = nalgebra_glm::Vec4::new(displaced_pos.x, displaced_pos.y, displaced_pos.z, 1.0);
      let transformed = uniforms.model_matrix * world_pos;
      let w = transformed.w;
      let transformed_position = Vec3::new(transformed.x / w, transformed.y / w, transformed.z / w);

      // Transform modified normal with normal matrix
      let model_mat3 = Mat3::new(
        uniforms.model_matrix[0], uniforms.model_matrix[1], uniforms.model_matrix[2],
        uniforms.model_matrix[4], uniforms.model_matrix[5], uniforms.model_matrix[6],
        uniforms.model_matrix[8], uniforms.model_matrix[9], uniforms.model_matrix[10]
      );
      let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());
      let crater_normal = nalgebra_glm::normalize(&(dir * (1.0 - accumulated_depth * 0.6)));
      let transformed_normal = normal_matrix * crater_normal;

      // Slight overall color tint for craters/spots (make them a bit more natural)
      let coarse = fbm(dir * detail_scale, 4, 0.9, 1.0); // broad boulder shapes
      let fine = fbm(dir * detail_scale * 6.0, 3, 1.7, 7.0); // small grain
      let shade = 0.6 + coarse * 0.4 + fine * 0.15;
      let hue_shift = Vec3::new(1.0, 0.97, 0.92); // slight warm rock tint
      let rock_color = base_color.component_mul(&hue_shift) * shade;

      Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: crate::color::Color::from_vec3(rock_color),
        transformed_position,
        transformed_normal,
      }
}


pub fn gas_planet_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
  use std::f32::consts::PI;
  let original_pos = vertex.position;
  let pos_len = nalgebra_glm::length(&original_pos);
  let dir = if pos_len > 0.0 { original_pos / pos_len } else { Vec3::new(0.0, 0.0, 1.0) };

  // deterministic noise helpers
  fn snoise(p: Vec3, freq: f32, seed: f32) -> f32 {
    let s = (p.x * 12.9898 + p.y * 78.233 + p.z * 45.164) * freq + seed;
    (s.sin() * 43758.5453).fract().abs()
  }
  fn fbm(p: Vec3, octaves: usize, base_freq: f32, seed: f32) -> f32 {
    let mut val = 0.0;
    let mut amp = 0.5;
    let mut freq = base_freq;
    for i in 0..octaves {
      val += amp * snoise(p, freq, seed + i as f32 * 13.37);
      freq *= 2.0;
      amp *= 0.5;
    }
    val
  }

  // Convert direction to spherical coordinates
  let lat = dir.y.asin(); // [-pi/2, pi/2]
  let lon = dir.z.atan2(dir.x); // [-pi, pi]

  // Large-scale atmospheric bands
  let band_freq = 8.0;
  let bands = (lat * band_freq).sin().abs();
  let turbulence = fbm(dir * 4.0, 5, 1.2, 3.3);
  let swirl = fbm(Vec3::new(lat * 2.0, lon * 2.0, turbulence), 3, 2.0, 5.0);

  // Base hues: gas giants tend to be layered with colors
  let hue1 = Vec3::new(0.95, 0.8, 0.65); // warm beige
  let hue2 = Vec3::new(0.5, 0.7, 0.9);   // cool blue
  let hue3 = Vec3::new(0.9, 0.6, 0.3);   // orange tones

  // Blend bands with turbulence
  let mix1 = (bands * 0.8 + turbulence * 0.4).clamp(0.0, 1.0);
  let mix2 = (swirl * 0.6 + bands * 0.4).clamp(0.0, 1.0);
  let color_mix = hue1 * (1.0 - mix1) + hue2 * mix1;
  let final_color = color_mix * (1.0 - 0.3 * mix2) + hue3 * (0.3 * mix2);

  // subtle volumetric haze and soft light scattering effect
  let haze = 0.5 + 0.5 * fbm(dir * 10.0, 4, 1.6, 7.0);
  let softened = nalgebra_glm::normalize(&(final_color * haze));

  // No displacement (gas is soft), just color and smooth shading
  let displaced_pos = dir * pos_len;

  let world_pos = Vec4::new(displaced_pos.x, displaced_pos.y, displaced_pos.z, 1.0);
  let transformed = uniforms.model_matrix * world_pos;
  let w = transformed.w;
  let transformed_position = Vec3::new(transformed.x / w, transformed.y / w, transformed.z / w);

  let model_mat3 = Mat3::new(
    uniforms.model_matrix[0], uniforms.model_matrix[1], uniforms.model_matrix[2],
    uniforms.model_matrix[4], uniforms.model_matrix[5], uniforms.model_matrix[6],
    uniforms.model_matrix[8], uniforms.model_matrix[9], uniforms.model_matrix[10]
  );
  let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());
  let transformed_normal = normal_matrix * dir; // smooth normal

  // Slight transparency: output color with alpha < 1.0.
  // Use a modest alpha so the planet appears slightly translucent.
  let alpha = 0.85_f32;
  let color_vec4 = Vec4::new(softened.x, softened.y, softened.z, alpha);

  Vertex {
    position: vertex.position,
    normal: vertex.normal,
    tex_coords: vertex.tex_coords,
    color: crate::color::Color::from_vec4(color_vec4),
    transformed_position,
    transformed_normal,
  }
}


pub fn lava_planet_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
  // object-space position and direction
  let original_pos = vertex.position;
  let pos_len = nalgebra_glm::length(&original_pos);
  let dir = if pos_len > 0.0 { original_pos / pos_len } else { Vec3::new(0.0, 0.0, 1.0) };

  // deterministic single-octave noise and fbm helpers
  fn snoise(p: Vec3, freq: f32, seed: f32) -> f32 {
    let s = (p.x * 12.9898 + p.y * 78.233 + p.z * 45.164) * freq + seed;
    (s.sin() * 43758.5453).fract().abs()
  }
  fn fbm(p: Vec3, octaves: usize, base_freq: f32, seed: f32) -> f32 {
    let mut value = 0.0;
    let mut amp = 0.5;
    let mut freq = base_freq;
    for i in 0..octaves {
    value += amp * snoise(p, freq, seed + (i as f32) * 19.19);
    freq *= 2.0;
    amp *= 0.5;
    }
    value
  }

  // base color of crust from vertex
  let base_color = Vec3::new(
    vertex.color.r as f32 / 255.0,
    vertex.color.g as f32 / 255.0,
    vertex.color.b as f32 / 255.0,
  );

  // multi-scale noise to carve cracks and flows
  let scale = 4.0;
  let coarse = fbm(dir * scale, 4, 0.9, 2.0); // large forms
  let fine = fbm(dir * scale * 6.0, 3, 1.7, 9.0); // small detail

  // carve cracks: produce negative displacement for cracks exposing hot core
  // value in [-0.08, 0.12] roughly
  let displacement = coarse * 0.06 + (fine - 0.3) * 0.015 - (snoise(dir * 12.0, 1.0, 5.0) * 0.04);

  // deeper cracks (lava channels) where noise is below threshold -> produce emissive glow factor
  let crack_mask = ((0.28 - fbm(dir * 12.0, 3, 1.6, 4.0)).max(0.0) * 8.0).min(1.0);
  let glow_intensity = crack_mask.powf(1.5);

  // radial displacement and displaced position
  let displaced_len = (pos_len * (1.0 + displacement)).max(0.001);
  let displaced_pos = dir * displaced_len;

  // perturb normal by sampling neighbors on sphere (small bump detail)
  let up = Vec3::new(0.0, 1.0, 0.0);
  let tangent = {
    let t = if dir.x.abs() < 0.8 { nalgebra_glm::cross(&up, &dir) } else { nalgebra_glm::cross(&Vec3::new(1.0, 0.0, 0.0), &dir) };
    let ln = nalgebra_glm::length(&t);
    if ln > 0.0 { t / ln } else { Vec3::new(1.0, 0.0, 0.0) }
  };
  let bitangent = nalgebra_glm::cross(&dir, &tangent);

  let eps = 0.015;
  let sample_p0 = nalgebra_glm::normalize(&(dir + tangent * eps));
  let sample_p1 = nalgebra_glm::normalize(&(dir + bitangent * eps));

  let d0 = fbm(sample_p0 * scale, 4, 0.9, 2.0) * 0.06 + (fbm(sample_p0 * scale * 6.0, 3, 1.7, 9.0) - 0.3) * 0.015;
  let d1 = fbm(sample_p1 * scale, 4, 0.9, 2.0) * 0.06 + (fbm(sample_p1 * scale * 6.0, 3, 1.7, 9.0) - 0.3) * 0.015;

  let p_center = dir * (pos_len * (1.0 + displacement));
  let p_t = sample_p0 * (pos_len * (1.0 + d0));
  let p_b = sample_p1 * (pos_len * (1.0 + d1));
  let perturbed_normal_obj = nalgebra_glm::normalize(&nalgebra_glm::cross(&(p_t - p_center), &(p_b - p_center)));

  // world-space transform and perspective divide (match style of other shaders)
  let world_pos = Vec4::new(displaced_pos.x, displaced_pos.y, displaced_pos.z, 1.0);
  let transformed = uniforms.model_matrix * world_pos;
  let w = transformed.w;
  let transformed_position = Vec3::new(transformed.x / w, transformed.y / w, transformed.z / w);

  // normal matrix transform
  let model_mat3 = Mat3::new(
    uniforms.model_matrix[0], uniforms.model_matrix[1], uniforms.model_matrix[2],
    uniforms.model_matrix[4], uniforms.model_matrix[5], uniforms.model_matrix[6],
    uniforms.model_matrix[8], uniforms.model_matrix[9], uniforms.model_matrix[10]
  );
  let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());
  let transformed_normal = normal_matrix * perturbed_normal_obj;

  // color composition:
  // crust: dark, ashy; lava: bright orange -> yellow inside cracks
  let crust_tint = Vec3::new(0.15, 0.12, 0.10);
  let lava_color = Vec3::new(1.0, 0.55, 0.1); // molten core
  let ember_color = Vec3::new(1.0, 0.8, 0.45); // hot edges

  let crust_base = base_color.component_mul(&crust_tint) * (0.5 + coarse * 0.5);
  // glow mixes in where cracks appear; add rim ember for small-scale detail
  let ember = ember_color * (fine * 0.6);
  let glow = lava_color * glow_intensity * (0.8 + coarse * 0.4);

  // final color: crust darkened + ember + glow leaking from cracks
  let mut final_color = crust_base + ember * 0.6 + glow;

  // apply slight fresnel-like rim to simulate heated edges facing camera (cheap approximation)
  let view_dir_approx = nalgebra_glm::normalize(&Vec3::new(0.0, 0.0, 1.0)); // approximate view
  let rim = 1.0 - nalgebra_glm::dot(&nalgebra_glm::normalize(&transformed_normal), &view_dir_approx).max(0.0);
  final_color += ember * rim * 0.25;

  // clamp to reasonable range
  final_color.x = final_color.x.max(0.0).min(1.5);
  final_color.y = final_color.y.max(0.0).min(1.5);
  final_color.z = final_color.z.max(0.0).min(1.5);

  Vertex {
    position: vertex.position,
    normal: vertex.normal,
    tex_coords: vertex.tex_coords,
    color: crate::color::Color::from_vec3(final_color),
    transformed_position,
    transformed_normal,
  }
}


pub struct StarParams {
    pub star_center: nalgebra_glm::Vec3,
    pub core_radius: f32,   // R
    pub plasma_radius: f32, // R * 1.05
    pub corona_radius: f32, // R * 1.25
    pub flare_radius: f32,  // R * 1.6 (max reach of arcs)
    pub time: f32,
}


fn hash3(p: Vec3) -> f32 {
    let q = Vec3::new(
        p.x * 127.1 + p.y * 311.7 + p.z * 74.7,
        p.x * 269.5 + p.y * 183.3 + p.z * 246.1,
        p.x * 113.5 + p.y * 271.9 + p.z * 124.6,
    );
    (q.dot(&Vec3::new(1.0, 57.0, 113.0))).sin().fract().abs()
}

fn noise3(p: Vec3) -> f32 {
    // simple sin-based hash noise
    let s = p.x * 12.9898 + p.y * 78.233 + p.z * 45.164;
    (s.sin() * 43758.5453).fract().abs()
}

fn fbm(p: Vec3, octaves: usize) -> f32 {
    let mut val = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..octaves {
        val += amp * noise3(p * freq);
        freq *= 2.0;
        amp *= 0.5;
    }
    val
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn star_core_color(
    world_pos: Vec3,
    params: &StarParams,
) -> (Vec3, f32) {
    let dir = nalgebra_glm::normalize(&(world_pos - params.star_center));

    // Base core intensity
    let emissive_intensity = 1.3_f32; // in [1.2, 1.5]
    let core_color = Vec3::new(1.0, 1.0, 1.0) * emissive_intensity;

    // Flicker
    let dot_val = (dir * 12.0).dot(&Vec3::new(1.3, 5.7, 9.1));
    let mut flicker = 0.9 + 0.1 * (dot_val + params.time * 5.0).sin();
    flicker = flicker.clamp(0.8, 1.1);

    let color_core = core_color * flicker;
    let alpha_core = 1.0; // fully opaque

    (color_core, alpha_core)
}

pub fn star_plasma_color(
    world_pos: Vec3,
    params: &StarParams,
) -> (Vec3, f32) {
    let star_center = params.star_center;
    let R = params.core_radius;

    let offset = world_pos - star_center;
    let r = nalgebra_glm::length(&offset);
    let dir = offset / r;

    // Gradient factor: u = smoothstep(R, R*1.05, r)
    let inner_r = R;
    let outer_r = R * 1.05;
    let u = smoothstep(inner_r, outer_r, r);

    let inner_col = Vec3::new(0.91, 1.0, 1.0); // #E8FFFF
    let outer_col = Vec3::new(0.49, 0.97, 1.0); // #7DF9FF
    let base_plasma_color = inner_col * (1.0 - u) + outer_col * u;

    // Noise / texture
    let n1 = fbm(dir * 8.0 + Vec3::new(params.time * 0.4, 0.0, 0.0), 4);
    let n2 = fbm(dir * 16.0 + Vec3::new(params.time * 0.9, 0.0, 0.0), 3);

    let mut turb = 0.4 + 0.6 * n1 + 0.3 * n2;
    turb = turb.clamp(0.0, 1.3);

    let filament = n2.max(0.0).powf(3.0) * 0.5;

    let color_plasma =
        base_plasma_color * (0.8 + turb * 0.7) +
        Vec3::new(0.3, 0.5, 0.7) * filament;

    let alpha_plasma = 0.8_f32; // in [0.7, 0.9]

    (color_plasma, alpha_plasma)
}

pub fn star_corona_color(
    world_pos: Vec3,
    params: &StarParams,
) -> (Vec3, f32) {
    let star_center = params.star_center;
    let R = params.core_radius;

    let offset = world_pos - star_center;
    let r = nalgebra_glm::length(&offset);
    let dir = offset / r;

    // Normalized radius for halo: v = (r - R*1.1) / (R*0.2)
    let mut v = (r - R * 1.1) / (R * 0.2);
    v = v.clamp(0.0, 1.0);

    let halo = 1.0 - smoothstep(0.3, 1.0, v);

    // Base colors
    let deep_blue = Vec3::new(0.0, 0.02, 0.2);   // #000533
    let cyan_edge = Vec3::new(0.0, 0.75, 1.0);   // #00BFFF

    let base_corona_color = deep_blue * (1.0 - halo) + cyan_edge * halo;

    // Angular coords
    let angle = dir.y.atan2(dir.x); // [-pi, pi]
    let lat = dir.z.asin();         // [-pi/2, pi/2]

    // band_noise = fbm(vec3(angle*2.0, lat*2.0, 0.0) + t*0.2, 3)
    let band_noise = fbm(
        Vec3::new(angle * 2.0, lat * 2.0, 0.0)
            + Vec3::new(params.time * 0.2, 0.0, 0.0),
        3
    );

    // radial_noise = fbm(dir*6.0 + t*0.3, 4)
    let radial_noise = fbm(
        dir * 6.0 + Vec3::new(params.time * 0.3, 0.0, 0.0),
        4
    );

    let mut corona_variation = 0.7 + 0.4 * band_noise + 0.6 * radial_noise;
    corona_variation = corona_variation.clamp(0.0, 1.5);

    let color_corona = base_corona_color * corona_variation;

    // Alpha
    let mut alpha_corona = 0.45_f32; // in [0.3, 0.6]
    alpha_corona *= halo;

    (color_corona, alpha_corona)
}

pub fn star_flare_color(
    world_pos: Vec3,
    s: f32,       // [0,1] along the arc
    dist: f32,    // distance from the arc center (for edge alpha)
    random_phase: f32,
    params: &StarParams,
) -> (Vec3, f32) {
    // Base gradient along arc
    let inner_color = Vec3::new(0.75, 1.0, 1.0) * (1.0 - s)
        + Vec3::new(0.0, 1.0, 1.0) * s; // #BFFFFF -> #00FFFF
    let outer_color = inner_color * (1.0 - s.powf(3.0))
        + Vec3::new(1.0, 1.0, 1.0) * s.powf(3.0); // tip near white
    let mut color_flare = outer_color * 1.4; // boosted

    // Pulse over time
    let pulse = 0.7 + 0.3 * (params.time * 3.0 + random_phase).sin();
    color_flare *= pulse;

    // Alpha: center bright, edges fade out
    let alpha_center = 0.8_f32;
    let falloff = smoothstep(0.0, 1.0, (1.0 - dist).clamp(0.0, 1.0));
    let alpha_flare = alpha_center * falloff;

    (color_flare, alpha_flare)
}

pub fn star_core_shader(vertex: &Vertex, uniforms: &Uniforms, params: &StarParams) -> Vertex {
    let world_pos4 = uniforms.model_matrix * Vec4::new(
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        1.0,
    );
    let world_pos = Vec3::new(world_pos4.x, world_pos4.y, world_pos4.z);

    let (color_core, _alpha) = star_core_color(world_pos, params);

    // transform like your other shaders
    let w = world_pos4.w;
    let transformed_position = Vec3::new(world_pos4.x / w, world_pos4.y / w, world_pos4.z / w);

    // normal transform (optional, for a Fresnel rim)
    // ...

    Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: crate::color::Color::from_vec3(color_core),
        transformed_position,
        transformed_normal: vertex.normal, // or proper normal transform
    }
}