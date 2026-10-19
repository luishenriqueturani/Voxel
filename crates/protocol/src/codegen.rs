use std::fmt::Write;

use serde_json::Value;

use crate::error::SchemaError;

fn emit(out: &mut String, args: std::fmt::Arguments<'_>) {
    out.write_fmt(args).expect("texto");
}

pub struct Generated {
    pub rust: String,
    pub typescript: String,
    pub python: String,
}

struct Field {
    json_name: String,
    rust_name: String,
    ty: FieldType,
    required: bool,
}

enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Ref(String),
}

enum TypeBody {
    Enum(Vec<String>),
    Object(Vec<Field>),
}

struct TypeDef {
    name: String,
    body: TypeBody,
}

struct Document {
    paths: Vec<String>,
    types: Vec<TypeDef>,
}

/// Gera os tipos de Rust, TypeScript e Python a partir do `OpenAPI`.
///
/// # Errors
///
/// Falha se o JSON não for o subconjunto de esquema que o gerador conhece.
pub fn generate(openapi: &str) -> Result<Generated, SchemaError> {
    let document = parse(openapi)?;
    Ok(Generated {
        rust: rustfmt(&emit_rust(&document))?,
        typescript: emit_typescript(&document),
        python: emit_python(&document),
    })
}

fn rustfmt(source: &str) -> Result<String, SchemaError> {
    let mut child = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| SchemaError::Unsupported(format!("rustfmt: {error}")))?;
    {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err(SchemaError::Unsupported("rustfmt sem stdin".to_owned()));
        };
        std::io::Write::write_all(stdin, source.as_bytes())
            .map_err(|error| SchemaError::Unsupported(error.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| SchemaError::Unsupported(error.to_string()))?;
    if !output.status.success() {
        return Err(SchemaError::Unsupported(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| SchemaError::Unsupported(error.to_string()))
}

fn parse(openapi: &str) -> Result<Document, SchemaError> {
    let root: Value = serde_json::from_str(openapi)?;
    let paths = root
        .get("paths")
        .and_then(Value::as_object)
        .ok_or(SchemaError::Missing("paths"))?;
    let schemas = root
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .ok_or(SchemaError::Missing("components.schemas"))?;
    Ok(Document {
        paths: paths.keys().cloned().collect(),
        types: schemas
            .iter()
            .map(|(name, schema)| parse_type(name, schema))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_type(name: &str, schema: &Value) -> Result<TypeDef, SchemaError> {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let variants = values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| SchemaError::Unsupported(format!("{name} tem enum não textual")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(TypeDef {
            name: name.to_owned(),
            body: TypeBody::Enum(variants),
        });
    }
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| SchemaError::Unsupported(format!("{name} não é objeto nem enum")))?;
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let fields = properties
        .iter()
        .map(|(field_name, field)| parse_field(name, field_name, field, &required))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeDef {
        name: name.to_owned(),
        body: TypeBody::Object(fields),
    })
}

fn parse_field(
    parent: &str,
    name: &str,
    schema: &Value,
    required: &[String],
) -> Result<Field, SchemaError> {
    let ty = if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let type_name = reference
            .rsplit('/')
            .next()
            .ok_or_else(|| SchemaError::Unsupported(format!("{parent}.{name} tem $ref vazio")))?;
        FieldType::Ref(type_name.to_owned())
    } else {
        match schema.get("type").and_then(Value::as_str) {
            Some("string") => FieldType::String,
            Some("integer") => FieldType::Integer,
            Some("number") => FieldType::Number,
            Some("boolean") => FieldType::Boolean,
            other => {
                return Err(SchemaError::Unsupported(format!(
                    "{parent}.{name} tem tipo {other:?}"
                )));
            }
        }
    };
    Ok(Field {
        rust_name: snake(name),
        json_name: name.to_owned(),
        ty,
        required: required.iter().any(|item| item == name),
    })
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }
    if matches!(
        out.as_str(),
        "type" | "ref" | "self" | "async" | "await" | "fn" | "mod"
    ) {
        out.push('_');
    }
    out
}

fn variant(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return "Empty".to_owned();
    };
    let mut name = String::new();
    name.extend(first.to_uppercase());
    name.push_str(chars.as_str());
    name
}

fn const_name(path: &str) -> String {
    path.trim_start_matches('/')
        .replace(['/', '-'], "_")
        .to_uppercase()
}

fn camel_key(path: &str) -> String {
    let mut parts = path.trim_start_matches('/').split(['/', '-']);
    let Some(first) = parts.next() else {
        return String::new();
    };
    let mut name = first.to_owned();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name
}

fn rust_type(ty: &FieldType) -> String {
    match ty {
        FieldType::String => "String".to_owned(),
        FieldType::Integer => "i64".to_owned(),
        FieldType::Number => "f64".to_owned(),
        FieldType::Boolean => "bool".to_owned(),
        FieldType::Ref(name) => name.clone(),
    }
}

fn ts_type(ty: &FieldType) -> String {
    match ty {
        FieldType::String => "string".to_owned(),
        FieldType::Integer | FieldType::Number => "number".to_owned(),
        FieldType::Boolean => "boolean".to_owned(),
        FieldType::Ref(name) => name.clone(),
    }
}

fn python_type(ty: &FieldType) -> String {
    match ty {
        FieldType::String => "str".to_owned(),
        FieldType::Integer => "int".to_owned(),
        FieldType::Number => "float".to_owned(),
        FieldType::Boolean => "bool".to_owned(),
        FieldType::Ref(name) => name.clone(),
    }
}

fn has_float(fields: &[Field]) -> bool {
    fields
        .iter()
        .any(|field| matches!(field.ty, FieldType::Number))
}

fn emit_rust(document: &Document) -> String {
    let mut out = String::from(
        "// Gerado a partir de protocol/openapi.json. Não edite à mão.\n\nuse serde::{Deserialize, Serialize};\n\n",
    );
    for definition in &document.types {
        match &definition.body {
            TypeBody::Enum(values) => {
                out.push_str(
                    "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n",
                );
                emit(&mut out, format_args!("pub enum {} {{\n", definition.name));
                for value in values {
                    emit(
                        &mut out,
                        format_args!(
                            "    #[serde(rename = \"{value}\")]\n    {},\n",
                            variant(value)
                        ),
                    );
                }
                out.push_str("}\n\n");
            }
            TypeBody::Object(fields) => {
                let eq = if has_float(fields) { "" } else { ", Eq" };
                if has_float(fields) {
                    out.push_str("#[allow(clippy::derive_partial_eq_without_eq)]\n");
                }
                emit(
                    &mut out,
                    format_args!(
                        "#[derive(Debug, Clone, PartialEq{eq}, Serialize, Deserialize)]\n"
                    ),
                );
                out.push_str("#[serde(rename_all = \"camelCase\")]\n");
                out.push_str("#[serde(deny_unknown_fields)]\n");
                emit(
                    &mut out,
                    format_args!("pub struct {} {{\n", definition.name),
                );
                for field in fields {
                    if !field.required {
                        out.push_str(
                            "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n",
                        );
                    }
                    let ty = rust_type(&field.ty);
                    if field.required {
                        emit(
                            &mut out,
                            format_args!("    pub {}: {ty},\n", field.rust_name),
                        );
                    } else {
                        emit(
                            &mut out,
                            format_args!("    pub {}: Option<{ty}>,\n", field.rust_name),
                        );
                    }
                }
                out.push_str("}\n\n");
            }
        }
    }
    out.push_str("pub mod paths {\n");
    for path in &document.paths {
        emit(
            &mut out,
            format_args!("    pub const {}: &str = \"{path}\";\n", const_name(path)),
        );
    }
    out.push_str("}\n");
    out
}

fn emit_typescript(document: &Document) -> String {
    let mut out = String::from("// Gerado a partir de protocol/openapi.json. Não edite à mão.\n\n");
    for definition in &document.types {
        match &definition.body {
            TypeBody::Enum(values) => {
                let joined = values
                    .iter()
                    .map(|value| format!("\"{value}\""))
                    .collect::<Vec<_>>()
                    .join(" | ");
                emit(
                    &mut out,
                    format_args!("export type {} = {joined};\n\n", definition.name),
                );
            }
            TypeBody::Object(fields) => {
                emit(
                    &mut out,
                    format_args!("export type {} = {{\n", definition.name),
                );
                for field in fields {
                    let optional = if field.required { "" } else { "?" };
                    emit(
                        &mut out,
                        format_args!("  {}{optional}: {};\n", field.json_name, ts_type(&field.ty)),
                    );
                }
                out.push_str("};\n\n");
            }
        }
    }
    out.push_str("export const paths = {\n");
    for path in &document.paths {
        emit(
            &mut out,
            format_args!("  {}: \"{path}\",\n", camel_key(path)),
        );
    }
    out.push_str("} as const;\n");
    out
}

fn emit_python(document: &Document) -> String {
    let mut out = String::from(
        "\"\"\"Tipos gerados a partir de protocol/openapi.json. Não edite à mão.\"\"\"\n\nfrom typing import Literal, NotRequired, TypedDict\n\n\n",
    );
    for definition in &document.types {
        match &definition.body {
            TypeBody::Enum(values) => {
                let joined = values
                    .iter()
                    .map(|value| format!("\"{value}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                emit(
                    &mut out,
                    format_args!("{} = Literal[{joined}]\n\n\n", definition.name),
                );
            }
            TypeBody::Object(fields) => {
                emit(
                    &mut out,
                    format_args!("class {}(TypedDict):\n", definition.name),
                );
                if fields.is_empty() {
                    out.push_str("    pass\n\n");
                    continue;
                }
                for field in fields {
                    let ty = python_type(&field.ty);
                    if field.required {
                        emit(&mut out, format_args!("    {}: {ty}\n", field.json_name));
                    } else {
                        emit(
                            &mut out,
                            format_args!("    {}: NotRequired[{ty}]\n", field.json_name),
                        );
                    }
                }
                out.push_str("\n\n");
            }
        }
    }
    out.push_str("PATHS: dict[str, str] = {\n");
    for path in &document.paths {
        emit(
            &mut out,
            format_args!("    \"{}\": \"{path}\",\n", camel_key(path)),
        );
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gera_enum_objeto_opcional_e_caminho() {
        let openapi = r##"{
          "paths": { "/health": {} },
          "components": { "schemas": {
            "Kind": { "type": "string", "enum": ["image"] },
            "Ask": {
              "type": "object",
              "properties": {
                "jobId": { "type": "string" },
                "note": { "type": "string" },
                "kind": { "$ref": "#/components/schemas/Kind" }
              },
              "required": ["jobId", "kind"]
            }
          } }
        }"##;
        let generated = generate(openapi).unwrap();
        assert!(generated.rust.contains("pub enum Kind"));
        assert!(generated.rust.contains("pub job_id: String"));
        assert!(generated.rust.contains("pub note: Option<String>"));
        assert!(
            generated
                .rust
                .contains("pub const HEALTH: &str = \"/health\";")
        );
        assert!(
            generated
                .typescript
                .contains("export type Kind = \"image\";")
        );
        assert!(generated.typescript.contains("note?: string;"));
        assert!(generated.python.contains("note: NotRequired[str]"));
        assert!(generated.python.contains("\"health\": \"/health\""));
    }

    #[test]
    fn rejeita_json_invalido_e_esquema_incompleto() {
        assert!(generate("[").is_err());
        assert!(generate("{}").is_err());
        assert!(generate(r#"{"paths":{}}"#).is_err());
    }

    #[test]
    fn rejeita_tipo_desconhecido() {
        let openapi = r#"{
          "paths": {},
          "components": { "schemas": { "Bad": { "type": "array" } } }
        }"#;
        assert!(generate(openapi).is_err());
    }

    #[test]
    fn arquivos_gerados_batem_com_o_esquema() {
        let openapi = include_str!("../../../protocol/openapi.json");
        let generated = generate(openapi).unwrap();
        assert_eq!(generated.rust, include_str!("generated.rs"));
        assert_eq!(
            generated.typescript,
            include_str!("../../../apps/desktop/src/protocol/types.ts")
        );
        assert_eq!(
            generated.python,
            include_str!("../../../workers/python/src/voxel_worker/protocol.py")
        );
    }
}
