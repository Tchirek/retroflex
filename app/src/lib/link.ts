export type Target =
  | { kind: "station"; hub: string; station: string; key: string }
  | { kind: "room"; hub: string; room: string; secret: string }
  | { kind: "none" };

const ROUTE = /#\/([sr])\/([a-z0-9-]{1,32})\/([A-Za-z0-9_-]{8,128})/;

// 道標　駅か部屋かを　読み分ける
export function parseTarget(input: string, fallbackHub: string): Target {
  const text = input.trim();
  const m = ROUTE.exec(text);
  if (!m) return { kind: "none" };
  const before = text.slice(0, m.index).replace(/\/+$/, "");
  const hub = /^https?:\/\//.test(before) ? before : fallbackHub;
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

export function hash(target: Target): string {
  if (target.kind === "station") return target.key ? `#/s/${target.station}/${target.key}` : "";
  if (target.kind === "room") return `#/r/${target.room}/${target.secret}`;
  return "";
}
