use raylib::prelude::{Vector2, Vector3};

use crate::vertex::Vertex;
use crate::fragment::Fragment;
use crate::light::Light;

// Simple CPU triangle rasterizer that interpolates vertex.color
pub fn triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex, _light: &Light) -> Vec<Fragment> {
    let mut fragments = Vec::new();

    // Use transformed_position as screen-space
    let p0 = v0.transformed_position;
    let p1 = v1.transformed_position;
    let p2 = v2.transformed_position;

    // Bounding box
    let min_x = p0.x.min(p1.x).min(p2.x).floor() as i32;
    let max_x = p0.x.max(p1.x).max(p2.x).ceil() as i32;
    let min_y = p0.y.min(p1.y).min(p2.y).floor() as i32;
    let max_y = p0.y.max(p1.y).max(p2.y).ceil() as i32;

    // Helper for barycentric coordinates
    fn edge(a: Vector3, b: Vector3, c: Vector3) -> f32 {
        (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
    }

    let area = edge(p0, p1, p2);
    if area == 0.0 {
        return fragments; // Degenerate triangle
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let p = Vector3::new(px, py, 0.0);

            let w0 = edge(p1, p2, p);
            let w1 = edge(p2, p0, p);
            let w2 = edge(p0, p1, p);

            // Same sign as area => inside triangle
            if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 && area > 0.0)
                || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0 && area < 0.0)
            {
                let w0n = w0 / area;
                let w1n = w1 / area;
                let w2n = w2 / area;

                // Interpolate depth
                let depth = w0n * p0.z + w1n * p1.z + w2n * p2.z;

                // Interpolate color from vertex colors (set in planetshaders)
                let c0 = v0.color;
                let c1 = v1.color;
                let c2 = v2.color;

                let color = Vector3::new(
                    c0.x * w0n + c1.x * w1n + c2.x * w2n,
                    c0.y * w0n + c1.y * w1n + c2.y * w2n,
                    c0.z * w0n + c1.z * w1n + c2.z * w2n,
                );

                fragments.push(Fragment {
                    position: Vector2::new(px, py),
                    color,
                    depth,
                });
            }
        }
    }

    fragments
}