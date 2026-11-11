import assert from "node:assert/strict";
import test from "node:test";
import { dockDragOrigin, dockScreenPointer } from "../src/lib/dockGeometry.ts";

for (const scale of [1, 1.25, 1.5, 2]) {
  test(`dock drag follows the cursor one-to-one at ${scale * 100}% DPI`, () => {
    const displays = [
      { id: "primary", bounds: { origin: { x: 0, y: 0 } }, scaleFactor: scale },
    ];
    const grab = { x: 9, y: 21 };
    const start = dockScreenPointer(
      { display: "primary", x: 409 * scale, y: 321 * scale },
      displays,
    )!;
    const moved = dockScreenPointer(
      { display: "primary", x: 529 * scale, y: 261 * scale },
      displays,
    )!;
    assert.deepEqual(dockDragOrigin(start, grab), { x: 400, y: 300 });
    assert.deepEqual(dockDragOrigin(moved, grab), { x: 520, y: 240 });
    // Repeated events after the window moves cannot feed its new origin back into the delta.
    for (let i = 0; i < 40; i++) {
      assert.deepEqual(dockDragOrigin(moved, grab), { x: 520, y: 240 });
    }
    assert.deepEqual(dockDragOrigin(start, grab), { x: 400, y: 300 });
  });
}

test("mixed-DPI negative monitors use their own NSH origin and scale", () => {
  const displays = [
    { id: "primary", bounds: { origin: { x: 0, y: 0 } }, scaleFactor: 1.5 },
    { id: "left", bounds: { origin: { x: -1920, y: -100 } }, scaleFactor: 1 },
  ];
  assert.deepEqual(
    dockScreenPointer({ display: "left", x: 300, y: 400 }, displays),
    { x: -1620, y: 300 },
  );
  assert.deepEqual(
    dockScreenPointer({ display: "primary", x: 450, y: 600 }, displays),
    { x: 300, y: 400 },
  );
  assert.equal(
    dockScreenPointer({ display: "removed", x: 1, y: 2 }, displays),
    null,
  );
});
