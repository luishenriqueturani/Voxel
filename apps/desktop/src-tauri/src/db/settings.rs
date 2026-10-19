use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::{Database, DbError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "pt-BR")]
    Portuguese,
    #[serde(rename = "en")]
    English,
}

impl Language {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Portuguese => "pt-BR",
            Self::English => "en",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pt-BR" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
}

impl Database {
    /// Lê as configurações gravadas. Valor ausente ou desconhecido vira `None`.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn settings(&self) -> Result<UserSettings, DbError> {
        Ok(UserSettings {
            language: self.get("language")?.as_deref().and_then(Language::parse),
            theme: self.get("theme")?.as_deref().and_then(Theme::parse),
        })
    }

    /// Grava só os campos presentes e devolve o estado resultante.
    ///
    /// # Errors
    ///
    /// Falha se a escrita ou a leitura seguinte falhar.
    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<UserSettings, DbError> {
        if let Some(language) = update.language {
            self.set("language", language.as_str())?;
        }
        if let Some(theme) = update.theme {
            self.set("theme", theme.as_str())?;
        }
        self.settings()
    }

    /// Backend de imagem escolhido nas configurações, se o usuário tiver trocado.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn image_backend_override(&self) -> Result<Option<String>, DbError> {
        self.get("image_backend")
    }

    /// Grava ou apaga a escolha manual do backend de imagem.
    ///
    /// # Errors
    ///
    /// Falha se a escrita falhar.
    pub fn set_image_backend_override(&self, backend: Option<&str>) -> Result<(), DbError> {
        if let Some(value) = backend {
            self.set("image_backend", value)
        } else {
            let connection = self.lock()?;
            connection.execute("DELETE FROM settings WHERE key = ?1", ["image_backend"])?;
            Ok(())
        }
    }

    /// Informa se há um token do Hugging Face gravado, sem devolver o valor.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn hf_token_configured(&self) -> Result<bool, DbError> {
        Ok(self
            .hf_token()?
            .is_some_and(|value| !value.trim().is_empty()))
    }

    /// Lê o token só para o processo que vai baixar um modelo restrito.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub(crate) fn hf_token(&self) -> Result<Option<String>, DbError> {
        self.get("hf_token")
    }

    /// Grava ou apaga o token do Hugging Face.
    ///
    /// # Errors
    ///
    /// Falha se a escrita falhar.
    pub fn set_hf_token(&self, token: Option<&str>) -> Result<(), DbError> {
        if let Some(value) = token.map(str::trim).filter(|value| !value.is_empty()) {
            self.set("hf_token", value)
        } else {
            let connection = self.lock()?;
            connection.execute("DELETE FROM settings WHERE key = ?1", ["hf_token"])?;
            Ok(())
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, value),
        )?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        Ok(connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, SettingsUpdate, Theme, UserSettings};
    use crate::db::migrated;

    #[test]
    fn migra_uma_vez_e_preserva_os_dados_na_segunda() {
        let database = migrated();
        database
            .update_settings(&SettingsUpdate {
                language: Some(Language::English),
                theme: None,
            })
            .unwrap();

        database.migrate().unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: Some(Language::English),
                theme: None
            }
        );
    }

    #[test]
    fn configuracoes_novas_sao_ausentes() {
        let settings = migrated().settings().unwrap();
        assert_eq!(
            settings,
            UserSettings {
                language: None,
                theme: None
            }
        );
    }

    #[test]
    fn grava_e_rele_idioma_e_tema() {
        let database = migrated();
        let saved = database
            .update_settings(&SettingsUpdate {
                language: Some(Language::Portuguese),
                theme: Some(Theme::Dark),
            })
            .unwrap();

        assert_eq!(saved.language, Some(Language::Portuguese));
        assert_eq!(saved.theme, Some(Theme::Dark));
        assert_eq!(database.settings().unwrap(), saved);
    }

    #[test]
    fn atualizacao_parcial_nao_apaga_o_outro_campo() {
        let database = migrated();
        database
            .update_settings(&SettingsUpdate {
                language: Some(Language::English),
                theme: None,
            })
            .unwrap();
        database
            .update_settings(&SettingsUpdate {
                language: None,
                theme: Some(Theme::Light),
            })
            .unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: Some(Language::English),
                theme: Some(Theme::Light)
            }
        );
    }

    #[test]
    fn valor_desconhecido_e_ignorado_na_leitura() {
        let database = migrated();
        database.set("language", "klingon").unwrap();
        database.set("theme", "sepia").unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: None,
                theme: None
            }
        );
    }

    #[test]
    fn arquivo_reabre_com_os_mesmos_dados() {
        let directory = std::env::temp_dir().join(format!("voxel-db-{}", std::process::id()));
        let path = directory.join("voxel.db");
        let _ = std::fs::remove_dir_all(&directory);

        let database = crate::db::Database::open(&path).unwrap();
        database.migrate().unwrap();
        database
            .update_settings(&SettingsUpdate {
                language: None,
                theme: Some(Theme::Dark),
            })
            .unwrap();
        drop(database);

        let reopened = crate::db::Database::open(&path).unwrap();
        reopened.migrate().unwrap();
        assert_eq!(reopened.settings().unwrap().theme, Some(Theme::Dark));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn parse_de_idioma_e_tema() {
        assert_eq!(Language::parse("pt-BR"), Some(Language::Portuguese));
        assert_eq!(Language::parse("en"), Some(Language::English));
        assert_eq!(Language::parse("es"), None);
        assert_eq!(Language::Portuguese.as_str(), "pt-BR");
        assert_eq!(Theme::parse("dark"), Some(Theme::Dark));
        assert_eq!(Theme::parse("azul"), None);
        assert_eq!(Theme::Light.as_str(), "light");
    }

    #[test]
    fn a_escolha_manual_do_backend_sai_quando_e_apagada() {
        let database = migrated();
        assert_eq!(database.image_backend_override().unwrap(), None);
        database.set_image_backend_override(Some("cpu")).unwrap();
        assert_eq!(
            database.image_backend_override().unwrap().as_deref(),
            Some("cpu")
        );
        database.set_image_backend_override(None).unwrap();
        assert_eq!(database.image_backend_override().unwrap(), None);
    }

    #[test]
    fn o_token_fica_no_banco_e_fora_das_configuracoes_exportadas() {
        let database = migrated();
        assert!(!database.hf_token_configured().unwrap());
        database.set_hf_token(Some("hf_teste")).unwrap();
        assert!(database.hf_token_configured().unwrap());
        let exported = serde_json::to_string(&database.settings().unwrap()).unwrap();
        assert!(!exported.contains("hf_teste"));
        database.set_hf_token(Some("  ")).unwrap();
        assert!(!database.hf_token_configured().unwrap());
    }

    #[test]
    fn json_parcial_nao_exige_os_dois_campos() {
        let update: SettingsUpdate = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(update.language, None);
        assert_eq!(update.theme, Some(Theme::Light));
    }
}
