import {
  cleanup,
  effect,
  Image,
  PanResponder,
  signal,
  Text,
  View,
  type ViewRef,
} from "nsh";
import {
  Button,
  ContextMenu,
  MenuItem,
  MenuSeparator,
  Popover,
} from "nsh/base";
import type { NoteModel } from "../model";
import { getAttachmentSrc } from "../lib/images";
import { NOTE_COLORS, type Note, type NoteAttachment } from "../types";
import { colors as C } from "../theme";
import { Action, Icon, presence } from "../ui";

export function AttachmentImage(props: {
  attachment: NoteAttachment;
  size?: number;
  onPress: () => void;
}) {
  const src = signal("");
  const failed = signal(false);
  effect(() => {
    const attachment = props.attachment;
    let active = true;
    src.set("");
    failed.set(false);
    void getAttachmentSrc(attachment)
      .then((value) => {
        if (active) src.set(value);
      })
      .catch(() => {
        if (active) failed.set(true);
      });
    return () => {
      active = false;
    };
  });
  return (
    <Button
      accessibilityLabel={props.attachment.name || props.attachment.value}
      onPress={props.onPress}
      style={{
        width: props.size ?? 58,
        height: props.size ?? 58,
        backgroundColor: "#ffffff80",
        borderRadius: 6,
        overflow: "hidden",
        justifyContent: "center",
        alignItems: "center",
      }}
    >
      <If
        when={!!src() && !failed()}
        else={<Icon name="image" color={C.muted} />}
      >
        <Image
          src={src()}
          style={{ width: "100%", height: "100%", objectFit: "cover" }}
          onError={() => failed.set(true)}
        />
      </If>
    </Button>
  );
}

export function ColorPalette(props: {
  value: string;
  onChange: (color: string) => void;
  testID?: string;
  compact?: boolean;
}) {
  return (
    <View
      testID={props.testID}
      style={{
        flexDirection: "row",
        flexWrap: "wrap",
        gap: props.compact ? 5 : 7,
        width: props.compact ? 95 : undefined,
      }}
    >
      <Each of={NOTE_COLORS}>
        {(color, index) => (
          <Button
            accessibilityLabel={color()}
            testID={`color-${index()}`}
            selected={props.value === color()}
            onPress={() => props.onChange(color())}
            style={{
              width: 20,
              height: 20,
              borderRadius: 10,
              backgroundColor: color(),
              borderWidth: 1,
              borderColor: props.value === color() ? "#1d273573" : "#1d273529",
              boxShadow:
                props.value === color() && !props.compact
                  ? "0 0 0 3px #1d273514"
                  : undefined,
              alignItems: "center",
              justifyContent: "center",
            }}
            hoverStyle={{ opacity: 0.75 }}
            focusStyle={{ borderColor: C.primary }}
          >
            <If when={props.compact && props.value === color()}>
              <Icon name="check" size={11} color="#1d2735" />
            </If>
          </Button>
        )}
      </Each>
    </View>
  );
}

export interface NoteDrag {
  start: (id: string) => void;
  over: (id: string) => void;
  finish: () => void;
  active: () => string | null;
  suppressed: () => boolean;
  register: (id: string, ref: ViewRef | null) => void;
  move: (y: number) => void;
}

export function NoteCard(props: {
  model: NoteModel;
  note: Note;
  drag: NoteDrag;
}) {
  const model = props.model;
  const hovered = signal(false);
  const colorOpen = signal(false);
  const actionsOpen = signal(false);
  const motion = presence(actionsOpen);
  const totalLines = signal(2);
  const resizeLines = signal<number | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let pressed = false;
  let suppressUntil = 0;
  let startHeight = 44;
  let card!: ViewRef;
  cleanup(() => {
    clearTimeout(hoverTimer);
    props.drag.register(props.note.id, null);
  });
  const body = () =>
    props.note.content ||
    (props.note.attachments.some((a) => a.kind === "image")
      ? model.t().imageOnly
      : "");
  const minimum = () => Math.min(2, Math.max(1, totalLines()));
  const visibleLines = () =>
    Math.max(
      minimum(),
      Math.min(
        totalLines(),
        resizeLines() ??
          (props.note.textHeight
            ? Math.round(props.note.textHeight / 22)
            : minimum()),
      ),
    );
  const images = () => props.note.attachments.filter((a) => a.kind === "image");
  const files = () => props.note.attachments.filter((a) => a.kind === "file");

  function leave() {
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => {
      if (!pressed && !colorOpen()) {
        hovered.set(false);
        actionsOpen.set(false);
      }
    }, 150);
  }

  const resize = PanResponder.create({
    onStartShouldSetPanResponder: () => true,
    onPanResponderGrant: () => {
      pressed = true;
      startHeight = visibleLines() * 22;
    },
    onPanResponderMove: (_e, gesture) => {
      resizeLines.set(
        Math.max(
          minimum(),
          Math.min(totalLines(), Math.round((startHeight + gesture.dy) / 22)),
        ),
      );
      suppressUntil = Date.now() + 250;
    },
    onPanResponderRelease: () => {
      pressed = false;
      suppressUntil = Date.now() + 250;
      const next = visibleLines() * 22;
      resizeLines.set(null);
      void model.changeNote(props.note, { textHeight: next });
    },
    onPanResponderTerminate: () => {
      pressed = false;
      resizeLines.set(null);
      suppressUntil = Date.now() + 250;
    },
  });

  const sort = PanResponder.create({
    onMoveShouldSetPanResponder: (_event, gesture) =>
      !pressed && Math.hypot(gesture.dx, gesture.dy) > 4,
    onPanResponderGrant: () => {
      pressed = true;
      props.drag.start(props.note.id);
    },
    onPanResponderMove: (_event, gesture) => props.drag.move(gesture.moveY),
    onPanResponderRelease: () => {
      pressed = false;
      props.drag.finish();
      suppressUntil = Date.now() + 250;
    },
    onPanResponderTerminate: () => {
      pressed = false;
      props.drag.finish();
    },
  });

  return (
    <ContextMenu
      popupStyle={{
        minWidth: 140,
        padding: 5,
        borderRadius: 8,
        backgroundColor: C.surface,
        boxShadow: "0 3px 16px #00000030",
      }}
      trigger={
        <View
          {...sort.panHandlers}
          ref={card}
          testID={`note-${props.note.id}`}
          accessibilityLabel={body()}
          onLayout={() => props.drag.register(props.note.id, card)}
          onMouseEnter={() => {
            clearTimeout(hoverTimer);
            hovered.set(true);
            props.drag.over(props.note.id);
          }}
          onMouseLeave={leave}
          onMouseMove={() => props.drag.over(props.note.id)}
          onPress={() => {
            if (Date.now() > suppressUntil && !props.drag.suppressed())
              void model.copy(props.note);
          }}
          style={{
            backgroundColor: props.note.color,
            borderRadius: 8,
            paddingTop: 10,
            paddingBottom: 9,
            paddingLeft: 10,
            paddingRight: 28,
            borderWidth: 0,
            opacity: props.drag.active() === props.note.id ? 0.55 : 1,
            boxShadow: props.note.pinned
              ? "inset 0 0 0 1px #007aff47, 0 8px 24px #1f232814"
              : "inset 0 0 0 1px #ffffff80, 0 8px 24px #1f232814",
          }}
        >
          <If when={!!body()}>
            <Text
              style={{
                position: "absolute",
                left: 10,
                right: 28,
                top: 10,
                opacity: 0,
                fontSize: 13.44,
                lineHeight: 22,
                pointerEvents: "none",
              }}
              onTextLayout={({ lines }) => {
                const count = Math.max(1, lines.length);
                if (count !== totalLines()) totalLines.set(count);
              }}
            >
              {body()}
            </Text>
            <Text
              style={{
                numberOfLines: visibleLines(),
                textOverflow: "ellipsis",
                fontSize: 13.44,
                lineHeight: 22,
                height: visibleLines() * 22,
                color: C.foreground,
              }}
            >
              {body()}
            </Text>
          </If>
          <If when={images().length > 0}>
            <View
              style={{
                flexDirection: "row",
                gap: 6,
                marginTop: body() ? 8 : 0,
                height: 58,
              }}
            >
              <Each of={images().slice(0, 4)}>
                {(attachment, index) => (
                  <AttachmentImage
                    attachment={attachment()}
                    onPress={() =>
                      model.preview.set({
                        images: images(),
                        index: index(),
                        editor: false,
                      })
                    }
                  />
                )}
              </Each>
            </View>
          </If>
          <If when={files().length > 0}>
            <View
              style={{ gap: 6, marginTop: body() || images().length ? 8 : 0 }}
            >
              <Each of={files().slice(0, 3)}>
                {(attachment) => (
                  <Button
                    onPress={() => {}}
                    accessibilityLabel={attachment().value}
                    style={{
                      flexDirection: "row",
                      alignItems: "center",
                      gap: 6,
                      height: 30,
                      paddingHorizontal: 8,
                      backgroundColor: "#ffffff66",
                      borderRadius: 5,
                    }}
                  >
                    <Icon name="file" />
                    <Text style={{ numberOfLines: 1, flex: 1, fontSize: 12 }}>
                      {attachment().name || attachment().value}
                    </Text>
                  </Button>
                )}
              </Each>
            </View>
          </If>
          <If when={props.note.pinned}>
            <View
              style={{
                pointerEvents: "none",
                position: "absolute",
                left: 2,
                top: 2,
                width: 14,
                height: 14,
                borderRadius: 7,
                backgroundColor: "#ffffff6b",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Icon name="pin" size={8} color={C.primary} />
            </View>
          </If>
          <If when={hovered() || motion.shown()}>
            <View
              style={{
                position: "absolute",
                top: "50%",
                right: 2,
                width: actionsOpen() ? 160 : 20,
                height: 30,
                transform: [{ translateY: -15 }],
                zIndex: 5,
              }}
              onMouseEnter={() => {
                clearTimeout(hoverTimer);
                hovered.set(true);
                actionsOpen.set(true);
              }}
              onMouseLeave={leave}
            >
              <Button
                accessibilityLabel={model.t().moreActions}
                testID={`actions-${props.note.id}`}
                onPress={() => actionsOpen.set(!actionsOpen())}
                style={{
                  position: "absolute",
                  right: 0,
                  top: 5,
                  width: 20,
                  height: 20,
                  borderRadius: 10,
                  backgroundColor: actionsOpen() ? "#ffffffc7" : "#ffffff75",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Icon
                  name="left"
                  size={12}
                  color={actionsOpen() ? C.primary : "#1d1d1f94"}
                />
              </Button>
              <If when={motion.shown()}>
                <View
                  style={{
                    position: "absolute",
                    right: 22,
                    top: -2,
                    flexDirection: "row",
                    gap: 5,
                    padding: 4,
                    borderWidth: 1,
                    borderColor: "#3c3c4314",
                    borderRadius: 12,
                    backgroundColor: "#ffffffdb",
                    boxShadow: "0 10px 24px #1f23281f",
                    opacity: motion.opacity,
                    transform: [{ translateX: motion.offset }],
                  }}
                >
                  <Action
                    compact
                    iconOnly
                    icon="pin"
                    label={props.note.pinned ? model.t().unpin : model.t().pin}
                    selected={props.note.pinned}
                    onPress={() => {
                      actionsOpen.set(false);
                      void model.changeNote(props.note, {
                        pinned: !props.note.pinned,
                      });
                    }}
                  />
                  <Action
                    compact
                    iconOnly
                    icon="edit"
                    label={model.t().edit}
                    onPress={() => {
                      actionsOpen.set(false);
                      model.openEditor(props.note);
                    }}
                  />
                  <Popover
                    open={colorOpen()}
                    onOpenChange={({ open }) => {
                      colorOpen.set(open);
                      if (!open) actionsOpen.set(false);
                    }}
                    onCancel={() => {
                      colorOpen.set(false);
                      actionsOpen.set(false);
                    }}
                    placement="bottom"
                    offset={6}
                    popupStyle={{
                      padding: 6,
                      borderWidth: 1,
                      borderColor: "#1d273524",
                      borderRadius: 8,
                      backgroundColor: "#ffffffeb",
                      boxShadow: "0 16px 34px #1f232829",
                    }}
                    popupClass="transition-all duration-150 ease-out"
                    popupStartingStyle={{
                      opacity: 0,
                      transform: [{ translateY: 6 }],
                    }}
                    popupEndingStyle={{
                      opacity: 0,
                      transform: [{ translateY: 6 }],
                    }}
                    trigger={
                      <Action
                        compact
                        iconOnly
                        icon="palette"
                        label={model.t().color}
                        onPress={() => {}}
                      />
                    }
                  >
                    <ColorPalette
                      compact
                      value={props.note.color}
                      testID="note-palette"
                      onChange={(color) => {
                        colorOpen.set(false);
                        actionsOpen.set(false);
                        void model.changeNote(props.note, { color });
                      }}
                    />
                  </Popover>
                  <Action
                    compact
                    iconOnly
                    icon="copy"
                    label={model.t().copy}
                    onPress={() => {
                      actionsOpen.set(false);
                      void model.copy(props.note);
                    }}
                  />
                  <Action
                    compact
                    iconOnly
                    icon="trash"
                    danger
                    label={model.t().delete}
                    onPress={() => {
                      actionsOpen.set(false);
                      void model.remove(props.note.id);
                    }}
                  />
                </View>
              </If>
            </View>
            <If when={totalLines() > minimum()}>
              <View
                {...resize.panHandlers}
                testID={`resize-${props.note.id}`}
                accessibilityLabel={model.t().resize}
                style={{
                  position: "absolute",
                  bottom: 1,
                  right: 8,
                  width: 24,
                  height: 12,
                  alignItems: "center",
                  justifyContent: "center",
                  cursor: "ns-resize",
                }}
              >
                <Icon name="grip" size={12} color="#1f293785" />
              </View>
            </If>
          </If>
        </View>
      }
    >
      <MenuItem
        onSelect={() => {
          void model.copy(props.note);
        }}
        style={{ padding: 8, borderRadius: 4 }}
        hoverStyle={{ backgroundColor: "#007aff22" }}
      >
        <Text style={{ fontSize: 13 }}>{model.t().copy}</Text>
      </MenuItem>
      <MenuItem
        onSelect={() => model.openEditor(props.note)}
        style={{ padding: 8, borderRadius: 4 }}
        hoverStyle={{ backgroundColor: "#007aff22" }}
      >
        <Text style={{ fontSize: 13 }}>{model.t().edit}</Text>
      </MenuItem>
      <MenuItem
        onSelect={() => {
          void model.changeNote(props.note, { pinned: !props.note.pinned });
        }}
        style={{ padding: 8, borderRadius: 4 }}
        hoverStyle={{ backgroundColor: "#007aff22" }}
      >
        <Text style={{ fontSize: 13 }}>
          {props.note.pinned ? model.t().unpin : model.t().pin}
        </Text>
      </MenuItem>
      <MenuSeparator
        style={{ height: 1, backgroundColor: C.border, marginVertical: 3 }}
      />
      <MenuItem
        onSelect={() => {
          void model.remove(props.note.id);
        }}
        style={{ padding: 8, borderRadius: 4 }}
        hoverStyle={{ backgroundColor: "#ff3b3022" }}
      >
        <Text style={{ fontSize: 13, color: C.danger }}>
          {model.t().delete}
        </Text>
      </MenuItem>
    </ContextMenu>
  );
}
