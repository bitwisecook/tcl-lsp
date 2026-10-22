# LSP agent (after Luna-41) — measurements, fixes, routed causes

Owned paths: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server` (not the BIG-IP
modules). I committed nothing; the coordinator's WIP snapshots carry my edits (last seen in
`56893780b9` and later). Run conditions: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0`, `TCL_LSP_REQUIRE_TCLSH=9.0,8.6`,
`TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`, `--test-threads=2`, each test executable deleted
after its run. Numbers are "HEAD + shared working tree at that moment"; compiler agents landed
changes between runs. Temporary probes (`tests/zz_lsp_agent_probe.rs`) were deleted after use.

## 1. Integration targets (tcl-lsp-core) — before / after

origin/rust passes all of these (validation row 20: 239/239 incl. name_resolution 75/75 on
`fcb7927f02`).

| target | Luna-41 hand-back | first run here (nr0/h1) | latest run | remaining failures (owner) |
|---|---|---|---|---|
| name_resolution | 69/77 | 68/77 | **69/77** (p5) | itcl ×4 (compiler: in-namespace class join + calls inside class-member bodies), `fn_to_tp_an_explicit_namespace_import_is_followed` (C6, compiler-names), `tn_object_command_dispatch_is_scoped_to_its_creation_namespace` (compiler-names, §3a), `irules_switch_glob_body_is_followed` (iRules `proc` invocation, §3d), `pkgindex_dir::fp_other_undefined_var_in_pkgindex_still_fires` (C9 W210) |
| hover_completion | 50/56 | 52/56 | **55/56** (p5) → 56/56 expected with the past-end-of-line fix (pending re-run) | — |
| hover_residual | 27/36 | 28/36 | **33/36** (p4) | option word / 2× regexp pattern with an undefined `$var` operand (compiler, §3c) |
| references_residual | 16/23 | 16/23 | **21/23** (p5) | `document_highlights_dedup_keeps_write_over_read_on_collision` (synthetic `AnalysisResult::default()` + injected reporting map — the original-context contract refuses it; needs a decision), `references_external_obj_method_inside_proc_body_subst` (tclsh contradicts the fixture: `$d` is unset inside `proc run`; needs a decision) |
| references_rename | 70/98 | — | **71/98** (p4) | ~15 refuse `unknown-completed-command-world` (`puts [greet]`, compiler R1 not effective on this tree, §3b), class/object-variable renames refuse (§3b), itcl rename ×2 (in-namespace join), others listed in `scratchpad/lsp/out-p4-references_rename.log` |
| force_import_shadow_consumers | 2/10 | 2/10 | **6/10** (p5, ABSTAIN decision) | 4 not-shadowed halves that never passed on work: inline-proc / code-action offer no local `{args}` body, prepare_rename None, rename refused `unknown-completed-command-world` |
| call_hierarchy | (7/7 row 20a) | — | **7/7** | — |
| inlay_hints | (9/9 row 20) | — | **9/9** | — |

(Draft — sections 2–6 follow at hand-back.)

## 4. WASI host failures (validation row 17) — diagnosis

No wasip1 build was made (a release LTO wasip1 build of the whole stack needs well over the
shared ~2 GB). Diagnosis from code plus native measurements:

**Mechanism (confirmed in code).** `tcl_lsp_server::rt::spawn_blocking` on wasip1 runs the
closure *inline* on the only thread (`rt.rs`, `mod wasi`), and `rust/tcl-lsp-server-wasi/src/driver.rs`
only reads stdin / routes `exit` between task polls. Every analysis and the spec-pack load
therefore block the driver for their whole duration; nothing is lost, it is just late.

**Measured cost on this tree (native, debug test profile, `scratchpad/lsp/out-p4-zz_lsp_agent_probe.log`).**
- `Analyser::analyse("proc p {a} {return $a}\n" + "p 1\n"×n)`: n=10 3.1 s, 20 5.9 s, 40 17.7 s,
  80 66.0 s — quadratic; n=500 did not finish in 15 min. The e2e `exit`+stdin-close scenario
  opens exactly this shape with n=4000.
- 41 empty procs: 8.5 s per analysis.
- `tcl_spectcl::bundled::load_embedded()`: 84 s cold, 65 s warm (second call, cache redirected).

**Each failed check explained.**
- `exit` + stdin close ×12 timeouts: `didOpen busy.tcl` → inline analysis of 4000 calls; the
  driver never gets back to `route()` within 30 s.
- idle session, exit without shutdown → timeout: after the 10 s configuration deadline the
  server loads spec packs inline (`reload_spec_packs` → `spawn_blocking` →
  `load_discovered_configuration_with_studio_sources_and_inputs_in`), longer than the 30 s
  exit budget.
- main session, APP `publishDiagnostics` missing within 60 s, then BROKEN arrives 6 s later:
  APP's didOpen queues behind the post-`initialized` pack load + workspace scan; BROKEN is
  opened after they finish.
- go-to-definition / hover into `lib.tcl` empty: issued while the workspace scan / source
  rehoming is still pending behind the same work (no index row for `lib.tcl` yet).
- cache scenario: the cache directory is checked right after the `initialised` log line, i.e.
  before the (inline, slow) pack load wrote it; "analysis still answers" then fails exactly
  like the APP publish above.

**Owner.** Compiler (analysis cost superlinear in command count) and vm-runtime/tcl-spectcl
(pack load), routed to the coordinator; a dedicated perf agent was started. Server108
(below) only removes *repeated* analyses of an unchanged revision.

**Exact verification steps once disk allows (~2.5 GB free for the wasip1 target dir):**
1. `bash rust/tcl-lsp-server-wasi/build-wasi.sh` (or `CARGO_PROFILE_RELEASE_LTO=false` to save
   time/disk), then `node rust/tcl-lsp-server-wasi/test/e2e.mjs`.
2. Natively first, cheaper: time `Analyser::analyse` on the busy fixture for n=100/200 and
   `bundled::load_embedded()` in `--release`; if analysis is not ~linear the WASI checks
   cannot pass whatever the host does.
3. After the perf fix: rerun step 1; expect all 26 checks. If only the cache check still
   fails, wait for the cache directory with `until(...)` instead of checking once (test-side
   race, needs a decision since it changes the harness).
