const tauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const isApp = tauri;

// 外への道　窓の外へと　送り出す
export async function openExternal(url: string): Promise<void> {
  if (tauri) {
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
      return;
    } catch {}
  }
  window.open(url, "_blank", "noopener");
}
