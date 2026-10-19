//! Núcleos e conjuntos de instrução a partir de `/proc/cpuinfo`.

use std::collections::HashSet;

pub struct CpuLayout {
    pub model: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
}

pub struct InstructionSets {
    pub avx2: bool,
    pub avx512: bool,
}

pub fn parse_cpuinfo(text: &str) -> CpuLayout {
    let blocks: Vec<&str> = text
        .split("\n\n")
        .filter(|block| block.contains("processor"))
        .collect();
    let logical_cores = u32::try_from(blocks.len()).unwrap_or(u32::MAX);
    let model = blocks
        .first()
        .and_then(|block| field(block, "model name"))
        .unwrap_or_default();

    let mut pairs = HashSet::new();
    let mut physical_ids = HashSet::new();
    let mut cores_per_socket = 0_u32;
    for block in &blocks {
        let physical = field(block, "physical id").unwrap_or_else(|| "0".to_owned());
        physical_ids.insert(physical.clone());
        if let Some(cores) = field(block, "cpu cores").and_then(|value| value.parse().ok()) {
            cores_per_socket = cores;
        }
        if let Some(core) = field(block, "core id") {
            pairs.insert((physical, core));
        }
    }
    let physical_cores = if !pairs.is_empty() {
        u32::try_from(pairs.len()).unwrap_or(logical_cores)
    } else if cores_per_socket > 0 {
        cores_per_socket.saturating_mul(u32::try_from(physical_ids.len()).unwrap_or(1))
    } else {
        logical_cores
    };

    CpuLayout {
        model,
        physical_cores,
        logical_cores,
    }
}

pub fn instruction_sets() -> InstructionSets {
    InstructionSets {
        avx2: detect_avx2(),
        avx512: detect_avx512(),
    }
}

fn field(block: &str, key: &str) -> Option<String> {
    for line in block.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == key {
            return Some(value.trim().to_owned());
        }
    }
    None
}

#[cfg(target_arch = "x86_64")]
fn detect_avx2() -> bool {
    std::is_x86_feature_detected!("avx2")
}

#[cfg(target_arch = "x86_64")]
fn detect_avx512() -> bool {
    std::is_x86_feature_detected!("avx512f")
}

#[cfg(not(target_arch = "x86_64"))]
const fn detect_avx2() -> bool {
    false
}

#[cfg(not(target_arch = "x86_64"))]
const fn detect_avx512() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::parse_cpuinfo;

    #[test]
    fn separa_nucleos_fisicos_do_hyperthread() {
        let text = "\
processor\t: 0
model name\t: AMD Ryzen
physical id\t: 0
core id\t\t: 0
cpu cores\t: 2
flags\t\t: avx avx2

processor\t: 1
model name\t: AMD Ryzen
physical id\t: 0
core id\t\t: 1
cpu cores\t: 2
flags\t\t: avx avx2

processor\t: 2
model name\t: AMD Ryzen
physical id\t: 0
core id\t\t: 0
cpu cores\t: 2
flags\t\t: avx avx2

processor\t: 3
model name\t: AMD Ryzen
physical id\t: 0
core id\t\t: 1
cpu cores\t: 2
flags\t\t: avx avx2
";
        let cpu = parse_cpuinfo(text);
        assert_eq!(cpu.model, "AMD Ryzen");
        assert_eq!(cpu.logical_cores, 4);
        assert_eq!(cpu.physical_cores, 2);
    }

    #[test]
    fn dois_soquetes_somam_os_pares() {
        let text = "\
processor : 0
physical id : 0
core id : 0
cpu cores : 1

processor : 1
physical id : 1
core id : 0
cpu cores : 1
";
        let cpu = parse_cpuinfo(text);
        assert_eq!(cpu.physical_cores, 2);
        assert_eq!(cpu.logical_cores, 2);
    }

    #[test]
    fn sem_core_id_usa_o_campo_cpu_cores() {
        let cpu = parse_cpuinfo(
            "processor : 0\nmodel name : CPU\ncpu cores : 8\n\nprocessor : 1\ncpu cores : 8\n",
        );
        assert_eq!(cpu.logical_cores, 2);
        assert_eq!(cpu.physical_cores, 8);
        assert_eq!(cpu.model, "CPU");
    }

    #[test]
    fn texto_vazio_nao_inventa_cpu() {
        let cpu = parse_cpuinfo("");
        assert_eq!(cpu.logical_cores, 0);
        assert!(cpu.model.is_empty());
    }
}
