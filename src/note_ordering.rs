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

#[allow(dead_code)]
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
