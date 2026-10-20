mod library;

use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use voxel_models::{Catalog, Compatibility, MachineFacts, compatibility, fetch, prepare_download};

pub use library::{
    ListedModel, current_facts, ensure_models_dir, import_model, list_models, remove_model,
};

use crate::db::Database;
use crate::paths::AppPaths;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    model_id: String,
    received: u64,
    total: u64,
}

/// Diz se o token está gravado, sem revelar o valor.
///
/// # Errors
///
/// Falha se a leitura do banco falhar.
pub fn token_configured(database: &Database) -> Result<bool, String> {
    database
        .hf_token_configured()
        .map_err(|error| error.to_string())
}

/// Grava o token ou apaga quando o texto vem vazio.
///
/// # Errors
///
/// Falha se a escrita do banco falhar.
pub fn store_token(database: &Database, token: &str) -> Result<bool, String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        database
            .set_hf_token(None)
            .map_err(|error| error.to_string())?;
        Ok(false)
    } else {
        database
            .set_hf_token(Some(trimmed))
            .map_err(|error| error.to_string())?;
        Ok(true)
    }
}

/// Monta o pedido a partir do catálogo e do token local.
///
/// # Errors
///
/// Falha se o modelo não existe ou se um modelo restrito está sem token.
pub fn job_for(database: &Database, model_id: &str) -> Result<voxel_models::DownloadJob, String> {
    let catalog = Catalog::bundled().map_err(|error| error.to_string())?;
    let model = catalog.get(model_id).ok_or("modelo ausente")?;
    let token = database.hf_token().map_err(|error| error.to_string())?;
    prepare_download(model, token.as_deref()).map_err(|error| error.to_string())
}

/// Recusa baixar um modelo incompatível sem a confirmação explícita.
///
/// # Errors
///
/// Falha se o modelo não existe ou se a confirmação falta.
pub fn allow_download(model_id: &str, facts: &MachineFacts, confirmed: bool) -> Result<(), String> {
    let catalog = Catalog::bundled().map_err(|error| error.to_string())?;
    let model = catalog.get(model_id).ok_or("modelo ausente")?;
    if matches!(compatibility(model, facts), Compatibility::Incompatible(_)) && !confirmed {
        return Err("confirmação ausente".to_owned());
    }
    Ok(())
}

/// Baixa o modelo do catálogo para a pasta de dados e publica o progresso.
///
/// # Errors
///
/// Falha se o modelo, o token, a confirmação ou a rede não permitirem concluir o arquivo.
pub fn download(
    app: &AppHandle,
    database: &Database,
    paths: &AppPaths,
    model_id: &str,
    confirmed: bool,
) -> Result<String, String> {
    allow_download(model_id, &current_facts(), confirmed)?;
    let job = job_for(database, model_id)?;
    let destination = paths.data.join("models").join(file_name(model_id)?);
    let id = model_id.to_owned();
    let mut emitted: Option<Instant> = None;
    fetch(&job, &destination, |received, total| {
        let finished = total > 0 && received >= total;
        let recent = emitted.is_some_and(|instant| instant.elapsed() < Duration::from_millis(200));
        if !finished && recent {
            return;
        }
        emitted = Some(Instant::now());
        let _ = app.emit(
            "model://progress",
            DownloadProgress {
                model_id: id.clone(),
                received,
                total,
            },
        );
    })
    .map_err(|error| error.to_string())?;
    Ok(destination.display().to_string())
}

fn file_name(model_id: &str) -> Result<String, String> {
    Catalog::bundled()
        .map_err(|error| error.to_string())?
        .get(model_id)
        .map(|model| model.filename.clone())
        .ok_or_else(|| "modelo ausente".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{allow_download, job_for};
    use crate::db::Database;
    use voxel_models::MachineFacts;

    #[test]
    fn o_stable_fast_3d_exige_token_e_o_sd15_nao() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        assert_eq!(job_for(&database, "sf3d").unwrap_err(), "token ausente");
        let sd15 = job_for(&database, "sd15").unwrap();
        assert!(sd15.authorization.is_none());
        assert_eq!(
            job_for(&database, "nao-existe").unwrap_err(),
            "modelo ausente"
        );
        database.set_hf_token(Some("hf_teste")).unwrap();
        let restricted = job_for(&database, "sf3d").unwrap();
        assert!(restricted.authorization.is_some());
        assert!(!format!("{restricted:?}").contains("hf_teste"));
    }

    #[test]
    fn sem_confirmacao_o_incompativel_nao_passa() {
        let facts = MachineFacts {
            vram_bytes: 0,
            ram_bytes: 0,
            vendor: None,
            vulkan: false,
            rocm: false,
            cuda: false,
        };
        assert_eq!(
            allow_download("sd15", &facts, false).unwrap_err(),
            "confirmação ausente"
        );
        assert!(allow_download("sd15", &facts, true).is_ok());
    }
}
