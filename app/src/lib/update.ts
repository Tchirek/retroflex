const REPO = "Tchirek/retroflex";

export type Update = { version: string; url: string };

function newer(a: string, b: string): boolean {
  const pa = a.replace(/^v/, "").split(".").map(Number);
  const pb = b.replace(/^v/, "").split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) > (pb[i] ?? 0);
  }
  return false;
}

// 新しき　版の便りを　問いにゆく
export async function checkUpdate(current: string): Promise<Update | null> {
  try {
    const r = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { accept: "application/vnd.github+json" } });
    if (!r.ok) return null;
    const rel = await r.json();
    const tag = String(rel.tag_name ?? "");
    return tag && newer(tag, current) ? { version: tag, url: String(rel.html_url) } : null;
  } catch {
    return null;
  }
}
