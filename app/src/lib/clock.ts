const KEEP = 12;
const JUMP_MS = 60;

type Sample = { rtt: number; offset: number };

// 往き還り　その短きを　信じたり
export class Clock {
  offset = 0;
  rtt = Infinity;
  private samples: Sample[] = [];
  private ready = false;

  constructor(private source: () => number = () => performance.timeOrigin + performance.now()) {}

  local(): number {
    return this.source();
  }

  now(): number {
    return this.source() + this.offset;
  }

  // 問うた刻　答えの刻を　並べ見る
  sample(sentAt: number, serverAt: number): void {
    const back = this.source();
    const rtt = back - sentAt;
    if (rtt < 0) return;
    this.samples.push({ rtt, offset: serverAt + rtt / 2 - back });
    if (this.samples.length > KEEP) this.samples.shift();
    const best = this.samples.reduce((a, b) => (b.rtt < a.rtt ? b : a));
    this.rtt = best.rtt;
    if (!this.ready || Math.abs(best.offset - this.offset) > JUMP_MS) {
      this.offset = best.offset;
      this.ready = true;
    } else {
      this.offset += (best.offset - this.offset) * 0.25;
    }
  }

  get synced(): boolean {
    return this.ready;
  }
}
