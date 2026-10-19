//! Barra de status: GPU, backend, uso, VRAM, CPU, RAM e temperaturas.
//!
//! A primeira leitura de CPU não tem percentual, porque `/proc/stat` só diz o acumulado.
//! A leitura seguinte, alguns segundos depois, compara com a anterior.

use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use voxel_hardware::{
    CpuTimes, cpu_percent, gpu_busy_percent, hwmon_celsius, parse_cpu_times, parse_meminfo,
    vram_bytes,
};

use crate::db::Database;
use crate::generate::backend_choice;

/// Instantâneo mostrado na barra de baixo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineStatus {
    pub gpu_name: Option<String>,
    pub backend: String,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub gpu_percent: Option<u8>,
    pub cpu_percent: Option<u8>,
    pub ram_used_bytes: Option<u64>,
    pub ram_total_bytes: Option<u64>,
    pub cpu_celsius: Option<i32>,
    pub gpu_celsius: Option<i32>,
}

static PREVIOUS_CPU: Mutex<Option<CpuTimes>> = Mutex::new(None);
static GPU_NAME: OnceLock<Option<String>> = OnceLock::new();

/// Lê a máquina agora. Temperatura ou VRAM ausentes voltam vazias, sem valor inventado.
///
/// # Errors
///
/// Falha se o backend gravado no banco não puder ser lido.
pub fn read_status(database: &Database) -> Result<MachineStatus, String> {
    let backend = backend_choice(database)?.current;
    let stat = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let sample = parse_cpu_times(&stat);
    let cpu = sample.and_then(|current| {
        let Ok(mut previous) = PREVIOUS_CPU.lock() else {
            return None;
        };
        let percent = previous.and_then(|before| cpu_percent(before, current));
        *previous = Some(current);
        percent
    });
    let memory = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .as_deref()
        .and_then(parse_meminfo);
    let (ram_used_bytes, ram_total_bytes) = memory.map_or((None, None), |memory| {
        (
            Some(memory.total_bytes.saturating_sub(memory.available_bytes)),
            Some(memory.total_bytes),
        )
    });
    let vram = vram_bytes(std::path::Path::new("/sys/class/drm"));
    Ok(MachineStatus {
        gpu_name: gpu_name(),
        backend,
        vram_used_bytes: vram.map(|(used, _)| used),
        vram_total_bytes: vram.map(|(_, total)| total),
        gpu_percent: gpu_busy_percent(std::path::Path::new("/sys/class/drm")),
        cpu_percent: cpu,
        ram_used_bytes,
        ram_total_bytes,
        cpu_celsius: hwmon_celsius(
            std::path::Path::new("/sys/class/hwmon"),
            &["k10temp", "coretemp", "zenpower"],
        ),
        gpu_celsius: hwmon_celsius(
            std::path::Path::new("/sys/class/hwmon"),
            &["amdgpu", "nvidia"],
        ),
    })
}

fn gpu_name() -> Option<String> {
    GPU_NAME
        .get_or_init(|| {
            voxel_hardware::detect()
                .gpus
                .into_iter()
                .max_by_key(|gpu| gpu.vram_bytes)
                .map(|gpu| gpu.name)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::read_status;
    use crate::db::Database;

    #[test]
    fn a_leitura_desta_maquina_traz_a_ram() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let status = read_status(&database).unwrap();
        assert!(status.ram_total_bytes.unwrap_or(0) > 0);
        assert!(!status.backend.is_empty());
    }
}
