//! Flat hash set of the active sets that `mplcp` has seen (built or rejected).
//!
//! Every constraint index i has a random 128-bit word z[i]; the key of an active set A is the XOR of z[i] over i in A.
//! The key of a neighbour that differs from A by one or two indices is then the key of A XOR one or two words, so a
//! candidate can be looked up without building, sorting or hashing its index vector. The table is one contiguous array
//! of 32-byte entries (open addressing, linear probing): a probe touches one cache line, two entries per line.
//!
//! The table is exact. A key match is only a hint: the stored set (kept in a contiguous arena) is compared with the
//! candidate before it is accepted, and an entry with an equal key but a different set does not stop the probe.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    Built(usize),
    Rejected,
}

#[derive(Clone, Copy)]
struct Entry {
    key: u128,
    /// start of the set in the arena; EMPTY marks a free entry
    off: u32,
    alen: u32,
    slot: u32,
    _pad: u32,
}

const EMPTY: u32 = u32::MAX;
const FREE: Entry = Entry { key: 0, off: EMPTY, alen: 0, slot: 0, _pad: 0 };

pub(crate) struct SeenSet {
    zob: Vec<u128>,
    eq_key: u128,
    entries: Vec<Entry>,
    mask: usize,
    len: usize,
    arena: Vec<u32>,
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(feature = "lp_census")]
#[allow(dead_code)]
pub(crate) mod stats {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    pub static LOOKUPS: AtomicU64 = AtomicU64::new(0);
    pub static HITS: AtomicU64 = AtomicU64::new(0);
    pub static VERIFIED: AtomicU64 = AtomicU64::new(0);
    pub static MISMATCH: AtomicU64 = AtomicU64::new(0);
    pub static PROBES: AtomicU64 = AtomicU64::new(0);
    pub fn reset() {
        for c in [&LOOKUPS, &HITS, &VERIFIED, &MISMATCH, &PROBES] {
            c.store(0, Relaxed);
        }
    }
    pub fn snapshot() -> [u64; 5] {
        [LOOKUPS.load(Relaxed), HITS.load(Relaxed), VERIFIED.load(Relaxed), MISMATCH.load(Relaxed), PROBES.load(Relaxed)]
    }
}

impl SeenSet {
    /// `max_index`: the largest constraint index that can occur in an active set; `eq`: the equality indices that
    /// every active set starts with.
    pub(crate) fn new(max_index: usize, eq: &[usize]) -> SeenSet {
        let mut s = 0x5EED_0000_0000_0001u64;
        let zob: Vec<u128> = (0..=max_index).map(|_| ((splitmix(&mut s) as u128) << 64) | splitmix(&mut s) as u128).collect();
        let eq_key = eq.iter().fold(0u128, |k, &e| k ^ zob[e]);
        let cap = 1024;
        SeenSet { zob, eq_key, entries: vec![FREE; cap], mask: cap - 1, len: 0, arena: Vec::new() }
    }

    #[inline]
    pub(crate) fn z(&self, i: usize) -> u128 {
        self.zob[i]
    }

    /// XOR of the words of the equality indices (the key of the empty inequality part).
    #[inline]
    pub(crate) fn eq_key(&self) -> u128 {
        self.eq_key
    }

    #[inline]
    pub(crate) fn key_of(&self, set: &[usize]) -> u128 {
        set.iter().fold(0u128, |k, &i| k ^ self.zob[i])
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Bytes held by the table, the arena and the key words.
    pub(crate) fn bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<Entry>() + self.arena.capacity() * 4 + self.zob.capacity() * 16
    }

    /// Looks `key` up; `verify` compares the stored set (as `u32` indices, `alen` of them) with the candidate.
    #[inline]
    pub(crate) fn find(&self, key: u128, alen: usize, verify: impl Fn(&[u32]) -> bool) -> Option<usize> {
        #[cfg(feature = "lp_census")]
        stats::LOOKUPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut i = (key as u64 as usize) & self.mask;
        loop {
            let e = &self.entries[i];
            #[cfg(feature = "lp_census")]
            stats::PROBES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if e.off == EMPTY {
                return None;
            }
            if e.key == key && e.alen as usize == alen {
                #[cfg(feature = "lp_census")]
                stats::HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let stored = &self.arena[e.off as usize..e.off as usize + alen];
                if verify(stored) {
                    #[cfg(feature = "lp_census")]
                    stats::VERIFIED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Some(i);
                }
                #[cfg(feature = "lp_census")]
                stats::MISMATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            i = (i + 1) & self.mask;
        }
    }

    /// The slot stored for entry `i` of the table (an index returned by `find`): the region index, or `u32::MAX` for a
    /// claimed set that has no region.
    #[inline]
    pub(crate) fn slot_of(&self, i: usize) -> u32 {
        self.entries[i].slot
    }

    /// The slot of the active set `set` (canonical order), if it is in the table (used by the census counters).
    #[allow(dead_code)]
    pub(crate) fn get(&self, set: &[usize]) -> Option<Slot> {
        self.find(self.key_of(set), set.len(), |s| s.iter().zip(set).all(|(&a, &b)| a as usize == b))
            .map(|i| match self.entries[i].slot {
                u32::MAX => Slot::Rejected,
                j => Slot::Built(j as usize),
            })
    }

    /// Is the active set `set` (canonical order, as `Plcp::active_set` returns it) in the table?
    pub(crate) fn contains_key(&self, set: &[usize]) -> bool {
        self.find(self.key_of(set), set.len(), |s| s.iter().zip(set).all(|(&a, &b)| a as usize == b)).is_some()
    }

    fn grow(&mut self) {
        let cap = self.entries.len() * 2;
        let mut next = vec![FREE; cap];
        let mask = cap - 1;
        for e in self.entries.iter().filter(|e| e.off != EMPTY) {
            let mut i = (e.key as u64 as usize) & mask;
            while next[i].off != EMPTY {
                i = (i + 1) & mask;
            }
            next[i] = *e;
        }
        self.entries = next;
        self.mask = mask;
    }

    /// Adds `set` (or replaces the slot of the same set).
    pub(crate) fn insert(&mut self, set: &[usize], slot: Slot) {
        let slot = match slot {
            Slot::Built(i) => i as u32,
            Slot::Rejected => u32::MAX,
        };
        let key = self.key_of(set);
        if let Some(i) = self.find(key, set.len(), |s| s.iter().zip(set).all(|(&a, &b)| a as usize == b)) {
            self.entries[i].slot = slot;
            return;
        }
        if (self.len + 1) * 10 > self.entries.len() * 6 {
            self.grow();
        }
        let off = self.arena.len() as u32;
        self.arena.extend(set.iter().map(|&x| x as u32));
        let mut i = (key as u64 as usize) & self.mask;
        while self.entries[i].off != EMPTY {
            i = (i + 1) & self.mask;
        }
        self.entries[i] = Entry { key, off, alen: set.len() as u32, slot, _pad: 0 };
        self.len += 1;
    }
}
