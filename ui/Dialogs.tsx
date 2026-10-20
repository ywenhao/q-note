import { Scroll, signal, Text, View } from "nsh";
import { presence } from "./presence";
import { Button, Icon, Modal } from "./components";
import type { AppState } from "./state";

export function Settings(props: {
  state: AppState;
  onClose: () => void;
  closing: boolean;
}) {
  const s = props.state;
  const autoStartBusy = signal(false);
  return (
    <Modal onClose={props.onClose} width={276} compact closing={props.closing}>
      <View
        style={{ height: 24, alignItems: "center", justifyContent: "center" }}
      >
        <Text style={{ fontSize: 12.6, fontWeight: "600", color: "#1d1d1f" }}>
          {s.t().settingsTitle}
        </Text>
        <View style={{ position: "absolute", right: 0, top: 0 }}>
          <Button
            small
            label={s.t().cancel}
            icon="X"
            onPress={props.onClose}
            style={{
              borderRadius: 99,
              backgroundColor: "rgba(120,120,128,0.12)",
            }}
          />
        </View>
      </View>
      <View
        style={{
          borderRadius: 9,
          overflow: "hidden",
          backgroundColor: "rgba(60,60,67,0.08)",
          gap: 4,
          paddingVertical: 2,
        }}
      >
        <View
          accessibilityRole="switch"
          accessibilityLabel={s.t().startupSetting}
          disabled={autoStartBusy()}
          onPress={() => {
            if (autoStartBusy()) return;
            autoStartBusy.set(true);
            void s
              .call("autoStart", {
                enabled: !s.snapshot().data.settings.autoStart,
              })
              .finally(() => autoStartBusy.set(false));
          }}
          class="hover:bg-white/80"
          style={{
            height: 34,
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "space-between",
            paddingHorizontal: 9,
            backgroundColor: "rgba(255,255,255,0.58)",
            cursor: "pointer",
          }}
        >
          <View style={{ flexDirection: "row", alignItems: "center", gap: 6 }}>
            <Icon name="Power" size={12} />
            <Text style={{ fontSize: 12.04, color: "#1d1d1f" }}>
              {s.t().startupSetting}
            </Text>
          </View>
          <View
            style={{
              width: 28,
              height: 17,
              borderRadius: 99,
              backgroundColor: s.snapshot().data.settings.autoStart
                ? "#34c759"
                : "rgba(120,120,128,0.28)",
              padding: 2,
              alignItems: s.snapshot().data.settings.autoStart
                ? "flex-end"
                : "flex-start",
            }}
          >
            <View
              style={{
                width: 13,
                height: 13,
                borderRadius: 99,
                backgroundColor: "white",
              }}
            />
          </View>
        </View>
        <View style={{ flexDirection: "row", gap: 1 }}>
          <Button
            label={s.t().import}
            icon="Upload"
            text={s.t().import}
            onPress={() => void s.call("import")}
            style={{ flex: 1, height: 34, borderWidth: 0, borderRadius: 0 }}
          />
          <Button
            label={s.t().export}
            icon="Download"
            text={s.t().export}
            onPress={() => void s.call("export")}
            style={{ flex: 1, height: 34, borderWidth: 0, borderRadius: 0 }}
          />
        </View>
        <View>
          <Button
            label={s.t().checkUpdate}
            icon="RefreshCw"
            text={
              s.snapshot().update.phase === "checking"
                ? s.t().checkingUpdate
                : s.t().checkUpdate
            }
            disabled={s.snapshot().update.phase !== "idle"}
            onPress={() => void s.call("checkUpdate")}
            style={{ height: 34, borderRadius: 0, borderWidth: 0 }}
          />
          <If when={s.snapshot().update.info}>
            <View
              pointerEvents="none"
              style={{
                position: "absolute",
                right: 8,
                top: 6,
                width: 6,
                height: 6,
                borderRadius: 99,
                backgroundColor: "#ff3b30",
              }}
            />
          </If>
        </View>
      </View>
      <View style={{ alignItems: "center" }}>
        <View
          onPress={() =>
            void s.call("release", { version: s.snapshot().version })
          }
          style={{
            cursor: "pointer",
            paddingHorizontal: 6,
            paddingVertical: 1,
          }}
        >
          <Text style={{ fontSize: 10.36, color: "rgba(60,60,67,0.72)" }}>
            v{s.snapshot().version}
          </Text>
        </View>
      </View>
    </Modal>
  );
}

export function DeleteAll(props: {
  state: AppState;
  onClose: () => void;
  closing: boolean;
}) {
  const s = props.state;
  return (
    <Modal onClose={props.onClose} closing={props.closing}>
      <Text style={{ fontSize: 15.12, fontWeight: "600", color: "#1d1d1f" }}>
        {s.t().confirmDeleteAll}
      </Text>
      <Text
        style={{ fontSize: 14, lineHeight: 21, color: "rgba(60,60,67,0.78)" }}
      >
        {s.t().deleteAllBody}
      </Text>
      <View
        style={{ flexDirection: "row", justifyContent: "flex-end", gap: 8 }}
      >
        <Button
          label={s.t().cancel}
          text={s.t().cancel}
          onPress={props.onClose}
        />
        <Button
          danger
          label={s.t().deleteAll}
          text={s.t().deleteAll}
          onPress={() => {
            void s.call("deleteAll").then(() => props.onClose());
          }}
        />
      </View>
    </Modal>
  );
}

export function UpdateDialog(props: { state: AppState }) {
  const s = props.state,
    shown = presence(() =>
      s.snapshot().update.show ? s.snapshot().update : null,
    ),
    update = () => shown.value() ?? s.snapshot().update;
  const working = () =>
    ["downloading", "preparing", "installing"].includes(update().phase);
  const cancel = () => {
    if (update().phase === "installing") return;
    void s.call(working() ? "cancelUpdate" : "dismissUpdate");
  };
  const progress = () =>
    Math.min(
      100,
      Math.round(
        ((update().downloaded ?? 0) / Math.max(1, update().total ?? 1)) * 100,
      ),
    );
  const size = (bytes: number) =>
    bytes < 1024
      ? `${bytes} B`
      : bytes < 1048576
        ? `${(bytes / 1024).toFixed(1)} KB`
        : `${(bytes / 1048576).toFixed(1)} MB`;
  return (
    <If when={shown.value()?.info}>
      <Modal onClose={cancel} closing={shown.closing()}>
        <Text style={{ fontSize: 15.12, fontWeight: "600", color: "#1d1d1f" }}>
          {working()
            ? update().phase === "downloading"
              ? s.t().updateDownloading
              : update().phase === "preparing"
                ? s.t().updatePreparing
                : s.t().updateInstalling
            : s.t().updateAvailableTitle(update().info!.version)}
        </Text>
        <Text
          style={{ fontSize: 14, lineHeight: 21, color: "rgba(60,60,67,0.78)" }}
        >
          {working()
            ? `v${update().info!.version}`
            : s.t().updateConfirmDefault}
        </Text>
        <If when={!!update().info?.notes}>
          <Scroll style={{ maxHeight: 140 }}>
            <Text
              style={{
                fontSize: 12.88,
                lineHeight: 18.68,
                color: "rgba(60,60,67,0.62)",
              }}
            >
              {update().info?.notes}
            </Text>
          </Scroll>
        </If>
        <If when={working()}>
          <View style={{ gap: 8 }}>
            <View
              style={{
                height: 7,
                borderRadius: 99,
                backgroundColor: "rgba(0,122,255,0.12)",
                overflow: "hidden",
              }}
            >
              <View
                style={{
                  height: 7,
                  width: `${progress()}%`,
                  backgroundColor: "#007aff",
                  borderRadius: 99,
                }}
              />
            </View>
            <Text
              style={{ fontSize: 12, color: "#414144", textAlign: "center" }}
            >
              {progress()}% · {size(update().downloaded ?? 0)} /{" "}
              {size(update().total ?? 0)}
            </Text>
          </View>
        </If>
        <View
          style={{ flexDirection: "row", justifyContent: "flex-end", gap: 8 }}
        >
          <Button
            label={s.t().cancel}
            text={s.t().cancel}
            disabled={update().phase === "installing"}
            onPress={cancel}
          />
          <If when={!working()}>
            <Button
              primary
              label={s.t().updateConfirm}
              text={s.t().updateConfirm}
              onPress={() => void s.call("installUpdate")}
            />
          </If>
        </View>
      </Modal>
    </If>
  );
}
