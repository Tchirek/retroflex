import { xchacha20poly1305 } from "@noble/ciphers/chacha.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";

const enc = new TextEncoder();
const dec = new TextDecoder();

// 二進の　粒を文字へと　織りなおす
export function b64u(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

// 文字の帯　ほどけば元の　粒に還る
export function unb64u(text: string): Uint8Array {
  const s = atob(text.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(s, (c) => c.charCodeAt(0));
}

export function randomBytes(n: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(n));
}

// 合言葉　ひとつ生まれて　部屋となる
export function newSecret(): string {
  return b64u(randomBytes(16));
}

export type Keys = { ticket: string; seal: Uint8Array };

// 一つ種　札と錠とに　分かれ咲く
export function derive(secret: string): Keys {
  const ikm = unb64u(secret);
  const salt = enc.encode("retroflex/v1");
  return {
    ticket: b64u(hkdf(sha256, ikm, salt, enc.encode("ticket"), 32)),
    seal: hkdf(sha256, ikm, salt, enc.encode("seal"), 32),
  };
}

// 封をして　中枢の目には　ただの霧
export function seal(key: Uint8Array, value: unknown): string {
  const nonce = randomBytes(24);
  const body = xchacha20poly1305(key, nonce).encrypt(enc.encode(JSON.stringify(value)));
  const out = new Uint8Array(nonce.length + body.length);
  out.set(nonce);
  out.set(body, nonce.length);
  return b64u(out);
}

// 封を切る　鍵持つ者に　だけ声が
export function unseal<T>(key: Uint8Array, text: string): T | null {
  try {
    const raw = unb64u(text);
    const plain = xchacha20poly1305(key, raw.subarray(0, 24)).decrypt(raw.subarray(24));
    return JSON.parse(dec.decode(plain)) as T;
  } catch {
    return null;
  }
}
