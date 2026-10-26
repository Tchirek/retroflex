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
    return this.url(`audio/${id}`, `${relay ? "&relay=1" : ""}&v=2`);
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

  // 一行ごと　訳を中枢に　頼みけり
  async translate(id: string, lang: string): Promise<Record<string, string> | null> {
    try {
      const r = await fetch(this.url(`lyrics/${id}/to/${lang}`));
      if (!r.ok) return null;
      const body = await r.json();
      return body.lines && Object.keys(body.lines).length ? (body.lines as Record<string, string>) : null;
    } catch {
      return null;
    }
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
export async function openRoom(hub: string, station: string, key: string, ticket: string, open?: string): Promise<string> {
  const body = open ? { station, key, ticket, public: true, secret: open } : { station, key, ticket };
  const r = await post(`${hub}/api/rooms`, body);
  if (!r.ok) throw new Error(String(r.status));
  return (await r.json()).id as string;
}

function post(url: string, body: unknown): Promise<Response> {
  return fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
}

export type Seat = { id: string; secret: string };

// 広間へと　耳の合いそな　席を問う
export async function matchRoom(hub: string, station: string, artists: string[]): Promise<Seat | null> {
  try {
    const r = await post(`${hub}/api/match`, { station, artists });
    return r.ok ? ((await r.json()) as Seat) : null;
  } catch {
    return null;
  }
}

// 六つ字の　合言葉をば　借りてくる
export async function makeInvite(hub: string, room: string, ticket: string, secret: string): Promise<string | null> {
  try {
    const r = await post(`${hub}/api/invites`, { room, ticket, secret });
    return r.ok ? ((await r.json()).code as string) : null;
  } catch {
    return null;
  }
}

export async function redeemInvite(hub: string, code: string): Promise<Seat | null> {
  try {
    const r = await fetch(`${hub}/api/invites/${encodeURIComponent(code)}`);
    return r.ok ? ((await r.json()) as Seat) : null;
  } catch {
    return null;
  }
}

export function snap(t: Track): Snap {
  return { id: t.id, title: t.title, artist: t.artist, album: t.album, duration: t.duration, cover: t.cover };
}
