mod framebuffer;
mod triangle;
mod line;
mod vertex;
mod fragment;
mod shaders;
mod obj;
mod matrix;
mod camera;
mod light;
mod planetshaders;

use crate::matrix::{create_model_matrix, create_projection_matrix, create_viewport_matrix};
use crate::camera::Camera;
use crate::light::Light;
use framebuffer::Framebuffer;
use vertex::Vertex;
use triangle::triangle;
use crate::shaders::*;
use obj::Obj;
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use std::f32::consts::PI;
use crate::planetshaders::*;

pub struct Uniforms {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
}

struct SceneObject {
    vertices: Vec<Vertex>,
    object_type: String,
    translation: Vector3,
    rotation: Vector3,
    scale: f32,
    color: Vector3,
}

fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    light: &Light,
    object_type: &str,
    color: Vector3,
) {
    // Build an object-specific model matrix and compose a per-object Uniforms
    //let model_matrix = create_model_matrix(translation, scale, rotation);
    // let object_uniforms = Uniforms {
    //     model_matrix,
    //     view_matrix: uniforms.view_matrix,
    //     projection_matrix: uniforms.projection_matrix,
    //     viewport_matrix: uniforms.viewport_matrix,
    // };

    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, &uniforms);
        transformed_vertices.push(transformed);
    }

    for vertex in &mut transformed_vertices {
        match object_type {
            "rocky1" => rocky_planet_vertex_shader(vertex),
            "rocky2" => hot_cold_rocky_planet_vertex_shader(vertex),
            "gassy1" => gassy_planet_vertex_shader(vertex),
            "gassy2" => uranus_like_vertex_shader(vertex),
            "gassy3" => cyan_redband_gas_vertex_shader(vertex),
            "moon"  => moon_vertex_shader(vertex),
            "ring"  => ring_vertex_shader(vertex),
            "sun"  => sun_vertex_shader(vertex),
            "earth" => earth_planet_vertex_shader(vertex),
            _ => {}
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
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2], light));
    }

    // Compute smooth per-fragment shading using the fragment shader.
    // This uses the interpolated normals stored in each fragment to produce smooth (Phong-like) lighting.
    

    

    // // Fragment Processing Stage
    // for fragment in fragments {
    //     framebuffer.point(
    //         fragment.position.x as i32,
    //         fragment.position.y as i32,
    //         fragment.color,
    //         fragment.depth,
    //     );
    // }

    // Fragment Processing Stage
    for fragment in fragments {
        // Run fragment shader to compute final color
        let final_color = match object_type {
            "sun"  => sun_fragment_shader(&fragment, &uniforms),
            "rocky1" => rocky_fragment_shader(&fragment, &uniforms),
            "rocky2" => rocky_fragment_shader(&fragment, &uniforms),
            "gassy1" => gas_giant_fragment_shader(&fragment, &uniforms),
            "gassy2" => gas_giant_fragment_shader(&fragment, &uniforms),
            "gassy3" => gas_giant_fragment_shader(&fragment, &uniforms),
            "earth" => earth_fragment_shader(&fragment, &uniforms),
            "moon"  => moon_fragment_shader(&fragment, &uniforms),
            "ring"  => ring_fragment_shader(&fragment, &uniforms),
            _       => sun_fragment_shader(&fragment, &uniforms), // default
        };

        if object_type == "shuttle" {
            framebuffer.point(
                fragment.position.x as i32,
                fragment.position.y as i32,
                color,
                fragment.depth
            );
        } else {
            framebuffer.point(
                fragment.position.x as i32,
                fragment.position.y as i32,
                final_color,
                fragment.depth
            );
        }
    }

    
}

fn main() {
    let window_width = 800;
    let window_height = 600;

    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Rust Graphics - Renderer Example")
        .log_level(TraceLogLevel::LOG_WARNING) // Suppress INFO messages
        .build();

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);
    framebuffer.set_background_color(Vector3::new(0.2, 0.2, 0.4)); // Dark blue-ish

    // Initialize the texture inside the framebuffer
    framebuffer.init_texture(&mut window, &thread);

    // Camera setup
    let camera_position = Vector3::new(0.0, 5.0, 10.0);
    let camera_target = Vector3::new(0.0, 0.0, 0.0);
    let camera_up = Vector3::new(0.0, 1.0, 0.0);
    let mut camera = Camera::new(camera_position, camera_target, camera_up);

    // Projection setup
    let fov_y = PI / 3.0; // 60 degrees
    let aspect = window_width as f32 / window_height as f32;
    let near = 0.1;
    let far = 100.0;

    // Model setup (rotating model at origin)
    let translation = Vector3::new(0.0, 0.0, 0.0);
    let mut rotation_y = 0.0f32;
    let rotation_speed = 0.02; // Radians per frame
    let scale = 1.0f32;

    // Light setup (place light at the origin so it matches the sun position)
    let light = Light::new(Vector3::new(0.0, 0.0, 0.0));

    let shuttle_obj = Obj::load("assets/SpaceShuttle.obj").expect("Failed to load obj");
    let planet_obj = Obj::load("assets/sphere.obj").expect("Failed to load obj");
    let sun_obj = Obj::load("assets/sun.obj").expect("Failed to load obj");
    let ring_obj = Obj::load("assets/ring.obj").expect("Failed to load obj");
    
    let shuttle = SceneObject {
        vertices: shuttle_obj.get_vertex_array(),
        object_type: "shuttle".to_string(),
        translation: Vector3::new(0.0, 0.0, -20.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0,
        color: Vector3::new(181.0 / 255.0, 220.0 / 255.0, 185.0 / 255.0),
    };

    let planet_gassy_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy1".to_string(),
        // Gas giant to the right of the origin
        translation: Vector3::new(4.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.8,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let planet_gassy_2 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy2".to_string(),
        // Gas giant to the right of the origin
        translation: Vector3::new(8.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 0.8,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let planet_gassy_3 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy3".to_string(),
        // Gas giant to the right of the origin
        translation: Vector3::new(12.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let planet_rocky_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "rocky1".to_string(),
        // Rocky planet to the left of the origin
        translation: Vector3::new(-6.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.2,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let planet_rocky_2 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "rocky2".to_string(),
        // Rocky planet to the left of the origin
        translation: Vector3::new(-10.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let earth = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "earth".to_string(),
        // Earth in front of the origin (slightly towards the camera)
        translation: Vector3::new(0.0, 0.0, -7.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.2,
        color: Vector3::new(102.0 / 255.0, 187.0 / 255.0, 255.0 / 255.0),
    };

    let ring = SceneObject {
        vertices: ring_obj.get_vertex_array(),
        object_type: "ring".to_string(),
        // Ring centered on the gas giant
        translation: Vector3::new(4.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.8,
        color: Vector3::new(204.0 / 255.0, 204.0 / 255.0, 204.0 / 255.0),
    };


    

    let moon = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "moon".to_string(),
        // Small moon offset from Earth
        translation: Vector3::new(2.5, -2.0, -10.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 0.5,
        color: Vector3::new(170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0),
    };

    let sun = SceneObject {
        vertices: sun_obj.get_vertex_array(),
        object_type: "sun".to_string(),
        // Sun at the origin (also matches the light position)
        translation: light.position,
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 2.5,
        color: Vector3::new(255.0 / 255.0, 255.0 / 255.0, 102.0 / 255.0), // Amarillo brillante
    };

    let scene_objects = vec![
        //planet_rocky_1,
        planet_rocky_2,
        //planet_gassy_1,
        planet_gassy_2,
        planet_gassy_3,
        //earth,
        //moon,
        //ring,
        //sun,
        //shuttle, // descomenta si quieres ver el shuttle también
    ];



    while !window.window_should_close() {
        // Process camera input
        camera.process_input(&window);

        // Update model rotation
        rotation_y += rotation_speed;

        framebuffer.clear();

        // Matrices that are global for this frame (camera and projection)
        let view_matrix = camera.get_view_matrix();
        let projection_matrix = create_projection_matrix(fov_y, aspect, near, far);
        let viewport_matrix = create_viewport_matrix(0.0, 0.0, window_width as f32, window_height as f32);

        for obj in &scene_objects {
            // Optional: apply global rotation_y to each object on top of its base rotation
            let rotation = Vector3::new(
                obj.rotation.x,
                obj.rotation.y + rotation_y,
                obj.rotation.z,
            );

            // Per-object model matrix using its own translation, rotation, and scale
            let model_matrix = create_model_matrix(obj.translation, obj.scale, rotation);

            let uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                viewport_matrix,
            };

            render(
                &mut framebuffer,
                &uniforms,
                obj.vertices.as_slice(),
                &light,
                &obj.object_type,
                obj.color,
            );
        }

        // for obj in &scene_objects {
        //     let model_matrix = create_model_matrix(obj.translation, obj.scale, obj.rotation);
        //     let uniforms = Uniforms {
        //         model_matrix,
        //         view_matrix,
        //         projection_matrix,
        //         viewport_matrix,
        //     };
        //     render(&mut framebuffer, &uniforms, obj.vertices.as_slice(), &light, obj.color);
        // }

        // Call the encapsulated swap_buffers function
        framebuffer.swap_buffers(&mut window, &thread);

        

        thread::sleep(Duration::from_millis(16));
    }
}
