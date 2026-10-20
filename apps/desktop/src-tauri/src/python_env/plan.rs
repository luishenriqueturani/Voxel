//! O que instalar e quais variáveis o worker Python herda.
//!
//! O `uv` baixa o `CPython` para a pasta de dados do app. O `PyTorch` vem do índice da
//! variante (CUDA, `ROCm`, XPU ou CPU), sempre em wheel. A versão fica travada em
//! `workers/python/constraints/torch.txt`.

use std::path::{Path, PathBuf};

use voxel_hardware::PythonBackend;

pub const PYTHON_VERSION: &str = "3.12";
pub const TORCH_SPEC: &str = include_str!("../../../../../workers/python/constraints/torch.txt");
const RUNTIME_SPEC: &str = include_str!("../../../../../workers/python/constraints/runtime.txt");
const PYPI: &str = "https://pypi.org/simple";

const CPU_INDEX: &str = "https://download.pytorch.org/whl/cpu";
const CUDA_INDEX: &str = "https://download.pytorch.org/whl/cu130";
const XPU_INDEX: &str = "https://download.pytorch.org/whl/xpu";

/// Variáveis que o fish desta máquina exporta e que o teste da fase 1 rejeitou.
const INHERITED: [&str; 6] = [
    "HSA_OVERRIDE_GFX_VERSION",
    "PYTORCH_ROCM_ARCH",
    "MIOPEN_DISABLE_CACHE",
    "MIOPEN_DEBUG_DISABLE_ASM_KERNELS",
    "MIOPEN_DEBUG_CONV_IMPLICIT_GEMM",
    "MIOPEN_FIND_ENFORCE",
];

/// Pastas do ambiente, todas sob os diretórios XDG do app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub root: PathBuf,
    pub interpreters: PathBuf,
    pub venv: PathBuf,
    pub marker: PathBuf,
    pub cache: PathBuf,
}

/// O que o processo do worker recebe e o que ele deixa de herdar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEnv {
    pub set: Vec<(String, String)>,
    pub remove: Vec<String>,
}

/// Motivo pelo qual o usuário demonstrou que vai usar 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Panel,
    FlowItem,
    Template,
    ModelDownload,
    SendTo3d,
}

impl Intent {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "panel" => Some(Self::Panel),
            "flow-item" => Some(Self::FlowItem),
            "template" => Some(Self::Template),
            "model-download" => Some(Self::ModelDownload),
            "send-to-3d" => Some(Self::SendTo3d),
            _ => None,
        }
    }
}

impl Layout {
    #[must_use]
    pub fn new(data: &Path, cache: &Path) -> Self {
        let root = data.join("python");
        Self {
            interpreters: root.join("interpreters"),
            venv: root.join("venv"),
            marker: root.join("environment.json"),
            cache: cache.join("uv"),
            root,
        }
    }

    #[must_use]
    pub fn python(&self) -> PathBuf {
        self.venv.join("bin").join("python")
    }
}

#[must_use]
pub fn torch_requirement() -> &'static str {
    TORCH_SPEC.trim()
}

/// Índice do wheel. `ROCm` usa o major.minor instalado na máquina.
#[must_use]
pub fn index_url(backend: PythonBackend, rocm_version: Option<&str>) -> Option<String> {
    match backend {
        PythonBackend::Cpu => Some(CPU_INDEX.to_owned()),
        PythonBackend::Cuda => Some(CUDA_INDEX.to_owned()),
        PythonBackend::Xpu => Some(XPU_INDEX.to_owned()),
        PythonBackend::RocmNative | PythonBackend::RocmSafeOverride => {
            let (major, minor) = major_minor(rocm_version?)?;
            Some(format!(
                "https://download.pytorch.org/whl/rocm{major}.{minor}"
            ))
        }
    }
}

#[must_use]
pub fn venv_args(venv: &Path) -> Vec<String> {
    vec![
        "venv".to_owned(),
        "--no-project".to_owned(),
        "--managed-python".to_owned(),
        "--python".to_owned(),
        PYTHON_VERSION.to_owned(),
        "--clear".to_owned(),
        venv.display().to_string(),
    ]
}

#[must_use]
pub fn pip_args(python: &Path, index: &str) -> Vec<String> {
    let mut args = vec![
        "pip".to_owned(),
        "install".to_owned(),
        "--python".to_owned(),
        python.display().to_string(),
        "--only-binary".to_owned(),
        ":all:".to_owned(),
        "--reinstall".to_owned(),
        "--index-url".to_owned(),
        index.to_owned(),
        "--extra-index-url".to_owned(),
        PYPI.to_owned(),
        torch_requirement().to_owned(),
    ];
    args.extend(runtime_requirements());
    args
}

fn runtime_requirements() -> impl Iterator<Item = String> {
    RUNTIME_SPEC
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
}

/// Ambiente do processo. `ROCm` nativo tira o override; o fallback seguro desliga o assembly.
#[must_use]
pub fn process_env(backend: PythonBackend, isa: Option<&str>) -> ProcessEnv {
    let mut set = Vec::new();
    if backend == PythonBackend::RocmSafeOverride {
        set.push(("MIOPEN_DEBUG_GCN_ASM_KERNELS".to_owned(), "0".to_owned()));
        if isa == Some("gfx1102") {
            set.push(("HSA_OVERRIDE_GFX_VERSION".to_owned(), "11.0.0".to_owned()));
        }
    }
    let remove = INHERITED
        .into_iter()
        .filter(|name| set.iter().all(|(key, _)| key != name))
        .map(str::to_owned)
        .collect();
    ProcessEnv { set, remove }
}

#[must_use]
pub fn probe_device(backend: PythonBackend) -> &'static str {
    match backend {
        PythonBackend::Cpu => "cpu",
        PythonBackend::Xpu => "xpu",
        PythonBackend::Cuda | PythonBackend::RocmNative | PythonBackend::RocmSafeOverride => "cuda",
    }
}

fn major_minor(version: &str) -> Option<(u32, u32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_versao_do_torch_vem_do_arquivo_travado() {
        assert_eq!(torch_requirement(), "torch==2.14.0");
    }

    #[test]
    fn o_indice_acompanha_o_hardware() {
        assert_eq!(
            index_url(PythonBackend::Cpu, None).as_deref(),
            Some(CPU_INDEX)
        );
        assert_eq!(
            index_url(PythonBackend::Cuda, None).as_deref(),
            Some(CUDA_INDEX)
        );
        assert_eq!(
            index_url(PythonBackend::RocmNative, Some("7.2.4")).as_deref(),
            Some("https://download.pytorch.org/whl/rocm7.2")
        );
        assert_eq!(
            index_url(PythonBackend::RocmSafeOverride, Some("7.14.0")).as_deref(),
            Some("https://download.pytorch.org/whl/rocm7.14")
        );
        assert_eq!(index_url(PythonBackend::RocmNative, None), None);
    }

    #[test]
    fn o_python_fica_na_pasta_de_dados() {
        let layout = Layout::new(Path::new("/dados/voxel"), Path::new("/cache/voxel"));
        assert_eq!(layout.venv, PathBuf::from("/dados/voxel/python/venv"));
        assert_eq!(
            layout.interpreters,
            PathBuf::from("/dados/voxel/python/interpreters")
        );
        assert_eq!(layout.cache, PathBuf::from("/cache/voxel/uv"));
        let args = venv_args(&layout.venv);
        assert!(args.contains(&"--managed-python".to_owned()));
        assert!(args.iter().any(|arg| arg == "/dados/voxel/python/venv"));
        let pip = pip_args(&layout.python(), CPU_INDEX);
        assert!(pip.contains(&"--only-binary".to_owned()));
        assert!(pip.contains(&"--extra-index-url".to_owned()));
        assert!(pip.contains(&"torch==2.14.0".to_owned()));
        assert!(pip.contains(&"fastapi==0.156.1".to_owned()));
        assert!(pip.contains(&"transformers==4.57.6".to_owned()));
        assert!(pip.contains(&"omegaconf==2.4.0".to_owned()));
    }

    #[test]
    fn o_rocm_nativo_nao_herda_o_override_e_o_seguro_desliga_o_assembly() {
        let native = process_env(PythonBackend::RocmNative, Some("gfx1102"));
        assert!(native.set.is_empty());
        assert!(
            native
                .remove
                .iter()
                .any(|name| name == "HSA_OVERRIDE_GFX_VERSION")
        );
        assert!(native.remove.iter().any(|name| name == "PYTORCH_ROCM_ARCH"));
        assert!(
            native
                .remove
                .iter()
                .any(|name| name == "MIOPEN_DISABLE_CACHE")
        );

        let safe = process_env(PythonBackend::RocmSafeOverride, Some("gfx1102"));
        assert!(
            safe.set
                .iter()
                .any(|(key, value)| { key == "MIOPEN_DEBUG_GCN_ASM_KERNELS" && value == "0" })
        );
        assert!(
            safe.set
                .iter()
                .any(|(key, value)| { key == "HSA_OVERRIDE_GFX_VERSION" && value == "11.0.0" })
        );
        assert!(
            !safe
                .remove
                .iter()
                .any(|name| name == "HSA_OVERRIDE_GFX_VERSION")
        );
        assert!(
            safe.remove
                .iter()
                .any(|name| name == "MIOPEN_DISABLE_CACHE")
        );

        let other = process_env(PythonBackend::RocmSafeOverride, Some("gfx1030"));
        assert!(
            !other
                .set
                .iter()
                .any(|(key, _)| key == "HSA_OVERRIDE_GFX_VERSION")
        );
        assert_eq!(probe_device(PythonBackend::Cpu), "cpu");
        assert_eq!(probe_device(PythonBackend::RocmNative), "cuda");
    }

    #[test]
    fn os_cinco_gatilhos_de_intencao_sao_reconhecidos() {
        assert_eq!(Intent::parse("panel"), Some(Intent::Panel));
        assert_eq!(Intent::parse("flow-item"), Some(Intent::FlowItem));
        assert_eq!(Intent::parse("template"), Some(Intent::Template));
        assert_eq!(Intent::parse("model-download"), Some(Intent::ModelDownload));
        assert_eq!(Intent::parse("send-to-3d"), Some(Intent::SendTo3d));
        assert_eq!(Intent::parse("2d"), None);
    }
}
