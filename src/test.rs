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

pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Vector3 {
    // Access fragment properties (all interpolated values)
    let _position = fragment.position;  // Screen-space position
    let base_color = fragment.color;    // Interpolated color from triangle
    let _depth = fragment.depth;        // Interpolated depth

    // Access uniforms (non-interpolated global values)
    let _model = &uniforms.model_matrix;
    let _view = &uniforms.view_matrix;
    let _projection = &uniforms.projection_matrix;
    let _viewport = &uniforms.viewport_matrix;

    // Create a colorful pattern based on screen position
    // This is just for demonstration - you can compute any per-fragment effect here!
    let x_pattern = (fragment.position.x / 50.0).sin() * 0.5 + 0.5;
    let y_pattern = (fragment.position.y / 50.0).cos() * 0.5 + 0.5;

    // Mix the base color with the pattern
    let pattern_color = Vector3::new(
        x_pattern,
        y_pattern,
        (x_pattern + y_pattern) / 2.0,
    );

    // Blend pattern with base color (50% each)
    Vector3::new(
        base_color.x * 0.5 + pattern_color.x * 0.5,
        base_color.y * 0.5 + pattern_color.y * 0.5,
        base_color.z * 0.5 + pattern_color.z * 0.5,
    )
}



