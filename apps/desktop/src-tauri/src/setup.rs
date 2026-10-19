use serde::Serialize;
use voxel_hardware::{
    AttemptState, Combination, CompatibilityMemory, ImageBackend, ImageSetup, ProbeVerdict,
    SETUP_CONFIGURATION, SETUP_MODEL, hardware_line, image_setup,
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
    }
    database
        .set_setup_flag("profile", outcome.profile.id())
        .map_err(|error| error.to_string())?;
    database
        .set_setup_flag("completed", "1")
        .map_err(|error| error.to_string())?;
    Ok(snapshot(true, &line, &outcome))
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
        assert!(snapshot.completed);
        assert!(snapshot.backend.is_none());
        assert!(snapshot.profile.is_some());
        assert!(snapshot.steps.iter().any(|step| step.id == "profile"));
        assert!(seen.iter().any(|id| id == "hardware"));
        assert_eq!(
            database.setup_flag("completed").unwrap().as_deref(),
            Some("1")
        );
        assert!(status(&database).unwrap().completed);
    }

    #[test]
    fn reset_permite_rodar_de_novo() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        execute(&database, &SdBinaries::default(), |_| {}).unwrap();
        database.reset_setup().unwrap();
        assert!(!status(&database).unwrap().completed);
    }
}
