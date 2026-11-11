import {
  cleanup,
  effect,
  PanResponder,
  Scroll,
  signal,
  Text,
  View,
  type ViewRef,
} from "nsh";
import { Button, Progress, Switch } from "nsh/base";
import { currentWindow, isAvailable, Menu, Window } from "nsh/desktop";
import { createDesktop, clamp, type DesktopController } from "./desktop";
import { createModel, type NoteModel } from "./model";
import { colors as C } from "./theme";
import { Action, Modal, presence, QMark } from "./ui";
import { Editor } from "./components/Editor";
import { NoteCard, type NoteDrag } from "./components/Notes";
import { ImagePreview } from "./components/Preview";
import { WindowBar } from "./components/WindowBar";

function DockBall(props: { model: NoteModel; desktop: DesktopController }) {
  const d = props.desktop;
  let start = { x: 0, y: 0 };
  let dragged = 0;
  let moving = false;
  const motion = presence(() => true, 180);
  const responder = PanResponder.create({
    onMoveShouldSetPanResponder: (_e, state) =>
      Math.hypot(state.dx, state.dy) > 4,
    onPanResponderGrant: () => {
      moving = true;
      start = d.beginDockDrag();
    },
    onPanResponderMove: (_e, state) => {
      dragged = Date.now();
      d.moveDock({ x: start.x + state.dx, y: start.y + state.dy });
    },
    onPanResponderRelease: () => {
      moving = false;
      dragged = Date.now();
      d.endDockDrag();
    },
    onPanResponderTerminate: () => {
      moving = false;
      d.endDockDrag();
    },
  });
  return (
    <View
      {...responder.panHandlers}
      testID="dock-ball"
      onPress={() => {
        if (Date.now() - dragged > 250) d.restore();
      }}
      onMouseEnter={d.expandDock}
      onMouseLeave={() => {
        if (!moving) d.hideDock();
      }}
      onContextMenu={(e) => {
        void Menu.popup(d.menu(), e.pagePosition);
      }}
      style={{
        width: 30,
        height: 30,
        borderRadius: 15,
        overflow: "hidden",
        opacity: motion.opacity,
      }}
    >
      <QMark size={30} />
    </View>
  );
}

function Settings(props: { model: NoteModel }) {
  const m = props.model;
  return (
    <Modal
      open={m.settingsOpen()}
      title={m.t().settingsTitle}
      onClose={() => m.settingsOpen.set(false)}
      testID="settings"
      compact
    >
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
        }}
      >
        <Text style={{ fontSize: 13 }}>{m.t().startupSetting}</Text>
        <Switch
          testID="startup-switch"
          accessibilityLabel={m.t().startupSetting}
          checked={m.settings().autoStart}
          disabled={m.saving() || !isAvailable}
          onChange={({ checked }) => {
            void m.toggleAutoStart(checked);
          }}
          trackStyle={{
            width: 34,
            height: 20,
            borderRadius: 10,
            backgroundColor: m.settings().autoStart ? C.success : "#d0cbbd",
          }}
          thumbStyle={{
            width: 16,
            height: 16,
            margin: 2,
            borderRadius: 8,
            backgroundColor: "#fff",
            transform: [{ translateX: m.settings().autoStart ? 14 : 0 }],
          }}
        />
      </View>
      <View style={{ height: 1, backgroundColor: C.border }} />
      <View style={{ gap: 6 }}>
        <Action
          label={m.t().import}
          icon="upload"
          disabled={m.saving() || !isAvailable}
          onPress={() => {
            void m.importData();
          }}
          testID="settings-import"
        />
        <Action
          label={m.t().export}
          icon="download"
          disabled={!isAvailable}
          onPress={() => {
            void m.exportData();
          }}
          testID="settings-export"
        />
        <View>
          <Action
            label={m.checking() ? m.t().checkingUpdate : m.t().checkUpdate}
            icon="reset"
            disabled={m.checking()}
            onPress={() => {
              void m.checkUpdate(true);
            }}
            testID="settings-update"
          />
          <If when={!!m.update()}>
            <View
              style={{
                pointerEvents: "none",
                position: "absolute",
                right: 6,
                top: 5,
                width: 6,
                height: 6,
                borderRadius: 3,
                backgroundColor: C.danger,
              }}
            />
          </If>
        </View>
      </View>
      <Button
        testID="settings-version"
        onPress={() => {
          void m.currentRelease();
        }}
        style={{ alignItems: "center" }}
      >
        <Text
          style={{ fontSize: 12, color: C.muted }}
        >{`Q Note v${m.version}`}</Text>
      </Button>
    </Modal>
  );
}

function UpdateDialog(props: { model: NoteModel }) {
  const m = props.model;
  const phase = () => m.updatePhase();
  return (
    <Modal
      open={m.updateOpen()}
      title={m.t().updateAvailableTitle(m.update()?.version ?? "")}
      onClose={() => {
        if (phase()) m.cancelUpdate();
        else m.updateOpen.set(false);
      }}
      blocked={phase() === "installing"}
      testID="update"
    >
      <If when={!!m.update()?.notes}>
        <Scroll style={{ maxHeight: 180 }}>
          <Text style={{ fontSize: 13, lineHeight: 21 }}>
            {m.update()?.notes ?? ""}
          </Text>
        </Scroll>
      </If>
      <Text style={{ fontSize: 13, lineHeight: 21 }}>
        {m.t().updateConfirmDefault}
      </Text>
      <If when={!!phase()}>
        <Text style={{ fontSize: 13 }}>
          {phase() === "downloading"
            ? m.t().updateDownloading
            : phase() === "preparing"
              ? m.t().updatePreparing
              : m.t().updateInstalling}
        </Text>
        <Progress
          value={
            m.progress().total
              ? (m.progress().received / m.progress().total) * 100
              : undefined
          }
          trackStyle={{ height: 5, borderRadius: 3, backgroundColor: C.border }}
          indicatorStyle={{
            height: 5,
            backgroundColor: C.primary,
            borderRadius: 3,
          }}
        />
        <Text
          style={{ fontSize: 12, color: C.muted }}
        >{`${(m.progress().received / 1048576).toFixed(1)} / ${(m.progress().total / 1048576).toFixed(1)} MB${m.progress().total ? ` · ${Math.round((m.progress().received / m.progress().total) * 100)}%` : ""}`}</Text>
      </If>
      <View
        style={{ flexDirection: "row", justifyContent: "flex-end", gap: 8 }}
      >
        <Action
          label={m.t().cancel}
          disabled={phase() === "installing"}
          onPress={() => {
            if (phase()) m.cancelUpdate();
            else m.updateOpen.set(false);
          }}
        />
        <If when={!phase()}>
          <Action
            label={m.t().updateConfirm}
            primary
            onPress={() => {
              void m.installUpdate();
            }}
          />
        </If>
      </View>
    </Modal>
  );
}

export function QNote() {
  const m = createModel();
  const desktop = createDesktop(m);
  const toastMotion = presence(() => !!m.toast());
  const toastText = signal("");
  effect(() => {
    if (m.toast()) toastText.set(m.toast());
  });
  const dragId = signal<string | null>(null);
  let dragTarget: string | null = null;
  let suppressUntil = 0;
  const cards = new Map<string, ViewRef>();
  let dragMeasure: Promise<void> = Promise.resolve();
  const drag: NoteDrag = {
    start(id) {
      dragId.set(id);
      dragTarget = id;
    },
    over(id) {
      if (dragId()) dragTarget = id;
    },
    async finish() {
      await dragMeasure;
      const id = dragId();
      dragId.set(null);
      suppressUntil = Date.now() + 250;
      if (id && dragTarget) void m.reorder(id, dragTarget);
      dragTarget = null;
    },
    active: dragId,
    suppressed: () => !!dragId() || Date.now() < suppressUntil,
    register(id, ref) {
      if (ref) cards.set(id, ref);
      else cards.delete(id);
    },
    move(y) {
      dragMeasure = Promise.all(
        [...cards].map(async ([id, ref]) => ({ id, box: await ref.measure() })),
      ).then((positions) => {
        if (!dragId()) return;
        const target = positions.find(
          (p) =>
            p.box &&
            y >= p.box.pagePosition.y &&
            y <= p.box.pagePosition.y + p.box.bounds.size.height,
        );
        if (target) dragTarget = target.id;
      });
    },
  };
  let updateTimer: ReturnType<typeof setTimeout> | undefined;

  function dailyCheck() {
    const now = new Date();
    const next = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 17);
    if (next.getTime() <= now.getTime()) next.setDate(next.getDate() + 1);
    updateTimer = setTimeout(() => {
      void m.checkUpdate().finally(dailyCheck);
    }, next.getTime() - now.getTime());
  }
  void m.boot().then(() => {
    void m.checkUpdate();
    dailyCheck();
  });
  cleanup(() => clearTimeout(updateTimer));

  function editorPosition() {
    const main = isAvailable ? currentWindow().state()?.bounds : null;
    const area = desktop.area(isAvailable ? currentWindow() : null);
    return {
      x: clamp(
        (main?.origin.x ?? 532) - 532,
        area.origin.x,
        area.origin.x + area.size.width - 520,
      ),
      y: clamp(
        (main?.origin.y ?? 0) + ((main?.size.height ?? 450) - 640) / 2,
        area.origin.y,
        area.origin.y + area.size.height - 640,
      ),
    };
  }

  let previousDocked = false;
  effect(() => {
    const docked = m.docked();
    if (isAvailable && previousDocked && !docked) Window.get("main")?.focus();
    previousDocked = docked;
  });
  effect(() => {
    const session = m.editor();
    if (session && isAvailable) {
      const focusTimer = setTimeout(() => Window.get("editor")?.focus(), 0);
      return () => clearTimeout(focusTimer);
    }
  });

  return (
    <View
      testID="q-note"
      style={{
        flex: 1,
        backgroundColor: C.background,
        backgroundImage: "linear-gradient(180deg, #ffffff57, #ffffff00 46%)",
        borderRadius: 12,
        overflow: "hidden",
        paddingTop: 8,
        paddingHorizontal: 10,
        paddingBottom: 28,
      }}
    >
      <View
        style={{
          flex: 1,
          minHeight: 0,
          width: "100%",
          maxWidth: 980,
          alignSelf: "center",
        }}
      >
        <WindowBar model={m} title="Q Note" />
        <View
          style={{
            flexDirection: "row",
            alignItems: "center",
            flexWrap: "wrap",
            gap: 4,
            padding: 3,
            marginBottom: 7,
            borderWidth: 1,
            borderColor: "#ffffff52",
            borderRadius: 8,
            backgroundColor: "#ffffff24",
            boxShadow: "inset 0 1px 0 #ffffff42, 0 4px 12px #1f232809",
          }}
        >
          <Action
            label={m.t().newNote}
            icon="plus"
            iconOnly
            onPress={() => m.openEditor()}
            disabled={m.loading()}
            testID="new-note"
          />
          <Action
            label={m.t().deleteAll}
            icon="trash"
            danger
            iconOnly
            onPress={() => m.deleteOpen.set(true)}
            disabled={!m.notes().length || m.saving()}
            testID="delete-all"
          />
          <View>
            <Action
              icon="settings"
              iconOnly
              label={m.t().settings}
              onPress={() => m.settingsOpen.set(true)}
              testID="open-settings"
            />
            <If when={!!m.update()}>
              <View
                style={{
                  pointerEvents: "none",
                  position: "absolute",
                  right: 3,
                  top: 3,
                  width: 5,
                  height: 5,
                  borderRadius: 3,
                  backgroundColor: C.danger,
                }}
              />
            </If>
          </View>
          <Action
            label={m.t().language}
            icon="languages"
            onPress={() => {
              void m.toggleLanguage();
            }}
            disabled={m.saving()}
            testID="language"
          />
        </View>
        <If
          when={m.notes().length > 0}
          else={
            <View
              style={{
                flex: 1,
                alignItems: "center",
                justifyContent: "center",
                gap: 8,
                paddingVertical: 8,
              }}
            >
              <QMark size={48} />
              <Text style={{ fontSize: 15, fontWeight: "600" }}>
                {m.loading() ? "Q Note" : m.t().emptyTitle}
              </Text>
              <If when={!m.loading()}>
                <Text
                  style={{ fontSize: 12, color: C.muted, textAlign: "center" }}
                >
                  {m.t().noNotesBody}
                </Text>
                <Action
                  label={m.t().emptyAction}
                  icon="plus"
                  primary
                  onPress={() => m.openEditor()}
                  testID="empty-new"
                />
              </If>
            </View>
          }
        >
          <Scroll
            testID="note-list"
            style={{ flex: 1, minHeight: 0 }}
            contentStyle={{
              paddingBottom: 8,
              gap: 10,
              width: "100%",
              alignSelf: "center",
            }}
          >
            <Each of={m.notes()} key={(note) => note.id}>
              {(note) => <NoteCard model={m} note={note()} drag={drag} />}
            </Each>
          </Scroll>
        </If>
      </View>
      <View
        style={{
          position: "absolute",
          bottom: 7,
          left: 12,
          right: 12,
          minHeight: 18,
          alignItems: "center",
          justifyContent: "center",
          pointerEvents: "none",
        }}
      >
        <Text style={{ fontSize: 12, color: "#1d27359e" }}>
          {m.t().statusSummary(m.notes().length)}
        </Text>
      </View>
      <Button
        accessibilityLabel={m.t().switchFloatingBall}
        onPress={() => {
          void desktop.collapse();
        }}
        disabled={!isAvailable}
        testID="collapse"
        style={{
          position: "absolute",
          right: 14,
          bottom: 22,
          width: 28,
          height: 28,
          borderRadius: 14,
          borderWidth: 1,
          borderColor: "#ffffff61",
          backgroundColor: "#ffffff38",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <QMark size={18} />
      </Button>
      <If when={isAvailable}>
        <Window
          id="editor"
          title={
            m.editor()?.original ? m.t().editorEditTitle : m.t().editorNewTitle
          }
          open={!!m.editor()}
          frame="none"
          transparent
          defaultSize={{ width: 520, height: 640 }}
          minSize={{ width: 420, height: 520 }}
          maxSize={{ width: 1000, height: 800 }}
          defaultPosition={editorPosition()}
          maximizable={false}
          level={m.settings().alwaysOnTop ? "floating" : "normal"}
          closeOnPress={false}
          onClosePress={() => {
            void m.closeEditor();
          }}
        >
          <Editor model={m} desktop />
        </Window>
        <Window
          id="dock"
          kind="normal"
          title="Q Note - Dock"
          open={m.docked()}
          frame="none"
          transparent
          shadow={false}
          defaultSize={{ width: 30, height: 30 }}
          minSize={{ width: 30, height: 30 }}
          maxSize={{ width: 30, height: 30 }}
          resizable={false}
          taskbar={false}
          activate={false}
          maximizable={false}
          minimizable={false}
          level={m.settings().alwaysOnTop ? "floating" : "normal"}
          closeOnPress={false}
          onClosePress={desktop.restore}
        >
          <DockBall model={m} desktop={desktop} />
        </Window>
      </If>
      <If when={!isAvailable}>
        <DialogEditor model={m} />
      </If>
      <Settings model={m} />
      <Modal
        open={m.deleteOpen()}
        title={m.t().confirmDeleteAll}
        onClose={() => m.deleteOpen.set(false)}
        testID="delete-confirm"
      >
        <Text style={{ fontSize: 13, lineHeight: 21 }}>
          {m.t().deleteAllBody}
        </Text>
        <View
          style={{ flexDirection: "row", justifyContent: "flex-end", gap: 8 }}
        >
          <Action
            label={m.t().cancel}
            onPress={() => m.deleteOpen.set(false)}
            testID="delete-cancel"
          />
          <Action
            label={m.t().deleteAll}
            danger
            onPress={() => {
              void m.clear();
            }}
            disabled={m.saving()}
            testID="delete-confirm-submit"
          />
        </View>
      </Modal>
      <UpdateDialog model={m} />
      <ImagePreview model={m} />
      <If when={toastMotion.shown()}>
        <View
          testID="toast"
          style={{
            pointerEvents: "none",
            position: "absolute",
            bottom: 40,
            left: 12,
            right: 12,
            alignItems: "center",
            zIndex: 50,
            opacity: toastMotion.opacity,
            transform: [{ translateY: toastMotion.offset }],
          }}
        >
          <Text
            style={{
              fontSize: 12,
              color: "#fff",
              backgroundColor: "#1f2328eb",
              paddingHorizontal: 13,
              paddingVertical: 8,
              borderRadius: 8,
            }}
          >
            {toastText()}
          </Text>
        </View>
      </If>
    </View>
  );
}

function DialogEditor(props: { model: NoteModel }) {
  const m = props.model;
  return (
    <Modal
      open={!!m.editor()}
      title={
        m.editor()?.original ? m.t().editorEditTitle : m.t().editorNewTitle
      }
      onClose={() => {
        void m.closeEditor();
      }}
      testID="web-editor"
    >
      <View style={{ height: 500, maxHeight: "80%" }}>
        <Editor model={m} />
      </View>
    </Modal>
  );
}
