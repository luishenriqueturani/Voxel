//! Detecção de hardware, perfis de hardware e memória de compatibilidade local.
//!
//! A leitura do sistema fica nesta crate. O que dá para decidir a partir de um texto
//! (os-release, cpuinfo, rocminfo, topologia do KFD) é função pura e tem teste próprio.

mod compat;
mod cpu;
mod defaults;
mod disk;
mod fallback;
mod gpu;
mod memory;
mod os;
mod power;
mod profile;
mod report;
mod rocm;
mod sensors;
mod setup;
mod suggestions;

pub use compat::{Combination, CompatibilityMemory, Failure};
pub use defaults::{GenerationDefaults, defaults_for};
pub use fallback::{
    BackendFacts, ImageBackend, PythonBackend, facts_from, image_fallback, python_fallback,
};
pub use memory::parse_meminfo;
pub use power::{
    POWER_LIMIT_MILLIWATTS, PowerNotice, power_notice, read_power_caps, uses_listed_connector,
};
pub use profile::{Profile, profile_for};
pub use report::{Cpu, DiskSpace, Driver, Gpu, MachineReport, Memory, OperatingSystem, Vendor};
pub use sensors::{
    CpuTimes, celsius_from_millidegree, cpu_percent, gpu_busy_percent, hwmon_celsius,
    parse_byte_count, parse_cpu_times, vram_bytes,
};
pub use setup::{
    Attempt, AttemptState, ImageSetup, ProbeVerdict, SETUP_CONFIGURATION, SETUP_MODEL, device_for,
    generation_args, hardware_line, image_setup, parse_devices,
};
pub use suggestions::{ModelSuggestion, suggestions_for};

/// Lê a máquina em que o processo está rodando.
///
/// O que não existir nesta máquina volta vazio ou falso: sem Vulkan a lista de GPUs fica
/// vazia, sem `rocminfo` o `ROCm` fica ausente, sem NVML o CUDA fica ausente.
#[must_use]
pub fn detect() -> MachineReport {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let layout = cpu::parse_cpuinfo(&cpuinfo);
    let features = cpu::instruction_sets();
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut os = os::parse_os_release(&os_release);
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_default()
        .trim()
        .clone_into(&mut os.kernel);

    let memory = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| memory::parse_meminfo(&text))
        .unwrap_or(Memory {
            total_bytes: 0,
            available_bytes: 0,
        });

    let disk = disk::space_for_data_dir();
    let mut gpus = gpu::enumerate();
    let agents = rocm::agents();
    rocm::attach_isa(&mut gpus, &agents);
    if let Some(version) = os::nvidia_driver_version() {
        os.drivers.push(Driver {
            name: "nvidia".to_owned(),
            version,
        });
    }
    for gpu in &gpus {
        push_driver(&mut os.drivers, &gpu.driver_name, &gpu.driver_version);
    }

    MachineReport {
        os,
        cpu: Cpu {
            model: layout.model,
            physical_cores: layout.physical_cores,
            logical_cores: layout.logical_cores,
            avx2: features.avx2,
            avx512: features.avx512,
        },
        memory,
        disk,
        rocm: !agents.is_empty(),
        rocm_version: rocm::version(),
        cuda: rocm::cuda_present(),
        gpus,
    }
}

fn push_driver(drivers: &mut Vec<Driver>, name: &str, version: &str) {
    if name.is_empty() || version.is_empty() {
        return;
    }
    if drivers
        .iter()
        .any(|driver| driver.name == name && driver.version == version)
    {
        return;
    }
    drivers.push(Driver {
        name: name.to_owned(),
        version: version.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::{detect, push_driver};
    use crate::report::Driver;

    #[test]
    fn a_maquina_de_teste_tem_sistema_cpu_e_memoria() {
        let machine = detect();
        assert!(!machine.os.kernel.is_empty());
        assert!(!machine.os.name.is_empty());
        assert!(machine.cpu.logical_cores > 0);
        assert!(machine.memory.total_bytes > 0);
        assert!(machine.disk.total_bytes > 0);
    }

    #[test]
    fn nao_repete_o_mesmo_driver() {
        let mut drivers = vec![Driver {
            name: "radv".to_owned(),
            version: "Mesa 26.2.4".to_owned(),
        }];
        push_driver(&mut drivers, "radv", "Mesa 26.2.4");
        push_driver(&mut drivers, "", "Mesa 26.2.4");
        push_driver(&mut drivers, "nvidia", "570.86.16");
        assert_eq!(drivers.len(), 2);
    }
}
