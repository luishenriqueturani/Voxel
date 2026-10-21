//! Pós-processamento de malha para jogos.
//!
//! A decimação usa o `meshoptimizer`. O arquivo de origem não é reescrito: a
//! cópia otimizada fica ao lado, em GLB, OBJ e FBX.

mod error;
mod read;
mod simplify;
mod surface;
mod write_fbx;
mod write_glb;
mod write_obj;

use std::path::{Path, PathBuf};

use image::imageops::FilterType;

use error::MeshError;
pub use error::MeshError as Error;
use surface::{Surface, face_normals, place_on_floor};

/// Alvo da cópia otimizada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Triângulos desejados depois da decimação.
    pub triangles: u32,
    /// Lado da textura, em pixels. Vale 512, 1024 ou 2048.
    pub texture_size: u32,
}

/// Arquivos gravados ao lado do original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub glb: PathBuf,
    pub obj: PathBuf,
    pub fbx: PathBuf,
}

/// Reduz, normaliza e grava a cópia sem alterar o arquivo original.
///
/// # Errors
///
/// Falha se o GLB não tiver triângulos ou se a gravação falhar.
pub fn optimize(source: &Path, options: Options) -> Result<Written, MeshError> {
    let original = std::fs::read(source)?;
    let mut mesh = read::load(source)?;
    mesh = simplify::reduce(&mesh, options.triangles.max(1))?;
    place_on_floor(&mut mesh.positions);
    mesh.normals = face_normals(&mesh.positions, &mesh.indices);
    resize_texture(&mut mesh, texture_side(options.texture_size));
    let written = write_outputs(source, &mesh)?;
    if std::fs::read(source)? != original {
        return Err(MeshError::Write(
            "o arquivo original foi alterado".to_owned(),
        ));
    }
    Ok(written)
}

fn texture_side(value: u32) -> u32 {
    match value {
        512 | 1024 | 2048 => value,
        _ => 1024,
    }
}

fn resize_texture(mesh: &mut Surface, side: u32) {
    let Some(texture) = mesh.texture.as_mut() else {
        return;
    };
    if texture.width() == side && texture.height() == side {
        return;
    }
    *texture = image::imageops::resize(texture, side, side, FilterType::Triangle);
}

fn write_outputs(source: &Path, mesh: &Surface) -> Result<Written, MeshError> {
    let glb = sibling(source, "glb")?;
    let obj = sibling(source, "obj")?;
    let fbx = sibling(source, "fbx")?;
    let texture_name = if let Some(bytes) = image_png(mesh)? {
        let png = sibling(source, "png")?;
        std::fs::write(&png, bytes)?;
        Some(file_label(&png)?)
    } else {
        None
    };
    write_glb::store(&glb, mesh)?;
    write_obj::store(&obj, mesh, texture_name.as_deref())?;
    write_fbx::store(&fbx, mesh, texture_name.as_deref())?;
    Ok(Written { glb, obj, fbx })
}

fn image_png(mesh: &Surface) -> Result<Option<Vec<u8>>, MeshError> {
    let Some(texture) = &mesh.texture else {
        return Ok(None);
    };
    let mut png = Vec::new();
    texture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    Ok(Some(png))
}

fn sibling(source: &Path, extension: &str) -> Result<PathBuf, MeshError> {
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MeshError::Write("nome inválido".to_owned()))?;
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!("{stem}_otimizado.{extension}")))
}

fn file_label(path: &Path) -> Result<String, MeshError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| MeshError::Write("nome inválido".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Options, optimize, sibling};
    use crate::read::load;
    use crate::surface::{Surface, place_on_floor};
    use crate::write_glb::store;

    #[test]
    fn a_copia_reduz_poe_no_chao_e_preserva_o_original() {
        let root = std::env::temp_dir().join(format!("voxel-mesh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("pasta");
        let source = root.join("modelo.glb");
        let mut mesh = bump(8);
        mesh.texture = Some(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([10, 20, 30, 255]),
        ));
        mesh.uvs = vec![[0.0, 0.0]; mesh.positions.len()];
        store(&source, &mesh).expect("glb");
        let before = std::fs::read(&source).expect("ler");
        let written = optimize(
            &source,
            Options {
                triangles: 20,
                texture_size: 512,
            },
        )
        .expect("otimizar");
        assert_eq!(std::fs::read(&source).expect("original"), before);
        assert_eq!(written.glb, sibling(&source, "glb").expect("nome"));
        assert!(written.obj.is_file());
        assert!(written.fbx.is_file());
        assert!(root.join("modelo_otimizado.png").is_file());
        let copy = load(&written.glb).expect("cópia");
        assert!(copy.triangles() <= 20);
        assert!(copy.triangles() < mesh.triangles());
        let floor = copy
            .positions
            .iter()
            .map(|position| position[1])
            .fold(f32::MAX, f32::min);
        assert!(floor.abs() < 1e-3);
        let mut span = 0.0_f32;
        for position in &copy.positions {
            span = span
                .max(position[0].abs())
                .max(position[1])
                .max(position[2].abs());
        }
        assert!((span - 1.0).abs() < 1e-3 || span <= 1.0 + 1e-3);
        let texture = copy.texture.expect("textura");
        assert_eq!((texture.width(), texture.height()), (512, 512));
        let fbx = std::fs::read_to_string(&written.fbx).expect("fbx");
        assert!(fbx.contains("FBXVersion: 7400"));
        assert!(fbx.contains("PolygonVertexIndex"));
        let obj = std::fs::read_to_string(&written.obj).expect("obj");
        assert!(obj.contains("mtllib modelo_otimizado.mtl"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn o_pivô_fica_no_chão_e_o_maior_lado_vale_um() {
        let mut positions = [[0.0, 5.0, 0.0], [2.0, 7.0, 0.0], [0.0, 5.0, 2.0]];
        place_on_floor(&mut positions);
        assert!(positions.iter().all(|position| position[1] >= -1e-4));
        assert!(positions.iter().any(|position| position[1].abs() < 1e-4));
        let height = positions
            .iter()
            .map(|position| position[1])
            .fold(0.0, f32::max);
        assert!((height - 1.0).abs() < 1e-4);
        let min_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::MAX, f32::min);
        let max_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::MIN, f32::max);
        assert!(((min_x + max_x) * 0.5).abs() < 1e-4);
    }

    fn bump(cells: u32) -> Surface {
        let steps = cells + 1;
        let mut positions = Vec::new();
        for row in 0..steps {
            for column in 0..steps {
                let column_u = u16::try_from(column).expect("coluna");
                let row_u = u16::try_from(row).expect("linha");
                let height = f32::from((column_u * row_u) % 3) + 4.0;
                positions.push([f32::from(column_u), height, f32::from(row_u)]);
            }
        }
        let mut indices = Vec::new();
        for row in 0..cells {
            for column in 0..cells {
                let corner = row * steps + column;
                indices.extend([corner, corner + 1, corner + steps]);
                indices.extend([corner + 1, corner + steps + 1, corner + steps]);
            }
        }
        Surface {
            positions,
            normals: Vec::new(),
            uvs: Vec::new(),
            colors: Vec::new(),
            indices,
            texture: None,
        }
    }
}
