#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::thread;
use nalgebra_glm::{Vec3, Mat4};
use std::time::{Duration, Instant};
use minifb::{Key, Window, WindowOptions};
use raylib::ffi::TextFormat;
use raylib::prelude::*;
use raylib::prelude::RaylibDraw;
use std::f32::consts::PI;
use std::io::BufReader;
use rayon::prelude::*;

mod framebuffers;
mod triangle;
mod line;
mod vertex;
mod obj;
mod color;
mod fragment;
mod shaders;

const ORIGIN_BIAS: f32 = 1e-4;

use framebuffers::Framebuffer;
use vertex::Vertex;
use obj::Obj;
use triangle::triangle;
use shaders::vertex_shader;

// const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);

pub struct Uniforms {
    model_matrix: Mat4,
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
    );

    transform_matrix * rotation_matrix
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex]) {
    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Primitive Assembly Stage
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterization Stage
    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2]));
    }

    // Fragment Processing Stage
    for fragment in fragments {
        let x = fragment.position.x as usize;
        let y = fragment.position.y as usize;
        if x < framebuffer.width && y < framebuffer.height {
            let color = fragment.color.to_hex();
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, fragment.depth);
        }
    }

    // Fragment Processing Stage
    // use signed coords to avoid accidental underflow when casting negative positions
    // for fragment in fragments {
    //     let x = fragment.position.x as isize;
    //     let y = fragment.position.y as isize;
    //     if x >= 0 && y >= 0 && (x as usize) < framebuffer.width && (y as usize) < framebuffer.height {
    //         // obtain color and apply a small ambient boost to prevent overly dark shadows
    //         let mut color = fragment.color.to_hex();

    //         // extract channels
    //         let mut r = ((color >> 16) & 0xFF) as u8;
    //         let mut g = ((color >> 8) & 0xFF) as u8;
    //         let mut b = (color & 0xFF) as u8;

    //         // compute luminance (perceptual) and apply ambient lift when too dark
    //         let luminance = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
    //         let ambient = 0.22f32; // ambient light factor (tweak to taste)
    //         if luminance < 0.18 {
    //             // boost channels toward white proportional to how dark the pixel is
    //             let boost = ambient * (1.0 - luminance);
    //             r = ((r as f32 + (255.0 - r as f32) * boost).min(255.0)) as u8;
    //             g = ((g as f32 + (255.0 - g as f32) * boost).min(255.0)) as u8;
    //             b = ((b as f32 + (255.0 - b as f32) * boost).min(255.0)) as u8;
    //             color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    //         }

    //         framebuffer.set_current_color(color);
    //         framebuffer.point(x as usize, y as usize, fragment.depth);
    //     }
    // }

    // // draw subtle dark panel edges on top but preserve existing shading (blend instead of overwrite)
    // for y in 0..framebuffer.height {
    //     for x in 0..framebuffer.width {
    //         if x < 5 || x >= framebuffer.width - 5 || y < 5 || y >= framebuffer.height - 5 {
    //             let index = y * framebuffer.width + x;
    //             let cur = framebuffer.buffer[index];
    //             // blend current color with black to darken edges gently (40% of original intensity kept)
    //             let nr = (((cur >> 16) & 0xFF) as f32 * 0.4) as u32;
    //             let ng = (((cur >> 8) & 0xFF) as f32 * 0.4) as u32;
    //             let nb = ((cur & 0xFF) as f32 * 0.4) as u32;
    //             framebuffer.buffer[index] = (nr << 16) | (ng << 8) | nb;
    //         }
    //     }
    // }
    
    
}

fn main() {
    let framebuffer_width = 1300;
    let framebuffer_height = 900;
    let window_width = 1300;
    let window_height = 900;
    let frame_delay = Duration::from_millis(16);

    let mut window = Window::new(
        "Solar System Renderer",
        window_width,
        window_height,
        WindowOptions::default(),
    ).unwrap();

    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);

    window.set_position(500, 500);
    window.update();

    framebuffer.set_background_color(0x333355);

    let mut translation = Vec3::new(300.0, 200.0, 0.0);
    let mut rotation = Vec3::new(0.0, 0.0, 0.0);
    let mut scale = 100.0f32;

    let obj = Obj::load("assets/SpaceShuttle.obj").expect("Failed to load obj");
    let vertex_arrays = obj.get_vertex_array(); 


    

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        handle_input(&window, &mut translation, &mut rotation, &mut scale);

        framebuffer.clear();

        let model_matrix = create_model_matrix(translation, scale, rotation);
        let uniforms = Uniforms { model_matrix };

        framebuffer.set_current_color(0xFFDDDD);
        render(&mut framebuffer, &uniforms, &vertex_arrays);

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }


}


fn handle_input(window: &Window, translation: &mut Vec3, rotation: &mut Vec3, scale: &mut f32) {
    if window.is_key_down(Key::Right) {
        translation.x += 10.0;
    }
    if window.is_key_down(Key::Left) {
        translation.x -= 10.0;
    }
    if window.is_key_down(Key::Up) {
        translation.y -= 10.0;
    }
    if window.is_key_down(Key::Down) {
        translation.y += 10.0;
    }
    if window.is_key_down(Key::S) {
        *scale += 2.0;
    }
    if window.is_key_down(Key::A) {
        *scale -= 2.0;
    }
    if window.is_key_down(Key::Q) {
        rotation.x -= PI / 10.0;
    }
    if window.is_key_down(Key::W) {
        rotation.x += PI / 10.0;
    }
    if window.is_key_down(Key::E) {
        rotation.y -= PI / 10.0;
    }
    if window.is_key_down(Key::R) {
        rotation.y += PI / 10.0;
    }
    if window.is_key_down(Key::T) {
        rotation.z -= PI / 10.0;
    }
    if window.is_key_down(Key::Y) {
        rotation.z += PI / 10.0;
    }
}
