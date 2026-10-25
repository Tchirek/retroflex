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

export function dropPlaylist(id: string): void {
  const i = playlists.findIndex((p) => p.id === id);
  if (i >= 0) {
    playlists.splice(i, 1);
    persist();
  }
}
