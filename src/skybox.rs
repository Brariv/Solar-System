use raylib::math::Vector3;
use raylib::prelude::{Image, Color};

pub struct SkyboxFace {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Vector3>, // RGB en [0,1]
}

pub struct Skybox {
    pub right: SkyboxFace,
    pub left: SkyboxFace,
    pub top: SkyboxFace,
    pub bottom: SkyboxFace,
    pub front: SkyboxFace,
    pub back: SkyboxFace,
}

/// Convierte una `Image` de raylib en un vector de colores RGB normalizados ([0,1])
/// en el mismo orden de escaneo que los datos de la imagen (row-major).
pub fn image_to_colors(image: &Image) -> Vec<Vector3> {
    // `get_image_data` devuelve un slice de `Color` con todos los píxeles de la imagen.
    // Lo clonamos a un Vec para poder transformarlo.
    let pixels: Vec<Color> = image.get_image_data().to_vec();

    pixels
        .into_iter()
        .map(|c| {
            Vector3::new(
                c.r as f32 / 255.0,
                c.g as f32 / 255.0,
                c.b as f32 / 255.0,
            )
        })
        .collect()
}

// ... SkyboxFace, Skybox, image_to_colors ...

/// Samplea el skybox como un cubemap usando una dirección 3D.
/// `dir` debe ser un vector de dirección en espacio mundo.
pub fn sample_cubemap(skybox: &Skybox, dir: Vector3) -> Vector3 {
    // Normalizar la dirección
    let mut d = dir;
    d.normalize();
    let x = d.x;
    let y = d.y;
    let z = d.z;

    let ax = x.abs();
    let ay = y.abs();
    let az = z.abs();

    // Elegir cara y coords de textura en [-1, 1]
    let (face, u, v) = if ax >= ay && ax >= az {
        // ±X
        if x > 0.0 {
            // +X → right
            let uc = -z / ax;
            let vc = -y / ax;
            (&skybox.right, uc, vc)
        } else {
            // -X → left
            let uc = z / ax;
            let vc = -y / ax;
            (&skybox.left, uc, vc)
        }
    } else if ay >= ax && ay >= az {
        // ±Y
        if y > 0.0 {
            // +Y → top
            let uc = x / ay;
            let vc = z / ay;
            (&skybox.top, uc, vc)
        } else {
            // -Y → bottom
            let uc = x / ay;
            let vc = -z / ay;
            (&skybox.bottom, uc, vc)
        }
    } else {
        // ±Z
        if z > 0.0 {
            // +Z → front
            let uc = x / az;
            let vc = -y / az;
            (&skybox.front, uc, vc)
        } else {
            // -Z → back
            let uc = -x / az;
            let vc = -y / az;
            (&skybox.back, uc, vc)
        }
    };

    // De [-1, 1] a [0, 1]
    let u_tex = (u + 1.0) * 0.5;
    let v_tex = (v + 1.0) * 0.5;

    let w = face.width.max(1) as f32;
    let h = face.height.max(1) as f32;

    let ix = (u_tex * (w - 1.0)).clamp(0.0, w - 1.0) as i32;
    let iy = ((1.0 - v_tex) * (h - 1.0)).clamp(0.0, h - 1.0) as i32;

    let idx = (iy * face.width + ix)
        .clamp(0, face.width * face.height - 1) as usize;

    face.pixels[idx]
}