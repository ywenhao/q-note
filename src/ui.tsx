import { Buffer } from "buffer";
import {
  cleanup,
  createAnimatedValue,
  effect,
  Image,
  signal,
  Text,
  untracked,
  View,
  windowSize,
  type JSX,
} from "nsh";
import { Button, Dialog } from "nsh/base";
import { colors as C } from "./theme";

const paths = {
  plus: "M12 5v14M5 12h14",
  close: "m6 6 12 12M18 6 6 18",
  minus: "M5 12h14",
  pin: "m16 3 5 5-4 1-3 5 1 3-2 2-4-4-6 6 6-6-4-4 2-2 3 1 5-3z",
  copy: "M9 9h12v12H9zM5 15H3V3h12v2",
  edit: "m16 3 5 5-12 12-6 1 1-6zM14 5l5 5",
  trash: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7",
  settings:
    "M9 3h6l1 4 4 2v6l-4 2-1 4H9l-1-4-4-2V9l4-2zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
  palette:
    "M12 3a9 9 0 1 0 0 18h2a2 2 0 0 0 0-4h-1a1 1 0 0 1 0-2h3a5 5 0 0 0 5-5c0-4-4-7-9-7M7 9h.01M11 6h.01M16 8h.01",
  image: "M3 3h18v18H3zM3 17l6-6 4 4 3-3 5 5M16 7h.01",
  file: "M6 2h8l4 4v16H6zM14 2v5h5M9 12h6M9 16h6",
  left: "m15 5-7 7 7 7",
  right: "m9 5 7 7-7 7",
  check: "m5 12 4 4L19 6",
  reset: "M3 10a9 9 0 1 1 2 8M3 4v6h6",
  grip: "M8 8h.01M16 8h.01M8 16h.01M16 16h.01",
  download: "M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4",
  upload: "M12 15V3m-5 5 5-5 5 5M4 17v4h16v-4",
  languages:
    "m4 5 7 0M7 3v2M5 5c0 5 5 8 5 8M10 5c0 5-6 9-6 9M13 21l4-10 4 10M14.5 17h5",
  link: "M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-2 2M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l2-2",
} as const;
export type IconName = keyof typeof paths;
const iconCache = new Map<string, string>();

export function Icon(props: { name: IconName; size?: number; color?: string }) {
  const source = () => {
    const key = `${props.name}:${props.color || C.foreground}`;
    if (!iconCache.has(key))
      iconCache.set(
        key,
        `data:image/svg+xml;base64,${Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="${props.color || C.foreground}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="${paths[props.name]}"/></svg>`).toString("base64")}`,
      );
    return iconCache.get(key)!;
  };
  return (
    <Image
      src={source()}
      style={{
        width: props.size ?? 16,
        height: props.size ?? 16,
        flexShrink: 0,
      }}
    />
  );
}

export function Action(props: {
  label: string;
  icon?: IconName;
  onPress: () => void;
  primary?: boolean;
  danger?: boolean;
  compact?: boolean;
  iconOnly?: boolean;
  selected?: boolean;
  disabled?: boolean;
  testID?: string;
  grow?: boolean;
}) {
  const ink = () =>
    props.primary
      ? "#fff"
      : props.danger
        ? C.danger
        : props.selected
          ? C.primary
          : C.foreground;
  return (
    <Button
      testID={props.testID}
      accessibilityLabel={props.label}
      disabled={props.disabled}
      selected={props.selected}
      onPress={props.onPress}
      tooltip={
        <Text
          style={{
            backgroundColor: C.foreground,
            color: "#fff",
            padding: 6,
            borderRadius: 5,
            fontSize: 12,
          }}
        >
          {props.label}
        </Text>
      }
      style={{
        height: props.compact ? 24 : props.iconOnly || props.icon ? 26 : 28,
        minWidth: props.iconOnly ? (props.compact ? 24 : 26) : undefined,
        paddingHorizontal: props.iconOnly ? 5 : props.icon ? 6 : 10,
        flexDirection: "row",
        gap: 5,
        alignItems: "center",
        justifyContent: "center",
        borderRadius: 8,
        flexGrow: props.grow ? 1 : 0,
        flexShrink: 0,
        borderWidth: 1,
        borderColor: props.primary ? "#1d1d1f2e" : "#3c3c4314",
        backgroundColor: props.primary
          ? "#1d1d1feb"
          : props.selected
            ? "#1d1d1fe6"
            : props.icon
              ? "#ffffff75"
              : "#ffffffb8",
      }}
      hoverStyle={{
        backgroundColor: props.primary ? "#1d1d1fbf" : "#ffffffa3",
      }}
      activeStyle={{ opacity: 0.65 }}
      focusStyle={{ borderWidth: 1, borderColor: C.primary }}
      disabledStyle={{ opacity: 0.46 }}
    >
      <If when={props.icon}>
        <Icon
          name={props.icon!}
          size={13}
          color={props.selected ? "#fff" : ink()}
        />
      </If>
      <If when={!props.iconOnly}>
        <Text style={{ fontSize: 14, color: ink() }}>{props.label}</Text>
      </If>
    </Button>
  );
}

export function QMark(props: { size?: number }) {
  return (
    <Image
      src={`data:image/svg+xml;base64,${Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle fill="#ffd150" cx="50" cy="50" r="48"/><path fill="#18212f" d="M50 23C33.5 23 23 34.5 23 50.3C23 66.4 34.5 77 50.4 77C56.6 77 62.1 75.4 66.5 72.6L76.8 80.6L84.5 69.4L74.4 63.4C76.1 59.5 77 55.1 77 50.3C77 34.5 66.5 23 50 23ZM50.2 35.8C58.9 35.8 64 41.8 64 50.3C64 58.9 58.9 64.4 50.2 64.4C41.3 64.4 36 58.9 36 50.3C36 41.8 41.3 35.8 50.2 35.8Z"/></svg>').toString("base64")}`}
      style={{
        width: props.size ?? 24,
        height: props.size ?? 24,
        borderRadius: (props.size ?? 24) / 2,
        flexShrink: 0,
      }}
    />
  );
}

export function presence(open: () => boolean, duration = 150) {
  const shown = signal(false);
  const opacity = createAnimatedValue(0);
  let generation = 0;
  effect(() => {
    const visible = open();
    untracked(() => {
      const current = ++generation;
      if (visible) shown.set(true);
      void opacity
        .timing(visible ? 1 : 0, { duration, easing: "easeOut" })
        .then(({ finished }) => {
          if (finished && current === generation && !visible) shown.set(false);
        });
    });
  });
  cleanup(() => {
    generation++;
    void opacity.stop();
  });
  return {
    shown,
    opacity,
    offset: opacity.interpolate({ inputRange: [0, 1], outputRange: [6, 0] }),
  };
}

export function Modal(props: {
  open: boolean;
  title: string;
  onClose: () => void;
  children?: JSX.Element;
  testID?: string;
  blocked?: boolean;
  compact?: boolean;
}) {
  const motion = presence(() => props.open);
  return (
    <Dialog
      open={motion.shown()}
      testID={props.testID}
      animation="none"
      closeOnBackdropPress={false}
      closeOnEscape={false}
      onCancel={() => {
        if (!props.blocked) props.onClose();
      }}
      backdropStyle={{ backgroundColor: "#1f232866", opacity: motion.opacity }}
      backdropTestID={`${props.testID}-backdrop`}
      style={{
        width: windowSize().width,
        height: windowSize().height,
        alignItems: "center",
        justifyContent: "center",
        padding: 16,
      }}
    >
      <View
        style={{
          width: "100%",
          maxWidth: props.compact ? 276 : 420,
          borderRadius: props.compact ? 10 : 8,
          backgroundColor: C.surface,
          borderWidth: 1,
          borderColor: "#ffffff94",
          padding: props.compact ? 8 : 18,
          boxShadow: "0 10px 36px #00000033",
          gap: props.compact ? 7 : 12,
          opacity: motion.opacity,
          transform: [{ translateY: motion.offset }],
        }}
      >
        <View
          style={{
            minHeight: 26,
            justifyContent: "center",
            alignItems: "center",
          }}
        >
          <Text style={{ fontSize: 15, fontWeight: "700" }}>{props.title}</Text>
          <View style={{ position: "absolute", right: -4, top: -3 }}>
            <Action
              label={props.title}
              icon="close"
              iconOnly
              compact
              onPress={props.onClose}
              disabled={props.blocked}
              testID={`${props.testID}-close`}
            />
          </View>
        </View>
        {props.children}
      </View>
    </Dialog>
  );
}
