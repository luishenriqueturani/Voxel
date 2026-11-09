//! Fila de jobs de geração: ordem intercalada, falha isolada e retomada.

/// Menor lado que o gerador aceita no retry de VRAM.
pub const RESOLUTION_MIN: u32 = 256;

/// Modo 3D gravado no item do fluxo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMode {
    None,
    All,
    Later,
}

/// Situação de um item na fila.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Pending,
    Images,
    Mesh,
    Done,
    Failed,
    Cancelled,
}

/// Próximo trabalho de um item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Images,
    Mesh,
}

/// Estado persistido de um item, suficiente para decidir o próximo passo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemProgress {
    pub status: ItemStatus,
    pub mesh: MeshMode,
    pub count: u32,
    pub images: u32,
    pub meshes: u32,
}

impl ItemProgress {
    /// Item recém-enfileirado, ainda sem saídas.
    #[must_use]
    pub fn pending(count: u32, mesh: MeshMode) -> Self {
        Self {
            status: ItemStatus::Pending,
            mesh,
            count: count.max(1),
            images: 0,
            meshes: 0,
        }
    }
}

/// Primeiro passo pendente na ordem intercalada: imagens do item, 3D do item, próximo.
#[must_use]
pub fn next_action(items: &[ItemProgress]) -> Option<(usize, Action)> {
    items
        .iter()
        .enumerate()
        .find_map(|(index, item)| action_for(item).map(|action| (index, action)))
}

fn action_for(item: &ItemProgress) -> Option<Action> {
    match item.status {
        ItemStatus::Failed | ItemStatus::Done | ItemStatus::Cancelled => None,
        ItemStatus::Pending | ItemStatus::Images => {
            if item.images < item.count {
                Some(Action::Images)
            } else if needs_mesh(item) {
                Some(Action::Mesh)
            } else {
                None
            }
        }
        ItemStatus::Mesh => {
            if needs_mesh(item) {
                Some(Action::Mesh)
            } else {
                None
            }
        }
    }
}

fn needs_mesh(item: &ItemProgress) -> bool {
    item.mesh == MeshMode::All && item.images >= item.count && item.meshes < item.images
}

/// Depois de gravar mais imagens. O 3D só começa quando todas as imagens do item existem.
pub fn record_images(item: &mut ItemProgress, added: u32) {
    item.images = item.images.saturating_add(added).min(item.count);
    item.status = if item.images < item.count {
        ItemStatus::Images
    } else if item.mesh == MeshMode::All {
        ItemStatus::Mesh
    } else {
        ItemStatus::Done
    };
}

/// Depois de gerar a malha de mais uma variação.
pub fn record_mesh(item: &mut ItemProgress) {
    item.meshes = item.meshes.saturating_add(1);
    item.status = if item.mesh == MeshMode::All && item.meshes < item.images {
        ItemStatus::Mesh
    } else {
        ItemStatus::Done
    };
}

/// Marca o item como falho e deixa a fila seguir para o próximo.
pub fn record_failure(item: &mut ItemProgress) {
    item.status = ItemStatus::Failed;
}

/// Interrompe o item sem contar como falha do relatório.
pub fn record_cancel(item: &mut ItemProgress) {
    item.status = ItemStatus::Cancelled;
}

/// Itens já encerrados sobre o total, para a barra de status.
#[must_use]
pub fn progress_counts(items: &[ItemProgress]) -> (u32, u32) {
    let total = u32::try_from(items.len()).unwrap_or(0);
    let done = u32::try_from(
        items
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    ItemStatus::Done | ItemStatus::Failed | ItemStatus::Cancelled
                )
            })
            .count(),
    )
    .unwrap_or(0);
    (done, total)
}

/// A fila acabou quando não resta passo e os falhos já tiveram a segunda chance, ou não há falhos.
#[must_use]
pub fn job_finished(items: &[ItemProgress], end_retry: bool) -> bool {
    !items.is_empty()
        && next_action(items).is_none()
        && (end_retry || !items.iter().any(|item| item.status == ItemStatus::Failed))
}

/// Reabre os falhos uma vez, no ponto em que pararam, para a passagem final.
pub fn prepare_end_retry(items: &mut [ItemProgress]) -> bool {
    let mut any = false;
    for item in items {
        if item.status != ItemStatus::Failed {
            continue;
        }
        let next = resume_status(item);
        if next != ItemStatus::Failed {
            item.status = next;
            any = true;
        }
    }
    any
}

fn resume_status(item: &ItemProgress) -> ItemStatus {
    if item.images < item.count {
        if item.images == 0 {
            ItemStatus::Pending
        } else {
            ItemStatus::Images
        }
    } else if needs_mesh(item) {
        ItemStatus::Mesh
    } else {
        ItemStatus::Failed
    }
}

/// Falta de VRAM reconhecida na mensagem do worker.
#[must_use]
pub fn vram_error(message: &str) -> bool {
    let haystack = message.to_lowercase();
    haystack.contains("out of memory") || haystack.contains("outofmemory")
}

/// Interrupção pedida pelo usuário (pausar ou cancelar).
#[must_use]
pub fn cancelled(message: &str) -> bool {
    message.to_lowercase().contains("cancelado")
}

/// Próxima resolução mais leve, ou `None` se já está no mínimo.
#[must_use]
pub fn lighten_resolution(resolution: u32) -> Option<u32> {
    match resolution {
        1024 => Some(768),
        768 => Some(512),
        n if n > RESOLUTION_MIN => Some(RESOLUTION_MIN),
        _ => None,
    }
}

/// Desliga o pós-processamento da malha. `None` se já estava desligado.
#[must_use]
pub fn lighten_mesh(postprocess: bool) -> Option<bool> {
    postprocess.then_some(false)
}

/// O item ainda pode mudar de lugar: não começou imagens nem malha.
#[must_use]
pub fn movable(item: &ItemProgress) -> bool {
    item.status == ItemStatus::Pending && item.images == 0 && item.meshes == 0
}

/// Nova ordem: itens travados ficam no mesmo índice; os pendentes seguem `pending_ids`.
///
/// `pending_ids` são índices atuais dos itens móveis, na ordem nova.
#[must_use]
pub fn reorder(items: &[ItemProgress], pending_ids: &[usize]) -> Option<Vec<usize>> {
    let movable_idx: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| movable(item))
        .map(|(index, _)| index)
        .collect();
    if pending_ids.len() != movable_idx.len() {
        return None;
    }
    let mut seen = std::collections::BTreeSet::new();
    for index in pending_ids {
        if !movable_idx.contains(index) || !seen.insert(*index) {
            return None;
        }
    }
    let mut next_pending = pending_ids.iter();
    let mut order = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if movable(item) {
            order.push(*next_pending.next()?);
        } else {
            order.push(index);
        }
    }
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::{
        Action, ItemProgress, ItemStatus, MeshMode, cancelled, job_finished, lighten_mesh,
        lighten_resolution, next_action, prepare_end_retry, progress_counts, record_cancel,
        record_failure, record_images, record_mesh, reorder, vram_error,
    };

    #[test]
    fn executa_imagens_depois_o_3d_depois_o_proximo_item() {
        let mut items = vec![
            ItemProgress::pending(2, MeshMode::All),
            ItemProgress::pending(1, MeshMode::None),
        ];
        assert_eq!(next_action(&items), Some((0, Action::Images)));
        record_images(&mut items[0], 2);
        assert_eq!(items[0].status, ItemStatus::Mesh);
        assert_eq!(next_action(&items), Some((0, Action::Mesh)));
        record_mesh(&mut items[0]);
        assert_eq!(next_action(&items), Some((0, Action::Mesh)));
        record_mesh(&mut items[0]);
        assert_eq!(items[0].status, ItemStatus::Done);
        assert_eq!(next_action(&items), Some((1, Action::Images)));
        record_images(&mut items[1], 1);
        assert_eq!(items[1].status, ItemStatus::Done);
        assert_eq!(next_action(&items), None);
        assert!(job_finished(&items, false));
    }

    #[test]
    fn falha_isolada_segue_e_reabre_os_falhos_no_fim() {
        let mut items = vec![
            ItemProgress::pending(1, MeshMode::All),
            ItemProgress::pending(1, MeshMode::Later),
        ];
        record_failure(&mut items[0]);
        assert_eq!(items[0].status, ItemStatus::Failed);
        assert_eq!(next_action(&items), Some((1, Action::Images)));
        record_images(&mut items[1], 1);
        assert_eq!(items[1].status, ItemStatus::Done);
        assert!(!job_finished(&items, false));
        assert!(prepare_end_retry(&mut items));
        assert_eq!(items[0].status, ItemStatus::Pending);
        assert_eq!(next_action(&items), Some((0, Action::Images)));
        record_failure(&mut items[0]);
        assert!(job_finished(&items, true));
        assert_eq!(next_action(&items), None);
    }

    #[test]
    fn retoma_as_imagens_que_faltavam_depois_de_um_corte() {
        let mut item = ItemProgress::pending(3, MeshMode::All);
        record_images(&mut item, 1);
        assert_eq!(item.status, ItemStatus::Images);
        assert_eq!(next_action(&[item.clone()]), Some((0, Action::Images)));
        record_images(&mut item, 2);
        assert_eq!(item.status, ItemStatus::Mesh);
        assert_eq!(next_action(&[item]), Some((0, Action::Mesh)));
    }

    #[test]
    fn escolher_depois_encerra_nas_imagens() {
        let mut item = ItemProgress::pending(2, MeshMode::Later);
        record_images(&mut item, 2);
        assert_eq!(item.status, ItemStatus::Done);
        assert_eq!(next_action(&[item]), None);
    }

    #[test]
    fn falta_de_vram_reduz_a_resolucao_e_desliga_o_3d_pesado() {
        assert!(vram_error("CUDA out of memory"));
        assert!(vram_error("torch.cuda.OutOfMemoryError"));
        assert!(!vram_error("modelo ausente"));
        assert_eq!(lighten_resolution(1024), Some(768));
        assert_eq!(lighten_resolution(768), Some(512));
        assert_eq!(lighten_resolution(512), Some(256));
        assert_eq!(lighten_resolution(256), None);
        assert_eq!(lighten_mesh(true), Some(false));
        assert_eq!(lighten_mesh(false), None);
        assert!(cancelled("cancelado"));
        assert!(!cancelled("gpu"));
    }

    #[test]
    fn reordena_so_os_pendentes_e_ignora_cancelados() {
        let mut items = vec![
            ItemProgress::pending(1, MeshMode::None),
            ItemProgress::pending(1, MeshMode::None),
            ItemProgress::pending(1, MeshMode::None),
        ];
        record_images(&mut items[0], 1);
        record_cancel(&mut items[2]);
        items.push(ItemProgress::pending(1, MeshMode::None));
        // índices: 0 done, 1 pending, 2 cancelled, 3 pending
        assert_eq!(reorder(&items, &[3, 1]), Some(vec![0, 3, 2, 1]));
        assert_eq!(reorder(&items, &[1]), None);
        assert_eq!(next_action(&items), Some((1, Action::Images)));
    }

    #[test]
    fn conta_encerrados_para_a_barra_de_status() {
        let mut items = vec![
            ItemProgress::pending(1, MeshMode::None),
            ItemProgress::pending(1, MeshMode::None),
            ItemProgress::pending(1, MeshMode::None),
        ];
        record_images(&mut items[0], 1);
        record_failure(&mut items[1]);
        record_cancel(&mut items[2]);
        assert_eq!(progress_counts(&items), (3, 3));
        items.push(ItemProgress::pending(1, MeshMode::None));
        assert_eq!(progress_counts(&items), (3, 4));
    }
}
