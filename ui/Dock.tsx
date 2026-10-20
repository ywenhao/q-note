import { signal, View } from "nsh";
import { Menu } from "nsh/desktop";
import { QMark } from "./components";
import type { AppState } from "./state";

export function Dock(props: { state: AppState }) {
  const s = props.state,
    visible = signal(false);
  let start: { x: number; y: number } | null = null,
    last = { x: 0, y: 0 },
    dragging = false,
    suppressUntil = 0;
  setTimeout(() => visible.set(true), 16);
  function menu() {
    void Menu.popup([
      {
        label: s.snapshot().data.settings.alwaysOnTop
          ? s.t().alwaysOff
          : s.t().alwaysOn,
        onSelect: () => void s.call("topmost"),
      },
      { label: s.t().switchLanguage, onSelect: () => void s.call("language") },
      {
        label: s.t().switchMainWindow,
        onSelect: () => void s.call("show", { window: "main" }),
      },
      { label: s.t().quit, onSelect: () => void s.call("quit") },
    ]);
  }
  s.onEvent((e) => {
    if (e.kind === "context") menu();
    if (e.kind === "showDock") {
      visible.set(false);
      setTimeout(() => visible.set(true), 16);
    }
  });
  return (
    <View
      accessibilityRole="button"
      accessibilityLabel={s.t().switchMainWindow}
      onPress={() => {
        if (Date.now() >= suppressUntil)
          void s.call("show", { window: "main" });
      }}
      onHoverIn={() => void s.call("hoverDock", { inside: true })}
      onHoverOut={() => void s.call("hoverDock", { inside: false })}
      onTouchStart={(e) => {
        start = { x: e.pageX, y: e.pageY };
        last = { ...start };
        dragging = false;
        void s.call("beginDockDrag");
      }}
      onTouchMove={(e) => {
        if (!start) return;
        const dx = e.pageX - start.x,
          dy = e.pageY - start.y;
        if (Math.hypot(dx, dy) > 4 || dragging) {
          dragging = true;
          suppressUntil = Date.now() + 300;
          void s.call("dragDock", {
            dx: e.pageX - last.x,
            dy: e.pageY - last.y,
          });
          last = { x: e.pageX, y: e.pageY };
        }
      }}
      onTouchEnd={() => {
        if (dragging) suppressUntil = Date.now() + 300;
        void s.call("endDockDrag", { dragged: dragging });
        start = null;
        dragging = false;
      }}
      style={{
        width: 30,
        height: 30,
        borderRadius: 99,
        overflow: "hidden",
        backgroundColor: "transparent",
        opacity: visible() ? 1 : 0,
        transitionProperty: "opacity",
        transitionDuration: 180,
        cursor: "grab",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <QMark size={30} />
    </View>
  );
}
