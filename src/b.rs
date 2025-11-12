// ================== FRAGMENTO QUE MUEVE / USA LA CÁMARA ==================

pub struct App {
    // camera & view
    camera: Camera,
    pub fov_y: f32,
    near: f32,
    far: f32,

    // ...
}

impl App {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(3200.0),
            fov_y: 60.0_f32.to_radians(),
            near: 50.0,
            far: 8000.0,
            // ...
        }
    }

    pub fn handle_hotkeys(&mut self, window: &Window, dt: f32) {
        // ...
        // si cambia el modo de inspección, reajusta el zoom de cámara
        if self.inspect != self.prev_inspect && !self.warp_active {
            let focus_radius = match self.inspect {
                Inspect::Star     => self.star_scale * 1.30,
                Inspect::Rocky    => self.rocky_scale * 1.20,
                Inspect::Gas      => self.gas_scale * 1.20,
                Inspect::Lava     => self.lava_scale * 1.20,
                Inspect::Verdant  => self.verdant_scale * 1.20,
                Inspect::GasGold  => self.gas_gold_scale * 1.20,
                Inspect::All      => 1000.0,
            };
            let inspect_zoom = if self.inspect == Inspect::All { 1.0 } else { 1.35 };
            self.camera.frame_radius(focus_radius * inspect_zoom);
            self.prev_inspect = self.inspect;
        }
    }

    pub fn frame(&mut self, framebuffer: &mut Framebuffer, window: &Window) {
        // input de la cámara (rotar, hacer zoom, etc.)
        self.camera.update_input(window);

        // delta time, teclas, etc.
        // ...

        let aspect = framebuffer.width as f32 / framebuffer.height as f32;

        // centros de los planetas (targets posibles para la cámara)
        let rocky_center_p  = self.center_on_circle(self.rocky_orbit,                             self.rocky_orbit_speed * t_orbit);
        let lava_center_p   = self.center_on_circle(self.rocky_orbit + self.lava_orbit_rel,       self.lava_orbit_speed * t_orbit);
        let verdant_center_p= self.center_on_circle(self.rocky_orbit + self.verdant_orbit_rel,    self.verdant_orbit_speed * t_orbit);
        let gas_center_p    = self.center_on_circle(self.rocky_orbit + self.gas_orbit_rel,        self.gas_orbit_speed * t_orbit);
        let gas_gold_center_p = self.center_on_circle(self.rocky_orbit + self.gas_gold_orbit_rel, self.gas_gold_orbit_speed * t_orbit);

        let target_for = |insp: Inspect| -> Vec3 {
            match insp {
                Inspect::All     => Vec3::new(0.0,0.0,0.0),
                Inspect::Star    => Vec3::new(0.0,0.0,0.0),
                Inspect::Rocky   => rocky_center_p,
                Inspect::Gas     => gas_center_p,
                Inspect::Lava    => lava_center_p,
                Inspect::Verdant => verdant_center_p,
                Inspect::GasGold => gas_gold_center_p,
            }
        };
        let radius_for = |insp: Inspect| -> f32 {
            match insp {
                Inspect::All     => 1000.0,
                Inspect::Star    => self.star_scale * 1.30,
                Inspect::Rocky   => self.rocky_scale * 1.20,
                Inspect::Gas     => self.gas_scale * 1.20,
                Inspect::Lava    => self.lava_scale * 1.20,
                Inspect::Verdant => self.verdant_scale * 1.20,
                Inspect::GasGold => self.gas_gold_scale * 1.20,
            }
        };
        let inspect_zoom = if matches!(self.inspect, Inspect::All) { 1.0 } else { 1.35 };

        // interpolación de cámara durante el “warp”
        let (target, ambient_for_planets) = if self.warp_active {
            self.warp_t += dt;
            let raw = (self.warp_t / self.warp_duration).clamp(0.0, 1.0);
            let s = Self::ease_in_out_cubic(raw);

            let t_from = target_for(self.warp_from);
            let t_to   = target_for(self.warp_to);
            let blended_target = Self::lerp3(t_from, t_to, s);

            let r_from = radius_for(self.warp_from) * inspect_zoom;
            let r_to   = radius_for(self.warp_to)   * inspect_zoom;
            let mut r_blend = Self::lerp(r_from, r_to, s);
            if !r_blend.is_finite() || r_blend.is_nan() || r_blend <= 0.0 { r_blend = 1000.0; }
            self.camera.frame_radius(r_blend);

            if raw >= 1.0 {
                self.warp_active = false;
                self.inspect = self.warp_to;
                self.prev_inspect = self.inspect;

                let mut safe_r_to = r_to;
                if !safe_r_to.is_finite() || safe_r_to.is_nan() || safe_r_to.is_sign_negative() {
                    safe_r_to = 1000.0;
                }
                self.camera.frame_radius(safe_r_to);
                self.warp_cooldown = 0.15;
                if let Some(next) = self.pending_inspect.take() {
                    if next != self.inspect {
                        self.start_warp(next);
                    }
                }
            }
            (blended_target, 1.0)
        } else {
            let tgt = target_for(self.inspect);
            let mut amb = if matches!(self.inspect, Inspect::All) { self.ambient_planet } else { 1.0 };
            if !amb.is_finite() { amb = 1.0; }
            (tgt, amb)
        };

        // MATRIZ DE VISTA (CÁMARA) Y PROYECCIÓN
        let view = self.camera.view_matrix(target);
        let proj = glm::perspective(aspect, self.fov_y, self.near, self.far);
        let screen_size = (framebuffer.width as f32, framebuffer.height as f32);

        // orientación de la cámara usada para el skybox
        let (yaw, pitch) = self.camera.angles();
        self.skybox.draw(framebuffer, self.fov_y, yaw, pitch);

        // ...
        // resto del dibujado de la escena usando `view` y `proj`
        // ...
    }

    fn draw_system(
        &mut self,
        framebuffer: &mut Framebuffer,
        view: Mat4,
        proj: Mat4,
        screen_size: (f32, f32),
        t_real: f32, t_orbit: f32, t_spin: f32,
        ambient_for_planets: f32,
        frozen: bool,
        aspect: f32,
    ) {
        // ...
        // === Nave que sigue a la cámara (third-person) ===
        if !self.warp_active {
            let current_target = match self.inspect {
                Inspect::All | Inspect::Star => Vec3::new(0.0,0.0,0.0),
                Inspect::Rocky   => self.center_on_circle(self.rocky_orbit,                             self.rocky_orbit_speed * t_orbit),
                Inspect::Gas     => self.center_on_circle(self.rocky_orbit + self.gas_orbit_rel,        self.gas_orbit_speed * t_orbit),
                Inspect::Lava    => self.center_on_circle(self.rocky_orbit + self.lava_orbit_rel,       self.lava_orbit_speed * t_orbit),
                Inspect::Verdant => self.center_on_circle(self.rocky_orbit + self.verdant_orbit_rel,    self.verdant_orbit_speed * t_orbit),
                Inspect::GasGold => self.center_on_circle(self.rocky_orbit + self.gas_gold_orbit_rel,   self.gas_gold_orbit_speed * t_orbit),
            };
            let eye = self.camera.eye(current_target);
            // ...
            let mut cam_dist = self.camera.distance();
            // posible ajuste por colisiones…
            // ...
            self.camera.set_distance(cam_dist);
            // ...
        }
    }
}