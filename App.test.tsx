import { expect, open, test } from "nsh/test";

test("creates, edits, copies and deletes a note using NSH controls", async () => {
  const app = await open("./App.tsx", {
    platform: "web",
    locale: "en-US",
    width: 520,
    height: 740,
  });
  await app.press("new-note");
  await app.type("editor-content", "  NSH migration  ");
  await app.press("color-2");
  await app.press("editor-pin");
  await app.press("editor-save");
  await app.wait(180);
  await app.waitUntil(() => !!app.storage()["q-note:web-data"]);
  const data = JSON.parse(app.storage()["q-note:web-data"]);
  expect(data.notes).toHaveLength(1);
  expect(data.notes[0]).toMatchObject({
    content: "NSH migration",
    color: "#ffd8a8",
    pinned: true,
  });
  const id = `note-${data.notes[0].id}`;
  await app.press(id);
  expect(app.texts()).toContain("Copied");
  await app.rightClick(id);
  await app.press("text:Edit");
  await app.type("editor-content", " updated");
  await app.keyboard.press("Control+Enter");
  await app.wait(180);
  await app.waitUntil(() =>
    JSON.parse(app.storage()["q-note:web-data"]).notes[0].content.endsWith(
      "updated",
    ),
  );
  await app.press("delete-all");
  await app.press("delete-cancel");
  await app.wait(180);
  expect(JSON.parse(app.storage()["q-note:web-data"]).notes).toHaveLength(1);
  await app.press("delete-all");
  await app.press("delete-confirm-submit");
  await app.waitUntil(
    () => JSON.parse(app.storage()["q-note:web-data"]).notes.length === 0,
  );
});

test("keeps attachment values, supports preview and cancels unsaved changes", async () => {
  const app = await open("./App.tsx", {
    platform: "web",
    locale: "en-US",
    width: 540,
    height: 800,
  });
  await app.press("empty-new");
  const png = new Uint8Array([
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0,
    0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120,
    156, 99, 96, 0, 2, 0, 0, 5, 0, 1, 165, 246, 69, 64, 0, 0, 0, 0, 73, 69, 78,
    68, 174, 66, 96, 130,
  ]);
  app.onPick(() => [{ name: "pixel.png", type: "image/png", data: png }]);
  await app.press("editor-image");
  await app.type("editor-media", "C:\\notes\\reference.pdf");
  await app.submit("editor-media");
  await app.press("editor-save");
  await app.wait(180);
  await app.waitUntil(() => !!app.storage()["q-note:web-data"]);
  const note = JSON.parse(app.storage()["q-note:web-data"]).notes[0];
  expect(note.attachments).toHaveLength(2);
  expect(note.attachments[0]).toMatchObject({
    kind: "image",
    source: "data",
    name: "pixel.png",
  });
  expect(note.attachments[1]).toMatchObject({
    kind: "file",
    source: "path",
    value: "C:\\notes\\reference.pdf",
  });
  await app.press("label:pixel.png");
  await app.press("preview-plus");
  expect(app.texts()).toContain("125%");
  await app.keyboard.press("0");
  expect(app.texts()).toContain("100%");
  await app.keyboard.press("Escape");
  await app.wait(180);
  await app.rightClick(`note-${note.id}`);
  await app.press("text:Edit");
  await app.type("editor-content", "discard me");
  await app.press("editor-cancel");
  await app.wait(180);
  expect(JSON.parse(app.storage()["q-note:web-data"]).notes[0].content).toBe(
    "",
  );
});

test("persists language and leaves settings open after internal interaction", async () => {
  const app = await open("./App.tsx", {
    platform: "web",
    locale: "zh-CN",
    width: 300,
    height: 450,
  });
  await app.press("language");
  await app.waitUntil(() => !!app.storage()["q-note:web-data"]);
  expect(JSON.parse(app.storage()["q-note:web-data"]).settings.language).toBe(
    "en",
  );
  await app.press("open-settings");
  expect(app.shows("settings")).toBe(true);
  expect(app.enabled("startup-switch")).toBe(false);
  await app.press("settings-version");
  expect(app.shows("settings")).toBe(true);
  expect(app.openedUrls()[0]).toContain("/releases/tag/nsh-v");
  await app.keyboard.press("Escape");
  await app.wait(180);
  expect(app.shows("settings")).toBe(false);
});

test("keeps card actions separate from copy and persists color and order", async () => {
  const app = await open("./App.tsx", {
    platform: "web",
    locale: "en-US",
    width: 300,
    height: 600,
  });
  const read = () => JSON.parse(app.storage()["q-note:web-data"]);
  for (const content of [
    "first\nsecond\nthird\nfourth\nfifth",
    "another note",
  ]) {
    await app.press("new-note");
    await app.type("editor-content", content);
    await app.press("editor-save");
    await app.wait(180);
  }
  const long = read().notes.find((n: { content: string }) =>
    n.content.includes("first"),
  );
  const other = read().notes.find((n: { id: string }) => n.id !== long.id);
  expect(
    app.texts().indexOf(other.content) < app.texts().indexOf(long.content),
  ).toBe(true);
  await app.hover(`note-${long.id}`);
  await app.hover(`actions-${long.id}`);
  await app.wait(180);
  await app.press("label:Color");
  await app.press("color-4");
  await app.waitUntil(
    () =>
      read().notes.find((n: { id: string }) => n.id === long.id).color ===
      "#b2f2bb",
  );
  expect(app.clipboard()).toBe(null);
  await app.hover(`note-${long.id}`);
  await app.drag(`note-${long.id}`, { y: -65 });
  await app.waitUntil(() => read().notes[0].id === long.id);
  expect(
    app.texts().indexOf(long.content) < app.texts().indexOf(other.content),
  ).toBe(true);
  expect(read().notes[1].id).toBe(other.id);
  expect(app.clipboard()).toBe(null);
});
