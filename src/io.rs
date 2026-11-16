//! Shared plumbing for the compact binary formats (solutions, hash tables).
//!
//! Layout of every file: `magic[8] | version u32 | payload_len u64 | payload | checksum u64`, all
//! little-endian. f64 values are stored as their raw IEEE bits, indices and lengths as u32.
//! The checksum covers every byte before it. Readers return [`Error`] for any malformed input.

use crate::linalg::Mat;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    BadMagic,
    UnsupportedVersion(u32),
    Truncated,
    Corrupt(String),
    DimensionMismatch(String),
    Unsupported(String),
    Json(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::BadMagic => write!(f, "bad magic: not an RPOPT file of this kind"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported format version {v}"),
            Error::Truncated => write!(f, "file is truncated"),
            Error::Corrupt(s) => write!(f, "corrupt file: {s}"),
            Error::DimensionMismatch(s) => write!(f, "dimension mismatch: {s}"),
            Error::Unsupported(s) => write!(f, "unsupported: {s}"),
            Error::Json(s) => write!(f, "JSON error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub(crate) const HEADER_LEN: usize = 8 + 4 + 8;

/// 4-lane multiply-rotate hash over 8-byte words (not cryptographic; detects corruption).
pub fn checksum(bytes: &[u8]) -> u64 {
    const K: u64 = 0x517c_c1b7_2722_0a95;
    let mut h = [0x9E37_79B9_7F4A_7C15u64, 0xC2B2_AE3D_27D4_EB4F, 0x1656_67B1_9E37_79F9, 0x27D4_EB2F_1656_67C5];
    let mut blocks = bytes.chunks_exact(32);
    for b in &mut blocks {
        for l in 0..4 {
            let w = u64::from_le_bytes(b[l * 8..l * 8 + 8].try_into().unwrap());
            h[l] = (h[l].rotate_left(5) ^ w).wrapping_mul(K);
        }
    }
    let mut t = 0u64;
    for (i, &x) in blocks.remainder().iter().enumerate() {
        t = (t.rotate_left(5) ^ (x as u64 + ((i as u64) << 8))).wrapping_mul(K);
    }
    let mut r = t ^ bytes.len() as u64;
    for l in 0..4 {
        r = (r.rotate_left(5) ^ h[l]).wrapping_mul(K);
    }
    r
}

/// Incremental form of [`checksum`]: feeding a byte string in any chunking gives the same value.
pub(crate) struct StreamHasher {
    h: [u64; 4],
    pending: Vec<u8>,
    len: u64,
}

const HASH_K: u64 = 0x517c_c1b7_2722_0a95;

impl StreamHasher {
    pub fn new() -> Self {
        StreamHasher { h: [0x9E37_79B9_7F4A_7C15u64, 0xC2B2_AE3D_27D4_EB4F, 0x1656_67B1_9E37_79F9, 0x27D4_EB2F_1656_67C5], pending: Vec::with_capacity(32), len: 0 }
    }
    fn block(&mut self, b: &[u8]) {
        for l in 0..4 {
            let w = u64::from_le_bytes(b[l * 8..l * 8 + 8].try_into().unwrap());
            self.h[l] = (self.h[l].rotate_left(5) ^ w).wrapping_mul(HASH_K);
        }
    }
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.len += bytes.len() as u64;
        if !self.pending.is_empty() {
            let take = (32 - self.pending.len()).min(bytes.len());
            self.pending.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.pending.len() < 32 {
                return;
            }
            let b = std::mem::take(&mut self.pending);
            self.block(&b);
            self.pending = b;
            self.pending.clear();
        }
        let mut blocks = bytes.chunks_exact(32);
        for b in &mut blocks {
            self.block(b);
        }
        self.pending.extend_from_slice(blocks.remainder());
    }
    pub fn finish(&self) -> u64 {
        let mut t = 0u64;
        for (i, &x) in self.pending.iter().enumerate() {
            t = (t.rotate_left(5) ^ (x as u64 + ((i as u64) << 8))).wrapping_mul(HASH_K);
        }
        let mut r = t ^ self.len;
        for l in 0..4 {
            r = (r.rotate_left(5) ^ self.h[l]).wrapping_mul(HASH_K);
        }
        r
    }
}

/// Where a streaming [`Writer`] sends its full chunks.
pub(crate) enum Sink {
    /// Only counts the bytes (first pass: the payload length for the header).
    Count(u64),
    /// Writes the bytes to a file and hashes them.
    File(std::io::BufWriter<std::fs::File>, StreamHasher),
    /// Compares the bytes with an existing file instead of writing (verifies a saved file without a second in-memory copy).
    Compare(std::io::BufReader<std::fs::File>, StreamHasher),
}

pub(crate) struct Writer {
    pub buf: Vec<u8>,
    sink: Option<Sink>,
}

impl Writer {
    pub fn new(magic: &[u8; 8], version: u32, capacity: usize) -> Self {
        let mut buf = Vec::with_capacity(capacity + HEADER_LEN + 8);
        buf.extend_from_slice(magic);
        buf.extend_from_slice(&version.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes());
        Writer { buf, sink: None }
    }
    /// A writer that hands its buffer to `sink` whenever [`Writer::spill`] sees a full chunk, so
    /// memory stays small. `payload_len` goes into the header (use a `Sink::Count` pass to get it).
    pub fn streaming(magic: &[u8; 8], version: u32, payload_len: u64, sink: Sink) -> Self {
        let mut w = Writer::new(magic, version, 1 << 21);
        w.buf[12..20].copy_from_slice(&payload_len.to_le_bytes());
        w.sink = Some(sink);
        w
    }
    /// For a streaming writer with a full chunk buffered: passes it to the sink. No-op otherwise.
    pub fn spill(&mut self) -> Result<(), Error> {
        if self.sink.is_some() && self.buf.len() >= 1 << 20 {
            self.drain()?;
        }
        Ok(())
    }
    fn drain(&mut self) -> Result<(), Error> {
        use std::io::Write;
        match self.sink.as_mut() {
            Some(Sink::Count(n)) => *n += self.buf.len() as u64,
            Some(Sink::File(f, h)) => {
                h.update(&self.buf);
                f.write_all(&self.buf)?;
            }
            Some(Sink::Compare(f, h)) => {
                use std::io::Read;
                h.update(&self.buf);
                let mut got = vec![0u8; self.buf.len()];
                f.read_exact(&mut got)?;
                if got != self.buf {
                    return Err(Error::Corrupt("saved file differs from the encoding".into()));
                }
            }
            None => return Ok(()),
        }
        self.buf.clear();
        Ok(())
    }
    /// Ends a streaming writer. A counting sink returns the payload length (without header and
    /// checksum); a file sink appends the checksum, flushes, and returns the file size.
    pub fn finish_stream(mut self) -> Result<u64, Error> {
        use std::io::Write;
        self.drain()?;
        match self.sink.take() {
            Some(Sink::Count(n)) => Ok(n - HEADER_LEN as u64),
            Some(Sink::File(mut f, h)) => {
                f.write_all(&h.finish().to_le_bytes())?;
                f.flush()?;
                Ok(h_len(&h) + 8)
            }
            Some(Sink::Compare(mut f, h)) => {
                use std::io::Read;
                let mut tail = Vec::new();
                f.read_to_end(&mut tail)?;
                if tail != h.finish().to_le_bytes() {
                    return Err(Error::Corrupt("saved file has a wrong checksum or trailing bytes".into()));
                }
                Ok(h_len(&h) + 8)
            }
            None => Err(Error::Unsupported("not a streaming writer".into())),
        }
    }
    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    pub fn len(&mut self, n: usize) -> Result<(), Error> {
        let v = u32::try_from(n).map_err(|_| Error::Unsupported("length exceeds u32".into()))?;
        self.u32(v);
        Ok(())
    }
    pub fn f64s(&mut self, v: &[f64]) -> Result<(), Error> {
        self.len(v.len())?;
        self.raw_f64s(v);
        Ok(())
    }
    pub fn raw_f64s(&mut self, v: &[f64]) {
        self.buf.reserve(v.len() * 8);
        for x in v {
            self.buf.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    }
    pub fn raw_u32s(&mut self, v: &[u32]) {
        self.buf.reserve(v.len() * 4);
        for x in v {
            self.buf.extend_from_slice(&x.to_le_bytes());
        }
    }
    pub fn usizes(&mut self, v: &[usize]) -> Result<(), Error> {
        self.len(v.len())?;
        for &x in v {
            self.len(x)?;
        }
        Ok(())
    }
    pub fn mat(&mut self, m: &Mat) -> Result<(), Error> {
        if m.data.len() != m.rows * m.cols {
            return Err(Error::Corrupt("matrix data length does not match its shape".into()));
        }
        self.len(m.rows)?;
        self.len(m.cols)?;
        self.raw_f64s(&m.data);
        Ok(())
    }
    pub fn opt_f64s(&mut self, v: &Option<Vec<f64>>) -> Result<(), Error> {
        match v {
            None => self.u8(0),
            Some(v) => {
                self.u8(1);
                self.f64s(v)?;
            }
        }
        Ok(())
    }
    pub fn opt_usizes(&mut self, v: &Option<Vec<usize>>) -> Result<(), Error> {
        match v {
            None => self.u8(0),
            Some(v) => {
                self.u8(1);
                self.usizes(v)?;
            }
        }
        Ok(())
    }
    pub fn opt_f64(&mut self, v: Option<f64>) {
        match v {
            None => self.u8(0),
            Some(x) => {
                self.u8(1);
                self.f64(x);
            }
        }
    }
    /// Fills in the payload length and appends the checksum.
    pub fn finish(mut self) -> Vec<u8> {
        let plen = (self.buf.len() - HEADER_LEN) as u64;
        self.buf[12..20].copy_from_slice(&plen.to_le_bytes());
        let c = checksum(&self.buf);
        self.buf.extend_from_slice(&c.to_le_bytes());
        self.buf
    }
}

fn h_len(h: &StreamHasher) -> u64 {
    h.len
}

pub(crate) struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    /// Validates magic, version, length and checksum; the reader then covers the payload.
    pub fn open(bytes: &'a [u8], magic: &[u8; 8], version: u32) -> Result<Self, Error> {
        if bytes.len() < 8 {
            return Err(Error::Truncated);
        }
        if &bytes[..8] != magic {
            return Err(Error::BadMagic);
        }
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated);
        }
        let v = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if v != version {
            return Err(Error::UnsupportedVersion(v));
        }
        let plen = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        let total = (HEADER_LEN as u64).checked_add(plen).and_then(|x| x.checked_add(8));
        match total {
            Some(t) if t == bytes.len() as u64 => {}
            Some(t) if t > bytes.len() as u64 => return Err(Error::Truncated),
            _ => return Err(Error::Corrupt("payload length does not match file size".into())),
        }
        let body = bytes.len() - 8;
        let stored = u64::from_le_bytes(bytes[body..].try_into().unwrap());
        if checksum(&bytes[..body]) != stored {
            return Err(Error::Corrupt("checksum mismatch".into()));
        }
        Ok(Reader { b: &bytes[HEADER_LEN..body], p: 0 })
    }
    pub fn finish(&self) -> Result<(), Error> {
        if self.p == self.b.len() { Ok(()) } else { Err(Error::Corrupt("trailing bytes in payload".into())) }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if n > self.b.len() - self.p {
            return Err(Error::Truncated);
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_bits(self.u64()?))
    }
    pub fn len(&mut self) -> Result<usize, Error> {
        Ok(self.u32()? as usize)
    }
    pub fn raw_f64s(&mut self, n: usize) -> Result<Vec<f64>, Error> {
        let nb = n.checked_mul(8).ok_or(Error::Truncated)?;
        let s = self.take(nb)?;
        Ok(s.chunks_exact(8).map(|c| f64::from_bits(u64::from_le_bytes(c.try_into().unwrap()))).collect())
    }
    pub fn raw_u32s(&mut self, n: usize) -> Result<Vec<u32>, Error> {
        let nb = n.checked_mul(4).ok_or(Error::Truncated)?;
        let s = self.take(nb)?;
        Ok(s.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect())
    }
    pub fn f64s(&mut self) -> Result<Vec<f64>, Error> {
        let n = self.len()?;
        self.raw_f64s(n)
    }
    pub fn usizes(&mut self) -> Result<Vec<usize>, Error> {
        let n = self.len()?;
        Ok(self.raw_u32s(n)?.into_iter().map(|x| x as usize).collect())
    }
    pub fn mat(&mut self) -> Result<Mat, Error> {
        let rows = self.len()?;
        let cols = self.len()?;
        let n = (rows as u64).checked_mul(cols as u64).ok_or(Error::Truncated)?;
        if n > (self.b.len() - self.p) as u64 / 8 {
            return Err(Error::Truncated);
        }
        let data = self.raw_f64s(n as usize)?;
        Ok(Mat { rows, cols, data })
    }
    fn tag(&mut self) -> Result<bool, Error> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            t => Err(Error::Corrupt(format!("bad option tag {t}"))),
        }
    }
    pub fn opt_f64s(&mut self) -> Result<Option<Vec<f64>>, Error> {
        if self.tag()? { Ok(Some(self.f64s()?)) } else { Ok(None) }
    }
    pub fn opt_usizes(&mut self) -> Result<Option<Vec<usize>>, Error> {
        if self.tag()? { Ok(Some(self.usizes()?)) } else { Ok(None) }
    }
    pub fn opt_f64(&mut self) -> Result<Option<f64>, Error> {
        if self.tag()? { Ok(Some(self.f64()?)) } else { Ok(None) }
    }
}
