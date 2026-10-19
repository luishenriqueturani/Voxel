use std::path::Path;

use crate::compat::{Combination, CompatibilityMemory};
use crate::fallback::{ImageBackend, facts_from, image_fallback};
use crate::profile::{Profile, profile_for};
use crate::report::MachineReport;

/// Modelo da geração mínima do setup.
pub const SETUP_MODEL: &str = "sd15";

/// Configuração que o teste exercita: VAE em fp16, 512 px, 4 passos, Euler A.
pub const SETUP_CONFIGURATION: &str = "vae-f16;512x512;steps=4;euler_a";

/// Resultado de um processo de teste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// O binário ou o dispositivo não está nesta máquina.
    Unavailable,
    Passed,
    Failed(String),
}

/// O que aconteceu com um backend na passagem do setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptState {
    Unavailable,
    Passed,
    Failed(String),
    /// Já estava na memória de compatibilidade.
    Skipped,
    /// Um backend anterior passou.
    NotNeeded,
}

/// Uma linha do progresso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub backend: ImageBackend,
    pub state: AttemptState,
}

/// Perfil escolhido e o que cada backend fez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSetup {
    pub profile: Profile,
    pub attempts: Vec<Attempt>,
    pub chosen: Option<ImageBackend>,
}

/// Testa os backends de imagem na ordem de fallback e para no primeiro que passa.
///
/// Combinação que falha entra na memória e não é tentada de novo.
pub fn image_setup(
    machine: &MachineReport,
    memory: &mut CompatibilityMemory,
    model: &str,
    configuration: &str,
    mut probe: impl FnMut(ImageBackend) -> ProbeVerdict,
) -> ImageSetup {
    let order = image_fallback(&facts_from(machine, false));
    let mut attempts = Vec::new();
    let mut chosen = None;
    for backend in order {
        if chosen.is_some() {
            attempts.push(Attempt {
                backend,
                state: AttemptState::NotNeeded,
            });
            continue;
        }
        let combination = Combination {
            model: model.to_owned(),
            backend: backend.id().to_owned(),
            configuration: configuration.to_owned(),
        };
        if !memory.allows(&combination) {
            attempts.push(Attempt {
                backend,
                state: AttemptState::Skipped,
            });
            continue;
        }
        let verdict = probe(backend);
        if let ProbeVerdict::Failed(detail) = &verdict {
            memory.record_failure(combination, detail.clone());
        }
        if matches!(verdict, ProbeVerdict::Passed) {
            chosen = Some(backend);
        }
        attempts.push(Attempt {
            backend,
            state: match verdict {
                ProbeVerdict::Unavailable => AttemptState::Unavailable,
                ProbeVerdict::Passed => AttemptState::Passed,
                ProbeVerdict::Failed(detail) => AttemptState::Failed(detail),
            },
        });
    }
    ImageSetup {
        profile: profile_for(&machine.gpus),
        attempts,
        chosen,
    }
}

/// Argumentos da geração mínima, com o VAE forçado em fp16.
#[must_use]
pub fn generation_args(model: &Path, output: &Path, device: &str) -> Vec<String> {
    vec![
        "-m".to_owned(),
        model.display().to_string(),
        "-p".to_owned(),
        "a red square on a white background".to_owned(),
        "-o".to_owned(),
        output.display().to_string(),
        "--steps".to_owned(),
        "4".to_owned(),
        "-W".to_owned(),
        "512".to_owned(),
        "-H".to_owned(),
        "512".to_owned(),
        "--seed".to_owned(),
        "1".to_owned(),
        "--sampling-method".to_owned(),
        "euler_a".to_owned(),
        "--backend".to_owned(),
        device.to_owned(),
        "--tensor-type-rules".to_owned(),
        r"^vae\.=f16".to_owned(),
        "--auto-fit".to_owned(),
        "on".to_owned(),
    ]
}

/// Nomes de dispositivo do `--list-devices`, uma linha `nome<TAB>descrição`.
#[must_use]
pub fn parse_devices(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let (name, _) = line.split_once('\t')?;
            let name = name.trim();
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

/// Dispositivo da lista cujo nome corresponde ao backend.
#[must_use]
pub fn device_for(backend: ImageBackend, devices: &[String]) -> Option<String> {
    let prefix = match backend {
        ImageBackend::Vulkan => "vulkan",
        ImageBackend::Rocm => "rocm",
        ImageBackend::Cuda => "cuda",
        ImageBackend::Cpu => "cpu",
    };
    devices
        .iter()
        .find(|name| name.to_lowercase().starts_with(prefix))
        .cloned()
}

/// Uma linha curta para a tela de progresso.
#[must_use]
pub fn hardware_line(machine: &MachineReport) -> String {
    let gpu = machine
        .gpus
        .iter()
        .max_by_key(|gpu| gpu.vram_bytes)
        .map_or("CPU", |gpu| gpu.name.as_str());
    let gib = machine.memory.total_bytes / (1024 * 1024 * 1024);
    format!("{gpu} · {gib} GiB")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Cpu, DiskSpace, Gpu, Memory, OperatingSystem, Vendor};

    fn machine(gpus: Vec<Gpu>, rocm: bool) -> MachineReport {
        MachineReport {
            os: OperatingSystem {
                name: String::new(),
                id: String::new(),
                version: String::new(),
                kernel: String::new(),
                drivers: Vec::new(),
            },
            cpu: Cpu {
                model: String::new(),
                physical_cores: 1,
                logical_cores: 1,
                avx2: true,
                avx512: false,
            },
            memory: Memory {
                total_bytes: 32 * 1024 * 1024 * 1024,
                available_bytes: 0,
            },
            disk: DiskSpace {
                path: std::path::PathBuf::new(),
                free_bytes: 0,
                total_bytes: 0,
            },
            gpus,
            rocm,
            rocm_version: None,
            cuda: false,
        }
    }

    fn gpu(name: &str, vendor: Vendor, isa: Option<&str>) -> Gpu {
        Gpu {
            vendor,
            name: name.to_owned(),
            vram_bytes: 8 * 1024 * 1024 * 1024,
            device_id: 1,
            driver_name: String::new(),
            driver_version: String::new(),
            isa: isa.map(str::to_owned),
        }
    }

    #[test]
    fn para_no_primeiro_que_passa_e_lembra_quem_quebrou() {
        let report = machine(
            vec![gpu("AMD Radeon RX 7600", Vendor::Amd, Some("gfx1102"))],
            true,
        );
        let mut memory = CompatibilityMemory::default();
        let mut calls = Vec::new();
        let outcome = image_setup(
            &report,
            &mut memory,
            SETUP_MODEL,
            SETUP_CONFIGURATION,
            |backend| {
                calls.push(backend);
                match backend {
                    ImageBackend::Vulkan => ProbeVerdict::Failed("SIGABRT".to_owned()),
                    ImageBackend::Rocm => ProbeVerdict::Passed,
                    ImageBackend::Cpu | ImageBackend::Cuda => ProbeVerdict::Unavailable,
                }
            },
        );
        assert_eq!(calls, vec![ImageBackend::Vulkan, ImageBackend::Rocm]);
        assert_eq!(outcome.chosen, Some(ImageBackend::Rocm));
        assert_eq!(outcome.profile, Profile::Intermediario);
        assert!(matches!(outcome.attempts[2].state, AttemptState::NotNeeded));
        assert!(!memory.allows(&Combination {
            model: SETUP_MODEL.to_owned(),
            backend: "vulkan".to_owned(),
            configuration: SETUP_CONFIGURATION.to_owned(),
        }));

        calls.clear();
        let again = image_setup(
            &report,
            &mut memory,
            SETUP_MODEL,
            SETUP_CONFIGURATION,
            |backend| {
                calls.push(backend);
                ProbeVerdict::Passed
            },
        );
        assert_eq!(calls, vec![ImageBackend::Rocm]);
        assert!(matches!(again.attempts[0].state, AttemptState::Skipped));
        assert_eq!(again.chosen, Some(ImageBackend::Rocm));
    }

    #[test]
    fn so_cpu_nao_inventa_gpu() {
        let outcome = image_setup(
            &machine(Vec::new(), false),
            &mut CompatibilityMemory::default(),
            SETUP_MODEL,
            SETUP_CONFIGURATION,
            |_| ProbeVerdict::Unavailable,
        );
        assert_eq!(outcome.profile, Profile::Minimo);
        assert_eq!(outcome.attempts.len(), 1);
        assert_eq!(outcome.chosen, None);
    }

    #[test]
    fn a_geracao_minima_forca_o_vae_em_fp16() {
        let args = generation_args(Path::new("modelo"), Path::new("saida.png"), "Vulkan0");
        assert!(args.contains(&"--tensor-type-rules".to_owned()));
        assert!(args.contains(&r"^vae\.=f16".to_owned()));
        assert!(args.contains(&"--backend".to_owned()));
        assert!(args.contains(&"Vulkan0".to_owned()));
        assert!(args.contains(&"4".to_owned()));
    }

    #[test]
    fn le_o_dispositivo_certo_e_ignora_o_log() {
        let text = "ggml_vulkan: Found 1\nVulkan0\tAMD Radeon RX 7600\nCPU\tRyzen\n";
        let devices = parse_devices(text);
        assert_eq!(
            device_for(ImageBackend::Vulkan, &devices).as_deref(),
            Some("Vulkan0")
        );
        assert_eq!(device_for(ImageBackend::Rocm, &devices).as_deref(), None);
        assert_eq!(
            device_for(ImageBackend::Cpu, &devices).as_deref(),
            Some("CPU")
        );
        let rocm = vec!["ROCm0".to_owned()];
        assert_eq!(device_for(ImageBackend::Cuda, &rocm), None);
    }

    #[test]
    fn a_linha_de_hardware_usa_a_maior_gpu() {
        let report = machine(vec![gpu("AMD Radeon RX 7600", Vendor::Amd, None)], false);
        assert_eq!(hardware_line(&report), "AMD Radeon RX 7600 · 32 GiB");
        assert_eq!(hardware_line(&machine(Vec::new(), false)), "CPU · 32 GiB");
    }
}
