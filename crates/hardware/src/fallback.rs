use crate::report::{MachineReport, Vendor};

/// Backend de imagem do stable-diffusion.cpp, na ordem em que o setup os testa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageBackend {
    Vulkan,
    Rocm,
    Cuda,
    Cpu,
}

impl ImageBackend {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
            Self::Rocm => "rocm",
            Self::Cuda => "cuda",
            Self::Cpu => "cpu",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "vulkan" => Some(Self::Vulkan),
            "rocm" => Some(Self::Rocm),
            "cuda" => Some(Self::Cuda),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }
}

/// Backend Python da coluna 3D do §6.4. O setup inicial não o executa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PythonBackend {
    Cuda,
    RocmNative,
    RocmSafeOverride,
    Xpu,
    Cpu,
}

impl PythonBackend {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Cuda => "cuda",
            Self::RocmNative => "rocm_native",
            Self::RocmSafeOverride => "rocm_safe_override",
            Self::Xpu => "xpu",
            Self::Cpu => "cpu",
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "cuda" => Some(Self::Cuda),
            "rocm_native" => Some(Self::RocmNative),
            "rocm_safe_override" => Some(Self::RocmSafeOverride),
            "xpu" => Some(Self::Xpu),
            "cpu" => Some(Self::Cpu),
            _ => None,
        }
    }
}

/// O que a ordem de fallback precisa saber sobre a máquina.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendFacts {
    pub vendor: Option<Vendor>,
    pub vulkan: bool,
    pub rocm: bool,
    pub native_isa: bool,
    pub hsa_override_set: bool,
    pub cuda: bool,
    pub xpu: bool,
}

/// Fatos a partir da detecção. `XPU` ainda não é lido do sistema.
#[must_use]
pub fn facts_from(machine: &MachineReport, hsa_override_set: bool) -> BackendFacts {
    let primary = machine.gpus.iter().max_by_key(|gpu| gpu.vram_bytes);
    BackendFacts {
        vendor: primary.map(|gpu| gpu.vendor),
        vulkan: !machine.gpus.is_empty(),
        rocm: machine.rocm,
        native_isa: machine.gpus.iter().any(|gpu| gpu.isa.is_some()),
        hsa_override_set,
        cuda: machine.cuda,
        xpu: false,
    }
}

/// Ordem 2D do §6.4. Vulkan primeiro; `ROCm` só com ISA nativa; CPU por último.
#[must_use]
pub fn image_fallback(facts: &BackendFacts) -> Vec<ImageBackend> {
    if !facts.vulkan {
        return vec![ImageBackend::Cpu];
    }
    let mut order = vec![ImageBackend::Vulkan];
    match facts.vendor {
        Some(Vendor::Nvidia) if facts.cuda => order.push(ImageBackend::Cuda),
        Some(Vendor::Amd) if facts.rocm && facts.native_isa => order.push(ImageBackend::Rocm),
        _ => {}
    }
    order.push(ImageBackend::Cpu);
    order
}

/// Ordem 3D do §6.4. O override seguro só entra quando o `ROCm` nativo não é o caminho.
#[must_use]
pub fn python_fallback(facts: &BackendFacts) -> Vec<PythonBackend> {
    let mut order = Vec::new();
    match facts.vendor {
        Some(Vendor::Nvidia) if facts.cuda => order.push(PythonBackend::Cuda),
        Some(Vendor::Amd) if facts.rocm && facts.native_isa && !facts.hsa_override_set => {
            order.push(PythonBackend::RocmNative);
            order.push(PythonBackend::RocmSafeOverride);
        }
        Some(Vendor::Amd) if facts.rocm => order.push(PythonBackend::RocmSafeOverride),
        Some(Vendor::Intel) if facts.xpu => order.push(PythonBackend::Xpu),
        _ => {}
    }
    order.push(PythonBackend::Cpu);
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Cpu, DiskSpace, Gpu, Memory, OperatingSystem};

    #[test]
    fn o_id_do_backend_de_imagem_e_estavel() {
        assert_eq!(ImageBackend::from_id("rocm"), Some(ImageBackend::Rocm));
        assert_eq!(ImageBackend::from_id("metal"), None);
    }

    fn facts(vendor: Option<Vendor>) -> BackendFacts {
        BackendFacts {
            vendor,
            vulkan: vendor.is_some(),
            rocm: false,
            native_isa: false,
            hsa_override_set: false,
            cuda: false,
            xpu: false,
        }
    }

    fn machine(vendor: Vendor, isa: Option<&str>, rocm: bool, cuda: bool) -> MachineReport {
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
                total_bytes: 32 << 30,
                available_bytes: 0,
            },
            disk: DiskSpace {
                path: std::path::PathBuf::new(),
                free_bytes: 0,
                total_bytes: 0,
            },
            gpus: vec![Gpu {
                vendor,
                name: "GPU".to_owned(),
                vram_bytes: 8 << 30,
                device_id: 1,
                driver_name: String::new(),
                driver_version: String::new(),
                isa: isa.map(str::to_owned),
            }],
            rocm,
            rocm_version: None,
            cuda,
        }
    }

    #[test]
    fn imagem_segue_a_tabela() {
        assert_eq!(image_fallback(&facts(None)), vec![ImageBackend::Cpu]);

        let mut amd = facts(Some(Vendor::Amd));
        amd.rocm = true;
        amd.native_isa = true;
        assert_eq!(
            image_fallback(&amd),
            vec![ImageBackend::Vulkan, ImageBackend::Rocm, ImageBackend::Cpu]
        );

        amd.native_isa = false;
        assert_eq!(
            image_fallback(&amd),
            vec![ImageBackend::Vulkan, ImageBackend::Cpu]
        );

        let mut nvidia = facts(Some(Vendor::Nvidia));
        nvidia.cuda = true;
        assert_eq!(
            image_fallback(&nvidia),
            vec![ImageBackend::Vulkan, ImageBackend::Cuda, ImageBackend::Cpu]
        );

        assert_eq!(
            image_fallback(&facts(Some(Vendor::Intel))),
            vec![ImageBackend::Vulkan, ImageBackend::Cpu]
        );
    }

    #[test]
    fn python_segue_a_tabela() {
        assert_eq!(python_fallback(&facts(None)), vec![PythonBackend::Cpu]);

        let mut amd = facts(Some(Vendor::Amd));
        amd.rocm = true;
        amd.native_isa = true;
        assert_eq!(
            python_fallback(&amd),
            vec![
                PythonBackend::RocmNative,
                PythonBackend::RocmSafeOverride,
                PythonBackend::Cpu
            ]
        );

        amd.hsa_override_set = true;
        assert_eq!(
            python_fallback(&amd),
            vec![PythonBackend::RocmSafeOverride, PythonBackend::Cpu]
        );

        let mut nvidia = facts(Some(Vendor::Nvidia));
        nvidia.cuda = true;
        assert_eq!(
            python_fallback(&nvidia),
            vec![PythonBackend::Cuda, PythonBackend::Cpu]
        );

        let mut intel = facts(Some(Vendor::Intel));
        intel.xpu = true;
        assert_eq!(
            python_fallback(&intel),
            vec![PythonBackend::Xpu, PythonBackend::Cpu]
        );
    }

    #[test]
    fn os_fatos_saem_da_gpu_de_maior_vram() {
        let mut report = machine(Vendor::Amd, Some("gfx1102"), true, false);
        report.gpus.push(Gpu {
            vendor: Vendor::Nvidia,
            name: "pequena".to_owned(),
            vram_bytes: 1 << 30,
            device_id: 2,
            driver_name: String::new(),
            driver_version: String::new(),
            isa: None,
        });
        let facts = facts_from(&report, true);
        assert_eq!(facts.vendor, Some(Vendor::Amd));
        assert!(facts.vulkan && facts.rocm && facts.native_isa && facts.hsa_override_set);
        assert!(!facts.cuda && !facts.xpu);
        assert_eq!(ImageBackend::Vulkan.id(), "vulkan");
        assert_eq!(PythonBackend::RocmNative.id(), "rocm_native");
        assert_eq!(
            PythonBackend::from_id("rocm_safe_override"),
            Some(PythonBackend::RocmSafeOverride)
        );
        assert_eq!(PythonBackend::from_id("outro"), None);
    }
}
