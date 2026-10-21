use std::path::Path;

use serde_json::{Map, Value, json};

use crate::error::MeshError;
use crate::surface::Surface;

pub(crate) fn store(path: &Path, mesh: &Surface) -> Result<(), MeshError> {
    let bytes = encode(mesh)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

fn encode(mesh: &Surface) -> Result<Vec<u8>, MeshError> {
    let mut bin = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let position = push_vec3(&mut bin, &mut views, &mut accessors, &mesh.positions, true);
    let normal = push_vec3(&mut bin, &mut views, &mut accessors, &mesh.normals, false);
    let mut attributes = Map::new();
    attributes.insert("POSITION".to_owned(), json!(position));
    attributes.insert("NORMAL".to_owned(), json!(normal));
    if mesh.uvs.len() == mesh.positions.len() {
        let uv = push_vec2(&mut bin, &mut views, &mut accessors, &mesh.uvs);
        attributes.insert("TEXCOORD_0".to_owned(), json!(uv));
    }
    if mesh.texture.is_none() && mesh.colors.len() == mesh.positions.len() {
        let color = push_vec3(&mut bin, &mut views, &mut accessors, &mesh.colors, false);
        attributes.insert("COLOR_0".to_owned(), json!(color));
    }
    let indices = push_indices(&mut bin, &mut views, &mut accessors, &mesh.indices);
    let png = texture_png(mesh)?;
    let mut pbr = json!({"metallicFactor": 0.0, "roughnessFactor": 1.0});
    let mut images = Vec::new();
    let mut textures = Vec::new();
    if let Some(bytes) = png {
        let view = push_bytes(&mut bin, &mut views, &bytes);
        images.push(json!({"mimeType": "image/png", "bufferView": view}));
        textures.push(json!({"sampler": 0, "source": 0}));
        pbr["baseColorTexture"] = json!({"index": 0});
    }
    let mut document = json!({
        "asset": {"version": "2.0", "generator": "Voxel"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{
            "primitives": [{
                "attributes": attributes,
                "indices": indices,
                "material": 0
            }]
        }],
        "materials": [{"pbrMetallicRoughness": pbr}],
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}]
    });
    if !textures.is_empty() {
        document["images"] = json!(images);
        document["textures"] = json!(textures);
        document["samplers"] = json!([{}]);
    }
    pack(&document, &bin)
}

fn texture_png(mesh: &Surface) -> Result<Option<Vec<u8>>, MeshError> {
    let Some(texture) = &mesh.texture else {
        return Ok(None);
    };
    if mesh.uvs.len() != mesh.positions.len() {
        return Ok(None);
    }
    let mut png = Vec::new();
    texture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    Ok(Some(png))
}

fn pack(document: &Value, bin: &[u8]) -> Result<Vec<u8>, MeshError> {
    let mut json_bytes =
        serde_json::to_vec(document).map_err(|error| MeshError::Write(error.to_string()))?;
    pad(&mut json_bytes, b' ');
    let mut binary = bin.to_vec();
    pad(&mut binary, 0);
    let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&u32::try_from(total)?.to_le_bytes());
    out.extend_from_slice(&u32::try_from(json_bytes.len())?.to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&u32::try_from(binary.len())?.to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&binary);
    Ok(out)
}

fn push_vec3(
    bin: &mut Vec<u8>,
    views: &mut Vec<Value>,
    accessors: &mut Vec<Value>,
    values: &[[f32; 3]],
    bounds: bool,
) -> usize {
    let mut bytes = Vec::with_capacity(values.len().saturating_mul(12));
    for value in values {
        for component in value {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    let view = push_bytes(bin, views, &bytes);
    let mut accessor = json!({
        "bufferView": view,
        "componentType": 5126,
        "count": values.len(),
        "type": "VEC3"
    });
    if bounds {
        let (min, max) = bounds_of(values);
        accessor["min"] = json!(min);
        accessor["max"] = json!(max);
    }
    accessors.push(accessor);
    accessors.len() - 1
}

fn push_vec2(
    bin: &mut Vec<u8>,
    views: &mut Vec<Value>,
    accessors: &mut Vec<Value>,
    values: &[[f32; 2]],
) -> usize {
    let mut bytes = Vec::with_capacity(values.len().saturating_mul(8));
    for value in values {
        bytes.extend_from_slice(&value[0].to_le_bytes());
        bytes.extend_from_slice(&value[1].to_le_bytes());
    }
    let view = push_bytes(bin, views, &bytes);
    accessors.push(json!({
        "bufferView": view,
        "componentType": 5126,
        "count": values.len(),
        "type": "VEC2"
    }));
    accessors.len() - 1
}

fn push_indices(
    bin: &mut Vec<u8>,
    views: &mut Vec<Value>,
    accessors: &mut Vec<Value>,
    indices: &[u32],
) -> usize {
    let mut bytes = Vec::with_capacity(indices.len().saturating_mul(4));
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    let view = push_bytes(bin, views, &bytes);
    accessors.push(json!({
        "bufferView": view,
        "componentType": 5125,
        "count": indices.len(),
        "type": "SCALAR"
    }));
    accessors.len() - 1
}

fn push_bytes(bin: &mut Vec<u8>, views: &mut Vec<Value>, bytes: &[u8]) -> usize {
    let offset = bin.len();
    bin.extend_from_slice(bytes);
    pad(bin, 0);
    views.push(json!({
        "buffer": 0,
        "byteOffset": offset,
        "byteLength": bytes.len()
    }));
    views.len() - 1
}

fn bounds_of(values: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let Some(first) = values.first().copied() else {
        return ([0.0; 3], [0.0; 3]);
    };
    let mut min = first;
    let mut max = first;
    for value in values {
        for axis in 0..3 {
            min[axis] = min[axis].min(value[axis]);
            max[axis] = max[axis].max(value[axis]);
        }
    }
    (min, max)
}

fn pad(bytes: &mut Vec<u8>, filler: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(filler);
    }
}
