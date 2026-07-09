# KCS: How do I run the tcl-lsp stress-test suites?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

all-editors

## Question

How do I run the two issue #829 robustness stress suites — one that
drives the real server over the LSP API, one that hammers the
[salsa](../GLOSSARY.md#salsa) query database directly?

## Before you start

- A Rust toolchain (`cargo`) able to build the workspace; `rustup update
  stable` first if `cargo check` complains about an unsupported `rustc`
  version.
- Python 3.9+ on `PATH` for the LSP-API suite — standard library only,
  no `pip install` needed.
- Nothing else: `run_all.sh` builds the server binary itself every
  time it runs.

## Answer

### Run both suites together

```
scripts/stress/run_all.sh
```

Builds `target/release/tcl-lsp-server` on every run — cargo's cache
makes an up-to-date rebuild a fast no-op. Then it runs the
direct-infrastructure example, followed by all three LSP-API
scenarios. Defaults are moderate, sized for a pre-push sanity check
rather than a long soak.

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

The three scenarios test different things:

- `tokens` bursts concurrent edits and `semanticTokens/full` requests
  against several large documents. It asserts every response arrives
  within a hard latency ceiling.
- `startup` reproduces the exact race from issue #829: a workspace
  with a `source`-ancestor file and a module that inherits its
  `package require`, opened before the workspace scan finishes. It
  asserts the false-positive
  [`W120`](codes/kcs-diagnostic-w120-missing-package-require.md)
  clears once the server settles.
- `chaos` opens, edits, closes, and reopens many documents
  concurrently for the whole run, as a crash/hang/deadlock smoke
  test.

### Scale up for a longer soak run

```
DURATION=120 DOCS=16 PROCS=600 scripts/stress/run_all.sh
```

`PROFILE=debug` runs an unoptimised build instead (faster to rebuild,
slower to execute).

## How to tell it worked

The Python suite prints a `PASS`/`FAIL` line per scenario, for example
`[tokens] PASS` or `[startup] FAIL`. The Rust suite prints
`stress_concurrent_analysis: PASS` on success; it has no scenarios and
no equivalent `FAIL` line, so a failure shows up as failure-specific
detail (`TIMEOUT after N of M writes…`, `FINAL CHECK FAILED — …`)
followed by a non-zero exit. Either suite exits non-zero if anything
failed. `run_all.sh` ends with `==> stress suite: PASS` only when both
halves succeed; a failure stops the script before that line prints. If
a run fails, see [reconstructing a stress-test
failure](kcs-issue-reconstruct-a-stress-test-failure.md).

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [A stress-test suite run reported a failure](kcs-issue-reconstruct-a-stress-test-failure.md)
  — read the `STRESS_FAILURE:` reproduction bundle a failed run leaves
  behind.
- [`scripts/stress/README.md`](../../scripts/stress/README.md) — full
  reference: every scenario, every tunable, and the reproduction-bundle
  contents in detail.
- [`docs/design/rust/lsp-performance.md`](../design/rust/lsp-performance.md)
  — the semantic-token prioritisation fix these suites exist to prove.
- [W120 diagnostic](codes/kcs-diagnostic-w120-missing-package-require.md)
  — the workspace-scan race the `startup` scenario reproduces.
