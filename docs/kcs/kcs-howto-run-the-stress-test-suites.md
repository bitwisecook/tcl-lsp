# KCS: How do I run the tcl-lsp stress-test suites?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

all-editors

## Question

How do I run the two issue #829 robustness stress suites — one that
drives the real server over the LSP API, one that hammers the salsa
query database directly — and what do I do when one reports a failure?

## Before you start

- A Rust toolchain (`cargo`) able to build the workspace; `rustup update
  stable` first if `cargo check` complains about an unsupported `rustc`
  version.
- Python 3.9+ on `PATH` for the LSP-API suite — standard library only,
  no `pip install` needed.
- Nothing else: `run_all.sh` builds the release server binary itself if
  it is missing.

## Answer

### Run both suites together

```
scripts/stress/run_all.sh
```

Builds `target/release/tcl-lsp-server` if it does not already exist,
then runs the direct-infrastructure example followed by all three
LSP-API scenarios, with moderate defaults sized for a pre-push sanity
check rather than a long soak.

### Run the direct-infrastructure suite alone

```
cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
```

One writer thread hammers `SourceFile::set_text` against several reader
threads running `semantic_tokens` / `file_analysis_incremental`
concurrently — no LSP, no subprocess, no JSON-RPC. Tune it with
environment variables:

```
STRESS_PROCS=600 STRESS_READERS=8 STRESS_WRITES=200 STRESS_TIMEOUT_SECS=120 \
  cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
```

### Run the LSP-API suite alone

```
make rust-server PROFILE=release
python3 scripts/stress/stress_lsp.py --server-bin target/release/tcl-lsp-server
```

`--server-bin` accepts a relative or absolute path, or a name resolved
against `PATH`. Run a single scenario, or tune duration and
concurrency:

```
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario tokens --duration 30 --docs 8
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario startup --startup-iterations 10
python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario chaos --duration 30
```

The three scenarios: `tokens` bursts concurrent edit +
`semanticTokens/full` requests against several large documents and
asserts every response arrives within a hard latency ceiling; `startup`
reproduces the exact race from issue #829 (a workspace with a
`source`-ancestor file and a module that inherits its `package
require`, opened before the workspace scan finishes) and asserts the
false-positive `W120` clears once the server settles; `chaos` opens,
edits, closes, and reopens many documents concurrently for the whole
run as a crash/hang/deadlock smoke test.

### Scale up for a longer soak run

```
DURATION=120 DOCS=16 PROCS=600 scripts/stress/run_all.sh
```

`PROFILE=debug` runs an unoptimised build instead (faster to rebuild,
slower to execute).

## How to tell it worked

Both suites print a `PASS`/`FAIL` line per scenario and exit non-zero if
anything failed. `run_all.sh` ends with `==> stress suite: PASS` when
both halves succeed.

## What to do when it fails

Grep the combined output for `STRESS_FAILURE:` — each line names a
self-contained reproduction bundle directory with everything needed to
reconstruct the failure without re-running the (inherently
timing-dependent) stress harness: the exact document text as a
directly-loadable `.tcl` file, a JSON-RPC replay transcript and recent
server stderr for the LSP suite, and a ready-to-adapt static unit-test
skeleton for the Rust suite. The Rust suite writes bundles under
`$TMPDIR/tcl-lsp-stress-failure-*`; the Python suite writes them under
`$TMPDIR/tcl-lsp-stress-artifacts/` by default (override with
`--artifacts-dir` or `TCL_LSP_STRESS_ARTIFACTS`). Both suites use the
identical marker, so one `grep STRESS_FAILURE:` over a `run_all.sh` run
finds every bundle from either half. Turn the bundle into a permanent
regression test near the query or handler it exercised, matching the
suite's own test-fixture conventions.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [`scripts/stress/README.md`](../../scripts/stress/README.md) — full
  reference: every scenario, every tunable, and the reproduction-bundle
  contents in detail.
- [`docs/design/rust/lsp-performance.md`](../design/rust/lsp-performance.md)
  — the semantic-token prioritisation fix these suites exist to prove.
- [W120 diagnostic](codes/kcs-diagnostic-w120-missing-package-require.md)
  — the workspace-scan race the `startup` scenario reproduces.
