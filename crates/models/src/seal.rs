use crate::catalog::{BackendId, BackendNeed, ListedCaveat, Model};

/// O que a máquina oferece para escolher o selo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineFacts {
    pub vram_bytes: u64,
    pub ram_bytes: u64,
    pub vendor: Option<GpuVendor>,
    pub vulkan: bool,
    pub rocm: bool,
    pub cuda: bool,
}

/// Fabricante da GPU usada na checagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
}

/// Selo do §9.4 para um modelo nesta máquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    Caveat(Reason),
    Incompatible(Reason),
}

/// Motivo do selo. A interface traduz o código.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    RequiresCuda,
    NeedsVram {
        required_bytes: u64,
        available_bytes: u64,
    },
    NeedsRam {
        required_bytes: u64,
        available_bytes: u64,
    },
    OffloadRam,
    CpuSlow,
    ExperimentalAttention,
    UnprovenVram {
        proven_bytes: u64,
        available_bytes: u64,
    },
}

impl MachineFacts {
    fn supports(self, id: BackendId) -> bool {
        match id {
            BackendId::Cpu => true,
            BackendId::Vulkan => self.vulkan && self.vram_bytes > 0,
            BackendId::Rocm => self.rocm && self.vendor == Some(GpuVendor::Amd),
            BackendId::Cuda => self.cuda && self.vendor == Some(GpuVendor::Nvidia),
        }
    }
}

/// Escolhe o melhor selo entre os backends que esta máquina consegue acionar.
#[must_use]
pub fn compatibility(model: &Model, facts: &MachineFacts) -> Compatibility {
    if model.requires_cuda && facts.vendor != Some(GpuVendor::Nvidia) {
        return Compatibility::Incompatible(Reason::RequiresCuda);
    }
    let mut outcomes: Vec<Compatibility> = model
        .backends
        .iter()
        .filter(|backend| facts.supports(backend.id))
        .map(|backend| assess(backend, facts))
        .collect();
    outcomes.sort_by_key(rank);
    outcomes
        .into_iter()
        .next()
        .unwrap_or_else(|| fallback(model, facts))
}

fn assess(backend: &BackendNeed, facts: &MachineFacts) -> Compatibility {
    if backend.id == BackendId::Cpu {
        return assess_cpu(backend, facts);
    }
    if let Some(proven) = backend.proven_vram_bytes
        && facts.vram_bytes < proven
    {
        return Compatibility::Caveat(Reason::UnprovenVram {
            proven_bytes: proven,
            available_bytes: facts.vram_bytes,
        });
    }
    if let Some(required) = backend.min_vram_bytes
        && facts.vram_bytes < required
    {
        if backend
            .offload_at_or_below_bytes
            .is_some_and(|limit| facts.vram_bytes <= limit)
        {
            return Compatibility::Caveat(Reason::OffloadRam);
        }
        return Compatibility::Incompatible(Reason::NeedsVram {
            required_bytes: required,
            available_bytes: facts.vram_bytes,
        });
    }
    if backend
        .offload_at_or_below_bytes
        .is_some_and(|limit| facts.vram_bytes <= limit)
    {
        return Compatibility::Caveat(Reason::OffloadRam);
    }
    match backend.caveat {
        Some(ListedCaveat::ExperimentalAttention) => {
            Compatibility::Caveat(Reason::ExperimentalAttention)
        }
        None => Compatibility::Compatible,
    }
}

fn assess_cpu(backend: &BackendNeed, facts: &MachineFacts) -> Compatibility {
    let required = backend.min_ram_bytes.unwrap_or(0);
    if facts.ram_bytes < required {
        return Compatibility::Incompatible(Reason::NeedsRam {
            required_bytes: required,
            available_bytes: facts.ram_bytes,
        });
    }
    if backend.primary {
        Compatibility::Compatible
    } else {
        Compatibility::Caveat(Reason::CpuSlow)
    }
}

fn fallback(model: &Model, facts: &MachineFacts) -> Compatibility {
    if model
        .backends
        .iter()
        .any(|backend| backend.id == BackendId::Cuda)
        && facts.vendor != Some(GpuVendor::Nvidia)
    {
        return Compatibility::Incompatible(Reason::RequiresCuda);
    }
    let required = model
        .backends
        .iter()
        .filter_map(|backend| backend.min_vram_bytes)
        .min()
        .unwrap_or(0);
    Compatibility::Incompatible(Reason::NeedsVram {
        required_bytes: required,
        available_bytes: facts.vram_bytes,
    })
}

fn rank(outcome: &Compatibility) -> u8 {
    match outcome {
        Compatibility::Compatible => 0,
        Compatibility::Caveat(Reason::OffloadRam | Reason::ExperimentalAttention) => 1,
        Compatibility::Caveat(Reason::UnprovenVram { .. }) => 2,
        Compatibility::Caveat(_) => 3,
        Compatibility::Incompatible(_) => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::{Compatibility, GpuVendor, MachineFacts, Reason, compatibility};
    use crate::catalog::{BackendId, BackendNeed, Catalog, Kind, Model, Source};

    const RX_7600: u64 = 8_573_157_376;
    const RAM: u64 = 32 * 1024 * 1024 * 1024;

    fn machine(vram: u64, vendor: Option<GpuVendor>, rocm: bool, cuda: bool) -> MachineFacts {
        MachineFacts {
            vram_bytes: vram,
            ram_bytes: RAM,
            vendor,
            vulkan: vram > 0,
            rocm,
            cuda,
        }
    }

    fn rx7600() -> MachineFacts {
        machine(RX_7600, Some(GpuVendor::Amd), true, false)
    }

    #[test]
    fn o_sd15_no_rocm_da_rx_7600_e_compativel() {
        let model = Catalog::bundled().unwrap().get("sd15").unwrap().clone();
        assert_eq!(compatibility(&model, &rx7600()), Compatibility::Compatible);
    }

    #[test]
    fn o_vulkan_sozinho_na_rx_7600_deixa_o_vae_na_ram() {
        let model = Catalog::bundled().unwrap().get("sd15").unwrap().clone();
        let facts = machine(RX_7600, Some(GpuVendor::Amd), false, false);
        assert_eq!(
            compatibility(&model, &facts),
            Compatibility::Caveat(Reason::OffloadRam)
        );
    }

    #[test]
    fn sem_gpu_o_sd15_fica_na_cpu_e_a_u2net_continua_compativel() {
        let catalog = Catalog::bundled().unwrap();
        let facts = machine(0, None, false, false);
        assert_eq!(
            compatibility(catalog.get("sd15").unwrap(), &facts),
            Compatibility::Caveat(Reason::CpuSlow)
        );
        assert_eq!(
            compatibility(catalog.get("u2net").unwrap(), &facts),
            Compatibility::Compatible
        );
    }

    #[test]
    fn o_stable_fast_3d_na_rx_7600_pede_atencao_experimental() {
        let model = Catalog::bundled().unwrap().get("sf3d").unwrap().clone();
        assert_eq!(
            compatibility(&model, &rx7600()),
            Compatibility::Caveat(Reason::ExperimentalAttention)
        );
        let small = machine(4_000_000_000, Some(GpuVendor::Amd), true, false);
        assert_eq!(
            compatibility(&model, &small),
            Compatibility::Caveat(Reason::CpuSlow)
        );
    }

    #[test]
    fn o_triposr_abaixo_da_vram_medida_fica_com_ressalva() {
        let model = Catalog::bundled().unwrap().get("triposr").unwrap().clone();
        assert_eq!(compatibility(&model, &rx7600()), Compatibility::Compatible);
        let smaller = machine(6 * 1024 * 1024 * 1024, Some(GpuVendor::Amd), true, false);
        assert_eq!(
            compatibility(&model, &smaller),
            Compatibility::Caveat(Reason::UnprovenVram {
                proven_bytes: RX_7600,
                available_bytes: 6 * 1024 * 1024 * 1024,
            })
        );
    }

    #[test]
    fn modelo_que_exige_cuda_nao_roda_em_amd() {
        let model = Model {
            id: "cuda-only".to_owned(),
            name: "CUDA".to_owned(),
            kind: Kind::Mesh,
            filename: "x.bin".to_owned(),
            bytes: 1,
            sha256: "ab".repeat(32),
            license: "teste".to_owned(),
            gated: false,
            requires_cuda: true,
            source: Source::Url {
                url: "http://127.0.0.1/x".to_owned(),
            },
            backends: vec![BackendNeed {
                id: BackendId::Cuda,
                min_vram_bytes: Some(16_000_000_000),
                min_ram_bytes: None,
                offload_at_or_below_bytes: None,
                proven_vram_bytes: None,
                caveat: None,
                primary: false,
            }],
        };
        let amd = machine(24 * 1024 * 1024 * 1024, Some(GpuVendor::Amd), true, false);
        assert_eq!(
            compatibility(&model, &amd),
            Compatibility::Incompatible(Reason::RequiresCuda)
        );
        let nvidia = machine(8 * 1024 * 1024 * 1024, Some(GpuVendor::Nvidia), false, true);
        assert_eq!(
            compatibility(&model, &nvidia),
            Compatibility::Incompatible(Reason::NeedsVram {
                required_bytes: 16_000_000_000,
                available_bytes: 8 * 1024 * 1024 * 1024,
            })
        );
    }
}
