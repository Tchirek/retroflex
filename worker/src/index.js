// 倉の番　札の印を　確かめる
const enc = new TextEncoder();

function hex(buf) {
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

// 比べても　急がず揺れず　影見せず
function same(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

const cors = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, HEAD, OPTIONS",
  "access-control-allow-headers": "Range",
  "access-control-expose-headers": "Content-Length, Content-Range, Accept-Ranges, ETag",
  "access-control-max-age": "86400",
};

function deny(status) {
  return new Response(null, { status, headers: cors });
}

export default {
  // 名と期限　鍵の筆跡　揃うなら
  async fetch(request, env) {
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: cors });
    if (request.method !== "GET" && request.method !== "HEAD") return deny(405);

    const url = new URL(request.url);
    const key = decodeURIComponent(url.pathname.slice(1));
    const exp = Number(url.searchParams.get("exp"));
    const sig = url.searchParams.get("sig") || "";
    if (!key || !Number.isFinite(exp) || exp * 1000 < Date.now()) return deny(403);

    const mac = await crypto.subtle.importKey("raw", enc.encode(env.MEDIA_SECRET), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
    const want = hex(await crypto.subtle.sign("HMAC", mac, enc.encode(`${key}\n${exp}`)));
    if (!same(want, sig)) return deny(403);

    // 頼まれし　幅だけ倉から　汲み出して
    const obj = await env.BUCKET.get(key, { range: request.headers, onlyIf: request.headers });
    if (obj === null) return deny(404);

    const headers = new Headers(cors);
    obj.writeHttpMetadata(headers);
    headers.set("etag", obj.httpEtag);
    headers.set("accept-ranges", "bytes");
    headers.set("cache-control", "private, max-age=21600");

    if (!("body" in obj)) return new Response(null, { status: 304, headers });

    let status = 200;
    let length = obj.size;
    const r = obj.range;
    if (request.headers.has("range") && r) {
      const suffix = typeof r.suffix === "number";
      const start = suffix ? obj.size - r.suffix : (r.offset ?? 0);
      const end = suffix ? obj.size - 1 : typeof r.length === "number" ? start + r.length - 1 : obj.size - 1;
      length = end - start + 1;
      status = 206;
      headers.set("content-range", `bytes ${start}-${end}/${obj.size}`);
    }
    headers.set("content-length", String(length));
    return new Response(request.method === "HEAD" ? null : obj.body, { status, headers });
  },
};
