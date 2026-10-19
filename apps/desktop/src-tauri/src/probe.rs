use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use voxel_hardware::{ImageBackend, ProbeVerdict, device_for, generation_args, parse_devices};

const LIST_LIMIT: Duration = Duration::from_secs(30);
const GENERATE_LIMIT: Duration = Duration::from_secs(180);

/// Binários do stable-diffusion.cpp e o modelo da geração mínima.
#[derive(Debug, Clone, Default)]
pub struct SdBinaries {
    pub vulkan: Option<PathBuf>,
    pub rocm: Option<PathBuf>,
    pub cuda: Option<PathBuf>,
    pub cpu: Option<PathBuf>,
    pub model: Option<PathBuf>,
}

impl SdBinaries {
    #[must_use]
    pub fn for_backend(&self, backend: ImageBackend) -> Option<&Path> {
        match backend {
            ImageBackend::Vulkan => self.vulkan.as_deref(),
            ImageBackend::Rocm => self.rocm.as_deref(),
            ImageBackend::Cuda => self.cuda.as_deref(),
            ImageBackend::Cpu => self.cpu.as_deref(),
        }
    }
}

/// Procura os binários no ambiente e subindo a partir dos diretórios informados.
#[must_use]
pub fn discover_from(starts: &[PathBuf], env: impl Fn(&str) -> Option<String>) -> SdBinaries {
    let file = |key: &str, relative: &str| -> Option<PathBuf> {
        if let Some(path) = env(key) {
            let path = PathBuf::from(path);
            return path.is_file().then_some(path);
        }
        starts.iter().find_map(|start| find_up(start, relative))
    };
    SdBinaries {
        vulkan: file("VOXEL_SD_VULKAN", "workers/sdcpp/build/vulkan/bin/sd-cli"),
        rocm: file("VOXEL_SD_ROCM", "workers/sdcpp/build/hip/bin/sd-cli"),
        cuda: file("VOXEL_SD_CUDA", "workers/sdcpp/build/cuda/bin/sd-cli"),
        cpu: file("VOXEL_SD_CPU", "workers/sdcpp/build/cpu/bin/sd-cli"),
        model: file(
            "VOXEL_SD_MODEL",
            "models/sd15/v1-5-pruned-emaonly.safetensors",
        ),
    }
}

#[must_use]
pub fn discover() -> SdBinaries {
    let mut starts = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        starts.push(current);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        starts.push(parent.to_owned());
    }
    discover_from(&starts, |key| std::env::var(key).ok())
}

/// Roda a geração mínima num processo próprio. O override de ISA não chega no filho.
#[must_use]
pub fn run_probe(binary: &Path, model: &Path, backend: ImageBackend, work: &Path) -> ProbeVerdict {
    if let Err(error) = std::fs::create_dir_all(work) {
        return ProbeVerdict::Failed(error.to_string());
    }
    let listed = match command_output(binary, &["--list-devices".to_owned()], work, LIST_LIMIT) {
        Ok(output) => output,
        Err(detail) => return ProbeVerdict::Failed(detail),
    };
    if !listed.status.success() {
        return ProbeVerdict::Failed(failure_detail(&listed));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );
    let Some(device) = device_for(backend, &parse_devices(&text)) else {
        return ProbeVerdict::Unavailable;
    };
    let output_path = work.join("output.png");
    let args = generation_args(model, &output_path, &device);
    let generated = match command_output(binary, &args, work, GENERATE_LIMIT) {
        Ok(output) => output,
        Err(detail) => return ProbeVerdict::Failed(detail),
    };
    if !generated.status.success() {
        return ProbeVerdict::Failed(failure_detail(&generated));
    }
    match std::fs::read(&output_path) {
        Ok(bytes) if is_png(&bytes) => ProbeVerdict::Passed,
        Ok(_) => ProbeVerdict::Failed("a saída não é um png".to_owned()),
        Err(error) => ProbeVerdict::Failed(error.to_string()),
    }
}

/// Tira do filho as variáveis que fariam o `ROCm` anunciar outra ISA.
pub fn isolate(command: &mut Command) {
    command.env_remove("HSA_OVERRIDE_GFX_VERSION");
    command.env_remove("PYTORCH_ROCM_ARCH");
}

fn find_up(start: &Path, relative: &str) -> Option<PathBuf> {
    let mut current = Some(start);
    for _ in 0..8 {
        let dir = current?;
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

fn command_output(
    binary: &Path,
    args: &[String],
    work: &Path,
    limit: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(work)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    isolate(&mut command);
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = std::thread::spawn(move || read_pipe(stdout));
    let stderr_handle = std::thread::spawn(move || read_pipe(stderr));
    let status = wait_for(&mut child, limit)?;
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_pipe(pipe: Option<impl Read>) -> Vec<u8> {
    let mut buffer = Vec::new();
    if let Some(mut pipe) = pipe {
        let _ = pipe.read_to_end(&mut buffer);
    }
    buffer
}

fn wait_for(
    child: &mut std::process::Child,
    limit: Duration,
) -> Result<std::process::ExitStatus, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("tempo esgotado".to_owned());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn failure_detail(output: &Output) -> String {
    let code = output
        .status
        .code()
        .map_or_else(|| "sinal".to_owned(), |code| code.to_string());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let tail: String = stderr
        .trim()
        .chars()
        .rev()
        .take(300)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if tail.is_empty() {
        format!("saída {code}")
    } else {
        format!("saída {code}: {tail}")
    }
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() > 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[test]
    fn o_filho_nao_herda_o_override_de_isa() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("printf %s \"$HSA_OVERRIDE_GFX_VERSION$PYTORCH_ROCM_ARCH\"")
            .env("HSA_OVERRIDE_GFX_VERSION", "11.0.0")
            .env("PYTORCH_ROCM_ARCH", "gfx1100");
        isolate(&mut command);
        let output = command.output().unwrap();
        assert_eq!(output.stdout, b"");
    }

    #[test]
    fn processo_isolado_exige_png_e_dispositivo() {
        let dir = std::env::temp_dir().join(format!("voxel-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ok = script(
            &dir,
            "ok",
            r#"#!/bin/sh
if [ "$1" = "--list-devices" ]; then
  printf 'Vulkan0\tGPU\n'
  exit 0
fi
prev=""
for arg in "$@"; do
  if [ "$prev" = "-o" ]; then
    printf '\x89PNG\r\n\x1a\n' > "$arg"
    dd if=/dev/zero bs=1100 count=1 >> "$arg" 2>/dev/null
  fi
  prev="$arg"
done
exit 0
"#,
        );
        let passed = run_probe(
            &ok,
            Path::new("modelo"),
            ImageBackend::Vulkan,
            &dir.join("ok-work"),
        );
        assert_eq!(passed, ProbeVerdict::Passed);

        let missing = script(
            &dir,
            "missing",
            "#!/bin/sh\nprintf 'CPU\\tCPU\\n'\nexit 0\n",
        );
        let unavailable = run_probe(
            &missing,
            Path::new("modelo"),
            ImageBackend::Vulkan,
            &dir.join("missing-work"),
        );
        assert_eq!(unavailable, ProbeVerdict::Unavailable);

        let broken = script(
            &dir,
            "broken",
            "#!/bin/sh\nif [ \"$1\" = \"--list-devices\" ]; then printf 'Vulkan0\\tGPU\\n'; exit 0; fi\necho quebrou >&2\nexit 134\n",
        );
        let failed = run_probe(
            &broken,
            Path::new("modelo"),
            ImageBackend::Vulkan,
            &dir.join("broken-work"),
        );
        assert!(matches!(failed, ProbeVerdict::Failed(detail) if detail.contains("quebrou")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn descobre_o_binario_subindo_diretorios_ou_pelo_ambiente() {
        let root = std::env::temp_dir().join(format!("voxel-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let binary = root.join("workers/sdcpp/build/vulkan/bin");
        std::fs::create_dir_all(&binary).unwrap();
        std::fs::write(binary.join("sd-cli"), "x").unwrap();
        let nested = root.join("apps/desktop");
        std::fs::create_dir_all(&nested).unwrap();
        let found = discover_from(&[nested], |_| None);
        assert_eq!(
            found.vulkan.as_deref(),
            Some(binary.join("sd-cli").as_path())
        );
        assert!(found.model.is_none());

        let forced = discover_from(&[], |key| {
            (key == "VOXEL_SD_MODEL").then(|| binary.join("sd-cli").display().to_string())
        });
        assert!(forced.model.is_some());
        assert!(forced.vulkan.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "gera uma imagem real com o VAE em fp16"]
    fn cada_backend_presente_gera_um_png() {
        let bins = discover();
        let model = bins.model.clone().expect("modelo sd15");
        let work = std::env::temp_dir().join("voxel-probe-real");
        let mut ran = 0;
        for backend in [ImageBackend::Vulkan, ImageBackend::Rocm, ImageBackend::Cpu] {
            let Some(binary) = bins.for_backend(backend) else {
                continue;
            };
            ran += 1;
            let verdict = run_probe(binary, &model, backend, &work.join(backend.id()));
            assert_eq!(verdict, ProbeVerdict::Passed, "{backend:?}");
        }
        assert!(ran >= 1, "nenhum binário encontrado");
    }
}
