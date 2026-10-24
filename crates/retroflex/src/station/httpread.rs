use std::io::{self, Read, Seek, SeekFrom};

const BLOCK: u64 = 256 * 1024;
const KEEP: usize = 6;

// 遠き棚　頁を借りて　読むごとく
pub struct HttpReader {
    agent: ureq::Agent,
    url: String,
    size: u64,
    pos: u64,
    blocks: Vec<(u64, Vec<u8>)>,
}

impl HttpReader {
    pub fn new(agent: ureq::Agent, url: String, size: u64) -> Self {
        Self { agent, url, size, pos: 0, blocks: Vec::new() }
    }

    // 欠けし頁　その一枚を　取り寄せる
    fn block(&mut self, start: u64) -> io::Result<usize> {
        if let Some(i) = self.blocks.iter().position(|(s, _)| *s == start) {
            let hit = self.blocks.remove(i);
            self.blocks.push(hit);
            return Ok(self.blocks.len() - 1);
        }
        let end = (start + BLOCK).min(self.size) - 1;
        let mut resp = self
            .agent
            .get(&self.url)
            .header("Range", format!("bytes={start}-{end}"))
            .call()
            .map_err(io::Error::other)?;
        let mut data = Vec::with_capacity((end - start + 1) as usize);
        resp.body_mut()
            .as_reader()
            .take(end - start + 1)
            .read_to_end(&mut data)?;
        if self.blocks.len() >= KEEP {
            self.blocks.remove(0);
        }
        self.blocks.push((start, data));
        Ok(self.blocks.len() - 1)
    }
}

impl Read for HttpReader {
    // 栞から　続きの文字を　写しとる
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.size || buf.is_empty() {
            return Ok(0);
        }
        let start = self.pos / BLOCK * BLOCK;
        let i = self.block(start)?;
        let data = &self.blocks[i].1;
        let off = (self.pos - start) as usize;
        if off >= data.len() {
            return Ok(0);
        }
        let n = buf.len().min(data.len() - off);
        buf[..n].copy_from_slice(&data[off..off + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for HttpReader {
    // 指をさす　頁の位置を　改めて
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let next = match to {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::End(d) => self.size as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if next < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before start"));
        }
        self.pos = next as u64;
        Ok(self.pos)
    }
}
