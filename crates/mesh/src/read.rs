use gltf::mesh::Mode;

use crate::error::MeshError;
use crate::surface::Surface;

pub(crate) fn load(path: &std::path::Path) -> Result<Surface, MeshError> {
    let (document, buffers, images) = gltf::import(path)?;
    let mut surface = Surface {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        colors: Vec::new(),
        indices: Vec::new(),
        texture: None,
    };
    let mut texture_index = None;
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if texture_index.is_none() {
                texture_index = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|info| info.texture().source().index());
            }
            append(&mut surface, &primitive, &buffers)?;
        }
    }
    if surface.triangles() == 0 {
        return Err(MeshError::Empty);
    }
    if let Some(index) = texture_index {
        surface.texture = images.get(index).and_then(image_from);
    }
    Ok(surface)
}

fn append(
    surface: &mut Surface,
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
) -> Result<(), MeshError> {
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let Some(positions) = reader.read_positions() else {
        return Ok(());
    };
    let positions = positions.collect::<Vec<_>>();
    let vertex_count = positions.len();
    let count = u32::try_from(vertex_count)?;
    let raw = reader.read_indices().map_or_else(
        || (0..count).collect(),
        |indices| indices.into_u32().collect::<Vec<_>>(),
    );
    let indices = triangulate(primitive.mode(), raw);
    if indices.is_empty() {
        return Ok(());
    }
    let uvs = reader
        .read_tex_coords(0)
        .map(|coords| coords.into_f32().collect::<Vec<_>>());
    let colors = reader
        .read_colors(0)
        .map(|colors| colors.into_rgb_f32().collect::<Vec<_>>());
    let base = u32::try_from(surface.positions.len())?;
    align(&mut surface.uvs, surface.positions.len(), uvs.is_some());
    align(
        &mut surface.colors,
        surface.positions.len(),
        colors.is_some(),
    );
    surface.positions.extend(positions);
    extend_channel(&mut surface.uvs, vertex_count, uvs);
    extend_channel(&mut surface.colors, vertex_count, colors);
    surface
        .indices
        .extend(indices.into_iter().map(|index| index.saturating_add(base)));
    Ok(())
}

fn align<T: Copy + Default>(values: &mut Vec<T>, length: usize, needed: bool) {
    if needed && values.len() < length {
        values.resize(length, T::default());
    }
}

fn extend_channel<T: Copy + Default>(values: &mut Vec<T>, count: usize, incoming: Option<Vec<T>>) {
    if values.is_empty() && incoming.is_none() {
        return;
    }
    match incoming {
        Some(channel) if channel.len() == count => values.extend(channel),
        _ => values.resize(values.len() + count, T::default()),
    }
}

fn triangulate(mode: Mode, indices: Vec<u32>) -> Vec<u32> {
    match mode {
        Mode::Triangles => indices,
        Mode::TriangleStrip => strip(&indices),
        Mode::TriangleFan => fan(&indices),
        _ => Vec::new(),
    }
}

fn strip(indices: &[u32]) -> Vec<u32> {
    let mut triangles = Vec::new();
    for index in 0..indices.len().saturating_sub(2) {
        let face = if index % 2 == 0 {
            [indices[index], indices[index + 1], indices[index + 2]]
        } else {
            [indices[index + 1], indices[index], indices[index + 2]]
        };
        if face[0] != face[1] && face[1] != face[2] && face[0] != face[2] {
            triangles.extend(face);
        }
    }
    triangles
}

fn fan(indices: &[u32]) -> Vec<u32> {
    let Some(center) = indices.first().copied() else {
        return Vec::new();
    };
    let mut triangles = Vec::new();
    for index in 1..indices.len().saturating_sub(1) {
        triangles.extend([center, indices[index], indices[index + 1]]);
    }
    triangles
}

fn image_from(data: &gltf::image::Data) -> Option<image::RgbaImage> {
    let pixels = match data.format {
        gltf::image::Format::R8G8B8A8 => data.pixels.clone(),
        gltf::image::Format::R8G8B8 => expand_rgb(&data.pixels),
        _ => return None,
    };
    image::RgbaImage::from_raw(data.width, data.height, pixels)
}

fn expand_rgb(pixels: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(pixels.len() / 3 * 4);
    for chunk in pixels.chunks_exact(3) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    rgba
}
