//! Mescla uma `LoRA` PEFT que ficou sem mesclar dentro do transformer Diffusers.
//!
//! Alguns pacotes "merged" guardam `x.base_layer.weight` mais `x.lora_A.<nome>.weight` e
//! `x.lora_B.<nome>.weight`. O `sd-cli` só conhece `x.weight`, então cada camada vira
//! `base + B·A` uma vez, no próprio disco. A `LoRA` vai antes para `voxel-lora.safetensors`,
//! e uma interrupção no meio retoma de onde parou.

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const BASE: &str = ".base_layer.";
const SIDE: &str = "voxel-lora.safetensors";
const MAX_HEADER: u64 = 100 * 1_048_576;

#[derive(Debug, Clone)]
struct Tensor {
    name: String,
    dtype: String,
    shape: Vec<usize>,
    start: u64,
    end: u64,
}

struct Shard {
    path: PathBuf,
    data_start: u64,
    tensors: Vec<Tensor>,
}

/// Mescla o transformer e o `UNet` do pacote, se tiverem camadas PEFT soltas.
///
/// # Errors
///
/// Falha se um peso não puder ser lido ou gravado, ou se o tipo não for BF16 ou F32.
pub fn prepare(model: &Path, cancel: &AtomicBool) -> Result<(), String> {
    if !model.is_dir() {
        return Ok(());
    }
    for folder in ["transformer", "unet"] {
        let dir = model.join(folder);
        if dir.is_dir() {
            merge_dir(&dir, cancel)?;
        }
    }
    Ok(())
}

fn merge_dir(dir: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let shards = shard_files(dir)
        .iter()
        .map(|path| read_shard(path))
        .collect::<Result<Vec<_>, _>>()?;
    let side = dir.join(SIDE);
    let pending: Vec<&Shard> = shards.iter().filter(|shard| needs_work(shard)).collect();
    if pending.is_empty() {
        let _ = std::fs::remove_file(&side);
        return Ok(());
    }
    eprintln!("voxel: mesclando a LoRA PEFT em {}", dir.display());
    if !side.is_file() {
        save_loras(&shards, &side)?;
    }
    let loras = load_tensors(&side)?;
    for shard in pending {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelado".to_owned());
        }
        rewrite_shard(shard, &loras, cancel)?;
    }
    rewrite_index(dir)?;
    std::fs::remove_file(&side).map_err(|error| error.to_string())
}

fn needs_work(shard: &Shard) -> bool {
    shard
        .tensors
        .iter()
        .any(|tensor| tensor.name.contains(BASE) || is_lora(&tensor.name))
}

fn is_lora(name: &str) -> bool {
    name.contains(".lora_A.") || name.contains(".lora_B.")
}

fn shard_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".safetensors") && name != SIDE {
                found.insert(entry.path());
            }
        }
    }
    found.into_iter().collect()
}

fn read_shard(path: &Path) -> Result<Shard, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut len = [0; 8];
    file.read_exact(&mut len)
        .map_err(|error| error.to_string())?;
    let len = u64::from_le_bytes(len);
    if len > MAX_HEADER {
        return Err("cabeçalho do safetensors grande demais".to_owned());
    }
    let mut header = vec![0; usize::try_from(len).map_err(|error| error.to_string())?];
    file.read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&header).map_err(|error| error.to_string())?;
    let map = value.as_object().ok_or("cabeçalho inválido")?;
    let mut tensors = Vec::new();
    for (name, info) in map {
        if name == "__metadata__" {
            continue;
        }
        tensors.push(parse_tensor(name, info).ok_or("tensor inválido")?);
    }
    tensors.sort_by_key(|tensor| tensor.start);
    Ok(Shard {
        path: path.to_path_buf(),
        data_start: 8 + len,
        tensors,
    })
}

fn parse_tensor(name: &str, info: &serde_json::Value) -> Option<Tensor> {
    let dtype = info.get("dtype")?.as_str()?.to_owned();
    let shape = info
        .get("shape")?
        .as_array()?
        .iter()
        .map(|dim| dim.as_u64().and_then(|dim| usize::try_from(dim).ok()))
        .collect::<Option<Vec<_>>>()?;
    let offsets = info.get("data_offsets")?.as_array()?;
    Some(Tensor {
        name: name.to_owned(),
        dtype,
        shape,
        start: offsets.first()?.as_u64()?,
        end: offsets.get(1)?.as_u64()?,
    })
}

fn read_bytes(file: &mut File, shard_start: u64, tensor: &Tensor) -> Result<Vec<u8>, String> {
    let len = usize::try_from(tensor.end - tensor.start).map_err(|error| error.to_string())?;
    let mut bytes = vec![0; len];
    file.seek(SeekFrom::Start(shard_start + tensor.start))
        .map_err(|error| error.to_string())?;
    file.read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn save_loras(shards: &[Shard], side: &Path) -> Result<(), String> {
    let mut picked = Vec::new();
    for shard in shards {
        let mut file = File::open(&shard.path).map_err(|error| error.to_string())?;
        for tensor in shard.tensors.iter().filter(|tensor| is_lora(&tensor.name)) {
            picked.push((
                tensor.clone(),
                read_bytes(&mut file, shard.data_start, tensor)?,
            ));
        }
    }
    let tmp = side.with_extension("tmp");
    write_safetensors(&tmp, &picked)?;
    std::fs::rename(&tmp, side).map_err(|error| error.to_string())
}

fn load_tensors(path: &Path) -> Result<HashMap<String, (Tensor, Vec<u8>)>, String> {
    let shard = read_shard(path)?;
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut out = HashMap::new();
    for tensor in &shard.tensors {
        let bytes = read_bytes(&mut file, shard.data_start, tensor)?;
        out.insert(tensor.name.clone(), (tensor.clone(), bytes));
    }
    Ok(out)
}

fn write_safetensors(path: &Path, tensors: &[(Tensor, Vec<u8>)]) -> Result<(), String> {
    let layout: Vec<Tensor> = tensors.iter().map(|(tensor, _)| tensor.clone()).collect();
    let mut out = start_file(path, &layout)?;
    for (_, bytes) in tensors {
        out.write_all(bytes).map_err(|error| error.to_string())?;
    }
    out.flush().map_err(|error| error.to_string())
}

fn start_file(path: &Path, tensors: &[Tensor]) -> Result<BufWriter<File>, String> {
    let mut header = serde_json::Map::new();
    header.insert(
        "__metadata__".to_owned(),
        serde_json::json!({ "format": "pt" }),
    );
    let mut offset = 0_u64;
    for tensor in tensors {
        let size = tensor.end - tensor.start;
        header.insert(
            tensor.name.clone(),
            serde_json::json!({
                "dtype": tensor.dtype,
                "shape": tensor.shape,
                "data_offsets": [offset, offset + size],
            }),
        );
        offset += size;
    }
    let mut text = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    while text.len() % 8 != 0 {
        text.push(b' ');
    }
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut out = BufWriter::with_capacity(8 * 1_048_576, file);
    let len = u64::try_from(text.len()).map_err(|error| error.to_string())?;
    out.write_all(&len.to_le_bytes())
        .map_err(|error| error.to_string())?;
    out.write_all(&text).map_err(|error| error.to_string())?;
    Ok(out)
}

fn rewrite_shard(
    shard: &Shard,
    loras: &HashMap<String, (Tensor, Vec<u8>)>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let kept: Vec<&Tensor> = shard
        .tensors
        .iter()
        .filter(|tensor| !is_lora(&tensor.name))
        .collect();
    let layout: Vec<Tensor> = kept
        .iter()
        .map(|tensor| Tensor {
            name: tensor.name.replace(BASE, "."),
            ..(*tensor).clone()
        })
        .collect();
    let tmp = shard.path.with_extension("voxel-tmp");
    let mut out = start_file(&tmp, &layout)?;
    let mut source = File::open(&shard.path).map_err(|error| error.to_string())?;
    for tensor in kept {
        if cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = std::fs::remove_file(&tmp);
            return Err("cancelado".to_owned());
        }
        let mut bytes = read_bytes(&mut source, shard.data_start, tensor)?;
        if let Some(prefix) = tensor.name.strip_suffix(".base_layer.weight") {
            for (a, b) in pairs_for(prefix, loras) {
                merge_into(&mut bytes, tensor, a, b)?;
            }
        }
        out.write_all(&bytes).map_err(|error| error.to_string())?;
    }
    out.flush().map_err(|error| error.to_string())?;
    drop(out);
    std::fs::rename(&tmp, &shard.path).map_err(|error| error.to_string())
}

type Pair<'a> = (&'a (Tensor, Vec<u8>), &'a (Tensor, Vec<u8>));

fn pairs_for<'a>(prefix: &str, loras: &'a HashMap<String, (Tensor, Vec<u8>)>) -> Vec<Pair<'a>> {
    let head = format!("{prefix}.lora_A.");
    let mut names: Vec<&String> = loras
        .keys()
        .filter(|name| name.starts_with(&head) && name.ends_with(".weight"))
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let adapter = &name[head.len()..name.len() - ".weight".len()];
            let b = loras.get(&format!("{prefix}.lora_B.{adapter}.weight"))?;
            Some((loras.get(name)?, b))
        })
        .collect()
}

/// `W += B·A`, com escala 1: o pacote não traz o `alpha` da `LoRA`.
fn merge_into(
    weight: &mut [u8],
    tensor: &Tensor,
    a: &(Tensor, Vec<u8>),
    b: &(Tensor, Vec<u8>),
) -> Result<(), String> {
    let [rows, cols] = tensor.shape[..] else {
        return Err(format!("{} não é uma matriz", tensor.name));
    };
    let rank = a.0.shape.first().copied().unwrap_or(0);
    if a.0.shape != [rank, cols] || b.0.shape != [rows, rank] {
        return Err(format!("LoRA com formato diferente em {}", tensor.name));
    }
    let a = to_f32(&a.1, &a.0.dtype)?;
    let b = to_f32(&b.1, &b.0.dtype)?;
    let width = element_size(&tensor.dtype)?;
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunk_rows = rows.div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (index, chunk) in weight.chunks_mut(chunk_rows * cols * width).enumerate() {
            let (a, b) = (&a, &b);
            let dtype = tensor.dtype.as_str();
            scope.spawn(move || {
                let mut row = vec![0_f32; cols];
                for (offset, bytes) in chunk.chunks_mut(cols * width).enumerate() {
                    let r = index * chunk_rows + offset;
                    decode_row(bytes, dtype, &mut row);
                    for k in 0..rank {
                        let factor = b[r * rank + k];
                        if factor == 0.0 {
                            continue;
                        }
                        let source = &a[k * cols..(k + 1) * cols];
                        for (value, add) in row.iter_mut().zip(source) {
                            *value += factor * add;
                        }
                    }
                    encode_row(&row, dtype, bytes);
                }
            });
        }
    });
    Ok(())
}

fn element_size(dtype: &str) -> Result<usize, String> {
    match dtype {
        "BF16" => Ok(2),
        "F32" => Ok(4),
        other => Err(format!("tipo {other} sem suporte na mesclagem da LoRA")),
    }
}

fn to_f32(bytes: &[u8], dtype: &str) -> Result<Vec<f32>, String> {
    let width = element_size(dtype)?;
    let mut out = vec![0_f32; bytes.len() / width];
    decode_row(bytes, dtype, &mut out);
    Ok(out)
}

fn decode_row(bytes: &[u8], dtype: &str, out: &mut [f32]) {
    if dtype == "BF16" {
        for (value, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
            *value = f32::from_bits(u32::from(u16::from_le_bytes([pair[0], pair[1]])) << 16);
        }
    } else {
        for (value, quad) in out.iter_mut().zip(bytes.chunks_exact(4)) {
            *value = f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]);
        }
    }
}

fn encode_row(row: &[f32], dtype: &str, bytes: &mut [u8]) {
    if dtype == "BF16" {
        for (value, pair) in row.iter().zip(bytes.chunks_exact_mut(2)) {
            pair.copy_from_slice(&to_bf16(*value).to_le_bytes());
        }
    } else {
        for (value, quad) in row.iter().zip(bytes.chunks_exact_mut(4)) {
            quad.copy_from_slice(&value.to_le_bytes());
        }
    }
}

fn to_bf16(value: f32) -> u16 {
    if value.is_nan() {
        return 0x7FC0;
    }
    let bits = value.to_bits();
    let rounded = bits.wrapping_add(0x7FFF + ((bits >> 16) & 1));
    u16::try_from(rounded >> 16).unwrap_or(0x7FC0)
}

fn rewrite_index(dir: &Path) -> Result<(), String> {
    let Some(index) = std::fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".safetensors.index.json"))
        })
    else {
        return Ok(());
    };
    let text = std::fs::read_to_string(&index).map_err(|error| error.to_string())?;
    let mut value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut map = serde_json::Map::new();
    for path in shard_files(dir) {
        let shard = read_shard(&path)?;
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        for tensor in shard.tensors {
            map.insert(tensor.name, serde_json::Value::String(file.clone()));
        }
    }
    value["weight_map"] = serde_json::Value::Object(map);
    let tmp = index.with_extension("voxel-tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::rename(&tmp, &index).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{Tensor, prepare, read_shard, to_bf16, write_safetensors};
    use std::sync::atomic::AtomicBool;

    fn bf16(values: &[f32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| to_bf16(*value).to_le_bytes())
            .collect()
    }

    fn tensor(name: &str, shape: Vec<usize>, bytes: &[u8]) -> (Tensor, Vec<u8>) {
        let tensor = Tensor {
            name: name.to_owned(),
            dtype: "BF16".to_owned(),
            shape,
            start: 0,
            end: u64::try_from(bytes.len()).unwrap(),
        };
        (tensor, bytes.to_vec())
    }

    #[test]
    fn mescla_o_pacote_da_variavel() {
        let Ok(model) = std::env::var("VOXEL_PEFT_MODEL") else {
            return;
        };
        prepare(std::path::Path::new(&model), &AtomicBool::new(false)).unwrap();
    }

    #[test]
    fn a_lora_solta_vira_um_peso_so() {
        let root = std::env::temp_dir().join(format!("voxel-peft-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("transformer");
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("diffusion_pytorch_model-00001-of-00002.safetensors");
        let second = dir.join("diffusion_pytorch_model-00002-of-00002.safetensors");
        write_safetensors(
            &first,
            &[
                tensor(
                    "blk.to_q.base_layer.weight",
                    vec![2, 2],
                    &bf16(&[1.0, 2.0, 3.0, 4.0]),
                ),
                tensor("blk.to_q.base_layer.bias", vec![2], &bf16(&[0.5, 0.5])),
                tensor("norm.weight", vec![2], &bf16(&[1.0, 1.0])),
            ],
        )
        .unwrap();
        write_safetensors(
            &second,
            &[
                tensor(
                    "blk.to_q.lora_A.unce.weight",
                    vec![1, 2],
                    &bf16(&[1.0, 2.0]),
                ),
                tensor(
                    "blk.to_q.lora_B.unce.weight",
                    vec![2, 1],
                    &bf16(&[1.0, -1.0]),
                ),
            ],
        )
        .unwrap();
        std::fs::write(
            dir.join("diffusion_pytorch_model.safetensors.index.json"),
            r#"{"metadata":{},"weight_map":{}}"#,
        )
        .unwrap();
        prepare(&root, &AtomicBool::new(false)).unwrap();
        let merged = read_shard(&first).unwrap();
        let names: Vec<&str> = merged.tensors.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"blk.to_q.weight"));
        assert!(names.contains(&"blk.to_q.bias"));
        assert!(!names.iter().any(|name| name.contains("base_layer")));
        let weight = merged
            .tensors
            .iter()
            .find(|t| t.name == "blk.to_q.weight")
            .unwrap();
        let bytes = std::fs::read(&first).unwrap();
        let start = usize::try_from(merged.data_start + weight.start).unwrap();
        assert_eq!(
            &bytes[start..start + 8],
            bf16(&[2.0, 4.0, 2.0, 2.0]).as_slice()
        );
        assert!(read_shard(&second).unwrap().tensors.is_empty());
        let index =
            std::fs::read_to_string(dir.join("diffusion_pytorch_model.safetensors.index.json"))
                .unwrap();
        assert!(index.contains("blk.to_q.weight"));
        assert!(!dir.join("voxel-lora.safetensors").exists());
        prepare(&root, &AtomicBool::new(false)).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
