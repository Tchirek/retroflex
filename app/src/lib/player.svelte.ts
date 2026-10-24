import type { Station } from "./api";
import type { Room } from "./room.svelte";

const HARD = 0.35;
const SOFT = 0.015;
const NUDGE = 0.04;

function stored(key: string, fallback: number): number {
  try {
    const v = Number(localStorage.getItem(key));
    return Number.isFinite(v) && localStorage.getItem(key) !== null ? v : fallback;
  } catch {
    return fallback;
  }
}

// 遠き耳　同じ拍へと　寄せてゆく
export class Player {
  blocked = $state(false);
  volume = $state(stored("retroflex.volume", 1));
  readonly audio: HTMLAudioElement;
  private latency = 0;
  private src = "";
  private timer = 0;
  private kick = 0;
  private ctx: AudioContext | null = null;
  private session = "";

  constructor(
    private room: Room,
    private station: () => Station | null,
  ) {
    const a = new Audio();
    a.crossOrigin = "anonymous";
    a.preload = "auto";
    a.addEventListener("ended", () => {
      if (this.room.cur != null) this.room.ended(this.room.cur);
    });
    this.audio = a;
    this.timer = window.setInterval(() => this.align(), 200);
    this.bindSession();
  }

  destroy(): void {
    clearInterval(this.timer);
    clearTimeout(this.kick);
    this.audio.pause();
    this.audio.removeAttribute("src");
    this.ctx?.close();
  }

  setVolume(v: number): void {
    this.volume = Math.min(1, Math.max(0, v));
    try {
      localStorage.setItem("retroflex.volume", String(this.volume));
    } catch {}
  }

  // 指先の　一打で耳の　封を解く
  unlock(): void {
    this.blocked = false;
    this.measure();
    this.start();
  }

  // 声の遅れ　機器の告げるを　鵜呑みにす
  private async measure(): Promise<void> {
    try {
      this.ctx ??= new AudioContext();
      await this.ctx.resume();
      const l = (this.ctx as AudioContext & { outputLatency?: number }).outputLatency || this.ctx.baseLatency || 0;
      this.latency = Math.min(0.4, Math.max(0, l));
      await this.ctx.suspend();
    } catch {}
  }

  // 拍を見て　速めて遅めて　時に跳ぶ
  private align(): void {
    const { room, audio: a } = this;
    const st = this.station();
    a.volume = this.volume;
    const item = room.current;
    if (!item || !st) {
      if (!a.paused) a.pause();
      if (this.src) {
        this.src = "";
        a.removeAttribute("src");
        a.load();
      }
      return;
    }
    const url = st.audio(item.id);
    if (url !== this.src) {
      this.src = url;
      a.src = url;
      a.playbackRate = 1;
      this.describe(st);
    }
    if (a.readyState < 1 || a.seeking) return;

    const now = room.clock.now();
    if (!room.playing) {
      clearTimeout(this.kick);
      if (!a.paused) a.pause();
      if (Math.abs(a.currentTime - room.pos) > 0.08) a.currentTime = room.pos;
      return;
    }
    const lead = room.at - now - this.latency * 1000;
    if (lead > 0) {
      if (!a.paused) a.pause();
      if (Math.abs(a.currentTime - room.pos) > 0.03) a.currentTime = room.pos;
      clearTimeout(this.kick);
      this.kick = window.setTimeout(() => this.start(), lead);
      return;
    }
    if (this.blocked) return;
    if (a.paused) return this.start();
    if (a.readyState < 3) return;

    const delta = a.currentTime - this.target(now);
    if (Math.abs(delta) > HARD) {
      a.playbackRate = 1;
      a.currentTime = this.target(now) + 0.04;
    } else if (Math.abs(delta) > SOFT) {
      a.playbackRate = 1 - Math.max(-NUDGE, Math.min(NUDGE, delta * 0.6));
    } else if (a.playbackRate !== 1) {
      a.playbackRate = 1;
    }
  }

  // 耳に着く　その刻をこそ　合わせけれ
  private target(now: number): number {
    return this.room.position(now + this.latency * 1000);
  }

  private start(): void {
    const { room, audio: a } = this;
    if (!room.current || !room.playing) return;
    const target = this.target(room.clock.now());
    if (a.readyState >= 1 && Math.abs(a.currentTime - target) > 0.05) a.currentTime = Math.max(0, target);
    a.play().then(
      () => (this.blocked = false),
      (err: DOMException) => {
        if (err?.name === "NotAllowedError") this.blocked = true;
      },
    );
  }

  // 鍵盤の　外の釦も　部屋へ告ぐ
  private bindSession(): void {
    if (!("mediaSession" in navigator)) return;
    const ms = navigator.mediaSession;
    const on = (action: MediaSessionAction, fn: MediaSessionActionHandler) => {
      try {
        ms.setActionHandler(action, fn);
      } catch {}
    };
    on("play", () => (this.blocked ? this.unlock() : this.room.play()));
    on("pause", () => this.room.pause());
    on("nexttrack", () => this.room.next());
    on("previoustrack", () => this.room.prev());
    on("seekto", (d) => d.seekTime != null && this.room.seek(d.seekTime));
  }

  private describe(st: Station): void {
    const item = this.room.current;
    if (!item || !("mediaSession" in navigator) || this.session === item.id) return;
    this.session = item.id;
    const art = st.cover(item.cover, false);
    navigator.mediaSession.metadata = new MediaMetadata({
      title: item.title,
      artist: item.artist,
      album: item.album,
      artwork: art ? [{ src: art, sizes: "1024x1024" }] : [],
    });
  }
}
