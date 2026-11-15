# mplcp: allocation-free neighbour pre-check and a flat Seen table

Base main 59b2afb (the scale-free multiplier rule is in it). Branch `worktree-agent-a3946c55839ffe05f`. The change keeps
every decision of the neighbour search; it only makes the "is this neighbour already known" test cheap.

## What changed

1. **Zobrist keys.** Every constraint index i has a random 128-bit word z[i]; the key of an active set is the XOR of
   z[i] over its members. A fast-path neighbour has the key of A XOR one word; a rank-1 member A + {c} - {j} has the
   key of A XOR z[c] XOR z[j]. No vector is built, sorted or hashed to ask whether a neighbour is known.
2. **A flat `Seen` table** (`src/algorithms/seen_set.rs`): open addressing with linear probing over one array of
   32-byte entries {key, arena offset, length, slot}, at most 60% full, plus one arena of the stored sets (u32). It
   replaces `FxHashMap<Vec<usize>, Slot>`.
3. **Exact.** A key match is only a hint. The stored set is compared with the candidate before it is accepted (for
   an incremental key: the stored set must lie inside A + {c} without j and have the length of A), and an entry with an
   equal key but a different set does not end the probe. In the measured solves there were 0 mismatches on 4.2 M hits.
4. **Fast path.** A Case-1 neighbour that is already known is no longer built and returned (the caller dropped it
   anyway): `adjacent` returns an empty list. The decisions are the same; only the work for it is gone.
5. **Pooled scratch.** `adjacent` takes its five working vectors from a thread-local pool, and `rank1_case2` takes the
   multiplier set, the multipliers, the bit set of A, the null basis and the lambdas from another. The tie test between
   two members runs without allocating, with the same arithmetic as before (it is skipped when there is a single
   member, and the nullspace and lambdas are rebuilt in the old form only when a member has to be tested).

## Gate

- **Region hashes** (sorted active sets and facet rows rounded to 1e-9, `examples/region_hash.rs`): identical to
  59b2afb on 51 of 51 problems (the 49 of the rank-1 study plus `masses5_N3`, `masses4_N4`, `masses4_N3`,
  `masses3_N4`); also identical to ddf0aa8 on the same 51 before the merge of the multiplier rule.
- **Tests:** 144 passed, 0 failed, 3 ignored (the CI gate, 147 tests, gate profile, 1 CPU each).
- **Paired timing**, Negishi, 1-CPU array tasks, one task per (problem, alternation), 5 alternations, the binaries
  old, new and old again (the A/A pair) once each in a shuffled order, 3 solves per binary, median per task, then the
  median ratio over the 5 tasks. Ratio below 1: the new build is faster.

| problem | old ms | new / old | A/A |
|---|---:|---:|---:|
| rand_t8_x5_m20_s0 | 665 | 0.918 | 1.004 |
| rand_t10_x5_m20_s0 | 1673 | 0.930 | 1.000 |
| pop_017 | 524 | 0.924 | 1.000 |
| rand_t12_x5_m20_s0 | 3080 | 0.939 | 1.001 |
| masses4_N3 | 2030 | 0.971 | 0.998 |
| masses5_N3 | 8887 | 0.995 | 1.002 |
| masses4_N4 | 21133 | 0.985 | 1.001 |
| masses8_N2 | 1021 | 0.977 | 1.006 |
| pop_005 | 2318 | 0.971 | 1.000 |
| rand_t6_x5_m20_s0 | 172 | 0.909 | 0.998 |
| pop_021 | 101 | 0.939 | 1.000 |
| sys3x2_N5 | 24 | 0.905 | 1.004 |

The 33 problems of 20 ms or more: geometric mean 0.947 (range 0.905 to 0.995), the A/A pair 1.000. Every problem is
faster; the gain is 5% to 9% on the problems that lose to `parametric_daqp` by 1.2x to 1.3x with equal region builds
and 1% to 3% on the masses problems whose time is in the region build and the Case-2 tests.
`pop_002` (321,026 regions, the large problem for the cache effect), measured against ddf0aa8 before the merge
of the multiplier rule, 3 alternations of one solve each: new / old 0.958, 0.957, 0.973 (A/A 0.988, 1.012, 1.004).

## Lookups and memory (lp_census build, one solve each, new code)

| problem | lookups | hits | probes per lookup | whole-key lookup ns | table + arena + words, KB | old map, KB (estimate) |
|---|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 82,944 | 62,006 | 1.68 | 42 to 50 | 321 | 438 |
| pop_017 | 89,015 | 68,168 | 1.72 | 43 | 320 | 511 |
| rand_t8_x5_m20_s0 | 82,666 | 69,885 | 2.17 | 62 to 74 | 384 | 524 |
| rand_t10_x5_m20_s0 | 150,603 | 131,856 | 1.48 | 63 to 96 | 672 | 626 |
| rand_t12_x5_m20_s0 | 169,764 | 146,512 | 1.60 | 66 | 640 | 955 |
| pop_002 | 5,278,856 | 4,170,330 | 1.78 | 164 to 186 | 40,961 | 41,951 |

The whole-key lookup here includes XORing the stored set's words and the verification; the incremental keys of the
fast path and the rank-1 members skip the XOR over the set. The old map size is an estimate from the hashbrown layout
(40 bytes per bucket plus 8 bytes per index and an allocation header per key). For `pop_036` (6.66 M regions of about
30 indices) the table is 537 MB (16.7 M entries) and the arena 0.8 GB, against about 2 GB for the old map.

## Locality of the hits (why there is no local cache)

For every verified hit: the number of regions created since the found region (its age) and its relation to the region
being expanded (the found region's `Slot` is its index; hits on a claimed but not yet built set have no index).

| problem | age <= 8 | <= 64 | <= 512 | older | parent | sibling | sibling's child | other |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| masses4_N3 | 0.7% | 3.7% | 25.9% | 74% | 4.2% | 17.1% | 2.6% | 76.1% |
| pop_017 | 2.3% | 6.7% | 20.0% | 80% | 1.9% | 8.5% | 4.0% | 85.6% |
| rand_t8_x5_m20_s0 | 4.2% | 9.5% | 24.5% | 75% | 1.3% | 5.1% | 4.7% | 88.9% |
| rand_t10_x5_m20_s0 | 3.5% | 8.4% | 19.2% | 81% | 4.2% | 6.7% | 3.4% | 85.8% |
| rand_t12_x5_m20_s0 | 3.2% | 7.5% | 18.0% | 82% | 0.9% | 5.6% | 3.7% | 89.8% |
| pop_002 | 7.1% | 14.1% | 20.6% | 79% | 0.8% | 1.0% | 6.5% | 91.7% |

(Shares of the hits that carry a region index; 5% to 21% of all hits are on claimed sets without an index.) A breadth-first
search creates the regions around one parent a whole frontier away from the regions that rediscover them: at most 7%
of the hits fall in the last 8 regions and at most 26% in the last 512, and the parent, the siblings and the siblings'
children together are 7% to 24% of the hits. A per-item cache of parent and sibling keys would answer at most that
share, ahead of a lookup that costs 40 to 100 ns; it was not built. Region storage is already in creation order
(the arena of the table and the `Vec` of regions).
