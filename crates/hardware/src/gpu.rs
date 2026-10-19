//! GPUs pelo Vulkan. Só entram dispositivos discretos e integrados; o `llvmpipe` fica de fora.

use crate::report::{Gpu, Vendor};

const AMD: u32 = 0x1002;
const NVIDIA: u32 = 0x10de;
const INTEL: u32 = 0x8086;

pub struct Heap {
    pub size: u64,
    pub device_local: bool,
    pub host_visible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GpuClass {
    Discrete,
    Integrated,
    Other,
}

pub fn vendor_from_id(id: u32) -> Vendor {
    match id {
        AMD => Vendor::Amd,
        NVIDIA => Vendor::Nvidia,
        INTEL => Vendor::Intel,
        other => Vendor::Other(other),
    }
}

pub fn is_reported_gpu(class: GpuClass) -> bool {
    matches!(class, GpuClass::Discrete | GpuClass::Integrated)
}

/// VRAM dedicada. Sem um heap só da placa, usa a memória local do integrado.
#[must_use]
pub fn vram_bytes(heaps: &[Heap]) -> u64 {
    let dedicated: u64 = heaps
        .iter()
        .filter(|heap| heap.device_local && !heap.host_visible)
        .map(|heap| heap.size)
        .sum();
    if dedicated > 0 {
        return dedicated;
    }
    heaps
        .iter()
        .filter(|heap| heap.device_local)
        .map(|heap| heap.size)
        .sum()
}

pub fn enumerate() -> Vec<Gpu> {
    enumerate_vulkan().unwrap_or_default()
}

#[allow(unsafe_code)]
fn enumerate_vulkan() -> Option<Vec<Gpu>> {
    let entry = unsafe { ash::Entry::load() }.ok()?;
    let app_info = ash::vk::ApplicationInfo::default().api_version(ash::vk::API_VERSION_1_3);
    let create_info = ash::vk::InstanceCreateInfo::default().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&create_info, None) }.ok()?;
    let Ok(devices) = (unsafe { instance.enumerate_physical_devices() }) else {
        unsafe { instance.destroy_instance(None) };
        return None;
    };

    let mut gpus = Vec::new();
    for device in devices {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let class = match properties.device_type {
            ash::vk::PhysicalDeviceType::DISCRETE_GPU => GpuClass::Discrete,
            ash::vk::PhysicalDeviceType::INTEGRATED_GPU => GpuClass::Integrated,
            _ => GpuClass::Other,
        };
        if !is_reported_gpu(class) {
            continue;
        }

        let mut driver = ash::vk::PhysicalDeviceDriverProperties::default();
        let mut properties2 = ash::vk::PhysicalDeviceProperties2::default().push_next(&mut driver);
        unsafe { instance.get_physical_device_properties2(device, &mut properties2) };

        let memory = unsafe { instance.get_physical_device_memory_properties(device) };
        let heaps = heaps_from(&memory);
        gpus.push(Gpu {
            vendor: vendor_from_id(properties.vendor_id),
            name: decode_c_chars(&properties.device_name),
            vram_bytes: vram_bytes(&heaps),
            device_id: properties.device_id,
            driver_name: decode_c_chars(&driver.driver_name),
            driver_version: decode_c_chars(&driver.driver_info),
            isa: None,
        });
    }

    unsafe { instance.destroy_instance(None) };
    Some(gpus)
}

fn heaps_from(memory: &ash::vk::PhysicalDeviceMemoryProperties) -> Vec<Heap> {
    let count = memory.memory_heap_count as usize;
    (0..count)
        .map(|index| {
            let heap = memory.memory_heaps[index];
            let host_visible = memory
                .memory_types
                .iter()
                .take(memory.memory_type_count as usize)
                .any(|kind| {
                    kind.heap_index == u32::try_from(index).unwrap_or(u32::MAX)
                        && kind
                            .property_flags
                            .contains(ash::vk::MemoryPropertyFlags::HOST_VISIBLE)
                });
            Heap {
                size: heap.size,
                device_local: heap.flags.contains(ash::vk::MemoryHeapFlags::DEVICE_LOCAL),
                host_visible,
            }
        })
        .collect()
}

fn decode_c_chars(chars: &[std::ffi::c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .copied()
        .map(|ch| u8::try_from(ch).unwrap_or(0))
        .take_while(|byte| *byte != 0)
        .collect();
    String::from_utf8_lossy(&bytes).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{GpuClass, Heap, is_reported_gpu, vendor_from_id, vram_bytes};
    use crate::report::Vendor;

    #[test]
    fn fabricantes_conhecidos() {
        assert_eq!(vendor_from_id(0x1002), Vendor::Amd);
        assert_eq!(vendor_from_id(0x10de), Vendor::Nvidia);
        assert_eq!(vendor_from_id(0x8086), Vendor::Intel);
        assert_eq!(vendor_from_id(0x1234), Vendor::Other(0x1234));
    }

    #[test]
    fn ignora_rasterizador_de_cpu() {
        assert!(is_reported_gpu(GpuClass::Discrete));
        assert!(is_reported_gpu(GpuClass::Integrated));
        assert!(!is_reported_gpu(GpuClass::Other));
    }

    #[test]
    fn vram_dedicada_ignora_o_heap_visivel_pela_cpu() {
        let heaps = vec![
            Heap {
                size: 512 * 1024 * 1024,
                device_local: true,
                host_visible: true,
            },
            Heap {
                size: 8 * 1024 * 1024 * 1024,
                device_local: true,
                host_visible: false,
            },
        ];
        assert_eq!(vram_bytes(&heaps), 8 * 1024 * 1024 * 1024);
    }

    #[test]
    fn integrado_usa_a_memoria_local_visivel() {
        let heaps = vec![Heap {
            size: 1024,
            device_local: true,
            host_visible: true,
        }];
        assert_eq!(vram_bytes(&heaps), 1024);
    }
}
