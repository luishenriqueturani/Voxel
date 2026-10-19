//! Retrato da máquina. Os campos vazios significam que a leitura não existia aqui.

use serde::{Deserialize, Serialize};

/// Resultado de [`crate::detect`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineReport {
    pub os: OperatingSystem,
    pub cpu: Cpu,
    pub memory: Memory,
    pub disk: DiskSpace,
    pub gpus: Vec<Gpu>,
    /// O `rocminfo` listou ao menos uma GPU, com o override de ISA desligado.
    pub rocm: bool,
    pub rocm_version: Option<String>,
    /// O NVML enxergou ao menos um dispositivo.
    pub cuda: bool,
}

/// Distribuição, kernel e drivers gráficos encontrados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatingSystem {
    pub name: String,
    pub id: String,
    pub version: String,
    pub kernel: String,
    pub drivers: Vec<Driver>,
}

/// Driver gráfico, como o Mesa `radv` ou o módulo da NVIDIA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Driver {
    pub name: String,
    pub version: String,
}

/// Processador e os conjuntos de instrução que o processo pode usar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cpu {
    pub model: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub avx2: bool,
    pub avx512: bool,
}

/// Memória principal, em bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

/// Espaço do sistema de arquivos que guarda os dados do Voxel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskSpace {
    pub path: std::path::PathBuf,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// GPU exposta pelo Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gpu {
    pub vendor: Vendor,
    pub name: String,
    pub vram_bytes: u64,
    pub device_id: u32,
    pub driver_name: String,
    pub driver_version: String,
    /// ISA real da GPU AMD, como `gfx1102`. Ausente em NVIDIA e Intel.
    pub isa: Option<String>,
}

/// Fabricante PCI da GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    Amd,
    Nvidia,
    Intel,
    Other(u32),
}
