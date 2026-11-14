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

pub(crate) struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn new(magic: &[u8; 8], version: u32, capacity: usize) -> Self {
        let mut buf = Vec::with_capacity(capacity + HEADER_LEN + 8);
        buf.extend_from_slice(magic);
        buf.extend_from_slice(&version.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes());
        Writer { buf }
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
