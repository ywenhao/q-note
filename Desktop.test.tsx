import { expect, open, test } from "nsh/test";

test("native windows retain independent bounds and hide instead of quitting", async () => {
  const app = await open("./App.tsx", { platform: "windows", locale: "en-US" });
  await app.waitFor("new-note");
  await app.wait(200);
  const initial = app.window("main").state();
  expect(initial?.visible).toBe(true);
  expect(initial?.contentBounds.size).toEqual(initial?.bounds.size);
  await app.screenshot(".nsh/qa-main.png");
  await app.press("new-note");
  await app.wait(200);
  expect(app.window("editor").state()?.visible).toBe(true);
  expect(app.window("editor").state()?.bounds.size).toEqual({
    width: 520,
    height: 640,
  });
  const editorOrigin = app.window("editor").state()!.bounds.origin;
  const mainOrigin = initial!.bounds.origin;
  expect(Math.abs(editorOrigin.x - Math.max(0, mainOrigin.x - 532)) <= 1).toBe(
    true,
  );
  expect(
    Math.abs(
      editorOrigin.y -
        Math.max(0, mainOrigin.y + (initial!.bounds.size.height - 640) / 2),
    ) <= 1,
  ).toBe(true);
  expect(app.window("main").state()?.bounds.size).toEqual(initial?.bounds.size);
  expect(app.window("editor").state()?.contentBounds.size).toEqual({
    width: 520,
    height: 640,
  });
  await app.window("editor").screenshot(".nsh/qa-editor.png");
  await app.setViewport(520, 640);
  await app.type("editor-content", "hello world!");
  await app.press("editor-save");
  await app.wait(200);
  expect(app.window("editor").state()).toBe(null);
  await app.setViewport(300, 450);
  await app.screenshot(".nsh/qa-main.png");
  await app.press("open-settings");
  await app.wait(180);
  expect(app.shows("settings-import")).toBe(true);
  await app.screenshot(".nsh/qa-settings.png");
  await app.keyboard.press("Escape");
  await app.wait(180);
  await app.press("new-note");
  await app.setViewport(520, 640);
  await app.type("editor-content", "first\nsecond\nthird\nfourth\nfifth");
  await app.press("editor-save");
  await app.wait(200);
  await app.setViewport(300, 450);
  const long = "first\nsecond\nthird\nfourth\nfifth";
  expect(app.texts().indexOf(long) < app.texts().indexOf("hello world!")).toBe(
    true,
  );
  await app.hover(`label:${long}`);
  await app.drag(`label:${long} >> label:Resize`, { y: 44 });
  expect(app.texts().indexOf(long) < app.texts().indexOf("hello world!")).toBe(
    true,
  );
  await app.screenshot(".nsh/qa-card-expanded.png");
  const beforeCollapse = app.window("main").state()!.bounds;
  await app.press("collapse");
  await app.waitUntil(() => app.window("main").state()?.visible === false);
  expect(app.window("main").state()?.visible).toBe(false);
  expect(app.window("dock").state()?.visible).toBe(true);
  expect(app.window("dock").state()?.bounds.size).toEqual({
    width: 30,
    height: 30,
  });
  const dockBounds = app.window("dock").state()!.bounds;
  expect(
    Math.abs(
      dockBounds.origin.x -
        (beforeCollapse.origin.x + beforeCollapse.size.width - 30),
    ) <= 1,
  ).toBe(true);
  expect(
    Math.abs(
      dockBounds.origin.y -
        (beforeCollapse.origin.y + beforeCollapse.size.height - 30),
    ) <= 1,
  ).toBe(true);
  await app.window("dock").screenshot(".nsh/qa-dock.png");
  await app.press("dock-ball");
  await app.wait(180);
  expect(app.window("main").state()?.visible).toBe(true);
  expect(app.window("dock").state()).toBe(null);
  for (let cycle = 0; cycle < 3; cycle++) {
    await app.press("collapse");
    await app.waitUntil(() => app.window("main").state()?.visible === false);
    expect(app.window("dock").state()?.visible).toBe(true);
    expect(app.window("dock").state()?.bounds).toEqual(dockBounds);
    await app.rightClick("dock-ball");
    const pinLabel = cycle === 1 ? "Unpin window" : "Pin window";
    expect(app.menu()?.map((item) => item.label)).toEqual([
      pinLabel,
      "Switch language",
      "Switch to main window",
      "Quit",
    ]);
    if (cycle < 2) await app.chooseMenuItem(pinLabel);
    else await app.closeMenu();
    expect(app.window("main").state()?.visible).toBe(false);
    expect(app.window("dock").state()?.visible).toBe(true);
    expect(app.window("dock").state()?.bounds).toEqual(dockBounds);
    await app.press("dock-ball");
    await app.waitUntil(() => app.window("dock").state() === null);
    expect(app.window("main").state()?.visible).toBe(true);
  }
  await app.press("window-close");
  expect(app.window("main").state()?.visible).toBe(false);
});
