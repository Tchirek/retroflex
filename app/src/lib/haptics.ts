const tauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
const android = /Android/i.test(ua);
const ios = /iPhone|iPad|iPod/i.test(ua) || (/Macintosh/.test(ua) && typeof navigator !== "undefined" && navigator.maxTouchPoints > 1);

type Weight = "light" | "medium";
let plugin: Promise<typeof import("@tauri-apps/plugin-haptics") | null> | null = null;

// 林檎窓　小さき鍵の　手応えを
function flick(): void {
  const label = document.createElement("label");
  label.ariaHidden = "true";
  label.style.display = "none";
  const input = document.createElement("input");
  input.type = "checkbox";
  input.setAttribute("switch", "");
  label.append(input);
  document.head.append(label);
  label.click();
  label.remove();
}

// 指先に　かすかな鼓動　返しけり
export function haptic(weight: Weight = "light"): void {
  if (!android && !ios) return;
  if (tauri) {
    plugin ??= import("@tauri-apps/plugin-haptics").catch(() => null);
    plugin.then((m) => {
      if (!m) return;
      const done = android ? m.vibrate(weight === "light" ? 12 : 24) : m.impactFeedback(weight);
      done.catch(() => {});
    });
    return;
  }
  if (ios) return flick();
  navigator.vibrate?.(weight === "light" ? 10 : 20);
}
