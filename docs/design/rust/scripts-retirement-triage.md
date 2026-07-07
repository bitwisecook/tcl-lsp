# `scripts/` retirement triage (Python-retirement prep)

> **Audience:** Maintainer / Contributor
> **Type:** Design (retirement triage / worklist)

> **Update (2026): completed.** Python is fully retired on this branch:
> the Bucket-A Python scripts and the `.pyz` zipapp builders were deleted
> with the Python engine, the Bucket-B survivors were ported to `cargo
> xtask` (or dropped), and CI runs the native gates. The "PORTED — not
> yet wired into CI", "not yet", and "what remains is execution" notes
> below describe the triage as it stood mid-flight; treat this doc as the
> historical worklist, not the current state.

Goal: decide, per `scripts/` tool, what happens to it as the in-tree Python
retires (**PYTHON-RETIRE**, API-PYO3). Verified against the **code + import
surface** of each script, not its docstring alone.

**Provenance note (why git history doesn't help here).** Almost every script
was mass-imported in **two squash commits** whose messages describe none of it:
`1e3a71b7` "Add regression tests and line continuation folding support (#541)"
added **7,901 files** (95 scripts + all CI / `.claude` / workflows), and
`94858698` "Rust rewrite rebased onto main" changed **3,353** (the 30
Python↔Rust port-scaffold scripts). So add-date / commit-subject tell you
nothing about intent per script — the code is the only reliable signal. The
handful with honest, single-purpose commits are the recent ones
(`rust_vm_tier_gap.py`, `tclsh_check.sh`, the extracted `release/*` helpers).

Buckets:

- **A — retire WITH Python (do NOT port).** Its subject-under-measurement is the
  Python implementation, or it's a Python↔Rust differential, or a
  Python-oracle→Rust generator whose output freezes at retirement. Kept only
  until PYTHON-RETIRE, then deleted with the engine. No porting work.
- **B — survivor (needs a decision).** Not tied to the Python impl; outlives it.
  Deduped into families below; each needs a call (port to `xtask` / keep as a
  Python dev-dep / freeze-artifact-and-drop-generator / delete).
- **RT — runtime scope.** Measurement for the VM / WASM / `runtime/rust` port,
  which is a separate scope ([`../runtime/runtime-execution-gaps.md`]). Decide
  there, not here.
- **PORTED — already in `xtask`.** The Python original is redundant once CI is
  flipped to the Rust verb; delete-after-flip.

---

## Bucket A — retire with Python (do not port)

**Python↔Rust differentials / cross-backend benches** (die the moment there's no
Python side):

- `dev/bench_lsp_backends.py` — spawns *both* LSP servers, Python vs Rust bench.
- `dev/bench_diff.py` — differ for the bench JSON the above emits.
- `dev/diag_parity/run.py` — Rust `tcl` vs Python `tooling.tcl.main` `diag` diff.
- `bigip_kind_differential.py` — Python vs Rust BIG-IP parser, per-kind.

**Measurement of the Python implementation** (no Rust subject):

- `dev/perf_semantic_tokens.py`, `dev/profile_semantic_tokens.py` (LSP vs direct
  — duplicate subjects), `dev/perf_spicegentcl.py`, `dev/perf_track.py` — Python
  LSP / semantic-tokens perf.
- `dev/memprof_workspace.py` — Python `BackgroundScanner` / `WorkspaceIndex`
  memory.
- `dev/dump_our_bytecode.py` — dumps the **Python** `compiler.codegen` disasm
  (Rust has the `bytecode-compare` skill + `tcl-explorer`).
- `dev/tcl_test_client.py` — Python diagnostics console client.

**Python-oracle → Rust generators** (their output is a checked-in Rust fixture /
data file; freeze it at retirement, the generator dies):

- `codegen/gen_f5_query_*_fixtures.py` (×9) — golden fixtures for the Rust
  `tcl-bigip-query` port.
- `codegen/gen_bigip_model_rust.py`, `codegen/registry_baselines.py`.
- `registry-audit/gen_bigip_rust.py`, `registry-audit/gen_event_descriptions.py`,
  `registry-audit/reconcile_irules_dialects.py`.

**Python-era tcltest sweep + triage infra** (the Rust-side gate + the RT trackers
replace these; the Python-VM / both-backend runners have no post-Python subject):

- `dev/run_tcl9_tcltest_sweep.py` (both backends), `dev/run_tcl9_vm_core.py`
  (Python VM), `dev/_tcl9_classify.py` (shared classifier).
- `dev/diff_tcl9_tcltest.py`, `dev/refresh_tcl9_baseline.py`,
  `dev/tcl9_triage_report.py`, `dev/tcl9_baseline_to_csv.py`,
  `dev/tcl9_samples_refgen.py`.
- `tcltest_sweep/{aggregate,measure_perf}.py`, `tcltest_sweep/{run_all,run_one}.sh`
  — the external-suite baseline generation (older generation).

**Python authoring aids for the Python registry** (registry authoring moves to
Rust; these scaffold Python sources):

- `registry/scaffold_tcl_commands.py` — scaffolds Python command-class files from
  man pages.

---

## PORTED — the `xtask` verbs (⚠ not yet wired into CI)

The `xtask` check verbs now have **Makefile targets** (`make xtask-check` →
`cargo xtask kcs-index-links` + `refcount-contract` + `diag-tables --check`; `make
xtask-audit-option-dialects` on-demand), verified locally. The **CI step** that
runs `make xtask-check` in the Rust-capable `rust-tests` job (`rust-gate.yml` +
`ci.yml`) is **prepared but pending a workflow-scoped push** — the session's
OAuth token can't modify `.github/workflows/*.yml`. Until that lands,
`kcs-index-links` keeps running as the Python gate in `make lint-py` so the
docs-link check never goes dark. The Python-only `ci-fast` job has no Rust
toolchain (`.github/actions/setup-build` installs uv/Python only), which is why
the gates must live in `rust-tests`, not `ci-fast`.

**Deleted 2026-07-01 (fully orphaned — no live Makefile/CI invocation of either
the Python or the xtask verb; nothing that runs today changed):**

- `check/refcount_contract.py` → `xtask refcount-contract` (both removed with
  the retired runtime — the refcount contract was specific to that
  runtime's exports and no longer applies to the Rust runtime). *(deleted)*
- `check/audit_option_dialects.py` → `xtask audit-option-dialects` — a *generator*
  (writes `tmp/option_dialect_audit.json`, needs built `tmp/tcl*/unix` trees), not
  a pass/fail gate, so it is a `make xtask-audit-option-dialects` on-demand target,
  **not** in the CI aggregate. *(deleted)*
- `build/tzdata_bundle.py` → `xtask tzdata-bundle` *(deleted; `data/tzdata.bin`
  is generated on demand, no build target invoked it)*

**Kept:**

- `check/kcs_index_links.py` → `xtask kcs-index-links` — the xtask verb is the
  successor (hard gate: broken docs link / unindexed design doc → non-zero), but
  the Python copy stays wired in `make lint-py` until the CI workflow step lands;
  then flip it out and delete the Python.
- `print_version.py` — live `:=` computing `HATCH_VCS_VERSION` / the Python
  **wheel filename** on every `make` parse; entangled with the still-live Python
  packaging (B3). Flipping it to `cargo xtask version` would force a cargo build
  on every `make` parse, so it retires with B3 instead.

## Decisions (2026-07-01)

- **B1 shipped-artifact generators → port to Rust** (3 xtask verbs above), each
  self-gated by `make check-generated` during the transition. `port_names.py`
  and the Rust-model/data generators are **not** ports (B1a → Bucket A).
- **B2 KCS DB builder → port to Rust** (`xtask gen-kcs-db`).
- **B3 Python zipapp distribution → retire with Python** (native-binary
  distribution takes over). Kept until that lands; not deleted now (it is the
  live shipping path).
- **PORTED → flip CI & delete**: 3 orphans deleted; `kcs-index-links` flips once
  the CI workflow step lands; `print_version.py` retires with B3.

## Prep status — is this "finished"?

The **prep** (triage + safe mechanical actions + turnkey specs) is complete:

- ✅ **100% triaged** — every `scripts/` tool has a bucket + successor + retirement
  trigger (families cover the globbed sets: `gen_f5_query_*`, `capture/*.sh`,
  `release/*.sh`, `zipapp-main/*`, `tcltest_sweep/*`, `explain_flow_test_harness/*`,
  `install/*`).
- ✅ **Safe deletions done** — 4 orphaned/ported Python dupes removed
  (`refcount_contract`, `audit_option_dialects`, `tzdata_bundle`; + earlier).
- ✅ **Check gates** — `make xtask-check` targets added + verified; CI step
  prepared (pending a workflow-scoped push).
- ✅ **Port targets specified turnkey** — B1 (×3) + B2 have input/output/Rust-source/
  verb/acceptance rows above; each self-gates via `check-generated`.

What remains is **execution, not prep**: implementing the 4 xtask generator verbs
and, at PYTHON-RETIRE, deleting Bucket A + B3. The order:

## Retirement sequence (PYTHON-RETIRE order of operations)

1. **Land the CI workflow step** (`make xtask-check` in `rust-tests`), then flip
   `kcs-index-links` out of `lint-py` and `git rm scripts/check/kcs_index_links.py`.
2. **Implement the 4 generator xtask verbs** (B1 ×3 + B2); repoint `make generate`
   / `check-generated` / `kcs-db` at them; delete the Python generators once
   `check-generated` is green off the Rust path. Freeze the B1a Rust-model output.
3. **Native distribution parity** (B3): replace `build/zipapps.py` + `zipapp-main/*`
   with native-binary packaging; retire `print_version.py` (wheel version).
4. **Delete Bucket A** — the measurement / Python↔Rust-differential / Python-era
   sweep scripts — in the terminal PYTHON-RETIRE sweep, alongside
   `compiler/` / `analyser/` / `server/` / ported `tooling/`.
5. **Runtime-scope (RT) scripts** are handled by the runtime scope
   ([`../runtime/runtime-execution-gaps.md`]) — reimplement the live tracker
   generators (`rust_vm_tier_gap` etc.) Rust-side if their scoreboards outlive
   Python; otherwise they retire in step 4.

Keep (never retire): `capture/*.sh` (reference capture from tclsh), the
`dev/*.sh` env helpers, `release/*.sh`, `install/*.sh`, and the integration-test
harnesses (B4–B7) — all backend-agnostic.

---

## RT — runtime scope (decide in runtime-execution-gaps, not here)

Measurement/generators for the VM / WASM / `runtime/rust` port. Several are the
**live tracker generators** the runtime index cites, so they outlive Python only
if reimplemented Rust-side:

- `dev/rust_tcltest_sweep.py`, `dev/rust_vm_tier_gap.py` (regenerates
  `rust-vm-tier-parity.md`), `dev/tcl9_ctcl_baseline.py` (C reference baseline).
- `dev/tcl9_wasm_sweep.py`, `dev/run_tcl9_wasm_core.py` (targeted the retired runtime),
  `dev/classify_tcl9_wasm_failures.py`, `dev/gen_tcl9_wasm_classification.py`.
- `dev/leak_sweep.py`, `dev/diff_leak_sweep.py`, `dev/bench_wasm_runtime.py`,
  `dev/perf_microbench.py`, `dev/run_external_test_suites.py`,
  `dev/bisect_tcltest.py`.
- `check/wasm_command_parity.py` — Python registry vs the retired runtime parity.

---

## Bucket B — survivors (need a decision)

Grouped by identical fate. One decision per family.

### B1 — Shipped-artifact generators → **port to Rust** (turnkey specs)

Each produces a **checked-in shipped artifact** from the Python registry and is
verified by `make check-generated`. Post-retirement the artifact must regenerate
from the **Rust** registry (`tcl-registry` / `tcl-bigip-query`). Port each to a
`cargo xtask <verb>`; the **transition is self-gating** — while both exist,
`make check-generated` diffs the Python output against the checked-in artifact,
so wire the xtask to write the same artifact and confirm `check-generated` stays
green, then delete the Python.

| Target (Python) | Artifact(s) | Rust source | xtask verb | Acceptance |
|---|---|---|---|---|
| `codegen/catalogs.py` | `editors/zed/src/generated/{tcl_commands,irule_events}.json`, `editors/vscode/src/generated/iruleEvents.json` | `tcl-registry` command/event tables | `xtask gen-editor-catalogs` | byte-diff vs committed JSON (`check-generated`) |
| `codegen/editor_settings.py` | `editors/vscode/src/generated/diagnosticCatalog.ts` (+ the `*.j2`-rendered settings) | `tcl-registry` `DiagCode` catalogue + `formatting::config` | `xtask gen-editor-settings` | byte-diff vs committed `.ts` |
| `dev/gen_query_builtins_doc.py` | `docs/references/f5_query/builtins.md` | `tcl-bigip-query` builtin registry | `xtask gen-query-builtins-doc` | byte-diff vs committed `.md` |

Ordering: `gen-editor-catalogs` first (pure registry read, JSON out — simplest),
then `gen-editor-settings` (needs the `.j2` template step reproduced or the
Tera/askama equivalent), then the doc. The existing `xtask diag-tables` is the
model — it already reads the `DiagCode` catalogue and writes `docs/generated/`
with a `--check` mode.

### B2 — KCS help database builder → **port to Rust**

- `build/kcs_db.py` — builds the KCS SQLite help index from `docs/kcs/**`
  markdown (imports `shared.help.kcs_db`), consumed at runtime for hover/help.
  Port to `xtask gen-kcs-db` (needs a Rust SQLite dep, e.g. `rusqlite`; the
  parse rules — `applies_to` / category — live in `shared.help.kcs_db` and are
  small). Acceptance: the emitted `.db` answers the same hover/help queries;
  a schema+row differential against the Python `.db` during transition.

### B1a — Python-only artifacts → **Bucket A** (retire with Python; no port)

Reclassified after checking the Rust side — these do **not** feed a Rust
artifact, so there is nothing to port; they die with Python:

- `codegen/port_names.py` — generates the **Python** `dialects/f5/bigip/_port_names_table.py`
  from `data/scf_port_names.csv`. The **Rust** table
  (`rust/tcl-bigip/src/model/port_names.rs`) already exists **independently** —
  no script generates it (hand-maintained/frozen), so the Python generator + its
  Python table just retire. *(If a Rust CSV→`port_names.rs` regenerator is ever
  wanted it is a fresh xtask, not a port of this script.)*
- `codegen/gen_bigip_model_rust.py`, `registry-audit/gen_bigip_rust.py`,
  `registry-audit/gen_event_descriptions.py`,
  `registry-audit/reconcile_irules_dialects.py` — Python-oracle→Rust generators.
  Their output (`rust/tcl-bigip/src/model/gen/*.rs`, registry data) is checked
  in; at retirement **freeze the generated Rust** (it becomes the source of
  truth) or move the `OBJECT_SPECS` definition into Rust. Either way the Python
  generator retires; no Rust-side regenerator is required for retirement.
- `codegen/registry_baselines.py` — regenerates the `tests/baselines/registry/`
  fixtures from the registries; a test oracle, frozen at retirement.

### B3 — Python distribution packaging

- `build/zipapps.py` + `zipapp-main/{ai,gui,lsp,mcp,tcl}.py` — build the `.pyz`
  zipapps that are the **current** distribution for the LSP/CLI/MCP/AI/GUI.
  These die when native Rust-binary distribution reaches parity — but not before.

### B4 — Reference capture (backend-agnostic, feeds the Rust gates)

- `capture/{bytecode,bytecode_84,test_results,test_results_84}.sh` — capture
  reference disassembly / test results from real tclsh 8.4–9.0. Drive `tclsh`,
  not Python; the Rust differential gates consume their output. **Keep.**

### B5 — Dev environment / CI shell (backend-agnostic)

- `dev/ensure-test-deps.sh`, `dev/resolve-tcl-library.sh`, `dev/tclsh_check.sh`,
  `dev/test-slow-runner.sh`, `test-slow-stamp.sh`, `worktree-fingerprint.sh`,
  `build/render_logo.sh`, `screenshots.sh`. **Keep.**
  (`fetch_tcl_regex.sh` retired with the C Henry-Spencer regex vendoring — the
  ARE engine is now the pure-Rust `tcl-regex` crate.)

### B6 — Release / install shell (backend-agnostic)

- `release/*.sh` (tag, prerelease, github_release, vsce_publish, publish_{zed,
  sublime,verify}, jetbrains_token, codeql_gate, smoke_installer),
  `install/{install,hooks}.sh`. **Keep** (the zipapp-publishing steps adjust when
  B3 changes).

### B7 — Integration-test harnesses

- `eglot_test/run.sh` (Emacs eglot LSP client), `explain_flow_test_harness/*`
  (f5 explain-flow E2E), `test-cross-server.sh` (compares servers — Python side
  drops at retirement). Mostly **keep** as behavioural tests; `test-cross-server`
  loses its Python arm.
