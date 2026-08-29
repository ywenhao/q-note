const IMAGE_EXTENSIONS = new Set(["avif", "bmp", "gif", "jpeg", "jpg", "png", "svg", "webp"]);

export function isLikelyImagePath(value: string) {
  const cleanValue = value.split(/[?#]/)[0] ?? value;
  const extension = cleanValue.split(".").pop()?.toLowerCase();

  return Boolean(extension && IMAGE_EXTENSIONS.has(extension));
}
