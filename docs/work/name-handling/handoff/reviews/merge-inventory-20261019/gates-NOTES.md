# Gates agent notes, 21 October 2026

These notes cover `current-state.md` rows 23, 14 and the cargo-deny part of row 1. Row 10 is the C84 oracle identity, which is not a gate file, so nothing changed there. The base is `work` at `e1117ebcd1`. The patch is `gates.patch`, made with `git diff` over the 14 files listed at the end. Nothing was committed, pushed or stashed. No file that another agent had modified was touched.

## Before and after

| Gate | Before | After |
| --- | --- | --- |
| cargo-deny (`scripts/dev/cargo-deny-all.sh`, cargo-deny 0.20.2) | rc=1 in CI job 119278178543: 4 advisories in the root `Cargo.lock`. The script stops at the first failing workspace. | rc=0 locally. All 12 workspaces print `advisories ok, bans ok, licenses ok, sources ok`. |
| `ty` (`make typecheck-py` step 1, ty 0.0.88) | rc=1, 27 diagnostics: 26 errors and 1 warning. This matches CI job 119280405813 exactly. | rc=0, `All checks passed!` (79 files). |
| `pyright` 1.1.414 (step 2, never reached in CI) | rc=1, 647 errors, all under `experiments/name-handling/` | rc=0, 0 errors |
| `ruff` (`make lint-py`) | pass | pass: 81 files formatted, `All checks passed!` |
| `test_name_resolution_proofs` (unittest) | n/a | 22/22 OK |
| owner-resolution (work) | rc=1, **608** diagnostics, reproduced exactly | rc=1, **310** diagnostics (see below) |
| owner-resolution (origin/rust `fcb7927f02`, its own gate and manifest) | rc=0: `OK (49 owner row(s) ...)` | n/a |
| owner-resolution unit tests | n/a | 76/76 pass, including one new test |

How these were measured:

- **xtask was not built.** It links tcl-compiler, tcl-lsp-core and the runtime, and there was not enough disk (1.7 to 2.7 GB free).
- **owner-resolution ran through a scratch harness.** The harness compiles the unmodified `rust/xtask/src/owner_resolution.rs` and `owner_resolution/source_internal.rs` through symlinks. It uses the same `syn` 3.0.9 and `rustc_lexer` 0.1.0 from the lockfile, and swaps in a stub `util::repo_root`. It reproduced the CI count of 608 exactly. The origin/rust run and the pre-merge run used the same harness on `git archive` snapshots.
- **Python checks ran in a scratch venv.** It holds pytest 9.1.1, minijinja and a `.pth` pointing at the f5report sources. The native `_engine` was not built; ty and pyright read the committed `_engine.pyi`. `make test-py-engine` (the native binding tests) was not run, because it needs a maturin release build.
- **All binaries built for this work have been deleted.** That covers the harness targets and the downloaded cargo-deny.

## 1. cargo-deny: dependencies upgraded, no ignores

CI found four advisories in the root `Cargo.lock`. `rust/bigip-report-gen/python/Cargo.lock` also had the two rustls ones; CI never reached it. Every advisory has a fixed version, so each was fixed with `cargo update -p <crate>@<old> --precise <new>`. `Cargo.toml` and `deny.toml` are unchanged.

| Advisory | Crate | Fix |
| --- | --- | --- |
| RUSTSEC-2026-0378 (component-model stream payload panic) | wasmtime 49.0.3 | 49.0.4 |
| RUSTSEC-2026-0379 (aarch64 Cranelift large `br_table`) | wasmtime 49.0.3 | 49.0.4 |
| RUSTSEC-2026-0381 (distrusted emSign roots) | webpki-roots 1.0.9 | 1.0.10 |
| RUSTSEC-2026-0382 (distrusted emSign intermediates) | rustls-webpki 0.103.15 | 0.103.16 |

The wasmtime update also moves its lockstep family from x.3 to x.4: 13 `cranelift-*` crates, 2 `pulley-*` crates and 10 `wasmtime*` crates. The `wasmtime = "49.0.3"` requirement already admits 49.0.4.

The root lock changes 54 lines and the bigip-report-gen python lock changes 4. Nothing else moves.

Not my files: `.github/workflows/ci.yml:1867,2058` and `.claude/hooks/session-start.sh:45` still install the wasmtime **CLI** at 49.0.3. cargo-deny does not audit that CLI, and CI runs on x86_64. The workflow owner may want to bump it to 49.0.4.

## 2. Python typechecking

All the failing files are `work`-only; none exist on origin/rust.

**Fixed in place.** These files have no current-byte SHA-256 recorded anywhere in the repository, so editing them breaks no receipt.

| File | Change |
| --- | --- |
| `bigip-probes/name-patterns/capture.py` | `TRAFFIC_HOOK: Callable[..., None] \| None` and a declaration-only `TAG: str`, which clears the warning. A `None` check on `extractfile` raises the same `RuntimeError` style. |
| `appliance.py`, `persistence.py` | Each deliberately overrides `capture.make_config` with its own config builder; that assignment is commented and given `# ty: ignore[invalid-assignment]`. `observations: list[dict[str, object]]`. |
| `parse_capture.py` | `if not errors and count is not None`. A `None` count always records an error first, so behaviour is unchanged. |
| `summarize-context-naming-evidence.py` | `dict[str, Any]` returns for `traffic_summary`, `chunked_event_results` and `trace_results`. |
| `check-name-resolution-proofs.py` | `cast(dict, pointer(...))`, with no behaviour change. |
| `test_name_resolution_proofs.py` | Explicit `None` guards for the importlib spec and loader. |
| `generate-certificate-ucs.py` | A `None` guard on `extractfile`. |

**Excluded from the type checkers only.** These two files stay in ruff lint. The Makefile adds `PY_TYPECHECK_EXCLUDED` and `PY_TYPECHECK_FILES`, with a comment.

- `scripts/dev/update-name-resolution-proof-index.py` (3 errors). Its current SHA-256 `e3a6728b…` is recorded in 16 receipts, under `experiments/name-handling/results/proof-source-currency/**` and in `docs/work/name-handling/repository-consumer-audit-final.json`.
- `scripts/dev/bigip-probes/resolution-2286/generate-followup-controls.py` (1 error). Its hash is pinned in `resolution-2286/evidence/CURRENT_TRANSPORT_SHA256SUMS`.

Editing either would invalidate retained evidence. Under user decision 2026-10-21T12:05Z that evidence stays on `work`.

**pyright** was never reached in CI. Its `include` covers `experiments/`, and locally all 647 errors are in `experiments/name-handling/**`, which is captured probe and oracle evidence. `pyrightconfig.json` now excludes `"experiments/name-handling"`, with a JSONC comment giving the same reason. ruff and ty already exclude all of `experiments/`. The other `experiments/*` directories still pass pyright.

## 3. owner-resolution (row 23)

Inputs:

- the contract `docs/design/contracts/shared-utility-contracts-rust.md`, which holds three marked tables: the five-column owner manifest, the internal ten-column table, and the source registration ten-column table;
- `Makefile`;
- `rust/xtask/src/main.rs`;
- fixed consumer files named inside the gate.

### Root cause of most of the 608

The source registration table contained one **five-column** row: "Live Studio source publication", from commit `681ab58e02` on 17 October, `work`-only. `source_internal::parse` therefore returned `source owner row requires ten columns`. That discarded every registration, so every private entry that was correctly registered also reported "no public declaration".

A second five-column row, "Original whole-document transform source currency" (`8ba203b880`, 16 October), sat just *after* `<!-- end-owner-resolution-manifest -->`, so the gate never checked it. Both rows were moved into the five-column manifest. That single change took the count from 608 to 389.

**The merge is not the cause.** The pre-merge `work` commit `03c943a52c`, with the same two rows moved, gives 388 diagnostics. Those are identical to the post-merge set except for the `.runtime_version()` finding. The working tree and committed HEAD give the same results.

### What I fixed in the manifest and gate (389 → 310)

1. **Formatting.** 40 source rows had a backticked visibility cell. The parser compares that cell raw, so `` `pub(crate)` `` never equals `pub(crate)`. I removed the backticks.
2. **Stale visibility.** `runtime/rust/src/interp/native_jim_lookup.rs` declares `OriginalJimLookupNamespace`, `::capture` and `::check_current_arguments` as `pub(crate)`, and has since 15–16 October. Five rows still said `pub(super)` or `private`. The manifest now records the code's actual visibility. No code was changed.
3. **Stale source paths.** Four rows name `Analyser` without listing its declaring file `rust/tcl-compiler/src/analyser/state.rs`, which has `pub struct Analyser`. One row names `SignatureSourceVariableOccurrence` without `rust/tcl-compiler/src/signature_scan/variable_symbol.rs`. I added the declaring file to each.
4. **Brace entry the parser cannot read.** I split `OriginalProjectSourceLoad::{path_word,literal_path,matches_source}` into three entries. All three are `pub` in `source_load.rs`.
5. **Unsupported cross-crate source rows removed (3).** These are `EditorWireSession::allocate`, consumed from `tcl-lsp-server` (2 rows), and `source_configured_class_words_at`, consumed from `tcl-lsp-core` (1 row). The source table only accepts same-crate routes and fails closed with "foreign consumer crate". All three entries are `pub`, and they are still verified through their five-column owner rows.
6. **Gate gap.** `xtask_dispatches` had no `gen-editor-catalogs` mapping, although `main.rs:548` dispatches `Command::GenEditorCatalogs { check }`. I added `"gen-editor-catalogs" => "GenEditorCatalogs"` and the unit test `the_editor_catalog_gate_resolves_to_its_dispatch_arm`.

No private item was made public. No code outside `rust/xtask` changed.

### Remaining 310

The full list, with the reason for each item, is in `gates-owner-resolution-remaining.json`.

**A. 227 "no public declaration".**

- **190 have no source registration at all.** Of these, 182 are real but non-public items: 141 `pub(crate)`, 51 private, 23 `pub(super)` and 4 `pub(in …)`. This count also includes some whose source rows fail. Rows name them under "Public entry points", and the contract requires a public declaration or a checked registration. One example is `safe_join`, `extract_tar` and `extract_zip` in `tcl-pkg/src/fetchers.rs`, which have always been private (row added by `29a96a7163`).
  - **Truthful options for each entry:** add a ten-column registration naming an actual same-crate consumer and route, or drop it from the entry-point column. Either is a contract-content decision for the naming owners, who know the intended consumer.
  - **Not done here:** generating about 180 consumer claims by script into a design contract that goes to the PR. Making the items `pub` is also ruled out.
- **37 cascade from failing source rows** (see B).
- **Gate limitation (3):**
  - `SOURCE_PRESENTATION` is generated by a macro in `tcl-registry/src/hooks.rs`, which the checker cannot see.
  - `OriginalLinkedSourceSpelling::text` and `RetainedSourceModuleBindings::matches_module` are effectively crate-private, because their owner types are `pub(crate)`.
- **Code missing: 4 entries never existed in any `.rs` file.**
  - `VendorSourceLiteralPurpose::EventName`
  - `Vm::refresh_native_jim_namespace_helpers`
  - `Vm::namespace_unknown_subcommand`
  - `OriginalSourceVariableAliasOperands::namespace_target_scope`

  `git log -S` finds them only in docs. These need the implementation owner to supply the code or withdraw the claim.
- **1 public item that exists only in `experiments/`:** `NativeInvocationState::arguments`. In product code `NativeInvocationState` is `pub(crate)` in `rust/tcl-vm/src/interp.rs`.

**B. 82 source registrations that do not resolve.** I instrumented a copy of `check_row` to see where each one fails.

Counts are per manifest row:

| Reason | Rows |
| --- | --- |
| The consumer body no longer reaches the entry by the registered route | 28 |
| The owner type cannot be resolved from the owner module (see below) | 21 |
| The consumer function was renamed or removed | 16 |
| The route matched, but another reference is ambiguous, shadowed or conditional | 13 |
| The owner entry was renamed or removed (`IndexLoader::collect_word`, `select_in_plan`) | 2 |
| Signature-type reference absent | 1 |
| Missing consumer module | 1 |

The 21 unresolvable owner types are 15 distinct entries. For 14 of them, the owner module uses a `use super::*` glob import, which the checker deliberately refuses. Most are `tcl-lsp-server` `Backend::*` methods in `document_transform.rs`.

The missing consumer module is `runtime/rust/src/interp/native_body_artifact/native_dictionary_scope.rs`. It is declared through a redundant `#[path = ...]` attribute, and the checker refuses path-changing attributes; this has been the case since `bfe46f7fc7`.

All of these are code/manifest drift on `work`. The registrations are code facts that only the owning agents can re-establish:

- compiler: 49
- runtime: 46
- lsp-server: 38
- lsp-core: 20
- vm: 15

**C. One real code finding: `.runtime_version()`, vm-runtime.** `rust/tcl-vm/src/cmd_package.rs:1163` `version_conflict` calls `requirement_text(requested, exact, vm.runtime_version())`. This came from `1fba5c3be5`, a `work`-only commit of 20 October made after the merge. Every other release choice in this file comes from the selected `NativePackageProtocol::C(release)`, for example lines 184 and 269. The gate is right to flag it as a bypass.

Suggested fix, not applied because the file is not mine: thread the protocol-selected `release` into `version_conflict` instead of reading `vm.runtime_version()`.

### Does the gate pass on origin/rust?

Yes. origin/rust's gate (616 lines, no source registration table) on origin/rust's 49-row manifest gives `owner-resolution: OK (49 owner row(s) resolve to live source and gates)`, rc=0. The expanded contract and gate exist only on `work`, and they enter the PR to `rust`. The 310 must therefore be resolved, or the rows withdrawn, before that PR's xtask-check can pass.

## Files in gates.patch

- `Cargo.lock`
- `rust/bigip-report-gen/python/Cargo.lock`
- `Makefile` (ty file list only)
- `pyrightconfig.json`
- `docs/design/contracts/shared-utility-contracts-rust.md`
- `rust/xtask/src/owner_resolution.rs`
- `scripts/dev/check-name-resolution-proofs.py`
- `scripts/dev/test_name_resolution_proofs.py`
- `scripts/dev/generate-certificate-ucs.py`
- `scripts/dev/bigip-probes/name-patterns/{capture,appliance,persistence,parse_capture}.py`
- `scripts/dev/bigip-probes/resolution-2286/summarize-context-naming-evidence.py`
