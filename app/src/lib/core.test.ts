import { describe, expect, it } from "vitest";
import { Clock } from "./clock";
import { derive, newSecret, seal, sealText, unb64u, unseal, unsealText } from "./crypto";
import { parseTarget, roomLink } from "./link";
import { activeLine, parseLrc } from "./lrc";

describe("lrc", () => {
  // 刻と詞　訳も並べて　読み解けり
  it("parses stamps, translations and offsets", () => {
    const l = parseLrc("[ti:x]\n[offset:500]\n[00:01.50]hello\n[00:01.50]你好\n[00:03.00][00:05.25]again\n[00:04.00]\n");
    expect(l.synced).toBe(true);
    expect(l.lines.map((x) => [x.t, x.text, x.sub])).toEqual([
      [1, "hello", "你好"],
      [2.5, "again", undefined],
      [3.5, "", undefined],
      [4.75, "again", undefined],
    ]);
    expect(activeLine(l.lines, 0.5)).toBe(-1);
    expect(activeLine(l.lines, 2.6)).toBe(1);
    expect(activeLine(l.lines, 99)).toBe(3);
  });

  // 刻のなき　詞はただ並ぶ　紙のまま
  it("keeps plain lyrics unsynced", () => {
    const l = parseLrc("one\ntwo");
    expect(l.synced).toBe(false);
    expect(l.lines.map((x) => x.text)).toEqual(["one", "two"]);
  });
});

describe("link", () => {
  // 貼られたる　道の名前を　読み違えず
  it("reads station and room links", () => {
    expect(parseTarget("https://a.b/#/s/kksk/abcdefghijk", "")).toEqual({
      kind: "station",
      hub: "https://a.b",
      station: "kksk",
      key: "abcdefghijk",
    });
    const link = roomLink("https://a.b", "room1234", "Zm9vYmFyYmF6cXV4");
    expect(parseTarget(link, "")).toMatchObject({ kind: "room", hub: "https://a.b", room: "room1234" });
    expect(parseTarget("#/r/room1234/Zm9vYmFyYmF6cXV4", "http://x")).toMatchObject({ hub: "http://x" });
    expect(parseTarget("nonsense", "http://x")).toEqual({ kind: "none" });
  });
});

describe("crypto", () => {
  // 封じては　同じ鍵にて　また開く
  it("seals for the room and nobody else", () => {
    const secret = newSecret();
    const a = derive(secret);
    const b = derive(secret);
    expect(a.ticket).toBe(b.ticket);
    expect(a.ticket).toHaveLength(43);
    const ct = seal(a.seal, { text: "晚风" });
    expect(unseal(b.seal, ct)).toEqual({ text: "晚风" });
    expect(unseal(derive(newSecret()).seal, ct)).toBeNull();
    expect(seal(a.seal, 1)).not.toBe(seal(a.seal, 1));
  });

  it("seals chat as four bytes a character", () => {
    const a = derive(newSecret());
    const ct = sealText(a.seal, "晚风🎐 ok");
    expect(unb64u(ct).length - 40).toBe(6 * 4);
    expect(unsealText(a.seal, ct)).toBe("晚风🎐 ok");
    expect(unsealText(a.seal, seal(a.seal, { text: "old" }))).toBe("old");
  });
});

describe("clock", () => {
  // 遅き問い　早き答えに　席譲る
  it("trusts the fastest round trip", () => {
    let t = 1000;
    const c = new Clock(() => t);
    c.sample(900, 5050);
    expect(c.offset).toBeCloseTo(4100);
    t = 2000;
    c.sample(1980, 6080);
    expect(c.rtt).toBe(20);
    expect(c.offset).toBeCloseTo(4100 - 10 * 0.25);
    t = 3000;
    c.sample(2600, 7500);
    expect(c.rtt).toBe(20);
    t = 4000;
    c.sample(3990, 8395);
    expect(c.offset).toBeCloseTo(4400);
  });
});
