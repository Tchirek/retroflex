import type { Snap } from "./api";
import { Clock } from "./clock";
import { derive, seal, unseal, type Keys } from "./crypto";
import type { Profile } from "./names";

export type QItem = Snap & { qid: number; by: number };
export type Member = { id: number; profile: Profile | null; draft: string };
export type Line = { key: string; from: number; text: string; at: number; nick: string; hue: number; mine: boolean };
export type Mode = "end" | "next" | "now";
type Status = "connecting" | "open" | "closed" | "gone";

// 小部屋への　糸一本を　手繰り寄す
export class Room {
  status = $state<Status>("connecting");
  me = $state(0);
  station = $state("");
  members = $state<Record<number, Member>>({});
  queue = $state<QItem[]>([]);
  cur = $state<number | null>(null);
  playing = $state(false);
  pos = $state(0);
  at = $state(0);
  lines = $state<Line[]>([]);
  current = $derived(this.queue.find((i) => i.qid === this.cur));

  readonly clock = new Clock();
  readonly keys: Keys;
  private ws: WebSocket | null = null;
  private seen = new Set<string>();
  private outbox: string[] = [];
  private retry = 0;
  private pinger = 0;
  private stopped = false;

  constructor(
    readonly hub: string,
    readonly id: string,
    readonly secret: string,
    public profile: Profile,
  ) {
    this.keys = derive(secret);
  }

  get pass(): string {
    return `${this.id}.${this.keys.ticket}`;
  }

  // 戸を開き　名乗りを封じ　席に着く
  connect(): void {
    if (this.stopped) return;
    this.status = "connecting";
    const url = `${this.hub.replace(/^http/, "ws")}/api/rooms/${this.id}/ws?t=${encodeURIComponent(this.keys.ticket)}`;
    const ws = new WebSocket(url);
    let opened = false;
    this.ws = ws;
    ws.onopen = () => {
      opened = true;
      this.retry = 0;
      this.status = "open";
      this.raw({ t: "profile", ct: seal(this.keys.seal, this.profile) });
      for (const m of this.outbox.splice(0)) ws.send(m);
      this.burst();
    };
    ws.onmessage = (e) => this.receive(JSON.parse(e.data));
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      clearInterval(this.pinger);
      if (this.stopped) return;
      this.status = "closed";
      const wait = Math.min(15000, 600 * 2 ** this.retry++);
      const again = () => setTimeout(() => this.connect(), wait);
      if (opened) return again();
      this.exists().then((alive) => (alive ? again() : ((this.status = "gone"), (this.stopped = true))));
    };
  }

  // 応えなき　戸はもう誰も　住まぬかと
  private async exists(): Promise<boolean> {
    try {
      const r = await fetch(`${this.hub}/api/rooms/${this.id}?t=${encodeURIComponent(this.keys.ticket)}`);
      return r.status !== 404 && r.status !== 403;
    } catch {
      return true;
    }
  }

  close(): void {
    this.stopped = true;
    clearInterval(this.pinger);
    this.ws?.close();
    this.ws = null;
  }

  // 時を問う　矢継ぎ早にと　五たびほど
  private burst(): void {
    clearInterval(this.pinger);
    let n = 0;
    const ping = () => this.raw({ t: "ping", c: this.clock.local() });
    ping();
    this.pinger = window.setInterval(() => {
      ping();
      if (++n === 5) {
        clearInterval(this.pinger);
        this.pinger = window.setInterval(ping, 4000);
      }
    }, 150);
  }

  private raw(m: object): void {
    const text = JSON.stringify(m);
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(text);
    else if (!(m as { t: string }).t.match(/^(ping|draft)$/)) this.outbox.push(text);
  }

  // 届く声　型を見分けて　帳に書く
  private receive(m: any): void {
    switch (m.t) {
      case "welcome": {
        this.me = m.you;
        this.station = m.station;
        const members: Record<number, Member> = {};
        for (const x of m.members) members[x.id] = { id: x.id, profile: x.ct ? unseal(this.keys.seal, x.ct) : null, draft: "" };
        members[m.you] = { id: m.you, profile: this.profile, draft: "" };
        this.members = members;
        for (const line of m.backlog) this.hear(line);
        break;
      }
      case "state":
        this.queue = m.queue;
        this.cur = m.cur;
        this.playing = m.playing;
        this.pos = m.pos;
        this.at = m.at;
        break;
      case "join":
        this.members[m.id] = { id: m.id, profile: null, draft: "" };
        break;
      case "leave":
        delete this.members[m.id];
        break;
      case "profile": {
        const p = unseal<Profile>(this.keys.seal, m.ct);
        const member = this.members[m.id] ?? (this.members[m.id] = { id: m.id, profile: null, draft: "" });
        if (p) member.profile = { nick: String(p.nick).slice(0, 24), hue: Number(p.hue) % 360 };
        break;
      }
      case "chat":
        this.hear(m);
        break;
      case "draft": {
        const d = unseal<{ text: string }>(this.keys.seal, m.ct);
        const member = this.members[m.from];
        if (member && d) member.draft = String(d.text).slice(0, 2000);
        break;
      }
      case "pong":
        this.clock.sample(m.c, m.s);
        break;
    }
  }

  // 紙切れの　文字を開いて　声とする
  private hear(m: { from: number; ct: string; at: number }): void {
    if (this.seen.has(m.ct)) return;
    this.seen.add(m.ct);
    const body = unseal<{ text: string }>(this.keys.seal, m.ct);
    if (!body) return;
    const who = this.members[m.from];
    if (who) who.draft = "";
    const p = who?.profile ?? (m.from === this.me ? this.profile : null);
    this.lines.push({
      key: m.ct.slice(0, 16),
      from: m.from,
      text: String(body.text).slice(0, 4000),
      at: m.at,
      nick: p?.nick ?? "·",
      hue: p?.hue ?? 0,
      mine: m.from === this.me,
    });
    if (this.lines.length > 400) this.lines.splice(0, this.lines.length - 400);
  }

  // 針の位置　揃えし刻の　物差しで
  position(now = this.clock.now()): number {
    return this.playing && now > this.at ? this.pos + (now - this.at) / 1000 : this.pos;
  }

  say(text: string): void {
    this.raw({ t: "chat", ct: seal(this.keys.seal, { text }) });
  }

  draft(text: string): void {
    this.raw({ t: "draft", ct: seal(this.keys.seal, { text }) });
  }

  rename(profile: Profile): void {
    this.profile = profile;
    const me = this.members[this.me];
    if (me) me.profile = profile;
    this.raw({ t: "profile", ct: seal(this.keys.seal, profile) });
  }

  enqueue(tracks: Snap[], mode: Mode = "end"): void {
    if (tracks.length) this.raw({ t: "enqueue", tracks, mode });
  }

  play(qid?: number, pos?: number): void {
    this.raw({ t: "play", qid, pos });
  }

  pause(): void {
    this.raw({ t: "pause" });
  }

  seek(pos: number): void {
    this.raw({ t: "seek", pos });
  }

  next(): void {
    this.raw({ t: "next" });
  }

  prev(): void {
    this.raw({ t: "prev" });
  }

  remove(qid: number): void {
    this.raw({ t: "remove", qid });
  }

  ended(qid: number): void {
    this.raw({ t: "ended", qid });
  }
}
