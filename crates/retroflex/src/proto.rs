use serde::{Deserialize, Serialize};

// 一曲の　名と声と長さ　札に書く
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    pub duration: f64,
    pub mime: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default)]
    pub lyrics: bool,
}

// 棚ひとつ　持ち主の名を　背に記す
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub station: String,
    pub tracks: Vec<Track>,
}

// 頼みごと　四つの型に　折りたたむ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Op {
    Library,
    Audio { track: String },
    Cover { cover: String, thumb: bool },
    Lyrics { track: String },
}

// 駅からの　返しの文は　短くて
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Up {
    Hello { name: String, token: String, key: String, version: u32 },
    Head { id: u32, status: u16, headers: Vec<(String, String)> },
    End { id: u32 },
    Changed,
}

// 中枢の　声は頼みと　礼ばかり
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Down {
    Welcome,
    Reject { reason: String },
    Req {
        id: u32,
        #[serde(flatten)]
        op: Op,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        range: Option<String>,
    },
    Ack { id: u32, n: u32 },
    Cancel { id: u32 },
}

pub const PROTOCOL: u32 = 1;
pub const CHUNK: usize = 64 * 1024;
pub const WINDOW: usize = 1024 * 1024;

// 荷の頭　四つの字にて　宛を書く
pub fn frame(id: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data.len());
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(data);
    out
}

// 荷を解く　宛名を剥げば　中身あり
pub fn unframe(buf: &[u8]) -> Option<(u32, &[u8])> {
    let head: [u8; 4] = buf.get(..4)?.try_into().ok()?;
    Some((u32::from_be_bytes(head), &buf[4..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 往き来して　形崩さぬ　文の舟
    #[test]
    fn down_roundtrip() {
        let req = Down::Req { id: 7, op: Op::Cover { cover: "ab".into(), thumb: true }, range: None };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"t":"req","id":7,"op":"cover","cover":"ab","thumb":true}"#);
        match serde_json::from_str::<Down>(&json).unwrap() {
            Down::Req { id: 7, op: Op::Cover { thumb: true, .. }, range: None } => {}
            other => panic!("{other:?}"),
        }
    }

    // 四つ字の　宛名違えず　荷は届く
    #[test]
    fn frames() {
        let f = frame(258, b"hi");
        assert_eq!(unframe(&f), Some((258, &b"hi"[..])));
        assert_eq!(unframe(b"abc"), None);
    }
}
