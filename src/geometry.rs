use glam::{Mat4, Vec3};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs,
    path::Path,
};

use bytemuck::{Pod, Zeroable};

const MAX_MESH_VERTICES: usize = 5_000_000;
const MAX_MESH_INDICES: usize = 15_000_000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MeshVertex {
    /// Position normalized only for viewport framing.
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Unmodified source object-space position. The shader applies the selected
    /// real-world units-per-coordinate before evaluating skin detail.
    pub skin_position: [f32; 3],
    /// Bbox-normalized coordinates used by the original add-on's head-region heuristics.
    pub region_position: [f32; 3],
    /// UV coordinates from OBJ vt or glTF TEXCOORD_0; Z=1 means UVs exist.
    /// The shader uses bbox coordinates when Z=0.
    pub uv_data: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct MeshData {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn default_sphere() -> Self {
        let segments = 48_u32;
        let rings = 32_u32;
        let mut vertices = Vec::with_capacity(((segments + 1) * (rings + 1)) as usize);
        let mut indices = Vec::with_capacity((segments * rings * 6) as usize);

        for y in 0..=rings {
            let latitude = std::f32::consts::PI * y as f32 / rings as f32;
            let sin_lat = latitude.sin();
            let cos_lat = latitude.cos();
            for x in 0..=segments {
                let longitude = std::f32::consts::TAU * x as f32 / segments as f32;
                let normal = Vec3::new(
                    sin_lat * longitude.cos(),
                    cos_lat,
                    sin_lat * longitude.sin(),
                )
                .normalize_or_zero();
                vertices.push(MeshVertex {
                    position: (normal * 0.82).to_array(),
                    normal: normal.to_array(),
                    // Treat the demo sphere as a head-sized 22 cm surface.
                    skin_position: (normal * 0.11).to_array(),
                    region_position: (normal * 0.5 + Vec3::splat(0.5)).to_array(),
                    uv_data: [x as f32 / segments as f32, y as f32 / rings as f32, 1.0],
                });
            }
        }

        for y in 0..rings {
            for x in 0..segments {
                let a = y * (segments + 1) + x;
                let b = a + segments + 1;
                indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }

        Self {
            name: "Preview Sphere".to_owned(),
            vertices,
            indices,
        }
    }
}

/// Loads a mesh for the preview. Supported formats are Wavefront OBJ and glTF/GLB.
pub fn load_mesh(path: impl AsRef<Path>) -> Result<MeshData, MeshError> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut mesh = match extension.as_str() {
        "obj" => parse_obj(
            &fs::read_to_string(path).map_err(MeshError::Io)?,
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("OBJ mesh"),
        )?,
        "gltf" | "glb" => load_gltf(path)?,
        _ => return Err(MeshError::UnsupportedFormat(extension)),
    };

    mesh.normalize_to_preview_scale()?;
    mesh.recalculate_missing_normals();
    mesh.validate()?;
    Ok(mesh)
}

impl MeshData {
    fn normalize_to_preview_scale(&mut self) -> Result<(), MeshError> {
        if self.vertices.is_empty() {
            return Err(MeshError::InvalidMesh("mesh has no vertices".to_owned()));
        }

        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for vertex in &self.vertices {
            let position = Vec3::from_array(vertex.position);
            if !position.is_finite() {
                return Err(MeshError::InvalidMesh(
                    "mesh contains a non-finite vertex".to_owned(),
                ));
            }
            min = min.min(position);
            max = max.max(position);
        }
        let center = (min + max) * 0.5;
        let extent = (max - min).max_element();
        if !extent.is_finite() || extent <= 1.0e-7 {
            return Err(MeshError::InvalidMesh(
                "mesh has zero or near-zero dimensions".to_owned(),
            ));
        }

        let scale = 1.7 / extent;
        let bounds_extent = (max - min).max(Vec3::splat(1.0e-6));
        for vertex in &mut self.vertices {
            let source_position = Vec3::from_array(vertex.position);
            // Keep the original object coordinates independent of preview framing.
            // WGSL applies the same metres-per-unit / 0.22 m conversion as the addon.
            vertex.skin_position = source_position.to_array();
            vertex.region_position = ((source_position - min) / bounds_extent).to_array();
            vertex.position = ((source_position - center) * scale).to_array();
        }
        Ok(())
    }

    fn recalculate_missing_normals(&mut self) {
        let mut accumulated = vec![Vec3::ZERO; self.vertices.len()];
        for triangle in self.indices.chunks_exact(3) {
            let a = Vec3::from_array(self.vertices[triangle[0] as usize].position);
            let b = Vec3::from_array(self.vertices[triangle[1] as usize].position);
            let c = Vec3::from_array(self.vertices[triangle[2] as usize].position);
            let face = (b - a).cross(c - a);
            if face.length_squared() <= 1.0e-16 {
                continue;
            }
            for &index in triangle {
                let normal = Vec3::from_array(self.vertices[index as usize].normal);
                if normal.length_squared() <= 1.0e-12 {
                    accumulated[index as usize] += face;
                }
            }
        }

        for (vertex, generated) in self.vertices.iter_mut().zip(accumulated) {
            let normal = Vec3::from_array(vertex.normal);
            vertex.normal = if normal.length_squared() > 1.0e-12 {
                normal.normalize().to_array()
            } else if generated.length_squared() > 1.0e-12 {
                generated.normalize().to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
        }
    }

    fn validate(&self) -> Result<(), MeshError> {
        if self.vertices.len() > MAX_MESH_VERTICES {
            return Err(MeshError::InvalidMesh(format!(
                "mesh exceeds the vertex limit of {MAX_MESH_VERTICES}"
            )));
        }
        if self.indices.is_empty() || self.indices.len() % 3 != 0 {
            return Err(MeshError::InvalidMesh(
                "mesh has no complete triangles".to_owned(),
            ));
        }
        if self.indices.len() > MAX_MESH_INDICES {
            return Err(MeshError::InvalidMesh(format!(
                "mesh exceeds the index limit of {MAX_MESH_INDICES}"
            )));
        }
        if self.indices.iter().any(|&i| i as usize >= self.vertices.len()) {
            return Err(MeshError::InvalidMesh(
                "mesh contains an out-of-range triangle index".to_owned(),
            ));
        }
        if self.vertices.iter().any(|vertex| {
            !vertex.position.iter().all(|v| v.is_finite())
                || !vertex.normal.iter().all(|v| v.is_finite())
                || !vertex.uv_data.iter().all(|v| v.is_finite())
        }) {
            return Err(MeshError::InvalidMesh(
                "mesh contains non-finite vertex data".to_owned(),
            ));
        }
        Ok(())
    }
}

fn parse_obj(source: &str, name: &str) -> Result<MeshData, MeshError> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut texcoords: Vec<[f32; 2]> = Vec::new();
    let mut vertices: Vec<MeshVertex> = Vec::new();
    let mut has_explicit_normal: Vec<bool> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut deduplicated: HashMap<(usize, Option<usize>, Option<usize>), u32> = HashMap::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        match words.next().unwrap_or_default() {
            "v" => {
                let values = parse_vec3(words, line_number, "vertex")?;
                positions.push(Vec3::from_array(values));
            }
            "vn" => {
                let values = parse_vec3(words, line_number, "normal")?;
                normals.push(Vec3::from_array(values).normalize_or_zero());
            }
            "vt" => {
                texcoords.push(parse_vec2(words, line_number, "texture coordinate")?);
            }
            "f" => {
                let face: Result<Vec<u32>, MeshError> = words
                    .map(|token| {
                        let mut parts = token.split('/');
                        let position_raw = parts.next().unwrap_or_default();
                        let position = resolve_obj_index(
                            position_raw,
                            positions.len(),
                            line_number,
                            "position",
                        )?;
                        let texcoord_raw = parts.next().filter(|s| !s.is_empty());
                        let texcoord = texcoord_raw
                            .map(|raw| resolve_obj_index(raw, texcoords.len(), line_number, "texture coordinate"))
                            .transpose()?;
                        let normal_raw = parts.next().filter(|s| !s.is_empty());
                        let normal = normal_raw
                            .map(|raw| {
                                resolve_obj_index(raw, normals.len(), line_number, "normal")
                            })
                            .transpose()?;

                        let key = (position, texcoord, normal);
                        if let Some(&existing) = deduplicated.get(&key) {
                            return Ok(existing);
                        }
                        if vertices.len() >= MAX_MESH_VERTICES {
                            return Err(MeshError::InvalidMesh(format!(
                                "OBJ exceeds the vertex limit of {MAX_MESH_VERTICES}"
                            )));
                        }
                        let normal_value = normal
                            .map(|i| normals[i].to_array())
                            .unwrap_or([0.0, 0.0, 0.0]);
                        let vertex_index = vertices.len() as u32;
                        vertices.push(MeshVertex {
                            position: positions[position].to_array(),
                            normal: normal_value,
                            skin_position: [0.0; 3],
                            region_position: [0.0; 3],
                            uv_data: texcoord.map(|i| [texcoords[i][0], texcoords[i][1], 1.0]).unwrap_or([0.0; 3]),
                        });
                        has_explicit_normal.push(normal.is_some());
                        deduplicated.insert(key, vertex_index);
                        Ok(vertex_index)
                    })
                    .collect();
                let face = face?;
                if face.len() < 3 {
                    return Err(MeshError::Parse {
                        line: line_number,
                        detail: "face needs at least three vertices".to_owned(),
                    });
                }
                for i in 1..face.len() - 1 {
                    indices.extend_from_slice(&[face[0], face[i], face[i + 1]]);
                }
                if indices.len() > MAX_MESH_INDICES {
                    return Err(MeshError::InvalidMesh(format!(
                        "OBJ exceeds the index limit of {MAX_MESH_INDICES}"
                    )));
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(MeshError::InvalidMesh(
            "OBJ contains no usable triangle mesh".to_owned(),
        ));
    }

    // Generate smooth normals only where the OBJ file did not provide any.
    let mut accumulated = vec![Vec3::ZERO; vertices.len()];
    for triangle in indices.chunks_exact(3) {
        let a = Vec3::from_array(vertices[triangle[0] as usize].position);
        let b = Vec3::from_array(vertices[triangle[1] as usize].position);
        let c = Vec3::from_array(vertices[triangle[2] as usize].position);
        let face_normal = (b - a).cross(c - a);
        for &index in triangle {
            let i = index as usize;
            if !has_explicit_normal[i] {
                accumulated[i] += face_normal;
            }
        }
    }
    for (i, vertex) in vertices.iter_mut().enumerate() {
        if !has_explicit_normal[i] {
            vertex.normal = accumulated[i].normalize_or_zero().to_array();
        }
    }

    Ok(MeshData {
        name: name.to_owned(),
        vertices,
        indices,
    })
}

fn parse_vec2<'a>(
    mut words: impl Iterator<Item = &'a str>,
    line: usize,
    kind: &str,
) -> Result<[f32; 2], MeshError> {
    let mut values = [0.0; 2];
    for value in values.iter_mut() {
        let raw = words.next().ok_or_else(|| MeshError::Parse {
            line,
            detail: format!("{kind} needs two coordinates"),
        })?;
        *value = raw.parse::<f32>().map_err(|_| MeshError::Parse {
            line,
            detail: format!("invalid {kind} coordinate '{raw}'"),
        })?;
    }
    if !values.iter().all(|value| value.is_finite()) {
        return Err(MeshError::Parse {
            line,
            detail: format!("{kind} contains a non-finite coordinate"),
        });
    }
    Ok(values)
}

fn parse_vec3<'a>(
    mut words: impl Iterator<Item = &'a str>,
    line: usize,
    kind: &str,
) -> Result<[f32; 3], MeshError> {
    let mut values = [0.0; 3];
    for value in values.iter_mut() {
        let raw = words.next().ok_or_else(|| MeshError::Parse {
            line,
            detail: format!("{kind} needs three coordinates"),
        })?;
        *value = raw.parse::<f32>().map_err(|_| MeshError::Parse {
            line,
            detail: format!("invalid {kind} coordinate '{raw}'"),
        })?;
    }
    if !values.iter().all(|v| v.is_finite()) {
        return Err(MeshError::Parse {
            line,
            detail: format!("{kind} contains a non-finite coordinate"),
        });
    }
    Ok(values)
}

fn resolve_obj_index(
    raw: &str,
    count: usize,
    line: usize,
    kind: &str,
) -> Result<usize, MeshError> {
    let index = raw.parse::<i64>().map_err(|_| MeshError::Parse {
        line,
        detail: format!("invalid OBJ {kind} index '{raw}'"),
    })?;
    if index == 0 {
        return Err(MeshError::Parse {
            line,
            detail: format!("OBJ {kind} indices are 1-based; zero is invalid"),
        });
    }
    let resolved = if index > 0 {
        index - 1
    } else {
        count as i64 + index
    };
    if resolved < 0 || resolved >= count as i64 {
        return Err(MeshError::Parse {
            line,
            detail: format!("OBJ {kind} index '{raw}' is out of range"),
        });
    }
    Ok(resolved as usize)
}

fn load_gltf(path: &Path) -> Result<MeshData, MeshError> {
    let (document, buffers, _images) =
        gltf::import(path).map_err(|error| MeshError::Gltf(error.to_string()))?;
    let mut mesh = MeshData {
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("glTF mesh")
            .to_owned(),
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or_else(|| MeshError::InvalidMesh("glTF file has no scene".to_owned()))?;

    for node in scene.nodes() {
        append_gltf_node(node, Mat4::IDENTITY, &buffers, &mut mesh)?;
    }
    if mesh.vertices.is_empty() {
        return Err(MeshError::InvalidMesh(
            "glTF scene contains no triangle meshes".to_owned(),
        ));
    }
    if mesh.vertices.len() > MAX_MESH_VERTICES || mesh.indices.len() > MAX_MESH_INDICES {
        return Err(MeshError::InvalidMesh(
            "glTF scene exceeds supported preview mesh limits".to_owned(),
        ));
    }
    Ok(mesh)
}

fn append_gltf_node(
    node: gltf::Node<'_>,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    output: &mut MeshData,
) -> Result<(), MeshError> {
    let local_matrix = Mat4::from_cols_array_2d(&node.transform().matrix());
    let transform = parent_transform * local_matrix;

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| MeshError::InvalidMesh("glTF primitive has no positions".to_owned()))?
                .collect();
            let normals: Option<Vec<[f32; 3]>> =
                reader.read_normals().map(|values| values.collect());
            let texcoords: Option<Vec<[f32; 2]>> = reader
                .read_tex_coords(0)
                .map(|values| values.into_f32().collect());
            let source_indices: Vec<u32> = reader
                .read_indices()
                .map(|values| values.into_u32().collect())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());

            if source_indices.len() % 3 != 0 {
                return Err(MeshError::InvalidMesh(
                    "glTF triangle primitive index count is not divisible by three".to_owned(),
                ));
            }
            if output.vertices.len().saturating_add(positions.len()) > MAX_MESH_VERTICES
                || output.indices.len().saturating_add(source_indices.len()) > MAX_MESH_INDICES
            {
                return Err(MeshError::InvalidMesh(
                    "glTF scene exceeds supported preview mesh limits".to_owned(),
                ));
            }

            let vertex_offset = output.vertices.len() as u32;
            let normal_matrix = transform.inverse().transpose();
            for (index, position) in positions.iter().enumerate() {
                let world_position = transform.transform_point3(Vec3::from_array(*position));
                let world_normal = normals
                    .as_ref()
                    .and_then(|values| values.get(index))
                    .map(|normal| {
                        normal_matrix
                            .transform_vector3(Vec3::from_array(*normal))
                            .normalize_or_zero()
                    })
                    .unwrap_or(Vec3::ZERO);
                output.vertices.push(MeshVertex {
                    position: world_position.to_array(),
                    normal: world_normal.to_array(),
                    skin_position: [0.0; 3],
                    region_position: [0.0; 3],
                    uv_data: texcoords.as_ref().and_then(|values| values.get(index)).map(|uv| [uv[0], uv[1], 1.0]).unwrap_or([0.0; 3]),
                });
            }
            for index in source_indices {
                if index as usize >= positions.len() {
                    return Err(MeshError::InvalidMesh(
                        "glTF primitive contains an out-of-range index".to_owned(),
                    ));
                }
                output.indices.push(vertex_offset + index);
            }
        }
    }

    for child in node.children() {
        append_gltf_node(child, transform, buffers, output)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum MeshError {
    Io(std::io::Error),
    UnsupportedFormat(String),
    Parse { line: usize, detail: String },
    InvalidMesh(String),
    Gltf(String),
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read mesh: {error}"),
            Self::UnsupportedFormat(ext) => {
                write!(f, "unsupported mesh format '.{ext}'; use OBJ, glTF or GLB")
            }
            Self::Parse { line, detail } => write!(f, "OBJ line {line}: {detail}"),
            Self::InvalidMesh(detail) => write!(f, "invalid mesh: {detail}"),
            Self::Gltf(detail) => write!(f, "could not import glTF/GLB: {detail}"),
        }
    }
}

impl Error for MeshError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_quad_is_triangulated_and_normals_are_generated() {
        let source = "\
v -1 0 -1
v 1 0 -1
v 1 0 1
v -1 0 1
f 1 2 3 4
";
        let mut mesh = parse_obj(source, "quad").unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        mesh.normalize_to_preview_scale().unwrap();
        mesh.recalculate_missing_normals();
        mesh.validate().unwrap();
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| (Vec3::from_array(vertex.normal).length() - 1.0).abs() < 1.0e-4));
    }

    #[test]
    fn obj_supports_negative_indices_and_explicit_normals() {
        let source = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 2
f -3//1 -2//1 -1//1
";
        let mesh = parse_obj(source, "negative indices").unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertices[0].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn obj_rejects_invalid_zero_indices() {
        let source = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 2 3\n";
        assert!(parse_obj(source, "bad").is_err());
    }

    #[test]
    fn preview_sphere_has_valid_triangles() {
        let mesh = MeshData::default_sphere();
        assert!(!mesh.vertices.is_empty());
        assert!(mesh.triangle_count() > 1000);
        mesh.validate().unwrap();
    }
}
