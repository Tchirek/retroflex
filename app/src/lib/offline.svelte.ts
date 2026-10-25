import type { Snap, Station } from "./api";
import { isApp } from "./links";

type Core = typeof import("@tauri-apps/api/core");

// 持ち帰る　歌の在り処と　汲む進み
export const kept = $state<Record<string, string>>({});
export const fetching = $state<Record<string, number>>({});
const paths: Record<string, string> = {};
let core: Core | null = null;
let ready: Promise<void> | null = null;

const EXT: Record<string, string> = {
  "audio/mpeg": "mp3",
  "audio/mp4": "m4a",
  "audio/x-m4a": "m4a",
  "audio/flac": "flac",
  "audio/x-flac": "flac",
  "audio/ogg": "ogg",
  "audio/opus": "opus",
  "audio/wav": "wav",
  "audio/x-wav": "wav",
  "audio/aac": "aac",
};

export const keyOf = (station: string, id: string) => `${station}/${id}`;

function put(key: string, path: string) {
  paths[key] = path;
  kept[key] = core!.convertFileSrc(path);
}

// 棚の中　先に数えて　覚えおく
export function loadKept(): Promise<void> {
  if (!isApp) return Promise.resolve();
  ready ??= (async () => {
    core = await import("@tauri-apps/api/core");
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{ key: string; got: number; total: number }>("kept", ({ payload: p }) => {
      if (p.key in fetching) fetching[p.key] = p.total ? Math.min(0.99, p.got / p.total) : 0.05;
    });
    for (const k of await core.invoke<{ key: string; path: string }[]>("kept")) put(k.key, k.path);
  })().catch(() => {});
  return ready;
}

export function local(station: string, id: string): string | undefined {
  return kept[keyOf(station, id)];
}

// 汲み手二人　順番待ちの　列を捌く
let running = 0;
const line: (() => Promise<void>)[] = [];
function pump() {
  while (running < 2 && line.length) {
    const job = line.shift()!;
    running++;
    job().finally(() => {
      running--;
      pump();
    });
  }
}

// 一押しで　束ごと棚へ　運び込む
export async function keep(st: Station, list: (Snap & { mime?: string })[]): Promise<void> {
  await loadKept();
  if (!core) return;
  for (const t of list) {
    const key = keyOf(st.name, t.id);
    if (kept[key] || key in fetching) continue;
    fetching[key] = 0;
    const name = `${t.artist} - ${t.title} [${st.name}_${t.id}]`;
    const hint = t.mime ? EXT[t.mime] : undefined;
    line.push(async () => {
      for (const relay of [false, true]) {
        try {
          put(key, await core!.invoke<string>("keep", { url: st.audio(t.id, relay), key, name, hint }));
          break;
        } catch {}
      }
      delete fetching[key];
    });
  }
  pump();
}

export async function forget(station: string, ids: string[]): Promise<void> {
  if (!core) return;
  for (const id of ids) {
    const key = keyOf(station, id);
    if (!paths[key]) continue;
    try {
      await core.invoke("forget", { path: paths[key] });
      delete paths[key];
      delete kept[key];
    } catch {}
  }
}
