# Stress-test suite

Two independent stress suites that put semantic-token prioritisation and the
W120 workspace scan under adversarial concurrent load, rather than the tidy
conditions of a unit or single-shot end-to-end test.

| Suite | What it exercises | Front end? |
|---|---|---|
| `../../rust/tcl-lsp-db/examples/stress_concurrent_analysis.rs` | The salsa query database directly — one writer thread hammering `SourceFile::set_text` against many reader threads running `semantic_tokens` / `file_analysis_incremental` concurrently | No — no LSP, no subprocess, no JSON-RPC |
| `stress_lsp.py` | The real `tcl-lsp-server` binary over stdio JSON-RPC, under concurrent edit/request load from multiple simulated editor panes | Yes — the real LSP API |

Run either independently, or use `run_all.sh` for a turnkey pass over both.

## When something fails: reproduction bundles

Both suites save failure details and fixtures for investigation.
Timing-dependent races are not guaranteed to reproduce on demand. The LSP
suite captures recent messages and the latest document text, which may differ
from the revision involved in an earlier failing request; it does not record
every intermediate state.

Grep the output for `STRESS_FAILURE:` — each line names a bundle directory.
The LSP `tokens` scenario caps bundles at three and reports any further
failures in its summary. Bundle contents depend on the scenario:

- a `FAILURE.md` / `repro.md` describing what happened, in plain English,
  plus (for the Rust suite) a ready-to-adapt static unit-test skeleton;
- (Rust suite and LSP `tokens` scenario) the captured document/fixture text
  as a directly-loadable `.tcl` file;
- (LSP suite) a bounded JSON-RPC transcript (`transcript.jsonl`) and recent
  server stderr. The transcript retains up to 4,000 recent messages and
  truncates outgoing document text beyond 400 characters; it is a diagnostic
  log, not a complete replay;
- (LSP suite `startup` scenario) the *entire on-disk workspace* — package
  index, ancestor file, filler files — copied out before its
  `TemporaryDirectory` would otherwise delete it the instant the failing run
  exits.

The Rust suite writes bundles under `$TMPDIR/tcl-lsp-stress-failure-*`; the
Python suite writes them under `$TMPDIR/tcl-lsp-stress-artifacts/` by default
(override with `--artifacts-dir` or `TCL_LSP_STRESS_ARTIFACTS`). Both use the
identical `STRESS_FAILURE:` marker, so `grep STRESS_FAILURE:` over the
combined output of a `run_all.sh` invocation finds every bundle from either
half in one command.

## Direct-infrastructure suite (no LSP)

```sh
cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
```

Tunables via environment variables (all optional):

```sh
STRESS_PROCS=600 STRESS_READERS=8 STRESS_WRITES=200 STRESS_TIMEOUT_SECS=120 \
  cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
```

Exit code is non-zero if any reader or the writer panicked, the writer failed
to complete every write within `STRESS_TIMEOUT_SECS` (the direct signal of a
deadlock — a reader must never block a concurrent edit indefinitely), or a
final correctness check against a fresh, uncached analysis fails. A read
coming back `Cancelled` is expected and healthy, not a failure.

## LSP-API suite

Requires only Python 3.10+ (standard library only — no `pip install` needed)
and a built server binary:

```sh
make rust-server PROFILE=release
python3 scripts/stress/stress_lsp.py --server-bin target/release/tcl-lsp-server
```

Run a single scenario, or tune duration / concurrency:

```sh
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario tokens --duration 30 --docs 8
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario startup --startup-iterations 10
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario chaos --duration 30
```

Scenarios:

- **`tokens`** — many large documents, concurrent rapid-edit + immediate
  `semanticTokens/full` bursts. Checks that responses arrive within the
  30-second request timeout and contain non-empty token data, and prints
  p50/p95/max latency. It does not validate token encoding.
- **`startup`** — a workspace with a `source`-ancestor file that requires a
  package, and a module using that package with no local `package require`,
  padded with filler files so the workspace scan has real work to do. Opens the module
  immediately (racing the server's own workspace scan, the way an editor
  restoring tabs races `initialized`), then polls `textDocument/diagnostic`
  for W120. Empty results and request errors are treated as no W120; this
  check does not establish that push diagnostics have converged.
- **`chaos`** — many documents opened / edited / closed / reopened rapidly and
  concurrently for the whole run; only checks the server stays alive and
  responsive throughout (a crash/hang/deadlock smoke test), not response
  correctness.

Exit code is 0 iff every requested scenario passed.

## Turnkey run

```sh
scripts/stress/run_all.sh
```

Builds the release server binary, then runs the direct-infra
example and all three LSP-API scenarios in sequence with moderate defaults
(short enough for a pre-push sanity check, not a long soak). Set `DURATION`,
`DOCS`, `PROCS`, or `PROFILE` in the environment to scale it up for a longer
soak run on a dedicated host:

```sh
DURATION=120 DOCS=16 PROCS=600 scripts/stress/run_all.sh
```

Exits non-zero if either suite fails.
