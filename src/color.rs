use std::fmt;
use nalgebra_glm::Vec3;
use nalgebra_glm::Vec4;

#[derive(Debug, Clone, Copy)]
pub struct Color {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

impl Color {
  // Constructor to initialize the color using r, g, b values as u8
  pub fn new(r: u8, g: u8, b: u8) -> Self {
    Color { r, g, b }
  }

  // default color
  pub fn black() -> Self {
    Color { r: 0, g: 0, b: 0 }
  }

  // New constructor to initialize the color using r, g, b values as f32 (0.0 to 1.0)
  pub fn from_float(r: f32, g: f32, b: f32) -> Self {
    Color {
      r: (r.clamp(0.0, 1.0) * 255.0) as u8,
      g: (g.clamp(0.0, 1.0) * 255.0) as u8,
      b: (b.clamp(0.0, 1.0) * 255.0) as u8,
    }
  }

  pub fn from_vec3(v: Vec3) -> Self {
        let r = (v.x.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (v.y.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (v.z.clamp(0.0, 1.0) * 255.0) as u8;
        Self { r, g, b }
    }

  pub fn from_vec4(v: Vec4) -> Self {
    // Apply alpha (v.w) by compositing the RGB over black:
    // result_rgb = src_rgb * alpha
    let a = v.w.clamp(0.0, 1.0);
    let r = (v.x.clamp(0.0, 1.0) * a * 255.0) as u8;
    let g = (v.y.clamp(0.0, 1.0) * a * 255.0) as u8;
    let b = (v.z.clamp(0.0, 1.0) * a * 255.0) as u8;
    Self { r, g, b }
  }

    /// Converts a Color (0–255 range) to a Vec3 (0.0–1.0 range)
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0)
    }

  // Function to create a color from a hex value
  pub fn from_hex(hex: u32) -> Self {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Color { r, g, b }
  }

  // Function to return the color as a hex value
  pub fn to_hex(&self) -> u32 {
    ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
  }
}

// Implement addition for Color
use std::ops::Add;

impl Add for Color {
  type Output = Color;

  fn add(self, other: Color) -> Color {
    Color {
      r: self.r.saturating_add(other.r),
      g: self.g.saturating_add(other.g),
      b: self.b.saturating_add(other.b),
    }
  }
}

// Implement multiplication by a constant for Color
use std::ops::Mul;

impl Mul<f32> for Color {
  type Output = Color;

  fn mul(self, scalar: f32) -> Color {
    Color {
      r: (self.r as f32 * scalar).clamp(0.0, 255.0) as u8,
      g: (self.g as f32 * scalar).clamp(0.0, 255.0) as u8,
      b: (self.b as f32 * scalar).clamp(0.0, 255.0) as u8,
    }
  }
}

// Implement display formatting for Color
impl fmt::Display for Color {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Color(r: {}, g: {}, b: {})", self.r, self.g, self.b)
  }
}
