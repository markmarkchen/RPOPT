//! Save / load of explicit solutions: compact deterministic binary and JSON.

use crate::critical_region::CriticalRegion;
use crate::io::{Error, Reader, Sink, StreamHasher, Writer};
use crate::linalg::Mat;
use crate::program::MpProgram;
use crate::solution::{Solution, SolutionData};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// On-disk format of [`Solution::save`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Compact binary (see [`Solution::to_bytes`]).
    Binary,
    /// serde_json text.
    Json,
}

const SOL_MAGIC: &[u8; 8] = b"RPOPTSOL";
/// Binary format version. Payload: flags u32 (bit 0 = overlapping), tolerance f64, the processed
/// program (A, b, c, H, Q?, A_t, b_t, F, c_c, c_t, Q_t, equality_indices), then the regions
/// (A, b, C, d, E, f, active/omega/lambda sets, regular set, y data, Chebyshev radius?).
/// Matrices are rows u32, cols u32, row-major f64 bits; vectors are length u32 plus items.
pub const SOLUTION_FORMAT_VERSION: u32 = 1;

#[derive(Deserialize)]
struct SolutionJson {
    program: MpProgram,
    solution: SolutionData,
}

#[derive(Serialize)]
struct SolutionJsonRef<'a> {
    program: &'a MpProgram,
    solution: SolutionData,
}

fn check_mat(name: &str, m: &Mat) -> Result<(), Error> {
    if m.rows.checked_mul(m.cols) != Some(m.data.len()) {
        return Err(Error::Corrupt(format!("{name}: data length does not match shape")));
    }
    Ok(())
}

fn mismatch<T>(msg: String) -> Result<T, Error> {
    Err(Error::DimensionMismatch(msg))
}

/// Shape consistency of a program and its regions (shared by every loader).
fn validate(p: &MpProgram, regions: &[CriticalRegion]) -> Result<(), Error> {
    for (n, m) in [("A", &p.a), ("H", &p.h), ("A_t", &p.a_t), ("F", &p.f), ("Q_t", &p.q_t)] {
        check_mat(n, m)?;
    }
    if let Some(q) = &p.q {
        check_mat("Q", q)?;
    }
    let (nx, nt) = (p.a.cols, p.f.cols);
    if p.b.len() != p.a.rows || p.f.rows != p.a.rows || p.c.len() != nx {
        return mismatch(format!("program: A is {}x{}, b has {}, c has {}, F has {} rows", p.a.rows, p.a.cols, p.b.len(), p.c.len(), p.f.rows));
    }
    if p.h.rows != nx || p.h.cols != nt || p.c_t.len() != nt || p.q_t.rows != nt || p.q_t.cols != nt {
        return mismatch(format!("program: H is {}x{} (expected {nx}x{nt}), c_t has {}", p.h.rows, p.h.cols, p.c_t.len()));
    }
    if p.a_t.rows != p.b_t.len() || (p.a_t.rows > 0 && p.a_t.cols != nt) {
        return mismatch("program: A_t / b_t do not match the parameter dimension".into());
    }
    if let Some(q) = &p.q {
        if q.rows != nx || q.cols != nx {
            return mismatch("program: Q is not n_x by n_x".into());
        }
    }
    if p.equality_indices.iter().any(|&i| i >= p.a.rows) {
        return mismatch("program: equality index out of range".into());
    }
    for (k, r) in regions.iter().enumerate() {
        for (n, m) in [("A", &r.a), ("C", &r.c), ("E", &r.e)] {
            check_mat(n, m)?;
        }
        let bad = r.a.rows != r.b.len()
            || r.c.rows != r.d.len()
            || r.e.rows != r.f.len()
            || (r.a.rows > 0 && r.a.cols != nt)
            || (r.c.rows > 0 && r.c.cols != nt)
            || (r.e.rows > 0 && r.e.cols != nt);
        if bad {
            return mismatch(format!("region {k} does not match the parameter dimension {nt}"));
        }
        let y_ok = match (&r.y_fixation, &r.y_indices, &r.x_indices) {
            (None, None, None) => true,
            (Some(y), Some(yi), Some(xi)) => {
                let n = xi.len() + yi.len();
                y.len() == yi.len() && r.a.rows == xi.len() && xi.iter().chain(yi).all(|&i| i < n)
            }
            _ => false,
        };
        if !y_ok {
            return mismatch(format!("region {k}: inconsistent mixed-integer data"));
        }
    }
    Ok(())
}

fn write_region(w: &mut Writer, r: &CriticalRegion) -> Result<(), Error> {
    w.mat(&r.a)?;
    w.f64s(&r.b)?;
    w.mat(&r.c)?;
    w.f64s(&r.d)?;
    w.mat(&r.e)?;
    w.f64s(&r.f)?;
    w.usizes(&r.active_set)?;
    w.usizes(&r.omega_set)?;
    w.usizes(&r.lambda_set)?;
    w.usizes(&r.regular_set.0)?;
    w.usizes(&r.regular_set.1)?;
    w.opt_f64s(&r.y_fixation)?;
    w.opt_usizes(&r.y_indices)?;
    w.opt_usizes(&r.x_indices)?;
    w.opt_f64(r.cheb_radius);
    Ok(())
}

fn read_region(rd: &mut Reader) -> Result<CriticalRegion, Error> {
    Ok(CriticalRegion {
        a: rd.mat()?,
        b: rd.f64s()?,
        c: rd.mat()?,
        d: rd.f64s()?,
        e: rd.mat()?,
        f: rd.f64s()?,
        active_set: rd.usizes()?,
        omega_set: rd.usizes()?,
        lambda_set: rd.usizes()?,
        regular_set: (rd.usizes()?, rd.usizes()?),
        y_fixation: rd.opt_f64s()?,
        facet_single: vec![],
        y_indices: rd.opt_usizes()?,
        x_indices: rd.opt_usizes()?,
        cheb_radius: rd.opt_f64()?,
    })
}

impl Solution {
    /// The processed program, absent for mixed-integer solutions (which cannot be saved).
    fn saved_program(&self) -> Result<&MpProgram, Error> {
        self.continuous.as_deref().ok_or_else(|| Error::Unsupported("solutions without a continuous MpProgram (mixed-integer) cannot be saved".into()))
    }

    /// Estimated in-memory size in bytes of the program and regions (heap payload of the
    /// matrices and index lists plus struct headers, excluding allocator overhead).
    pub fn memory_bytes(&self) -> usize {
        let mut b = 0usize;
        if let Some(p) = &self.continuous {
            let q = p.q.as_ref().map_or(0, |m| m.data.len());
            b += 8
                * (p.a.data.len() + p.b.len() + p.c.len() + p.h.data.len() + q + p.a_t.data.len() + p.b_t.len() + p.f.data.len() + p.c_t.len() + p.q_t.data.len() + p.equality_indices.len());
        }
        for r in &self.critical_regions {
            let floats = r.a.data.len() + r.b.len() + r.c.data.len() + r.d.len() + r.e.data.len() + r.f.len() + r.y_fixation.as_ref().map_or(0, |v| v.len());
            let idx = r.active_set.len()
                + r.omega_set.len()
                + r.lambda_set.len()
                + r.regular_set.0.len()
                + r.regular_set.1.len()
                + r.y_indices.as_ref().map_or(0, |v| v.len())
                + r.x_indices.as_ref().map_or(0, |v| v.len());
            b += 8 * floats + 8 * idx + std::mem::size_of::<CriticalRegion>();
        }
        b
    }

    /// Deterministic compact binary encoding (see [`SOLUTION_FORMAT_VERSION`]). Self-contained:
    /// includes the processed program.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut w = Writer::new(SOL_MAGIC, SOLUTION_FORMAT_VERSION, self.memory_bytes() / 2 + 64);
        self.encode(&mut w)?;
        Ok(w.finish())
    }

    /// Writes exactly the bytes of [`Solution::to_bytes`] to `path` in chunks, so the peak extra memory
    /// is about 2 MiB instead of the whole encoding. Returns the file size.
    pub fn save_streaming(&self, path: impl AsRef<Path>) -> Result<u64, Error> {
        let mut count = Writer::streaming(SOL_MAGIC, SOLUTION_FORMAT_VERSION, 0, Sink::Count(0));
        self.encode(&mut count)?;
        let plen = count.finish_stream()?;
        let file = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(path)?);
        let mut w = Writer::streaming(SOL_MAGIC, SOLUTION_FORMAT_VERSION, plen, Sink::File(file, StreamHasher::new()));
        self.encode(&mut w)?;
        w.finish_stream()
    }

    /// Checks that the file at `path` holds exactly the encoding of this solution, by encoding in chunks and
    /// comparing them with the file (no second copy of the solution in memory, unlike [`Solution::load`]).
    pub fn file_matches(&self, path: impl AsRef<Path>) -> Result<bool, Error> {
        let mut head = [0u8; 20];
        let mut file = std::fs::File::open(path.as_ref())?;
        std::io::Read::read_exact(&mut file, &mut head)?;
        let plen = u64::from_le_bytes(head[12..20].try_into().unwrap());
        let rd = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path.as_ref())?);
        let mut w = Writer::streaming(SOL_MAGIC, SOLUTION_FORMAT_VERSION, plen, Sink::Compare(rd, StreamHasher::new()));
        match self.encode(&mut w) {
            Err(Error::Corrupt(m)) if m.starts_with("saved file") => return Ok(false),
            r => r?,
        }
        match w.finish_stream() {
            Ok(_) => Ok(true),
            Err(Error::Corrupt(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn encode(&self, w: &mut Writer) -> Result<(), Error> {
        let p = self.saved_program()?;
        validate(p, &self.critical_regions)?;
        w.u32(self.is_overlapping as u32);
        w.f64(self.point_location_tolerance);
        w.mat(&p.a)?;
        w.f64s(&p.b)?;
        w.f64s(&p.c)?;
        w.mat(&p.h)?;
        match &p.q {
            None => w.u8(0),
            Some(q) => {
                w.u8(1);
                w.mat(q)?;
            }
        }
        w.mat(&p.a_t)?;
        w.f64s(&p.b_t)?;
        w.mat(&p.f)?;
        w.f64(p.c_c);
        w.f64s(&p.c_t)?;
        w.mat(&p.q_t)?;
        w.usizes(&p.equality_indices)?;
        w.len(self.critical_regions.len())?;
        for r in &self.critical_regions {
            write_region(w, r)?;
            w.spill()?;
        }
        Ok(())
    }

    /// Decodes [`Solution::to_bytes`]. Never panics: bad magic, version, truncation, corruption
    /// and shape inconsistencies are errors.
    pub fn from_bytes(bytes: &[u8]) -> Result<Solution, Error> {
        let mut rd = Reader::open(bytes, SOL_MAGIC, SOLUTION_FORMAT_VERSION)?;
        let flags = rd.u32()?;
        if flags > 1 {
            return Err(Error::Corrupt(format!("unknown flags {flags:#x}")));
        }
        let tol = rd.f64()?;
        let a = rd.mat()?;
        let b = rd.f64s()?;
        let c = rd.f64s()?;
        let h = rd.mat()?;
        let q = match rd.u8()? {
            0 => None,
            1 => Some(rd.mat()?),
            t => return Err(Error::Corrupt(format!("bad option tag {t}"))),
        };
        let a_t = rd.mat()?;
        let b_t = rd.f64s()?;
        let f = rd.mat()?;
        let c_c = rd.f64()?;
        let c_t = rd.f64s()?;
        let q_t = rd.mat()?;
        let eq = rd.usizes()?;
        let n = rd.len()?;
        // the count is bounded by the payload (a read past the end is an error), so cap the
        // initial reservation instead of trusting it
        let mut regions = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n {
            regions.push(read_region(&mut rd)?);
        }
        rd.finish()?;
        let prog = MpProgram::from_raw_parts(a, b, c, h, q, a_t, b_t, f, c_c, c_t, q_t, eq);
        validate(&prog, &regions)?;
        Ok(Self::assemble(prog, regions, flags == 1, tol))
    }

    /// True when the program and every region field are equal bit for bit (f64 by bits). With
    /// `with_cheb`, the cached Chebyshev radius is compared too (JSON does not store it).
    pub fn bit_identical(&self, o: &Solution, with_cheb: bool) -> bool {
        fn mb(a: &Mat, b: &Mat) -> bool {
            a.rows == b.rows && a.cols == b.cols && vb(&a.data, &b.data)
        }
        fn vb(a: &[f64], b: &[f64]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
        }
        fn ob(a: &Option<Vec<f64>>, b: &Option<Vec<f64>>) -> bool {
            match (a, b) {
                (None, None) => true,
                (Some(x), Some(y)) => vb(x, y),
                _ => false,
            }
        }
        let (Some(p), Some(q)) = (&self.continuous, &o.continuous) else { return false };
        let prog = mb(&p.a, &q.a)
            && vb(&p.b, &q.b)
            && vb(&p.c, &q.c)
            && mb(&p.h, &q.h)
            && match (&p.q, &q.q) {
                (None, None) => true,
                (Some(x), Some(y)) => mb(x, y),
                _ => false,
            }
            && mb(&p.a_t, &q.a_t)
            && vb(&p.b_t, &q.b_t)
            && mb(&p.f, &q.f)
            && p.c_c.to_bits() == q.c_c.to_bits()
            && vb(&p.c_t, &q.c_t)
            && mb(&p.q_t, &q.q_t)
            && p.equality_indices == q.equality_indices;
        prog && self.is_overlapping == o.is_overlapping
            && self.point_location_tolerance.to_bits() == o.point_location_tolerance.to_bits()
            && self.critical_regions.len() == o.critical_regions.len()
            && self.critical_regions.iter().zip(&o.critical_regions).all(|(r, s)| {
                mb(&r.a, &s.a)
                    && vb(&r.b, &s.b)
                    && mb(&r.c, &s.c)
                    && vb(&r.d, &s.d)
                    && mb(&r.e, &s.e)
                    && vb(&r.f, &s.f)
                    && r.active_set == s.active_set
                    && r.omega_set == s.omega_set
                    && r.lambda_set == s.lambda_set
                    && r.regular_set == s.regular_set
                    && ob(&r.y_fixation, &s.y_fixation)
                    && r.y_indices == s.y_indices
                    && r.x_indices == s.x_indices
                    && (!with_cheb || r.cheb_radius.map(f64::to_bits) == s.cheb_radius.map(f64::to_bits))
            })
    }

    fn assemble(prog: MpProgram, regions: Vec<CriticalRegion>, overlapping: bool, tol: f64) -> Solution {
        let prog = Arc::new(prog);
        Solution { program: prog.clone(), continuous: Some(prog), critical_regions: regions, is_overlapping: overlapping, point_location_tolerance: tol }
    }

    /// Writes the solution (with its program) to `path`.
    pub fn save(&self, path: impl AsRef<Path>, format: Format) -> Result<(), Error> {
        match format {
            Format::Binary => {
                self.save_streaming(path)?;
            }
            Format::Json => {
                let p = self.saved_program()?;
                validate(p, &self.critical_regions)?;
                let file = std::fs::File::create(path)?;
                let doc = SolutionJsonRef { program: p, solution: self.to_data() };
                serde_json::to_writer(std::io::BufWriter::new(file), &doc).map_err(|e| Error::Json(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Loads a file written by [`Solution::save`]; the format is detected from the first bytes.
    pub fn load(path: impl AsRef<Path>) -> Result<Solution, Error> {
        let bytes = std::fs::read(path)?;
        Self::from_any_bytes(&bytes)
    }

    /// Like [`Solution::from_bytes`] but also accepts the JSON format (text starting with `{`).
    pub fn from_any_bytes(bytes: &[u8]) -> Result<Solution, Error> {
        if bytes.first() != Some(&b'{') {
            return Self::from_bytes(bytes);
        }
        let doc: SolutionJson = serde_json::from_slice(bytes).map_err(|e| Error::Json(e.to_string()))?;
        validate(&doc.program, &doc.solution.critical_regions)?;
        Ok(Self::assemble(doc.program, doc.solution.critical_regions, doc.solution.is_overlapping, doc.solution.point_location_tolerance))
    }
}
