//! ISA real da GPU AMD e presença de `ROCm` e CUDA.
//!
//! O `rocminfo` herda `HSA_OVERRIDE_GFX_VERSION`. Com `11.0.0` ele anuncia `gfx1100` numa
//! RX 7600, cuja ISA é `gfx1102`. A consulta tira essa variável do processo filho.

use std::process::Command;

use crate::report::Gpu;

pub struct Agent {
    pub device_id: u32,
    pub isa: String,
}

pub fn agents() -> Vec<Agent> {
    read_rocminfo()
        .map(|text| parse_rocminfo(&text))
        .filter(|found| !found.is_empty())
        .unwrap_or_else(read_kfd)
}

pub fn version() -> Option<String> {
    let text = std::fs::read_to_string("/opt/rocm/.info/version").ok()?;
    let version = text.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_owned())
    }
}

pub fn attach_isa(gpus: &mut [Gpu], agents: &[Agent]) {
    for gpu in gpus
        .iter_mut()
        .filter(|gpu| gpu.vendor == crate::report::Vendor::Amd)
    {
        gpu.isa = agents
            .iter()
            .find(|agent| agent.device_id == gpu.device_id)
            .map(|agent| agent.isa.clone());
    }
}

pub fn cuda_present() -> bool {
    nvml_wrapper::Nvml::init()
        .and_then(|nvml| nvml.device_count())
        .is_ok_and(|count| count > 0)
}

pub fn parse_rocminfo(text: &str) -> Vec<Agent> {
    text.split("Agent ")
        .skip(1)
        .filter(|block| field(block, "Device Type").as_deref() == Some("GPU"))
        .filter_map(|block| {
            let isa = field(block, "Name")?;
            if !isa.starts_with("gfx") {
                return None;
            }
            let chip = field(block, "Chip ID")?;
            let device_id = chip.split('(').next()?.trim().parse().ok()?;
            Some(Agent { device_id, isa })
        })
        .collect()
}

pub fn parse_kfd_node(text: &str) -> Option<Agent> {
    let simd = value(text, "simd_count")?.parse::<u32>().ok()?;
    if simd == 0 {
        return None;
    }
    let device_id = value(text, "device_id")?.parse().ok()?;
    let version = value(text, "gfx_target_version")?.parse().ok()?;
    Some(Agent {
        device_id,
        isa: isa_from_gfx_target(version)?,
    })
}

pub fn isa_from_gfx_target(version: u32) -> Option<String> {
    if version == 0 {
        return None;
    }
    let major = version / 10_000;
    let minor = (version / 100) % 100;
    let stepping = version % 100;
    if major == 0 {
        return None;
    }
    Some(format!("gfx{major}{minor}{stepping}"))
}

fn read_rocminfo() -> Option<String> {
    command_output("/opt/rocm/bin/rocminfo").or_else(|| command_output("rocminfo"))
}

fn command_output(program: &str) -> Option<String> {
    let output = Command::new(program)
        .env_remove("HSA_OVERRIDE_GFX_VERSION")
        .env_remove("PYTORCH_ROCM_ARCH")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn read_kfd() -> Vec<Agent> {
    let Ok(entries) = std::fs::read_dir("/sys/class/kfd/kfd/topology/nodes") else {
        return Vec::new();
    };
    let mut agents = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path().join("properties");
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if let Some(agent) = parse_kfd_node(&text) {
            agents.push(agent);
        }
    }
    agents
}

fn field(block: &str, key: &str) -> Option<String> {
    for line in block.lines() {
        let trimmed = line.trim();
        let Some((name, value)) = trimmed.split_once(':') else {
            continue;
        };
        if name == key {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some(key) {
            return parts.next();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{attach_isa, isa_from_gfx_target, parse_kfd_node, parse_rocminfo};
    use crate::report::{Gpu, Vendor};

    const ROCMINFO: &str = "\
Agent 1
  Name:                    AMD Ryzen 7 5700X 8-Core Processor
  Device Type:             CPU
  Chip ID:                 0(0x0)
Agent 2
  Name:                    gfx1102
  Marketing Name:          AMD Radeon RX 7600
  Vendor Name:             AMD
  Device Type:             GPU
  Chip ID:                 29824(0x7480)
";

    #[test]
    fn le_so_a_gpu_e_o_chip() {
        let agents = parse_rocminfo(ROCMINFO);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].isa, "gfx1102");
        assert_eq!(agents[0].device_id, 0x7480);
    }

    #[test]
    fn kfd_ignora_o_no_da_cpu_e_decodifica_a_isa() {
        assert!(parse_kfd_node("simd_count 0\ngfx_target_version 0\ndevice_id 0\n").is_none());
        let agent =
            parse_kfd_node("simd_count 64\ngfx_target_version 110002\ndevice_id 29824\n").unwrap();
        assert_eq!(agent.isa, "gfx1102");
        assert_eq!(agent.device_id, 29824);
    }

    #[test]
    fn versoes_gfx_conhecidas() {
        assert_eq!(isa_from_gfx_target(110_002).as_deref(), Some("gfx1102"));
        assert_eq!(isa_from_gfx_target(110_000).as_deref(), Some("gfx1100"));
        assert_eq!(isa_from_gfx_target(100_300).as_deref(), Some("gfx1030"));
        assert_eq!(isa_from_gfx_target(0), None);
    }

    #[test]
    fn casa_a_isa_pelo_device_id() {
        let mut gpus = vec![Gpu {
            vendor: Vendor::Amd,
            name: "AMD Radeon RX 7600 (RADV NAVI33)".to_owned(),
            vram_bytes: 8,
            device_id: 0x7480,
            driver_name: "radv".to_owned(),
            driver_version: "Mesa 26.2.4".to_owned(),
            isa: None,
        }];
        attach_isa(&mut gpus, &parse_rocminfo(ROCMINFO));
        assert_eq!(gpus[0].isa.as_deref(), Some("gfx1102"));
    }

    #[test]
    fn nao_poe_isa_numa_nvidia() {
        let mut gpus = vec![Gpu {
            vendor: Vendor::Nvidia,
            name: "RTX 3060".to_owned(),
            vram_bytes: 8,
            device_id: 0x7480,
            driver_name: "nvidia".to_owned(),
            driver_version: "570".to_owned(),
            isa: None,
        }];
        attach_isa(&mut gpus, &parse_rocminfo(ROCMINFO));
        assert_eq!(gpus[0].isa, None);
    }
}
