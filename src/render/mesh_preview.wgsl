struct ViewportUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    base_color: vec4<f32>,
    surface: vec4<f32>,
    pigment_detail: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ViewportUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = u.view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = input.position;
    output.world_normal = normalize(input.normal);
    return output;
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let light_direction = normalize(vec3<f32>(-0.48, 0.68, 0.85));
    let view_direction = normalize(u.camera_position.xyz - input.world_position);
    let half_vector = normalize(light_direction + view_direction);

    let broad_variation = hash3(floor(input.world_position * 42.0)) - 0.5;
    let fine_variation = hash3(floor(input.world_position * 97.0)) - 0.5;
    let pore_noise = hash3(floor(input.world_position * 175.0));
    let pores = smoothstep(0.72, 0.96, pore_noise) * u.base_color.w;

    let freckle_cell = floor(input.world_position * 46.0);
    let freckle_local = fract(input.world_position * 46.0);
    let freckle_center = vec3<f32>(
        hash3(freckle_cell + vec3<f32>(11.0, 3.0, 7.0)),
        hash3(freckle_cell + vec3<f32>(5.0, 17.0, 13.0)),
        hash3(freckle_cell + vec3<f32>(19.0, 23.0, 29.0))
    );
    let freckle_gate = step(0.955, hash3(freckle_cell + vec3<f32>(31.0, 37.0, 41.0)));
    let freckle_disk = 1.0 - smoothstep(0.065, 0.14, length(freckle_local - freckle_center));
    let freckles = freckle_gate * freckle_disk * u.pigment_detail.y;

    let micro_wave = vec3<f32>(
        sin(normal.y * 287.0 + normal.z * 83.0),
        cos(normal.x * 263.0 - normal.z * 91.0),
        sin(normal.x * 241.0 + normal.y * 197.0)
    );
    let perturbed_normal = normalize(
        normal + micro_wave * (u.surface.w * u.base_color.w * 0.018)
    );

    let ndotl = max(dot(perturbed_normal, light_direction), 0.0);
    let diffuse = 0.25 + ndotl * 0.75;
    let oiliness = clamp(u.pigment_detail.z, 0.0, 1.0);
    let roughness = clamp(mix(u.surface.x, u.surface.x * 0.42, oiliness), 0.035, 1.0);
    let specular_power = mix(180.0, 5.0, roughness);
    let specular_strength = clamp(u.surface.y + oiliness * 0.35, 0.0, 1.0);
    let specular = pow(max(dot(perturbed_normal, half_vector), 0.0), specular_power) * specular_strength;
    let back_scatter = pow(max(dot(-perturbed_normal, light_direction), 0.0), 2.0) * u.surface.z;

    let variation = clamp(u.pigment_detail.w, 0.0, 1.0);
    var pigment = 1.0 + broad_variation * (0.04 + variation * 0.15);
    pigment += fine_variation * variation * 0.025;
    pigment -= pores * 0.10;
    var base = clamp(u.base_color.rgb * pigment, vec3<f32>(0.0), vec3<f32>(1.0));

    let blood_noise = hash3(floor(input.world_position * 23.0));
    let hemoglobin_mask = 0.35 + 0.65 * blood_noise;
    let hemoglobin_tint = base * vec3<f32>(1.10, 0.76, 0.72);
    base = mix(base, hemoglobin_tint, clamp(u.pigment_detail.x * hemoglobin_mask * 0.22, 0.0, 1.0));
    base *= 1.0 - freckles * 0.38;

    var color = base * diffuse;
    color += base * back_scatter * vec3<f32>(1.0, 0.48, 0.30);
    color += vec3<f32>(1.0, 0.78, 0.63) * specular;
    let fresnel = pow(1.0 - max(dot(perturbed_normal, view_direction), 0.0), 3.0);
    color += base * fresnel * 0.055;
    color = color / (vec3<f32>(1.0) + max(color, vec3<f32>(0.0)) * 0.22);
    // The sRGB render target performs the display transfer automatically.
    return vec4<f32>(color, 1.0);
}
