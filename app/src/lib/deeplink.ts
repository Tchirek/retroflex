import { derive } from "./crypto";
import { isApp } from "./links";
import type { Target } from "./link";

export const SCHEME = "retroflex";
const PACKAGE = "top.tchirek.retroflex";

// 外からの　呼び名を部屋の　道に解く
export function fromDeepLink(raw: string): string | null {
  try {
    const u = new URL(raw);
    if (u.protocol !== `${SCHEME}:`) return null;
    const hub = u.searchParams.get("hub");
    const parts = `${u.host}${u.pathname}`.split("/").filter(Boolean);
    if (!hub || parts.length !== 3 || (parts[0] !== "r" && parts[0] !== "l")) return null;
    return `${hub}/#/${parts[0]}/${parts[1]}/${parts[2]}`;
  } catch {
    return null;
  }
}

// 起動時も　開いた後も　呼び声を聴く
export async function listenDeepLinks(onlink: (text: string) => void): Promise<void> {
  if (!isApp) return;
  try {
    const dl = await import("@tauri-apps/plugin-deep-link");
    const take = (urls: string[] | null) => {
      for (const u of urls ?? []) {
        const text = fromDeepLink(u);
        if (text) return onlink(text);
      }
    };
    take(await dl.getCurrent());
    await dl.onOpenUrl(take);
  } catch {}
}

// 生きた部屋　ならば手元の　窓へ渡す
export async function offerApp(target: Target): Promise<void> {
  if (isApp || (target.kind !== "room" && target.kind !== "list")) return;
  const ua = navigator.userAgent;
  if (/iPhone|iPad|iPod/i.test(ua) || new URLSearchParams(location.search).has("web")) return;
  if (target.kind === "room") {
    try {
      const r = await fetch(`${target.hub}/api/rooms/${target.room}?t=${encodeURIComponent(derive(target.secret).ticket)}`);
      if (r.status !== 204) return;
    } catch {
      return;
    }
  }
  const tail = target.kind === "room" ? `r/${target.room}/${target.secret}` : location.hash.slice(2);
  const path = `${tail}?hub=${encodeURIComponent(target.hub)}`;
  if (/Android/i.test(ua)) {
    const back = `${location.origin}${location.pathname}?web=1${location.hash}`;
    location.href = `intent://${path}#Intent;scheme=${SCHEME};package=${PACKAGE};S.browser_fallback_url=${encodeURIComponent(back)};end`;
    return;
  }
  const frame = document.createElement("iframe");
  frame.style.display = "none";
  frame.src = `${SCHEME}://${path}`;
  document.body.append(frame);
  setTimeout(() => frame.remove(), 2000);
}
