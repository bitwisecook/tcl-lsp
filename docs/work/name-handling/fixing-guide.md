# Working rules for the naming migration burn-down

For every agent fixing failures on `work`. Read [`AGENTS.md`](../../../AGENTS.md) first; this adds the conventions specific to this burn-down. [Current state](current-state.md) holds the measured numbers.

## Goal

Every consumer (lexer, compiler, LSP, runtimes, VMs, tooling) reads Tcl names through `tcl_syntax::naming`, no consumer parses `::` or `name(index)` itself, and every test passes against its oracle: C Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and Jim Tcl 0.84. F5 dialects track Tcl 8.4 unless a BIG-IP observation says otherwise; an F5 question that needs a device goes into the [BIG-IP check list](bigip-name-checks.md), not into a guess.

## Deciding who is wrong

A failing test is one of four things. Say which in the commit.

1. **Code regressed.** The test exists on `rust` and its expectation is still right for its dialect. Fix the code. Compare with `git show origin/rust:<path>` (`git fetch --depth=1 origin rust` first).
2. **Fixture is not what it claims.** It names a release but builds a dialect-blind unit (`CommandRegistry::build_default()` with `CompilationUnit::build_for`), mixes two dialects, or its source never reaches the code under test (reads an unset variable, defines `proc ::ns::p` without the namespace, calls an undefined handler). Fix the fixture so it runs what it claims, under the release it names; keep the assertion.
3. **Expectation is wrong for the dialect.** Prove it with the real interpreter (`tmp/tcl8.4.20` … `tmp/tcl9.1.0`, the pinned `tclsh` binaries from `scripts/dev/tcl-reference-toolchains.sh`, `jimsh` where installed) and record the script and result in the test comment. Then change the expectation.
4. **Two tests contradict each other.** Decide with the oracle; delete or invert the loser.

Never weaken an assertion to make it pass, never `#[ignore]` a failure, never add a known-failure list.

## Display answers and proof answers

Semantic tokens, hover, completion, document symbols and inlay hints are *display* consumers: when the strict naming proof has no answer (a dynamic word, a body the analysis did not enter, a class declared but never instantiated), they answer from the written words, the document's own registry and the analysed declarations. Rename, edits, refactors, lowering, optimisation and codegen are *proof* consumers and keep the strict path. A display fallback must never feed a proof consumer. A diagnostic sits between the two: it may use the same fallback facts when the real interpreter confirms the defect it reports for that source and release, and must stay silent where the interpreter runs the code cleanly.

## Tests

- A test names the release it is about. A dialect-invariant test says so and runs under one named release; a release difference is one table-driven test over the releases, not copies.
- Collapse tests that assert the same thing at the same layer. Keep one test per behaviour per layer (owner unit test, consumer unit test, one end-to-end). Shared vector files (`rust/tcl-syntax/tests/data/*.txt`) are the pattern for cross-crate conformance.
- A test that silently returns when its oracle is missing is not coverage; make the skip visible or require the oracle.

## Running tests on a small machine

```bash
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
cargo nextest run -p <crate> --no-fail-fast --status-level fail --final-status-level none --failure-output never
```

- Install nextest 0.9.148 from `https://get.nexte.st/0.9.148/linux` into `~/.cargo/bin`.
- Take a baseline of the crate before changing it and diff failing-test lists; report "fixed N, new 0". A change that fixes some tests and breaks others is not done.
- `--failure-output final` on the compiler prints very large `Debug` values; pipe through `cut -c1-700`.
- Keep one package selection per session (`-p a -p b …`); changing it changes feature unification and rebuilds everything.
- The server e2e suite is not deterministic: rerun a new e2e failure three times alone (`--test-threads 1`) before believing it.

## Before every push

```bash
cargo fmt --all -- --check
cargo clippy -p <changed crates> --all-targets -- -D warnings -A clippy::assert_is_empty
cargo xtask registry-axes          # regenerates docs/generated/registry-axes.md; it pins line numbers
cargo xtask value-transfers --check
cargo xtask owner-resolution
cargo xtask callback-inventory --check
git fetch --depth=50 origin work && git rebase origin/work   # then rerun registry-axes if anything moved
git push -u origin work
```

`docs/generated/registry-axes.md` conflicts on almost every rebase: take either side, then regenerate. No hardcoded command names in consumers; if `registry-axes` reports an unwaived site, move the fact into the registry.

## Areas

Stay inside your area's crates so sessions do not collide; a fix that needs another area's crate is a small, separate commit, pushed promptly.

| Area | Crates |
| --- | --- |
| Compiler analysis | `tcl-compiler` (analyser, diagnostics, binding, CFG, SSA, SCCP, taint, optimiser) |
| Execution | `tcl-codegen`, `tcl-vm`, `tcl-engine-*`, `tcl-cmd-core`, `runtime/rust`, `tcl-cshim` |
| LSP features | `tcl-lsp-core` and `tcl-lsp-server` outside semantic tokens, `tcl-lsp-db` |
| Tokens, owner, tooling | semantic tokens, `tcl-syntax::naming`, `tcl-registry`, `tcl-spectcl`, `tcl-cli`, `tcl-mcp`, `tcl-irules`, `tcl-explorer`, `tcl-diagram`, `tcl-spec-studio`, `xtask` |

Record what you fixed, what is still open and why in [current state](current-state.md) under your area, as a short current snapshot.
