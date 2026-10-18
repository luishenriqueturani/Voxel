use std::path::{Path, PathBuf};

const APP_DIR: &str = "voxel";

/// Diretórios da aplicação segundo a especificação XDG Base Directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PathsError {
    HomeNotSet,
}

impl std::fmt::Display for PathsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HomeNotSet => write!(f, "a variável de ambiente HOME não está definida"),
        }
    }
}

impl std::error::Error for PathsError {}

impl AppPaths {
    /// Resolve os diretórios a partir das variáveis de ambiente do processo.
    ///
    /// # Errors
    ///
    /// Retorna [`PathsError::HomeNotSet`] se `HOME` não estiver definido e algum diretório
    /// XDG precisar do valor padrão.
    pub fn from_env() -> Result<Self, PathsError> {
        Self::resolve(|name| std::env::var_os(name).map(PathBuf::from))
    }

    /// Resolve os diretórios usando `lookup` para ler as variáveis de ambiente.
    ///
    /// # Errors
    ///
    /// Retorna [`PathsError::HomeNotSet`] se `HOME` não estiver definido e algum diretório
    /// XDG precisar do valor padrão.
    pub fn resolve(lookup: impl Fn(&str) -> Option<PathBuf>) -> Result<Self, PathsError> {
        let base = |var: &str, default: &str| -> Result<PathBuf, PathsError> {
            // A especificação XDG manda ignorar valores relativos.
            match lookup(var).filter(|path| path.is_absolute()) {
                Some(path) => Ok(path),
                None => home(&lookup).map(|home| home.join(default)),
            }
        };

        Ok(Self {
            config: base("XDG_CONFIG_HOME", ".config")?.join(APP_DIR),
            data: base("XDG_DATA_HOME", ".local/share")?.join(APP_DIR),
            state: base("XDG_STATE_HOME", ".local/state")?.join(APP_DIR),
            cache: base("XDG_CACHE_HOME", ".cache")?.join(APP_DIR),
        })
    }

    /// Todos os diretórios, na ordem config, data, state, cache.
    #[must_use]
    pub fn all(&self) -> [&Path; 4] {
        [&self.config, &self.data, &self.state, &self.cache]
    }

    /// Cria todos os diretórios que ainda não existem.
    ///
    /// # Errors
    ///
    /// Retorna o erro de E/S do primeiro diretório que não puder ser criado.
    pub fn create_all(&self) -> std::io::Result<()> {
        self.all().into_iter().try_for_each(std::fs::create_dir_all)
    }
}

fn home(lookup: &impl Fn(&str) -> Option<PathBuf>) -> Result<PathBuf, PathsError> {
    lookup("HOME")
        .filter(|path| path.is_absolute())
        .ok_or(PathsError::HomeNotSet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<PathBuf> {
        let vars: HashMap<String, PathBuf> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), PathBuf::from(value)))
            .collect();
        move |name| vars.get(name).cloned()
    }

    #[test]
    fn usa_padroes_relativos_a_home() {
        let paths = AppPaths::resolve(env(&[("HOME", "/home/ana")])).unwrap();

        assert_eq!(
            paths,
            AppPaths {
                config: "/home/ana/.config/voxel".into(),
                data: "/home/ana/.local/share/voxel".into(),
                state: "/home/ana/.local/state/voxel".into(),
                cache: "/home/ana/.cache/voxel".into(),
            }
        );
    }

    #[test]
    fn respeita_variaveis_xdg_absolutas() {
        let paths = AppPaths::resolve(env(&[
            ("HOME", "/home/ana"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "/dados"),
            ("XDG_STATE_HOME", "/estado"),
            ("XDG_CACHE_HOME", "/cache"),
        ]))
        .unwrap();

        assert_eq!(paths.config, PathBuf::from("/cfg/voxel"));
        assert_eq!(paths.data, PathBuf::from("/dados/voxel"));
        assert_eq!(paths.state, PathBuf::from("/estado/voxel"));
        assert_eq!(paths.cache, PathBuf::from("/cache/voxel"));
    }

    #[test]
    fn ignora_variaveis_xdg_relativas() {
        let paths = AppPaths::resolve(env(&[
            ("HOME", "/home/ana"),
            ("XDG_CONFIG_HOME", "relativo"),
        ]))
        .unwrap();

        assert_eq!(paths.config, PathBuf::from("/home/ana/.config/voxel"));
    }

    #[test]
    fn nao_precisa_de_home_quando_todas_as_xdg_estao_definidas() {
        let paths = AppPaths::resolve(env(&[
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "/dados"),
            ("XDG_STATE_HOME", "/estado"),
            ("XDG_CACHE_HOME", "/cache"),
        ]));

        assert!(paths.is_ok());
    }

    #[test]
    fn falha_sem_home() {
        assert_eq!(AppPaths::resolve(env(&[])), Err(PathsError::HomeNotSet));
    }

    #[test]
    fn falha_com_home_relativa() {
        assert_eq!(
            AppPaths::resolve(env(&[("HOME", "relativo")])),
            Err(PathsError::HomeNotSet)
        );
    }

    #[test]
    fn mensagem_de_erro_legivel() {
        assert_eq!(
            PathsError::HomeNotSet.to_string(),
            "a variável de ambiente HOME não está definida"
        );
    }

    #[test]
    fn cria_todos_os_diretorios() {
        let root = std::env::temp_dir().join(format!("voxel-paths-{}", std::process::id()));
        let root_str = root.to_str().unwrap().to_owned();
        let paths = AppPaths::resolve(env(&[("HOME", &root_str)])).unwrap();

        paths.create_all().unwrap();

        assert!(paths.all().iter().all(|dir| dir.is_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_env_le_o_ambiente_do_processo() {
        if std::env::var_os("HOME").is_some() {
            assert!(AppPaths::from_env().is_ok());
        }
    }
}
