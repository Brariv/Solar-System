use nalgebra_glm as glm;
use glm::{Vec3, Mat4};
use minifb::{Window, MouseMode, MouseButton, Key};

pub struct Camera {
    yaw: f32,
    pitch: f32,
    dist: f32,
    mouse_sense: f32,
    pitch_limit: f32,
    last_mouse: Option<(f32,f32)>,
    pub dist_min: f32,
    pub dist_max: f32,
}

impl Camera {
    pub fn new(initial_dist: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            dist: initial_dist,
            mouse_sense: 0.008,
            pitch_limit: 1.3,
            last_mouse: None,
            dist_min: 200.0,
            dist_max: 5000.0,
        }
    }

    #[inline] pub fn set_distance(&mut self, d: f32) {
        self.dist = d.clamp(self.dist_min, self.dist_max);
    }

    /// Frame a body of given radius (rough heuristic)
    #[inline] pub fn frame_radius(&mut self, radius: f32) {
        self.set_distance(radius * 3.8);
        self.last_mouse = None; // avoid jump
    }

    /// Update yaw/pitch (mouse-drag) and zoom (A/S keys)
    pub fn update_input(&mut self, window: &Window) {
        // Mouse drag → orbit camera
        if let Some((mx,my)) = window.get_mouse_pos(MouseMode::Clamp) {
            if window.get_mouse_down(MouseButton::Left) {
                if let Some((px,py)) = self.last_mouse {
                    let dx = mx - px;
                    let dy = my - py;
                    self.yaw   += dx * self.mouse_sense;
                    self.pitch += dy * self.mouse_sense;
                    self.pitch = self.pitch.clamp(-self.pitch_limit, self.pitch_limit);
                }
                self.last_mouse = Some((mx,my));
            } else {
                self.last_mouse = Some((mx,my));
            }
        }

        // Camera zoom on A/S
        if window.is_key_down(Key::S) { self.dist *= 0.98; }
        if window.is_key_down(Key::A) { self.dist *= 1.02; }
        self.dist = self.dist.clamp(self.dist_min, self.dist_max);
    }

    /// Compute view matrix looking at ⁠ target ⁠.
    pub fn view_matrix(&self, target: Vec3) -> Mat4 {
        let eye = Vec3::new(
            target.x + self.dist * self.pitch.cos() * self.yaw.sin(),
            target.y + self.dist * self.pitch.sin(),
            target.z + self.dist * self.pitch.cos() * self.yaw.cos(),
        );
        let up = Vec3::new(0.0, 1.0, 0.0);
        glm::look_at(&eye, &target, &up)
    }

    /// Return (yaw, pitch) for skybox drawing etc.
    #[inline] pub fn angles(&self) -> (f32, f32) { (self.yaw, self.pitch) }

    /// Reset mouse accumulator (call when switching inspect target)
    #[inline] pub fn reset_mouse(&mut self) { self.last_mouse = None; }

    /// Return the current camera distance from target.
    #[inline]
    pub fn distance(&self) -> f32 { self.dist }

    /// Compute and return the eye/world position given a target.
    pub fn eye(&self, target: Vec3) -> Vec3 {
        Vec3::new(
            target.x + self.dist * self.pitch.cos() * self.yaw.sin(),
            target.y + self.dist * self.pitch.sin(),
            target.z + self.dist * self.pitch.cos() * self.yaw.cos(),
        )
    }
}