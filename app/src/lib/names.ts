const WORDS = [
  "耳机", "磁带", "纸条", "粉笔", "汽水", "晚风", "校服", "单车", "橡皮", "课桌",
  "窗台", "操场", "黑板", "风扇", "铅笔", "书包", "雨伞", "路灯", "随身听", "卡带",
  "暮色", "海盐", "橘子", "薄荷", "帆布鞋", "晚自习", "广播站", "便利店", "天台", "末班车",
];

export type Profile = { nick: string; hue: number };

// 名も無くて　教室の隅の　物の名を
export function randomProfile(): Profile {
  const pick = new Uint32Array(2);
  crypto.getRandomValues(pick);
  return { nick: WORDS[pick[0] % WORDS.length], hue: pick[1] % 360 };
}

// 一度得し　名はこの机に　刻みおく
export function loadProfile(): Profile {
  try {
    const p = JSON.parse(localStorage.getItem("retroflex.profile") ?? "null") as Profile | null;
    if (p && typeof p.nick === "string" && typeof p.hue === "number") return p;
  } catch {}
  const p = randomProfile();
  saveProfile(p);
  return p;
}

export function saveProfile(p: Profile): void {
  try {
    localStorage.setItem("retroflex.profile", JSON.stringify(p));
  } catch {}
}
