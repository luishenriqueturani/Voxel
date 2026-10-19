use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use voxel_protocol::Txt2ImgRequest;

use crate::args::{self, DECODE_FALLBACKS, DecodeFallback};

/// Processo em andamento, para o cancelamento matar o filho na hora.
#[derive(Debug, Default)]
pub struct Running {
    pub cancel: AtomicBool,
    pid: Mutex<Option<u32>>,
}

impl Running {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Ok(slot) = self.pid.lock()
            && let Some(pid) = *slot
        {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    }

    fn remember(&self, pid: u32) {
        if let Ok(mut slot) = self.pid.lock() {
            *slot = Some(pid);
        }
    }

    fn clear(&self) {
        if let Ok(mut slot) = self.pid.lock() {
            *slot = None;
        }
    }
}

/// Gera a imagem. Se o processo morrer com o latente já no disco, só o decode é refeito.
///
/// # Errors
///
/// Falha se o binário, o dispositivo ou a imagem não existirem.
pub fn txt2img(
    binary: &Path,
    model: &Path,
    backend: &str,
    request: &Txt2ImgRequest,
    running: &Running,
    mut on_progress: impl FnMut(i64, i64, Option<PathBuf>),
) -> Result<PathBuf, String> {
    let output = PathBuf::from(&request.output_path);
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let work = output.parent().unwrap_or(Path::new(".")).to_path_buf();
    let preview = work.join(format!("{}.preview.png", request.job_id));
    let latent = work.join(format!("{}.latent", request.job_id));
    let device = listed_device(binary, backend)?;
    let args = args::txt2img_args(model, &output, &preview, &device, request);
    let status = spawn(
        binary,
        &args,
        &[("VOXEL_LATENT_OUT", latent.to_str().unwrap_or_default())],
        running,
        &preview,
        &mut on_progress,
    )?;
    if status.success() && is_png(&output) {
        return Ok(output);
    }
    if running.cancel.load(Ordering::Relaxed) {
        return Err("cancelado".to_owned());
    }
    if !latent.is_file() {
        return Err(if status.detail.is_empty() {
            format!("saída {}", status.code.unwrap_or(-1))
        } else {
            status.detail
        });
    }
    decode_saved(binary, model, &output, &device, &latent, running)
}

/// Decodifica um latente já gravado, na ordem de fallback.
///
/// # Errors
///
/// Falha se nenhum decode produzir um PNG.
pub fn decode_saved(
    binary: &Path,
    model: &Path,
    output: &Path,
    device: &str,
    latent: &Path,
    running: &Running,
) -> Result<PathBuf, String> {
    for fallback in DECODE_FALLBACKS {
        let decoded = decode_once(binary, model, output, device, latent, *fallback, running)?;
        if decoded.success() && is_png(output) {
            return Ok(output.to_path_buf());
        }
        if running.cancel.load(Ordering::Relaxed) {
            return Err("cancelado".to_owned());
        }
    }
    Err("o decode do latente falhou".to_owned())
}

fn decode_once(
    binary: &Path,
    model: &Path,
    output: &Path,
    device: &str,
    latent: &Path,
    fallback: DecodeFallback,
    running: &Running,
) -> Result<Finished, String> {
    let args = args::decode_args(model, output, device, fallback);
    spawn(
        binary,
        &args,
        &[("VOXEL_LATENT_IN", latent.to_str().unwrap_or_default())],
        running,
        Path::new(""),
        &mut |_, _, _| {},
    )
}

struct Finished {
    code: Option<i32>,
    detail: String,
}

impl Finished {
    fn success(&self) -> bool {
        self.code == Some(0)
    }
}

pub(crate) fn listed_device(binary: &Path, backend: &str) -> Result<String, String> {
    let output = Command::new(binary)
        .arg("--list-devices")
        .env_remove("HSA_OVERRIDE_GFX_VERSION")
        .env_remove("PYTORCH_ROCM_ARCH")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("list-devices saiu com {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    args::device_for(backend, &args::parse_devices(&text))
        .ok_or_else(|| "dispositivo ausente".to_owned())
}

fn spawn(
    binary: &Path,
    args: &[String],
    env: &[(&str, &str)],
    running: &Running,
    preview: &Path,
    on_progress: &mut impl FnMut(i64, i64, Option<PathBuf>),
) -> Result<Finished, String> {
    if running.cancel.load(Ordering::Relaxed) {
        return Err("cancelado".to_owned());
    }
    let mut command = Command::new(binary);
    command
        .args(args)
        .env_remove("HSA_OVERRIDE_GFX_VERSION")
        .env_remove("PYTORCH_ROCM_ARCH")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    running.remember(child.id());
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let logged = Arc::new(Mutex::new(String::new()));
    let stderr_log = Arc::clone(&logged);
    let stderr_thread = thread::spawn(move || {
        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(mut slot) = stderr_log.lock() {
                    slot.push_str(&line);
                    if slot.len() > 4000 {
                        let keep_from = slot.len() - 2000;
                        slot.drain(..keep_from);
                    }
                }
                line.clear();
            }
        }
    });
    if let Some(stdout) = stdout {
        let mut reader = BufReader::new(stdout);
        let mut pending = String::new();
        let mut buffer = [0_u8; 256];
        loop {
            if running.cancel.load(Ordering::Relaxed) {
                running.cancel();
                break;
            }
            let read = std::io::Read::read(&mut reader, &mut buffer).unwrap_or(0);
            if read == 0 {
                break;
            }
            pending.push_str(&String::from_utf8_lossy(&buffer[..read]));
            if let Some((step, total)) = args::parse_progress(&pending) {
                let preview_path = preview.is_file().then(|| preview.to_path_buf());
                on_progress(step, total, preview_path);
                pending.clear();
            }
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    running.clear();
    let _ = stderr_thread.join();
    let detail = logged.lock().map(|text| text.clone()).unwrap_or_default();
    Ok(Finished {
        code: status.code(),
        detail,
    })
}

fn is_png(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() > 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    use voxel_protocol::Txt2ImgRequest;

    fn script(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("sd-cli");
        std::fs::write(&path, body).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn request(output: &Path) -> Txt2ImgRequest {
        Txt2ImgRequest {
            job_id: "job".to_owned(),
            prompt: "um quadrado".to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            output_path: output.display().to_string(),
            steps: 2,
            width: 512,
            height: 512,
            seed: 1,
        }
    }

    #[test]
    fn publica_o_passo_e_grava_o_png() {
        let dir = std::env::temp_dir().join(format!("voxel-sdcpp-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = script(
            &dir,
            r#"#!/bin/sh
if [ "$1" = "--list-devices" ]; then printf 'Vulkan0\tGPU\n'; exit 0; fi
prev=""
for arg in "$@"; do
  if [ "$prev" = "--preview-path" ]; then
    printf '\x89PNG\r\n\x1a\n' > "$arg"
    dd if=/dev/zero bs=64 count=1 >> "$arg" 2>/dev/null
  fi
  if [ "$prev" = "-o" ]; then
    printf '\x89PNG\r\n\x1a\n' > "$arg"
    dd if=/dev/zero bs=1100 count=1 >> "$arg" 2>/dev/null
  fi
  prev="$arg"
done
printf '\r1/2 - 1.00it/s'
exit 0
"#,
        );
        let mut seen = Vec::new();
        let output = dir.join("out.png");
        let path = txt2img(
            &binary,
            Path::new("modelo"),
            "vulkan",
            &request(&output),
            &Running::default(),
            |step, total, preview| seen.push((step, total, preview.is_some())),
        )
        .unwrap();
        assert_eq!(path, output);
        assert!(seen.contains(&(1, 2, true)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn o_latente_gravado_pula_o_denoising() {
        let dir = std::env::temp_dir().join(format!("voxel-sdcpp-latent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = script(
            &dir,
            r#"#!/bin/sh
if [ "$1" = "--list-devices" ]; then printf 'Vulkan0\tGPU\n'; exit 0; fi
if [ -n "$VOXEL_LATENT_IN" ]; then
  printf '%s' "$*" > "$VOXEL_LATENT_IN.seen"
  prev=""
  for arg in "$@"; do
    if [ "$prev" = "-o" ]; then
      printf '\x89PNG\r\n\x1a\n' > "$arg"
      dd if=/dev/zero bs=1100 count=1 >> "$arg" 2>/dev/null
    fi
    prev="$arg"
  done
  exit 0
fi
printf latente > "$VOXEL_LATENT_OUT"
exit 134
"#,
        );
        let output = dir.join("out.png");
        txt2img(
            &binary,
            Path::new("modelo"),
            "vulkan",
            &request(&output),
            &Running::default(),
            |_, _, _| {},
        )
        .unwrap();
        let seen = std::fs::read_to_string(dir.join("job.latent.seen")).unwrap();
        assert!(seen.contains("--vae-tiling"));
        assert!(!seen.contains("um quadrado"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelar_interrompe_a_espera() {
        let dir = std::env::temp_dir().join(format!("voxel-sdcpp-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = script(
            &dir,
            "#!/bin/sh\nif [ \"$1\" = \"--list-devices\" ]; then printf 'Vulkan0\\tGPU\\n'; exit 0; fi\nexec sleep 30\n",
        );
        let running = Arc::new(Running::default());
        let flag = Arc::clone(&running);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            flag.cancel();
        });
        let error = txt2img(
            &binary,
            Path::new("modelo"),
            "vulkan",
            &request(&dir.join("out.png")),
            &running,
            |_, _, _| {},
        )
        .unwrap_err();
        assert_eq!(error, "cancelado");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
