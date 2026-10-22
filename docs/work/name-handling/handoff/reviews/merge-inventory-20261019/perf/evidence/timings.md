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
is based on `8a01a718` plus the refreshed full batch. Work corpus/synthetic/PackSet results remain in progress; the completed ordinary LSP rows below are pinned to this base, before rebasing.

Tk 8.6.18 `library/tclIndex`: 253 lines, SHA-256
`e7ea83cd5df322fb760c9787c9eda92bad3d048737be28d2c9606b9243cf0bbb`.
Exactly the same first 10/40/100/253 lines are used in both trees.

| Input | origin/rust | work old | work full batch |
|---|---:|---:|---:|
| calls 10 (median of 3 warmed runs) | 56 ms | pending | pending |
| calls 20 (median of 3 warmed runs) | 76 ms | pending | pending |
| calls 40 (median of 3 warmed runs) | 129 ms | pending | pending |
| calls 80 (median of 3 warmed runs) | 109 ms | pending | pending |
| tclIndex 10 lines | 163 ms | 1084 ms | 920 ms |
| tclIndex 40 lines | 492 ms | 34482 ms | 16086 ms |
| tclIndex 100 lines | 1021 ms | 459444 ms (instrumented) | 37237 ms |
| tclIndex 253 lines | 1596 ms | pending | pending |
| load_embedded | 4665 ms | old1 62828 / old2 120495 ms | 25493 ms |

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

### Ordinary analysed-document path, base 8a01a718 + full batch

All declaration/index/lens count assertions passed. Work rows use the actual
Logical publication path; they do not count an unavailable provider as fast.
The batch leaves these costs unresolved. Values below are milliseconds,
medians of three queries on each retained analysis; analysis itself runs once.
The host was contended, so use the results as diagnostic scaling evidence.

| Procedures/calls | origin index | work index | origin 100 misses | work 100 misses | origin lenses | work lenses |
|---:|---:|---:|---:|---:|---:|---:|
| 10 | 0.082 | 58.271 | 0.099 | 0.476 | 0.302 | 67.261 |
| 20 | 0.043 | 269.560 | 0.356 | 2.385 | 1.857 | 789.784 |
| 40 | 0.092 | 514.987 | 0.691 | 2.803 | 6.323 | 5042.101 |
| 80 | 0.547 | 931.050 | 2.417 | 3.920 | 24.129 | 11160.625 |

The current lens implementation reconstructs the Logical call inventory and
WorkspaceIndex multiple times per declaration, including inside currentness
checks. See residual-routing.md for the request-scoped owner handoff; none of
these checks may be bypassed to obtain a faster result.

The separately measured public RunOrder compatibility source chain still
copies full ancestry paths; this is not a Native/provider ingress benchmark.

| Files | origin chain build | work chain build |
|---:|---:|---:|
| 250 | 19.849 | 15.620 |
| 500 | 95.574 | 56.871 |
| 1000 | 379.542 | 929.938 |
| 2000 | 1544.137 | 1691.583 |

Raw outputs and exact library artifact paths accompany the table.

The 100-line tclIndex process was briefly stopped for a live stack while in
the old uncached header path. Its old timing includes instrumentation effects
and must not be used as an uninstrumented latency ratio. The captured stack
is index100-live-stack-symbols.out. The separate PackSet control compares the
complete ordered Debug structures, not sorted lines: deterministic=true and
equal_complete_debug=true (packset-base-8a01a718-threeway.out).

Corpus audit: the original 102-path roster produced only 101 origin analysis
rows; native_deep_array_word.tcl contains intentional non-UTF-8 bytes. The
corrected ordinary roster adds classes.tcl, whose origin time is 740 ms, for
102 text inputs. The byte fixture remains a separately tested native-codegen
control; see corpus-roster-audit.json. The ordinary LSP rows above are
provisional: they asserted surviving lenses but must be repeated with the new
positive-reference-title assertion before ruling out unavailable reports.

## Positive-reference ordinary LSP controls, latest e4a01bbe

Both runs keep every declaration/index row/lens AND require exactly the one
positive call per fixture procedure. Origin uses its original ingress; current
work uses the actual retained editing-input path used by the document driver.
No unavailable or zero-reference report is counted as a successful timing.
Values are milliseconds, median of three queries on the retained analysis.
The host is contended; these are diagnostic scaling results.

| Procedures | origin index | work index | origin 100 misses | work 100 misses | origin lenses | work lenses |
|---:|---:|---:|---:|---:|---:|---:|
| 10 | 0.041 | 64.244 | 0.348 | 1.228 | 1.180 | 83.642 |
| 20 | 0.082 | 445.391 | 4.932 | 0.701 | 2.172 | 1380.511 |
| 40 | 0.381 | 675.448 | 1.227 | 3.103 | 27.844 | 8034.283 |
| 80 | 0.497 | 1557.801 | 4.502 | 3.219 | 106.465 | 25542.691 |

Raw inputs/artifact paths and query outputs are recorded in origin-lsp-positive-inputs.json and latest-e4a01bbe/lsp-build-inputs.json and lsp-*-positive.out.

## Latest full-batch synthetic controls, e4a01bbe

All listed work shapes passed full AnalysisResult old/new/old equality, with
deterministic old endpoints. Work times are the new middle run and old second
endpoint; origin calls are medians of three warmed timing-only runs, other
origin shapes are single timing-only samples. Concurrent load makes these
wall-clock observations noisy (notably calls 20); no monotone speedup or
release-latency claim follows from this table. Units: milliseconds.

| Input | origin/rust | work old2 | work new |
| --- | ---: | ---: | ---: |
| calls 10 | 56 | 10484 | 5301 |
| calls 20 | 76 | 5595 | 24119 |
| calls 40 | 129 | 34908 | 7113 |
| calls 80 | 109 | 33372 | 4985 |
| procs 41 | 532 | 6659 | 3073 |
| upvar 10 | 247 | 31113 | 42310 |
| upvar 30 | 752 | 397924 | 91987 |

Raw work data: latest-e4a01bbe/continued-proof/{calls,procs,upvar}.out.
All 15 hosted iRules still fail strict whole-result equality. The ordinary Tcl
corpus is incomplete; synthetic successes do not discharge that gate. See analysis-equality-routing.md for all-old
controls and the privately traced object-report comparison blocker.

Latest full ordered PackSet Debug comparison PASSED: all 484,936 lines agree;
old endpoints are deterministic. load_embedded: origin 4,665 ms, work old1
44,471 ms / old2 82,646 ms, work new 10,673 ms. The different old endpoint
latencies show host noise; this is not a stable speedup-factor estimate.
Raw: latest-e4a01bbe/continued-proof/packset.out.

## Paired process CPU controls, latest e4a01bbe

Linux process user+system CPU is read before/after each full analysis call;
equality/output formatting are excluded. Tick frequency is 100 Hz.
All five old/new/old shapes (including n=1 warmup) have deterministic endpoints
and complete AnalysisResult equality. The unchanged frozen libraries are used.
One paired sample per size; cache/memory effects and scheduling still vary.

| Calls | old1 CPU ms | new CPU ms | old2 CPU ms |
| --- | ---: | ---: | ---: |
| 10 | 1470 | 810 | 1260 |
| 20 | 3280 | 3600 | 4340 |
| 40 | 10350 | 2460 | 8480 |
| 80 | 35450 | 3760 | 21660 |

Raw: latest-e4a01bbe/synthetic-cpu-{calls.out,inputs.json,summary.json}.
These are analyser timings, not end-to-end LSP or WASI exit measurements.

The same paired CPU control for upvar also passed full old/new/old equality
with deterministic endpoints (n=1 warmup excluded):

| Upvar callers | old1 CPU ms | new CPU ms | old2 CPU ms |
| --- | ---: | ---: | ---: |
| 10 | 12810 | 3880 | 6070 |
| 30 | 71930 | 18670 | 58020 |

Raw: latest-e4a01bbe/synthetic-cpu-upvar{.out,-inputs.json,-summary.json}.
