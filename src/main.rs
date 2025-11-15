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
mod skybox;

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
use crate::skybox::{SkyboxFace, Skybox, image_to_colors, sample_cubemap};

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
    scale: f32
}

fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    light: &Light,
    object_type: &str
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
            "shuttle" => shuttle_vertex_shader(vertex),
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
            //"shuttle" => shuttle_chrome_fragment_shader(&fragment, &uniforms),
            _       => rocky_fragment_shader(&fragment, &uniforms), // default
        };

        framebuffer.point(
            fragment.position.x as i32,
            fragment.position.y as i32,
            final_color,
            fragment.depth            
        );
        
    }

    
}

fn load_skybox_face(path: &str) -> SkyboxFace {
    let image = Image::load_image(path).expect("No pude cargar skybox face");
    let width = image.width;
    let height = image.height;

    // Aquí necesitarás obtener los datos de la imagen como píxeles (Color u8 -> Vector3 f32)
    let data: Vec<Vector3> = image_to_colors(&image); // función que tú implementas

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for c in data {
        pixels.push(Vector3::new(
            c.x,
            c.y,
            c.z,
        ));
    }

    SkyboxFace { width, height, pixels }
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
    let camera_position = Vector3::new(0.0, 5.0, 100.0);
    let camera_target = Vector3::new(0.0, 0.0, 0.0);
    let camera_up = Vector3::new(0.0, 1.0, 0.0);
    let mut camera = Camera::new(camera_position, camera_target, camera_up);

    // Projection setup
    let fov_y = PI / 3.0; // 60 degrees
    let aspect = window_width as f32 / window_height as f32;
    let near = 0.1;
    let far = 100.0;

    // Model setup (rotating model at origin)
    let mut rotation_y = 0.0f32;
    let rotation_speed = 0.02; // Radians per frame

    // Light setup (place light at the origin so it matches the sun position)
    let light = Light::new(Vector3::new(0.0, 0.0, 0.0));

    let skybox = Skybox {
        right:  load_skybox_face("assets/skybox/right.png"),
        left:   load_skybox_face("assets/skybox/left.png"),
        top:    load_skybox_face("assets/skybox/bottom.png"),
        bottom: load_skybox_face("assets/skybox/top.png"),
        front:  load_skybox_face("assets/skybox/front.png"),
        back:   load_skybox_face("assets/skybox/back.png"),
    };

    let shuttle_obj = Obj::load("assets/SpaceShuttle.obj").expect("Failed to load obj");
    let planet_obj = Obj::load("assets/sphere.obj").expect("Failed to load obj");
    let sun_obj = Obj::load("assets/sun.obj").expect("Failed to load obj");
    let ring_obj = Obj::load("assets/ring.obj").expect("Failed to load obj");
    
    let shuttle = SceneObject {
        vertices: shuttle_obj.get_vertex_array(),
        object_type: "shuttle".to_string(),
        translation: Vector3::new(0.0, 0.0, 70.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0
    };

    let planet_gassy_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy1".to_string(),
        translation: Vector3::new(18.0, 0.0, -20.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.8
    };

    let ring = SceneObject {
        vertices: ring_obj.get_vertex_array(),
        object_type: "ring".to_string(),
        translation: Vector3::new(18.0, 0.0, -20.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.8
    };

    let planet_gassy_2 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy2".to_string(),
        translation: Vector3::new(28.0, 0.0, 5.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 0.8
    };

    let planet_gassy_3 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "gassy3".to_string(),
        translation: Vector3::new(0.0, 0.0, 40.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0
    };

    let planet_rocky_1 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "rocky1".to_string(),
        translation: Vector3::new(-16.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.2
    };

    let planet_rocky_2 = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "rocky2".to_string(),
        // Rocky planet to the left of the origin
        translation: Vector3::new(-50.0, 0.0, 22.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0
    };

    let earth = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "earth".to_string(),
        translation: Vector3::new(10.0, 0.0, -27.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.2
    };


    let moon = SceneObject {
        vertices: planet_obj.get_vertex_array(),
        object_type: "moon".to_string(),
        translation: Vector3::new(15.0, -2.0, -60.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 0.5,
    };

    let sun = SceneObject {
        vertices: sun_obj.get_vertex_array(),
        object_type: "sun".to_string(),
        translation: light.position,
        rotation: Vector3::new(0.0, 0.0, 0.0),
        scale: 2.5,
    };

    let mut scene_objects = vec![
        planet_rocky_1,
        planet_rocky_2,
        planet_gassy_1,
        planet_gassy_2,
        planet_gassy_3,
        earth,
        moon,
        ring,
        sun,
        shuttle, 
    ];



    while !window.window_should_close() {
        // Process camera input
        camera.process_input(&window);

        if let Some(shuttle_obj) = scene_objects.iter_mut().find(|o| o.object_type == "shuttle") {
            // Camera eye (position) and target define the viewing direction
            let cam_pos = camera.eye;
            let cam_target = camera.target;

            // Forward direction from camera to target, normalized
            let fwd = Vector3::new(
                cam_target.x - cam_pos.x,
                cam_target.y - cam_pos.y,
                cam_target.z - cam_pos.z,
            );
            let len = (fwd.x * fwd.x + fwd.y * fwd.y + fwd.z * fwd.z).sqrt();
            let forward_dir = if len > 0.0 {
                Vector3::new(fwd.x / len, fwd.y / len, fwd.z / len)
            } else {
                Vector3::new(0.0, 0.0, -1.0)
            };

            // Position the shuttle in front of the camera
            let distance_ahead = 5.0;
            let vertical_offset = -1.0;

            shuttle_obj.translation = Vector3::new(
                cam_pos.x + forward_dir.x * distance_ahead,
                cam_pos.y + forward_dir.y * distance_ahead + vertical_offset,
                cam_pos.z + forward_dir.z * distance_ahead,
            );
        }

        // Update model rotation
        rotation_y += rotation_speed;

        // Clear framebuffer (color + depth) at the start of the frame
        framebuffer.clear();

        let cam_pos = camera.eye;
        let cam_target = camera.target;

        // Base de la cámara
        let mut forward = Vector3::new(
            cam_target.x - cam_pos.x,
            cam_target.y - cam_pos.y,
            cam_target.z - cam_pos.z,
        );
        forward.normalize();
        let mut right = forward.cross(camera_up);
        right.normalize();
        
        let mut up = right.cross(forward);
        up.normalize();

        for y in 0..window_height {
            for x in 0..window_width {
                // Coordenadas Normalized Device Coordinates (NDC) en [-1, 1]
                let ndc_x = (2.0 * x as f32 / window_width as f32) - 1.0;
                let ndc_y = 1.0 - (2.0 * y as f32 / window_height as f32);

                // Dirección en espacio de cámara
                let tan_half_fov = (fov_y * 0.5).tan();
                let dir_cam = Vector3::new(
                    ndc_x * aspect * tan_half_fov,
                    ndc_y * tan_half_fov,
                    -1.0, // mirando hacia -Z en espacio de cámara
                );

                // Transformar a espacio mundo usando la base de la cámara
                let dir_world = {
                    let dx = right.x * dir_cam.x + up.x * dir_cam.y + forward.x * dir_cam.z;
                    let dy = right.y * dir_cam.x + up.y * dir_cam.y + forward.y * dir_cam.z;
                    let dz = right.z * dir_cam.x + up.z * dir_cam.y + forward.z * dir_cam.z;
                    let mut dir_world = Vector3::new(dx, dy, dz);
                    dir_world.normalize();
                    dir_world
                };

                let sky_color = sample_cubemap(&skybox, dir_world);

                // Fondo con depth=1.0 (máximo), los objetos con menor depth lo sobreescriben
                framebuffer.point(
                    x as i32,
                    y as i32,
                    sky_color,
                    100.0,
                );
            }
        }

        // Matrices that are global for this frame (camera and projection)
        let view_matrix = camera.get_view_matrix();
        let projection_matrix = create_projection_matrix(fov_y, aspect, near, far);
        let viewport_matrix = create_viewport_matrix(0.0, 0.0, window_width as f32, window_height as f32);

        for obj in &scene_objects {
            // Apply global rotation to planets, but keep the shuttle stable relative to camera
            let rotation = if obj.object_type == "shuttle" {
                obj.rotation
            } else {
                Vector3::new(
                    obj.rotation.x,
                    obj.rotation.y + rotation_y,
                    obj.rotation.z,
                )
            };

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
