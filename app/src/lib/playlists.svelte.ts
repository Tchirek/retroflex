import type { Snap } from "./api";

export type Playlist = { id: string; station: string; at: number; tracks: Snap[] };

const KEY = "retroflex.playlists";

function load(): Playlist[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(raw) ? raw.filter((p) => p && Array.isArray(p.tracks)) : [];
  } catch {
    return [];
  }
}

// 選びたる　歌の束をば　手帳へと
export const playlists = $state<Playlist[]>(load());

function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify(playlists));
  } catch {}
}

export function savePlaylist(station: string, tracks: Snap[]): Playlist | null {
  if (!tracks.length) return null;
  const list: Playlist = { id: crypto.getRandomValues(new Uint32Array(2)).join("-"), station, at: Date.now(), tracks };
  playlists.unshift(list);
  persist();
  return list;
}

export function dropPlaylists(ids: Iterable<string>): void {
  const gone = new Set(ids);
  const keep = playlists.filter((p) => !gone.has(p.id));
  if (keep.length === playlists.length) return;
  playlists.splice(0, playlists.length, ...keep);
  persist();
}

// 貰い物　同じ束なら　前のまま
export function adoptPlaylist(station: string, tracks: Snap[]): Playlist | null {
  const sig = tracks.map((t) => t.id).join();
  const same = playlists.find((p) => p.station === station && p.tracks.map((t) => t.id).join() === sig);
  return same ?? savePlaylist(station, tracks);
}
