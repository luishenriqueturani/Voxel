use crate::error::MeshError;
use crate::surface::{Surface, face_normals};

pub(crate) fn reduce(mesh: &Surface, triangles: u32) -> Result<Surface, MeshError> {
    let indices = decimate(&mesh.indices, &mesh.positions, triangles);
    gather(mesh, &indices)
}

fn decimate(indices: &[u32], positions: &[[f32; 3]], triangles: u32) -> Vec<u32> {
    let target = usize::try_from(triangles.saturating_mul(3)).unwrap_or(usize::MAX);
    if indices.len() <= target || target < 3 || positions.is_empty() {
        return indices.to_vec();
    }
    let mut error = 0.0;
    let simplified = meshopt::simplify_decoder(
        indices,
        positions,
        target,
        1.0,
        meshopt::SimplifyOptions::empty(),
        Some(&mut error),
    );
    if simplified.len() < 3 {
        indices.to_vec()
    } else {
        simplified
    }
}

fn gather(mesh: &Surface, indices: &[u32]) -> Result<Surface, MeshError> {
    let mut remap = vec![None; mesh.positions.len()];
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut out = Vec::with_capacity(indices.len());
    let keep_uv = mesh.uvs.len() == mesh.positions.len();
    let keep_color = mesh.colors.len() == mesh.positions.len();
    for index in indices {
        let slot = usize::try_from(*index)?;
        let Some(source) = mesh.positions.get(slot) else {
            return Err(MeshError::Range("índice fora da malha".to_owned()));
        };
        let mapped = if let Some(existing) = remap.get(slot).copied().flatten() {
            existing
        } else {
            let next = u32::try_from(positions.len())?;
            if let Some(entry) = remap.get_mut(slot) {
                *entry = Some(next);
            }
            positions.push(*source);
            if keep_uv {
                uvs.push(mesh.uvs.get(slot).copied().unwrap_or([0.0, 0.0]));
            }
            if keep_color {
                colors.push(mesh.colors.get(slot).copied().unwrap_or([1.0, 1.0, 1.0]));
            }
            next
        };
        out.push(mapped);
    }
    if out.len() < 3 {
        return Err(MeshError::Empty);
    }
    let normals = face_normals(&positions, &out);
    Ok(Surface {
        positions,
        normals,
        uvs,
        colors,
        indices: out,
        texture: mesh.texture.clone(),
    })
}
