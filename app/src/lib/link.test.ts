import { describe, expect, it } from "vitest";
import { listLink, packIds, parseTarget, unpackIds } from "./link";

// 束の札　縮めて解けば　元のまま
describe("list links", () => {
  it("round-trips track ids", () => {
    const ids = ["0123456789abcdef", "fedcba9876543210"];
    expect(unpackIds(packIds(ids))).toEqual(ids);
    const t = parseTarget(listLink("https://hub.example", "kksk", ids), "");
    expect(t).toEqual({ kind: "list", hub: "https://hub.example", station: "kksk", ids });
  });
});
