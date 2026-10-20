import {
  cleanup,
  computed,
  signal,
  Text,
  View,
  type YNode,
  type GestureResponderEvent,
} from "nsh";
import { AttachmentImage, Button, Icon, Motion } from "./components";
import { presence } from "./presence";
import type { AppState } from "./state";
import type { Note, NoteAttachment } from "./types";

export interface CardHooks {
  refs: Map<string, YNode>;
  suppress: () => boolean;
  drag: (
    id: string,
    event: GestureResponderEvent,
    phase: "start" | "move" | "end",
  ) => void;
  preview: (images: NoteAttachment[], index: number) => void;
  palette: (id: string, color: string, node: YNode) => void;
}

export function NoteCard(props: {
  state: AppState;
  note: Note;
  hooks: CardHooks;
  dragging: boolean;
}) {
  const s = props.state;
  const hover = signal(false),
    actions = signal(false),
    pressed = signal(false),
    totalLines = signal(1),
    draftLines = signal<number | null>(null);
  const shownActions = presence(() => (actions() ? true : null));
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  let blockedUntil = 0;
  let resizeStart: { y: number; lines: number } | null = null;
  let node: YNode;
  const images = () => props.note.attachments.filter((a) => a.kind === "image");
  const files = () => props.note.attachments.filter((a) => a.kind === "file");
  const body = () =>
    props.note.content.trim()
      ? props.note.content
      : images().length
        ? s.t().imageOnly
        : "";
  const lines = computed(() =>
    body()
      ? Math.min(
          totalLines(),
          Math.max(
            Math.min(2, totalLines()),
            draftLines() ??
              (props.note.textHeight === null
                ? Math.min(2, totalLines())
                : Math.round(props.note.textHeight / 22)),
          ),
        )
      : 0,
  );
  const cardHeight = () =>
    19 +
    lines() * 22 +
    (images().length ? 58 + (lines() ? 8 : 0) : 0) +
    (files().length
      ? Math.min(files().length, 3) * 30 +
        Math.max(0, Math.min(files().length, 3) - 1) * 6 +
        (lines() || images().length ? 8 : 0)
      : 0);
  function hold() {
    if (closeTimer !== undefined) clearTimeout(closeTimer);
    actions.set(true);
  }
  function leave() {
    if (closeTimer !== undefined) clearTimeout(closeTimer);
    closeTimer = setTimeout(() => {
      if (!pressed()) actions.set(false);
    }, 150);
  }
  function action(command: string) {
    void s.call(command, { id: props.note.id });
    actions.set(false);
  }
  function resize(
    event: GestureResponderEvent,
    phase: "start" | "move" | "end",
  ) {
    event.stopPropagation();
    if (phase === "start") resizeStart = { y: event.pageY, lines: lines() };
    if (!resizeStart) return;
    const next = Math.max(
      Math.min(2, totalLines()),
      Math.min(
        totalLines(),
        Math.ceil(resizeStart.lines + (event.pageY - resizeStart.y) / 22),
      ),
    );
    if (phase === "move") draftLines.set(next);
    if (phase === "end") {
      blockedUntil = Date.now() + 300;
      resizeStart = null;
      draftLines.set(null);
      void s.call("height", { id: props.note.id, height: next * 22 });
    }
  }
  cleanup(() => {
    if (closeTimer !== undefined) clearTimeout(closeTimer);
    props.hooks.refs.delete(props.note.id);
  });
  return (
    <View
      ref={(ref) => {
        node = ref;
        props.hooks.refs.set(props.note.id, ref);
      }}
      testID={`note-${props.note.id}`}
      accessibilityLabel={props.note.content || s.t().imageOnly}
      onHoverIn={() => hover.set(true)}
      onHoverOut={() => {
        hover.set(false);
        leave();
      }}
      onPress={() => {
        if (Date.now() >= blockedUntil && !props.hooks.suppress())
          action("copy");
      }}
      onTouchStart={(e) => props.hooks.drag(props.note.id, e, "start")}
      onTouchMove={(e) => props.hooks.drag(props.note.id, e, "move")}
      onTouchEnd={(e) => props.hooks.drag(props.note.id, e, "end")}
      style={{
        position: "relative",
        height: cardHeight(),
        flexShrink: 0,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: hover()
          ? "rgba(0,122,255,0.36)"
          : props.note.pinned
            ? "rgba(0,122,255,0.28)"
            : "rgba(255,255,255,0.5)",
        backgroundColor: props.note.color,
        paddingTop: 9,
        paddingBottom: 8,
        paddingLeft: 9,
        paddingRight: 27,
        boxShadow: hover()
          ? "0 12px 28px rgba(31,35,40,0.10)"
          : "0 8px 24px rgba(31,35,40,0.08)",
        opacity: props.dragging ? 0.28 : 1,
        zIndex: actions() ? 10 : 0,
        cursor: "grab",
        transitionProperty: "background-color, border-color, box-shadow",
        transitionDuration: 160,
      }}
    >
      <If when={props.note.pinned}>
        <View
          pointerEvents="none"
          style={{
            position: "absolute",
            top: 1,
            left: 1,
            zIndex: 2,
            width: 14,
            height: 14,
            borderRadius: 99,
            backgroundColor: "rgba(255,255,255,0.42)",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Icon name="Pin" size={8} color="#007aff" />
        </View>
      </If>
      <If when={!!body()}>
        <Text
          allowFontScaling={false}
          numberOfLines={0}
          onTextLayout={(e) => totalLines.set(Math.max(1, e.lines.length))}
          style={{
            position: "absolute",
            left: 9,
            right: 27,
            top: 9,
            fontSize: 13.44,
            lineHeight: 22,
            opacity: 0,
          }}
          pointerEvents="none"
        >
          {body()}
        </Text>
        <Text
          allowFontScaling={false}
          numberOfLines={lines()}
          ellipsizeMode="tail"
          style={{
            height: lines() * 22,
            fontSize: 13.44,
            lineHeight: 22,
            color: props.note.content.trim()
              ? "#1d1d1f"
              : "rgba(31,41,55,0.58)",
          }}
        >
          {body()}
        </Text>
      </If>
      <If when={images().length > 0}>
        <View
          onPress={() => {}}
          style={{
            height: 58,
            marginTop: lines() ? 8 : 0,
            flexDirection: "row",
            gap: 8,
            overflow: "hidden",
          }}
        >
          <Each of={images().slice(0, 4)} key={(item) => item.id}>
            {(item, index) => (
              <View
                onPress={() => props.hooks.preview(images(), index())}
                accessibilityRole="button"
                accessibilityLabel={item().name ?? s.t().addImage}
                style={{
                  width: 58,
                  height: 58,
                  borderRadius: 7,
                  borderWidth: 1,
                  borderColor: "rgba(29,39,53,0.14)",
                  overflow: "hidden",
                  flexShrink: 0,
                  cursor: "pointer",
                }}
              >
                <AttachmentImage
                  state={s}
                  attachment={item()}
                  style={{ width: 56, height: 56 }}
                />
              </View>
            )}
          </Each>
        </View>
      </If>
      <If when={files().length > 0}>
        <View
          onTouchStart={(e) => e.stopPropagation()}
          onPress={() => {}}
          style={{ marginTop: lines() || images().length ? 8 : 0, gap: 6 }}
        >
          <Each of={files().slice(0, 3)} key={(item) => item.id}>
            {(file) => (
              <View
                style={{
                  height: 30,
                  flexDirection: "row",
                  alignItems: "center",
                  gap: 6,
                  borderRadius: 8,
                  borderWidth: 1,
                  borderColor: "rgba(60,60,67,0.12)",
                  paddingHorizontal: 8,
                  backgroundColor: "rgba(255,255,255,0.48)",
                }}
              >
                <Icon name="FileText" size={12} />
                <Text
                  numberOfLines={1}
                  ellipsizeMode="tail"
                  style={{
                    flex: 1,
                    fontSize: 11.48,
                    color: "rgba(29,29,31,0.76)",
                  }}
                >
                  {file().name || file().value}
                </Text>
              </View>
            )}
          </Each>
        </View>
      </If>
      <If when={hover() || shownActions.value()}>
        <View
          onTouchStart={(e) => e.stopPropagation()}
          onHoverIn={hold}
          onHoverOut={leave}
          onPressIn={() => pressed.set(true)}
          onPressOut={() => pressed.set(false)}
          style={{
            position: "absolute",
            right: 1,
            top: (cardHeight() - 30) / 2,
            width: shownActions.value() ? 180 : 20,
            height: 30,
            zIndex: 4,
            justifyContent: "center",
          }}
        >
          <If when={shownActions.value()}>
            <Motion
              closing={shownActions.closing()}
              style={{
                position: "absolute",
                right: 22,
                height: 34,
                flexDirection: "row",
                alignItems: "center",
                gap: 5,
                padding: 4,
                borderWidth: 1,
                borderColor: "rgba(60,60,67,0.08)",
                borderRadius: 12,
                backgroundColor: "rgba(255,255,255,0.94)",
                boxShadow: "0 10px 24px rgba(31,35,40,0.12)",
              }}
            >
              <Button
                small
                label={props.note.pinned ? s.t().unpin : s.t().pin}
                icon={props.note.pinned ? "PinOff" : "Pin"}
                onPress={() => action("pin")}
              />
              <Button
                small
                label={s.t().edit}
                icon="Pencil"
                onPress={() => action("edit")}
              />
              <Button
                small
                label={s.t().color}
                icon="Palette"
                onPress={() => {
                  props.hooks.palette(props.note.id, props.note.color, node);
                  actions.set(false);
                }}
              />
              <Button
                small
                label={s.t().copy}
                icon="Copy"
                onPress={() => action("copy")}
              />
              <Button
                small
                danger
                label={s.t().delete}
                icon="Trash2"
                onPress={() => action("delete")}
              />
            </Motion>
          </If>
          <View
            onPress={hold}
            style={{
              position: "absolute",
              right: 0,
              width: 20,
              height: 20,
              borderRadius: 99,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "rgba(255,255,255,0.60)",
              cursor: "pointer",
            }}
          >
            <Icon
              name="ChevronLeft"
              size={12}
              color={actions() ? "#007aff" : "#666"}
            />
          </View>
        </View>
      </If>
      <If
        when={
          (hover() || resizeStart !== null) &&
          totalLines() > Math.min(2, totalLines())
        }
      >
        <View
          accessibilityRole="button"
          accessibilityLabel={s.t().resize}
          onTouchStart={(e) => e.stopPropagation()}
          onStartShouldSetResponder={() => true}
          onResponderGrant={(e) => {
            resize(e, "start");
            return true;
          }}
          onResponderMove={(e) => resize(e, "move")}
          onResponderRelease={(e) => resize(e, "end")}
          onResponderTerminationRequest={() => false}
          onResponderTerminate={() => {
            resizeStart = null;
            draftLines.set(null);
            blockedUntil = Date.now() + 300;
          }}
          onPress={() => {}}
          style={{
            position: "absolute",
            right: 7,
            bottom: 0,
            width: 24,
            height: 12,
            zIndex: 3,
            alignItems: "center",
            justifyContent: "center",
            cursor: "ns-resize",
          }}
        >
          <Icon name="GripHorizontal" size={12} />
        </View>
      </If>
    </View>
  );
}
