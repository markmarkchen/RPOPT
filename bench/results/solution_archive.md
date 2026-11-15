# Solution archive: the critical regions of every benchmark instance, in binary

One solve per instance, saved in the `RPOPTSOL` binary format (`src/solution_io.rs`, version 1), so that later checks (region-set
audits, lexicographic checks, point location, cross-solver comparisons) load a solution instead of re-solving it. Compute once, reuse
while the solver's region sets do not change.

## Instances and solver choice

204 instances: `problems` 12, `problems_highdim` 7, `problems_large` 131, `problems_lp` 30, `problems_lp_highdim` 24 (the `*.model.json`
files are skipped). Each is archived with the fastest known solver for it (`bench/archive/plan.csv`, written by
`bench/archive/select_solver.py`):

* mpQPs in the 111-problem table of `mplcp_goal_status_2.md`: `mplcp` where mplcp / best <= 1, else the solver named in "best other"
  (the stored times, nothing re-timed). `masses5_N4` and `masses4_N5` use `graph` (large-problems table: 3x faster than `mplcp`).
* mpLPs (`problems_lp`, `problems_lp_highdim`): `mplp_lex`.
* The 46 instances with no stored comparison (the large POP problems and `sys3x2_N9`): `mplcp`, reason "default".

| solver | instances |
|---|---:|
| mplp_lex | 54 |
| mplcp | 79 |
| parametric_daqp | 35 |
| graph | 17 |
| combinatorial_graph | 14 |
| combinatorial | 5 |

## Commits

Two archives, both complete for what they cover; the tooling is the commit that adds this file on top of `1cced96`.

| archive | main commit | covers | bytes |
|---|---|---|---:|
| `1feb98b3ced6` | 1feb98b (refined gap-band rule) | all 204 instances | 47,273,160,884 (44.0 GiB) |
| `1cced96469da` | 1cced96 (duplicate-row merge in preprocessing) | the 52 instances whose processed program changes | 3,319,681,428 (3.1 GiB) |

The duplicate-row merge (ported from the RPOPT remote) removes twin rows from the processed program, so the active-set indices of
the 52 POP instances in `bench/archive/merge_changed.txt` shift (found with `examples/merge_changes.rs`: processed constraint count
with and without the merge; all 52 are `pop_*`). On those, use the `1cced96469da` archive; for the other 152 instances the
`1feb98b3ced6` files are current (programs are identical; the neighbour pre-check merged with 1cced96 is stated to leave the sets unchanged and was not re-checked here). Sorted-set hashes are equal between the two archives on 42 of the 52; region counts are equal
on 44. The 8 that differ are all non-mplcp solvers, whose counts moved with the merge: pop_007 11,738 -> 11,680, pop_010 37 -> 36,
pop_011 408 -> 397, pop_017 4,107 -> 4,106, pop_046 33 -> 24, pop_048 57 -> 46, pop_076 19,908 -> 15,007, pop_081 6 -> 4 (towards
mplcp's counts, as the merge intends).

**Re-archiving.** `submit.sh` skips instances whose three outputs exist and writes under a per-commit directory, so a later change
costs only the instances it touches: build the new commit, put the affected names in a plan file (`PLAN=... submit.sh <commit12>`;
the 52-instance case is `plan_merge_changed.csv`).

## Where the files are

`/scratch/negishi/hsuhsing/rpopt-solutions/<commit12>/<dir>/` with, per instance:

* `<problem>.rpsol`: the binary solution (program and regions, deterministic bytes, trailing checksum);
* `<problem>.sets`: the sorted active sets, one per line (no `RPOPT_CANON`: the indexing of the stored program);
* `<problem>.json`: one line (problem, algorithm, regions, solve seconds, bytes, region hash, sorted-set hash, round-trip ok, peak
  RSS, host).

`<commit12>/manifest.csv` has one row per instance (solver, reason table/README/default, status, regions, `mplcp_table_regions`,
`region_flag`, solve seconds, bytes, hashes, round-trip ok, peak RSS MB, host, sha256 of the `.rpsol`). The manifests only (not the
binaries) are copied to `~/rpopt/rpopt-solutions/<commit12>/manifest.csv` and to this directory as
`solution_archive_manifest_<commit12>.csv`. Sizes on disk: `1feb98b3ced6` 45 GB, `1cced96469da` 3.2 GB.

Largest files: pop_036 (6,655,641 regions, mplcp) 17.8 GB, pop_095 (5,012,500 regions) 13.3 GB, masses8one_N1 (523,189 regions,
mplp_lex) 6.0 GB, pop_033 2.3 GB, pop_051 2.0 GB.

## How to load one

```rust
use rpopt::solution::Solution;

let sol = Solution::load("/scratch/negishi/hsuhsing/rpopt-solutions/1feb98b3ced6/problems_large/pop_033.rpsol")?;
println!("{} regions", sol.critical_regions.len());
let x = sol.evaluate(&theta); // point location by the sequential scan; sol.program is the processed MpProgram
```

`Solution::load` reads the whole file and decodes it (about the in-memory size of the solution, so 18 GB on pop_036). To check a file
without a second copy, `sol.file_matches(path)` compares it with a fresh chunked encoding.

## Tool

`examples/region_hash` solves once and now also writes the archive files (the same solve gives the hash, the sets and the binary):
`RPOPT_SAVE=<file.rpsol>` (streamed through a `.part` file and renamed once checked), `RPOPT_SETS_OUT=<file>`,
`RPOPT_JSON_OUT=<file>`. After writing, the file is loaded back and compared with `bit_identical` (Chebyshev radius included);
for a solution over 1 GB in memory it is instead compared chunk by chunk with a fresh encoding (`file_matches`), because a second
loaded copy would double the peak. Both checks passed on all 256 files. The writer behind it, `Solution::save_streaming`, writes the
bytes of `to_bytes` in 1 MiB chunks (a counting pass first, for the payload length in the header; `to_bytes` is unchanged);
`tests/all/solution_io.rs::streaming_save_writes_the_same_bytes_as_to_bytes` proves equal bytes on the 12 `bench/problems` solutions
and on versions of them with repeated regions (over 3 MB, several chunks), and that `file_matches` catches a flipped byte. After the
save, the regions are consumed while the hash records are built, so the records do not add to the solution's peak.

## Manifest summary (`1feb98b3ced6`)

204 of 204 archived (`status ok`, round trip true, size on disk equal to the recorded bytes); none failed or timed out after the
memory fixes below. `region_flag` compares the archived region count with `mplcp`'s count in the table of `mplcp_goal_status_2.md`
for the 104 instances that have one: ok 62, **MISMATCH 39**, drift 3.

* MISMATCH is a solver other than mplcp whose count differs from mplcp's stored count. It marks a possibly incomplete (or, with a
  higher count, over-split) partition. Counts below mplcp: parametric_daqp on masses3_N2 163 (169), masses3_N3 907 (955),
  masses3_N4 3,431 (3,643), masses4_N2 281 (327), masses4_N3 2,292 (2,947), masses4_N4 12,346 (16,312), masses5_N2 303 (373),
  masses5_N3 3,503 (5,266), masses6_N2 310 (383), masses7_N2 312 (393), masses8_N2 317 (399), rand_t12_x5_m20_s0 6,061 (6,143),
  rand_t10_x6_m18_s0 15,126 (15,312), rand_t10_x6_m24_s0 70,298 (71,219), pop_007 11,738 (12,235), pop_017 4,107 (4,142),
  pop_028 81,255 (82,116), pop_032 178,392 (178,950), pop_061 33,403 (33,587), pop_097 20,476 (20,574), pop_037 18,598 (18,615),
  pop_025 5,434 (5,441), pop_083 13,260 (13,266); graph on masses4_N5 67,279 (67,963) and masses5_N4 42,649 (44,065).
  Counts above or within a few of it: graph on masses3_N5 10,440 (10,436), pop_005 10,287 (10,289), pop_011 408 (397), pop_046 33
  (24), pop_049 9,340 (9,341); combinatorial on pop_048 57 (46) and pop_076 19,908 (15,006); combinatorial_graph on
  rand_t6_x5_m20_s1, rand_t8_x6_m18_s0, rand_t8_x6_m24_s0, rand_t10_x5_m20_s1 (within 1 to 2); parametric_daqp on pop_010 37 (36),
  pop_059 54 (53), pop_081 6 (4). The thin-region loss of `parametric_daqp` on degenerate masses problems is the large effect
  (`masses4_N4` loses 3,966 regions, `masses5_N3` 1,763). Where a complete partition matters, load the `mplcp` solution of the
  problem instead (not archived: the archive holds one solver per instance by design).
* drift is an `mplcp` count that differs from its stored count: masses3_N6 22,152 (22,143), masses3_N7 38,344 (38,326), pop_091
  145,022 (145,018); the table predates the refined gap-band rule and the cold retry, which change thin regions.
* The 100 other instances have no stored count (the 54 LPs and the defaults) and carry no flag.

In the `1cced96469da` archive: 52 of 52 ok; 18 flagged ok, 28 without a stored count, 6 MISMATCH (pop_007, pop_017, pop_028, pop_061 with
parametric_daqp; pop_049 with graph; pop_076 with combinatorial 15,007 against 15,006), no drift.

## Memory and time: measured peaks against requests

The first pass requested the mplcp peaks from the task brief. Peaks are `sacct` MaxRSS of the `.batch` step. Five tasks died with
`OUT_OF_MEMORY` and were rerun with more (no timeouts; one run of `pop_036` was cancelled by hand at 26 min with 21 GB when its growth
showed 19 000M would not do, and rerun larger).

| instance(s) | first request | result | final request | measured peak |
|---|---|---|---|---:|
| 196 small instances (1 CPU) | 1900M, 30 min | all done (longest 4 min) | 1900M | 0.91 GB (largest) |
| masses7one_N1 (mplp_lex) | 1900M | OOM at 1.83 GB after 4.8 min | 3 CPU, 5900M | 1.38 GB |
| masses8one_N1 (mplp_lex, 523,189 regions) | 1900M | OOM; 3 CPU 5900M also OOM (6.0 GB) | 8 CPU, 16000M, 19 min | 7.36 GB |
| pop_002 | 2 CPU, 3000M | ok | same | 1.33 GB (1.40 in the 1cced96 run) |
| pop_024 | 2 CPU, 3800M | ok | same | 1.60 GB |
| pop_033 | 2 CPU, 3800M | OOM at 3.74 GB | 3 CPU, 5900M | 3.72 GB (3.76 in the 1cced96 run) |
| pop_051 | 6 CPU, 11400M | ok, 21 min | same | 3.62 GB |
| pop_095 | 10 CPU, 19000M | OOM at 19.2 GB | 16 CPU, 32000M, 35 min | 21.1 GB |
| pop_036 | 10 CPU, 19000M | OOM at 19.4 GB (24.6 min); retry at 24000M cancelled | 20 CPU, 40000M, 50 min | 26.4 GB |

The brief's peaks (pop_036/095 about 18.3 GB, pop_051 10.7 GB) did not hold for this build: pop_036 and pop_095 need 26.4 and 21.1 GB
(the stored regions alone are 17.8 and 13.3 GB on disk, and the save and the hash records come on top), while pop_051 needs only
3.6 GB. A job whose CPU count was set from the memory (2010 MB per CPU) is the only reason for the multi-CPU requests; every task is
single-threaded. The two `masses` graph runs (`masses4_N5`, `masses5_N4`) stayed in the 1900M class.

Solve times (seconds, region_hash timer): pop_036 2,930, pop_095 2,049, pop_051 1,275, masses8one_N1 1,126, pop_033 297,
masses7one_N1 182.

## Scratch purge policy

From the RCAC policy page (https://www.rcac.purdue.edu/policies/scratchpurge): "Any file not accessed or having its content modified
in 60 days on most clusters (30 days on Bell) will be subject to purge." Scratch is not backed up ("There is no backup service for
scratch directories"). The archive therefore **needs a backup or a periodic touch**: a read of every file at least every 60 days keeps
it (for example a monthly `find ... -exec cat {} +`-style pass or `touch`), and copies of the manifests (small, kept in the repository
here and in `~/rpopt`) are enough to know what to recompute. The solutions themselves are 48 GB, too large for home (25 GB quota);
Data Depot or Fortress are the RCAC archival options. `purgelist` on the login node lists files already scheduled for purge. The
scratch quota is 200 TB and 2.0 M files (the archive is 770 files).

## Reproducing

```
git archive HEAD:ppopt_rs | ssh negishi.rcac.purdue.edu 'mkdir -p ~/rpopt/archive-src/<c12> && tar -x -m -C ~/rpopt/archive-src/<c12>'
sbatch --export=ALL,SRC=$HOME/rpopt/archive-src/<c12>,COMMIT=<c12> bench/archive/build.sbatch   # job rpopt_build, singleton
env -u CC bash $HOME/rpopt/archive-src/<c12>/bench/archive/submit.sh <c12>                      # arrays per memory class + merge
```

`CC=mpicc` is set in the login environment and the compute nodes lack it (mimalloc's build script fails), so the build job exports
`CC=gcc CXX=g++`. `submit.sh` holds the memory classes updated to the measured peaks above (the first-pass classes are in this file's
table).
