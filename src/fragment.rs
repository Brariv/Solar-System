use nalgebra_glm::Vec2;
use crate::color::Color;

pub struct Fragment {
    pub position: Vec2,
    pub color: Color,
    pub depth: f32,
    pub world_pos: nalgebra_glm::Vec3,
    pub normal: nalgebra_glm::Vec3,
}

impl Fragment {
    pub fn new(x: f32, y: f32, color: Color, depth: f32, world_pos: nalgebra_glm::Vec3, normal: nalgebra_glm::Vec3) -> Self {
        Fragment {
            position: Vec2::new(x, y),
            color,
            depth,
            world_pos,
            normal,
        }
    }
}

