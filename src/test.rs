fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    edge_overlay: bool,
    edge_color: Color,
) {
    // Vertex Shader Stage
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Primitive Assembly Stage
    let mut tris = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            tris.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterization + Fragment Collection
    let mut fragments = Vec::new();
    for tri in &tris {
        fragments.extend(crate::triangle::triangle(
            &tri[0],
            &tri[1],
            &tri[2],
            uniforms.light_dir,
            uniforms.base_color,
            uniforms.ambient,
        ));
    }

    // Fragment Processing Stage (per-fragment color + depth test)
    for frag in fragments {
        let x = frag.position.x as isize;
        let y = frag.position.y as isize;
        if x >= 0 && y >= 0
            && (x as usize) < framebuffer.width
            && (y as usize) < framebuffer.height
        {
            framebuffer.plot(x as usize, y as usize, frag.color.to_hex(), frag.depth);
        }
    }

    // Optional: draw dark panel edges on top (depth-tested)
    if edge_overlay {
        for tri in &tris {
            let mut edges = Vec::new();
            edges.extend(crate::line::line(&tri[0], &tri[1]));
            edges.extend(crate::line::line(&tri[1], &tri[2]));
            edges.extend(crate::line::line(&tri[2], &tri[0]));
            for f in edges {
                let x = f.position.x as isize;
                let y = f.position.y as isize;
                if x >= 0 && y >= 0
                    && (x as usize) < framebuffer.width
                    && (y as usize) < framebuffer.height
                {
                    framebuffer.plot(x as usize, y as usize, edge_color.to_hex(), f.depth);
                }
            }
        }
    }
}