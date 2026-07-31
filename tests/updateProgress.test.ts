import assert from "node:assert/strict";
import test from "node:test";
import { reduceUpdateDownloadProgress } from "../src/lib/updateProgress.ts";

test("starts a known-length update at zero percent", () => {
  assert.deepEqual(
    reduceUpdateDownloadProgress(null, {
      event: "Started",
      data: { contentLength: 100 },
    }),
    { downloaded: 0, percent: 0, total: 100 },
  );
});

test("accumulates chunks and calculates percentage", () => {
  const started = reduceUpdateDownloadProgress(null, {
    event: "Started",
    data: { contentLength: 200 },
  });

  assert.deepEqual(
    reduceUpdateDownloadProgress(started, {
      event: "Progress",
      data: { chunkLength: 50 },
    }),
    { downloaded: 50, percent: 25, total: 200 },
  );
});

test("keeps byte progress when content length is unknown", () => {
  const started = reduceUpdateDownloadProgress(null, {
    event: "Started",
    data: {},
  });

  assert.deepEqual(
    reduceUpdateDownloadProgress(started, {
      event: "Progress",
      data: { chunkLength: 64 },
    }),
    { downloaded: 64, percent: 0, total: null },
  );
});

test("marks a finished known-length download complete", () => {
  assert.deepEqual(
    reduceUpdateDownloadProgress(
      { downloaded: 90, percent: 90, total: 100 },
      { event: "Finished" },
    ),
    { downloaded: 100, percent: 100, total: 100 },
  );
});
