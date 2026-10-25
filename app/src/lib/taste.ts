const KEY = "retroflex.heard";

function load(): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(raw) ? raw.filter((a) => typeof a === "string") : [];
  } catch {
    return [];
  }
}

// 聴いた名を　新しい順に　書き留める
export function remember(artist: string): void {
  if (!artist) return;
  const list = load().filter((a) => a !== artist);
  list.unshift(artist);
  try {
    localStorage.setItem(KEY, JSON.stringify(list.slice(0, 40)));
  } catch {}
}

export function taste(extra: string[] = []): string[] {
  return [...new Set([...extra, ...load()])].slice(0, 50);
}
