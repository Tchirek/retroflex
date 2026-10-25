export type LyricLine = { t: number; text: string; sub?: string };
export type Lyrics = { lines: LyricLine[]; synced: boolean };

const STAMP = /\[(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;
const META = /^\[([a-z]+):(.*)\]$/i;
const WORD = /<\d{1,3}:\d{1,2}(?:[.:]\d{1,3})?>/g;

function seconds(m: string, s: string, frac?: string): number {
  const f = frac ? Number(frac) / 10 ** frac.length : 0;
  return Number(m) * 60 + Number(s) + f;
}

// 刻印を　拾い集めて　歌を編む
export function parseLrc(source: string): Lyrics {
  let offset = 0;
  const stamped: LyricLine[] = [];
  const plain: string[] = [];
  for (const raw of source.replace(/\r/g, "").split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    const meta = META.exec(line);
    if (meta && !/^\d/.test(meta[1])) {
      if (meta[1].toLowerCase() === "offset") offset = Number(meta[2]) / 1000 || 0;
      continue;
    }
    const times: number[] = [];
    let rest = line;
    STAMP.lastIndex = 0;
    for (;;) {
      const m = /^\[(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?\]/.exec(rest);
      if (!m) break;
      times.push(seconds(m[1], m[2], m[3]));
      rest = rest.slice(m[0].length);
    }
    const text = rest.replace(WORD, "").replace(STAMP, "").trim();
    if (times.length === 0) plain.push(text);
    for (const t of times) stamped.push({ t, text });
  }
  if (stamped.length === 0) {
    return { lines: plain.map((text) => ({ t: -1, text })), synced: false };
  }

  // 同じ刻　重なる声は　訳として
  stamped.sort((a, b) => a.t - b.t);
  const lines: LyricLine[] = [];
  for (const l of stamped) {
    const prev = lines[lines.length - 1];
    if (prev && Math.abs(prev.t - l.t) < 0.01 && prev.text && l.text) {
      if (!prev.sub) prev.sub = l.text;
      continue;
    }
    if (prev && !prev.text && !l.text) continue;
    lines.push({ t: l.t, text: l.text });
  }
  while (lines.length && !lines[lines.length - 1].text) lines.pop();
  for (const l of lines) l.t = Math.max(0, l.t - offset);
  return { lines, synced: true };
}

// 聴く人の　母の言葉を　名で知る
export function tongue(): string {
  const l = (navigator.languages?.[0] ?? navigator.language ?? "en").toLowerCase();
  if (l.startsWith("zh")) return /-(tw|hk|mo|hant)/.test(l) ? "zh-TW" : "zh-CN";
  return l.split("-")[0] || "en";
}

// 今の刻　どの一行に　宿るかと
export function activeLine(lines: LyricLine[], time: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].t <= time) {
      found = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return found;
}
