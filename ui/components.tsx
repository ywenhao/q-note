import {
  cleanup,
  effect,
  signal,
  Text,
  View,
  type JSX,
  type ViewStyle,
} from "nsh";
import { Image } from "nsh/image";
import { Canvas, Path2D } from "nsh/canvas";
import { icons } from "./icons";
import type { AppState } from "./state";
import type { NoteAttachment } from "./types";
import { mark } from "./mark";
import { presence } from "./presence";

export function Icon(props: {
  name: keyof typeof icons;
  size?: number;
  color?: string;
}) {
  return (
    <Canvas
      pointerEvents="none"
      style={{ width: props.size ?? 13, height: props.size ?? 13 }}
      draw={(ctx, size) => {
        ctx.scale(size.width / 24, size.height / 24);
        ctx.strokeStyle = props.color ?? "#414144";
        ctx.lineWidth = 1.8;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";
        for (const match of icons[props.name].matchAll(
          /<(path|circle|line|rect|polyline|polygon)\b([^>]*)\/?\s*>/g,
        )) {
          const attrs = Object.fromEntries(
            [...match[2]!.matchAll(/([\w-]+)="([^"]*)"/g)].map((a) => [
              a[1]!,
              a[2]!,
            ]),
          );
          const number = (key: string) => Number(attrs[key] ?? 0);
          if (match[1] === "path") {
            ctx.stroke(new Path2D(attrs.d));
            continue;
          }
          ctx.beginPath();
          if (match[1] === "circle")
            ctx.arc(number("cx"), number("cy"), number("r"), 0, Math.PI * 2);
          else if (match[1] === "line") {
            ctx.moveTo(number("x1"), number("y1"));
            ctx.lineTo(number("x2"), number("y2"));
          } else if (match[1] === "rect")
            ctx.roundRect(
              number("x"),
              number("y"),
              number("width"),
              number("height"),
              number("rx"),
            );
          else {
            const points = (attrs.points ?? "")
              .trim()
              .split(/[\s,]+/)
              .map(Number);
            ctx.moveTo(points[0] ?? 0, points[1] ?? 0);
            for (let i = 2; i < points.length; i += 2)
              ctx.lineTo(points[i]!, points[i + 1]!);
            if (match[1] === "polygon") ctx.closePath();
          }
          ctx.stroke();
        }
      }}
    />
  );
}

export function QMark(props: { size: number }) {
  return (
    <Image
      src={mark}
      resizeMode="contain"
      style={{
        width: props.size,
        height: props.size,
        borderRadius: props.size / 2,
      }}
    />
  );
}

export function Button(props: {
  label: string;
  icon?: keyof typeof icons;
  iconSize?: number;
  iconColor?: string;
  text?: string;
  onPress: () => void;
  disabled?: boolean;
  active?: boolean;
  danger?: boolean;
  primary?: boolean;
  small?: boolean;
  style?: ViewStyle;
  testID?: string;
}) {
  return (
    <View
      testID={props.testID}
      accessibilityRole="button"
      accessibilityLabel={props.label}
      disabled={props.disabled}
      onPress={props.onPress}
      class={
        props.primary || (props.danger && !!props.text)
          ? "hover:opacity-90 active:opacity-80 focus-visible:ring-2 focus-visible:ring-blue-500"
          : "hover:bg-white/70 active:bg-white/90 focus-visible:ring-2 focus-visible:ring-blue-500"
      }
      style={{
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "center",
        gap: 5,
        height: props.small ? 24 : 28,
        minWidth: props.small ? 24 : 26,
        paddingHorizontal: props.text ? 8 : 5,
        borderRadius: 8,
        borderWidth: 1,
        borderColor: "rgba(60,60,67,0.08)",
        backgroundColor: props.primary
          ? "rgba(29,29,31,0.92)"
          : props.danger && props.text
            ? "#ff3b30"
            : "rgba(255,255,255,0.46)",
        opacity: props.disabled ? 0.46 : 1,
        cursor: props.disabled ? "not-allowed" : "pointer",
        ...props.style,
      }}
    >
      <If when={!!props.icon}>
        <Icon
          name={props.icon!}
          size={props.iconSize}
          color={
            props.iconColor ??
            (props.primary || (props.danger && !!props.text)
              ? "white"
              : props.danger
                ? "#ff3b30"
                : props.active
                  ? "#007aff"
                  : "#414144")
          }
        />
      </If>
      <If when={!!props.text}>
        <Text
          allowFontScaling={false}
          numberOfLines={1}
          style={{
            fontSize: 13,
            color: props.primary || props.danger ? "white" : "#1f2328",
          }}
        >
          {props.text}
        </Text>
      </If>
    </View>
  );
}

export function TitleBar(props: { state: AppState; title: string }) {
  const s = props.state;
  return (
    <View
      style={{
        height: 26,
        flexShrink: 0,
        marginBottom: 4,
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 6,
      }}
    >
      <View
        desktop={{ windowDrag: true }}
        style={{
          flex: 1,
          flexDirection: "row",
          alignItems: "center",
          gap: 7,
          height: 26,
          cursor: "grab",
        }}
      >
        <If when={s.role !== "editor"}>
          <QMark size={22} />
        </If>
        <Text
          allowFontScaling={false}
          numberOfLines={1}
          style={{ fontSize: 15.12, fontWeight: "600", color: "#1d1d1f" }}
        >
          {props.title}
        </Text>
      </View>
      <View
        style={{
          flexDirection: "row",
          gap: 2,
          padding: 2,
          borderWidth: 1,
          borderColor: "rgba(255,255,255,0.42)",
          borderRadius: 99,
          backgroundColor: "rgba(255,255,255,0.26)",
        }}
      >
        <Button
          label={
            s.snapshot().data.settings.alwaysOnTop
              ? s.t().alwaysOff
              : s.t().alwaysOn
          }
          icon="Pin"
          iconSize={10}
          active={s.snapshot().data.settings.alwaysOnTop}
          onPress={() => void s.call("topmost")}
          style={{
            width: 18,
            height: 18,
            minWidth: 18,
            borderWidth: 0,
            borderRadius: 99,
            backgroundColor: "transparent",
          }}
        />
        <Button
          label={s.t().minimize}
          icon="Minus"
          iconSize={10}
          onPress={() => void s.call("minimize", { window: s.role })}
          style={{
            width: 18,
            height: 18,
            minWidth: 18,
            borderWidth: 0,
            borderRadius: 99,
            backgroundColor: "transparent",
          }}
        />
        <Button
          label={s.t().closePanel}
          icon="X"
          iconSize={10}
          onPress={() => void s.call("close", { window: s.role })}
          style={{
            width: 18,
            height: 18,
            minWidth: 18,
            borderWidth: 0,
            borderRadius: 99,
            backgroundColor: "transparent",
          }}
        />
      </View>
    </View>
  );
}

export function Palette(props: {
  state: AppState;
  color: string;
  compact?: boolean;
  onChange: (color: string) => void;
}) {
  return (
    <View
      style={{
        flexDirection: "row",
        flexWrap: "wrap",
        gap: props.compact ? 5 : 7,
      }}
    >
      <Each of={props.state.snapshot().palette}>
        {(color) => (
          <View
            accessibilityRole="button"
            accessibilityLabel={color()}
            onPress={() => props.onChange(color())}
            style={{
              width: 20,
              height: 20,
              borderRadius: 99,
              backgroundColor: color(),
              borderWidth: 1,
              borderColor:
                props.color === color()
                  ? "rgba(29,39,53,0.45)"
                  : "rgba(29,39,53,0.16)",
              boxShadow:
                props.color === color()
                  ? "0 0 0 3px rgba(29,39,53,0.08)"
                  : "none",
              transform: [{ scale: props.color === color() ? 1.08 : 1 }],
              cursor: "pointer",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <If when={props.color === color() && props.compact}>
              <Icon name="Check" size={11} />
            </If>
          </View>
        )}
      </Each>
    </View>
  );
}

export function AttachmentImage(props: {
  state: AppState;
  attachment: NoteAttachment;
  style?: ViewStyle;
  mode?: "cover" | "contain";
  onLoad?: (size: { width: number; height: number }) => void;
}) {
  const src = signal("");
  const failed = signal(false);
  let previous = "",
    request = "";
  props.state.onEvent((event) => {
    if (event.kind === "imageReady" && event.request === request) {
      src.set(event.value ?? "");
      failed.set(!event.value);
    }
  });
  effect(() => {
    const attachment = props.attachment,
      key = attachment.source + ":" + attachment.value;
    if (key === previous) return;
    previous = key;
    failed.set(false);
    src.set("");
    if (
      attachment.source !== "path" &&
      !/\.svg(?:[?#]|$)|^data:image\/svg/i.test(attachment.value)
    ) {
      src.set(attachment.value);
    } else {
      request = `${attachment.id}-${Date.now()}-${Math.random()}`;
      void props.state.call("imageSrc", {
        ...attachment,
        request,
        window: props.state.role,
      });
    }
  });
  return (
    <View
      style={{
        overflow: "hidden",
        backgroundColor: "rgba(255,255,255,0.46)",
        ...props.style,
      }}
    >
      <If when={!!src() && !failed()}>
        <Image
          src={src()}
          resizeMode={props.mode ?? "cover"}
          onLoad={(size) => props.onLoad?.(size)}
          onError={() => failed.set(true)}
          style={{ width: "100%", height: "100%" }}
        />
      </If>
      <If when={failed()}>
        <View
          style={{ flex: 1, alignItems: "center", justifyContent: "center" }}
        >
          <Icon name="ImagePlus" size={18} color="#888" />
        </View>
      </If>
    </View>
  );
}

export function Motion(props: {
  children?: JSX.Element;
  style?: ViewStyle;
  closing?: boolean;
  pointerEvents?: "auto" | "none" | "box-none" | "box-only";
}) {
  const shown = signal(false);
  const timer = setTimeout(() => shown.set(true), 16);
  cleanup(() => clearTimeout(timer));
  return (
    <View
      pointerEvents={props.pointerEvents}
      style={{
        opacity: shown() && !props.closing ? 1 : 0,
        transform: [{ translateY: shown() && !props.closing ? 0 : 6 }],
        transitionProperty: "opacity, transform",
        transitionDuration: 150,
        transitionTimingFunction: "ease",
        ...props.style,
      }}
    >
      {props.children}
    </View>
  );
}

export function Modal(props: {
  onClose: () => void;
  children?: JSX.Element;
  width?: number;
  closing?: boolean;
  compact?: boolean;
}) {
  return (
    <Motion
      closing={props.closing}
      style={{
        position: "absolute",
        top: 0,
        bottom: 0,
        left: 0,
        right: 0,
        zIndex: 50,
        alignItems: "center",
        justifyContent: "center",
        padding: 18,
        backgroundColor: "rgba(29,29,31,0.24)",
      }}
    >
      <View
        onPress={props.onClose}
        style={{ position: "absolute", top: 0, bottom: 0, left: 0, right: 0 }}
      />
      <View
        testID="dialog"
        style={{
          width: "100%",
          maxWidth: props.width ?? 420,
          padding: props.compact ? 8 : 18,
          gap: props.compact ? 7 : 12,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: "rgba(255,255,255,0.58)",
          backgroundColor: "rgba(255,253,244,0.96)",
          boxShadow: "0 16px 38px rgba(31,35,40,0.16)",
        }}
      >
        {props.children}
      </View>
    </Motion>
  );
}

export function Toast(props: { state: AppState }) {
  const s = props.state;
  const shown = presence(() => s.toast());
  return (
    <If when={shown.value()}>
      <Motion
        closing={shown.closing()}
        pointerEvents="none"
        style={{
          position: "absolute",
          bottom: 42,
          left: 16,
          right: 16,
          zIndex: 100,
          alignItems: "flex-end",
        }}
      >
        <View
          pointerEvents="none"
          style={{
            maxWidth: 320,
            flexDirection: "row",
            alignItems: "center",
            gap: 8,
            paddingHorizontal: 12,
            paddingVertical: 7,
            borderRadius: 8,
            borderWidth: 1,
            borderColor: "rgba(255,255,255,0.58)",
            backgroundColor: "rgba(255,255,255,0.92)",
            boxShadow: "0 12px 30px rgba(31,35,40,0.14)",
          }}
        >
          <Icon
            name={shown.value()?.error ? "CircleAlert" : "Check"}
            color={shown.value()?.error ? "#ff3b30" : "#34c759"}
          />
          <Text
            numberOfLines={1}
            ellipsizeMode="tail"
            style={{ fontSize: 13, color: "#1f2328" }}
          >
            {String(
              s.t()[shown.value()!.key as keyof ReturnType<typeof s.t>] ??
                shown.value()!.key,
            )}
          </Text>
        </View>
      </Motion>
    </If>
  );
}
