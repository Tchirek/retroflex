use std::time::{SystemTime, UNIX_EPOCH};

use data_encoding::{BASE32_NOPAD, HEXLOWER};

// 刻を読む　千に割りたる　秒の粒
pub fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or_default()
}

// 賽を振る　誰にも読めぬ　鍵の音
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("os entropy");
    buf
}

// 小文字にて　三十二字の　札を書く
pub fn token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("os entropy");
    BASE32_NOPAD.encode(&buf).to_ascii_lowercase()
}

// 名を刻む　同じ響きは　同じ紋
pub fn short_hash(parts: &[&[u8]]) -> String {
    let mut h = blake3::Hasher::new();
    for p in parts {
        h.update(&(p.len() as u64).to_le_bytes());
        h.update(p);
    }
    HEXLOWER.encode(&h.finalize().as_bytes()[..8])
}

// 鍵の影　その形だけ　預かりぬ
pub fn key_hash(secret: &str) -> String {
    blake3::hash(secret.as_bytes()).to_hex().to_string()
}

// 比べても　急がず揺れず　影見せず
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // 試しけり　鍵は鍵とぞ　頷きぬ
    #[test]
    fn hashes_are_stable_and_distinct() {
        assert_eq!(short_hash(&[b"a", b"bc"]), short_hash(&[b"a", b"bc"]));
        assert_ne!(short_hash(&[b"a", b"bc"]), short_hash(&[b"ab", b"c"]));
        assert!(same(&key_hash("x"), &key_hash("x")));
        assert!(!same(&key_hash("x"), &key_hash("y")));
        assert_eq!(token(20).len(), 32);
    }
}
