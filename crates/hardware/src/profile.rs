use crate::report::Gpu;

const GIB: u64 = 1024 * 1024 * 1024;

/// Perfil de hardware do §6.6, escolhido pela maior VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Sem GPU.
    Minimo,
    /// Até 6 GiB. Cobre a GTX 1060.
    Basico,
    /// Acima de 6 GiB e até 10 GiB. Cobre a RX 7600.
    Intermediario,
    /// Acima de 10 GiB e até 18 GiB. Cobre a RTX 4070 e a Intel Arc de 16 GB.
    Avancado,
    /// Acima de 18 GiB. Cobre a RX 7900 de 20 GB e a de 24 GB.
    Alto,
}

impl Profile {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Minimo => "minimo",
            Self::Basico => "basico",
            Self::Intermediario => "intermediario",
            Self::Avancado => "avancado",
            Self::Alto => "alto",
        }
    }
}

/// Escolhe o perfil pela GPU de maior VRAM.
///
/// Sem GPU o perfil é Mínimo, mesmo com muita RAM. Os cortes em GiB binários são 6, 10 e 18.
#[must_use]
pub fn profile_for(gpus: &[Gpu]) -> Profile {
    match gpus.iter().map(|gpu| gpu.vram_bytes).max() {
        None => Profile::Minimo,
        Some(vram) => profile_for_vram(vram),
    }
}

fn profile_for_vram(vram_bytes: u64) -> Profile {
    if vram_bytes <= 6 * GIB {
        Profile::Basico
    } else if vram_bytes <= 10 * GIB {
        Profile::Intermediario
    } else if vram_bytes <= 18 * GIB {
        Profile::Avancado
    } else {
        Profile::Alto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Vendor;

    fn gpu(name: &str, vendor: Vendor, vram_bytes: u64) -> Gpu {
        Gpu {
            vendor,
            name: name.to_owned(),
            vram_bytes,
            device_id: 0,
            driver_name: String::new(),
            driver_version: String::new(),
            isa: None,
        }
    }

    #[test]
    fn so_cpu_fica_no_minimo() {
        assert_eq!(profile_for(&[]), Profile::Minimo);
    }

    #[test]
    fn gtx_1060_de_6_gb_fica_no_basico() {
        let card = gpu("NVIDIA GeForce GTX 1060", Vendor::Nvidia, 6 * GIB);
        assert_eq!(profile_for(&[card]), Profile::Basico);
    }

    #[test]
    fn rx_7600_medida_fica_no_intermediario() {
        // VRAM que o Vulkan leu nesta RX 7600: 8.176 MiB.
        let card = gpu("AMD Radeon RX 7600", Vendor::Amd, 8_573_157_376);
        assert_eq!(profile_for(&[card]), Profile::Intermediario);
    }

    #[test]
    fn rtx_4070_de_12_gb_fica_no_avancado() {
        let card = gpu("NVIDIA GeForce RTX 4070", Vendor::Nvidia, 12 * GIB);
        assert_eq!(profile_for(&[card]), Profile::Avancado);
    }

    #[test]
    fn intel_arc_de_16_gb_fica_no_avancado() {
        let card = gpu("Intel Arc A770", Vendor::Intel, 16 * GIB);
        assert_eq!(profile_for(&[card]), Profile::Avancado);
    }

    #[test]
    fn rx_7900_de_20_e_24_gb_fica_no_alto() {
        let xt = gpu("AMD Radeon RX 7900 XT", Vendor::Amd, 20 * GIB);
        let xtx = gpu("AMD Radeon RX 7900 XTX", Vendor::Amd, 24 * GIB);
        assert_eq!(profile_for(&[xt]), Profile::Alto);
        assert_eq!(profile_for(&[xtx]), Profile::Alto);
    }

    #[test]
    fn a_gpu_de_maior_vram_decide() {
        let integrada = gpu("AMD Radeon Graphics", Vendor::Amd, GIB / 2);
        let discreta = gpu("NVIDIA GeForce RTX 4070", Vendor::Nvidia, 12 * GIB);
        assert_eq!(profile_for(&[integrada, discreta]), Profile::Avancado);
    }

    #[test]
    fn o_id_do_perfil_e_estavel() {
        assert_eq!(Profile::Minimo.id(), "minimo");
        assert_eq!(Profile::Basico.id(), "basico");
        assert_eq!(Profile::Intermediario.id(), "intermediario");
        assert_eq!(Profile::Avancado.id(), "avancado");
        assert_eq!(Profile::Alto.id(), "alto");
    }
}
