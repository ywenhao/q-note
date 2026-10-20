import { cleanup, signal, Text, View, type GestureResponderEvent } from "nsh";
import { keymap } from "nsh/keymap";
import { windowSize } from "nsh/device";
import { AttachmentImage, Button, Motion } from "./components";
import type { AppState } from "./state";
import type { NoteAttachment } from "./types";

export function Preview(props: {
  state: AppState;
  items: NoteAttachment[];
  initialIndex: number;
  closing?: boolean;
  onClose: () => void;
}) {
  const s = props.state,
    index = signal(
      Math.max(0, Math.min(props.items.length - 1, props.initialIndex)),
    ),
    scale = signal(1),
    offset = signal({ x: 0, y: 0 }),
    natural = signal({ width: 800, height: 600 });
  let drag: { x: number; y: number; offset: { x: number; y: number } } | null =
      null,
    lastClick = 0;
  const fitted = () => {
    const size = natural(),
      ratio = Math.min(
        1,
        Math.min(1120, windowSize().width - 88) / size.width,
        (windowSize().height - 108) / size.height,
      );
    return { width: size.width * ratio, height: size.height * ratio };
  };
  const reset = () => {
    scale.set(1);
    offset.set({ x: 0, y: 0 });
    drag = null;
  };
  const show = (delta: number) => {
    if (props.items.length < 2) return;
    index.set((index() + delta + props.items.length) % props.items.length);
    reset();
  };
  const pan = (x: number, y: number) => {
    if (scale() <= 1) {
      offset.set({ x: 0, y: 0 });
      return;
    }
    const size = fitted(),
      maxX = Math.max(0, (size.width * scale() - windowSize().width) / 2) + 80,
      maxY =
        Math.max(0, (size.height * scale() - windowSize().height) / 2) + 80;
    offset.set({
      x: Math.max(-maxX, Math.min(maxX, x)),
      y: Math.max(-maxY, Math.min(maxY, y)),
    });
  };
  const zoom = (delta: number) => {
    scale.set(Math.max(0.5, Math.min(6, scale() + delta)));
    pan(offset().x, offset().y);
  };
  const tap = () => {
    if (Date.now() - lastClick < 300) reset();
    lastClick = Date.now();
  };
  keymap.bind(
    {
      escape: props.onClose,
      "[": () => show(-1),
      pageup: () => show(-1),
      "]": () => show(1),
      pagedown: () => show(1),
      "=": () => zoom(0.25),
      plus: () => zoom(0.25),
      "shift+plus": () => zoom(0.25),
      minus: () => zoom(-0.25),
      "0": reset,
      left: () => (scale() > 1 ? pan(offset().x + 36, offset().y) : show(-1)),
      right: () => (scale() > 1 ? pan(offset().x - 36, offset().y) : show(1)),
      up: () => pan(offset().x, offset().y + 36),
      down: () => pan(offset().x, offset().y - 36),
    },
    { when: "always", repeat: true },
  );
  s.onEvent((e) => {
    if (e.kind === "wheel" && !props.closing)
      zoom((e.delta ?? 0) > 0 ? -0.25 : 0.25);
  });
  void s.call("preview", { window: s.role, open: true });
  cleanup(() => void s.call("preview", { window: s.role, open: false }));
  function move(e: GestureResponderEvent) {
    e.stopPropagation();
    if (drag)
      pan(drag.offset.x + e.pageX - drag.x, drag.offset.y + e.pageY - drag.y);
  }
  const buttonStyle = {
    width: 28,
    height: 28,
    minWidth: 28,
    borderWidth: 0,
    borderRadius: 99,
    backgroundColor: "transparent",
  } as const;
  return (
    <Motion
      closing={props.closing}
      style={{
        position: "absolute",
        top: 0,
        bottom: 0,
        left: 0,
        right: 0,
        zIndex: 90,
        backgroundColor: "rgba(24,33,47,0.72)",
        overflow: "hidden",
      }}
    >
      <View
        onPress={props.onClose}
        style={{ position: "absolute", top: 0, bottom: 0, left: 0, right: 0 }}
      />
      <View
        style={{
          position: "absolute",
          top: 54,
          bottom: 42,
          left: 44,
          right: 44,
          alignItems: "center",
          justifyContent: "center",
        }}
        pointerEvents="box-none"
      >
        <View
          onPress={tap}
          onStartShouldSetResponder={() => scale() > 1}
          onResponderGrant={(e) => {
            drag = { x: e.pageX, y: e.pageY, offset: offset() };
          }}
          onResponderMove={move}
          onResponderRelease={(e) => {
            if (drag && Math.hypot(e.pageX - drag.x, e.pageY - drag.y) < 4)
              tap();
            drag = null;
          }}
          onResponderTerminate={() => (drag = null)}
          onResponderTerminationRequest={() => false}
          style={{
            width: fitted().width,
            height: fitted().height,
            borderRadius: 8,
            boxShadow: "0 24px 70px rgba(0,0,0,0.38)",
            transform: [
              { translateX: offset().x },
              { translateY: offset().y },
              { scale: scale() },
            ],
            transitionProperty: drag ? "none" : "transform",
            transitionDuration: 100,
            transitionTimingFunction: "ease-out",
            cursor: scale() > 1 ? "grab" : "zoom-in",
          }}
        >
          <AttachmentImage
            state={s}
            attachment={props.items[index()]!}
            onLoad={(size) => natural.set(size)}
            mode="contain"
            style={{
              width: "100%",
              height: "100%",
              borderRadius: 8,
              backgroundColor: "transparent",
            }}
          />
        </View>
      </View>
      <View
        style={{
          position: "absolute",
          top: 12,
          right: 12,
          flexDirection: "row",
          alignItems: "center",
          gap: 4,
          zIndex: 3,
          padding: 4,
          borderWidth: 1,
          borderColor: "rgba(255,255,255,0.36)",
          borderRadius: 99,
          backgroundColor: "rgba(255,255,255,0.22)",
        }}
      >
        <Text
          style={{
            color: "rgba(255,255,255,0.88)",
            fontSize: 10.92,
            minWidth: 42,
            textAlign: "center",
          }}
        >
          {index() + 1} / {props.items.length}
        </Text>
        <Text
          style={{
            color: "rgba(255,255,255,0.88)",
            fontSize: 10.92,
            minWidth: 42,
            textAlign: "center",
          }}
        >
          {Math.round(scale() * 100)}%
        </Text>
        <Button
          iconSize={14}
          iconColor="white"
          small
          label={s.t().zoomOut}
          icon="ZoomOut"
          onPress={() => zoom(-0.25)}
          style={buttonStyle}
        />
        <Button
          iconSize={14}
          iconColor="white"
          small
          label={s.t().zoomIn}
          icon="ZoomIn"
          onPress={() => zoom(0.25)}
          style={buttonStyle}
        />
        <Button
          iconSize={14}
          iconColor="white"
          small
          label={s.t().resetView}
          icon="RotateCcw"
          onPress={reset}
          style={buttonStyle}
        />
        <Button
          iconSize={14}
          iconColor="white"
          small
          label={s.t().cancel}
          icon="X"
          onPress={props.onClose}
          style={buttonStyle}
        />
      </View>
      <If when={props.items.length > 1}>
        <View
          style={{
            position: "absolute",
            left: 12,
            top: windowSize().height / 2 - 23,
            zIndex: 3,
          }}
        >
          <Button
            iconSize={14}
            iconColor="white"
            label={s.t().previousImage}
            icon="ChevronLeft"
            onPress={() => show(-1)}
            style={{
              ...buttonStyle,
              width: 34,
              height: 46,
              backgroundColor: "rgba(255,255,255,0.18)",
            }}
          />
        </View>
        <View
          style={{
            position: "absolute",
            right: 12,
            top: windowSize().height / 2 - 23,
            zIndex: 3,
          }}
        >
          <Button
            iconSize={14}
            iconColor="white"
            label={s.t().nextImage}
            icon="ChevronRight"
            onPress={() => show(1)}
            style={{
              ...buttonStyle,
              width: 34,
              height: 46,
              backgroundColor: "rgba(255,255,255,0.18)",
            }}
          />
        </View>
      </If>
    </Motion>
  );
}
