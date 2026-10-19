use serde::{Deserialize, Serialize};
use voxel_hardware::{
    AttemptState, Combination, CompatibilityMemory, ImageBackend, ImageSetup, ModelSuggestion,
    ProbeVerdict, Profile, SETUP_CONFIGURATION, SETUP_MODEL, hardware_line, image_setup,
    suggestions_for,
};

use crate::db::Database;
use crate::probe::{SdBinaries, run_probe};

/// Um passo mostrado na tela de boas-vindas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStep {
    pub id: String,
    pub state: String,
    pub detail: String,
}

/// Estado do setup para a interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupSnapshot {
    pub completed: bool,
    pub profile: Option<String>,
    pub backend: Option<String>,
    pub steps: Vec<SetupStep>,
    pub catalog: Vec<ProfileCatalog>,
}

/// Modelos sugeridos para um perfil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCatalog {
    pub profile: String,
    pub models: Vec<ModelSuggestion>,
}

/// Escolha confirmada na boas-vindas. Perfil e backend vazios mantêm o que o teste escolheu.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupChoices {
    pub models: Vec<String>,
    pub profile: Option<String>,
    pub backend: Option<String>,
}

/// Resultado da geração de teste que encerra o setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupConfirmation {
    pub completed: bool,
    /// PNG em base64, presente quando a geração passou.
    pub image: Option<String>,
    pub outcome: String,
    pub detail: String,
}

/// Lê se o setup já foi concluído.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
pub fn status(database: &Database) -> Result<SetupSnapshot, String> {
    Ok(SetupSnapshot {
        completed: database
            .setup_flag("completed")
            .map_err(|error| error.to_string())?
            .as_deref()
            == Some("1"),
        profile: database
            .setup_flag("profile")
            .map_err(|error| error.to_string())?,
        backend: database
            .setup_flag("backend")
            .map_err(|error| error.to_string())?,
        steps: Vec::new(),
        catalog: catalog(),
    })
}

/// Detecta o hardware, testa os backends em processo isolado e grava o que quebrou.
///
/// # Errors
///
/// Falha se o banco não puder ser lido ou gravado.
pub fn execute(
    database: &Database,
    paths: &SdBinaries,
    mut emit: impl FnMut(&SetupStep),
) -> Result<SetupSnapshot, String> {
    let machine = voxel_hardware::detect();
    let line = hardware_line(&machine);
    emit(&SetupStep {
        id: "hardware".to_owned(),
        state: "done".to_owned(),
        detail: line.clone(),
    });

    let mut memory = CompatibilityMemory::default();
    for row in database
        .compatibility_failures()
        .map_err(|error| error.to_string())?
    {
        memory.record_failure(
            Combination {
                model: row.model,
                backend: row.backend,
                configuration: row.configuration,
            },
            row.detail,
        );
    }

    let work_root = std::env::temp_dir().join("voxel-setup");
    let model = paths.model.clone();
    let outcome = image_setup(
        &machine,
        &mut memory,
        SETUP_MODEL,
        SETUP_CONFIGURATION,
        |backend| {
            emit(&SetupStep {
                id: backend.id().to_owned(),
                state: "running".to_owned(),
                detail: String::new(),
            });
            let verdict = match (model.as_deref(), paths.for_backend(backend)) {
                (Some(model_path), Some(binary)) => {
                    run_probe(binary, model_path, backend, &work_root.join(backend.id()))
                }
                _ => ProbeVerdict::Unavailable,
            };
            if let ProbeVerdict::Failed(detail) = &verdict {
                let _ =
                    database.record_failure(SETUP_MODEL, backend.id(), SETUP_CONFIGURATION, detail);
            }
            emit(&step_for(backend, &verdict));
            verdict
        },
    );

    if let Some(backend) = outcome.chosen {
        database
            .set_setup_flag("backend", backend.id())
            .map_err(|error| error.to_string())?;
        database
            .set_setup_flag("recommended", backend.id())
            .map_err(|error| error.to_string())?;
    }
    database
        .set_setup_flag("profile", outcome.profile.id())
        .map_err(|error| error.to_string())?;
    Ok(snapshot(false, &line, &outcome))
}

/// Grava a escolha do usuário e gera uma imagem pequena no backend confirmado.
///
/// # Errors
///
/// Falha se o banco não puder ser gravado ou se o perfil e o backend não existirem.
pub fn confirm(
    database: &Database,
    paths: &SdBinaries,
    choices: &SetupChoices,
) -> Result<SetupConfirmation, String> {
    let profile = chosen_profile(database, choices)?;
    let backend = chosen_backend(database, choices)?;
    let models = accepted_models(profile, &choices.models);
    database
        .set_setup_flag("profile", profile.id())
        .map_err(|error| error.to_string())?;
    database
        .set_setup_flag("models", &models)
        .map_err(|error| error.to_string())?;
    let Some(backend) = backend else {
        return Ok(finished(false, None, "no_backend", String::new()));
    };
    database
        .set_setup_flag("backend", backend.id())
        .map_err(|error| error.to_string())?;
    let (Some(binary), Some(model)) = (paths.for_backend(backend), paths.model.as_deref()) else {
        return Ok(finished(false, None, "missing", String::new()));
    };
    let work = std::env::temp_dir()
        .join("voxel-setup-final")
        .join(backend.id());
    match run_probe(binary, model, backend, &work) {
        ProbeVerdict::Passed => {
            let bytes =
                std::fs::read(work.join("output.png")).map_err(|error| error.to_string())?;
            database
                .set_setup_flag("completed", "1")
                .map_err(|error| error.to_string())?;
            Ok(finished(
                true,
                Some(encode_base64(&bytes)),
                "passed",
                String::new(),
            ))
        }
        ProbeVerdict::Failed(detail) => {
            database
                .record_failure(SETUP_MODEL, backend.id(), SETUP_CONFIGURATION, &detail)
                .map_err(|error| error.to_string())?;
            Ok(finished(false, None, "failed", detail))
        }
        ProbeVerdict::Unavailable => Ok(finished(false, None, "missing", String::new())),
    }
}

fn snapshot(completed: bool, hardware: &str, outcome: &ImageSetup) -> SetupSnapshot {
    let mut steps = vec![SetupStep {
        id: "hardware".to_owned(),
        state: "done".to_owned(),
        detail: hardware.to_owned(),
    }];
    for attempt in &outcome.attempts {
        let (state, detail) = match &attempt.state {
            AttemptState::Unavailable => ("unavailable", String::new()),
            AttemptState::Passed => ("passed", String::new()),
            AttemptState::Failed(detail) => ("failed", detail.clone()),
            AttemptState::Skipped => ("skipped", String::new()),
            AttemptState::NotNeeded => ("not_needed", String::new()),
        };
        steps.push(SetupStep {
            id: attempt.backend.id().to_owned(),
            state: state.to_owned(),
            detail,
        });
    }
    steps.push(SetupStep {
        id: "profile".to_owned(),
        state: "done".to_owned(),
        detail: outcome.profile.id().to_owned(),
    });
    SetupSnapshot {
        completed,
        profile: Some(outcome.profile.id().to_owned()),
        backend: outcome.chosen.map(|backend| backend.id().to_owned()),
        steps,
        catalog: catalog(),
    }
}

fn catalog() -> Vec<ProfileCatalog> {
    [
        Profile::Minimo,
        Profile::Basico,
        Profile::Intermediario,
        Profile::Avancado,
        Profile::Alto,
    ]
    .into_iter()
    .map(|profile| ProfileCatalog {
        profile: profile.id().to_owned(),
        models: suggestions_for(profile).to_vec(),
    })
    .collect()
}

fn chosen_profile(database: &Database, choices: &SetupChoices) -> Result<Profile, String> {
    let stored = database
        .setup_flag("profile")
        .map_err(|error| error.to_string())?;
    let id = choices
        .profile
        .as_deref()
        .or(stored.as_deref())
        .ok_or("perfil desconhecido")?;
    Profile::from_id(id).ok_or_else(|| "perfil desconhecido".to_owned())
}

fn chosen_backend(
    database: &Database,
    choices: &SetupChoices,
) -> Result<Option<ImageBackend>, String> {
    let stored = database
        .setup_flag("backend")
        .map_err(|error| error.to_string())?;
    let Some(id) = choices.backend.as_deref().or(stored.as_deref()) else {
        return Ok(None);
    };
    ImageBackend::from_id(id)
        .map(Some)
        .ok_or_else(|| "backend desconhecido".to_owned())
}

fn accepted_models(profile: Profile, requested: &[String]) -> String {
    suggestions_for(profile)
        .iter()
        .filter(|model| requested.iter().any(|id| id == model.id))
        .map(|model| model.id)
        .collect::<Vec<_>>()
        .join(",")
}

fn finished(
    completed: bool,
    image: Option<String>,
    outcome: &str,
    detail: String,
) -> SetupConfirmation {
    SetupConfirmation {
        completed,
        image,
        outcome: outcome.to_owned(),
        detail,
    }
}

pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let chunk = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        push_chunk(&mut out, TABLE, chunk, 4);
        index += 3;
    }
    let rest = &bytes[index..];
    if rest.len() == 1 {
        push_chunk(&mut out, TABLE, u32::from(rest[0]) << 16, 2);
        out.push_str("==");
    } else if rest.len() == 2 {
        let chunk = (u32::from(rest[0]) << 16) | (u32::from(rest[1]) << 8);
        push_chunk(&mut out, TABLE, chunk, 3);
        out.push('=');
    }
    out
}

fn push_chunk(out: &mut String, table: &[u8; 64], chunk: u32, chars: usize) {
    for shift in [18, 12, 6, 0].into_iter().take(chars) {
        let index = usize::try_from((chunk >> shift) & 63).unwrap_or(0);
        out.push(char::from(table[index]));
    }
}

fn step_for(backend: ImageBackend, verdict: &ProbeVerdict) -> SetupStep {
    let (state, detail) = match verdict {
        ProbeVerdict::Unavailable => ("unavailable", String::new()),
        ProbeVerdict::Passed => ("passed", String::new()),
        ProbeVerdict::Failed(detail) => ("failed", detail.clone()),
    };
    SetupStep {
        id: backend.id().to_owned(),
        state: state.to_owned(),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_binario_conclui_e_nao_escolhe_backend() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let mut seen = Vec::new();
        let snapshot = execute(&database, &SdBinaries::default(), |step| {
            seen.push(step.id.clone());
        })
        .unwrap();
        assert!(!snapshot.completed);
        assert!(snapshot.backend.is_none());
        assert!(snapshot.profile.is_some());
        assert_eq!(snapshot.catalog.len(), 5);
        assert!(snapshot.steps.iter().any(|step| step.id == "profile"));
        assert!(seen.iter().any(|id| id == "hardware"));
        assert_eq!(database.setup_flag("completed").unwrap().as_deref(), None);
        assert!(!status(&database).unwrap().completed);
    }

    #[test]
    fn reset_permite_rodar_de_novo() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        execute(&database, &SdBinaries::default(), |_| {}).unwrap();
        database.reset_setup().unwrap();
        assert!(!status(&database).unwrap().completed);
    }

    #[test]
    fn a_confirmacao_guarda_a_escolha_e_exige_a_imagem() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        execute(&database, &SdBinaries::default(), |_| {}).unwrap();
        let missing = confirm(
            &database,
            &SdBinaries::default(),
            &SetupChoices {
                models: vec!["sd15".to_owned(), "inventado".to_owned(), "sf3d".to_owned()],
                profile: Some("minimo".to_owned()),
                backend: Some("cpu".to_owned()),
            },
        )
        .unwrap();
        assert!(!missing.completed);
        assert_eq!(missing.outcome, "missing");
        assert_eq!(
            database.setup_flag("models").unwrap().as_deref(),
            Some("sd15")
        );
        assert_eq!(
            database.setup_flag("profile").unwrap().as_deref(),
            Some("minimo")
        );

        let dir = std::env::temp_dir().join(format!("voxel-confirm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = script(&dir);
        let model = dir.join("modelo");
        std::fs::write(&model, "pesos").unwrap();
        let passed = confirm(
            &database,
            &SdBinaries {
                cpu: Some(binary),
                model: Some(model),
                ..SdBinaries::default()
            },
            &SetupChoices {
                models: vec!["sd15".to_owned(), "triposr".to_owned()],
                profile: None,
                backend: None,
            },
        )
        .unwrap();
        assert!(passed.completed);
        assert_eq!(passed.outcome, "passed");
        assert!(passed.image.unwrap().starts_with("iVBORw0KGgo"));
        assert!(status(&database).unwrap().completed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn perfil_desconhecido_nao_grava() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let error = confirm(
            &database,
            &SdBinaries::default(),
            &SetupChoices {
                models: Vec::new(),
                profile: Some("lunar".to_owned()),
                backend: None,
            },
        );
        assert!(error.is_err());
    }

    #[test]
    fn sem_backend_ou_com_falha_o_setup_continua_aberto() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database.set_setup_flag("profile", "alto").unwrap();
        let open = confirm(
            &database,
            &SdBinaries::default(),
            &SetupChoices {
                models: Vec::new(),
                profile: None,
                backend: None,
            },
        )
        .unwrap();
        assert_eq!(open.outcome, "no_backend");
        assert!(
            confirm(
                &database,
                &SdBinaries::default(),
                &SetupChoices {
                    models: Vec::new(),
                    profile: None,
                    backend: Some("metal".to_owned()),
                },
            )
            .is_err()
        );

        let dir = std::env::temp_dir().join(format!("voxel-confirm-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let binary = failing_script(&dir);
        let model = dir.join("modelo");
        std::fs::write(&model, "pesos").unwrap();
        let failed = confirm(
            &database,
            &SdBinaries {
                vulkan: Some(binary),
                model: Some(model),
                ..SdBinaries::default()
            },
            &SetupChoices {
                models: vec!["u2net".to_owned()],
                profile: Some("alto".to_owned()),
                backend: Some("vulkan".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(failed.outcome, "failed");
        assert!(failed.detail.contains("quebrou"));
        assert!(!status(&database).unwrap().completed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "gera a imagem de teste do setup com o sd-cli"]
    fn a_imagem_final_sai_no_backend_escolhido() {
        let bins = crate::probe::discover();
        assert!(bins.model.is_some(), "modelo sd15");
        assert!(bins.vulkan.is_some(), "sd-cli vulkan");
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let result = confirm(
            &database,
            &bins,
            &SetupChoices {
                models: vec![
                    "sd15".to_owned(),
                    "triposr".to_owned(),
                    "u2net".to_owned(),
                    "sf3d".to_owned(),
                ],
                profile: Some("intermediario".to_owned()),
                backend: Some("vulkan".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(result.outcome, "passed", "{}", result.detail);
        assert!(result.image.unwrap().starts_with("iVBORw0KGgo"));
        assert!(status(&database).unwrap().completed);
        assert_eq!(
            database.setup_flag("models").unwrap().as_deref(),
            Some("sd15,triposr,u2net,sf3d")
        );
    }

    fn failing_script(dir: &std::path::Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("sd-cli");
        std::fs::write(
            &path,
            "#!/bin/sh\nif [ \"$1\" = \"--list-devices\" ]; then printf 'Vulkan0\\tGPU\\n'; exit 0; fi\necho quebrou >&2\nexit 134\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn script(dir: &std::path::Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("sd-cli");
        std::fs::write(
            &path,
            r#"#!/bin/sh
if [ "$1" = "--list-devices" ]; then
  printf 'CPU\tCPU\n'
  exit 0
fi
prev=""
for arg in "$@"; do
  if [ "$prev" = "-o" ]; then
    printf '\x89PNG\r\n\x1a\n' > "$arg"
    dd if=/dev/zero bs=1100 count=1 >> "$arg" 2>/dev/null
  fi
  prev="$arg"
done
exit 0
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }
}
