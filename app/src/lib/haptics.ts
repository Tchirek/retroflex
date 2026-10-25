const tauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const mobile = typeof navigator !== "undefined" && /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

type Impact = "light" | "medium" | "rigid" | "soft";
let native: ((style: Impact) => Promise<void>) | null | undefined;

// 指先に　かすかな鼓動　返しけり
export function haptic(style: Impact = "light"): void {
  if (!mobile) return;
  if (tauri) {
    if (native === undefined) {
      native = null;
      import("@tauri-apps/plugin-haptics")
        .then((m) => {
          native = (s) => m.impactFeedback(s).then(() => {});
          native(style).catch(() => {});
        })
        .catch(() => {});
      return;
    }
    native?.(style).catch(() => {});
    return;
  }
  navigator.vibrate?.(style === "light" ? 8 : 14);
}
