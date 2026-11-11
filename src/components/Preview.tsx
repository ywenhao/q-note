import {
  effect,
  Image,
  keymap,
  PanResponder,
  Scroll,
  signal,
  Text,
  View,
} from "nsh";
import { Dialog } from "nsh/base";
import { getAttachmentSrc } from "../lib/images";
import type { NoteModel } from "../model";
import { Action, presence } from "../ui";

export function ImagePreview(props: { model: NoteModel; editor?: boolean }) {
  const m = props.model;
  const open = () => !!m.preview() && m.preview()!.editor === !!props.editor;
  const motion = presence(open);
  const src = signal("");
  const zoom = signal(1);
  const position = signal({ x: 0, y: 0 });
  const viewport = signal({ width: 300, height: 300 });
  const natural = signal({ width: 300, height: 300 });
  let panStart = { x: 0, y: 0 };
  let lastWheel = 50000;
  let wheel = 0;
  let dragging = false;

  const reset = () => {
    zoom.set(1);
    position.set({ x: 0, y: 0 });
  };
  const close = () => m.preview.set(null);
  const changeZoom = (step: number) => {
    zoom.set(Math.max(0.5, Math.min(6, Math.round((zoom() + step) * 4) / 4)));
    if (zoom() <= 1) position.set({ x: 0, y: 0 });
  };
  const move = (step: number) => {
    const current = m.preview();
    if (!current || current.images.length < 2) return;
    m.preview.set({
      ...current,
      index:
        (current.index + step + current.images.length) % current.images.length,
    });
  };
  const pan = (x: number, y: number) =>
    position.update((p) => ({ x: p.x + x, y: p.y + y }));

  effect(() => {
    const current = open() ? m.preview() : null;
    const attachment = current?.images[current.index];
    let active = true;
    if (!attachment) return;
    reset();
    src.set("");
    void getAttachmentSrc(attachment)
      .then(async (value) => {
        if (!active) return;
        src.set(value);
        try {
          const size = await Image.getSize(value);
          if (active) natural.set(size);
        } catch {
          if (active) natural.set({ width: 300, height: 300 });
        }
      })
      .catch((error) => {
        console.error(error);
        if (active) m.notify(m.t().loadFailed);
      });
    return () => {
      active = false;
    };
  });

  effect(() => {
    if (!open()) return;
    return keymap.bind(
      {
        escape: close,
        plus: () => changeZoom(0.25),
        "shift+plus": () => changeZoom(0.25),
        minus: () => changeZoom(-0.25),
        "0": reset,
        "[": () => move(-1),
        pageup: () => move(-1),
        "]": () => move(1),
        pagedown: () => move(1),
        left: () => (zoom() > 1 ? pan(40, 0) : move(-1)),
        right: () => (zoom() > 1 ? pan(-40, 0) : move(1)),
        up: () => {
          if (zoom() > 1) pan(0, 40);
        },
        down: () => {
          if (zoom() > 1) pan(0, -40);
        },
      },
      { repeat: true, when: "always" },
    );
  });

  const responder = PanResponder.create({
    onMoveShouldSetPanResponder: (_event, state) =>
      zoom() > 1 && Math.hypot(state.dx, state.dy) > 4,
    onPanResponderGrant: () => {
      dragging = true;
      panStart = position();
    },
    onPanResponderMove: (_e, state) =>
      position.set({ x: panStart.x + state.dx, y: panStart.y + state.dy }),
    onPanResponderRelease: () => {
      dragging = false;
    },
    onPanResponderTerminate: () => {
      dragging = false;
    },
  });
  const fitted = () => {
    const scale = Math.min(
      (viewport().width - 24) / natural().width,
      (viewport().height - 24) / natural().height,
      1,
    );
    return { width: natural().width * scale, height: natural().height * scale };
  };

  return (
    <Dialog
      open={motion.shown()}
      animation="none"
      testID="image-preview"
      closeOnEscape={false}
      onCancel={close}
      style={{ flex: 1, backgroundColor: "#101318ed", opacity: motion.opacity }}
    >
      <View style={{ flex: 1, transform: [{ translateY: motion.offset }] }}>
        <View
          style={{
            height: 42,
            paddingHorizontal: 8,
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "space-between",
            backgroundColor: "#fffdf4",
          }}
        >
          <Text
            style={{ fontSize: 12 }}
          >{`${(m.preview()?.index ?? 0) + 1} / ${m.preview()?.images.length ?? 0}`}</Text>
          <Text style={{ fontSize: 12 }}>{`${Math.round(zoom() * 100)}%`}</Text>
          <Action
            icon="close"
            iconOnly
            label={m.t().closePanel}
            onPress={close}
            testID="preview-close"
          />
        </View>
        <View
          onLayout={({ bounds }) => viewport.set(bounds.size)}
          style={{
            flex: 1,
            overflow: "hidden",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <If when={!!src()}>
            <Image
              src={src()}
              style={{
                pointerEvents: "none",
                objectFit: "contain",
                width: fitted().width,
                height: fitted().height,
                transform: [
                  { translateX: position().x },
                  { translateY: position().y },
                  { scale: zoom() },
                ],
              }}
            />
          </If>
          <Scroll
            {...responder.panHandlers}
            testID="preview-viewport"
            scrollbar={false}
            contentOffset={{ y: 50000 }}
            style={{
              position: "absolute",
              top: 0,
              right: 0,
              bottom: 0,
              left: 0,
            }}
            onPress={(event) => {
              if (event.clickCount === 2) {
                reset();
                return;
              }
              const dx = Math.abs(
                event.position.x - viewport().width / 2 - position().x,
              );
              const dy = Math.abs(
                event.position.y - viewport().height / 2 - position().y,
              );
              if (
                dx > (fitted().width * zoom()) / 2 ||
                dy > (fitted().height * zoom()) / 2
              )
                close();
            }}
            onScroll={({ offset }) => {
              const delta = offset.y - lastWheel;
              lastWheel = offset.y;
              if (dragging || Math.abs(delta) > 500) return;
              wheel += delta;
              if (Math.abs(wheel) >= 40) {
                changeZoom(wheel < 0 ? 0.25 : -0.25);
                wheel = 0;
              }
            }}
          >
            <View style={{ height: 100000, width: "100%" }} />
          </Scroll>
        </View>
        <View
          style={{
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "center",
            gap: 4,
            height: 46,
            backgroundColor: "#fffdf4",
          }}
        >
          <Action
            icon="left"
            iconOnly
            label={m.t().previousImage}
            disabled={(m.preview()?.images.length ?? 0) < 2}
            onPress={() => move(-1)}
            testID="preview-previous"
          />
          <Action
            icon="minus"
            iconOnly
            label={m.t().zoomOut}
            disabled={zoom() <= 0.5}
            onPress={() => changeZoom(-0.25)}
            testID="preview-minus"
          />
          <Action
            icon="reset"
            iconOnly
            label={m.t().resetView}
            onPress={reset}
            testID="preview-reset"
          />
          <Action
            icon="plus"
            iconOnly
            label={m.t().zoomIn}
            disabled={zoom() >= 6}
            onPress={() => changeZoom(0.25)}
            testID="preview-plus"
          />
          <Action
            icon="right"
            iconOnly
            label={m.t().nextImage}
            disabled={(m.preview()?.images.length ?? 0) < 2}
            onPress={() => move(1)}
            testID="preview-next"
          />
        </View>
      </View>
    </Dialog>
  );
}
