use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use data_encoding::HEXLOWER;
use hmac::{Hmac, KeyInit, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sha2::{Digest, Sha256};

const STRICT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');
const PATH: &AsciiSet = &STRICT.remove(b'/');

// 倉の鍵　署名の作法　覚え書き
#[derive(Clone)]
pub struct S3 {
    scheme: String,
    host: String,
    region: String,
    key: String,
    secret: String,
}

// 一つの品　名と重さと　刻印と
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub key: String,
    pub size: u64,
    pub etag: String,
}

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("hmac key");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

// 日を数え　暦の上の　年月日
fn stamp(secs: u64) -> (String, String) {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let date = format!("{y:04}{m:02}{d:02}");
    let full = format!("{date}T{:02}{:02}{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60);
    (date, full)
}

fn xml_text(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn tag<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let a = body.find(&open)? + open.len();
    let b = body[a..].find(&close)? + a;
    Some(&body[a..b])
}

impl S3 {
    pub fn new(endpoint: &str, region: &str, key: &str, secret: &str) -> Result<Self> {
        let Some((scheme, rest)) = endpoint.trim_end_matches('/').split_once("://") else {
            bail!("endpoint needs a scheme: {endpoint}");
        };
        Ok(Self { scheme: scheme.into(), host: rest.into(), region: region.into(), key: key.into(), secret: secret.into() })
    }

    // 期限つき　誰でも開ける　封の文
    pub fn presign(&self, method: &str, path: &str, query: &[(&str, &str)], expires: u64) -> String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
        self.presign_at(method, path, query, expires, now)
    }

    fn presign_at(&self, method: &str, path: &str, query: &[(&str, &str)], expires: u64, now: u64) -> String {
        let (date, full) = stamp(now);
        let scope = format!("{date}/{}/s3/aws4_request", self.region);
        let credential = format!("{}/{scope}", self.key);
        let expires = expires.to_string();
        let mut params: Vec<(String, String)> = query.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        params.extend([
            ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
            ("X-Amz-Credential".into(), credential),
            ("X-Amz-Date".into(), full.clone()),
            ("X-Amz-Expires".into(), expires),
            ("X-Amz-SignedHeaders".into(), "host".into()),
        ]);
        let mut encoded: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (utf8_percent_encode(k, STRICT).to_string(), utf8_percent_encode(v, STRICT).to_string()))
            .collect();
        encoded.sort();
        let query_str = encoded.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
        let uri = utf8_percent_encode(path, PATH).to_string();
        let canonical = format!("{method}\n{uri}\n{query_str}\nhost:{}\n\nhost\nUNSIGNED-PAYLOAD", self.host);
        let to_sign = format!("AWS4-HMAC-SHA256\n{full}\n{scope}\n{}", HEXLOWER.encode(&Sha256::digest(canonical.as_bytes())));
        let k = hmac(format!("AWS4{}", self.secret).as_bytes(), &date);
        let k = hmac(&k, &self.region);
        let k = hmac(&k, "s3");
        let k = hmac(&k, "aws4_request");
        let signature = HEXLOWER.encode(&hmac(&k, &to_sign));
        format!("{}://{}{uri}?{query_str}&X-Amz-Signature={signature}", self.scheme, self.host)
    }

    pub fn object_path(bucket: &str, key: &str) -> String {
        format!("/{bucket}/{key}")
    }

    // 倉の中　頁を繰りて　品を数う
    pub fn list(&self, agent: &ureq::Agent, bucket: &str, prefix: &str) -> Result<Vec<Object>> {
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut q = vec![("list-type", "2"), ("prefix", prefix)];
            if let Some(t) = token.as_deref() {
                q.push(("continuation-token", t));
            }
            let url = self.presign("GET", &format!("/{bucket}"), &q, 300);
            let body = agent.get(&url).call()?.body_mut().read_to_string()?;
            out.extend(parse_list(&body));
            match (tag(&body, "IsTruncated"), tag(&body, "NextContinuationToken")) {
                (Some("true"), Some(next)) => token = Some(xml_text(next)),
                _ => return Ok(out),
            }
        }
    }
}

// 目録の　札を一枚　剥がし読む
fn parse_list(body: &str) -> Vec<Object> {
    body.split("<Contents>")
        .skip(1)
        .filter_map(|chunk| {
            let chunk = chunk.split("</Contents>").next()?;
            Some(Object {
                key: xml_text(tag(chunk, "Key")?),
                size: tag(chunk, "Size")?.parse().ok()?,
                etag: xml_text(tag(chunk, "ETag").unwrap_or_default()).trim_matches('"').to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 手本ある　署名と同じ　筆の跡
    #[test]
    fn matches_the_aws_presign_example() {
        let s3 = S3::new(
            "https://examplebucket.s3.amazonaws.com",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        )
        .unwrap();
        let url = s3.presign_at("GET", "/test.txt", &[], 86400, 1_369_353_600);
        assert!(url.ends_with("X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"), "{url}");
    }

    // 暦読み　閏の年も　違えずに
    #[test]
    fn dates() {
        assert_eq!(stamp(1_369_353_600), ("20130524".into(), "20130524T000000Z".into()));
        assert_eq!(stamp(951_782_400).0, "20000229");
    }

    // 札の束　解けば名と量　手に残る
    #[test]
    fn parses_listings() {
        let xml = r#"<ListBucketResult><IsTruncated>false</IsTruncated><Contents><Key>a/Tom &amp; Jerry.m4a</Key><Size>12</Size><ETag>"abc"</ETag></Contents><Contents><Key>b.lrc</Key><Size>3</Size></Contents></ListBucketResult>"#;
        let objs = parse_list(xml);
        assert_eq!(objs[0], Object { key: "a/Tom & Jerry.m4a".into(), size: 12, etag: "abc".into() });
        assert_eq!(objs[1].size, 3);
    }
}
