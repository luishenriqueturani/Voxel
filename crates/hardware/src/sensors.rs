//! Leituras de CPU, RAM, VRAM e temperatura a partir de textos e do sysfs.
//!
//! O percentual de CPU precisa de duas amostras de `/proc/stat`. Temperatura e VRAM
//! vêm de arquivos do sysfs; se o arquivo não existir, a leitura volta vazia.

use std::path::Path;

/// Soma ociosa e total da linha `cpu` de `/proc/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    pub idle: u64,
    pub total: u64,
}

/// Lê a primeira linha agregada `cpu` de `/proc/stat`.
#[must_use]
pub fn parse_cpu_times(text: &str) -> Option<CpuTimes> {
    let line = text.lines().find(|line| line.starts_with("cpu "))?;
    let numbers = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let idle = *numbers.get(3)?;
    let total = numbers.iter().sum();
    Some(CpuTimes { idle, total })
}

/// Percentual ocupado entre duas amostras. `None` se o intervalo não andou.
#[must_use]
pub fn cpu_percent(before: CpuTimes, after: CpuTimes) -> Option<u8> {
    let total = after.total.saturating_sub(before.total);
    if total == 0 {
        return None;
    }
    let idle = after.idle.saturating_sub(before.idle).min(total);
    let busy = total - idle;
    let percent = busy.saturating_mul(100) / total;
    u8::try_from(percent.min(100)).ok()
}

/// Inteiro escrito sozinho num arquivo do sysfs, como `mem_info_vram_used`.
#[must_use]
pub fn parse_byte_count(text: &str) -> Option<u64> {
    text.split_whitespace().next()?.parse().ok()
}

/// Converte `temp*_input`, que o kernel grava em miligraus Celsius.
#[must_use]
pub fn celsius_from_millidegree(value: i64) -> Option<i32> {
    i32::try_from(value / 1_000).ok()
}

/// Temperatura do primeiro hwmon cujo `name` está na lista.
///
/// `root` é o diretório que contém `hwmon0`, `hwmon1`, … Cada um tem `name` e `temp1_input`.
#[must_use]
pub fn hwmon_celsius(root: &Path, names: &[&str]) -> Option<i32> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let dir = entry.path();
        let Ok(name) = std::fs::read_to_string(dir.join("name")) else {
            continue;
        };
        if !names.iter().any(|wanted| name.trim() == *wanted) {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(dir.join("temp1_input")) else {
            continue;
        };
        let Some(millidegree) = parse_byte_count(&raw).and_then(|value| i64::try_from(value).ok())
        else {
            continue;
        };
        return celsius_from_millidegree(millidegree);
    }
    None
}

/// VRAM usada e total da placa com mais memória em `root` (`/sys/class/drm`).
#[must_use]
pub fn vram_bytes(root: &Path) -> Option<(u64, u64)> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut best: Option<(u64, u64)> = None;
    for entry in entries.filter_map(Result::ok) {
        let device = entry.path().join("device");
        let Ok(used_text) = std::fs::read_to_string(device.join("mem_info_vram_used")) else {
            continue;
        };
        let Ok(total_text) = std::fs::read_to_string(device.join("mem_info_vram_total")) else {
            continue;
        };
        let (Some(used), Some(total)) =
            (parse_byte_count(&used_text), parse_byte_count(&total_text))
        else {
            continue;
        };
        if best.is_none_or(|(_, current)| total > current) {
            best = Some((used, total));
        }
    }
    best
}

/// Percentual de ocupação da GPU, lido de `gpu_busy_percent` no sysfs da AMD.
#[must_use]
pub fn gpu_busy_percent(root: &Path) -> Option<u8> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let Ok(text) = std::fs::read_to_string(entry.path().join("device/gpu_busy_percent")) else {
            continue;
        };
        let Some(value) = parse_byte_count(&text) else {
            continue;
        };
        return u8::try_from(value.min(100)).ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        CpuTimes, celsius_from_millidegree, cpu_percent, gpu_busy_percent, hwmon_celsius,
        parse_byte_count, parse_cpu_times, vram_bytes,
    };

    #[test]
    fn o_uso_de_cpu_sai_da_diferenca_entre_duas_amostras() {
        let before = parse_cpu_times("cpu  10 0 10 80 0 0 0 0\ncpu0 1 0 1 8 0 0 0 0\n").unwrap();
        assert_eq!(
            before,
            CpuTimes {
                idle: 80,
                total: 100
            }
        );
        let after = CpuTimes {
            idle: 90,
            total: 200,
        };
        assert_eq!(cpu_percent(before, after), Some(90));
        assert_eq!(cpu_percent(before, before), None);
        assert!(parse_cpu_times("cpu0 1 2 3\n").is_none());
    }

    #[test]
    fn temperatura_e_vram_saem_dos_arquivos_do_sysfs() {
        assert_eq!(celsius_from_millidegree(54_500), Some(54));
        assert_eq!(parse_byte_count("8192\n"), Some(8192));

        let root = std::env::temp_dir().join(format!("voxel-sensors-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cpu = root.join("hwmon/hwmon0");
        let gpu = root.join("hwmon/hwmon1");
        std::fs::create_dir_all(&cpu).unwrap();
        std::fs::create_dir_all(&gpu).unwrap();
        std::fs::write(cpu.join("name"), "k10temp\n").unwrap();
        std::fs::write(cpu.join("temp1_input"), "54500\n").unwrap();
        std::fs::write(gpu.join("name"), "amdgpu\n").unwrap();
        std::fs::write(gpu.join("temp1_input"), "48000\n").unwrap();
        assert_eq!(hwmon_celsius(&root.join("hwmon"), &["k10temp"]), Some(54));
        assert_eq!(hwmon_celsius(&root.join("hwmon"), &["amdgpu"]), Some(48));
        assert_eq!(hwmon_celsius(&root.join("hwmon"), &["coretemp"]), None);

        let card = root.join("drm/card0/device");
        std::fs::create_dir_all(&card).unwrap();
        std::fs::write(card.join("mem_info_vram_used"), "1000\n").unwrap();
        std::fs::write(card.join("mem_info_vram_total"), "8000\n").unwrap();
        std::fs::write(card.join("gpu_busy_percent"), "37\n").unwrap();
        assert_eq!(vram_bytes(&root.join("drm")), Some((1000, 8000)));
        assert_eq!(gpu_busy_percent(&root.join("drm")), Some(37));
        let _ = std::fs::remove_dir_all(&root);
    }
}
