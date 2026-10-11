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
    // Trigonometry-free hash keeps the procedural preview affordable per pixel.
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += vec3<f32>(dot(p3, p3.yzx + vec3<f32>(33.33)));
    return fract((p3.x + p3.y) * p3.z);
}

// Trilinear interpolation removes the hard square boundaries caused by hashing
// floor(position) directly. It is used for broad pigmentation and fine grain.
fn value_noise3(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let w = f * f * (vec3<f32>(3.0) - 2.0 * f);

    let n000 = hash3(cell + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash3(cell + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash3(cell + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash3(cell + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash3(cell + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash3(cell + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash3(cell + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash3(cell + vec3<f32>(1.0, 1.0, 1.0));

    let x00 = mix(n000, n100, w.x);
    let x10 = mix(n010, n110, w.x);
    let x01 = mix(n001, n101, w.x);
    let x11 = mix(n011, n111, w.x);
    return mix(mix(x00, x10, w.y), mix(x01, x11, w.y), w.z);
}

// A jittered 3D feature-point field gives pores rounded, uneven spacing instead
// of the visible square cells produced by thresholded random values.
fn pore_field(position: vec3<f32>) -> vec4<f32> {
    let frequency = 112.0;
    let scaled = position * frequency;
    let cell = floor(scaled);
    let local = fract(scaled);
    var nearest = 2.0;
    var nearest_delta = vec3<f32>(0.0, 0.0, 0.0);

    // Only the current cell and the adjacent cell on the point's side can
    // contain the nearest feature when jitter is bounded to [0.18, 0.82].
    for (var z = 0; z <= 1; z = z + 1) {
        for (var y = 0; y <= 1; y = y + 1) {
            for (var x = 0; x <= 1; x = x + 1) {
                let neighbor = vec3<f32>(
                    select(0.0, select(-1.0, 1.0, local.x >= 0.5), x == 1),
                    select(0.0, select(-1.0, 1.0, local.y >= 0.5), y == 1),
                    select(0.0, select(-1.0, 1.0, local.z >= 0.5), z == 1)
                );
                let feature_cell = cell + neighbor;
                let jitter = vec3<f32>(
                    hash3(feature_cell + vec3<f32>(17.0, 53.0, 11.0)),
                    hash3(feature_cell + vec3<f32>(29.0, 7.0, 43.0)),
                    hash3(feature_cell + vec3<f32>(3.0, 31.0, 61.0))
                );
                let feature = neighbor + vec3<f32>(0.18) + jitter * 0.64;
                let delta = local - feature;
                let distance_to_feature = length(delta);
                if (distance_to_feature < nearest) {
                    nearest = distance_to_feature;
                    nearest_delta = delta;
                }
            }
        }
    }
    return vec4<f32>(nearest, nearest_delta);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let light_direction = normalize(vec3<f32>(-0.48, 0.68, 0.85));
    let view_direction = normalize(u.camera_position.xyz - input.world_position);
    let half_vector = normalize(light_direction + view_direction);

    // Smooth, multiscale pigmentation; unlike quantized cell noise these
    // signals transition continuously across the surface.
    let broad_variation = value_noise3(input.world_position * 10.0) - 0.5;
    let fine_variation = value_noise3(input.world_position * 68.0) - 0.5;

    let pore_info = pore_field(input.world_position);
    let pore_distance = pore_info.x;
    let pore_delta = pore_info.yzw;
    let pore_strength = clamp(u.base_color.w, 0.0, 1.0);

    // A soft, darker opening and a restrained raised rim make pores read as
    // shallow skin indentations rather than black dots or a halftone pattern.
    let pore_core = 1.0 - smoothstep(0.09, 0.34, pore_distance);
    let pore_rim = exp(-pow((pore_distance - 0.405) / 0.09, 2.0));
    let pores = pore_core * pore_strength;
    let rim = pore_rim * pore_strength;

    // Approximate a small bump response from the local pore profile. Project
    // the feature direction onto the surface tangent plane before perturbing N.
    let pore_t = clamp((pore_distance - 0.09) / 0.25, 0.0, 1.0);
    let pore_slope = 6.0 * pore_t * (1.0 - pore_t) / 0.25;
    let pore_active = step(0.09, pore_distance) * (1.0 - step(0.34, pore_distance));
    let pore_direction = normalize(pore_delta + vec3<f32>(0.00001));
    let tangent_direction = pore_direction - normal * dot(pore_direction, normal);
    let pore_bump = tangent_direction * pore_slope * pore_active * pore_strength * 0.00055 * 112.0;

    // Fine stochastic grain contributes weakly to albedo, while pore relief
    // gives the strongest close-up cue. No extra mesh tessellation is needed.
    let micro_relief = clamp(u.surface.w, 0.0, 1.0);
    let grain = fine_variation * micro_relief * 0.045;
    let perturbed_normal = normalize(normal - pore_bump);

    let ndotl = max(dot(perturbed_normal, light_direction), 0.0);
    let diffuse = 0.25 + ndotl * 0.75;
    let oiliness = clamp(u.pigment_detail.z, 0.0, 1.0);
    let roughness = clamp(mix(u.surface.x, u.surface.x * 0.42, oiliness), 0.035, 1.0);
    let specular_power = mix(180.0, 5.0, roughness);
    let specular_strength = clamp(u.surface.y + oiliness * 0.35, 0.0, 1.0);
    let specular = pow(max(dot(perturbed_normal, half_vector), 0.0), specular_power) * specular_strength;
    let back_scatter = pow(max(dot(-perturbed_normal, light_direction), 0.0), 2.0) * u.surface.z;

    let variation = clamp(u.pigment_detail.w, 0.0, 1.0);
    var pigment = 1.0 + broad_variation * (0.025 + variation * 0.10);
    pigment += fine_variation * variation * 0.035;
    pigment += grain;
    pigment -= pores * 0.30;
    pigment += rim * 0.045;
    var base = clamp(u.base_color.rgb * pigment, vec3<f32>(0.0), vec3<f32>(1.0));

    let blood_noise = value_noise3(input.world_position * 23.0);
    let hemoglobin_mask = 0.35 + 0.65 * blood_noise;
    let hemoglobin_tint = base * vec3<f32>(1.10, 0.76, 0.72);
    base = mix(base, hemoglobin_tint, clamp(u.pigment_detail.x * hemoglobin_mask * 0.22, 0.0, 1.0));

    let freckle_cell = floor(input.world_position * 46.0);
    let freckle_local = fract(input.world_position * 46.0);
    let freckle_center = vec3<f32>(
        hash3(freckle_cell + vec3<f32>(11.0, 3.0, 7.0)),
        hash3(freckle_cell + vec3<f32>(5.0, 17.0, 13.0)),
        hash3(freckle_cell + vec3<f32>(19.0, 23.0, 29.0))
    );
    let freckle_gate = step(0.955, hash3(freckle_cell + vec3<f32>(31.0, 37.0, 41.0)));
    let freckle_disk = 1.0 - smoothstep(0.065, 0.14, length(freckle_local - freckle_center));
    let freckles = freckle_gate * freckle_disk * clamp(u.pigment_detail.y, 0.0, 1.0);
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
