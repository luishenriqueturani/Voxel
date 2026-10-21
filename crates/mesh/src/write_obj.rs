use std::fmt::Write as _;
use std::path::Path;

use crate::error::MeshError;
use crate::surface::Surface;

pub(crate) fn store(
    path: &Path,
    mesh: &Surface,
    texture_name: Option<&str>,
) -> Result<(), MeshError> {
    let mut text = String::new();
    if let Some(name) = texture_name {
        let mtl = path.with_extension("mtl");
        let mtl_name = file_name(&mtl)?;
        let _ = writeln!(text, "mtllib {mtl_name}");
        let _ = writeln!(text, "usemtl material");
        std::fs::write(mtl, format!("newmtl material\nmap_Kd {name}\n"))?;
    }
    for (position, color) in mesh.positions.iter().zip(colors_of(mesh)) {
        let _ = write!(
            text,
            "v {} {} {}",
            num(position[0]),
            num(position[1]),
            num(position[2])
        );
        if let Some(color) = color {
            let _ = write!(
                text,
                " {} {} {}",
                num(color[0]),
                num(color[1]),
                num(color[2])
            );
        }
        text.push('\n');
    }
    for normal in &mesh.normals {
        let _ = writeln!(
            text,
            "vn {} {} {}",
            num(normal[0]),
            num(normal[1]),
            num(normal[2])
        );
    }
    let has_uv = mesh.uvs.len() == mesh.positions.len();
    if has_uv {
        for uv in &mesh.uvs {
            let _ = writeln!(text, "vt {} {}", num(uv[0]), num(1.0 - uv[1]));
        }
    }
    for face in mesh.indices.chunks_exact(3) {
        text.push('f');
        for index in face {
            let slot = index.saturating_add(1);
            if has_uv {
                let _ = write!(text, " {slot}/{slot}/{slot}");
            } else {
                let _ = write!(text, " {slot}//{slot}");
            }
        }
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn colors_of(mesh: &Surface) -> Vec<Option<[f32; 3]>> {
    if mesh.texture.is_some() || mesh.colors.len() != mesh.positions.len() {
        return vec![None; mesh.positions.len()];
    }
    mesh.colors.iter().copied().map(Some).collect()
}

fn file_name(path: &Path) -> Result<String, MeshError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| MeshError::Write("nome inválido".to_owned()))
}

fn num(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
