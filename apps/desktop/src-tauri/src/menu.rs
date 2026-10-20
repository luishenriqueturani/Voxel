use std::collections::HashMap;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Runtime};

const PT_BR: &str = include_str!("../../src/i18n/locales/pt-BR.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuNode {
    Item {
        id: &'static str,
        enabled: bool,
        accelerator: Option<&'static str>,
    },
    Separator,
}

pub struct MenuGroup {
    pub id: &'static str,
    items: Vec<MenuNode>,
}

enum OwnedItem<R: Runtime> {
    Item(MenuItem<R>),
    Separator(PredefinedMenuItem<R>),
}

fn item(id: &'static str, enabled: bool, accelerator: Option<&'static str>) -> MenuNode {
    MenuNode::Item {
        id,
        enabled,
        accelerator,
    }
}

/// Estrutura fixa do menu nativo. Os rótulos vêm das traduções.
#[must_use]
pub fn menu_groups() -> Vec<MenuGroup> {
    vec![
        MenuGroup {
            id: "file",
            items: vec![
                item("file.newFlow", false, None),
                item("file.openFlow", false, None),
                item("file.saveFlow", false, None),
                MenuNode::Separator,
                item("file.openImage", true, None),
                item("file.export", false, None),
                item("file.openOutput", false, None),
                MenuNode::Separator,
                item("file.quit", true, Some("CmdOrCtrl+Q")),
            ],
        },
        MenuGroup {
            id: "edit",
            items: vec![
                item("edit.undo", false, None),
                item("edit.redo", false, None),
                item("edit.copy", false, None),
                item("edit.paste", false, None),
                item("edit.removeBackground", true, None),
                MenuNode::Separator,
                item("edit.preferences", true, None),
            ],
        },
        MenuGroup {
            id: "generate",
            items: vec![
                item("generate.run", true, Some("CmdOrCtrl+Return")),
                item("generate.cancel", false, None),
                item("generate.regenerate", false, None),
            ],
        },
        MenuGroup {
            id: "flows",
            items: vec![
                item("flows.manage", false, None),
                item("flows.templates", false, None),
                item("flows.queue", false, None),
                item("flows.pause", false, None),
            ],
        },
        MenuGroup {
            id: "models",
            items: vec![
                item("models.manage", false, None),
                item("models.import", false, None),
                item("models.openFolder", false, None),
            ],
        },
        MenuGroup {
            id: "view",
            items: vec![
                item("view.gallery", true, None),
                item("view.viewer", true, None),
                item("view.theme", true, None),
                item("view.logs", false, None),
            ],
        },
        MenuGroup {
            id: "help",
            items: vec![
                item("help.docs", false, None),
                item("help.diagnostics", false, None),
                item("help.redoSetup", true, None),
                item("help.openLogs", false, None),
                item("help.about", false, None),
            ],
        },
    ]
}

pub fn embedded_labels(json: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let menu = value
        .get("menu")
        .and_then(serde_json::Value::as_object)
        .ok_or("objeto menu ausente")?;
    Ok(menu
        .iter()
        .map(|(key, text)| (key.clone(), text.as_str().unwrap_or(key).to_owned()))
        .collect())
}

#[must_use]
pub fn default_labels() -> HashMap<String, String> {
    embedded_labels(PT_BR).unwrap_or_default()
}

fn label(labels: &HashMap<String, String>, key: &str) -> String {
    labels.get(key).cloned().unwrap_or_else(|| key.to_owned())
}

pub fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    labels: &HashMap<String, String>,
) -> tauri::Result<Menu<R>> {
    let mut submenus = Vec::new();
    for group in menu_groups() {
        submenus.push(build_group(app, labels, &group)?);
    }
    let refs: Vec<&dyn IsMenuItem<R>> = submenus.iter().map(|submenu| submenu as _).collect();
    Menu::with_items(app, &refs)
}

fn build_group<R: Runtime>(
    app: &AppHandle<R>,
    labels: &HashMap<String, String>,
    group: &MenuGroup,
) -> tauri::Result<Submenu<R>> {
    let mut owned = Vec::new();
    for node in &group.items {
        owned.push(match *node {
            MenuNode::Separator => OwnedItem::Separator(PredefinedMenuItem::separator(app)?),
            MenuNode::Item {
                id,
                enabled,
                accelerator,
            } => OwnedItem::Item(MenuItem::with_id(
                app,
                id,
                label(labels, id),
                enabled,
                accelerator,
            )?),
        });
    }
    let refs: Vec<&dyn IsMenuItem<R>> = owned
        .iter()
        .map(|item| -> &dyn IsMenuItem<R> {
            match item {
                OwnedItem::Item(item) => item,
                OwnedItem::Separator(item) => item,
            }
        })
        .collect();
    Submenu::with_id_and_items(app, group.id, label(labels, group.id), true, &refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = include_str!("../../src/i18n/locales/en.json");

    #[test]
    fn traducoes_cobrem_todos_os_itens() {
        for json in [PT_BR, EN] {
            let labels = embedded_labels(json).expect("json de tradução válido");
            for group in menu_groups() {
                assert!(labels.contains_key(group.id), "falta o menu {}", group.id);
                for node in group.items {
                    if let MenuNode::Item { id, .. } = node {
                        assert!(labels.contains_key(id), "falta o item {id}");
                    }
                }
            }
        }
    }

    #[test]
    fn json_invalido_falha() {
        assert!(embedded_labels("[]").is_err());
    }

    #[test]
    fn rotulos_padrao_sao_portugues() {
        let labels = default_labels();
        assert_eq!(labels.get("file").map(String::as_str), Some("Arquivo"));
        assert_eq!(
            labels.get("generate.run").map(String::as_str),
            Some("Gerar")
        );
    }
}
