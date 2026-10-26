import type { Track } from "./api";

export type Album = { key: string; title: string; artist: string; year?: number; cover?: string; tracks: Track[] };

// 名を辿る　窓の行先　ここに置く
export type Look = { kind: "album"; id: string } | { kind: "artist"; name: string };

// 散る歌を　盤ごとにまた　束ね直す
export function albums(tracks: Track[]): Album[] {
  const map = new Map<string, Album>();
  for (const t of tracks) {
    const artist = t.album_artist ?? t.artist;
    const key = `${artist}\u0000${t.album}`;
    let a = map.get(key);
    if (!a) {
      a = { key, title: t.album, artist, year: t.year, cover: t.cover, tracks: [] };
      map.set(key, a);
    }
    a.cover ??= t.cover;
    a.year ??= t.year;
    a.tracks.push(t);
  }
  return [...map.values()];
}

const fold = (s: string) => s.normalize("NFKC").toLowerCase();

// 断片の　言葉ひとつで　歌を引く
export function search(tracks: Track[], query: string): Track[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  return tracks.filter((t) => {
    const hay = fold(`${t.title} ${t.artist} ${t.album} ${t.album_artist ?? ""}`);
    return words.every((w) => hay.includes(w));
  });
}

export function clock(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0;
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
