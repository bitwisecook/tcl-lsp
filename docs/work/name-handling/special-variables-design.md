# Special and global variables

How the interpreter-provided globals (`env`, `argv`/`argc`/`argv0`, `tcl_platform`, `tcl_library`, `tcl_version`/`tcl_patchLevel`, `auto_path`, `auto_index`, `errorInfo`/`errorCode`, `tcl_interactive`, `tcl_precision`, `tcl_rcFileName`, `tcl_wordchars`/`tcl_nonwordchars`, `tcl_traceExec`/`tcl_traceCompile`, `tcl_pkgPath`, Tk's globals) come into existence in each release, and how the registry (`tcl-registry/src/special_vars.rs`), the engines, the analyser, the optimiser and the LSP model them. Method: 20 fixtures in [`oo-dispatch-probe/special-variables/`](handover/oo-dispatch-probe/special-variables/) run under tclsh 8.4.20–9.1.0, jimsh, the VM and the runtime for all five releases (`engines.tsv`, 304 rows); the fixtures and inline probes through the MCP analyser, optimiser, hover and completion (`analyser-mcp.out`; cached build, sources at `afe2f2d8`); a read-only census.

**Verdict:** the registry's roster is accurate where it speaks — every startup-existence and per-release claim measured matches tclsh — and W210, W211 and W220 use it well. The faults are in what does **not** read it: the **optimiser gives three wrong rewrites** (it forwards `errorInfo`/`errorCode` across a `catch`ed error, folds floats after `set tcl_precision`, and deletes a read-modify-write of `auto_path`), W210 does not know that an error writes `errorInfo`/`errorCode`, hover and completion ignore the selected release, and the engines publish a `tcl_platform` and startup roster that no release has. The facts exist in four rosters that are not derived from each other.

## Measured

**What tclsh does** (`sv01`–`sv05`, `sv10`): before user code a script run has `argc argv argv0 auto_path env tcl_interactive tcl_library tcl_patchLevel tcl_pkgPath tcl_platform tcl_rcFileName tcl_version`, plus `errorCode errorInfo tcl_libPath` in 8.4 only; `tcl_precision` is not listed but `info exists` and reads succeed through its read trace in 8.4–8.6 (default `12` in 8.4, `0` in 8.5–8.6; `0` is refused by 8.4) and it does not exist in 9.0–9.1. `auto_index`, `auto_execs`, `auto_noexec`, `auto_noload`, `tcl_wordchars`, `tcl_nonwordchars`, `tcl_trace*`, `tcl_prompt*` do not exist until something creates them (`tcl_wordBreakAfter` creates the word variables, `sv08`). `tcl_platform` keys: 8.4 has 7; 8.5 adds `engine`, `pointerSize`; 8.6 adds `pathSeparator` (and `threaded` on threaded builds); 9.0 drops `threaded`. errorCode for a failed variable read is `NONE` in 8.4–8.5 and `TCL LOOKUP VARNAME name` (top level) or `TCL READ VARNAME` (compiled procedure) in 8.6+ (`sv16`). In 9.0–9.1 an unqualified `$tcl_version` inside `namespace eval` fails; 8.x falls back to the global (`sv10`). Jim has `argc argv argv0 auto_path env tcl_interactive tcl_platform` and no `tcl_version`, `errorInfo` or `trace`.

**Engines** (divergences from tclsh of the same release):

| Fixture | VM | Runtime |
| --- | --- | --- |
| `sv01`–`sv03` startup roster | no `tcl_interactive` or `tcl_rcFileName` on any release; no `tcl_libPath` in 8.4 | `run_script` has no `argc argv argv0 auto_path tcl_library tcl_interactive tcl_rcFileName` (and no 8.4 `errorInfo errorCode tcl_libPath`): the runner never runs the startup and `Tcl_Init` step |
| `sv04` `tcl_platform` keys | adds `ebpf runtime runtimeVersion wasi wasiVersion wasm` on every release; gives 8.4 the 8.5/8.6 keys and 9.x `threaded` | same as the VM |
| `sv06`, `sv16` errorCode after a failed top-level read, 8.6+ | matches | **wrong**: `TCL READ VARNAME` or `NONE` instead of `TCL LOOKUP VARNAME nosuch` |
| `sv05` `tcl_precision`, `sv07` `env` (VM), `sv09` procedure shadowing, `sv11` unsetting `tcl_platform`/`env`, `sv13`, `sv14`, `sv17`, `sv18` | match | match where the runner allows (`exec` is unsupported in the runtime build) |
| `sv08`, `sv15` auto-loading and `package require` | match, but take 9–17 s (the 20 s timeout fired three times) | `tcl_wordBreakAfter` missing (no library) |

The VM's `auto_path` ends in a cwd-relative `./lib` where tclsh has `<prefix>/lib /usr/local/lib`. Outside this area, `package require` of any package not already loaded fails in the VM under 8.5 with `invalid command name "add"` from `tm.tcl`'s `path add` (`sv19`).

**Analysis and tools** (`tcl8.6` unless stated):

| Case | Result | Correct? |
| --- | --- | --- |
| `sv17`: `set errorInfo seeded; catch {error x}; puts [lindex [split $errorInfo \n] 0]`, same for `errorCode` | optimiser rewrites both reads to `seeded` | **wrong**: tclsh prints `x`, `NONE` |
| `sv18`: `set tcl_precision 3; puts [expr {1/3.0}]` | O109 deletes the write, O101 folds to `0.3333333333333333` | **wrong** on 8.4–8.6: tclsh prints `0.333` |
| `sv19`: `set auto_path [linsert $auto_path 0 sv19-pkg]` then `package require svprobe` | O109 deletes the `auto_path` write | **wrong**: the package is then not found; a plain `set auto_path /x` is kept, and W220 is silent (it reads the registry) |
| `catch {error boom}; puts $errorCode; puts $errorInfo` | W210 on both | false: the error writes both |
| `sv12` write-only `auto_path`, `env`, `tcl_interactive`, `errorInfo`, `argv` | no W211/W220 | yes |
| `sv09` `$tcl_version` in a procedure without `global` | W210 | yes |
| `sv10` under `tcl9.0` | no W210 | false negative: 9.x has no global fallback in `namespace eval` |
| `"…[info exists tcl_wordchars]…"` (`sv05` 9.0, `sv06`, `sv08`, `sv15`) | W210 | false: `info exists` nested in a quoted word is counted as a read (also for ordinary names) |
| hover `$tcl_precision` under `tcl9.0`; `$tcl_platform(…)` under `tcl8.4` | describes `tcl_precision`; lists `pointerSize`, `engine`, `pathSeparator` | wrong: hover uses the authoring surface, W210 the release |
| completion `$`, `$tcl_`, `$tcl_platform(` | no special variables or keys | gap |
| hover `$tk_version` | nothing | gap: no Tk rows |
| optimiser `info exists tcl_platform`, `tcl_precision`, `errorInfo` | not folded | yes (the host may differ) |

## Census

**Four rosters of the same facts.**

1. `SPECIAL_VARS` (`tcl-registry/src/special_vars.rs:555`): 27 rows with surface, startup binding, read trace, write effect, taint, keys (`TCL_PLATFORM_KEYS`, `:479`). Read through free functions (`:381`–`:474`) and through `Registry` faces that include pack rows (`registry/lookup.rs:195–268`). Analysis uses both: `analyser/diagnostics/dataflow/dead_stores.rs:113,482` and `unused_bindings.rs:231`, and `sccp/original_value_facts/existence_branches.rs:706` use the pack-blind free functions; `dataflow.rs:158`, `helpers/phi_undef.rs:182`, `sccp/solver/existence.rs:645,680`, `value_transfer/existence.rs:256`, `taint/propagation.rs:455`, `analyser/handlers/code_loading.rs:483` and the LSP use the registry faces.
2. `NativeBootstrapVariable` and `NativeBootstrapProtocol::allocations` (`special_vars.rs:1387,1640`): the engines' constructor and main-argument roster, checked against `tests/data/native_bootstrap/core-roots.tsv`. It names `tclDefaultLibrary`, which roster 1 lacks, and its `MainArguments` plan (`argv0 argc argv tcl_interactive`) is used by neither command-line host.
3. `tcl-platform/src/lib.rs:690` `ENTRIES`: one release-blind `tcl_platform` schema for both engines, including six backend keys for the test-suite overlay. Roster 1's per-release keys are not consulted.
4. Hand-written hosts: `tcl-vm-cli/src/main.rs:285` `set_argv` (no `tcl_interactive`, no `tcl_rcFileName`); `runtime/rust/src/interp/setup/bootstrap.rs:72` `set_startup_globals` (not reached by `run_script`); nobody produces `tcl_rcFileName`.

**Engine callbacks written twice:** `env` (VM `interp/traces/env_and_precision.rs`, 364 lines; runtime `env_trace.rs`, 782), `tcl_precision` (the same VM file; runtime `interp/native_precision.rs`, 441), `errorInfo`/`errorCode` (VM `interp/native_error_variables.rs`, 1,939; runtime the same name, 1,545), bootstrap (VM `interp/bootstrap.rs`, `native_bootstrap.rs`; runtime `interp/setup/bootstrap.rs`). Each reads roster 2 for its allocation order; the runtime's error-code fault is in its own copy.

**Analysis rules beside the roster:**

- **Startup existence** is computed in five places (`dataflow.rs:150–162`, `phi_undef.rs:175–186`, `sccp/solver/existence.rs:645`, `value_transfer/existence.rs:256`, `existence_branches.rs:706`), the first two with the same three-field logic.
- **Error storage:** `native_error_storage` (`special_vars.rs:82`) is consumed only by binding (`command_binding/compiled_preflight.rs:362`, which marks both cells "defined, unknown contents" on an error edge); SSA, W210 and the optimiser never see that edge, hence `sv17` and the W210s.
- **Precision:** `command_binding/mutation_summary.rs:512` `module_may_set_precision` searches the source bytes for the variable name; the fact reaches value transfer only when `folds` is present (`value_transfer.rs:628` defaults to "unwritten"), and the O101 path in `sv18` folds regardless (*inference* from the measured fold).
- **Write effects:** the optimiser's dead-store finder (`dead_stores.rs`, `optimiser/manager.rs:964`) never consults `is_externally_read` or `special_var_write_effect`; the analyser's W220 does. Two dead-store owners, two answers (`sv19`).
- **Hover** (`tcl-lsp-core/src/hover/variables.rs:57,246`) and `variable_symbol.rs:468,538` ask with `authoring_query()`, the union of releases; W210 asks with the selected release. Completion has no special-variable source.
- **Names written elsewhere:** `auto_path`/`auto_index` in `command_binding/original_namespace_binding.rs:54`, `package_resolver/original_names.rs`, `auto_path_eval/`; `tcl_version`/`tcl_patchLevel` in `native_info_version.rs`; `::env` in `model/binding.rs:85`. These are package and info protocols, not rival rosters, but none is derived from roster 1.

## Design

- **One roster, generated and checked.** Roster 1 is the semantic owner; roster 2 (birth order), the `tcl_platform` key schema (3) and the host producers (4) become projections of it, with per-release key surfaces and a separate, declared list of backend-only keys that the engines add only when a test overlay asks for them. Every row is checked against the tclsh and Jim oracles (`sv01`, `sv02`, `sv04` already give the data); Jim and Tk (`tk_version`, `tk_patchLevel`, `tk_library`, `tk_strictMotif`, `tk::Priv`) get rows.
- **Special variables are places with effects** (fault 9). The registry rows are effect descriptors: a startup binding is an entry fact of the global frame (decision 2), a native read trace is a registration in the name world like a script trace (see [traces](traces-design.md)), an `externally_read` write is a write with an observer, and `native_error_storage` is an effect of every abrupt error edge. Binding already applies the last; SSA, W210 and the optimiser must take it from the annotated command instead of re-deciding.
- **`Answer<T>` with receipts** (decision 7): "is `errorCode` defined after this `catch`?" is proven-defined with unknown contents; "is `tk_version` defined?" is unknown until `package require Tk`; "is `tcl_precision` defined in 9.0?" is absent. Display features may use the authoring surface, but must say so; release-bound answers (W210, hover of existence, completion) use the selected release.
- **Whole-program facts** (decision 8): "no write to `tcl_precision`" and "no write to `auto_path` that a later `package require` could see" are absence proofs over every reachable body; the byte search is replaced by the potential-execution owner, and its absence must fail closed.
- **Engines:** host start-up (`Tcl_Main` arguments, `tcl_interactive`, `tcl_rcFileName`, `Tcl_Init`) is one shared step driven by roster 2's `MainArguments` plan, used by `tclvm`, `run_script` and the embedders; the error, `env` and precision callbacks follow one contract in `tcl-runtime-api` with the fixtures as conformance rows.

## Plan

- **Stop the three wrong rewrites now.** In the optimiser (`dead_stores.rs`, `optimiser/manager.rs`, `optimiser/propagation/`): never forward or delete a store to a name for which `special_var_write_effect` or `is_externally_read` holds in the selected release (via the registry face); kill `errorInfo`/`errorCode` values at every edge where a command may raise, using `native_error_storage`; make O101's double rendering read the module's precision fact and fail closed when it is missing. Done when `sv17`, `sv18` and `sv19` produce no rewrite and their `.optimised.tcl` files are deleted; owner: optimiser.
- **W210 sees the error edge.** Thread binding's `materialise_error_storage` result (`command_binding/compiled_preflight.rs:362`) into SSA as a definition of both globals on the error edge of a `catch`/`try` body. Done when `catch {error boom}; puts $errorCode` and `sv16` line 2 are silent; owner: SSA and W210.
- **Fix the runtime error code.** `runtime/rust/src/interp/native_error_variables.rs` (or the top-level read path) must set `TCL LOOKUP VARNAME name` for an uncompiled failed read in 8.6+. Done when `sv06` and `sv16` match tclsh in `engines.tsv`; owner: runtime.
- **One host start-up for the engines.** Drive `tcl-vm-cli` and `run_script` from `NativeBootstrapProtocol::allocations(MainArguments)` plus a shared `Tcl_Init` and app-init step that sets `tcl_interactive` and `tcl_rcFileName`; delete `set_argv` and fold `set_startup_globals` into it. Done when `sv01`–`sv03` and `sv12`, `sv13` match tclsh on every release in both engines (bar host-path values); owners: VM CLI, runtime.
- **Release-gated `tcl_platform`.** Make `tcl-platform`'s schema take the release and project the registry's `TCL_PLATFORM_KEYS` surfaces; publish the six backend keys only when the test overlay asks. Done when `sv04` matches tclsh on all five releases in both engines; owner: `tcl-platform`.
- **One startup-existence query.** Replace the five startup computations with one registry face `startup_existence(name, release) -> Answer<…>` and delete the pack-blind free-function calls in `dead_stores.rs`, `unused_bindings.rs` and `existence_branches.rs`. Done when a gate forbids `special_vars::is_*` outside `tcl-registry` and the fixtures give unchanged diagnostics.
- **Hover and completion use the selected release.** `hover/variables.rs` and `variable_symbol.rs` ask with the release query; completion offers the special variables and `tcl_platform` keys of the release. Done when hover of `$tcl_precision` under 9.0 is empty, `$tcl_platform(` under 8.4 lists seven keys, and `$tcl_` completes; owner: LSP core.
- **Roster completeness.** Add `tclDefaultLibrary` (8.4, platform-conditional), Jim's roster (`jim::argv0`, `jim::exe`, no `tcl_version`) and a Tk roster available after `package require Tk`, with oracle rows from tclsh and wish for 8.4–9.1. Done when `sv01`/`sv02` rows are generated from the registry and compared with each oracle.
- **9.x namespace resolution.** Teach variable lookup that 9.0+ has no global fallback for unqualified names in `namespace eval` (shared with the [namespace page](namespace-design.md)). Done when `sv10` reports W210 under `tcl9.0` and not under `tcl8.6`.
- **`info exists` is not a read.** Fix the analyser so `info exists` inside a quoted word is an existence test, as it already is at top level. Done when every W210 marked false in `special-variables/analyser-mcp.out` is gone.
- **VM package scan time.** Profile `package require` of an unknown package in the VM (9–17 s in a debug build against the source tree's `auto_path`), and fix the 8.5 `tm.tcl` `path add` failure with the package owner. Done when `sv08`, `sv15` and `sv19` finish within the 20 s budget and `sv19` passes under 8.5.

## Status on the current source

Item 153 keeps the error-variable reads in `sv14` and `sv17` live across
the caught errors. W210 on `errorCode` after `catch` (`sv16`) and the missed
9.0 `namespace eval` read (`sv10`) remain open. The `sv18` and `sv19`
rewrites did not reproduce in the current-source audit.
[Execution audit](handover/oo-dispatch-probe/tcl-areas-head/README.md)
