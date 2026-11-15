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
source scripts/dev/agent-build-env.sh      # per-worktree target, incremental on, no debuginfo
export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
# edit loop: one test binary, filtered
cargo nextest run -p tcl-compiler --lib -E 'test(/class_factories/)' --no-fail-fast --status-level fail --final-status-level none --failure-output final
# before a push: the whole suite of every crate you touched
cargo nextest run -p tcl-compiler -p tcl-irules --no-fail-fast --status-level fail --final-status-level none --failure-output never
```

Measured on a 4-core, 15 GB host, `tcl-compiler` library tests (the worst case):

| Build | Cold build | Rebuild after a one-line edit in `tcl-compiler` | Running 13,198 compiler and iRules tests |
| --- | --- | --- | --- |
| `opt-level 1`, incremental off (the old setting) | about 25 min | about 11 min | 10 min |
| `opt-level 0`, incremental on | 5.5 min | 50 s | 49 min |
| `opt-level 1`, incremental on (use this) | 14 min | 51 s | 10 min |

So keep `opt-level 1` and leave incremental on: an edit-to-test cycle is about a minute and the pre-push suite is a 1 minute rebuild plus its run. `agent-build-env.sh` now turns incremental on; it costs 5 to 10 GB of target directory, depending on how many test binaries the selection builds.

- Build only the binary you need in the loop: `--lib`, or one `--test it_4`. A nextest `-E` filter alone still builds every test binary of the selected packages.
- Keep one package selection per target directory (`-p a -p b …`). A different selection resolves different features (`syn`, `tokio`, `hashbrown`, and the workspace's own `test-support`, `html` and `truth-table` features) and rebuilds most of the tree.
- Run one cargo build at a time; two concurrent builds exhaust 15 GB of memory.
- When the disk fills, remove `target/debug/incremental` (it is rebuilt) or `cargo clean -p <crate>`. Do not delete files under `target/` by age: build-script outputs such as `aws-lc-sys` are not tracked by cargo.
- CI builds tests at `opt-level 0`. Four tests pass at `opt-level 1` and fail at 0 (stack and time budgets): `depth_guard::tests::the_source_walk_cap_fits_its_stack_budget`, the two `phi_undef_memo::nested_if_stages_*` tests and `tcl-irules` `walker::tests::deeply_nested_command_substitution_does_not_crash`. Check a change to recursion depth or a hot loop at `opt-level 0` as well.
- Install nextest 0.9.148 from `https://get.nexte.st/0.9.148/linux` into `~/.cargo/bin`.
- Take a baseline of the crate before changing it and diff failing-test lists; report "fixed N, new 0". A change that fixes some tests and breaks others is not done.
- `--failure-output final` on the compiler prints very large `Debug` values; pipe through `cut -c1-700`.
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
| Tokens, owner | semantic tokens, `tcl-syntax::naming`, `tcl-lexer`, `xtask` |
| iRules, registry, SpecTcl, tooling | `tcl-irules`, `tcl-registry`, `tcl-spectcl`, `tcl-sslictcl`, `tcl-cli`, `tcl-mcp`, `tcl-explorer`, `tcl-diagram`, `tcl-spec-studio`, the iRules rule-loader paths of `tcl-compiler` |
| Analysis performance | hot paths of `tcl-compiler` and `tcl-lexer`; behaviour must not change |

Record what you fixed, what is still open and why in [current state](current-state.md) under your area, as a short current snapshot.
