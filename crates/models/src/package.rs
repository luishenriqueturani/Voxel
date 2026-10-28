//! Agrupa os arquivos de um repositório do Hub em pacotes que se baixam juntos.

use serde::Serialize;

use crate::hub::HubFile;

const COMPONENTS: &[&str] = &[
    "unet",
    "transformer",
    "vae",
    "text_encoder",
    "text_encoder_2",
    "tokenizer",
    "tokenizer_2",
];

const ROLE_ORDER: &[&str] = &[
    "text_encoder",
    "text_encoder_2",
    "unet",
    "transformer",
    "vae",
    "tokenizer",
    "tokenizer_2",
    "index",
    "file",
];

/// Uma parte de um pacote: peso, fatia ou índice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubPart {
    pub repo: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub role: String,
    pub shard_index: Option<u32>,
    pub shard_count: Option<u32>,
}

/// Pacote que a busca mostra num cartão só.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubPackage {
    pub repo: String,
    pub name: String,
    pub folder: String,
    pub license: String,
    pub gated: bool,
    pub opener: String,
    pub directory: bool,
    pub marker: String,
    pub parts: Vec<HubPart>,
}

#[derive(Clone, PartialEq, Eq)]
enum Key {
    Tree {
        repo: String,
        folder: String,
    },
    Shards {
        repo: String,
        dir: String,
        stem: String,
    },
    Single {
        repo: String,
        filename: String,
    },
}

struct Classified {
    key: Key,
    role: String,
    shard_index: Option<u32>,
    shard_count: Option<u32>,
}

struct Draft {
    key: Key,
    license: String,
    gated: bool,
    parts: Vec<HubPart>,
}

/// Junta fatias, componentes Diffusers e arquivos soltos.
#[must_use]
pub fn group_packages(files: &[HubFile]) -> Vec<HubPackage> {
    let mut drafts: Vec<Draft> = Vec::new();
    for file in files {
        let classified = classify(file);
        let part = HubPart {
            repo: file.repo.clone(),
            filename: file.filename.clone(),
            bytes: file.bytes,
            sha256: file.sha256.clone(),
            role: classified.role,
            shard_index: classified.shard_index,
            shard_count: classified.shard_count,
        };
        if let Some(index) = drafts.iter().position(|draft| draft.key == classified.key) {
            drafts[index].parts.push(part);
        } else {
            drafts.push(Draft {
                key: classified.key,
                license: file.license.clone(),
                gated: file.gated,
                parts: vec![part],
            });
        }
    }
    drafts.into_iter().map(finish_draft).collect()
}

/// Devolve no máximo `limit` pacotes inteiros, sem cortar uma parte no meio.
#[must_use]
pub fn packages_within(files: &[HubFile], limit: usize) -> Vec<HubPackage> {
    let mut packages = Vec::new();
    for package in group_packages(files) {
        if packages.len() >= limit {
            break;
        }
        packages.push(package);
    }
    packages
}

fn finish_draft(draft: Draft) -> HubPackage {
    let mut parts = draft.parts;
    parts.sort_by(part_order);
    let repo = repo_of(&draft.key);
    let (name, folder, opener, directory, marker) = identity(&draft.key);
    HubPackage {
        repo,
        name,
        folder,
        license: draft.license,
        gated: draft.gated,
        opener,
        directory,
        marker,
        parts,
    }
}

fn identity(key: &Key) -> (String, String, String, bool, String) {
    match key {
        Key::Tree { repo, folder } => (
            package_title(repo, folder),
            folder.clone(),
            folder.clone(),
            true,
            "voxel-package.json".to_owned(),
        ),
        Key::Shards { repo, dir, stem } => (
            format!("{} · {stem}", repo_title(repo)),
            dir.clone(),
            dir.clone(),
            true,
            format!("voxel-package-{stem}.json"),
        ),
        Key::Single { filename, .. } => (
            filename.clone(),
            String::new(),
            filename.clone(),
            false,
            String::new(),
        ),
    }
}

fn repo_of(key: &Key) -> String {
    match key {
        Key::Tree { repo, .. } | Key::Shards { repo, .. } | Key::Single { repo, .. } => {
            repo.clone()
        }
    }
}

fn package_title(repo: &str, folder: &str) -> String {
    let title = repo_title(repo);
    if folder.is_empty() {
        title.to_owned()
    } else {
        let leaf = folder.rsplit('/').next().unwrap_or(folder);
        format!("{title} · {leaf}")
    }
}

fn repo_title(repo: &str) -> &str {
    repo.split('/').nth(1).unwrap_or(repo)
}

fn part_order(left: &HubPart, right: &HubPart) -> std::cmp::Ordering {
    role_rank(&left.role)
        .cmp(&role_rank(&right.role))
        .then_with(|| {
            left.shard_index
                .unwrap_or(u32::MAX)
                .cmp(&right.shard_index.unwrap_or(u32::MAX))
        })
        .then_with(|| left.filename.cmp(&right.filename))
}

fn role_rank(role: &str) -> usize {
    ROLE_ORDER
        .iter()
        .position(|item| *item == role)
        .unwrap_or(ROLE_ORDER.len())
}

fn classify(file: &HubFile) -> Classified {
    let segments: Vec<&str> = file.filename.split('/').collect();
    let base = segments.last().copied().unwrap_or(file.filename.as_str());
    let dirs = &segments[..segments.len().saturating_sub(1)];
    let shard = shard_of(base);
    if let Some(position) = dirs.iter().position(|dir| is_component(dir)) {
        return Classified {
            key: Key::Tree {
                repo: file.repo.clone(),
                folder: dirs[..position].join("/"),
            },
            role: if is_index_name(base) {
                "index".to_owned()
            } else {
                (*dirs[position]).to_owned()
            },
            shard_index: shard.as_ref().map(|(_, index, _)| *index),
            shard_count: shard.as_ref().map(|(_, _, total)| *total),
        };
    }
    if base == "model_index.json" {
        return Classified {
            key: Key::Tree {
                repo: file.repo.clone(),
                folder: dirs.join("/"),
            },
            role: "index".to_owned(),
            shard_index: None,
            shard_count: None,
        };
    }
    if let Some(stem) = base.strip_suffix(".safetensors.index.json") {
        return Classified {
            key: Key::Shards {
                repo: file.repo.clone(),
                dir: dirs.join("/"),
                stem: stem.to_owned(),
            },
            role: "index".to_owned(),
            shard_index: None,
            shard_count: None,
        };
    }
    if let Some((stem, index, total)) = shard {
        return Classified {
            key: Key::Shards {
                repo: file.repo.clone(),
                dir: dirs.join("/"),
                stem,
            },
            role: "file".to_owned(),
            shard_index: Some(index),
            shard_count: Some(total),
        };
    }
    Classified {
        key: Key::Single {
            repo: file.repo.clone(),
            filename: file.filename.clone(),
        },
        role: "file".to_owned(),
        shard_index: None,
        shard_count: None,
    }
}

fn is_component(name: &str) -> bool {
    COMPONENTS.contains(&name)
}

fn is_index_name(base: &str) -> bool {
    base == "model_index.json" || base.ends_with(".safetensors.index.json")
}

fn shard_of(base: &str) -> Option<(String, u32, u32)> {
    let stem = base
        .strip_suffix(".safetensors")
        .or_else(|| base.strip_suffix(".gguf"))?;
    let (left, total_text) = stem.rsplit_once("-of-")?;
    let (name, index_text) = left.rsplit_once('-')?;
    if name.is_empty() {
        return None;
    }
    let index = index_text.parse::<u32>().ok()?;
    let total = total_text.parse::<u32>().ok()?;
    if index == 0 || total == 0 || index > total {
        return None;
    }
    Some((name.to_owned(), index, total))
}

#[cfg(test)]
mod tests {
    use super::{group_packages, packages_within};
    use crate::hub::HubFile;

    fn file(repo: &str, filename: &str, bytes: u64) -> HubFile {
        HubFile {
            repo: repo.to_owned(),
            filename: filename.to_owned(),
            bytes,
            sha256: String::new(),
            license: "mit".to_owned(),
            gated: false,
        }
    }

    #[test]
    fn o_diffusers_fatiado_e_o_arquivo_solto_sao_dois_pacotes() {
        let files = vec![
            file(
                "acme/gato",
                "merged-model/text_encoder/model-00001-of-00004.safetensors",
                10,
            ),
            file(
                "acme/gato",
                "merged-model/text_encoder/model-00002-of-00004.safetensors",
                11,
            ),
            file(
                "acme/gato",
                "merged-model/text_encoder/model-00003-of-00004.safetensors",
                12,
            ),
            file(
                "acme/gato",
                "merged-model/text_encoder/model-00004-of-00004.safetensors",
                13,
            ),
            file(
                "acme/gato",
                "merged-model/transformer/diffusion_pytorch_model-00001-of-00003.safetensors",
                20,
            ),
            file(
                "acme/gato",
                "merged-model/transformer/diffusion_pytorch_model-00002-of-00003.safetensors",
                21,
            ),
            file(
                "acme/gato",
                "merged-model/transformer/diffusion_pytorch_model-00003-of-00003.safetensors",
                22,
            ),
            file(
                "acme/gato",
                "merged-model/transformer/diffusion_pytorch_model.safetensors.index.json",
                1,
            ),
            file(
                "acme/gato",
                "merged-model/vae/diffusion_pytorch_model.safetensors",
                30,
            ),
            file("acme/gato", "merged-model/model_index.json", 1),
            file("acme/gato", "lora.safetensors", 40),
        ];
        let packages = group_packages(&files);
        assert_eq!(packages.len(), 2);
        let tree = &packages[0];
        assert_eq!(tree.folder, "merged-model");
        assert_eq!(tree.name, "gato · merged-model");
        assert!(tree.directory);
        assert_eq!(tree.opener, "merged-model");
        assert_eq!(tree.marker, "voxel-package.json");
        assert_eq!(tree.parts.len(), 10);
        let encoder = tree
            .parts
            .iter()
            .find(|part| part.filename.ends_with("00002-of-00004.safetensors"))
            .unwrap();
        assert_eq!(encoder.role, "text_encoder");
        assert_eq!(encoder.shard_index, Some(2));
        assert_eq!(encoder.shard_count, Some(4));
        let roles: Vec<_> = tree.parts.iter().map(|part| part.role.as_str()).collect();
        let text = roles
            .iter()
            .position(|role| *role == "text_encoder")
            .unwrap();
        let transformer = roles
            .iter()
            .position(|role| *role == "transformer")
            .unwrap();
        let vae = roles.iter().position(|role| *role == "vae").unwrap();
        let index = roles.iter().position(|role| *role == "index").unwrap();
        assert!(text < transformer && transformer < vae && vae < index);
        let lora = &packages[1];
        assert!(!lora.directory);
        assert_eq!(lora.opener, "lora.safetensors");
        assert_eq!(lora.parts.len(), 1);
    }

    #[test]
    fn duas_quantizacoes_continuam_dois_pacotes() {
        let packages = group_packages(&[
            file("acme/gato", "modelo-Q4_K_M.gguf", 10),
            file("acme/gato", "modelo-Q8_0.gguf", 20),
        ]);
        assert_eq!(packages.len(), 2);
        assert!(packages.iter().all(|package| !package.directory));
    }

    #[test]
    fn fatias_de_nomes_diferentes_nao_se_misturam() {
        let packages = group_packages(&[
            file("acme/gato", "weights/a-00001-of-00002.safetensors", 1),
            file("acme/gato", "weights/a-00002-of-00002.safetensors", 1),
            file("acme/gato", "weights/b-00001-of-00002.safetensors", 1),
        ]);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].parts.len(), 2);
        assert_eq!(packages[1].parts.len(), 1);
        assert!(packages.iter().all(|package| package.directory));
        assert!(packages.iter().all(|package| package.opener == "weights"));
        assert!(
            packages
                .iter()
                .all(|package| package.marker.starts_with("voxel-package-"))
        );
    }

    #[test]
    fn componentes_na_raiz_formam_um_pacote() {
        let packages = group_packages(&[
            file("acme/gato", "unet/diffusion_pytorch_model.safetensors", 1),
            file("acme/gato", "vae/diffusion_pytorch_model.safetensors", 1),
            file("acme/gato", "text_encoder/model.safetensors", 1),
            file("acme/gato", "text_encoder_2/model.safetensors", 1),
            file("acme/gato", "model_index.json", 1),
            file("acme/gato", "extra.safetensors", 1),
        ]);
        assert_eq!(packages.len(), 2);
        assert!(packages[0].directory);
        assert_eq!(packages[0].folder, "");
        assert_eq!(packages[0].name, "gato");
        assert_eq!(packages[0].parts.len(), 5);
        assert!(
            packages[0]
                .parts
                .iter()
                .any(|part| part.role == "text_encoder_2")
        );
        assert!(!packages[1].directory);
        assert_eq!(packages[1].name, "extra.safetensors");
    }

    #[test]
    fn repositorios_diferentes_nao_se_juntam() {
        let packages = group_packages(&[
            file(
                "acme/gato",
                "merged-model/vae/diffusion_pytorch_model.safetensors",
                1,
            ),
            file(
                "acme/cao",
                "merged-model/vae/diffusion_pytorch_model.safetensors",
                1,
            ),
        ]);
        assert_eq!(packages.len(), 2);
    }

    #[test]
    fn o_limite_nao_parte_o_pacote() {
        let mut files = Vec::new();
        for index in 1..=25 {
            files.push(file(
                "acme/gato",
                &format!("merged-model/transformer/model-{index:05}-of-00025.safetensors"),
                1,
            ));
        }
        files.push(file("acme/gato", "lora.safetensors", 1));
        let kept = packages_within(&files, 1);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].parts.len(), 25);
        assert!(kept[0].parts.iter().all(|part| part.role == "transformer"));
    }

    #[test]
    fn o_tokenizer_entra_na_pasta_do_modelo() {
        let packages = group_packages(&[
            file("acme/gato", "merged-model/model_index.json", 1),
            file("acme/gato", "merged-model/tokenizer/tokenizer.json", 20),
            file(
                "acme/gato",
                "merged-model/transformer/diffusion_pytorch_model.safetensors",
                30,
            ),
        ]);
        assert_eq!(packages.len(), 1);
        assert!(packages[0].directory);
        let tokenizer = packages[0]
            .parts
            .iter()
            .find(|part| part.filename.ends_with("tokenizer.json"))
            .unwrap();
        assert_eq!(tokenizer.role, "tokenizer");
    }
}
