import { effect, keymap, Picker, Scroll, signal, Text, View } from "nsh";
import { Checkbox, Input } from "nsh/base";
import type { NoteModel } from "../model";
import {
  attachmentFromValue,
  pathFromUri,
  readFileAsDataUrl,
} from "../lib/images";
import { colors as C } from "../theme";
import { Action, Icon, presence } from "../ui";
import { AttachmentImage, ColorPalette } from "./Notes";
import { ImagePreview } from "./Preview";
import { WindowBar } from "./WindowBar";

export function Editor(props: { model: NoteModel; desktop?: boolean }) {
  const m = props.model;
  const toastMotion = presence(() => !!m.toast());
  const toastText = signal("");
  effect(() => {
    if (m.toast()) toastText.set(m.toast());
  });
  const media = signal("");
  const dragging = signal(false);
  const images = () => m.draft().attachments.filter((a) => a.kind === "image");
  const addValues = (values: string[]) =>
    m.updateDraft({
      attachments: [
        ...m.draft().attachments,
        ...values
          .filter((value) => value.trim())
          .map((value) => attachmentFromValue(pathFromUri(value))),
      ],
    });
  const addMedia = () => {
    if (media().trim()) {
      addValues([media()]);
      media.set("");
    }
  };

  async function pickImages() {
    try {
      const picked = await Picker.pickFiles({
        multiple: true,
        filters: [
          {
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"],
          },
        ],
      });
      const attachments = await Promise.all(
        picked.map(async (item) =>
          attachmentFromValue(
            await readFileAsDataUrl(item.file),
            item.file.name,
          ),
        ),
      );
      if (attachments.length)
        m.updateDraft({
          attachments: [...m.draft().attachments, ...attachments],
        });
    } catch (error) {
      console.error(error);
      m.notify(m.t().loadFailed);
    }
  }

  keymap.bind(
    "mod+enter",
    () => {
      if (m.editor() && !m.preview()) void m.saveEditor();
    },
    { when: "always" },
  );

  return (
    <View
      testID="editor"
      onFileDragEnter={() => dragging.set(true)}
      onFileDragLeave={() => dragging.set(false)}
      onFileDrop={({ uris }) => {
        dragging.set(false);
        addValues(uris);
      }}
      style={{
        flex: 1,
        backgroundColor: C.editor,
        borderRadius: 12,
        overflow: "hidden",
        borderWidth: dragging() ? 2 : 0,
        borderColor: C.primary,
        paddingTop: props.desktop ? 8 : 0,
        paddingHorizontal: props.desktop ? 10 : 0,
        paddingBottom: props.desktop ? 12 : 0,
      }}
    >
      <If when={props.desktop}>
        <WindowBar
          model={m}
          title={
            m.editor()?.original ? m.t().editorEditTitle : m.t().editorNewTitle
          }
          editor
        />
      </If>
      <View
        style={{ flex: 1, gap: 12, minHeight: 0, backgroundColor: "#fffdf4e0" }}
      >
        <View style={{ paddingTop: 8, paddingHorizontal: 12 }}>
          <ColorPalette
            value={m.draft().color}
            onChange={(color) => m.updateDraft({ color })}
            testID="editor-colors"
          />
        </View>
        <Input
          testID="editor-content"
          value={m.draft().content}
          onChange={({ value }) => m.updateDraft({ content: value })}
          autoFocus
          multiline
          autoCorrect={false}
          placeholder={m.t().contentPlaceholder}
          onPaste={async ({ files }) => {
            try {
              const pictures = files.filter((file) =>
                file.type.startsWith("image/"),
              );
              const attachments = await Promise.all(
                pictures.map(async (file) =>
                  attachmentFromValue(await readFileAsDataUrl(file), file.name),
                ),
              );
              if (attachments.length)
                m.updateDraft({
                  attachments: [...m.draft().attachments, ...attachments],
                });
            } catch (error) {
              console.error(error);
              m.notify(m.t().loadFailed);
            }
          }}
          style={{
            flex: 1,
            minHeight: 140,
            paddingVertical: 14,
            paddingHorizontal: 16,
            backgroundColor: "#ffffff80",
            fontSize: 14,
            lineHeight: 21.7,
            color: C.foreground,
            borderTopWidth: 1,
            borderBottomWidth: 1,
            borderColor: "#3c3c431a",
          }}
        />
        <View
          style={{
            flexDirection: "row",
            alignItems: "center",
            gap: 10,
            paddingHorizontal: 12,
          }}
        >
          <Action
            label={m.t().addImage}
            icon="image"
            onPress={() => {
              void pickImages();
            }}
            testID="editor-image"
          />
          <View
            style={{
              flex: 1,
              minWidth: 0,
              flexDirection: "row",
              alignItems: "center",
              gap: 7,
              height: 34,
              paddingHorizontal: 10,
              borderWidth: 1,
              borderColor: "#3c3c431f",
              borderRadius: 8,
              backgroundColor: "#ffffff9e",
            }}
          >
            <Icon name="link" size={12} color="#3c3c439e" />
            <Input
              testID="editor-media"
              value={media()}
              onChange={({ value }) => media.set(value)}
              onSubmit={addMedia}
              placeholder={m.t().mediaPlaceholder}
              style={{
                flex: 1,
                height: 32,
                minWidth: 0,
                backgroundColor: "transparent",
                fontSize: 14,
              }}
            />
          </View>
          <Action
            label={m.t().addMedia}
            icon="image"
            onPress={addMedia}
            testID="editor-add-media"
          />
        </View>
        <If when={m.draft().attachments.length > 0}>
          <Scroll
            style={{ maxHeight: 150, flexShrink: 1 }}
            contentStyle={{ gap: 8, paddingHorizontal: 12 }}
          >
            <View style={{ flexDirection: "row", flexWrap: "wrap", gap: 8 }}>
              <Each of={m.draft().attachments}>
                {(attachment) => (
                  <View style={{ position: "relative" }}>
                    <If
                      when={attachment().kind === "image"}
                      else={
                        <View
                          style={{
                            height: 34,
                            maxWidth: 360,
                            flexDirection: "row",
                            alignItems: "center",
                            gap: 6,
                            paddingHorizontal: 8,
                            paddingRight: 30,
                            borderRadius: 6,
                            backgroundColor: "#fff",
                          }}
                        >
                          <Icon name="file" />
                          <Text
                            style={{
                              numberOfLines: 1,
                              fontSize: 12,
                              flexShrink: 1,
                            }}
                          >
                            {attachment().name || attachment().value}
                          </Text>
                        </View>
                      }
                    >
                      <AttachmentImage
                        attachment={attachment()}
                        size={70}
                        onPress={() =>
                          m.preview.set({
                            images: images(),
                            index: images().findIndex(
                              (a) => a.id === attachment().id,
                            ),
                            editor: true,
                          })
                        }
                      />
                    </If>
                    <View
                      style={{
                        position: "absolute",
                        right: 0,
                        top: 0,
                        backgroundColor: "#fffdf4e6",
                        borderRadius: 5,
                      }}
                    >
                      <Action
                        icon="close"
                        iconOnly
                        compact
                        label={m.t().removeAttachment}
                        onPress={() =>
                          m.updateDraft({
                            attachments: m
                              .draft()
                              .attachments.filter(
                                (a) => a.id !== attachment().id,
                              ),
                          })
                        }
                        testID={`remove-${attachment().id}`}
                      />
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </Scroll>
        </If>
        <View
          style={{
            flexDirection: "row",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 12,
            padding: 12,
          }}
        >
          <Checkbox
            testID="editor-pin"
            checked={m.draft().pinned}
            onChange={({ checked }) => m.updateDraft({ pinned: checked })}
            accessibilityLabel={m.t().pin}
            style={{ flexDirection: "row", alignItems: "center", gap: 8 }}
            indicatorStyle={{
              width: 16,
              height: 16,
              borderRadius: 3,
              borderWidth: 1,
              borderColor: m.draft().pinned ? C.primary : "#737373",
              backgroundColor: m.draft().pinned ? C.primary : "#fff",
              alignItems: "center",
              justifyContent: "center",
            }}
            indicator={
              <If when={m.draft().pinned}>
                <Icon name="check" size={12} color="#fff" />
              </If>
            }
          >
            <Text style={{ fontSize: 14 }}>{m.t().pin}</Text>
          </Checkbox>
          <View style={{ flexDirection: "row", gap: 8 }}>
            <Action
              label={m.t().cancel}
              onPress={() => {
                void m.closeEditor();
              }}
              disabled={m.saving()}
              testID="editor-cancel"
            />
            <Action
              label={m.t().save}
              primary
              onPress={() => {
                void m.saveEditor();
              }}
              disabled={m.saving()}
              testID="editor-save"
            />
          </View>
        </View>
      </View>
      <ImagePreview model={m} editor />
      <If when={toastMotion.shown()}>
        <View
          style={{
            pointerEvents: "none",
            position: "absolute",
            bottom: 60,
            left: 16,
            right: 16,
            alignItems: "center",
            opacity: toastMotion.opacity,
            transform: [{ translateY: toastMotion.offset }],
          }}
        >
          <Text
            style={{
              color: "#fff",
              backgroundColor: "#1f2328eb",
              padding: 10,
              borderRadius: 8,
              fontSize: 12,
            }}
          >
            {toastText()}
          </Text>
        </View>
      </If>
    </View>
  );
}
