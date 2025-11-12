#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::thread;
use nalgebra_glm::{Vec3, Vec4, Mat3, Mat4};
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
mod planetshaders;

const ORIGIN_BIAS: f32 = 1e-4;

use framebuffers::Framebuffer;
use vertex::Vertex;
use planetshaders::rock_planet_shader;
use obj::Obj;
use triangle::triangle;
use shaders::vertex_shader;

use crate::planetshaders::crater_planet_shader;
use crate::planetshaders::lava_planet_shader;
use crate::planetshaders::gas_planet_shader;
use crate::planetshaders::star_core_shader;
use crate::planetshaders::StarParams;
use std::sync::{OnceLock, Mutex};



// const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);


struct SceneObject {
    vertices: Vec<Vertex>,
    object_type: String,
    translation: Vec3,
    rotation: Vec3,
    scale: f32,
    color: u32,
}


pub struct Light {
    pub position: nalgebra_glm::Vec3,
    pub color: nalgebra_glm::Vec3,
    pub intensity: f32,
}

pub struct Uniforms {
    pub model_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub light_position: Vec3,
    pub light_color: Vec3,
    pub light_intensity: f32,
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

fn create_perspective_matrix(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y / 2.0).tan();
    Mat4::new(
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, (far + near) / (near - far), (2.0 * far * near) / (near - far),
        0.0, 0.0, -1.0, 0.0,
    )
}



fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], colorobj: color::Color, object_type: &str, star_params: &StarParams) {
    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    for vertex in &mut transformed_vertices {
        if object_type == "star" {
            let shaded_vertex = star_core_shader(vertex, uniforms, star_params);
            *vertex = shaded_vertex;
        }
        if object_type == "rocky" {
            let shaded_vertex = rock_planet_shader(vertex, uniforms);
            *vertex = shaded_vertex;
        }
        if object_type == "moon" {
            let shaded_vertex = crater_planet_shader(vertex, uniforms);
            *vertex = shaded_vertex;
        }
        if object_type == "lava" {
            let shaded_vertex = lava_planet_shader(vertex, uniforms);
            *vertex = shaded_vertex;
        }
        if object_type == "gassy" {
            let shaded_vertex = gas_planet_shader(vertex, uniforms);
            *vertex = shaded_vertex;
        }
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
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2], colorobj)); // Medium gray
    }

    // Fragment Processing Stage
    for fragment in fragments {
        // use signed coords to avoid accidental underflow when casting negative positions
        let x = fragment.position.x as isize;
        let y = fragment.position.y as isize;
        if x >= 0 && y >= 0 && (x as usize) < framebuffer.width && (y as usize) < framebuffer.height {
            // lighting basis
            let light_dir = (uniforms.light_position - fragment.world_pos).normalize();
            let normal = fragment.normal.normalize();

            // simple ambient + lambert diffuse
            let ambient = 0.12f32;
            let diff = normal.dot(&light_dir).max(0.0);
            let diffuse = diff * uniforms.light_intensity;
            let mut final_color = fragment.color.to_vec3().component_mul(&uniforms.light_color) * (ambient + diffuse);

            // per-shader procedural modifications (interpret shader by object_type)
            match object_type {
                // Rocky planet: noisy, slightly darkened, some color variation
                "rocky" => {
                    let p = fragment.world_pos;
                    let hash = (p.x * 12.9898 + p.y * 78.233 + p.z * 37.719).sin() * 43758.5453;
                    let n = hash.fract().abs();
                    // bias towards darker, and add subtle color variation
                    let roughness = 0.5 + 0.5 * n;
                    let tint = 0.85 + 0.3 * ( (p.x * 0.03).sin() * 0.5 + 0.5 );
                    final_color *= roughness * tint;
                }

                // Lava / molten planet: emphasize reds, add glow veins
                "lava" | "lava_planet" => {
                    let p = fragment.world_pos;
                    let veins = ((p.x * 0.08).sin() + (p.y * 0.06).sin() * 0.5).abs();
                    let glow = veins.powf(3.0) * 2.0; // concentrated bright veins
                    final_color = final_color.component_mul(&Vec3::new(1.2, 0.6, 0.45)); // warm base
                    final_color += Vec3::new(1.0, 0.35, 0.05) * glow; // add lava glow
                }

                // Moon / cratered: dark spots with soft edges
                "moon" | "moon_planet" | "crater" => {
                    let p = fragment.world_pos;
                    let freq = 0.02f32;
                    let hash = (p.x * 12.9898 * freq + p.y * 78.233 * freq + p.z * 37.719 * freq).sin() * 43758.5453;
                    let n = hash.fract().abs();
                    let threshold = 0.46f32;
                    let edge = 0.18f32;
                    let raw = ((n - threshold) / edge).clamp(0.0, 1.0);
                    let mask = 1.0 - raw; // 1 in center of craters
                    let darken_amount = 0.78f32;
                    let darkening = 1.0 - darken_amount * mask;
                    final_color *= darkening;
                    // slight ambient lift so craters are visible
                    final_color += Vec3::repeat(0.02);
                }

                // Sun: emissive, largely unaffected by surface normal, bright
                "sun" => {
                    // combine light color/intensity with base color to get emissive result
                    let emissive = uniforms.light_color * (uniforms.light_intensity * 0.9);
                    final_color = fragment.color.to_vec3().component_mul(&emissive);
                    // add radial flicker using world pos
                    let p = fragment.world_pos;
                    let flicker = 0.9 + 0.2 * ((p.x * 0.12).sin() + (p.y * 0.1).sin());
                    final_color *= flicker;
                }

                // Ring: desaturated, mostly flat shading
                "ring" => {
                    // convert to grayscale then tint slightly
                    let c = final_color;
                    let lum = 0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z;
                    let tint = Vec3::new(0.9, 0.9, 0.95);
                    final_color = Vec3::repeat(lum) .component_mul(&tint) * 0.9;
                }

                "gassy" => {
                    // richer gas giant scheme: layered bands, subtle turbulence, and a localized "storm" spot
                    let p = fragment.world_pos;
                    let n = fragment.normal.normalize();

                    // base band coordinate (latitude-like) with small longitudinal wiggle for turbulence
                    let lat = p.y * 0.025;
                    let lon_wobble = (p.x * 0.02).sin() * 0.08 + (p.z * 0.015).cos() * 0.06;
                    let band_coord = lat + lon_wobble;

                    // band pattern (0..1)
                    let bands = 0.5 + 0.5 * (band_coord).sin();

                    // small-scale granular variation
                    let hash = (p.x * 12.9898 + p.y * 78.233 + p.z * 37.719).sin() * 43758.5453;
                    let grain = (hash.fract().abs() * 1.25).clamp(0.0, 1.0);

                    // palette: creamy, orange/brown, deep brown, pale blue highlights
                    let cream = Vec3::new(0.96, 0.88, 0.78);
                    let amber = Vec3::new(0.92, 0.62, 0.35);
                    let umber = Vec3::new(0.55, 0.40, 0.33);
                    let pale_blue = Vec3::new(0.65, 0.78, 0.9); // intentionally replaced below
                    final_color = if bands < 0.33 {
                        cream
                    } else if bands < 0.66 {
                        amber
                    } else {
                        umber
                    };
                }

                // Shuttle / default: add a small specular highlight
                _ => {
                    // approximate view direction as +z (camera looking along -z)
                    let view_dir = Vec3::new(0.0, 0.0, 1.0);
                    let half = (light_dir + view_dir).normalize();
                    let spec = normal.dot(&half).max(0.0).powf(24.0);
                    final_color += Vec3::repeat(1.0) * (spec * 0.65);
                }
            }

            // clamp and write to framebuffer
            let clamped = Vec3::new(
                final_color.x.clamp(0.0, 1.0),
                final_color.y.clamp(0.0, 1.0),
                final_color.z.clamp(0.0, 1.0),
            );

            framebuffer.set_current_color(crate::color::Color::from_vec3(clamped).to_hex());
            framebuffer.point(x as usize, y as usize, fragment.depth);

            
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

    window.set_position(100, 100);
    window.update();

    framebuffer.set_background_color(0x333355);

    // let mut translation = Vec3::new(300.0, 200.0, 0.0);
    // let mut rotation = Vec3::new(0.0, 0.0, 0.0);
    // let mut scale = 100.0f32;

    let shuttle_obj = Obj::load("assets/SpaceShuttle.obj").expect("Failed to load obj");
    let planet_obj = Obj::load("assets/sphere.obj").expect("Failed to load obj");
    let sun_obj = Obj::load("assets/sun.obj").expect("Failed to load obj");
    let ring_obj = Obj::load("assets/ring.obj").expect("Failed to load obj");
    
    let shuttle = SceneObject {
        vertices: shuttle_obj.get_vertex_array(),
        object_type: "shuttle".to_string(),
        translation: Vec3::new(300.0, 200.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        scale: 100.0,
        color: 0xB5DCB9,
    };

    let planet_gassy_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy".to_string(),
        translation: Vec3::new(700.0, 500.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        scale: 50.0,
        color: 0x66BBFF,
    };

    let planet_rocky_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "rocky".to_string(),
        translation: Vec3::new(500.0, 500.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        scale: 50.0,
        color: 0x66BBFF,
    };

    let ring = SceneObject {
        vertices: ring_obj.get_vertex_array(),
        object_type: "ring".to_string(),
        translation: Vec3::new(700.0, 500.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        scale: 50.0,
        color: 0xCCCCCC,
    };

    let light = Light {
        position: Vec3::new(1000.0, 500.0, 000.0), // cerca del planeta
        color: Vec3::new(1.0, 1.0, 0.9),          // amarillento
        intensity: 2.0,                           // brillo
    };

    let star_params = StarParams {
        star_center: light.position,
        core_radius: 50.0,
        plasma_radius: 50.0 * 1.05,
        corona_radius: 50.0 * 1.25,
        flare_radius: 50.0 * 1.6,
        time: 0.0,
    };

    let star = SceneObject {
        vertices: sun_obj.get_vertex_array(),
        object_type: "star".to_string(),
        translation: light.position,
        rotation: Vec3::zeros(),
        scale: 50.0,
        color: 0xFFFF66, // Amarillo brillante
    }; 

    let moon = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "moon".to_string(),
        translation: Vec3::new(850.0, 500.0, 0.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        scale: 20.0,
        color: 0xAAAAAA,
    };

    let sun = SceneObject {
        vertices: sun_obj.get_vertex_array(),
        object_type: "lava".to_string(),
        translation: light.position,
        rotation: Vec3::zeros(),
        scale: 50.0,
        color: 0xFFFF66, // Amarillo brillante
    };

    let scene_objects = vec![shuttle, moon, planet_gassy_1, planet_rocky_1, ring, sun, star];


    

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        //handle_input(&window, &mut scene_objects[0]);

        

        framebuffer.clear();

        let target = Vec3::new(700.0, 500.0, 0.0);
        let aspect_ratio = framebuffer_width as f32 / framebuffer_height as f32;
        let projection_matrix = create_perspective_matrix(PI / 3.0, aspect_ratio, 0.1, 10000.0);

        framebuffer.set_current_color(0xFFDDDD);

        for obj in &scene_objects {
            let model_matrix = create_model_matrix(obj.translation, obj.scale, obj.rotation);
            let uniforms = Uniforms {
                model_matrix,
                projection_matrix,
                light_position: light.position,
                light_color: light.color,
                light_intensity: light.intensity,
            };
            render(&mut framebuffer, &uniforms, &obj.vertices, crate::color::Color::from_hex(obj.color), &obj.object_type, &star_params);
        }

        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }


}


fn handle_input(window: &Window, object: &mut SceneObject) {
    if window.is_key_down(Key::Right) {
        object.translation.x += 10.0;
    }
    if window.is_key_down(Key::Left) {
        object.translation.x -= 10.0;
    }
    if window.is_key_down(Key::Up) {
        object.translation.y -= 10.0;
    }
    if window.is_key_down(Key::Down) {
        object.translation.y += 10.0;
    }
    if window.is_key_down(Key::S) {
        object.scale += 2.0;
    }
    if window.is_key_down(Key::A) {
        object.scale -= 2.0;
    }
    if window.is_key_down(Key::Q) {
        object.rotation.x -= PI / 10.0;
    }
    if window.is_key_down(Key::W) {
        object.rotation.x += PI / 10.0;
    }
    if window.is_key_down(Key::E) {
        object.rotation.y -= PI / 10.0;
    }
    if window.is_key_down(Key::R) {
        object.rotation.y += PI / 10.0;
    }
    if window.is_key_down(Key::T) {
        object.rotation.z -= PI / 10.0;
    }
    if window.is_key_down(Key::Y) {
        object.rotation.z += PI / 10.0;
    }
}