import {
  cleanup,
  Scroll,
  signal,
  Text,
  View,
  type YNode,
  type GestureResponderEvent,
} from "nsh";
import { windowSize } from "nsh/device";
import { keymap } from "nsh/keymap";
import {
  Button,
  Icon,
  Motion,
  Palette,
  QMark,
  TitleBar,
  Toast,
} from "./components";
import { DeleteAll, Settings, UpdateDialog } from "./Dialogs";
import { NoteCard, type CardHooks } from "./NoteCard";
import { Preview } from "./Preview";
import { presence } from "./presence";
import type { AppState } from "./state";
import type { NoteAttachment } from "./types";

export function Main(props: { state: AppState }) {
  const s = props.state,
    modal = signal<"settings" | "delete" | null>(null),
    closing = signal(false),
    palette = signal<{
      id: string;
      color: string;
      x: number;
      y: number;
    } | null>(null),
    preview = signal<{ items: NoteAttachment[]; index: number } | null>(null),
    dragging = signal("");
  const shownPreview = presence(() => preview());
  const shownUpdate = presence(() => (s.snapshot().update.show ? true : null));
  const context = signal<{ id: string; x: number; y: number } | null>(null);
  const shownContext = presence(() => context());
  const shownPalette = presence(() => palette());
  const refs = new Map<string, YNode>();
  let dragStart: { id: string; x: number; y: number } | null = null,
    suppressUntil = 0,
    closeTimer: ReturnType<typeof setTimeout>;
  let scroll: YNode,
    scrollOffset = 0;
  const closeModal = () => {
    if (closing()) return;
    closing.set(true);
    closeTimer = setTimeout(() => {
      modal.set(null);
      closing.set(false);
    }, 150);
  };
  const openModal = (name: "settings" | "delete") => {
    if (closeTimer !== undefined) clearTimeout(closeTimer);
    closing.set(false);
    palette.set(null);
    modal.set(name);
    if (name === "settings") void s.call("readAutoStart");
  };
  cleanup(() => {
    if (closeTimer !== undefined) clearTimeout(closeTimer);
  });
  keymap.bind("escape", () => {
    if (context()) context.set(null);
    else if (palette()) palette.set(null);
    else if (
      s.snapshot().update.show &&
      s.snapshot().update.phase !== "installing"
    )
      void s.call(
        s.snapshot().update.phase === "idle" ? "dismissUpdate" : "cancelUpdate",
      );
    else if (modal()) closeModal();
  });
  keymap.bind("mod+n", () => {
    if (!modal() && !shownPreview.value() && !shownUpdate.value())
      void s.call("edit");
  });
  async function drag(
    id: string,
    e: GestureResponderEvent,
    phase: "start" | "move" | "end",
  ) {
    if (phase === "start") {
      dragStart = { id, x: e.pageX, y: e.pageY };
      return;
    }
    if (!dragStart || dragStart.id !== id) return;
    const moved = Math.hypot(e.pageX - dragStart.x, e.pageY - dragStart.y) > 4;
    if (phase === "move" && moved) {
      dragging.set(id);
      suppressUntil = Date.now() + 300;
      if (e.pageY < 90) {
        scrollOffset = Math.max(0, scrollOffset - 12);
        scroll?.scrollTo({ y: scrollOffset });
      } else if (e.pageY > windowSize().height - 48) {
        scrollOffset += 12;
        scroll?.scrollTo({ y: scrollOffset });
      }
    }
    if (phase === "end") {
      dragStart = null;
      if (dragging()) {
        suppressUntil = Date.now() + 300;
        dragging.set("");
        for (const [target, node] of refs) {
          if (target === id) continue;
          const rect = await node.measure();
          if (
            rect &&
            e.pageX >= rect.pageX &&
            e.pageX <= rect.pageX + rect.width &&
            e.pageY >= rect.pageY &&
            e.pageY <= rect.pageY + rect.height
          ) {
            void s.call("reorder", {
              id,
              target,
              after: e.pageY > rect.pageY + rect.height / 2,
            });
            break;
          }
        }
      }
    }
  }
  const hooks: CardHooks = {
    refs,
    suppress: () => Date.now() < suppressUntil,
    drag: (id, e, phase) => void drag(id, e, phase),
    preview: (items, index) => preview.set({ items, index }),
    palette: (id, color, node) => {
      void node.measure().then((rect) => {
        if (rect)
          palette.set({
            id,
            color,
            x: Math.max(
              8,
              Math.min(windowSize().width - 117, rect.pageX + rect.width - 120),
            ),
            y: Math.max(
              8,
              Math.min(
                windowSize().height - 90,
                rect.pageY + rect.height / 2 + 18,
              ),
            ),
          });
      });
    },
  };
  s.onEvent((event) => {
    if (
      event.kind === "context" &&
      !modal() &&
      !shownPreview.value() &&
      !shownUpdate.value()
    ) {
      palette.set(null);
      for (const [id, node] of refs)
        void node.measure().then((rect) => {
          if (
            rect &&
            (event.x ?? -1) >= rect.pageX &&
            (event.x ?? -1) < rect.pageX + rect.width &&
            (event.y ?? -1) >= rect.pageY &&
            (event.y ?? -1) < rect.pageY + rect.height
          ) {
            context.set({
              id,
              x: Math.max(8, Math.min(windowSize().width - 160, event.x ?? 8)),
              y: Math.max(8, Math.min(windowSize().height - 132, event.y ?? 8)),
            });
          }
        });
    }
  });
  return (
    <View
      style={{
        flex: 1,
        borderRadius: 12,
        overflow: "hidden",
        backgroundColor: "#ffd150",
        backgroundImage:
          "linear-gradient(180deg, rgba(255,255,255,0.34), rgba(255,255,255,0) 46%)",
        paddingTop: 8,
        paddingHorizontal: 10,
        paddingBottom: 28,
      }}
    >
      <View
        style={{
          flex: 1,
          width: "100%",
          maxWidth: 980,
          alignSelf: "center",
          minHeight: 0,
        }}
      >
        <TitleBar state={s} title="Q Note" />
        <View
          style={{
            flexDirection: "row",
            justifyContent: "flex-end",
            flexWrap: "wrap",
            gap: 4,
            padding: 3,
            marginBottom: 7,
            borderRadius: 8,
            borderWidth: 1,
            borderColor: "rgba(255,255,255,0.32)",
            backgroundColor: "rgba(255,255,255,0.14)",
            flexShrink: 0,
          }}
        >
          <Button
            testID="new-note"
            label={s.t().newNote}
            text={s.t().newNote}
            icon="Plus"
            onPress={() => void s.call("edit")}
          />
          <Button
            label={s.t().deleteAll}
            icon="Trash2"
            disabled={!s.snapshot().data.notes.length}
            onPress={() => openModal("delete")}
          />
          <View>
            <Button
              label={s.t().settings}
              icon="Settings"
              onPress={() => openModal("settings")}
            />
            <If when={s.snapshot().update.info}>
              <View
                pointerEvents="none"
                style={{
                  position: "absolute",
                  top: 3,
                  right: 3,
                  width: 6,
                  height: 6,
                  borderRadius: 99,
                  backgroundColor: "#ff3b30",
                }}
              />
            </If>
          </View>
          <Button
            label={s.t().switchLanguage}
            text={s.t().language}
            onPress={() => void s.call("language")}
          />
        </View>
        <If when={!s.snapshot().data.notes.length}>
          <View
            style={{
              flex: 1,
              alignItems: "center",
              justifyContent: "center",
              gap: 8,
            }}
          >
            <QMark size={48} />
            <Text style={{ fontSize: 14, fontWeight: "600", color: "#1d2735" }}>
              {s.t().emptyTitle}
            </Text>
            <Text
              style={{ fontSize: 14, color: "#1d2735", textAlign: "center" }}
            >
              {s.t().noNotesBody}
            </Text>
            <Button
              primary
              label={s.t().emptyAction}
              text={s.t().emptyAction}
              onPress={() => void s.call("edit")}
            />
          </View>
        </If>
        <If when={s.snapshot().data.notes.length > 0}>
          <Scroll
            ref={(node) => (scroll = node)}
            onScroll={(e) => (scrollOffset = e.y)}
            style={{ flex: 1, minHeight: 0, marginRight: -8 }}
            contentContainerStyle={{
              gap: 10,
              paddingRight: 8,
              paddingBottom: 8,
            }}
          >
            <Each of={s.snapshot().data.notes} key={(note) => note.id}>
              {(note) => (
                <NoteCard
                  state={s}
                  note={note()}
                  hooks={hooks}
                  dragging={dragging() === note().id}
                />
              )}
            </Each>
          </Scroll>
        </If>
      </View>
      <View
        pointerEvents="none"
        style={{
          position: "absolute",
          left: 12,
          right: 12,
          bottom: 7,
          height: 18,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Text style={{ fontSize: 12.04, color: "rgba(60,60,67,0.78)" }}>
          {s.t().statusSummary(s.snapshot().data.notes.length)}
        </Text>
      </View>
      <View
        accessibilityRole="button"
        accessibilityLabel={s.t().switchFloatingBall}
        onPress={() => void s.call("dock", { window: "main" })}
        style={{
          position: "absolute",
          right: 14,
          bottom: 22,
          width: 28,
          height: 28,
          borderRadius: 99,
          borderWidth: 1,
          borderColor: "rgba(255,255,255,0.38)",
          backgroundColor: "rgba(255,255,255,0.22)",
          alignItems: "center",
          justifyContent: "center",
          cursor: "pointer",
          boxShadow: "0 4px 12px rgba(31,35,40,0.06)",
        }}
      >
        <QMark size={18} />
      </View>
      <If when={shownPalette.value()}>
        <View
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            zIndex: 40,
          }}
        >
          <View
            onPress={() => palette.set(null)}
            style={{
              position: "absolute",
              top: 0,
              bottom: 0,
              left: 0,
              right: 0,
            }}
          />
          <Motion
            closing={shownPalette.closing()}
            style={{
              position: "absolute",
              left: shownPalette.value()!.x,
              top: shownPalette.value()!.y,
              width: 109,
              padding: 6,
              borderRadius: 8,
              borderWidth: 1,
              borderColor: "rgba(29,39,53,0.14)",
              backgroundColor: "rgba(255,255,255,0.92)",
              boxShadow: "0 16px 34px rgba(31,35,40,0.16)",
            }}
          >
            <Palette
              compact
              state={s}
              color={shownPalette.value()!.color}
              onChange={(color) => {
                void s.call("color", { id: shownPalette.value()!.id, color });
                palette.set(null);
              }}
            />
          </Motion>
        </View>
      </If>
      <If when={shownContext.value()}>
        <View
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            zIndex: 45,
          }}
        >
          <View
            onPress={() => context.set(null)}
            style={{
              position: "absolute",
              top: 0,
              bottom: 0,
              left: 0,
              right: 0,
            }}
          />
          <Motion
            closing={shownContext.closing()}
            style={{
              position: "absolute",
              left: shownContext.value()!.x,
              top: shownContext.value()!.y,
              width: 152,
              borderRadius: 7,
              borderWidth: 1,
              borderColor: "rgba(255,255,255,0.52)",
              backgroundColor: "rgba(255,253,244,0.96)",
              boxShadow: "0 10px 24px rgba(31,35,40,0.14)",
              overflow: "hidden",
            }}
          >
            <Each of={["copy", "edit", "pin", "delete"] as const}>
              {(command) => (
                <View
                  accessibilityRole="button"
                  accessibilityLabel={
                    command() === "pin"
                      ? s
                          .snapshot()
                          .data.notes.find(
                            (n) => n.id === shownContext.value()?.id,
                          )?.pinned
                        ? s.t().unpin
                        : s.t().pin
                      : s.t()[command()]
                  }
                  onPress={() => {
                    void s.call(command(), { id: shownContext.value()!.id });
                    context.set(null);
                  }}
                  class="hover:bg-black/10"
                  style={{
                    height: 30,
                    flexDirection: "row",
                    alignItems: "center",
                    gap: 6,
                    paddingHorizontal: 8,
                    cursor: "pointer",
                  }}
                >
                  <Icon
                    name={
                      command() === "copy"
                        ? "Copy"
                        : command() === "edit"
                          ? "Pencil"
                          : command() === "pin"
                            ? "Pin"
                            : "Trash2"
                    }
                    color={command() === "delete" ? "#ff3b30" : "#414144"}
                  />
                  <Text
                    style={{
                      fontSize: 13,
                      color: command() === "delete" ? "#ff3b30" : "#1d1d1f",
                    }}
                  >
                    {command() === "pin"
                      ? s
                          .snapshot()
                          .data.notes.find(
                            (n) => n.id === shownContext.value()?.id,
                          )?.pinned
                        ? s.t().unpin
                        : s.t().pin
                      : s.t()[command()]}
                  </Text>
                </View>
              )}
            </Each>
          </Motion>
        </View>
      </If>
      <If when={modal() === "settings"}>
        <Settings state={s} onClose={closeModal} closing={closing()} />
      </If>
      <If when={modal() === "delete"}>
        <DeleteAll state={s} onClose={closeModal} closing={closing()} />
      </If>
      <UpdateDialog state={s} />
      <If when={shownPreview.value()}>
        <Preview
          state={s}
          items={shownPreview.value()!.items}
          initialIndex={shownPreview.value()!.index}
          closing={shownPreview.closing()}
          onClose={() => preview.set(null)}
        />
      </If>
      <Toast state={s} />
    </View>
  );
}
