# perf timings (opt-level 1, debug assertions on, line-tables; 4-core shared box)

Probe: `tools/perfprobe_example.rs.txt` (example in tcl-compiler), built in an
isolated /dev/shm copy with `batch/apply_batch.py ROOT --zz`. With `--zz`,
`tcl_lexer::ZZ_OLD` / `tcl_compiler::realm::ZZ_NO_MEMO` switch the whole batch
off in-process, so "old" and "new" run in the same binary.

## Analyser `proc p {a} {return $a}` + `p 1` x n (Analyser::new().analyse(.., "tcl8.6"))

| n | work before | work after |
|---|---|---|
| 10 | 0.66 s | 0.54-0.66 s |
| 20 | 1.21 s | 0.71 s |
| 40 | 3.5 s | 1.37 s |
| 80 | 14.8 s | 2.9 s |
| 160 | - | ~7.0 s |

Phase trace before the fix (n=10/20/40): walk 0.24/0.57/1.96 s, diagnostics
0.30/0.55/1.41 s. Both phases were super-linear.

## `proc inc {name} {upvar 1 $name v; incr v}` + n callers (upvar shape)

| n | old | new |
|---|---|---|
| 10 | 3.8 s | 2.5 s |
| 30 | 39.6 s | 11.8 s |

## tk8.6 tclIndex (`set auto_index(X) [list source [file join $dir f.tcl]]`)

| lines | old | new (batch incl. incomplete_body header memo) |
|---|---|---|
| 10 | 1.1-1.5 s | 0.7 s |
| 40 | 10.7-13.5 s | 2.4 s |
| 100 | 90.7 s | 7.9 s |
| 253 (whole file) | ~208 s (first memos only; full old >> 10 min) | 29.3 s |

The new curve is still mildly super-linear. The residual is spread over
per-command whole-document work: SourceImage/SourceMap/LineIndex rebuilt per
command, and re-segmentation per query (`evidence/idx253.stacks`).

## tcl_spectcl::bundled::load_embedded (8 packs, ~2 MB)

| build | time |
|---|---|
| old | 7.3-10.4 s |
| new, with full-chain debug_assert in in_body | 6.9 s |
| new, with inductive parent-link debug_assert | 7.0 s |
| new, root assert in in_document only (landed shape) | 1.9-3.1 s |

Equality: the sorted `{:#?}` lines of the PackSet are identical old vs new
(484,936 lines), in-process.

## Isolated cloud session, 2026-10-22

Rust 1.99.1, opt-level 1, debug assertions and line tables, incremental off,
`-j2`; separate target directories for each tree. Builds ran concurrently,
so these wall-clock diagnostics are not release latency or a clean exponent
measurement. The baseline is the verified origin/rust `fcb7927f`; work proof
is based on `8a01a718` plus the refreshed full batch. Work results are pending.

Tk 8.6.18 `library/tclIndex`: 253 lines, SHA-256
`e7ea83cd5df322fb760c9787c9eda92bad3d048737be28d2c9606b9243cf0bbb`.
Exactly the same first 10/40/100/253 lines are used in both trees.

| Input | origin/rust | work old | work full batch |
|---|---:|---:|---:|
| calls 10 (median of 3 warmed runs) | 56 ms | pending | pending |
| calls 20 (median of 3 warmed runs) | 76 ms | pending | pending |
| calls 40 (median of 3 warmed runs) | 129 ms | pending | pending |
| calls 80 (median of 3 warmed runs) | 109 ms | pending | pending |
| tclIndex 10 lines | 163 ms | pending | pending |
| tclIndex 40 lines | 492 ms | pending | pending |
| tclIndex 100 lines | 1021 ms | pending | pending |
| tclIndex 253 lines | 1596 ms | pending | pending |
| load_embedded | 4665 ms | pending | pending |

Raw timing-only runs: `origin-calls.out`, `origin-upvar.out`, `origin-procs.out`,
`origin-tclIndex.out`, `origin-corpus.out` (102), `origin-irules.out` (15),
`origin-spectcl.out`, `origin-lsp-document.out`, `origin-lsp-chain.out`.
The baseline probe's `eq=true` field is a placeholder in timing-only mode,
not an equality proof between baseline and migrated results.
The ordinary LSP probe uses actual analysed declarations and asserts that
index intake and code lenses retain every measured procedure. Source-chain
construction uses the public RunOrder compatibility ingress, separately from
actual source/provider admission. No unavailable provider is counted as a
fast successful document query.
