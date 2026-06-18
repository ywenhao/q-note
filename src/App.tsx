import { getCurrentWindow } from "@tauri-apps/api/window";
import { useState } from "react";
import "./App.css";
import { DockWindowView } from "./app/DockWindowView";
import { MainWindowView } from "./app/MainWindowView";
import { useMenuController } from "./features/menu/useMenuController";
import { useNotesController, useNotesState } from "./features/notes/useNotesController";
import { useSettingsController, useSettingsState } from "./features/settings/useSettingsController";
import { useWindowController } from "./features/window/useWindowController";
import { useAppBoot } from "./hooks/useAppBoot";
import { useDockMode } from "./hooks/useDockMode";
import { useTauriEventBridge } from "./hooks/useTauriEventBridge";
import { useToast } from "./hooks/useToast";
import { useTrayMenuLabels } from "./hooks/useTrayMenuLabels";
import { useUpdateManager } from "./hooks/useUpdateManager";
import { useWindowStatePersistence } from "./hooks/useWindowStatePersistence";
import { translations } from "./i18n";
import { isTauriRuntime } from "./lib/env";
import { DOCK_WINDOW_LABEL, MAIN_WINDOW_LABEL } from "./lib/windowControls";
import type { Note } from "./types";

function App() {
  const [editorNote, setEditorNote] = useState<Note | null | undefined>(undefined);
  const [ready, setReady] = useState(false);
  const [showDeleteAllConfirm, setShowDeleteAllConfirm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const { showToast, toast } = useToast();
  const { commitNotes, notes, notesRef, setNotes } = useNotesState();
  const { setSettings, settings, settingsRef } = useSettingsState();

  const currentWindowLabel = isTauriRuntime() ? getCurrentWindow().label : MAIN_WINDOW_LABEL;
  const isDockWindow = currentWindowLabel === DOCK_WINDOW_LABEL;
  const t = translations[settings.language];
  const dockToggleLabel = settings.docked ? t.switchMainWindow : t.switchFloatingBall;
  const alwaysOnLabel = settings.alwaysOnTop ? t.alwaysOff : t.alwaysOn;
  const editorOpen = editorNote !== undefined;

  const {
    appVersion,
    checkingUpdate,
    handleCancelUpdateDownload,
    handleCheckUpdate,
    handleOpenCurrentRelease,
    handleRevealDownloadedUpdate,
    setUpdateDialogOpen,
    updateDialogOpen,
    updateDownloadProgress,
    updateDownloadResult,
    updateInfo,
  } = useUpdateManager({
    currentWindowLabel,
    language: settings.language,
    ready,
    showToast,
  });

  const {
    handleExport,
    handleImport,
    persistSettings,
    toggleAlwaysOnTop,
    toggleAutoStart,
    toggleLanguage,
  } = useSettingsController({
    commitNotes,
    notesRef,
    setSettings,
    settingsRef,
    showToast,
    t,
  });

  const {
    collapseToQIcon,
    concealDockIcon,
    dockDragRef,
    dockGuardRef,
    dragQIcon,
    finishQIconDrag,
    moveQIcon,
    openMainFromDockIcon,
    persistIconSnap,
    restoreDock,
    revealDockIcon,
    toggleDockOnEdge,
  } = useDockMode({
    currentWindowLabel,
    persistSettings,
    settingsRef,
  });

  const {
    closeEditor,
    handleCopy,
    handleDelete,
    handleDeleteAll,
    handleSaveDraft,
    openEditor,
    patchNote,
    reorderNotes,
  } = useNotesController({
    commitNotes,
    editorNote,
    notesRef,
    restoreDock,
    setEditorNote,
    settingsRef,
    showToast,
    t,
  });

  const { closeWindow, dragMainWindow, minimizeWindow, quitApp } = useWindowController({
    collapseToQIcon,
  });

  const { closeMenu, getContextItems, getDockMenuItems, menu, openDockMenu, openMenu } =
    useMenuController({
      alwaysOnLabel,
      dockToggleLabel,
      handleCopy,
      handleDelete,
      notesCount: notes.length,
      notesRef,
      onDeleteAll: () => setShowDeleteAllConfirm(true),
      onOpenSettings: () => setShowSettings(true),
      openEditor,
      patchNote,
      quitApp,
      settings,
      t,
      toggleAlwaysOnTop,
      toggleAutoStart,
      toggleDockOnEdge,
      toggleLanguage,
    });

  useAppBoot({
    currentWindowLabel,
    notesRef,
    settingsRef,
    setNotes,
    setReady,
    setSettings,
  });

  useTrayMenuLabels({
    alwaysOnLabel,
    dockToggleLabel,
    ready,
    t,
  });

  useTauriEventBridge({
    commitNotes,
    notesRef,
    ready,
    restoreDock,
    setSettings,
    settingsRef,
    showToast,
    toggleAlwaysOnTop,
    toggleDockOnEdge,
    toggleLanguage,
  });

  useWindowStatePersistence({
    currentWindowLabel,
    dockDragRef,
    dockGuardRef,
    editorOpen,
    persistIconSnap,
    persistSettings,
    ready,
    settingsRef,
  });

  if (isDockWindow) {
    return (
      <DockWindowView
        menu={menu}
        menuItems={getDockMenuItems()}
        onCloseMenu={closeMenu}
        onConcealDockIcon={() => void concealDockIcon()}
        onDockDragEnd={() => void finishQIconDrag()}
        onDockDragMove={() => void moveQIcon()}
        onDockDragStart={() => void dragQIcon()}
        onOpenDockMenu={(event) => void openDockMenu(event)}
        onOpenMain={() => void openMainFromDockIcon()}
        onRevealDockIcon={() => void revealDockIcon()}
        ready={ready}
        t={t}
        toast={toast}
      />
    );
  }

  return (
    <MainWindowView
      alwaysOnLabel={alwaysOnLabel}
      alwaysOnTop={settings.alwaysOnTop}
      appVersion={appVersion}
      autoStart={settings.autoStart}
      checkingUpdate={checkingUpdate}
      contextItems={getContextItems()}
      dockButtonLabel={t.switchFloatingBall}
      editorNote={editorNote}
      hasUpdate={Boolean(updateInfo)}
      menu={menu}
      notes={notes}
      onCancelEditor={() => void closeEditor()}
      onCancelUpdateDownload={() => void handleCancelUpdateDownload()}
      onCheckUpdate={() => void handleCheckUpdate()}
      onCloseConfirmDeleteAll={() => setShowDeleteAllConfirm(false)}
      onCloseMenu={closeMenu}
      onCloseSettings={() => setShowSettings(false)}
      onCloseUpdateDialog={() => setUpdateDialogOpen(false)}
      onCloseWindow={() => void closeWindow()}
      onCollapseToDock={() => void collapseToQIcon({ useRevealAnchor: true })}
      onColorChange={(id, color) => void patchNote(id, { color })}
      onConfirmDeleteAll={() => {
        setShowDeleteAllConfirm(false);
        void handleDeleteAll();
      }}
      onCopyNote={(note) => void handleCopy(note)}
      onDeleteAll={() => setShowDeleteAllConfirm(true)}
      onDeleteNote={(id) => void handleDelete(id)}
      onDragMainWindow={dragMainWindow}
      onEditNote={(note) => void openEditor(note)}
      onExport={() => void handleExport()}
      onHeightChange={(id, textHeight) => void patchNote(id, { textHeight })}
      onImport={() => void handleImport()}
      onMinimizeWindow={() => void minimizeWindow()}
      onNewNote={() => void openEditor(null)}
      onOpenCurrentRelease={() => void handleOpenCurrentRelease()}
      onOpenMenu={openMenu}
      onOpenSettings={() => setShowSettings(true)}
      onReorderNotes={(draggedId, targetId, placement) =>
        void reorderNotes(draggedId, targetId, placement)
      }
      onRevealDownloadedUpdate={(path) => void handleRevealDownloadedUpdate(path)}
      onSaveDraft={(draft) => void handleSaveDraft(draft)}
      onToggleAlwaysOnTop={() => void toggleAlwaysOnTop()}
      onToggleAutoStart={() => void toggleAutoStart()}
      onToggleLanguage={() => void toggleLanguage()}
      onToggleNotePin={(id) => {
        const note = notesRef.current.find((item) => item.id === id);
        if (note) {
          void patchNote(id, { pinned: !note.pinned });
        }
      }}
      ready={ready}
      showDeleteAllConfirm={showDeleteAllConfirm}
      showSettings={showSettings}
      t={t}
      toast={toast}
      updateDialogOpen={updateDialogOpen}
      updateDownloadProgress={updateDownloadProgress}
      updateDownloadResult={updateDownloadResult}
      updateInfo={updateInfo}
    />
  );
}

export default App;
