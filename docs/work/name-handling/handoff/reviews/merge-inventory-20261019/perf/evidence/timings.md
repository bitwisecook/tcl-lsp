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
