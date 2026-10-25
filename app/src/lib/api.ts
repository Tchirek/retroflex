export type Track = {
  id: string;
  title: string;
  artist: string;
  album: string;
  album_artist?: string;
  track?: number;
  disc?: number;
  year?: number;
  duration: number;
  mime: string;
  size: number;
  cover?: string;
  lyrics: boolean;
};

export type Snap = Pick<Track, "id" | "title" | "artist" | "album" | "duration" | "cover">;

// 駅への道　鍵を添えれば　門ひらく
export class Station {
  constructor(
    readonly hub: string,
    readonly name: string,
    readonly pass: string,
  ) {}

  private url(path: string, extra = ""): string {
    return `${this.hub}/api/s/${this.name}/${path}?k=${encodeURIComponent(this.pass)}${extra}`;
  }

  audio(id: string, relay = false): string {
    return this.url(`audio/${id}`, relay ? "&relay=1" : "");
  }

  cover(id: string | undefined, thumb = true): string | undefined {
    return id ? this.url(`cover/${id}`, thumb ? "&thumb=1" : "") : undefined;
  }

  async library(): Promise<{ tracks: Track[]; blurs: Record<string, string> }> {
    const r = await fetch(this.url("library"));
    if (!r.ok) throw new Error(String(r.status));
    const body = await r.json();
    return { tracks: body.tracks as Track[], blurs: (body.blurs ?? {}) as Record<string, string> };
  }

  async lyrics(id: string): Promise<string | null> {
    const r = await fetch(this.url(`lyrics/${id}`));
    return r.ok ? r.text() : null;
  }
}

// 戸の開いた　駅の名前を　尋ねけり
export async function openStations(hub: string): Promise<string[]> {
  try {
    const r = await fetch(`${hub}/api/stations`);
    return r.ok ? ((await r.json()) as string[]) : [];
  } catch {
    return [];
  }
}

// 鍵ひとつ　見せて小部屋を　借り受ける
export async function openRoom(hub: string, station: string, key: string, ticket: string): Promise<string> {
  const r = await fetch(`${hub}/api/rooms`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ station, key, ticket }),
  });
  if (!r.ok) throw new Error(String(r.status));
  return (await r.json()).id as string;
}

export function snap(t: Track): Snap {
  return { id: t.id, title: t.title, artist: t.artist, album: t.album, duration: t.duration, cover: t.cover };
}
