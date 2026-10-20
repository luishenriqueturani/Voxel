use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, RANGE};
use sha2::{Digest, Sha256};

use crate::catalog::{Model, ModelError};

/// Pedido já autorizado. O token não aparece na depuração.
#[derive(Clone, PartialEq, Eq)]
pub struct DownloadJob {
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub authorization: Option<String>,
}

impl std::fmt::Debug for DownloadJob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DownloadJob")
            .field("url", &self.url)
            .field("bytes", &self.bytes)
            .field("sha256", &self.sha256)
            .field(
                "authorization",
                &self.authorization.as_ref().map(|_| "presente"),
            )
            .finish()
    }
}

/// Monta o pedido. Modelo restrito sem token não abre a rede.
///
/// # Errors
///
/// Falha com [`ModelError::TokenRequired`] quando o token está vazio.
pub fn prepare_download(model: &Model, token: Option<&str>) -> Result<DownloadJob, ModelError> {
    let authorization = if model.gated {
        let Some(value) = token.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(ModelError::TokenRequired);
        };
        Some(format!("Bearer {value}"))
    } else {
        None
    };
    Ok(DownloadJob {
        url: model.source.url(),
        bytes: model.bytes,
        sha256: model.sha256.clone(),
        authorization,
    })
}

/// Baixa o arquivo, continua de um parcial e só troca o nome se o SHA-256 bater.
///
/// # Errors
///
/// Falha se a rede cair, o servidor recusar, o arquivo vier incompleto ou o hash não bater.
pub fn fetch(
    job: &DownloadJob,
    destination: &Path,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), ModelError> {
    if satisfied(destination, job)? {
        on_progress(job.bytes, job.bytes);
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = partial_path(destination);
    let mut offset = std::fs::metadata(&partial).map_or(0, |meta| meta.len());
    if offset > job.bytes {
        offset = 0;
    }
    if offset == job.bytes && satisfied(&partial, job)? {
        finish(destination, &partial)?;
        on_progress(job.bytes, job.bytes);
        return Ok(());
    }
    if offset == job.bytes {
        offset = 0;
    }

    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|error| ModelError::Network(error.to_string()))?;
    let mut request = client.get(&job.url);
    if offset > 0 {
        request = request.header(RANGE, format!("bytes={offset}-"));
    }
    if let Some(value) = &job.authorization {
        request = request.header(AUTHORIZATION, value);
    }
    let mut response = request
        .send()
        .map_err(|error| ModelError::Network(error.to_string()))?;
    let status = response.status();
    let resume = status == reqwest::StatusCode::PARTIAL_CONTENT && offset > 0;
    if status != reqwest::StatusCode::OK && !resume {
        return Err(ModelError::Status(status.as_u16()));
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&partial)?;
    let mut received = if resume {
        file.seek(SeekFrom::Start(offset))?;
        offset
    } else {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        0
    };
    on_progress(received, job.bytes);
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        received += u64::try_from(read).unwrap_or(0);
        on_progress(received, job.bytes);
    }
    file.sync_all()?;
    drop(file);
    if received != job.bytes {
        let _ = std::fs::remove_file(&partial);
        return Err(ModelError::Incomplete {
            received,
            expected: job.bytes,
        });
    }
    if !job.sha256.is_empty() && sha256_file(&partial)? != job.sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err(ModelError::HashMismatch);
    }
    finish(destination, &partial)?;
    Ok(())
}

fn satisfied(path: &Path, job: &DownloadJob) -> Result<bool, ModelError> {
    if !path.is_file() {
        return Ok(false);
    }
    if job.sha256.is_empty() {
        return Ok(std::fs::metadata(path)?.len() == job.bytes);
    }
    Ok(sha256_file(path)? == job.sha256)
}

fn finish(destination: &Path, partial: &Path) -> Result<(), ModelError> {
    if destination.exists() {
        std::fs::remove_file(destination)?;
    }
    std::fs::rename(partial, destination)?;
    Ok(())
}

fn partial_path(destination: &Path) -> std::path::PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".partial");
    std::path::PathBuf::from(name)
}

fn sha256_file(path: &Path) -> Result<String, ModelError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use sha2::{Digest, Sha256};

    use super::{fetch, prepare_download};
    use crate::ModelError;
    use crate::catalog::Catalog;

    fn digest(bytes: &[u8]) -> String {
        super::hex(&Sha256::digest(bytes))
    }

    struct Script {
        status: u16,
        body: Vec<u8>,
        honor_range: bool,
        seen: Arc<Mutex<Vec<String>>>,
    }

    fn serve(script: Script) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = stream.read(&mut buffer).unwrap_or(0);
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&request).into_owned();
            script.seen.lock().unwrap().push(text.clone());
            if script.status != 200 && script.status != 206 {
                let header = format!(
                    "HTTP/1.1 {} NO\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    script.status
                );
                let _ = stream.write_all(header.as_bytes());
                return;
            }
            let start = text.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if !name.eq_ignore_ascii_case("range") {
                    return None;
                }
                value
                    .trim()
                    .strip_prefix("bytes=")?
                    .split('-')
                    .next()?
                    .parse::<usize>()
                    .ok()
            });
            let (status, slice) = if script.honor_range
                && let Some(start) = start
            {
                (206, &script.body[start.min(script.body.len())..])
            } else {
                (200, script.body.as_slice())
            };
            let header = if status == 206 {
                format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                    slice.len(),
                    start.unwrap_or(0),
                    start.unwrap_or(0) + slice.len().saturating_sub(1),
                    script.body.len()
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    script.body.len()
                )
            };
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(slice);
        });
        port
    }

    #[test]
    fn continua_do_parcial_e_confere_o_hash() {
        let body = b"abcdefghijklmnopqrstuvwxyz012345".to_vec();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let port = serve(Script {
            status: 200,
            body: body.clone(),
            honor_range: true,
            seen: Arc::clone(&seen),
        });
        let dir = std::env::temp_dir().join(format!("voxel-download-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("peso.bin");
        std::fs::write(partial_for(&destination), &body[..8]).unwrap();
        let job = super::DownloadJob {
            url: format!("http://127.0.0.1:{port}/peso.bin"),
            bytes: body.len() as u64,
            sha256: digest(&body),
            authorization: Some("Bearer hf_teste".to_owned()),
        };
        let mut steps = Vec::new();
        fetch(&job, &destination, |received, total| {
            steps.push((received, total));
        })
        .unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), body);
        assert!(!partial_for(&destination).exists());
        assert_eq!(
            *steps.last().unwrap(),
            (body.len() as u64, body.len() as u64)
        );
        let headers = seen.lock().unwrap();
        let header = headers[0].to_ascii_lowercase();
        assert!(header.contains("range: bytes=8-"));
        assert!(header.contains("authorization: bearer hf_teste"));
        assert!(!format!("{job:?}").contains("hf_teste"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_errado_apaga_o_parcial() {
        let body = b"arquivo-errado".to_vec();
        let port = serve(Script {
            status: 200,
            body,
            honor_range: false,
            seen: Arc::new(Mutex::new(Vec::new())),
        });
        let dir = std::env::temp_dir().join(format!("voxel-download-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("peso.bin");
        let job = super::DownloadJob {
            url: format!("http://127.0.0.1:{port}/peso.bin"),
            bytes: 14,
            sha256: "ab".repeat(32),
            authorization: None,
        };
        let error = fetch(&job, &destination, |_, _| {}).unwrap_err();
        assert!(matches!(error, ModelError::HashMismatch));
        assert!(!destination.exists());
        assert!(!partial_for(&destination).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sem_hash_o_arquivo_completo_fica() {
        let body = b"sem-hash-ok".to_vec();
        let port = serve(Script {
            status: 200,
            body: body.clone(),
            honor_range: false,
            seen: Arc::new(Mutex::new(Vec::new())),
        });
        let dir = std::env::temp_dir().join(format!("voxel-download-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("peso.bin");
        let job = super::DownloadJob {
            url: format!("http://127.0.0.1:{port}/peso.bin"),
            bytes: body.len() as u64,
            sha256: String::new(),
            authorization: None,
        };
        fetch(&job, &destination, |_, _| {}).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), body);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn modelo_restrito_sem_token_nao_abre_a_rede() {
        let model = Catalog::bundled().unwrap().get("sf3d").unwrap().clone();
        let error = prepare_download(&model, None).unwrap_err();
        assert_eq!(error.to_string(), "token ausente");
        assert!(prepare_download(&model, Some("  ")).is_err());
        let job = prepare_download(&model, Some("hf_segredo")).unwrap();
        assert!(!format!("{job:?}").contains("hf_segredo"));
        assert!(job.authorization.is_some());
    }

    fn partial_for(path: &std::path::Path) -> std::path::PathBuf {
        super::partial_path(path)
    }
}
