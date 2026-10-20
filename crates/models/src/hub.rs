//! Busca no Hugging Face e o pedido de download de um arquivo escolhido.

use std::fmt::Write;

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use serde::Serialize;

use crate::catalog::ModelError;
use crate::download::DownloadJob;

const SEARCH_LIMIT: usize = 5;
const FILE_LIMIT: usize = 20;

/// Um `.safetensors` ou `.gguf` publicado no Hub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubFile {
    pub repo: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub license: String,
    pub gated: bool,
}

/// Procura repositórios e devolve só os pesos com tamanho conhecido.
///
/// # Errors
///
/// Falha se a busca estiver vazia ou se a rede recusar o primeiro pedido.
pub fn search(query: &str, token: Option<&str>) -> Result<Vec<HubFile>, ModelError> {
    search_with("https://huggingface.co", query, token)
}

/// Igual a [`search`], com a raiz da API trocada nos testes.
///
/// # Errors
///
/// Falha se a busca estiver vazia ou se a rede recusar o primeiro pedido.
pub fn search_with(
    api: &str,
    query: &str,
    token: Option<&str>,
) -> Result<Vec<HubFile>, ModelError> {
    let query = query.trim();
    if query.is_empty() || query.len() > 80 {
        return Err(ModelError::Invalid("busca vazia ou longa".to_owned()));
    }
    let client = http_client()?;
    let list = http_get(
        &client,
        &format!(
            "{api}/api/models?search={}&limit={SEARCH_LIMIT}",
            encode(query)
        ),
        token,
    )?;
    let mut files = Vec::new();
    for repo in parse_repo_ids(&list)?.into_iter().take(SEARCH_LIMIT) {
        if !valid_repo(&repo) {
            continue;
        }
        let Ok(detail) = http_get(
            &client,
            &format!("{api}/api/models/{repo}?blobs=true"),
            token,
        ) else {
            continue;
        };
        let Ok(mut found) = parse_repo(&detail) else {
            continue;
        };
        files.append(&mut found);
        if files.len() >= FILE_LIMIT {
            files.truncate(FILE_LIMIT);
            break;
        }
    }
    Ok(files)
}

/// Relê o repositório e devolve o arquivo pedido, com o tamanho e o hash do Hub.
///
/// # Errors
///
/// Falha se o nome for inválido, a rede cair ou o arquivo não estiver no repositório.
pub fn find_file(
    api: &str,
    repo: &str,
    filename: &str,
    token: Option<&str>,
) -> Result<HubFile, ModelError> {
    if !valid_repo(repo) || !valid_filename(filename) {
        return Err(ModelError::Invalid("arquivo inválido".to_owned()));
    }
    let client = http_client()?;
    let detail = http_get(
        &client,
        &format!("{api}/api/models/{repo}?blobs=true"),
        token,
    )?;
    parse_repo(&detail)?
        .into_iter()
        .find(|file| file.repo == repo && file.filename == filename)
        .ok_or_else(|| ModelError::Invalid("arquivo ausente".to_owned()))
}

/// Monta o download. Repositório restrito sem token não abre a rede.
///
/// # Errors
///
/// Falha com [`ModelError::TokenRequired`] quando o token está vazio.
pub fn hub_job(file: &HubFile, token: Option<&str>) -> Result<DownloadJob, ModelError> {
    let authorization = if file.gated {
        let Some(value) = token.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(ModelError::TokenRequired);
        };
        Some(format!("Bearer {value}"))
    } else {
        None
    };
    Ok(DownloadJob {
        url: file_url(&file.repo, &file.filename),
        bytes: file.bytes,
        sha256: file.sha256.clone(),
        authorization,
    })
}

/// Nome seguro na pasta de modelos. Barras viram `__`.
#[must_use]
pub fn disk_name(filename: &str) -> String {
    filename.replace('/', "__")
}

fn parse_repo_ids(body: &str) -> Result<Vec<String>, ModelError> {
    let hits: Vec<SearchHit> = serde_json::from_str(body)?;
    Ok(hits.into_iter().map(|hit| hit.id).collect())
}

fn parse_repo(body: &str) -> Result<Vec<HubFile>, ModelError> {
    let repo: RepoBody = serde_json::from_str(body)?;
    if !valid_repo(&repo.id) {
        return Ok(Vec::new());
    }
    let license = license_text(repo.card.and_then(|card| card.license));
    let gated = is_gated(repo.gated);
    let mut files = Vec::new();
    for sibling in repo.siblings {
        if !valid_filename(&sibling.rfilename) {
            continue;
        }
        let Some(bytes) = sibling
            .size
            .or(sibling.lfs.as_ref().and_then(|lfs| lfs.size))
        else {
            continue;
        };
        if bytes == 0 {
            continue;
        }
        let sha256 = sibling
            .lfs
            .as_ref()
            .and_then(|lfs| lfs.sha256.clone())
            .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .unwrap_or_default();
        files.push(HubFile {
            repo: repo.id.clone(),
            filename: sibling.rfilename,
            bytes,
            sha256,
            license: license.clone(),
            gated,
        });
    }
    Ok(files)
}

fn file_url(repo: &str, filename: &str) -> String {
    let path = filename
        .split('/')
        .map(encode)
        .collect::<Vec<_>>()
        .join("/");
    format!("https://huggingface.co/{repo}/resolve/main/{path}")
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let Some(owner) = parts.next() else {
        return false;
    };
    let Some(name) = parts.next() else {
        return false;
    };
    parts.next().is_none() && segment(owner) && segment(name)
}

fn valid_filename(filename: &str) -> bool {
    let extension = std::path::Path::new(filename)
        .extension()
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("safetensors") || ext.eq_ignore_ascii_case("gguf")
        });
    if !extension {
        return false;
    }
    let parts = filename.split('/').collect::<Vec<_>>();
    !parts.is_empty()
        && parts.len() <= 4
        && parts
            .iter()
            .all(|part| segment(part) && *part != "." && *part != "..")
}

fn segment(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn is_gated(gated: Gated) -> bool {
    match gated {
        Gated::Flag(value) => value,
        Gated::Mode(mode) => !mode.is_empty(),
    }
}

fn license_text(license: Option<LicenseValue>) -> String {
    match license {
        Some(LicenseValue::Text(text)) => text,
        Some(LicenseValue::List(items)) => items
            .into_iter()
            .find(|item| !item.is_empty())
            .unwrap_or_default(),
        None => String::new(),
    }
}

fn http_client() -> Result<Client, ModelError> {
    Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| ModelError::Network(error.to_string()))
}

fn http_get(client: &Client, url: &str, token: Option<&str>) -> Result<String, ModelError> {
    let mut request = client.get(url).header(USER_AGENT, "voxel");
    if let Some(value) = token.map(str::trim).filter(|value| !value.is_empty()) {
        request = request.header(AUTHORIZATION, format!("Bearer {value}"));
    }
    let response = request
        .send()
        .map_err(|error| ModelError::Network(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ModelError::Status(status.as_u16()));
    }
    response
        .text()
        .map_err(|error| ModelError::Network(error.to_string()))
}

fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

#[derive(Deserialize)]
struct SearchHit {
    id: String,
}

#[derive(Deserialize)]
struct RepoBody {
    id: String,
    #[serde(default)]
    gated: Gated,
    #[serde(default, rename = "cardData")]
    card: Option<Card>,
    #[serde(default)]
    siblings: Vec<Sibling>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Gated {
    Flag(bool),
    Mode(String),
}

impl Default for Gated {
    fn default() -> Self {
        Self::Flag(false)
    }
}

#[derive(Default, Deserialize)]
struct Card {
    license: Option<LicenseValue>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LicenseValue {
    Text(String),
    List(Vec<String>),
}

#[derive(Deserialize)]
struct Sibling {
    rfilename: String,
    size: Option<u64>,
    lfs: Option<Lfs>,
}

#[derive(Deserialize)]
struct Lfs {
    sha256: Option<String>,
    size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::{disk_name, find_file, hub_job, parse_repo, search_with};

    const DETAIL: &str = r#"{
        "id": "acme/gato",
        "gated": "manual",
        "cardData": { "license": ["mit", ""] },
        "siblings": [
            { "rfilename": "README.md", "size": 10 },
            { "rfilename": "peso.safetensors", "size": 12, "lfs": { "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "size": 12 } },
            { "rfilename": "v2/outro.gguf", "lfs": { "size": 4 } },
            { "rfilename": "../mau.safetensors", "size": 8 }
        ]
    }"#;

    #[test]
    fn separa_os_pesos_e_ignora_o_resto() {
        let files = parse_repo(DETAIL).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filename, "peso.safetensors");
        assert_eq!(files[0].bytes, 12);
        assert_eq!(files[0].license, "mit");
        assert!(files[0].gated);
        assert_eq!(files[0].sha256.len(), 64);
        assert_eq!(files[1].filename, "v2/outro.gguf");
        assert!(files[1].sha256.is_empty());
        assert_eq!(disk_name(&files[1].filename), "v2__outro.gguf");
    }

    #[test]
    fn repositorio_restrito_sem_token_nao_abre_a_rede() {
        let file = parse_repo(DETAIL).unwrap().remove(0);
        assert_eq!(
            hub_job(&file, None).unwrap_err().to_string(),
            "token ausente"
        );
        let job = hub_job(&file, Some("hf_segredo")).unwrap();
        assert!(job.url.contains("acme/gato/resolve/main/peso.safetensors"));
        assert!(job.authorization.is_some());
        assert!(!format!("{job:?}").contains("hf_segredo"));
    }

    #[test]
    fn a_busca_junta_os_arquivos_do_repositorio() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let read = std::io::Read::read(&mut stream, &mut buffer).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let text = String::from_utf8_lossy(&request);
                let body = if text.contains("/api/models?") {
                    r#"[{"id":"acme/gato"},{"id":"../mau"}]"#
                } else {
                    DETAIL
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, header.as_bytes());
                let _ = std::io::Write::write_all(&mut stream, body.as_bytes());
            }
        });
        let files = search_with(
            &format!("http://127.0.0.1:{port}"),
            "gato",
            Some("hf_teste"),
        )
        .unwrap();
        assert_eq!(files.len(), 2);
        let again = find_file(
            &format!("http://127.0.0.1:{port}"),
            "acme/gato",
            "peso.safetensors",
            None,
        )
        .unwrap();
        assert_eq!(again.bytes, 12);
    }
}
