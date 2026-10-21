use std::fmt::Write as _;
use std::path::Path;

use crate::error::MeshError;
use crate::surface::Surface;

const GEOMETRY: i64 = 10;
const MODEL: i64 = 20;
const MATERIAL: i64 = 30;
const TEXTURE: i64 = 40;
const VIDEO: i64 = 50;

pub(crate) fn store(
    path: &Path,
    mesh: &Surface,
    texture_name: Option<&str>,
) -> Result<(), MeshError> {
    let mut text = String::new();
    text.push_str("; FBX 7.4.0 project file\n");
    text.push_str("FBXHeaderExtension:  {\n");
    text.push_str("\tFBXHeaderVersion: 1003\n\tFBXVersion: 7400\n\tCreator: \"Voxel\"\n}\n");
    text.push_str("GlobalSettings:  {\n\tVersion: 1000\n\tProperties70:  {\n");
    text.push_str("\t\tP: \"UpAxis\", \"int\", \"Integer\", \"\",1\n");
    text.push_str("\t\tP: \"UpAxisSign\", \"int\", \"Integer\", \"\",1\n");
    text.push_str("\t\tP: \"FrontAxis\", \"int\", \"Integer\", \"\",2\n");
    text.push_str("\t\tP: \"FrontAxisSign\", \"int\", \"Integer\", \"\",1\n");
    text.push_str("\t\tP: \"CoordAxis\", \"int\", \"Integer\", \"\",0\n");
    text.push_str("\t\tP: \"CoordAxisSign\", \"int\", \"Integer\", \"\",1\n");
    text.push_str("\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\",1\n");
    text.push_str("\t}\n}\n");
    geometry(&mut text, mesh)?;
    model(&mut text);
    if let Some(name) = texture_name {
        material(&mut text, name);
    }
    connections(&mut text, texture_name.is_some());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn geometry(text: &mut String, mesh: &Surface) -> Result<(), MeshError> {
    let _ = writeln!(text, "Objects:  {{");
    let _ = writeln!(
        text,
        "\tGeometry: {GEOMETRY}, \"Geometry::Mesh\", \"Mesh\" {{"
    );
    let _ = writeln!(text, "\t\tVertices: ");
    write_floats(text, mesh.positions.iter().flat_map(|position| *position));
    let _ = writeln!(text, "\t\tPolygonVertexIndex: ");
    write_ints(text, &polygon_indices(&mesh.indices)?);
    text.push_str("\t\tGeometryVersion: 124\n");
    layer_normal(text, &mesh.normals);
    let has_uv = mesh.uvs.len() == mesh.positions.len();
    if has_uv {
        layer_uv(text, &mesh.uvs);
    }
    let has_color = mesh.texture.is_none() && mesh.colors.len() == mesh.positions.len();
    if has_color {
        layer_color(text, &mesh.colors);
    }
    if mesh.texture.is_some() {
        text.push_str("\t\tLayerElementMaterial: 0 {\n");
        text.push_str("\t\t\tVersion: 101\n\t\t\tName: \"\"\n");
        text.push_str("\t\t\tMappingInformationType: \"AllSame\"\n");
        text.push_str("\t\t\tReferenceInformationType: \"IndexToDirect\"\n");
        text.push_str("\t\t\tMaterials: *1 {\n\t\t\t\ta: 0\n\t\t\t}\n\t\t}\n");
    }
    text.push_str("\t\tLayer: 0 {\n\t\t\tVersion: 100\n");
    layer_ref(text, "LayerElementNormal");
    if has_uv {
        layer_ref(text, "LayerElementUV");
    }
    if has_color {
        layer_ref(text, "LayerElementColor");
    }
    if mesh.texture.is_some() {
        layer_ref(text, "LayerElementMaterial");
    }
    text.push_str("\t\t}\n\t}\n");
    Ok(())
}

fn layer_normal(text: &mut String, normals: &[[f32; 3]]) {
    text.push_str("\t\tLayerElementNormal: 0 {\n\t\t\tVersion: 102\n\t\t\tName: \"\"\n");
    text.push_str("\t\t\tMappingInformationType: \"ByVertice\"\n");
    text.push_str("\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tNormals: ");
    write_floats(text, normals.iter().flat_map(|normal| *normal));
    text.push_str("\t\t}\n");
}

fn layer_uv(text: &mut String, uvs: &[[f32; 2]]) {
    text.push_str("\t\tLayerElementUV: 0 {\n\t\t\tVersion: 101\n\t\t\tName: \"UVMap\"\n");
    text.push_str("\t\t\tMappingInformationType: \"ByVertice\"\n");
    text.push_str("\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tUV: ");
    write_floats(text, uvs.iter().flat_map(|uv| [uv[0], 1.0 - uv[1]]));
    text.push_str("\t\t}\n");
}

fn layer_color(text: &mut String, colors: &[[f32; 3]]) {
    text.push_str("\t\tLayerElementColor: 0 {\n\t\t\tVersion: 101\n\t\t\tName: \"\"\n");
    text.push_str("\t\t\tMappingInformationType: \"ByVertice\"\n");
    text.push_str("\t\t\tReferenceInformationType: \"Direct\"\n\t\t\tColors: ");
    write_floats(
        text,
        colors
            .iter()
            .flat_map(|color| [color[0], color[1], color[2], 1.0]),
    );
    text.push_str("\t\t}\n");
}

fn layer_ref(text: &mut String, kind: &str) {
    let _ = writeln!(
        text,
        "\t\t\tLayerElement:  {{\n\t\t\t\tType: \"{kind}\"\n\t\t\t\tTypedIndex: 0\n\t\t\t}}"
    );
}

fn model(text: &mut String) {
    let _ = writeln!(text, "\tModel: {MODEL}, \"Model::Mesh\", \"Mesh\" {{");
    text.push_str("\t\tVersion: 232\n\t\tShading: T\n\t\tCulling: \"CullingOff\"\n\t}\n");
}

fn material(text: &mut String, texture_name: &str) {
    let _ = writeln!(
        text,
        "\tMaterial: {MATERIAL}, \"Material::material\", \"\" {{"
    );
    text.push_str("\t\tVersion: 102\n\t\tShadingModel: \"lambert\"\n\t\tMultiLayer: 0\n\t}\n");
    let _ = writeln!(text, "\tTexture: {TEXTURE}, \"Texture::texture\", \"\" {{");
    text.push_str("\t\tType: \"TextureVideoClip\"\n\t\tVersion: 202\n");
    let _ = writeln!(text, "\t\tMedia: \"Video::{texture_name}\"\n");
    let _ = writeln!(
        text,
        "\t\tFileName: \"{texture_name}\"\n\t\tRelativeFilename: \"{texture_name}\"\n\t}}\n"
    );
    let _ = writeln!(
        text,
        "\tVideo: {VIDEO}, \"Video::{texture_name}\", \"Clip\" {{"
    );
    text.push_str("\t\tType: \"Clip\"\n");
    let _ = writeln!(
        text,
        "\t\tFilename: \"{texture_name}\"\n\t\tRelativeFilename: \"{texture_name}\"\n\t}}\n"
    );
}

fn connections(text: &mut String, textured: bool) {
    text.push_str("}\nConnections:  {\n");
    let _ = writeln!(text, "\tC: \"OO\",{MODEL},0");
    let _ = writeln!(text, "\tC: \"OO\",{GEOMETRY},{MODEL}");
    if textured {
        let _ = writeln!(text, "\tC: \"OO\",{MATERIAL},{MODEL}");
        let _ = writeln!(text, "\tC: \"OP\",{TEXTURE},{MATERIAL},\"DiffuseColor\"");
        let _ = writeln!(text, "\tC: \"OO\",{VIDEO},{TEXTURE}");
    }
    text.push_str("}\n");
}

fn polygon_indices(indices: &[u32]) -> Result<Vec<i32>, MeshError> {
    let mut out = Vec::with_capacity(indices.len());
    for (offset, index) in indices.iter().enumerate() {
        let value = i32::try_from(*index)?;
        let last = offset % 3 == 2;
        out.push(if last { !value } else { value });
    }
    Ok(out)
}

fn write_floats(text: &mut String, values: impl Iterator<Item = f32>) {
    let values = values.map(finite).collect::<Vec<_>>();
    let _ = writeln!(text, "*{} {{", values.len());
    text.push_str("\t\t\ta: ");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "{value}");
    }
    text.push_str("\n\t\t}\n");
}

fn write_ints(text: &mut String, values: &[i32]) {
    let _ = writeln!(text, "*{} {{", values.len());
    text.push_str("\t\t\ta: ");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "{value}");
    }
    text.push_str("\n\t\t}\n");
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
