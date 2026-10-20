import { effect, Input, Scroll, signal, Text, View, type YNode } from "nsh";
import { keymap } from "nsh/keymap";
import {
  AttachmentImage,
  Button,
  Icon,
  Palette,
  TitleBar,
  Toast,
} from "./components";
import { Preview } from "./Preview";
import { presence } from "./presence";
import { createAttachment, type AppState } from "./state";
import type { NoteAttachment, NoteDraft } from "./types";

export function Editor(props: { state: AppState }) {
  const s = props.state,
    draft = signal<NoteDraft>({
      content: "",
      color: "#fff9db",
      pinned: false,
      attachments: [],
    }),
    session = signal(""),
    media = signal(""),
    preview = signal<{ items: NoteAttachment[]; index: number } | null>(null);
  let input: YNode,
    mediaInput: YNode,
    focused: "content" | "media" | null = null,
    selection = { start: 0, end: 0 },
    mediaSelection = { start: 0, end: 0 };
  effect(() => {
    const incoming = s.snapshot().editor;
    if (incoming) {
      const changed = incoming.id !== session();
      session.set(incoming.id);
      draft.set(incoming.draft);
      if (changed) {
        media.set("");
        preview.set(null);
        setTimeout(() => input?.focus(), 60);
      }
    }
  });
  function update(patch: Partial<NoteDraft>) {
    const next = { ...draft(), ...patch };
    draft.set(next);
    void s.call("draft", { session: session(), draft: next });
  }
  function addMedia() {
    const value = media().trim();
    if (!value) return;
    update({ attachments: [...draft().attachments, createAttachment(value)] });
    media.set("");
  }
  function remove(id: string) {
    update({ attachments: draft().attachments.filter((a) => a.id !== id) });
  }
  let saving = false;
  async function save() {
    if (shownPreview.value() || saving) return;
    saving = true;
    try {
      await s.call("save", { session: session(), draft: draft() });
    } finally {
      saving = false;
    }
  }
  keymap.bind("mod+s", save, { when: "always" });
  keymap.bind(
    "mod+v",
    () => {
      if (!focused || preview()) return;
      const target = focused,
        start = target === "content" ? selection : mediaSelection;
      void s
        .call<{ image?: NoteAttachment; text?: string }>("paste", {
          image: target === "content",
        })
        .then((result) => {
          if (!result) return;
          if (result.image && target === "content")
            update({ attachments: [...draft().attachments, result.image] });
          else if (result.text !== undefined) {
            if (target === "content") {
              update({
                content:
                  draft().content.slice(0, start.start) +
                  result.text +
                  draft().content.slice(start.end),
              });
              input?.setSelection(start.start + result.text.length);
            } else {
              media.set(
                media().slice(0, start.start) +
                  result.text +
                  media().slice(start.end),
              );
              mediaInput?.setSelection(start.start + result.text.length);
            }
          }
        });
    },
    { when: "always" },
  );
  s.onEvent((event) => {
    if (event.kind === "files" && event.paths)
      update({
        attachments: [
          ...draft().attachments,
          ...event.paths.map(createAttachment),
        ],
      });
  });
  const shownPreview = presence(() => preview());
  let previewFocus: "content" | "media" | null = null;
  effect(() => {
    if (shownPreview.value()) {
      previewFocus = focused;
      input?.blur();
      mediaInput?.blur();
    } else if (previewFocus) {
      (previewFocus === "content" ? input : mediaInput)?.focus();
      previewFocus = null;
    }
  });
  const images = () => draft().attachments.filter((a) => a.kind === "image");
  return (
    <View
      style={{
        flex: 1,
        borderRadius: 12,
        overflow: "hidden",
        backgroundColor: "#fff9df",
        paddingTop: 8,
        paddingHorizontal: 10,
        paddingBottom: 12,
      }}
    >
      <TitleBar
        state={s}
        title={
          s.snapshot().editor?.noteId
            ? s.t().editorEditTitle
            : s.t().editorNewTitle
        }
      />
      <View
        style={{
          flex: 1,
          minHeight: 0,
          gap: 12,
          backgroundColor: "rgba(255,253,244,0.88)",
        }}
      >
        <View style={{ flexShrink: 0, paddingTop: 8, paddingHorizontal: 12 }}>
          <Palette
            state={s}
            color={draft().color}
            onChange={(color) => update({ color })}
          />
        </View>
        <Input
          ref={(ref) => (input = ref)}
          testID="note-content"
          accessibilityLabel={s.t().contentPlaceholder}
          value={draft().content}
          onChangeText={(content) => update({ content })}
          multiline
          autoFocus
          autoCorrect={false}
          placeholder={s.t().contentPlaceholder}
          placeholderTextColor="rgba(60,60,67,0.5)"
          onFocus={() => (focused = "content")}
          onBlur={() => (focused = null)}
          onSelectionChange={(e) => (selection = e)}
          style={{
            flex: 1,
            minHeight: 140,
            borderTopWidth: 1,
            borderBottomWidth: 1,
            borderColor: "rgba(60,60,67,0.1)",
            paddingVertical: 14,
            paddingHorizontal: 16,
            backgroundColor: "rgba(255,255,255,0.5)",
            color: "#1d1d1f",
            fontSize: 14,
            lineHeight: 21.7,
          }}
        />
        <View
          style={{
            flexDirection: "row",
            flexWrap: "wrap",
            alignItems: "center",
            gap: 10,
            paddingHorizontal: 12,
            flexShrink: 0,
          }}
        >
          <Button
            label={s.t().addImage}
            icon="ImagePlus"
            text={s.t().addImage}
            onPress={() => void s.call("pickImages", { session: session() })}
          />
          <View
            style={{
              flex: 1,
              flexBasis: 260,
              minWidth: 0,
              height: 34,
              flexDirection: "row",
              alignItems: "center",
              gap: 7,
              borderWidth: 1,
              borderColor: "rgba(60,60,67,0.12)",
              borderRadius: 8,
              paddingHorizontal: 10,
              backgroundColor: "rgba(255,255,255,0.62)",
            }}
          >
            <Icon name="Link2" size={14} />
            <Input
              ref={(ref) => (mediaInput = ref)}
              accessibilityLabel={s.t().mediaPlaceholder}
              value={media()}
              onChangeText={(text) => media.set(text)}
              placeholder={s.t().mediaPlaceholder}
              onSubmitEditing={addMedia}
              blurOnSubmit={false}
              onFocus={() => (focused = "media")}
              onBlur={() => (focused = null)}
              onSelectionChange={(e) => (mediaSelection = e)}
              style={{
                flex: 1,
                fontSize: 13,
                color: "#1d1d1f",
                backgroundColor: "transparent",
              }}
            />
            <Icon name="Folder" size={14} />
          </View>
          <Button
            label={s.t().addMedia}
            icon="ImagePlus"
            text={s.t().addMedia}
            onPress={addMedia}
          />
        </View>
        <If when={draft().attachments.length > 0}>
          <Scroll
            style={{ maxHeight: 180, flexShrink: 1, marginHorizontal: 12 }}
            contentContainerStyle={{
              flexDirection: "row",
              flexWrap: "wrap",
              gap: 10,
            }}
          >
            <Each of={draft().attachments} key={(item) => item.id}>
              {(item) => (
                <If
                  when={item().kind === "image"}
                  else={
                    <View
                      style={{
                        width: 140,
                        minHeight: 38,
                        flexDirection: "row",
                        alignItems: "center",
                        gap: 7,
                        padding: 8,
                        borderWidth: 1,
                        borderColor: "rgba(60,60,67,0.12)",
                        borderRadius: 8,
                        backgroundColor: "rgba(255,255,255,0.62)",
                      }}
                    >
                      <Icon name="FileText" size={14} />
                      <Text
                        numberOfLines={1}
                        ellipsizeMode="tail"
                        style={{ flex: 1, fontSize: 12, color: "#1d1d1f" }}
                      >
                        {item().name || item().value}
                      </Text>
                      <Button
                        small
                        danger
                        label={s.t().removeAttachment}
                        icon="Trash2"
                        onPress={() => remove(item().id)}
                      />
                    </View>
                  }
                >
                  <View
                    style={{
                      position: "relative",
                      width: 108,
                      height: 86.4,
                      borderWidth: 1,
                      borderColor: "rgba(60,60,67,0.12)",
                      borderRadius: 8,
                      overflow: "hidden",
                    }}
                  >
                    <View
                      onPress={() =>
                        preview.set({
                          items: images(),
                          index: images().findIndex((a) => a.id === item().id),
                        })
                      }
                      style={{ flex: 1, cursor: "pointer" }}
                    >
                      <AttachmentImage
                        state={s}
                        attachment={item()}
                        style={{ width: "100%", height: "100%" }}
                      />
                    </View>
                    <View style={{ position: "absolute", right: 6, top: 6 }}>
                      <Button
                        small
                        danger
                        label={s.t().removeAttachment}
                        icon="Trash2"
                        onPress={() => remove(item().id)}
                        style={{
                          width: 22,
                          height: 22,
                          minWidth: 22,
                          borderRadius: 99,
                          backgroundColor: "rgba(255,255,255,0.86)",
                        }}
                      />
                    </View>
                  </View>
                </If>
              )}
            </Each>
          </Scroll>
        </If>
        <View
          style={{
            flexDirection: "row",
            justifyContent: "space-between",
            alignItems: "center",
            gap: 10,
            padding: 12,
            flexShrink: 0,
          }}
        >
          <View
            accessibilityRole="checkbox"
            accessibilityLabel={s.t().pinned}
            onPress={() => update({ pinned: !draft().pinned })}
            style={{
              flexDirection: "row",
              alignItems: "center",
              gap: 8,
              cursor: "pointer",
            }}
          >
            <View
              style={{
                width: 16,
                height: 16,
                borderWidth: 1,
                borderColor: draft().pinned ? "#007aff" : "rgba(60,60,67,0.3)",
                borderRadius: 3,
                backgroundColor: draft().pinned ? "#007aff" : "white",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <If when={draft().pinned}>
                <Icon name="Check" size={12} color="white" />
              </If>
            </View>
            <Text style={{ fontSize: 14, color: "#1d1d1f" }}>
              {s.t().pinned}
            </Text>
          </View>
          <View style={{ flexDirection: "row", gap: 8 }}>
            <Button
              label={s.t().cancel}
              text={s.t().cancel}
              onPress={() => void s.call("close", { window: "editor" })}
            />
            <Button
              testID="save-note"
              primary
              label={s.t().save}
              text={s.t().save}
              onPress={save}
            />
          </View>
        </View>
      </View>
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
