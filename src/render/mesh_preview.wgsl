struct ViewportUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    base_color: vec4<f32>,
    surface: vec4<f32>,
    pigment_detail: vec4<f32>,
    pore_controls: vec4<f32>,
    pore_variation: vec4<f32>,
    grain_controls: vec4<f32>,
    regional_a: vec4<f32>,
    regional_b: vec4<f32>,
    optical_controls: vec4<f32>,
    coordinate_controls: vec4<f32>,
    reference_controls: vec4<f32>,
    reference_aux: vec4<f32>,
    skin_line_controls: vec4<f32>,
    skin_line_aux: vec4<f32>,
    color_controls: vec4<f32>,
    color_scales: vec4<f32>,
    chromatic_controls: vec4<f32>,
    chromatic_scales: vec4<f32>,
    mottled_controls: vec4<f32>,
    albedo_controls: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ViewportUniforms;

// The source add-on generates this as a linear Non-Color height image and samples
// it with Repeat addressing and box/triplanar projection.
@group(1) @binding(0)
var pore_height_texture: texture_2d<f32>;
@group(1) @binding(1)
var pore_height_sampler: sampler;

@group(2) @binding(0)
var reference_height_texture: texture_2d<f32>;
@group(2) @binding(1)
var reference_height_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) skin_position: vec3<f32>,
    @location(3) region_position: vec3<f32>,
    @location(4) uv_data: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) skin_position: vec3<f32>,
    @location(3) region_position: vec3<f32>,
    @location(4) uv_data: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = u.view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = input.position;
    output.world_normal = normalize(input.normal);
    output.skin_position = input.skin_position;
    output.region_position = input.region_position;
    output.uv_data = input.uv_data;
    return output;
}

// Stable, trigonometry-free hash used for continuous value noise and cellular
// feature points. Cell IDs are deterministic across frames and GPU vendors.
fn hash3(p: vec3<f32>) -> f32 {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += vec3<f32>(dot(p3, p3.yzx + vec3<f32>(33.33)));
    return fract((p3.x + p3.y) * p3.z);
}

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

fn value_noise_color(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        value_noise3(p + vec3<f32>(17.0, 3.0, 29.0)),
        value_noise3(p + vec3<f32>(41.0, 13.0, 5.0)),
        value_noise3(p + vec3<f32>(7.0, 37.0, 19.0))
    );
}

// 3D Worley/Voronoi F1. The original Blender node evaluates a 3D cell lattice;
// checking the surrounding 27 cells avoids visible repetition and box-shaped
// artifacts. All three pore populations use this same deterministic field.
fn voronoi_f1_3d(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let local = fract(p);
    var nearest = 2.0;
    for (var z = -1; z <= 1; z = z + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            for (var x = -1; x <= 1; x = x + 1) {
                let neighbor = vec3<f32>(f32(x), f32(y), f32(z));
                let feature_cell = cell + neighbor;
                let jitter = vec3<f32>(
                    hash3(feature_cell + vec3<f32>(17.0, 53.0, 11.0)),
                    hash3(feature_cell + vec3<f32>(29.0, 7.0, 43.0)),
                    hash3(feature_cell + vec3<f32>(3.0, 31.0, 61.0))
                );
                let feature = neighbor + jitter;
                nearest = min(nearest, length(local - feature));
            }
        }
    }
    return nearest;
}

// Approximation of ShaderNodeTexVoronoi F1 + DISTANCE_TO_EDGE + Color.
// The nearest and second-nearest feature points produce a signed-cell identity
// and a distance to their bisector. This keeps cellular fields irregular while
// avoiding the hard voxel boundaries of quantized hash noise.
fn voronoi_f1_edge_3d(p: vec3<f32>) -> vec4<f32> {
    let cell = floor(p);
    let local = fract(p);
    var nearest = 2.0;
    var second = 2.0;
    var nearest_delta = vec3<f32>(0.0);
    var second_delta = vec3<f32>(0.0);
    var cell_id = 0.0;

    for (var z = -1; z <= 1; z = z + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            for (var x = -1; x <= 1; x = x + 1) {
                let neighbor = vec3<f32>(f32(x), f32(y), f32(z));
                let feature_cell = cell + neighbor;
                let jitter = vec3<f32>(
                    hash3(feature_cell + vec3<f32>(17.0, 53.0, 11.0)),
                    hash3(feature_cell + vec3<f32>(29.0, 7.0, 43.0)),
                    hash3(feature_cell + vec3<f32>(3.0, 31.0, 61.0))
                );
                let delta = local - (neighbor + jitter);
                let distance_to_feature = length(delta);
                if (distance_to_feature < nearest) {
                    second = nearest;
                    second_delta = nearest_delta;
                    nearest = distance_to_feature;
                    nearest_delta = delta;
                    cell_id = hash3(feature_cell + vec3<f32>(71.0, 19.0, 37.0));
                } else if (distance_to_feature < second) {
                    second = distance_to_feature;
                    second_delta = delta;
                }
            }
        }
    }

    let bisector_distance = abs(second * second - nearest * nearest)
        / max(2.0 * length(second_delta - nearest_delta), 0.0001);
    return vec4<f32>(nearest, bisector_distance, cell_id, second);
}

// Source build_color_layer(): three warped Voronoi pigmentation populations.
// Each population has a soft distance ramp, a cell-density gate and its own tint.
fn mottled_spot_mask(
    p: vec3<f32>,
    scale: f32,
    variation: f32,
    contrast: f32,
    salt: vec3<f32>
) -> f32 {
    let warp_amount = mix(0.02, 0.28, clamp(variation, 0.0, 1.0));
    let warp = value_noise_color(p * 2.4 + salt) * warp_amount;
    let warped = p + warp;
    let distance_to_cell = voronoi_f1_3d(warped * scale);
    let core_ramp = smoothstep(0.10, 0.42, distance_to_cell);
    let exponent = mix(0.45, 2.6, clamp(contrast, 0.0, 1.0));
    let shaped = pow(max(core_ramp, 0.0), exponent);
    let cell_id = hash3(floor(warped * scale) + salt + vec3<f32>(31.0, 17.0, 47.0));
    let density_threshold = mix(0.78, 0.32, clamp(variation, 0.0, 1.0));
    return shaped * step(density_threshold, cell_id);
}

// Box/triplanar projection mirrors the source's generated height image node.
// Coordinates are scaled in 0.22 m reference-head units, independent of UVs.
fn sample_pore_height(p: vec3<f32>, n: vec3<f32>, scale: f32) -> f32 {
    let weights_raw = pow(abs(n), vec3<f32>(4.0));
    let weights = weights_raw / max(weights_raw.x + weights_raw.y + weights_raw.z, 0.0001);
    let h_x = textureSample(pore_height_texture, pore_height_sampler, p.yz * scale).r;
    let h_y = textureSample(pore_height_texture, pore_height_sampler, p.xz * scale).r;
    let h_z = textureSample(pore_height_texture, pore_height_sampler, p.xy * scale).r;
    return h_x * weights.x + h_y * weights.y + h_z * weights.z;
}

// Reconstruct a surface gradient from screen derivatives, then perturb the normal.
// This is a shader-side equivalent of chaining Blender Bump nodes, not displacement.
fn sample_reference_height(p: vec3<f32>, n: vec3<f32>, scale: f32) -> f32 {
    let weights_raw = pow(abs(n), vec3<f32>(4.0));
    let weights = weights_raw / max(weights_raw.x + weights_raw.y + weights_raw.z, 0.0001);
    let h_x = textureSample(reference_height_texture, reference_height_sampler, p.yz * scale).r;
    let h_y = textureSample(reference_height_texture, reference_height_sampler, p.xz * scale).r;
    let h_z = textureSample(reference_height_texture, reference_height_sampler, p.xy * scale).r;
    return h_x * weights.x + h_y * weights.y + h_z * weights.z;
}

fn bump_from_height(
    normal: vec3<f32>,
    metric_position: vec3<f32>,
    height: f32,
    distance: f32,
    strength: f32
) -> vec3<f32> {
    let position_dx = dpdx(metric_position);
    let position_dy = dpdy(metric_position);
    let height_dx = dpdx(height);
    let height_dy = dpdy(height);
    let r1 = cross(position_dy, normal);
    let r2 = cross(normal, position_dx);
    let determinant = dot(position_dx, r1);
    let gradient = sign(determinant)
        * (height_dx * r1 + height_dy * r2)
        / max(abs(determinant), 0.0000001);
    return normalize(normal - gradient * distance * strength);
}

fn gaussian(value: f32, center: f32, radius: f32) -> f32 {
    let d = abs(value - center) / max(radius, 0.00001);
    return exp(-(d * d) * 2.2);
}

struct RegionDetail {
    large_pores: f32,
    primary_pores: f32,
    fine_pores: f32,
    visibility: f32,
    hemoglobin: f32,
    eye_area: f32,
};

// Port of _region_weight() / REGIONAL_RELIEF_TABLE from __init__.py. Imported
// OBJ/glTF files do not carry Blender vertex-group weights reliably, so this
// uses bbox-normalized head heuristics. The artist can adjust the weights in UI.
fn region_detail(c: vec3<f32>) -> RegionDetail {
    let x = c.x;
    let z = c.z;
    let front = 1.0 - c.y;
    let nose = gaussian(x, 0.5, 0.12) * gaussian(z, 0.50, 0.13) * gaussian(front, 0.90, 0.22);
    let cheeks = max(gaussian(x, 0.27, 0.18), gaussian(x, 0.73, 0.18))
        * gaussian(z, 0.58, 0.17) * gaussian(front, 0.80, 0.28);
    let ears = max(gaussian(x, 0.03, 0.12), gaussian(x, 0.97, 0.12))
        * gaussian(z, 0.66, 0.20) * gaussian(front, 0.45, 0.45);
    let forehead = gaussian(x, 0.5, 0.42) * gaussian(z, 0.80, 0.16) * gaussian(front, 0.75, 0.32);
    let chin = gaussian(x, 0.5, 0.28) * gaussian(z, 0.25, 0.13) * gaussian(front, 0.75, 0.35);
    let eyes = max(gaussian(x, 0.34, 0.13), gaussian(x, 0.66, 0.13))
        * gaussian(z, 0.66, 0.11) * gaussian(front, 0.82, 0.24);
    let neck = gaussian(x, 0.5, 0.44) * clamp((0.34 - z) / 0.20, 0.0, 1.0);

    let regional_scale = clamp(u.regional_a.x, 0.0, 1.0);
    let n = nose * regional_scale;
    let c2 = cheeks * regional_scale;
    let f = forehead * regional_scale;
    let ch = chin * regional_scale;
    let e = ears * regional_scale;
    let ey = eyes * regional_scale;
    let ne = neck * regional_scale;
    let sum = n + c2 + f + ch + e + ey + ne;
    let cover = clamp(sum, 0.0, 1.0);
    let safe_sum = max(sum, 0.0001);

    let large_avg = (
        n * 1.00 + c2 * 0.20 + f * 0.00 + ch * 0.45 + e * 0.00 + ey * 0.00 + ne * 0.00
    ) / safe_sum;
    let primary_avg = (
        n * 0.85 + c2 * 1.00 + f * 0.55 + ch * 1.00 + e * 0.60 + ey * 0.20 + ne * 0.45
    ) / safe_sum;
    let fine_avg = (
        n * 0.40 + c2 * 0.70 + f * 1.00 + ch * 0.60 + e * 0.80 + ey * 0.90 + ne * 1.00
    ) / safe_sum;
    let visibility_avg = (
        n * u.regional_a.y + c2 * u.regional_a.z + f * u.regional_a.w
        + ch * u.regional_b.x + e * u.regional_b.y + ey * u.regional_b.z + ne * u.regional_b.w
    ) / safe_sum;

    var result: RegionDetail;
    result.large_pores = large_avg * cover;
    result.primary_pores = primary_avg * cover + (1.0 - cover);
    result.fine_pores = fine_avg * cover + (1.0 - cover);
    result.visibility = visibility_avg * cover + (1.0 - cover);
    result.hemoglobin = clamp(max(max(nose, c2), max(e, ey)), 0.0, 1.0);
    result.eye_area = clamp(eyes, 0.0, 1.0);
    return result;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    // Coordinate conversion from __init__.py: Object * (meters per unit / 0.22 m).
    // This keeps pores at a stable physical size regardless of viewport framing.
    let skin_position = input.skin_position
        * (clamp(u.coordinate_controls.x, 0.00001, 1000.0) / 0.22);
    let light_direction = normalize(vec3<f32>(-0.48, 0.68, 0.85));
    let view_direction = normalize(u.camera_position.xyz - input.world_position);
    let half_vector = normalize(light_direction + view_direction);

    let pore_strength = clamp(u.base_color.w, 0.0, 2.0);
    let pore_scale = clamp(u.pore_controls.x, 0.25, 4.0);
    let pore_micro_scale = clamp(u.pore_controls.w, 40.0, 320.0) * pore_scale;
    let pore_irregularity = clamp(u.pore_variation.x, 0.0, 1.0);
    let pore_size_variation = clamp(u.pore_variation.y, 0.0, 1.0);
    let regions = region_detail(input.region_position);

    // Original node chain: Noise(Color) * 0.055 * Pore Irregularity, added to
    // shared object-space skin coordinates before the three Voronoi populations.
    let warp_color = value_noise_color(skin_position * 9.0);
    let warped_coord = skin_position + warp_color * (0.055 * pore_irregularity);

    let primary_distance = voronoi_f1_3d(warped_coord * pore_micro_scale);
    let fine_distance = voronoi_f1_3d(warped_coord * pore_micro_scale * 1.72);
    let large_distance = voronoi_f1_3d(warped_coord * pore_micro_scale * 0.60);
    let primary_opening = 1.0 - smoothstep(0.12, 0.235, primary_distance);
    let fine_opening = 1.0 - smoothstep(0.075, 0.155, fine_distance);
    let large_opening = 1.0 - smoothstep(0.12, 0.235, large_distance);

    // Match the source's three regional populations and MAXIMUM mix.
    let primary = primary_opening * regions.primary_pores;
    let fine = fine_opening * regions.fine_pores;
    let large = large_opening * regions.large_pores;
    let pore_population = max(max(primary, fine), large);

    let size_noise = value_noise3(skin_position * 4.8);
    let size_factor = smoothstep(0.30, 0.70, size_noise) * pore_size_variation;
    let pore_shape = pore_population * size_factor;
    let pore_shape_master = pore_shape * pore_strength;
    let pore_color_mask = pore_shape_master * clamp(u.pore_controls.y, 0.0, 1.0);
    // Specular occlusion follows the complete pore opening mask independently
    // from albedo darkening, matching the source add-on's separate controls.
    let pore_specular_mask = pore_shape_master;

    // Source's generated tileable height map (six tiles per 0.22 m at Pore Scale 1).
    // The image map and procedural openings remain separate bump contributions.
    let map_height = sample_pore_height(skin_position, normal, 6.0 * pore_scale);
    let metric_position = skin_position * 0.22;
    let image_strength = clamp(pore_strength, 0.0, 1.0) * regions.visibility;
    let image_distance = 0.00024 * max(pore_strength, 1.0);
    let image_normal = bump_from_height(
        normal, metric_position, map_height, image_distance, image_strength
    );

    // Blender DETAIL_ProceduralPoreBump uses Distance=0.009. Pore Micro Detail
    // is its strength; the mask also controls pore albedo and specular occlusion.
    let pore_micro_detail = clamp(u.pore_controls.z, 0.0, 1.0) * regions.visibility;
    let procedural_normal = bump_from_height(
        image_normal, metric_position, pore_shape_master, 0.009, pore_micro_detail
    );

    // Fine Detail and Micro Grain reproduce the two separate source Bump nodes;
    // they remain lower amplitude than the shape-defining pore layer.
    let fine_height = value_noise3(skin_position * clamp(u.pore_variation.w, 10.0, 500.0));
    let fine_normal = bump_from_height(
        procedural_normal, metric_position, fine_height, 0.022,
        clamp(u.pore_variation.z, 0.0, 0.5)
    );
    let grain_height = value_noise3(skin_position * clamp(u.grain_controls.y, 50.0, 1000.0));
    let micro_normal = bump_from_height(
        fine_normal, metric_position, grain_height, 0.008,
        clamp(u.grain_controls.x, 0.0, 0.20)
    );

    // Skin Line Network, ported from NET_TUNING / build_detail_layer():
    // smooth cellular plateaus, curved sulci, two line scales and masked creases.
    // This layer is distinct from pore openings and directional crosshatch grooves.
    let line_strength = clamp(u.skin_line_controls.x, 0.0, 1.5);
    let line_scale = clamp(u.skin_line_controls.y, 4.0, 400.0);
    let line_thickness = clamp(u.skin_line_controls.z, 0.05, 1.0);
    let line_relief = clamp(u.skin_line_controls.w, 0.1, 8.0);
    let network_warp = vec3<f32>(
        value_noise3(skin_position * 5.5 + vec3<f32>(1.0, 0.0, 0.0)),
        value_noise3(skin_position * 5.5 + vec3<f32>(0.0, 1.0, 1.0)),
        0.0
    ) * 0.045;
    let network_coord = skin_position + network_warp;

    let primary_network = voronoi_f1_edge_3d(network_coord * line_scale);
    let secondary_network = voronoi_f1_edge_3d(network_coord * line_scale * 2.35);
    let crease_network = voronoi_f1_edge_3d(network_coord * (line_scale / 5.68));
    let shoulder_width = line_thickness * 1.6;
    let plateau = smoothstep(0.0, max(shoulder_width, 0.0001), primary_network.y);
    let dome = 1.0 - 0.6 * primary_network.x * primary_network.x;
    let cell_tilt = primary_network.z * 0.08;
    let crease_mask_noise = value_noise3(network_coord * 6.0);
    let crease_mask = smoothstep(0.50, 0.66, crease_mask_noise);
    let crease_groove = 1.0 - smoothstep(0.0, 0.056, crease_network.y);
    let network_height = plateau * 1.3 + dome * 0.35 + cell_tilt
        - crease_groove * crease_mask
        + (value_noise3(network_coord * line_scale * 3.0) - 0.5) * 1.2;

    let primary_line = 1.0 - smoothstep(0.0, max(line_thickness * 0.18, 0.0001), primary_network.y);
    let secondary_line = (1.0 - smoothstep(
        0.0, max(line_thickness * 0.18, 0.0001), secondary_network.y
    )) * 0.55;
    let network_lines = max(primary_line, secondary_line);
    let network_region = network_lines * regions.visibility;
    let network_shading = network_region * clamp(u.skin_line_aux.x, 0.0, 1.0);
    let network_normal = bump_from_height(
        micro_normal,
        metric_position,
        network_height,
        line_relief * 0.00015 * (190.0 / line_scale),
        clamp(line_strength * regions.visibility, 0.0, 1.0)
    );

    // Independent tileable Reference Height layer, ported from _ref_skin_height().
    // Reference strength is zero by default and is not coupled to Pore Strength.
    let reference_height = sample_reference_height(
        skin_position, normal, clamp(u.reference_controls.z, 0.001, 1000.0)
    );
    let reference_cavity = 1.0 - smoothstep(0.32, 0.47, reference_height);
    let reference_region = mix(
        1.0,
        clamp(regions.visibility, 0.0, 1.6),
        clamp(u.reference_aux.z, 0.0, 1.0)
    );
    let perturbed_normal = bump_from_height(
        network_normal,
        metric_position,
        reference_height,
        max(u.reference_controls.y, 0.0),
        clamp(u.reference_controls.x * reference_region, 0.0, 1.0)
    );

    let ndotl = max(dot(perturbed_normal, light_direction), 0.0);
    let diffuse = 0.25 + ndotl * 0.75;
    let oiliness = clamp(u.pigment_detail.z, 0.0, 1.0);
    let roughness = clamp(
        mix(u.surface.x, u.surface.x * 0.42, oiliness)
        + (grain_height - 0.5) * clamp(u.grain_controls.z, 0.0, 0.15)
        + (reference_cavity - 0.2) * clamp(u.reference_aux.y, 0.0, 0.6),
        0.035,
        1.0
    );
    let specular_power = mix(180.0, 5.0, roughness);
    let specular_strength = clamp(u.surface.y + oiliness * 0.35, 0.0, 1.0)
        * (1.0 - clamp(pore_specular_mask * u.optical_controls.x, 0.0, 0.9));
    let specular = pow(max(dot(perturbed_normal, half_vector), 0.0), specular_power) * specular_strength;
    let back_scatter = pow(max(dot(-perturbed_normal, light_direction), 0.0), 2.0) * u.surface.z;

    // Source build_color_layer(): broad tone, melanin, erythema, golden warmth,
    // micro-color and legacy redness, followed by three separate mottled scales.
    let broad_noise = value_noise3(skin_position * clamp(u.color_scales.x, 0.1, 30.0));
    let broad_tone = mix(
        vec3<f32>(0.78, 0.62, 0.51),
        vec3<f32>(1.12, 0.92, 0.76),
        smoothstep(0.28, 0.72, broad_noise)
    );
    var base = mix(
        u.base_color.rgb,
        u.base_color.rgb * broad_tone,
        clamp(u.color_controls.x, 0.0, 1.0)
    );

    let melanin_noise = value_noise3(skin_position * clamp(u.color_scales.z, 0.5, 20.0));
    let melanin_tone = mix(
        vec3<f32>(0.72, 0.80, 0.86),
        vec3<f32>(0.54, 0.43, 0.34),
        smoothstep(0.24, 0.76, melanin_noise)
    );
    base = mix(base, melanin_tone, clamp(u.color_controls.w, 0.0, 1.0));

    let erythema_noise = value_noise3(skin_position * clamp(u.color_scales.w, 2.0, 60.0));
    let erythema_mask = smoothstep(0.49, 0.67, erythema_noise);
    base = mix(
        base,
        vec3<f32>(1.16, 0.40, 0.34),
        clamp(erythema_mask * u.chromatic_controls.x, 0.0, 1.0)
    );

    let golden_noise = value_noise3(skin_position * clamp(u.chromatic_scales.x, 2.0, 40.0));
    let golden_tone = mix(
        vec3<f32>(0.95, 0.84, 0.72),
        vec3<f32>(1.08, 0.96, 0.72),
        golden_noise
    );
    base = mix(base, base * golden_tone, clamp(u.chromatic_controls.y, 0.0, 1.0));

    let micro_color_noise = value_noise3(skin_position * clamp(u.chromatic_scales.y, 10.0, 180.0));
    let micro_color_tone = mix(
        vec3<f32>(0.91, 0.96, 1.00),
        vec3<f32>(1.03, 0.92, 0.89),
        smoothstep(0.40, 0.60, micro_color_noise)
    );
    base = mix(base, base * micro_color_tone, clamp(u.chromatic_controls.z, 0.0, 1.0));

    let warm_tone = vec3<f32>(1.035, 0.91, 0.83);
    base = mix(base, base * warm_tone, clamp(u.color_controls.z, 0.0, 1.0));

    let redness_noise = value_noise3(skin_position * clamp(u.color_scales.y, 0.5, 60.0));
    let redness_mask = smoothstep(0.53, 0.73, redness_noise);
    base = mix(
        base,
        base * vec3<f32>(1.075, 0.94, 0.93),
        clamp(redness_mask * u.color_controls.y, 0.0, 1.0)
    );

    let mottled_variation = clamp(u.mottled_controls.z, 0.0, 1.0);
    let mottled_contrast = clamp(u.mottled_controls.w, 0.0, 1.0);
    let mottled_strength = clamp(u.mottled_controls.y, 0.0, 1.0);
    let macro_spot = mottled_spot_mask(
        skin_position, clamp(u.chromatic_scales.z, 0.4, 12.0),
        mottled_variation, mottled_contrast, vec3<f32>(11.0, 3.0, 7.0)
    ) * mottled_strength;
    let medium_spot = mottled_spot_mask(
        skin_position, clamp(u.chromatic_scales.w, 2.0, 35.0),
        mottled_variation, mottled_contrast, vec3<f32>(5.0, 17.0, 13.0)
    ) * mottled_strength;
    let micro_spot = mottled_spot_mask(
        skin_position, clamp(u.mottled_controls.x, 10.0, 120.0),
        mottled_variation, mottled_contrast, vec3<f32>(19.0, 23.0, 29.0)
    ) * mottled_strength;
    base = mix(base, base * vec3<f32>(1.030, 0.965, 0.900), macro_spot);
    base = mix(base, base * vec3<f32>(1.018, 0.965, 0.955), medium_spot);
    base = mix(base, base * vec3<f32>(0.985, 0.995, 1.018), micro_spot);

    base *= 1.0 - pore_color_mask * 0.15 - network_shading * 0.10;
    base = clamp(base, vec3<f32>(0.0), vec3<f32>(1.0));

    let blood_noise = value_noise3(skin_position * 23.0);
    let hemoglobin_mask = (0.35 + 0.65 * blood_noise) * regions.hemoglobin;
    let hemoglobin_tint = base * vec3<f32>(1.10, 0.76, 0.72);
    base = mix(base, hemoglobin_tint, clamp(u.pigment_detail.x * hemoglobin_mask * 0.22, 0.0, 1.0));

    // Freckles remain a distinct color detail rather than being confused with pores.
    let freckle_cell = floor(skin_position * 46.0);
    let freckle_local = fract(skin_position * 46.0);
    let freckle_center = vec3<f32>(
        hash3(freckle_cell + vec3<f32>(11.0, 3.0, 7.0)),
        hash3(freckle_cell + vec3<f32>(5.0, 17.0, 13.0)),
        hash3(freckle_cell + vec3<f32>(19.0, 23.0, 29.0))
    );
    let freckle_gate = step(0.955, hash3(freckle_cell + vec3<f32>(31.0, 37.0, 41.0)));
    let freckle_disk = 1.0 - smoothstep(0.065, 0.14, length(freckle_local - freckle_center));
    let freckles = freckle_gate * freckle_disk * clamp(u.pigment_detail.y, 0.0, 1.0);
    base *= 1.0 - freckles * 0.38;
    let reference_color_strength = clamp(u.reference_aux.x, 0.0, 1.0);
    let reference_warm_tint = base * vec3<f32>(1.035, 0.965, 0.91);
    base = mix(
        base,
        reference_warm_tint,
        clamp(reference_cavity * reference_color_strength, 0.0, 1.0)
    );
    base *= 1.0 - reference_cavity * reference_color_strength * 0.055;

    // Port of SKIN_ALBEDO_REALISM_LAYER using imported UVs, with a safe
    // bbox-coordinate fallback when the source mesh has no UV set.
    let albedo_uv = mix(
        input.region_position.xy,
        input.uv_data.xy,
        clamp(input.uv_data.z, 0.0, 1.0)
    );
    let albedo_position = vec3<f32>(albedo_uv, 0.371);
    let realism_strength = clamp(u.albedo_controls.x, 0.0, 1.0);

    let albedo_broad = value_noise3(albedo_position * 2.7);
    let albedo_broad_ramp = mix(
        vec3<f32>(0.92, 0.94, 0.96),
        vec3<f32>(0.995, 0.985, 0.970),
        smoothstep(0.24, 0.76, albedo_broad)
    );
    base = mix(base, base * albedo_broad_ramp, realism_strength);

    let albedo_medium = value_noise3(albedo_position * 10.5);
    let albedo_medium_ramp = mix(
        vec3<f32>(0.955, 0.965, 0.975),
        vec3<f32>(0.998, 0.992, 0.980),
        smoothstep(0.28, 0.72, albedo_medium)
    );
    base = mix(base, base * albedo_medium_ramp, realism_strength);

    let albedo_micro = value_noise3(albedo_position * 58.0);
    let albedo_micro_ramp = mix(
        vec3<f32>(0.985, 0.992, 1.000),
        vec3<f32>(1.000, 0.998, 0.990),
        smoothstep(0.34, 0.66, albedo_micro)
    );
    base = mix(
        base,
        base * albedo_micro_ramp,
        realism_strength * clamp(u.albedo_controls.w, 0.0, 1.0)
    );

    let albedo_hemo_noise = value_noise3(albedo_position * 7.5);
    let albedo_hemo_ramp = smoothstep(0.32, 0.68, albedo_hemo_noise * regions.hemoglobin);
    let albedo_luminance = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    let albedo_hemo_tint = vec3<f32>(0.66, 0.22, 0.16)
        * max(albedo_luminance, 0.0001) / 0.32;
    base = mix(
        base,
        albedo_hemo_tint,
        clamp(albedo_hemo_ramp * u.albedo_controls.y * realism_strength * 0.24, 0.0, 0.24)
    );

    let albedo_eye_noise = value_noise3(albedo_position * 5.0);
    let albedo_eye_factor = regions.eye_area * albedo_eye_noise * 0.07;
    base = mix(
        base,
        base * vec3<f32>(0.92, 0.98, 1.0),
        clamp(albedo_eye_factor * realism_strength, 0.0, 0.07)
    );

    let albedo_melanin_noise = value_noise3(albedo_position * 2.2);
    let albedo_melanin_ramp = mix(
        vec3<f32>(0.965, 0.965, 0.955),
        vec3<f32>(0.998, 0.995, 0.988),
        smoothstep(0.28, 0.72, albedo_melanin_noise)
    );
    base = mix(
        base,
        base * albedo_melanin_ramp,
        realism_strength * clamp(u.albedo_controls.z, 0.0, 1.0)
    );
    base = clamp(base, vec3<f32>(0.0), vec3<f32>(1.0));

    var color = base * diffuse;
    color += base * back_scatter * vec3<f32>(1.0, 0.48, 0.30);
    color += vec3<f32>(1.0, 0.78, 0.63) * specular;
    let fresnel = pow(1.0 - max(dot(perturbed_normal, view_direction), 0.0), 3.0);
    color += base * fresnel * 0.055;
    color = color / (vec3<f32>(1.0) + max(color, vec3<f32>(0.0)) * 0.22);
    // The sRGB render target performs the display transfer automatically.
    return vec4<f32>(color, 1.0);
}
