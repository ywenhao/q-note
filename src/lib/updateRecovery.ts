import type { PendingUpdateDraft } from "./updateDraft";

type LoadPendingUpdateDraft = () => Promise<PendingUpdateDraft | null>;
type OpenPendingUpdateEditor = (noteId: string | null) => Promise<void>;

export async function restorePendingUpdateEditor(
  loadPendingDraft: LoadPendingUpdateDraft,
  openEditor: OpenPendingUpdateEditor,
) {
  const pendingDraft = await loadPendingDraft();
  if (!pendingDraft) {
    return false;
  }

  await openEditor(pendingDraft.noteId);
  return true;
}
