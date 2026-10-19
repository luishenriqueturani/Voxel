use serde::Deserialize;

const BUNDLED: &str = include_str!("../../../catalog/models.json");

/// Catálogo publicado com o aplicativo.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Catalog {
    pub version: u64,
    pub models: Vec<Model>,
}

/// Um arquivo de pesos que o Voxel sabe baixar.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub license: String,
    pub gated: bool,
    #[serde(default)]
    pub requires_cuda: bool,
    pub source: Source,
    pub backends: Vec<BackendNeed>,
}

/// Papel do modelo no fluxo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Image,
    Mesh,
    Background,
}

/// De onde o arquivo é baixado.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "host", rename_all = "snake_case")]
pub enum Source {
    Huggingface { repo: String, path: String },
    Url { url: String },
}

/// Backend em que o modelo já foi medido, com o piso de memória.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendNeed {
    pub id: BackendId,
    pub min_vram_bytes: Option<u64>,
    pub min_ram_bytes: Option<u64>,
    /// Nesta VRAM, ou abaixo, o VAE fica na RAM.
    pub offload_at_or_below_bytes: Option<u64>,
    /// Menor VRAM em que a pesquisa viu o modelo concluir.
    pub proven_vram_bytes: Option<u64>,
    pub caveat: Option<ListedCaveat>,
    #[serde(default)]
    pub primary: bool,
}

/// Backend de execução citado pelo catálogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendId {
    Vulkan,
    Rocm,
    Cuda,
    Cpu,
}

/// Limitação já conhecida quando o backend cabe na memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListedCaveat {
    ExperimentalAttention,
}

impl Catalog {
    /// Lê o `catalog/models.json` que acompanha o binário.
    ///
    /// # Errors
    ///
    /// Falha se o arquivo publicado deixar de bater com a versão suportada.
    pub fn bundled() -> Result<Self, ModelError> {
        Self::parse(BUNDLED)
    }

    /// Interpreta um catálogo e recusa versão ou hash fora do contrato.
    ///
    /// # Errors
    ///
    /// Falha se o JSON for inválido, a versão não for 1 ou um modelo estiver incompleto.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        let catalog: Self = serde_json::from_str(text)?;
        catalog.validate()?;
        Ok(catalog)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.id == id)
    }

    fn validate(&self) -> Result<(), ModelError> {
        if self.version != 1 {
            return Err(ModelError::Version(self.version));
        }
        let mut seen = Vec::new();
        for model in &self.models {
            if seen.contains(&model.id) {
                let id = &model.id;
                return Err(ModelError::Invalid(format!("id repetido: {id}")));
            }
            seen.push(model.id.clone());
            if model.backends.is_empty()
                || model.bytes == 0
                || model.sha256.len() != 64
                || !model.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(ModelError::Invalid(model.id.clone()));
            }
        }
        Ok(())
    }
}

impl Source {
    #[must_use]
    pub fn url(&self) -> String {
        match self {
            Self::Huggingface { repo, path } => {
                format!("https://huggingface.co/{repo}/resolve/main/{path}")
            }
            Self::Url { url } => url.clone(),
        }
    }
}

/// Falha ao ler o catálogo ou ao baixar um arquivo.
#[derive(Debug)]
pub enum ModelError {
    Json(serde_json::Error),
    Version(u64),
    Invalid(String),
    TokenRequired,
    Io(std::io::Error),
    Network(String),
    Status(u16),
    Incomplete { received: u64, expected: u64 },
    HashMismatch,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "json: {error}"),
            Self::Version(version) => {
                write!(formatter, "versão de catálogo não suportada: {version}")
            }
            Self::Invalid(detail) => write!(formatter, "catálogo inválido: {detail}"),
            Self::TokenRequired => write!(formatter, "token ausente"),
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::Network(detail) => write!(formatter, "rede: {detail}"),
            Self::Status(status) => write!(formatter, "download recusado ({status})"),
            Self::Incomplete { received, expected } => {
                write!(formatter, "download incompleto: {received} de {expected}")
            }
            Self::HashMismatch => write!(formatter, "hash não confere"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<serde_json::Error> for ModelError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::io::Error> for ModelError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::Catalog;

    #[test]
    fn o_catalogo_publicado_tem_os_quatro_modelos_medidos() {
        let catalog = Catalog::bundled().unwrap();
        assert_eq!(catalog.version, 1);
        assert_eq!(catalog.models.len(), 4);
        let sd15 = catalog.get("sd15").unwrap();
        assert_eq!(sd15.bytes, 4_265_146_304);
        assert_eq!(
            sd15.sha256,
            "6ce0161689b3853acaa03779ec93eafe75a02f4ced659bee03f50797806fa2fa"
        );
        assert!(!sd15.gated);
        assert!(catalog.get("sf3d").unwrap().gated);
        assert_eq!(catalog.get("u2net").unwrap().bytes, 175_997_641);
        assert_eq!(catalog.get("triposr").unwrap().bytes, 1_677_246_742);
    }

    #[test]
    fn recusa_versao_desconhecida_e_hash_curto() {
        assert!(Catalog::parse(r#"{"version":2,"models":[]}"#).is_err());
        let broken = r#"{
            "version": 1,
            "models": [{
                "id": "x", "name": "x", "kind": "image", "filename": "x",
                "bytes": 1, "sha256": "abcd", "license": "x", "gated": false,
                "source": {"host": "url", "url": "http://127.0.0.1/x"},
                "backends": [{"id": "cpu", "primary": true}]
            }]
        }"#;
        assert!(Catalog::parse(broken).is_err());
    }
}
