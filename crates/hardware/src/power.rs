use std::collections::HashMap;
use std::path::Path;

use nvml_wrapper::Nvml;

use crate::report::Gpu;

/// 300 W, em miliwatts. Teto a partir do qual o aviso deixa de aparecer.
pub const POWER_LIMIT_MILLIWATTS: u64 = 300_000;

/// Aviso único de power limit, com o nome da placa que o disparou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerNotice {
    pub gpu_name: String,
}

const LISTED: &[&[&str]] = &[
    &["3090", "ti"],
    &["4080"],
    &["4090"],
    &["5070", "ti"],
    &["5080"],
    &["5090"],
    &["pro", "6000"],
    &["9070", "xt"],
];

/// Diz se o nome da placa está na lista do conector 12V-2x6 do §6.9.
///
/// A comparação é por tokens, então `5070` não pega a `5070 Ti`, `3090` não pega a `3090 Ti`
/// e `9070` não pega a `9070 XT`. O token `4080` também pega a 4080 Super.
#[must_use]
pub fn uses_listed_connector(name: &str) -> bool {
    let tokens = tokens(name);
    LISTED.iter().any(|model| {
        tokens.windows(model.len()).any(|window| {
            window
                .iter()
                .zip(model.iter())
                .all(|(token, expected)| token == expected)
        })
    })
}

/// Devolve a primeira placa da lista cujo teto falta ou passa de 300 W.
///
/// `caps_milliwatts` acompanha `gpus` pelo índice. Um teto de 300 W ou menos cala o aviso.
/// Sem leitura, o aviso aparece: não dá para afirmar que a placa já está limitada.
#[must_use]
pub fn power_notice(gpus: &[Gpu], caps_milliwatts: &[Option<u64>]) -> Option<PowerNotice> {
    gpus.iter().enumerate().find_map(|(index, gpu)| {
        if !uses_listed_connector(&gpu.name) {
            return None;
        }
        let cap = caps_milliwatts.get(index).copied().flatten();
        match cap {
            Some(milliwatts) if milliwatts <= POWER_LIMIT_MILLIWATTS => None,
            _ => Some(PowerNotice {
                gpu_name: gpu.name.clone(),
            }),
        }
    })
}

/// Lê o teto vigente de cada GPU, em miliwatts.
///
/// Na AMD o valor vem de `power1_cap`, em microwatts. Na NVIDIA vem do NVML, já em miliwatts.
/// A leitura não precisa de root. O que não der para ler volta `None`.
#[must_use]
pub fn read_power_caps(gpus: &[Gpu]) -> Vec<Option<u64>> {
    let caps = collect_caps();
    gpus.iter()
        .map(|gpu| caps.get(&gpu.device_id).copied())
        .collect()
}

fn tokens(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn collect_caps() -> HashMap<u32, u64> {
    let mut caps = sysfs_caps();
    nvml_caps(&mut caps);
    caps
}

fn sysfs_caps() -> HashMap<u32, u64> {
    let mut caps = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return caps;
    };
    for entry in entries.flatten() {
        if !is_drm_card(&entry.file_name()) {
            continue;
        }
        let device = entry.path().join("device");
        let Ok(id_text) = std::fs::read_to_string(device.join("device")) else {
            continue;
        };
        let Some(id) = parse_device_id(&id_text) else {
            continue;
        };
        if let Some(milliwatts) = hwmon_cap(&device) {
            caps.insert(id, milliwatts);
        }
    }
    caps
}

fn is_drm_card(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(rest) = name.strip_prefix("card") else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

fn hwmon_cap(device: &Path) -> Option<u64> {
    let entries = std::fs::read_dir(device.join("hwmon")).ok()?;
    for entry in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path().join("power1_cap")) else {
            continue;
        };
        if let Some(milliwatts) = milliwatts_from_microwatts(&text) {
            return Some(milliwatts);
        }
    }
    None
}

fn milliwatts_from_microwatts(text: &str) -> Option<u64> {
    let microwatts: u64 = text.trim().parse().ok()?;
    Some(microwatts / 1_000)
}

fn parse_device_id(text: &str) -> Option<u32> {
    let text = text.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    u32::from_str_radix(hex, 16).ok()
}

/// NVML junta o device id nos 16 bits altos e o vendor nos baixos.
fn nvml_device_id(pci_device_id: u32) -> u32 {
    pci_device_id >> 16
}

fn nvml_caps(caps: &mut HashMap<u32, u64>) {
    let Ok(nvml) = Nvml::init() else {
        return;
    };
    let Ok(count) = nvml.device_count() else {
        return;
    };
    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let Ok(pci) = device.pci_info() else {
            continue;
        };
        let Ok(limit) = device.power_management_limit() else {
            continue;
        };
        caps.insert(nvml_device_id(pci.pci_device_id), u64::from(limit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Vendor;

    fn gpu(name: &str) -> Gpu {
        Gpu {
            vendor: Vendor::Nvidia,
            name: name.to_owned(),
            vram_bytes: 0,
            device_id: 0,
            driver_name: String::new(),
            driver_version: String::new(),
            isa: None,
        }
    }

    fn notice(name: &str, cap: Option<u64>) -> Option<String> {
        power_notice(&[gpu(name)], &[cap]).map(|found| found.gpu_name)
    }

    #[test]
    fn avisa_as_placas_da_lista_quando_o_teto_passa_de_300_w() {
        let above = Some(575_000);
        assert_eq!(
            notice("NVIDIA GeForce RTX 3090 Ti", above),
            Some("NVIDIA GeForce RTX 3090 Ti".to_owned())
        );
        assert_eq!(
            notice("NVIDIA GeForce RTX 4080", above).as_deref(),
            Some("NVIDIA GeForce RTX 4080")
        );
        assert_eq!(
            notice("NVIDIA GeForce RTX 4080 SUPER", None).as_deref(),
            Some("NVIDIA GeForce RTX 4080 SUPER")
        );
        assert_eq!(
            notice("GeForce RTX 4090", above).as_deref(),
            Some("GeForce RTX 4090")
        );
        assert_eq!(notice("RTX 5070 Ti", above).as_deref(), Some("RTX 5070 Ti"));
        assert_eq!(notice("RTX 5080", above).as_deref(), Some("RTX 5080"));
        assert_eq!(notice("RTX 5090", above).as_deref(), Some("RTX 5090"));
        assert_eq!(
            notice("NVIDIA RTX PRO 6000 Blackwell", None).as_deref(),
            Some("NVIDIA RTX PRO 6000 Blackwell")
        );
        assert_eq!(
            notice("AMD Radeon RX 9070 XT", above).as_deref(),
            Some("AMD Radeon RX 9070 XT")
        );
    }

    #[test]
    fn nao_avisa_fora_da_lista_nem_com_teto_de_300_w_ou_menos() {
        assert!(notice("NVIDIA GeForce RTX 3090", Some(350_000)).is_none());
        assert!(notice("NVIDIA GeForce RTX 5070", Some(250_000)).is_none());
        assert!(notice("AMD Radeon RX 9070", Some(220_000)).is_none());
        assert!(notice("AMD Radeon RX 7600", Some(145_000)).is_none());
        assert!(notice("NVIDIA GeForce RTX 4070", None).is_none());
        assert!(notice("NVIDIA GeForce RTX 4090", Some(POWER_LIMIT_MILLIWATTS)).is_none());
        assert!(notice("NVIDIA GeForce RTX 4090", Some(280_000)).is_none());
    }

    #[test]
    fn sem_leitura_do_teto_a_placa_da_lista_ainda_avisa() {
        assert_eq!(notice("RTX 5090", None).as_deref(), Some("RTX 5090"));
    }

    #[test]
    fn a_segunda_placa_avisa_se_a_primeira_ja_esta_limitada() {
        let gpus = [gpu("RTX 4090"), gpu("RTX 5090")];
        let caps = [Some(POWER_LIMIT_MILLIWATTS), Some(575_000)];
        let found = power_notice(&gpus, &caps).expect("aviso");
        assert_eq!(found.gpu_name, "RTX 5090");
    }

    #[test]
    fn converte_o_teto_da_amd_e_o_device_id() {
        assert_eq!(milliwatts_from_microwatts("145000000\n"), Some(145_000));
        assert_eq!(milliwatts_from_microwatts("300000000"), Some(300_000));
        assert_eq!(milliwatts_from_microwatts("nope"), None);
        assert_eq!(parse_device_id("0x7480\n"), Some(0x7480));
        assert_eq!(parse_device_id("0X10DE"), Some(0x10DE));
        assert_eq!(parse_device_id("zz"), None);
        assert_eq!(nvml_device_id(0x2684_10DE), 0x2684);
        assert!(!is_drm_card(std::ffi::OsStr::new("card1-DP-1")));
        assert!(is_drm_card(std::ffi::OsStr::new("card1")));
    }

    #[test]
    fn a_leitura_acompanha_cada_gpu() {
        let mut card = gpu("AMD Radeon RX 7600");
        card.device_id = 0x7480;
        card.vendor = Vendor::Amd;
        let caps = read_power_caps(std::slice::from_ref(&card));
        assert_eq!(caps.len(), 1);
        if sysfs_has_device(0x7480) {
            let milliwatts = caps[0].expect("teto da placa");
            assert!(milliwatts > 0);
            assert!(milliwatts <= POWER_LIMIT_MILLIWATTS);
            assert!(power_notice(&[card], &caps).is_none());
        }
    }

    fn sysfs_has_device(device_id: u32) -> bool {
        let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
            return false;
        };
        entries.flatten().any(|entry| {
            if !is_drm_card(&entry.file_name()) {
                return false;
            }
            let text =
                std::fs::read_to_string(entry.path().join("device/device")).unwrap_or_default();
            parse_device_id(&text) == Some(device_id)
        })
    }
}
