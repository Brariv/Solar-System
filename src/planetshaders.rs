
use std::f32::consts::PI;

use raylib::prelude::{Vector2, Vector3};

use crate::vertex::Vertex;

// ------------------------
// Helper math functions
// ------------------------

fn clamp(x: f32, min_v: f32, max_v: f32) -> f32 {
    if x < min_v {
        min_v
    } else if x > max_v {
        max_v
    } else {
        x
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn mix_vec3(a: Vector3, b: Vector3, t: f32) -> Vector3 {
    Vector3::new(
        mix(a.x, b.x, t),
        mix(a.y, b.y, t),
        mix(a.z, b.z, t),
    )
}

fn length3(v: Vector3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

fn normalize3(v: Vector3) -> Vector3 {
    let len = length3(v);
    if len > 0.0 {
        Vector3::new(v.x / len, v.y / len, v.z / len)
    } else {
        v
    }
}

fn saturate_vec3(v: Vector3) -> Vector3 {
    Vector3::new(
        clamp(v.x, 0.0, 1.0),
        clamp(v.y, 0.0, 1.0),
        clamp(v.z, 0.0, 1.0),
    )
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// Simple hash-based noise in 2D
fn hash2(p: Vector2) -> f32 {
    let n = p.x * 157.0 + p.y * 113.0;
    (n.sin() * 43758.5453).fract()
}

// Very cheap fractal noise (fbm)
fn fbm(uv: Vector2) -> f32 {
    let mut value = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;

    for _ in 0..4 {
        let p = Vector2::new(uv.x * freq, uv.y * freq);
        value += hash2(p) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }

    value
}

// Convert a normal to [0,1]x[0,1] spherical UV
fn spherical_uv(n: Vector3) -> Vector2 {
    let n = normalize3(n);
    let lon = n.z.atan2(n.x); // [-pi, pi]
    let lat = n.y.asin();     // [-pi/2, pi/2]

    let u = 0.5 + lon / (2.0 * PI);
    let v = 0.5 - lat / PI;

    Vector2::new(u, v)
}

// =======================================================
// SHADERS
// Cada función modifica v.color en función de su normal
// =======================================================

// 🪐 Planeta tipo Urano: púrpura / lila pálido con bandas muy suaves
pub fn uranus_like_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Paleta base más púrpura / lila
    let base_top    = Vector3::new(0.78, 0.72, 0.98); // lila claro
    let base_mid    = Vector3::new(0.65, 0.55, 0.92); // púrpura suave
    let base_bottom = Vector3::new(0.50, 0.40, 0.85); // púrpura más profundo

    // Gradiente vertical suave para que no sea completamente plano
    let t_lat = clamp(uv.y, 0.0, 1.0);
    let mut base_color = if t_lat < 0.5 {
        mix_vec3(base_mid, base_bottom, t_lat * 2.0)
    } else {
        mix_vec3(base_mid, base_top, (t_lat - 0.5) * 2.0)
    };

    // Bandas extremadamente suaves en la componente de brillo
    let band_freq = 10.0;
    let band = (uv.y * band_freq).sin() * 0.5 + 0.5; // 0..1
    let band_strength = mix(0.92, 1.08, band);

    base_color = Vector3::new(
        base_color.x * band_strength,
        base_color.y * band_strength,
        base_color.z * band_strength,
    );

    // Un poco de ruido muy suave para romper la uniformidad
    let noise = fbm(Vector2::new(uv.x * 3.0, uv.y * 3.0));
    let noise_mix = mix(0.96, 1.04, noise);
    base_color = Vector3::new(
        base_color.x * noise_mix,
        base_color.y * noise_mix,
        base_color.z * noise_mix,
    );

    v.color = saturate_vec3(base_color);
}

// 🌀 Gigante gaseoso celeste con una banda roja en el ecuador
pub fn cyan_redband_gas_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Capa 1: gas celeste con bandas suaves
    let band_freq = 12.0;
    let base_bands = (uv.y * band_freq).sin() * 0.5 + 0.5; // 0..1
    let cyan_light = Vector3::new(0.75, 0.92, 0.98);
    let cyan_dark  = Vector3::new(0.50, 0.78, 0.90);
    let mut color  = mix_vec3(cyan_dark, cyan_light, base_bands);

    // Un poco de ruido para rompre la perfección de las bandas
    let swirl = fbm(Vector2::new(uv.x * 5.0, uv.y * 8.0));
    let swirl_intensity = mix(0.9, 1.1, swirl);
    color = Vector3::new(
        color.x * swirl_intensity,
        color.y * swirl_intensity,
        color.z * swirl_intensity,
    );

    // Capa 2: banda roja en el ecuador
    // uv.y ~ 0.5 es el ecuador, usamos smoothstep para hacer una franja relativamente delgada
    let equator_dist = (uv.y - 0.5).abs();
    let band_mask = smoothstep(0.08, 0.0, equator_dist); // 1 cerca del ecuador, 0 lejos
    let red_band_color = Vector3::new(0.90, 0.20, 0.15);

    color = mix_vec3(color, red_band_color, band_mask * 0.9);

    v.color = saturate_vec3(color);
}

// 🪨 Planeta tipo "lava bajo hielo": parches de lava naranja con corteza blanca/gris
pub fn hot_cold_rocky_planet_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Capa 1: mapa base de parches (dónde hay lava vs corteza)
    let field = fbm(Vector2::new(uv.x * 3.0 + 2.0, uv.y * 3.0 + 5.0));
    let lava_mask = smoothstep(0.80, 0.80, field); // 0 = corteza, 1 = lava

    // Borde de transición (anillo)
    let inner = smoothstep(0.70, 0.90, field);
    let outer = smoothstep(0.30, 0.75, field);
    let edge_ring = clamp(outer - inner, 0.0, 1.0);

    // Capa 2: lava brillante (más roja y dominante)
    let lava_base = Vector3::new(1.0, 0.25, 0.05); // rojo/naranja más intenso
    let lava_hot  = Vector3::new(1.0, 0.95, 0.45); // puntos muy calientes casi amarillos
    let lava_detail = fbm(Vector2::new(uv.x * 18.0, uv.y * 18.0));
    let mut lava_color = mix_vec3(lava_base, lava_hot, lava_detail);

    // Pequeño boost extra hacia rojo en las zonas de lava
    lava_color = Vector3::new(
        lava_color.x,
        lava_color.y * 0.85,
        lava_color.z * 0.8,
    );

    // Capa 3: corteza blanca/gris
    let ice_white = Vector3::new(0.95, 0.96, 0.99);
    let ice_grey  = Vector3::new(0.75, 0.78, 0.82);
    let crust_detail = fbm(Vector2::new(uv.x * 10.0, uv.y * 10.0));
    let crust_color = mix_vec3(ice_white, ice_grey, crust_detail);

    // Mezcla lava vs corteza (lava un poco más dominante)
    let lava_influence = clamp(lava_mask * 1.0, 0.0, 1.0);
    let mut color = mix_vec3(crust_color, lava_color, lava_influence);

    // Capa 4: grietas oscuras alrededor de la lava
    let crack_color = Vector3::new(0.05, 0.03, 0.04);
    color = mix_vec3(color, crack_color, edge_ring * 0.9);

    // Capa 5: hollín / suciedad cerca de zonas de lava
    let soot_noise = fbm(Vector2::new(uv.x * 8.0 + 7.0, uv.y * 14.0 + 3.0));
    let soot_mask = edge_ring * smoothstep(0.4, 0.8, soot_noise);
    let soot_color = Vector3::new(0.12, 0.12, 0.14);
    color = mix_vec3(color, soot_color, soot_mask * 0.6);

    v.color = saturate_vec3(color);
}


// 🌞 Estrella / Sol: superficie caliente con granulación
pub fn sun_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Granulación en la superficie
    let motion = Vector2::new(uv.x * 20.0, uv.y * 20.0);
    let granulation = fbm(motion); // 0..1

    let hot_core = Vector3::new(1.0, 0.95, 0.6);
    let hot_edges = Vector3::new(1.0, 0.7, 0.15);
    let mut color = mix_vec3(hot_edges, hot_core, granulation);

    // Oscurecer un poco hacia el borde de la esfera (limb darkening)
    let facing = clamp(n.z * 0.5 + 0.5, 0.0, 1.0);
    let intensity = mix(0.7, 1.4, facing);
    color = Vector3::new(color.x * intensity, color.y * intensity, color.z * intensity);

    v.color = saturate_vec3(color);
}

// 🪨 Planeta rocoso tipo "galleta": placas grandes anaranjadas con bordes oscuros y cráteres
pub fn rocky_planet_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Escala de las "placas" en el planeta (pocas, grandes y redondeadas)
    let plate_uv = Vector2::new(uv.x * 6.0, uv.y * 4.0);

    // Celda entera y coordenadas locales dentro de la celda
    let cell = Vector2::new(plate_uv.x.floor(), plate_uv.y.floor());
    let local = Vector2::new(plate_uv.x.fract(), plate_uv.y.fract());

    // Centro pseudo-aleatorio de la placa dentro de la celda
    let jitter_x = hash2(cell) * 0.3 - 0.15;
    let jitter_y = hash2(Vector2::new(cell.x + 23.0, cell.y + 7.0)) * 0.3 - 0.15;
    let center = Vector2::new(0.5 + jitter_x, 0.5 + jitter_y);

    // Distancia al centro de la placa en coordenadas locales
    let dx = local.x - center.x;
    let dy = local.y - center.y;
    let dist = (dx * dx + dy * dy).sqrt();

    // Radio base de la placa y grosor del borde
    let plate_radius = 0.55;
    let edge_width = 0.06;

    // Máscara interior de la placa (1 = dentro de la placa)
    let plate_mask = smoothstep(plate_radius, plate_radius - edge_width * 1.5, dist);
    // Máscara de borde (anillo delgado alrededor de la placa)
    let edge_inner = smoothstep(plate_radius - edge_width * 0.4, plate_radius - edge_width * 1.4, dist);
    let edge_outer = smoothstep(plate_radius + edge_width * 0.3, plate_radius - edge_width * 0.2, dist);
    let edge_ring = clamp(edge_outer - edge_inner, 0.0, 1.0);

    // Colores base de la roca estilo cartoon
    let plate_light = Vector3::new(0.96, 0.78, 0.54); // beige anaranjado claro
    let plate_mid   = Vector3::new(0.88, 0.65, 0.42); // naranja suave
    let plate_dark  = Vector3::new(0.60, 0.37, 0.22); // marrón rojizo
    let gap_color   = Vector3::new(0.22, 0.10, 0.08); // color entre placas (grieta oscura)

    // Variación de color por placa usando ruido basado en la celda
    let plate_noise = hash2(Vector2::new(cell.x + 11.0, cell.y + 19.0));
    let plate_t = clamp(plate_noise * 1.2, 0.0, 1.0);
    let mut plate_color = mix_vec3(plate_mid, plate_light, plate_t);

    // Sombreado suave dentro de la placa (más claro en el centro)
    let center_shade = 1.0 - clamp(dist / (plate_radius + 0.1), 0.0, 1.0);
    let shade_factor = mix(0.85, 1.15, center_shade);
    plate_color = Vector3::new(
        plate_color.x * shade_factor,
        plate_color.y * shade_factor,
        plate_color.z * shade_factor,
    );

    // Color base: mezcla entre la grieta y la placa
    let mut color = mix_vec3(gap_color, plate_color, plate_mask);

    // Borde oscuro entre placas
    let edge_color = Vector3::new(0.15, 0.07, 0.05);
    color = mix_vec3(color, edge_color, edge_ring * 0.9);

    // --------------------------------
    // Cráteres pequeños dentro de las placas
    // --------------------------------
    // Usamos otra capa de ruido en la celda para decidir dónde hay cráteres
    let crater_seed = hash2(Vector2::new(cell.x + 5.0, cell.y + 37.0));

    // Solo generamos cráteres si el fragmento está dentro de la placa
    if plate_mask > 0.5 && crater_seed > 0.35 {
        // Hasta 3 posibles cráteres por celda
        for i in 0..3 {
            let offset_x = hash2(Vector2::new(cell.x + 31.0 + i as f32 * 13.0, cell.y + 17.0)) * 0.8 + 0.1;
            let offset_y = hash2(Vector2::new(cell.x + 47.0 + i as f32 * 29.0, cell.y + 3.0)) * 0.8 + 0.1;
            let crater_center = Vector2::new(offset_x, offset_y);

            let cdx = local.x - crater_center.x;
            let cdy = local.y - crater_center.y;
            let cdist = (cdx * cdx + cdy * cdy).sqrt();

            let crater_radius = 0.06 + hash2(Vector2::new(cell.x + 59.0 + i as f32 * 7.0, cell.y + 41.0)) * 0.03;
            let crater_edge = crater_radius * 1.4;

            let crater_mask = smoothstep(crater_radius, crater_radius * 0.4, cdist);
            let crater_rim  = clamp(
                smoothstep(crater_edge, crater_radius, cdist) - smoothstep(crater_radius, crater_radius * 0.6, cdist),
                0.0,
                1.0,
            );

            let crater_floor = Vector3::new(0.35, 0.20, 0.16);
            let crater_rim_color = Vector3::new(0.55, 0.32, 0.22);

            // Piso del cráter
            color = mix_vec3(color, crater_floor, crater_mask * 0.85);
            // Borde algo más claro alrededor
            color = mix_vec3(color, crater_rim_color, crater_rim * 0.8);
        }
    }

    v.color = saturate_vec3(color);
}

// 🪐 Gigante gaseoso: bandas y gran mancha
pub fn gassy_planet_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Capa 1: bandas latitudinales suavizadas
    let band_freq = 14.0;
    let base_bands = (uv.y * band_freq).sin() * 0.5 + 0.5; // 0..1
    let band_light = Vector3::new(0.9, 0.8, 0.65);
    let band_dark = Vector3::new(0.5, 0.4, 0.3);
    let mut color = mix_vec3(band_dark, band_light, base_bands);

    // Capa 2: ruido para romper las bandas perfectas
    let swirl = fbm(Vector2::new(uv.x * 6.0, uv.y * 10.0));
    let swirl_intensity = mix(0.8, 1.2, swirl);
    color = Vector3::new(color.x * swirl_intensity, color.y * swirl_intensity, color.z * swirl_intensity);

    // Capa 3: segunda frecuencia de bandas
    let band2 = (uv.y * band_freq * 2.5 + uv.x * 2.0).sin() * 0.5 + 0.5;
    let extra = mix_vec3(band_dark, band_light, band2);
    color = mix_vec3(color, extra, 0.3);

    // Capa 4: \"gran mancha\" tipo Júpiter
    let spot_center = Vector2::new(0.25, 0.55);
    let dx = uv.x - spot_center.x;
    let dy = uv.y - spot_center.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let spot_mask = smoothstep(0.22, 0.0, dist); // 1 en el centro, 0 afuera
    let spot_color = Vector3::new(1.0, 0.6, 0.3);
    color = mix_vec3(color, spot_color, spot_mask * 0.9);

    v.color = saturate_vec3(color);
}

// 🌑 Luna: gris con cráteres
pub fn moon_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    let rough = fbm(Vector2::new(uv.x * 6.0, uv.y * 6.0));

    let base_grey = Vector3::new(0.7, 0.7, 0.7);
    let dark_grey = Vector3::new(0.3, 0.3, 0.35);
    let mut color = mix_vec3(dark_grey, base_grey, rough);

    // Celdas para cráteres
    let cell = Vector2::new((uv.x * 16.0).floor(), (uv.y * 8.0).floor());
    let crater_noise = hash2(cell);

    if crater_noise > 0.8 {
        // Cráter profundo
        color = mix_vec3(color, dark_grey, 0.8);
    } else if crater_noise > 0.65 {
        // Cráter más suave
        color = mix_vec3(color, dark_grey, 0.5);
    }

    v.color = saturate_vec3(color);
}

// 💿 Anillo: disco con bandas concéntricas
pub fn ring_vertex_shader(v: &mut Vertex) {
    // Suponemos que el anillo está en el plano XZ centrado en el origen en espacio modelo.
    let x = v.position.x;
    let z = v.position.z;
    let r = (x * x + z * z).sqrt();

    // Normalizar radio aproximado a 0..1 con una constante (ajusta si tu modelo es distinto)
    let t = clamp(r * 0.02, 0.0, 1.0);

    let base_inner = Vector3::new(0.95, 0.9, 0.8);
    let base_outer = Vector3::new(0.6, 0.55, 0.5);
    let mut color = mix_vec3(base_inner, base_outer, t);

    // Bandas concéntricas finas usando el radio
    let band1 = (r * 35.0).sin() * 0.5 + 0.5;
    let band2 = (r * 70.0).sin() * 0.5 + 0.5;
    let band_mix = 0.6 * band1 + 0.4 * band2;

    let bright = Vector3::new(1.0, 0.95, 0.9);
    let dark = Vector3::new(0.4, 0.37, 0.33);
    let band_color = mix_vec3(dark, bright, band_mix);

    color = mix_vec3(color, band_color, 0.7);

    // Un poquito de variación angular para que no sean círculos perfectos
    let angle = z.atan2(x); // [-pi, pi]
    let angle_noise = (angle * 10.0).sin() * 0.5 + 0.5;
    color = mix_vec3(color, bright, angle_noise * 0.15);

    v.color = saturate_vec3(color);
}

// 🌍 Planeta Tierra: océanos, continentes, desiertos, polos de hielo y nubes
pub fn earth_planet_vertex_shader(v: &mut Vertex){
    let n = normalize3(v.normal);
    let uv = spherical_uv(n); // uv.x = longitud, uv.y = latitud mapeada

    // ------------------------
    // Capa 1: Océanos
    // ------------------------
    let ocean_noise = fbm(Vector2::new(uv.x * 8.0, uv.y * 8.0)); // detalle fino
    let ocean_deep   = Vector3::new(0.02, 0.08, 0.25); // azul profundo
    let ocean_shallow= Vector3::new(0.00, 0.35, 0.60); // azul más claro / turquesa
    let mut base_color = mix_vec3(ocean_deep, ocean_shallow, ocean_noise);

    // ------------------------
    // Capa 2: Continentes (máscara de tierra)
    // ------------------------
    // Ruido de baja frecuencia para dibujar "continentes"
    let continents = fbm(Vector2::new(uv.x * 3.0 + 10.0, uv.y * 3.0 + 5.0));

    // Hacer una transición suave alrededor del umbral
    let land_mask = smoothstep(0.50, 0.55, continents); // 0 = agua, 1 = tierra

    // ------------------------
    // Capa 3: Tipos de terreno (selva, zonas templadas, desierto)
    // ------------------------
    // latitud aprox: 0 en ecuador, 1 en polos
    let lat = (uv.y - 0.5).abs() * 2.0; 
    let lat_clamped = clamp(lat, 0.0, 1.0);

    // Colores base de tierra
    let tropical     = Vector3::new(0.02, 0.35, 0.05);  // verde muy saturado
    let temperate    = Vector3::new(0.15, 0.40, 0.10);  // verde más suave
    let desert       = Vector3::new(0.75, 0.65, 0.40);  // arena
    let tundra       = Vector3::new(0.60, 0.60, 0.55);  // grisáceo / rocoso

    // Bandas climáticas aproximadas según latitud
    let land_color = if lat_clamped < 0.25 {
        // Zona ecuatorial: mezcla selva + algo de desierto
        let mix_desert = fbm(Vector2::new(uv.x * 6.0, uv.y * 6.0));
        mix_vec3(tropical, desert, mix_desert * 0.4)
    } else if lat_clamped < 0.55 {
        // Zonas templadas
        mix_vec3(temperate, tropical, fbm(Vector2::new(uv.x * 4.0, uv.y * 4.0)))
    } else if lat_clamped < 0.80 {
        // Transición a tundra
        mix_vec3(temperate, tundra, fbm(Vector2::new(uv.x * 4.0, uv.y * 8.0)))
    } else {
        // Muy cercano a los polos, dejamos que la nieve domine en la siguiente capa
        tundra
    };

    // Mezclar océanos y tierra según land_mask
    base_color = mix_vec3(base_color, land_color, land_mask);

    // ------------------------
    // Capa 4: Polos de hielo
    // ------------------------
    let pole_factor = clamp(n.y.abs(), 0.0, 1.0); // |y| grande en los polos
    let ice_mask = smoothstep(0.55, 0.80, pole_factor); // 0 = sin hielo, 1 = hielo sólido
    let ice_color = Vector3::new(0.95, 0.98, 1.0);

    base_color = mix_vec3(base_color, ice_color, ice_mask);

    // ------------------------
    // Capa 5: Nubes
    // ------------------------
    // Ruido más de alta frecuencia para nubes
    let cloud_noise = fbm(Vector2::new(uv.x * 12.0 + 20.0, uv.y * 12.0 + 30.0));
    let cloud_mask = smoothstep(0.70, 0.88, cloud_noise); // zonas donde hay nubes

    let cloud_color = Vector3::new(1.0, 1.0, 1.0);
    // Mezclar nubes con el color base (las nubes se ven como velos blancos)
    base_color = mix_vec3(base_color, cloud_color, cloud_mask * 0.55);

    // ------------------------
    // Ajuste final: un poco de "limb darkening"
    // ------------------------
    let facing = clamp(n.z * 0.5 + 0.5, 0.0, 1.0);
    let brightness = mix(0.8, 1.2, facing);
    let final_color = Vector3::new(
        base_color.x * brightness,
        base_color.y * brightness,
        base_color.z * brightness,
    );

    v.color = saturate_vec3(final_color);
}


// 🚀 Shuttle shader: mint hull with dark accents and light-grey panels
pub fn shuttle_vertex_shader(v: &mut Vertex) {
    let n = normalize3(v.normal);
    let uv = spherical_uv(n);

    // Palette (converted from hex):
    // #b3deb8 (mint), #231f20 (almost black), #d1d3d8 (light grey)
    let mint = Vector3::new(0.702, 0.871, 0.722);
    let dark = Vector3::new(0.137, 0.125, 0.126);
    let light = Vector3::new(0.819, 0.827, 0.847);

    // ------------------------
    // Capa 1: casco base color menta
    // ------------------------
    let mut color = mint;

    // ------------------------
    // Capa 2: paneles del casco (rectángulos suaves)
    // ------------------------
    // Usamos un grid sobre uv para simular paneles
    let grid = Vector2::new(uv.x * 10.0, uv.y * 4.0);
    let cell = Vector2::new(grid.x.floor(), grid.y.floor());
    let local = Vector2::new(grid.x.fract(), grid.y.fract());

    // Distancia al borde de la celda (para panel outlines)
    let edge_dist = local.x
        .min(local.y)
        .min(1.0 - local.x)
        .min(1.0 - local.y);
    let edge = smoothstep(0.08, 0.02, edge_dist); // 1 cerca del borde, 0 en el centro

    // Color base de panel (mint mezclado con grey)
    let panel_color = mix_vec3(mint, light, 0.35);
    color = mix_vec3(color, panel_color, 0.8);

    // Borde más oscuro de los paneles
    let edge_color = mix_vec3(light, dark, 0.6);
    color = mix_vec3(color, edge_color, edge * 0.9);

    // ------------------------
    // Capa 3: franja oscura en la panza y la nariz
    // ------------------------
    // Panza: parte con normal hacia abajo
    let underside = clamp(-n.y * 0.7 + 0.3, 0.0, 1.0);
    // Nariz: área delantera (suponiendo eje Z como frente)
    let nose = clamp((n.z - 0.1) * 1.8, 0.0, 1.0);
    let hull_shadow = underside.max(nose);

    color = mix_vec3(color, dark, hull_shadow * 0.7);

    // ------------------------
    // Capa 4: "ventanas" o detalles oscuros
    // ------------------------
    let window_seed = hash2(cell);

    if window_seed > 0.55 && underside < 0.4 {
        // Posición pseudo-aleatoria de una ventana en la celda
        let w_cx = 0.25 + hash2(Vector2::new(cell.x + 13.0, cell.y + 5.0)) * 0.5;
        let w_cy = 0.35 + hash2(Vector2::new(cell.x + 31.0, cell.y + 9.0)) * 0.25;

        let dx = local.x - w_cx;
        let dy = local.y - w_cy;
        let wdist = dx.abs().max(dy.abs()); // ventana rectangular

        let win_mask = smoothstep(0.13, 0.07, wdist);
        let window_color = mix_vec3(dark, light, 0.2); // vidrio oscuro

        color = mix_vec3(color, window_color, win_mask);
    }

    // ------------------------
    // Capa 5: sombreado por facing (ligero)
    // ------------------------
    let facing = clamp(n.z * 0.5 + 0.5, 0.0, 1.0);
    let brightness = mix(0.85, 1.10, facing);
    color = Vector3::new(color.x * brightness, color.y * brightness, color.z * brightness);

    v.color = saturate_vec3(color);
}