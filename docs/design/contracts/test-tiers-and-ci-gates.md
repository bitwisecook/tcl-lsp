# Test tiers and CI gates

What runs where, which gate a push must pass, and the policies that keep the
tiers honest. The Makefile is the executable form; this file is the contract
behind it.

## Tiers

| Tier | Runs | What |
|---|---|---|
| **smoke** — `make smoke`, `make smoke-p P=<crate>` | locally after every compile; inside `make prep-pr` | the fail-closed smoke-named function/module and effective Cargo-target subset owned by `scripts/dev/smoke-targets.tsv`, one sanity check per crate, seconds warm. Reuses the dev-profile default-features build (never `--all-features`), so it never forces a recompile. |
| **deep** — CI jobs `rust-tests`, `rust-tests-heavy`, `runtime-rust-tests`, `lsp-e2e`, `test-ext`, `test-ext-web`, `cargo-deny`, `python`, `spectcl-compat` | every PR and every push to `rust` | the full workspace suite (native `lsp_e2e` included), the VM-sim heavies, the standalone `runtime/rust` unit suite, the VS Code extension on desktop and in a browser host, supply-chain audit, Python lint/typecheck. Skips only what demonstrably did not change (below). |
| **exhaustive** — `make test-exhaustive`, `make fuzz`, `make tcltest-sweep[-check]` | only when a human invokes it by name | every `#[ignore]`d corpus sweep over `tmp/tcl*` and tcllib, differential-fuzz gates, privileged bpf/kernel tests, fuzz campaigns. **Never** wired into `prep-pr`, `test`, `check-all`, or CI. |

## Decision rules / contracts

1. **Fuzzing is always manual.** Campaigns (`make fuzz`, `tcl-fuzz`) and
   fuzz-shaped `#[test]`s — any body driven by a generator or seeded-random
   exploration (edit storms, permuted-input loops) — are `#[ignore]`d into the
   exhaustive tier regardless of how fast they run today. Deterministic
   fixed-input tests over the same code stay in CI. `tcl-fuzz`'s own plumbing
   unit tests are ordinary tests of the fuzzer and run in `rust-tests-heavy`.
2. **`#[ignore]` has one sanctioned steady-state use:** a permanently
   expensive or environment-gated test (corpus sweep, differential-fuzz gate,
   root + live kernel), marked
   `#[ignore = "<reason>; run explicitly with --ignored"]`. These form the
   exhaustive tier and are not debt.
3. **No shipped xfails.** An expected-failure marker is an intermediate
   state while a feature is under development; fix the root cause and remove
   the marker before release. Do not confuse an xfail with rule 2. The one
   standing exception is the eglot semantic-token repaint failure in
   `make test-emacs`: an upstream eglot painter bug (issue #333,
   `eglot--semtok-font-lock-2` stacks stale faces mid-response). The server
   is proven correct by the reference client
   (`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`), so
   never chase it, block on it, or "fix" it server-side. Repro and evidence:
   `editors/emacs/README.md`, `scripts/eglot_test/`.
4. **Emacs runs nowhere in CI.** Touch `editors/emacs` only with a local
   `make test-emacs` run.
5. **Local gates, in order, before every push:**

   ```
   make rust-check     # fmt + clippy + xtask drift gates; mirrors CI's pr-gate
   make prep-pr        # format + codegen + lint/typecheck + smoke
   ```

   `rust-check` is the minimum for Rust-only changes. `check-all` (lint +
   typecheck across TypeScript, Rust, Python) is the surface to run alone
   after touching TypeScript or Python. Failures are fixed, not skipped;
   tooling-missing skips are deliberate (`SKIP_CHECK_RUST=1`, …). Commit
   whatever formatting `prep-pr` applies, then re-run the gate — a
   `cargo fmt` after the commit is not a pass. Every "pr-gate bounced on a
   trivial lint" on this repo was a push that skipped this step.
6. **CI carries the deep suites.** Rebase on `rust`, run `prep-pr`, open the
   PR, subscribe to its activity, fix forward. Do not block on the full suite
   locally. To reproduce a deep-tier failure or add confidence on a risky
   change: `make test` (workspace + extension + runtime port + Zed query
   check), `make test-spectcl-compat`, the browser host
   (`make lsp-server-wasm`, then `npm run test:web` in `editors/vscode`),
   and `make test-emacs`. No single target mirrors the whole CI test
   surface, and none is a precondition for the PR.
7. **Capture long gate output to a file, never just `tail`.** A failure in the
   middle of a ten-minute run is lost to a 50-line tail, and summary lines get
   pushed off by skip spam:

   ```
   make test-rust 2>&1 | tee /tmp/test-rust-<branch>.log
   grep -nE 'FAIL|ERROR|panicked|error\[|^E ' /tmp/test-rust-<branch>.log
   ```

   Keep the file until the PR merges so a CI ping can be re-investigated
   without re-running the gate.
8. **Every smoke source/target pair has one manifest row.**
   `scripts/dev/smoke-targets.tsv` maps smoke-bearing Rust sources to their
   library, binary, integration-test, example, or benchmark target. The
   `cargo xtask smoke-targets check` drift gate inventories ordinary
   `#[test]` functions whose names or module paths start with `smoke`, plus
   testable Cargo targets whose effective names are `smoke` or `*_smoke`, and
   rejects missing, stale, or ambiguous rows. Inventory is independent of the
   current platform's `required-features` availability so one manifest remains
   valid on every supported target; execution skips targets unavailable in the
   resolved host context. If distinct testable Cargo targets compile the same
   source, each target requires its own row. Ownership follows
   Cargo target/module traversal rather than package-directory ancestry because
   Cargo target paths may refer to workspace sources outside their package
   directory. Literal `include!` sources retain the Cargo target that reaches
   them; modules and further includes declared there resolve beside the
   included file, matching rustc. A test source filename is not a separate
   selection rule when the manifest explicitly gives its Cargo target another
   name. Attribute macros and unresolved `include!` expressions that generate
   a smoke test cannot be inferred from tracked Rust syntax; this includes
   non-literal paths and literal files that do not exist until `build.rs` runs.
   Mark their declaring source with an exact standalone
   `// tcl-lsp-smoke-target` line comment. Every lexically present invocation
   is checked, including those in
   macro bodies and expression or statement contexts. An outer declarative or
   procedural macro that constructs an `include!` invocation during expansion
   is arbitrary generated syntax, like an attribute-generated test, and its
   declaring source carries the smoke-target marker. A source with exactly one
   unresolved include that is known to generate no tests may instead use
   `// tcl-lsp-no-smoke-include`; adding a second invocation requires an
   explicit reclassification so one data include cannot mask generated tests.
   The checker tokenises Rust, so marker and include text inside strings or
   comments is ignored. `cargo
   xtask smoke-targets run` is the
   no-nextest fallback and must select the same tests as the nextest smoke
   profile without changing workspace feature resolution;
   `make smoke-p P=<crate>` keeps `--workspace` resolution in both branches,
   intersecting nextest's `package(<crate>)` test filter with the smoke
   profile's `default()` filter, or applying the fallback's validated
   `smoke-targets run --package <crate>` manifest filter. Direct
   harness execution preserves
   an owning build script's explicit platform dynamic-library-path variable
   verbatim, without appending the reconstructed profile/sysroot suffix;
   only packages without that override receive reconstructed qualifying
   `rustc-link-search` paths. Those paths include normal, development, and
   build-only dependency closures because Cargo exposes build-dependency
   search paths to a test harness even under resolver v2. The harness also
   preserves an inherited `CARGO_MANIFEST_LINKS`; an explicit value emitted by
   the owning build script overrides it, matching Cargo.

## CI redundancy contract

CI skips only what demonstrably did not change. The rules live in
`ci.yml`'s `channel` job; read its header before editing them.

- A **tag** whose SHA went green on a `rust` push within 24 h step-skips the
  test surface (the release graph still runs).
- A **merge push** byte-identical to its already-green PR head downgrades
  tests to a cache-warming build (`--no-run`).
- **Docs-only** changes skip the cargo test steps; `python`, `test-ext`, and
  `test-ext-web` run only when their input paths changed (`test-ext-web` on
  `ext_changed` or `lsp_wasm_changed`, since it consumes both).
- `runtime-rust-tests` runs the standalone `runtime/rust` unit suite
  (`make runtime-rust-test`) only when `runtime_rust_changed` is true — that
  crate plus the path-dependency closure its own lockfile resolves. It is its
  own job because `runtime/rust` is its own cargo workspace: the root
  `cargo test --workspace` never reaches it, and `wasm-real-link` builds and
  links it *without running its tests*, so before this job a standalone-runtime
  semantic regression could land with every required check green (#1768). The
  closure lives in `scripts/dev/runtime-rust-path.sh`, gated against the
  committed lockfile by `scripts/dev/test-runtime-rust-paths.sh`
  (`make check-runtime-rust-paths`, part of `xtask-check`), so it cannot
  silently narrow. It is an additional semantic gate, not a replacement for
  the real link.
- `cargo-deny` never skips: new advisories arrive against unchanged trees.
- Every skip fails safe: API error or ambiguity → run everything.

Trusted pull requests prefer the self-hosted `tank` runner for `rust-tests`.
When that serial queue is overloaded, a maintainer may cancel the queued run
and manually dispatch CI against the same pull-request head branch with
`rust_tests_runner` set to `hosted`. This changes runner placement only: it does
not skip a test, alter the nextest filter, or carry forward a result. Fork pull
requests and runner-policy changes always use hosted capacity. Tank jobs retain
every pending request and remain serial because its runner registrations share
one physical host.

Keep these properties: skips are **step-level** (jobs still report success so
required checks and the release `needs:` graph hold), keyed on **content
identity** (tree/SHA, never a label or commit message), and bounded in time.

## Suites worth knowing

- `rust/tcl-lsp-server/tests/*_e2e.rs` — native LSP end-to-end (30 suites,
  `cargo test`).
- `rust/tcl-registry/tests/registry_sweep.rs`, `registry_commands.rs` — the
  registry generates real Tcl and iRules and asserts live analysis (arity
  E002/E003, subcommands E001/W001, event scoping IRULE1001/1002, ordering).
  Contract: [registry-contract-tests.md](registry-contract-tests.md).
- `rust/tcl-irule-test` — TMM simulation for iRules without hardware.
  Contract: [irule-test-framework.md](irule-test-framework.md).
- `runtime/rust/` — the WASM runtime's own leak round-trip + eval suite,
  `make runtime-rust-test`.
- `make test-spectcl-compat` — fail-closed SpecTcl compatibility against the
  manifest-pinned exact Tcl 9.0 interpreter.

## File-path anchors

- `Makefile` — `smoke`, `prep-pr`, `rust-check`, `check-all`, `test`,
  `test-exhaustive`, `fuzz`; `make help` lists everything.
- `.config/nextest.toml` — the `smoke` and `exhaustive` profiles.
- `scripts/dev/smoke-targets.tsv` — exact smoke-source to Cargo-target
  ownership used by the no-nextest fallback.
- `rust/xtask/src/smoke_targets.rs` — fail-closed smoke inventory validation
  and exact Cargo fallback execution.
- `.github/workflows/ci.yml` — the `channel` and `pr-gate` jobs.

## Discoverability

- [`AGENTS.md`](../../../AGENTS.md) — the agent-facing summary of the gates.
- [differential-fuzzing.md](differential-fuzzing.md) — the fuzzer contract.
- [release-and-publish.md](release-and-publish.md) — what CI may and may not
  do after the tests pass.
