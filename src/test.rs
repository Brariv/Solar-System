
  pub fn crater_planet_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
      // Work in object space first
      let original_pos = vertex.position;
      let pos_len = nalgebra_glm::length(&original_pos);
      let mut dir = if pos_len > 0.0 { original_pos / pos_len } else { Vec3::new(0.0, 0.0, 1.0) };

      // Base color from vertex
      let mut base_color = Vec3::new(
        vertex.color.r as f32 / 255.0,
        vertex.color.g as f32 / 255.0,
        vertex.color.b as f32 / 255.0,
      );

      // Parameters for crater generation (small features)
      let crater_count = 6;
      let mut accumulated_depth = 0.0;
      let mut modified_normal = nalgebra_glm::normalize(&vertex.normal);

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

          // subtle rim brightening (peak near rim)
          let rim_width = 0.15_f32;
          let rim_t = ((k - (1.0 - rim_width)) / rim_width).max(0.0).min(1.0);
          let rim_highlight = 0.15 * (1.0 - rim_t) * (1.0 - rim_t);
          base_color += Vec3::new(rim_highlight, rim_highlight * 0.9, rim_highlight * 0.7);

          // modify normal to simulate inward slope: blend toward a normal pointing slightly toward crater center
          let crater_normal = nalgebra_glm::normalize(&(dir * (1.0 - k_smooth * 0.6) - center * (k_smooth * 0.6)));
          let blend = k_smooth * 0.9;
          modified_normal = nalgebra_glm::normalize(&(modified_normal * (1.0 - blend) + crater_normal * blend));
        }
      }

      // Now add a layer of huge spots (large-scale surface patches)
      // These are distinct from craters: few, large angular radius, soft falloff, can be brighter or darker and slightly raised/depressed
      let spot_count = 3;
      // total spot offset applied radially (positive = bulge, negative = depression)
      let mut spot_offset = 0.0;
      for i in 0..spot_count {
        // seed depends on index and vertex direction for deterministic placement
        let seed = (i as f32) * 99.13 + dir.y * 41.7 - dir.z * 23.9 + dir.x * 77.7;
        let r1 = (seed.sin() * 43758.5453).fract().abs(); // used for radius
        let r2 = ((seed * 1.732).sin() * 25143.157).fract().abs(); // used for sign / tint
        let r3 = ((seed * 2.718).sin() * 12497.531).fract().abs(); // used for rim/softness

        // center on unit sphere
        let lat = (r1 * 2.0 - 1.0).asin();
        let lon = r2 * std::f32::consts::TAU;
        let center = {
          let cl = lat.cos();
          Vec3::new(cl * lon.cos(), cl * lon.sin(), lat.sin())
        };

        // large angular radius: 0.5..1.2 rad (~30..70 degrees)
        let radius_ang = 0.5 + r1 * 0.7;
        // strength: how much radial offset relative to planet radius (small but noticeable)
        let max_bulge = 0.02 + r3 * 0.06; // up to ~0.08
        // decide if this spot is lighter (positive) or darker (negative)
        let sign = if r2 > 0.5 { 1.0 } else { -1.0 };

        // angular distance
        let dot = nalgebra_glm::dot(&dir, &center).max(-1.0).min(1.0);
        let ang_dist = dot.acos();

        if ang_dist < radius_ang * 1.2 {
          // soft falloff using smoothstep from center to radius * 1.2
          let t = (ang_dist / (radius_ang * 1.2)).min(1.0);
          let fall = 1.0 - (t * t * (3.0 - 2.0 * t)); // smoothstep inverse
          // apply stronger effect near center, softer near rim
          let influence = fall * fall;

          // radial offset accumulates (allow overlap)
          spot_offset += sign * max_bulge * influence;

          // color tint: lighter spots get warm tint, darker spots get ashy tint
          if sign > 0.0 {
        // brighten and warm slightly
        let add = 0.15 * influence;
        base_color += Vec3::new(add, add * 0.95, add * 0.85);
          } else {
        // darken and desaturate slightly
        let mul = 1.0 - 0.18 * influence;
        base_color *= mul;
        let gray = 0.06 * influence;
        base_color -= Vec3::new(gray, gray * 0.95, gray * 0.9);
          }

          // modify normal slightly: bulges push normals outward, depressions pull inward
          let spot_normal = nalgebra_glm::normalize(&(dir * (1.0 - influence * 0.2 * sign) + center * (influence * 0.2 * sign)));
          let nblend = influence * 0.6;
          modified_normal = nalgebra_glm::normalize(&(modified_normal * (1.0 - nblend) + spot_normal * nblend));
        }
      }

      // Apply radial displacement based on accumulated crater depth and spots
      // Craters subtract, spots add (bulge); final multiplier clipped to reasonable range
      let final_scale = (1.0 - accumulated_depth) + spot_offset;
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
      let transformed_normal = normal_matrix * modified_normal;

      // Slight overall color tint for craters/spots (make them a bit more natural)
      let final_color = base_color.component_mul(&Vec3::new(0.94, 0.96, 0.98));

      Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: crate::color::Color::from_vec3(final_color),
        transformed_position,
        transformed_normal,
      }
}