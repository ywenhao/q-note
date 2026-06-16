import type { Note } from "../types";

export function sortNotes(notes: Note[]) {
  return [...notes].sort((a, b) => {
    if (a.pinned !== b.pinned) {
      return Number(b.pinned) - Number(a.pinned);
    }

    if (a.sortOrder !== b.sortOrder) {
      return a.sortOrder - b.sortOrder;
    }

    return b.updatedAt - a.updatedAt;
  });
}

export function getTopSortOrder(notes: Note[], pinned: boolean) {
  const group = notes.filter((note) => note.pinned === pinned);
  if (group.length === 0) {
    return 0;
  }

  return Math.min(...group.map((note) => note.sortOrder)) - 1;
}

export function normalizeManualOrder(notes: Note[]) {
  return [...notes.filter((note) => note.pinned), ...notes.filter((note) => !note.pinned)].map(
    (note, index) => ({
      ...note,
      sortOrder: index,
    }),
  );
}
