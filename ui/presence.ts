import { cleanup, effect, signal } from "nsh";

export function presence<T>(read: () => T | null | undefined | false) {
  const value = signal<T | null>(null);
  const closing = signal(false);
  let timer: ReturnType<typeof setTimeout>;
  effect(() => {
    const next = read();
    if (timer !== undefined) clearTimeout(timer);
    if (next) {
      value.set(next);
      closing.set(false);
    } else {
      closing.set(true);
      timer = setTimeout(() => value.set(null), 150);
    }
  });
  cleanup(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
  return { value, closing };
}
