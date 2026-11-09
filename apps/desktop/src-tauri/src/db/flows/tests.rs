use super::{Database, DbError, FlowDraft, FlowItem, FlowOverrides, MeshMode};

fn aldeia() -> FlowDraft {
    FlowDraft {
        name: "NPCs da vila".to_owned(),
        template_id: None,
        path: None,
        items: vec![
            FlowItem {
                id: "i1".to_owned(),
                prompt: "ferreiro".to_owned(),
                count: 1,
                mesh: MeshMode::All,
                template_id: Some("tpl-1".to_owned()),
                overrides: FlowOverrides::default(),
            },
            FlowItem {
                id: "i2".to_owned(),
                prompt: "guarda".to_owned(),
                count: 4,
                mesh: MeshMode::Later,
                template_id: Some("tpl-2".to_owned()),
                overrides: FlowOverrides {
                    seed: Some("1234".to_owned()),
                    ..FlowOverrides::default()
                },
            },
        ],
    }
}

#[test]
fn cria_reordena_sobrescreve_e_exclui() {
    let database = Database::open_in_memory().unwrap();
    database.migrate().unwrap();
    let created = database.create_flow(&aldeia()).unwrap();
    assert_eq!(created.items.len(), 2);
    assert_eq!(created.items[1].mesh, MeshMode::Later);
    assert_eq!(created.items[1].overrides.seed.as_deref(), Some("1234"));
    assert_eq!(created.items[0].template_id.as_deref(), Some("tpl-1"));
    assert_eq!(created.items[1].template_id.as_deref(), Some("tpl-2"));

    let mut draft = aldeia();
    draft.items.reverse();
    draft.items[0].mesh = MeshMode::None;
    let updated = database.update_flow(&created.id, &draft).unwrap();
    assert_eq!(updated.items[0].prompt, "guarda");
    assert_eq!(updated.items[0].mesh, MeshMode::None);
    assert_eq!(updated.items[0].template_id.as_deref(), Some("tpl-2"));

    database.delete_flow(&created.id).unwrap();
    assert!(database.list_flows().unwrap().is_empty());
    assert!(matches!(
        database.delete_flow(&created.id),
        Err(DbError::Missing)
    ));
}

#[test]
fn arquivo_json_volta_com_os_mesmos_itens() {
    let database = Database::open_in_memory().unwrap();
    database.migrate().unwrap();
    let created = database.create_flow(&aldeia()).unwrap();
    let dir = std::env::temp_dir().join(format!("voxel-flow-{}", created.id));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vila.json");
    let exported = database.export_flow_file(&created.id, &path).unwrap();
    assert_eq!(exported.path.as_deref(), path.to_str());

    database.delete_flow(&created.id).unwrap();
    let imported = database.import_flow_file(&path).unwrap();
    assert_eq!(imported.name, "NPCs da vila");
    assert_eq!(imported.items[1].count, 4);
    assert_eq!(imported.items[1].overrides.seed.as_deref(), Some("1234"));
    assert_eq!(imported.items[0].template_id.as_deref(), Some("tpl-1"));
    assert_eq!(imported.items[1].template_id.as_deref(), Some("tpl-2"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recusa_quantidade_e_modo_invalidos() {
    let database = Database::open_in_memory().unwrap();
    database.migrate().unwrap();
    let mut draft = aldeia();
    draft.items[0].count = 0;
    assert!(matches!(
        database.create_flow(&draft),
        Err(DbError::Invalid("quantidade inválida"))
    ));
    draft.name = "   ".to_owned();
    draft.items[0].count = 1;
    assert!(matches!(
        database.create_flow(&draft),
        Err(DbError::Invalid("nome vazio"))
    ));
}
