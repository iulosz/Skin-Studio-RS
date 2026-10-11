struct PreviewUniforms {
    viewport: vec4<f32>,
    color_pores: vec4<f32>,
    surface: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: PreviewUniforms;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

@fragment
fn fs_main(@builtin(position) frag_position: vec4<f32>) -> @location(0) vec4<f32> {
    let background = vec3<f32>(0.075, 0.088, 0.105);
    let size = max(u.viewport.xy, vec2<f32>(1.0, 1.0));
    let local_pixel = frag_position.xy - u.viewport.zw;
    let uv = local_pixel / size;
    var p = uv * 2.0 - vec2<f32>(1.0, 1.0);
    p.y = -p.y;
    p.x = p.x * size.x / size.y;

    // View ray intersects a sphere, giving the first milestone a real GPU-rendered
    // material preview without depending on Blender or an external render engine.
    let ray_origin = vec3<f32>(0.0, 0.0, 2.65);
    let ray_direction = normalize(vec3<f32>(p.x, p.y, -2.25));
    let radius = 0.83;
    let b = dot(ray_origin, ray_direction);
    let c = dot(ray_origin, ray_origin) - radius * radius;
    let discriminant = b * b - c;

    if discriminant < 0.0 {
        return vec4<f32>(background, 1.0);
    }

    let t = -b - sqrt(discriminant);
    let hit = ray_origin + t * ray_direction;
    var normal = normalize(hit);
    let view_direction = normalize(-ray_direction);
    let light_direction = normalize(vec3<f32>(-0.48, 0.68, 0.85));

    // Non-repeating, multi-frequency procedural detail. It is a visual prototype,
    // not yet a replacement for the original regional anatomical shader.
    let coarse_cells = floor(normal * 42.0);
    let pore_cells = floor(normal * 175.0);
    let broad_variation = hash3(coarse_cells) - 0.5;
    let pore_noise = hash3(pore_cells);
    let pores = smoothstep(0.72, 0.96, pore_noise) * u.color_pores.w;

    let micro_wave = vec3<f32>(
        sin(normal.y * 287.0 + normal.z * 83.0),
        cos(normal.x * 263.0 - normal.z * 91.0),
        sin(normal.x * 241.0 + normal.y * 197.0)
    );
    normal = normalize(normal + micro_wave * (u.surface.w * u.color_pores.w * 0.018));

    let ndotl = max(dot(normal, light_direction), 0.0);
    let diffuse = 0.25 + ndotl * 0.75;
    let roughness = clamp(u.surface.x, 0.035, 1.0);
    let specular_power = mix(180.0, 5.0, roughness);
    let half_vector = normalize(light_direction + view_direction);
    let specular = pow(max(dot(normal, half_vector), 0.0), specular_power) * u.surface.y;
    let back_scatter = pow(max(dot(-normal, light_direction), 0.0), 2.0) * u.surface.z;

    var pigment = 1.0 + broad_variation * 0.09;
    pigment = pigment - pores * 0.10;
    let base = clamp(u.color_pores.rgb * pigment, vec3<f32>(0.0), vec3<f32>(1.0));
    var color = base * diffuse;
    color += base * back_scatter * vec3<f32>(1.0, 0.48, 0.30);
    color += vec3<f32>(1.0, 0.78, 0.63) * specular;

    // Soft Fresnel/rim contribution to help inspect material response.
    let fresnel = pow(1.0 - max(dot(normal, view_direction), 0.0), 3.0);
    color += base * fresnel * 0.055;
    color = color / (vec3<f32>(1.0) + max(color, vec3<f32>(0.0)) * 0.22);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}
