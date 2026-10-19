//! Espaço livre no sistema de arquivos dos dados do Voxel.

use std::path::{Path, PathBuf};

use sysinfo::Disks;

use crate::report::DiskSpace;

pub struct Mount {
    pub path: PathBuf,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

pub fn data_directory_from(xdg: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = xdg.filter(|value| !value.is_empty()).map_or_else(
        || {
            home.filter(|value| !value.is_empty())
                .map_or_else(|| PathBuf::from("/"), PathBuf::from)
                .join(".local/share")
        },
        PathBuf::from,
    );
    base.join("voxel")
}

pub fn space_on(path: &Path, mounts: &[Mount]) -> DiskSpace {
    let best = mounts
        .iter()
        .filter(|mount| path.starts_with(&mount.path))
        .max_by_key(|mount| mount.path.components().count());
    DiskSpace {
        path: path.to_path_buf(),
        free_bytes: best.map_or(0, |mount| mount.free_bytes),
        total_bytes: best.map_or(0, |mount| mount.total_bytes),
    }
}

pub fn space_for_data_dir() -> DiskSpace {
    let path = data_directory_from(
        std::env::var("XDG_DATA_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    );
    let existing = existing_ancestor(&path);
    let mounts = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| Mount {
            path: disk.mount_point().to_path_buf(),
            free_bytes: disk.available_space(),
            total_bytes: disk.total_space(),
        })
        .collect::<Vec<_>>();
    space_on(existing, &mounts)
}

fn existing_ancestor(path: &Path) -> &Path {
    let mut cursor = path;
    while !cursor.exists() {
        match cursor.parent() {
            Some(parent) if parent != cursor => cursor = parent,
            _ => break,
        }
    }
    cursor
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Mount, data_directory_from, space_on};

    #[test]
    fn xdg_vazio_cai_na_home() {
        assert_eq!(
            data_directory_from(Some(""), Some("/home/ana")),
            PathBuf::from("/home/ana/.local/share/voxel")
        );
        assert_eq!(
            data_directory_from(Some("/dados"), Some("/home/ana")),
            PathBuf::from("/dados/voxel")
        );
        assert_eq!(
            data_directory_from(None, None),
            PathBuf::from("/.local/share/voxel")
        );
    }

    #[test]
    fn escolhe_a_montagem_mais_especifica() {
        let mounts = vec![
            Mount {
                path: PathBuf::from("/"),
                free_bytes: 1,
                total_bytes: 10,
            },
            Mount {
                path: PathBuf::from("/home"),
                free_bytes: 2,
                total_bytes: 20,
            },
            Mount {
                path: PathBuf::from("/home/ana-extra"),
                free_bytes: 9,
                total_bytes: 90,
            },
        ];
        let space = space_on(std::path::Path::new("/home/ana/.local/share"), &mounts);
        assert_eq!(space.free_bytes, 2);
        assert_eq!(space.total_bytes, 20);
    }

    #[test]
    fn sem_montagem_o_espaco_e_zero() {
        let space = space_on(std::path::Path::new("/tmp/voxel"), &[]);
        assert_eq!(space.free_bytes, 0);
        assert_eq!(space.total_bytes, 0);
    }
}
