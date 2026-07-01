# `scripts/` retirement triage (Python-retirement prep)

> **Audience:** Maintainer / Contributor
> **Type:** Design (retirement triage / worklist)

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

The `xtask` Rust port exists and was parity-checked, **but `xtask` is not invoked
anywhere in the Makefile or CI** (`grep xtask Makefile .github/` is empty). So
the ports are dormant code, and "flip CI" means *wiring the verb into CI*, not
merely deleting the Python. Two states:

**Deleted 2026-07-01 (fully orphaned — no live Makefile/CI invocation of either
the Python or the xtask verb; nothing that runs today changed):**

- `check/refcount_contract.py` → `xtask refcount-contract` *(deleted)*
- `check/audit_option_dialects.py` → `xtask audit-option-dialects` *(deleted)*
- `build/tzdata_bundle.py` → `xtask tzdata-bundle` *(deleted; `data/tzdata.bin`
  is generated on demand, no build target invoked it)*

  ⚠ Follow-up: these contract checks now run **nowhere** — wire the three xtask
  verbs into CI so the gates are live again.

**Kept — still load-bearing; flip needs CI surgery, not just deletion:**

- `check/kcs_index_links.py` — live in `make lint-py`; flipping to
  `cargo xtask kcs-index-links` needs the Rust toolchain in that lint job.
- `print_version.py` — live `:=` computing `HATCH_VCS_VERSION` / the Python
  **wheel filename** on every `make` parse; entangled with the still-live Python
  packaging (B3). Retires with B3.

## Decisions (2026-07-01)

- **B1 + B2 artifact generators → port to Rust** (`xtask` / `build.rs`, reading
  the Rust registry) so the shipped artifacts keep regenerating post-Python.
  Recorded as API-PYO3 (`scripts`→`xtask`) targets; not deleted.
- **B3 Python zipapp distribution → retire with Python** (native-binary
  distribution takes over). Kept until that lands, then deleted with the engine —
  moved into Bucket A intent, not deleted now (it is the live shipping path).
- **PORTED → flip CI & delete**: applied to the 3 orphans above; the 2 live ones
  are blocked on wiring `xtask` into CI.

---

## RT — runtime scope (decide in runtime-execution-gaps, not here)

Measurement/generators for the VM / WASM / `runtime/rust` port. Several are the
**live tracker generators** the runtime index cites, so they outlive Python only
if reimplemented Rust-side:

- `dev/rust_tcltest_sweep.py`, `dev/rust_vm_tier_gap.py` (regenerates
  `rust-vm-tier-parity.md`), `dev/tcl9_ctcl_baseline.py` (C reference baseline).
- `dev/tcl9_wasm_sweep.py`, `dev/run_tcl9_wasm_core.py` (Zig),
  `dev/classify_tcl9_wasm_failures.py`, `dev/gen_tcl9_wasm_classification.py`.
- `dev/leak_sweep.py`, `dev/diff_leak_sweep.py`, `dev/bench_wasm_runtime.py`,
  `dev/perf_microbench.py`, `dev/run_external_test_suites.py`,
  `dev/bisect_tcltest.py`.
- `check/wasm_command_parity.py` — Python registry vs Zig runtime parity.

---

## Bucket B — survivors (need a decision)

Grouped by identical fate. One decision per family.

### B1 — Editor/registry/doc artifact generators (Python-registry-tied)

Produce **checked-in shipped artifacts** from the Python registry. Post-retirement
the artifact must be regenerated from the Rust registry, so each is
port-to-Rust *or* freeze-artifact-and-drop.

- `codegen/catalogs.py` — editor command catalogs from the registry.
- `codegen/editor_settings.py` — editor settings (imports `server._codes_init`,
  `shared.codes`, `tooling.formatter.config`; also shells `cargo`).
- `codegen/port_names.py` — generates `dialects/f5/bigip/_port_names_table.py`
  from the SCF port-name CSV.
- `dev/gen_query_builtins_doc.py` — `docs/references/f5_query/builtins.md` from
  `dialects.f5.query.builtins`.

### B2 — KCS help database builder

- `build/kcs_db.py` — builds the KCS SQLite help index from `docs/kcs/**`
  markdown (imports `shared.help.kcs_db`). Consumed at runtime for hover/help.
  Port the builder to Rust, keep as a Python build-dep, or freeze the `.db`.

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
  `fetch_tcl_regex.sh`, `build/render_logo.sh`, `screenshots.sh`. **Keep.**

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
