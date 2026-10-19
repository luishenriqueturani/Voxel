use serde::Serialize;

use crate::profile::Profile;

/// Modelo sugerido no setup, com o tamanho do arquivo e a licença.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSuggestion {
    pub id: &'static str,
    /// Tamanho do arquivo de pesos, em bytes.
    pub bytes: u64,
    pub license: &'static str,
    /// Opção extra do perfil. O setup não a marca sozinho.
    pub optional: bool,
}

// Tamanhos conferidos: SD 1.5 em docs/pesquisa/2d-stable-diffusion-cpp.md,
// TripoSR no `model.ckpt` publicado, Stable Fast 3D em docs/pesquisa/3d-modelos.md
// e U²-Net no `u2net.onnx` usado na pesquisa dos auxiliares.
const SD15: ModelSuggestion = ModelSuggestion {
    id: "sd15",
    bytes: 4_265_146_304,
    license: "CreativeML OpenRAIL-M",
    optional: false,
};
const TRIPOSR: ModelSuggestion = ModelSuggestion {
    id: "triposr",
    bytes: 1_677_246_742,
    license: "MIT",
    optional: false,
};
const U2NET: ModelSuggestion = ModelSuggestion {
    id: "u2net",
    bytes: 175_997_641,
    license: "Apache-2.0",
    optional: false,
};
const SF3D: ModelSuggestion = ModelSuggestion {
    id: "sf3d",
    bytes: 4_024_289_892,
    license: "Stability AI Community License",
    optional: true,
};

const BASE: &[ModelSuggestion] = &[SD15, TRIPOSR, U2NET];
const INTERMEDIARIO: &[ModelSuggestion] = &[SD15, TRIPOSR, U2NET, SF3D];

/// Modelos do MVP que cabem no perfil. SDXL, Flux e os modelos 3D pesados ficam de fora.
#[must_use]
pub const fn suggestions_for(profile: Profile) -> &'static [ModelSuggestion] {
    if matches!(profile, Profile::Intermediario) {
        INTERMEDIARIO
    } else {
        BASE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_intermediario_inclui_o_stable_fast_3d_como_opcao() {
        let models = suggestions_for(Profile::Intermediario);
        assert!(
            models
                .iter()
                .any(|model| model.id == "sf3d" && model.optional)
        );
        assert_eq!(models[0].bytes, 4_265_146_304);
        assert_eq!(models[0].license, "CreativeML OpenRAIL-M");
        assert_eq!(models[1].license, "MIT");
        assert_eq!(models[2].license, "Apache-2.0");
    }

    #[test]
    fn os_outros_perfis_ficam_no_mvp_sem_a_opcao_texturizada() {
        for profile in [
            Profile::Minimo,
            Profile::Basico,
            Profile::Avancado,
            Profile::Alto,
        ] {
            let models = suggestions_for(profile);
            assert_eq!(models.len(), 3);
            assert!(models.iter().all(|model| model.id != "sf3d"));
        }
    }
}
