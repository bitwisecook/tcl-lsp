# Issue #1364 dispatch-stability proof — performance comparison

Base: `82157a6f8` (`origin/rust` merge of #1365). Candidate: `93adecc25`. Both
built with the same Rust 1.97.1 toolchain and
`cargo build --locked --release -p tcl-lsp-server` in isolated worktrees with
separate target directories. The pinned #1181 `small` corpus contains 113 Tcl
files from 8 georgtree repositories (corpus revision 1).

Five measured pairs followed a discarded warm-up per build. Pair order
alternated base→candidate, then candidate→base, so monotonic drift in machine
state cannot systematically favour one build. Every run swept the machine for
competing `cargo test` processes first and recorded the load average it
started at. Values are median ± median absolute deviation (min–max). A
positive paired delta favours the base.

## Headline

| Metric | base | candidate | Median paired delta | Candidate wins |
|---|---:|---:|---:|---:|
| Total wall | 37.351 ± 0.777 s (36.430–40.633) | 41.882 ± 0.184 s (41.697–42.851) | **+12.8%** | 0/5 |
| Total CPU | 61.000 ± 1.000 s (59.000–65.000) | 68.000 ± 0.000 s (68.000–70.000) | **+11.5%** | 0/5 |
| Peak RSS | 277.801 ± 3.660 MiB (274.141–301.816) | 283.609 ± 4.457 MiB (276.039–289.301) | **−0.6%** | 3/5 |
| Final RSS | 277.801 ± 3.660 MiB | 283.609 ± 4.457 MiB | −0.6% | 3/5 |
| Failed checks | 0/15 | 0/15 | — | — |

![Per-check wall time](walltime.svg)

![Resident memory over the run](memory.svg)

![CPU utilisation over the run](cpu.svg)

## Per-check latency

| Check | base | candidate | Median paired delta | Candidate wins |
|---|---:|---:|---:|---:|
| `scan.workspace` | 6.693 ± 0.052 s | 7.691 ± 0.191 s | +14.7% | 0/5 |
| `open.docs` | 0.248 ± 0.009 s | 0.238 ± 0.006 s | −4.2% | 3/5 |
| `edit.storm` | 3.076 ± 0.142 s | 2.792 ± 0.086 s | −9.2% | 3/5 |
| `edit.tokens` | 6.371 ± 0.339 s | 6.251 ± 0.101 s | −3.5% | 4/5 |
| `nav.hover` | 0.007 ± 0.000 s | 0.008 ± 0.000 s | +2.7% | 2/5 |
| `nav.definition` | 0.008 ± 0.000 s | 0.009 ± 0.001 s | +6.1% | 2/5 |
| `nav.references` | 1.742 ± 0.041 s | 1.904 ± 0.048 s | +11.8% | 0/5 |
| `nav.completion` | 0.105 ± 0.003 s | 0.110 ± 0.007 s | +10.0% | 2/5 |
| `nav.symbols` | 0.641 ± 0.007 s | 0.727 ± 0.008 s | +13.5% | 0/5 |
| `lens.resolve` | 0.457 ± 0.014 s | 0.457 ± 0.007 s | +0.0% | 2/5 |
| `action.codeaction` | 0.350 ± 0.018 s | 0.373 ± 0.059 s | +2.7% | 2/5 |
| `refactor.rename` | 16.284 ± 0.063 s | 20.084 ± 0.491 s | +19.0% | 0/5 |
| `refactor.filerename` | 0.325 ± 0.000 s | 0.323 ± 0.003 s | −0.6% | 3/5 |
| `nav.after_rename` | 0.709 ± 0.008 s | 0.786 ± 0.019 s | +12.3% | 0/5 |
| `close.docs` | 0.301 ± 0.000 s | 0.301 ± 0.000 s | +0.0% | 1/5 |

## Reading

**Memory is neutral.** The median paired RSS delta is −0.6% with the
candidate winning 3 of 5 pairs; the two builds' resident sets overlap. This is
deliberately unlike the #1360 comparison, where an eager semantic sidecar
tripled resident memory. The base already builds world-state SSA for any file
using `llength`, `format`, `lindex`, `join`, or `split` — the ten registry
specs carrying `CLOSED_REFERENTIALLY_TRANSPARENT` at the base commit — so the
graph this change reasons over was already being materialised. The dispatch
proof adds bounded per-domain ledgers on top, not a second graph.

**The cost is time, and it scales with files, not edits.** Every
workspace-wide check regressed on all five pairs — `refactor.rename` +19.0%,
`scan.workspace` +14.7%, `nav.symbols` +13.5%, `nav.after_rename` +12.3%,
`nav.references` +11.8% — while the single-document paths did not regress at
all: `edit.storm` −9.2%, `edit.tokens` −3.5%, `open.docs` −4.2%. A per-function
analysis paid once per analysed file predicts exactly this shape. `lens.resolve`
and `close.docs` are unchanged to three decimal places.

Part of that cost is irreducible: at the base commit common GVN abstained at
every site, so it built the sidecar and then declined. The candidate runs the
contents lattice to a fixpoint and, where the proof completes, builds value
keys and reports O105. Work that produces a result costs more than work that
declines.

**Known reducible cost.** Two candidates for a follow-up, both in
`dispatch_proof.rs`:

- `analyse_dispatch_stability` walks each function twice: a worklist fixpoint,
  then a separate `capture_block_sites` replay to record site proofs. The
  replay exists because a proof must be read from the converged entry state,
  but the final fixpoint iteration could capture instead of a second pass.
- The command-substitution barrier added to `world_state_ssa` enlarges the
  world graph, and token application there scans the tracked location set.

Neither is required for soundness, and neither was measured in isolation.

## Method notes and caveats

- **Linux, not macOS.** `scripts/perf/README.md` documents that the two
  platforms take materially different code paths — `nav.references` reads
  0.38 s on Linux against 9.97 s on macOS on the same corpus. These figures are
  a trend line for the checks this platform exercises, not release figures.
- **The harness hangs intermittently on this platform**, on both builds,
  roughly one run in five: the client blocks on a response while the server
  sits idle at ~0% CPU. Runs therefore carry a 240 s deadline and one retry;
  `pr1364-r4` and `base-r4` each needed a second attempt. This is a
  pre-existing condition, unrelated to the change, and worth its own issue.
- **`bench.py` could not write results at all on Python 3.11** before the fix
  in this branch: `Sampler` shadowed `threading.Thread._stop`, so every
  successful `join()` raised `TypeError` at teardown, after all 15 checks had
  run.
- One base repetition peaked at 301.8 MiB against a 274–278 MiB band for its
  other four, which is why the paired median rather than the pooled minimum is
  the honest summary.
- Published release binaries could not be added for context: `get_server.py`
  requires the `gh` CLI, which this container does not provide.

`results/` holds all ten measured runs. The graphs are rendered from the
median run of each build by `scripts/perf/report.py`.
