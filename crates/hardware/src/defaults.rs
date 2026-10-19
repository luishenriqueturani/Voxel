//! Parâmetros iniciais de geração 2D, escolhidos pelo perfil de hardware.
//!
//! O MVP mede o SD 1.5 em 512 px. Perfis menores usam menos passos.

use serde::Serialize;

use crate::profile::Profile;

/// Resolução, passos e CFG que o painel mostra antes de o usuário mexer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationDefaults {
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg: f64,
}

/// Padrões do SD 1.5 para o perfil. A resolução fica em 512, que é o que foi medido.
#[must_use]
pub fn defaults_for(profile: Profile) -> GenerationDefaults {
    let steps = match profile {
        Profile::Minimo | Profile::Basico => 20,
        Profile::Intermediario | Profile::Avancado | Profile::Alto => 25,
    };
    GenerationDefaults {
        width: 512,
        height: 512,
        steps,
        cfg: 7.0,
    }
}

#[cfg(test)]
mod tests {
    use super::defaults_for;
    use crate::profile::Profile;

    #[test]
    fn o_perfil_menor_usa_menos_passos_e_todos_ficam_em_512() {
        let minimo = defaults_for(Profile::Minimo);
        assert_eq!(minimo.width, 512);
        assert_eq!(minimo.height, 512);
        assert_eq!(minimo.steps, 20);
        assert!((minimo.cfg - 7.0).abs() < f64::EPSILON);

        assert_eq!(defaults_for(Profile::Basico).steps, 20);
        assert_eq!(defaults_for(Profile::Intermediario).steps, 25);
        assert_eq!(defaults_for(Profile::Alto).width, 512);
    }
}
