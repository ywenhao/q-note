import { useState, type MouseEvent, type PointerEvent } from "react";
import { AppHeader } from "../components/AppHeader";
import { AppToolbar } from "../components/AppToolbar";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { ContextMenu, type ContextMenuItem } from "../components/ContextMenu";
import { ImagePreview, type ImagePreviewItem } from "../components/ImagePreview";
import { NoteEditor, type NoteDraft } from "../components/NoteEditor";
import { NoteList } from "../components/NoteList";
import { QMark } from "../components/QMark";
import { SettingsDialog } from "../components/SettingsDialog";
import { StatusBar } from "../components/StatusBar";
import { Toast } from "../components/Toast";
import { UpdateDownloadDialog } from "../components/UpdateDownloadDialog";
import type { MenuState } from "../features/menu/useMenuController";
import type { ToastState } from "../hooks/useToast";
import type { Translation } from "../i18n";
import type { UpdateDownloadProgress, UpdateDownloadResult, UpdateInfo } from "../lib/updater";
import type { Note } from "../types";

interface MainWindowViewProps {
  alwaysOnLabel: string;
  alwaysOnTop: boolean;
  appVersion: string;
  autoStart: boolean;
  checkingUpdate: boolean;
  contextItems: ContextMenuItem[];
  dockButtonLabel: string;
  editorNote: Note | null | undefined;
  hasUpdate: boolean;
  menu: MenuState | null;
  notes: Note[];
  onCancelEditor: () => void;
  onCancelUpdateDownload: () => void;
  onCheckUpdate: () => void;
  onCloseConfirmDeleteAll: () => void;
  onCloseMenu: () => void;
  onCloseSettings: () => void;
  onCloseUpdateDialog: () => void;
  onCloseWindow: () => void;
  onCollapseToDock: () => void;
  onColorChange: (id: string, color: string) => void;
  onConfirmDeleteAll: () => void;
  onCopyNote: (note: Note) => void;
  onDeleteAll: () => void;
  onDeleteNote: (id: string) => void;
  onDragMainWindow: (event: PointerEvent<HTMLElement>) => void;
  onEditNote: (note: Note) => void;
  onExport: () => void;
  onHeightChange: (id: string, textHeight: number) => void;
  onImport: () => void;
  onMinimizeWindow: () => void;
  onNewNote: () => void;
  onOpenCurrentRelease: () => void;
  onOpenMenu: (event: MouseEvent<HTMLElement>, noteId?: string) => void;
  onOpenSettings: () => void;
  onReorderNotes: (draggedId: string, targetId: string, placement: "before" | "after") => void;
  onRevealDownloadedUpdate: (path: string) => void;
  onSaveDraft: (draft: NoteDraft) => void;
  onToggleAlwaysOnTop: () => void;
  onToggleAutoStart: () => void;
  onToggleLanguage: () => void;
  onToggleNotePin: (id: string) => void;
  ready: boolean;
  showDeleteAllConfirm: boolean;
  showSettings: boolean;
  t: Translation;
  toast: ToastState | null;
  updateDialogOpen: boolean;
  updateDownloadProgress: UpdateDownloadProgress | null;
  updateDownloadResult: UpdateDownloadResult | null;
  updateInfo: UpdateInfo | null;
}

interface ImagePreviewState {
  index: number;
  items: ImagePreviewItem[];
}

export function MainWindowView({
  alwaysOnLabel,
  alwaysOnTop,
  appVersion,
  autoStart,
  checkingUpdate,
  contextItems,
  dockButtonLabel,
  editorNote,
  hasUpdate,
  menu,
  notes,
  onCancelEditor,
  onCancelUpdateDownload,
  onCheckUpdate,
  onCloseConfirmDeleteAll,
  onCloseMenu,
  onCloseSettings,
  onCloseUpdateDialog,
  onCloseWindow,
  onCollapseToDock,
  onColorChange,
  onConfirmDeleteAll,
  onCopyNote,
  onDeleteAll,
  onDeleteNote,
  onDragMainWindow,
  onEditNote,
  onExport,
  onHeightChange,
  onImport,
  onMinimizeWindow,
  onNewNote,
  onOpenCurrentRelease,
  onOpenMenu,
  onOpenSettings,
  onReorderNotes,
  onRevealDownloadedUpdate,
  onSaveDraft,
  onToggleAlwaysOnTop,
  onToggleAutoStart,
  onToggleLanguage,
  onToggleNotePin,
  ready,
  showDeleteAllConfirm,
  showSettings,
  t,
  toast,
  updateDialogOpen,
  updateDownloadProgress,
  updateDownloadResult,
  updateInfo,
}: MainWindowViewProps) {
  const [imagePreview, setImagePreview] = useState<ImagePreviewState | null>(null);

  if (!ready) {
    return (
      <main className="app-shell is-loading">
        <QMark className="loading-mark" />
      </main>
    );
  }

  return (
    <main className="app-shell" onClick={onCloseMenu} onContextMenu={onOpenMenu}>
      <AppHeader
        alwaysOnLabel={alwaysOnLabel}
        alwaysOnTop={alwaysOnTop}
        onClose={onCloseWindow}
        onDragStart={onDragMainWindow}
        onMinimize={onMinimizeWindow}
        onToggleAlwaysOnTop={onToggleAlwaysOnTop}
        t={t}
      />

      <AppToolbar
        hasUpdate={hasUpdate}
        notesCount={notes.length}
        onDeleteAll={onDeleteAll}
        onNewNote={onNewNote}
        onOpenSettings={onOpenSettings}
        onToggleLanguage={onToggleLanguage}
        t={t}
        updateVersion={updateInfo?.latestVersion}
      />

      <NoteList
        notes={notes}
        onColorChange={onColorChange}
        onContextMenu={onOpenMenu}
        onCopy={onCopyNote}
        onDelete={onDeleteNote}
        onEdit={onEditNote}
        onHeightChange={onHeightChange}
        onNewNote={onNewNote}
        onPreviewImages={(items, index) => setImagePreview({ index, items })}
        onReorder={onReorderNotes}
        onTogglePin={onToggleNotePin}
        t={t}
      />

      {editorNote !== undefined ? (
        <NoteEditor note={editorNote} onCancel={onCancelEditor} onSave={onSaveDraft} t={t} />
      ) : null}
      {showSettings ? (
        <SettingsDialog
          appVersion={appVersion}
          autoStart={autoStart}
          checkingUpdate={checkingUpdate}
          hasUpdate={hasUpdate}
          onCheckUpdate={onCheckUpdate}
          onClose={onCloseSettings}
          onExport={onExport}
          onImport={onImport}
          onOpenCurrentRelease={onOpenCurrentRelease}
          onToggleAutoStart={onToggleAutoStart}
          t={t}
        />
      ) : null}
      {updateDialogOpen && updateInfo ? (
        <UpdateDownloadDialog
          onCancel={onCancelUpdateDownload}
          onClose={onCloseUpdateDialog}
          onReveal={onRevealDownloadedUpdate}
          progress={updateDownloadProgress}
          result={updateDownloadResult}
          t={t}
          update={updateInfo}
        />
      ) : null}

      {menu ? (
        <ContextMenu items={contextItems} onClose={onCloseMenu} x={menu.x} y={menu.y} />
      ) : null}
      {showDeleteAllConfirm ? (
        <ConfirmDialog
          body={t.deleteAllBody}
          cancelLabel={t.cancel}
          confirmLabel={t.deleteAll}
          onCancel={onCloseConfirmDeleteAll}
          onConfirm={onConfirmDeleteAll}
          title={t.confirmDeleteAll}
        />
      ) : null}
      <button
        aria-label={dockButtonLabel}
        className="panel-dock-button"
        onClick={(event) => {
          event.stopPropagation();
          onCollapseToDock();
        }}
        title={dockButtonLabel}
        type="button"
      >
        <QMark />
      </button>
      <StatusBar notes={notes} t={t} />
      <Toast icon={toast?.icon} kind={toast?.kind} message={toast?.message ?? null} />
      {imagePreview ? (
        <ImagePreview
          initialIndex={imagePreview.index}
          items={imagePreview.items}
          onClose={() => setImagePreview(null)}
          t={t}
        />
      ) : null}
    </main>
  );
}
