export interface UpdateDownloadProgress {
  downloaded: number;
  percent: number;
  total: number | null;
}

export type UpdateDownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export function reduceUpdateDownloadProgress(
  current: UpdateDownloadProgress | null,
  event: UpdateDownloadEvent,
): UpdateDownloadProgress {
  if (event.event === "Started") {
    const contentLength = event.data.contentLength;
    return {
      downloaded: 0,
      percent: 0,
      total:
        typeof contentLength === "number" && Number.isFinite(contentLength) && contentLength > 0
          ? contentLength
          : null,
    };
  }

  if (event.event === "Finished") {
    return {
      downloaded: current?.total ?? current?.downloaded ?? 0,
      percent: 100,
      total: current?.total ?? null,
    };
  }

  const downloaded = (current?.downloaded ?? 0) + Math.max(0, event.data.chunkLength);
  const total = current?.total ?? null;
  return {
    downloaded,
    percent: total ? Math.min(100, (downloaded / total) * 100) : 0,
    total,
  };
}
