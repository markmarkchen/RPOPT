//! Binary save / load of a built [`HashTable`] (bit-identical round trip, validated on load).
//!
//! Payload after the common header (`RPOPTHTB`, see `crate::io`): dim u32, compact u8, name u8,
//! lo[dim] f64, inv_h[dim] f64, cells_per_dim[dim] u32, six statistics u32, tab, offs, stream
//! (each u32 count + u32 items), grows (u64 count + f64 bits). Strides and the sentinel are
//! recomputed. The loader checks every table entry so that `locate` cannot index out of bounds
//! or loop on a corrupted-but-checksum-valid file.

use super::*;
use crate::io::{Error, Reader, Writer};
use std::path::Path;

const HT_MAGIC: &[u8; 8] = b"RPOPTHTB";
pub const HASH_TABLE_FORMAT_VERSION: u32 = 1;

fn name_code(name: &str) -> u8 {
    if name == "hash table (adaptive)" { 1 } else { 0 }
}

fn name_of(code: u8) -> Result<&'static str, Error> {
    match code {
        0 => Ok("hash table"),
        1 => Ok("hash table (adaptive)"),
        c => Err(Error::Corrupt(format!("unknown name code {c}"))),
    }
}

impl HashTable {
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Number of regions the table was built for.
    pub fn num_regions(&self) -> usize {
        self.offs.len().saturating_sub(1)
    }

    /// Deterministic binary encoding.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut w = Writer::new(HT_MAGIC, HASH_TABLE_FORMAT_VERSION, 4 * (self.tab.len() + self.stream.len() + self.offs.len()) + 8 * self.grows.len() + 256);
        w.len(self.dim)?;
        w.u8(self.compact as u8);
        w.u8(name_code(self.name));
        w.raw_f64s(&self.lo);
        w.raw_f64s(&self.inv_h);
        for &c in &self.cells_per_dim {
            w.len(c)?;
        }
        for v in [self.mixed, self.leaves, self.total_cands, self.total_rows, self.max_cands, self.max_depth] {
            w.len(v)?;
        }
        for v in [&self.tab, &self.offs, &self.stream] {
            w.len(v.len())?;
            w.raw_u32s(v);
        }
        w.u64(self.grows.len() as u64);
        w.raw_f64s(&self.grows);
        Ok(w.finish())
    }

    /// Decodes [`HashTable::to_bytes`]; errors instead of panicking on any malformed input.
    pub fn from_bytes(bytes: &[u8]) -> Result<HashTable, Error> {
        let mut rd = Reader::open(bytes, HT_MAGIC, HASH_TABLE_FORMAT_VERSION)?;
        let dim = rd.len()?;
        if !(1..=8).contains(&dim) {
            return Err(Error::Corrupt(format!("hash table dimension {dim} outside 1..=8")));
        }
        let compact = match rd.u8()? {
            0 => false,
            1 => true,
            t => return Err(Error::Corrupt(format!("bad compact flag {t}"))),
        };
        let name = name_of(rd.u8()?)?;
        let lo = rd.raw_f64s(dim)?;
        let inv_h = rd.raw_f64s(dim)?;
        let cells_per_dim: Vec<usize> = rd.raw_u32s(dim)?.into_iter().map(|c| c as usize).collect();
        let mut st = [0usize; 6];
        for s in st.iter_mut() {
            *s = rd.len()?;
        }
        let tab_n = rd.len()?;
        let tab = rd.raw_u32s(tab_n)?;
        let offs_n = rd.len()?;
        let offs = rd.raw_u32s(offs_n)?;
        let stream_n = rd.len()?;
        let stream = rd.raw_u32s(stream_n)?;
        let grows_n = usize::try_from(rd.u64()?).map_err(|_| Error::Truncated)?;
        let grows = rd.raw_f64s(grows_n)?;
        rd.finish()?;

        let mut strides = vec![1usize; dim];
        let mut total = 1usize;
        for k in 0..dim {
            if cells_per_dim[k] == 0 {
                return Err(Error::Corrupt("zero cells in a dimension".into()));
            }
            if k > 0 {
                strides[k] = strides[k - 1] * cells_per_dim[k - 1];
            }
            total = total.checked_mul(cells_per_dim[k]).ok_or_else(|| Error::Corrupt("grid size overflow".into()))?;
        }
        let ht = HashTable {
            dim,
            lo,
            inv_h,
            cells_per_dim,
            strides,
            tab,
            sentinel: total,
            grows,
            compact,
            offs,
            stream,
            name,
            mixed: st[0],
            leaves: st[1],
            total_cands: st[2],
            total_rows: st[3],
            max_cands: st[4],
            max_depth: st[5],
        };
        ht.validate()?;
        Ok(ht)
    }

    /// Structural validation: every entry of every table block resolves inside the arrays and
    /// child blocks only point forward (so a query terminates).
    fn validate(&self) -> Result<(), Error> {
        let bad = |s: &str| Err(Error::Corrupt(format!("hash table: {s}")));
        let n = self.dim;
        let w = n + 1;
        let nreg = self.num_regions();
        if self.offs.is_empty() || self.offs[0] != 0 || self.offs.windows(2).any(|p| p[0] > p[1]) {
            return bad("row offsets are not monotone");
        }
        let nrows = *self.offs.last().unwrap() as usize;
        if self.grows.len() != nrows * w {
            return bad("row table size does not match the offsets");
        }
        if self.tab.len() < self.sentinel + 1 {
            return bad("table smaller than the base grid");
        }
        let nchild = 1usize << n;
        for (pos, &e) in self.tab.iter().enumerate() {
            match e >> TAG_SHIFT {
                0 => {
                    if e as usize > nreg {
                        return bad("pure entry names a missing region");
                    }
                }
                TAG_NODE => {
                    let base = (e & PAYLOAD) as usize;
                    if base <= pos || base + nchild > self.tab.len() {
                        return bad("child block out of range");
                    }
                }
                TAG_LEAF => self.validate_record((e & PAYLOAD) as usize, nreg, nrows)?,
                _ => return bad("unknown entry tag"),
            }
        }
        if self.tab[self.sentinel] != 0 {
            return bad("sentinel entry must be empty");
        }
        Ok(())
    }

    fn validate_record(&self, p: usize, nreg: usize, nrows: usize) -> Result<(), Error> {
        let bad = |s: &str| Err(Error::Corrupt(format!("hash table: {s}")));
        let Some(&nc) = self.stream.get(p) else { return bad("leaf record out of range") };
        let nc = nc as usize;
        let mut q = p + 1;
        if self.compact {
            let Some(c) = self.stream.get(q..q.saturating_add(nc)) else { return bad("leaf record out of range") };
            if c.iter().any(|&j| j as usize >= nreg) {
                return bad("candidate region out of range");
            }
            return Ok(());
        }
        for _ in 0..nc {
            let (Some(&j), Some(&nr)) = (self.stream.get(q), self.stream.get(q + 1)) else { return bad("leaf record out of range") };
            if j as usize >= nreg {
                return bad("candidate region out of range");
            }
            let Some(rows) = self.stream.get(q + 2..(q + 2).saturating_add(nr as usize)) else { return bad("leaf record out of range") };
            if rows.iter().any(|&id| id as usize >= nrows) {
                return bad("row id out of range");
            }
            q += 2 + nr as usize;
        }
        Ok(())
    }

    /// Errors unless the table was built for a partition with this parameter dimension and region count.
    pub fn check_compatible(&self, dim: usize, num_regions: usize) -> Result<(), Error> {
        if self.dim != dim || self.num_regions() != num_regions {
            return Err(Error::DimensionMismatch(format!("hash table is for {} parameters and {} regions, expected {dim} and {num_regions}", self.dim, self.num_regions())));
        }
        Ok(())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        std::fs::write(path, self.to_bytes()?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<HashTable, Error> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// Bitwise equality of all stored data (f64 compared by bits).
    pub fn bit_identical(&self, o: &HashTable) -> bool {
        let fb = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
        self.dim == o.dim
            && fb(&self.lo, &o.lo)
            && fb(&self.inv_h, &o.inv_h)
            && self.cells_per_dim == o.cells_per_dim
            && self.strides == o.strides
            && self.tab == o.tab
            && self.sentinel == o.sentinel
            && fb(&self.grows, &o.grows)
            && self.compact == o.compact
            && self.offs == o.offs
            && self.stream == o.stream
            && self.name == o.name
            && (self.mixed, self.leaves, self.total_cands, self.total_rows, self.max_cands, self.max_depth) == (o.mixed, o.leaves, o.total_cands, o.total_rows, o.max_cands, o.max_depth)
    }
}
