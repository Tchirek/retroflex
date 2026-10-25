import { b64u, unb64u } from "./crypto";

export type Target =
  | { kind: "station"; hub: string; station: string; key: string }
  | { kind: "list"; hub: string; station: string; ids: string[] }
  | { kind: "room"; hub: string; room: string; secret: string }
  | { kind: "none" };

const ROUTE = /#\/([sr])\/([a-z0-9-]{1,32})\/([A-Za-z0-9_-]{8,128})/;
const LIST = /#\/l\/([a-z0-9-]{1,32})\/([A-Za-z0-9_-]{11,6000})/;

// 歌の札　八つの粒に　詰め直す
export function packIds(ids: string[]): string {
  const hex = ids.filter((id) => /^[0-9a-f]{16}$/.test(id)).join("");
  return b64u(Uint8Array.from(hex.match(/../g) ?? [], (h) => parseInt(h, 16)));
}

export function unpackIds(text: string): string[] {
  try {
    const raw = unb64u(text);
    const hex = Array.from(raw, (b) => b.toString(16).padStart(2, "0")).join("");
    return hex.match(/.{16}/g) ?? [];
  } catch {
    return [];
  }
}

function hubOf(text: string, index: number, fallbackHub: string): string {
  const before = text.slice(0, index).replace(/\/+$/, "");
  return /^https?:\/\//.test(before) ? before : fallbackHub;
}

// 道標　駅か部屋かを　読み分ける
export function parseTarget(input: string, fallbackHub: string): Target {
  const text = input.trim();
  const l = LIST.exec(text);
  if (l) {
    const hub = hubOf(text, l.index, fallbackHub);
    const ids = unpackIds(l[2]);
    return hub && ids.length ? { kind: "list", hub, station: l[1], ids } : { kind: "none" };
  }
  const m = ROUTE.exec(text);
  if (!m) return { kind: "none" };
  const hub = hubOf(text, m.index, fallbackHub);
  if (!hub) return { kind: "none" };
  return m[1] === "s"
    ? { kind: "station", hub, station: m[2], key: m[3] }
    : { kind: "room", hub, room: m[2], secret: m[3] };
}

export function stationLink(hub: string, station: string, key: string): string {
  return `${hub}/#/s/${station}/${key}`;
}

// 合言葉　鍵の欠片は　紙の端
export function roomLink(hub: string, room: string, secret: string): string {
  return `${hub}/#/r/${room}/${secret}`;
}

export function listLink(hub: string, station: string, ids: string[]): string {
  return `${hub}/#/l/${station}/${packIds(ids)}`;
}

export function hash(target: Target): string {
  if (target.kind === "station") return target.key ? `#/s/${target.station}/${target.key}` : "";
  if (target.kind === "room") return `#/r/${target.room}/${target.secret}`;
  if (target.kind === "list") return `#/l/${target.station}/${packIds(target.ids)}`;
  return "";
}
