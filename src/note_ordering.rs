use crate::models::Note;

pub fn sort_notes(notes: Vec<Note>) -> Vec<Note> {
    let mut notes = notes;
    notes.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then(a.sort_order.cmp(&b.sort_order))
            .then(b.updated_at.cmp(&a.updated_at))
    });
    notes
}

pub fn get_top_sort_order(notes: &[Note], pinned: bool) -> i64 {
    let group: Vec<_> = notes.iter().filter(|n| n.pinned == pinned).collect();
    if group.is_empty() {
        return 0;
    }
    group.iter().map(|n| n.sort_order).min().unwrap_or(0) - 1
}

pub fn normalize_manual_order(notes: Vec<Note>) -> Vec<Note> {
    let mut pinned: Vec<_> = notes.iter().filter(|n| n.pinned).cloned().collect();
    let mut unpinned: Vec<_> = notes.into_iter().filter(|n| !n.pinned).collect();
    pinned.append(&mut unpinned);
    pinned
        .into_iter()
        .enumerate()
        .map(|(index, mut note)| {
            note.sort_order = index as i64;
            note
        })
        .collect()
}

pub fn reorder_note(
    notes: &[Note],
    dragged_id: &str,
    target_id: &str,
    after: bool,
) -> Option<Vec<Note>> {
    if dragged_id == target_id {
        return None;
    }
    let mut dragged = notes.iter().find(|note| note.id == dragged_id)?.clone();
    let target = notes.iter().find(|note| note.id == target_id)?;
    dragged.pinned = target.pinned;

    let mut reordered: Vec<_> = notes
        .iter()
        .filter(|note| note.id != dragged_id)
        .cloned()
        .collect();
    let target_index = reordered.iter().position(|note| note.id == target_id)?;
    reordered.insert(target_index + usize::from(after), dragged);
    Some(normalize_manual_order(reordered))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, pinned: bool, sort_order: i64) -> Note {
        Note {
            id: id.to_string(),
            content: id.to_string(),
            color: "#fff9db".to_string(),
            pinned,
            sort_order,
            text_height: None,
            attachments: Vec::new(),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn reorder_across_groups_adopts_target_pin_state() {
        let notes = vec![note("a", true, 0), note("b", false, 1), note("c", false, 2)];
        let reordered = reorder_note(&notes, "c", "a", false).unwrap();

        assert_eq!(
            reordered
                .iter()
                .map(|note| (note.id.as_str(), note.pinned, note.sort_order))
                .collect::<Vec<_>>(),
            vec![("c", true, 0), ("a", true, 1), ("b", false, 2)]
        );
    }

    #[test]
    fn reorder_after_target_preserves_manual_order() {
        let notes = vec![note("a", true, 0), note("b", false, 1), note("c", false, 2)];
        let reordered = reorder_note(&notes, "a", "b", true).unwrap();

        assert_eq!(
            reordered
                .iter()
                .map(|note| (note.id.as_str(), note.pinned, note.sort_order))
                .collect::<Vec<_>>(),
            vec![("b", false, 0), ("a", false, 1), ("c", false, 2)]
        );
    }
}
