# Traces: what a trace does, and what analysis must assume

How variable, command and execution traces (`trace add|remove|info`, and 8.x's `trace variable|vdelete|vinfo`) behave in each engine, and how the compiler, analyser, optimiser and LSP model traced names. Method: 23 fixtures in [`oo-dispatch-probe/traces/`](handover/oo-dispatch-probe/traces/), one behaviour each, run under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, jimsh, the VM (`tclvm --tcl-version`) and the Rust runtime (`run_script --quiet --tcl-version`) for all five releases (`engines.tsv`, 368 rows); the same fixtures and about twenty inline probes through the MCP analyser, optimiser and navigation tools (`analyser-mcp.out`; a cached build, sources at `afe2f2d8`, engine binaries from 21 November with no engine commits since); then a read-only census of `rust/` and `runtime/rust`.

**Verdict:** the engines are almost exactly right (one VM 8.4 fault), and the registry already describes traces well (`StateTransition::Trace`, operation sets, callback arity). Analysis has no single meaning for "this access fires code": at least eight models at three granularities (per point, per function, per module, mostly keyed by name strings), and none of them treats a trace firing as **a call to the callback**. The optimiser declines correctly whenever the traced name itself is read, but **rewrites two programs wrongly** when a trace callback writes a *different* variable, and the analyser gives false W211/W220 on the legacy spelling, false W214 on every fixed-signature callback, and misses W210 whenever any trace is registered.

## Measured

**Engines.** All 23 fixtures agree between tclsh and the runtime on every release. The VM agrees on 113 of 115 runs (both misses are one fault):

| Fixture | tclsh (all) | VM | Runtime |
| --- | --- | --- | --- |
| `tr04`, `tr21`: read trace on a **global** raises, caught by `catch` | `1 {can't read "x": nope-read}` | 8.4 only: the error escapes `catch` and ends the script; a procedure-local traced variable is fine; write traces are fine | matches |
| Every other fixture (write/read/unset/array traces, value-changing traces, `set` returning the post-trace value, newest-first order, no re-entry, trace on a missing variable, callback level and `upvar 1`, namespace and `global` targets, command rename/delete traces, enter/leave/enterstep/leavestep, leave traces cannot change the result, built-in `string` traced) | | matches | matches |

Release differences, all reproduced by both engines: 9.0 removed `trace variable|vinfo|vdelete` (`bad option "variable": must be add, info, or remove`); 9.0 passes `a(k)` instead of `a` as `name1` when an element is unset under a whole-array unset trace (`tr05`); `info exists` fires read traces (`tr13`: `exists-before=1`). An enter trace that redefines the traced procedure makes the current call run the *new* body (`tr18`). Jim has no `trace` command at all.

**Analysis and tools** (`tcl8.6` unless stated):

| Case | Result | Correct? |
| --- | --- | --- |
| `tr22`: `trace add execution puts enter cb` where `cb` does `set ::x 5`; then `puts "after=$x"; puts "again=$x"` | optimiser (full) deletes `set x 99`, rewrites both reads to `99` | **wrong**: tclsh prints `again=5`, the rewrite prints `again=99` |
| `tr23`: `trace add variable w write cb2`, `cb2` does `set ::z 7`; `set w 1; puts "z=$z"` | optimiser rewrites to `puts "z=1"` | **wrong**: tclsh prints `z=7` |
| Traced name itself read: `tr02`, `tr03`, `tr14`, `tr15`; trace added by a helper via `uplevel`, by a procedure on `::x`, through `$name`, through `eval`, on an `upvar` alias, in legacy spelling | no rewrite | yes |
| `tr08` legacy `trace variable x w cb`, then `set x 2` | W211 and W220 on `x` (8.6 and 9.0) | false: the trace observes both writes; the optimiser does see this trace |
| Trace callbacks with fixed formals (`n1 n2 op`, `cmd op`, `cmd code res op`) | W214 on unused formals in 13 fixtures | false |
| `trace add variable y write cb; puts $y`, top level and in a procedure | no W210 | false negative: tclsh errors; any trace suppresses W210 whatever its operations |
| Navigation on callback words (`tr01`, `tr09`, `tr10`) | definition, references, rename of the callback and of the traced variable all correct; `trace info execution f` missed by references; hover says 2 references where there are 3 | mostly |

The LSP's default optimiser profile (Readability) does not propagate, so `tr23` is wrong only through `tcl opt`, MCP `optimize` and `tcl compile --optimise`.

## Census

**Engines.** Argument decoding is shared: `tcl-cmd-core/src/trace.rs` (op lists, legacy letters, `bad option`/`bad operation` messages). Everything else is written twice:

- VM: `cmd_trace.rs` (384 lines), `interp/traces.rs` (448), `interp/traces/command_traces.rs` (707), `interp/traces/variable_firing.rs` (788), `interp/traces/env_and_precision.rs` (364), `interp/variable_trace_outcomes.rs` (197), `interp/variables/traced_reads.rs` (731), `exec/dispatch/execution_traces.rs` (499).
- Runtime: `cmd_trace.rs` (1,055), `interp/variables/traces.rs` (905), `interp/dispatch/traces.rs` (703), `interp/native_trace_result.rs` (680), `env_trace.rs` (782), `vars/trace_homes.rs`, `vars/trace_shell.rs`, `frame/trace_shell.rs`, `interp/native_command_traces.rs`, `cmd_var/member_trace_horizon.rs`.
- Shared contracts: `tcl-runtime-api/src/native_variable_trace.rs` and `tcl-registry/src/native_variable_trace.rs` (native C callbacks such as `env` and `tcl_precision`); no engine trait for registering, firing or listing script traces.

About 5,500 VM lines and 5,000 runtime lines implement the same `tclTrace.c` and `tclVar.c` firing rules. They agree with tclsh everywhere measured except the VM 8.4 read-error path.

**Registry.** `commands/tcl/trace.rs` (1,465 lines) gives forms, callback appended arity and `Traits::ESTABLISHES_VARIABLE_TRACE`; `state_transition.rs:540–620` gives `TraceTarget` (Variable, Command, Execution), `TraceOperation` and `TraceTransition::{Add, Remove}` with the callback prefix. This is the right vocabulary and is complete for the fixtures.

**Analysis: eight models of "this access is observed".**

1. Whole-module name sets: `populate_trace_facts` (`lowering/module_assembly.rs:254`) fills `Module::traced_variables`, `traced_commands`, `has_dynamic_variable_trace`, `has_dynamic_trace` (`ir/module.rs:293–318`). Position-independent, keyed by `::`-stripped strings.
2. Flow-sensitive `EscapeFlag::TRACED` lattice: `var_observability.rs` (its header says it has no optimiser consumer yet).
3. Per-point `observed` on a resolved access: `var_resolve::resolve_literal_access(…, TraceOperation)` and `Place::observed`, used by binding (`command_binding/compiled_preflight.rs:375`), contents reads (`var_resolve/contents_reads.rs:152–330`) and the dead-store places (`dead_stores.rs:60`).
4. Value transfer: `is_traced` (`value_transfer/statements/outcome_defs.rs:452`) over model 1.
5. GVN purity: `is_pure_command_with_traces` (`gvn.rs:469`) over model 1's command half; `registry_invocation/execution_trace.rs` for keys.
6. Optimiser load forwarding: `variable_observation_names::variable_name_may_be_observed` over model 1 (`optimiser/manager.rs:687`).
7. Callback writes: `deferred_writes.rs`, which reads callback scripts, and follows a callback that is a procedure of the module only "in standalone mode" — the path `tr23` takes does not.
8. LSP refactors: four separate `ESTABLISHES_VARIABLE_TRACE` checks (`refactor/extract_proc.rs:318`, `extract_variable.rs:293`, `inline_variable.rs:851`, `source_rewrite.rs:167`); navigation reads callback words through `command_binding/command_observers.rs:949`.

They disagree: the analyser's W220 (`analyser/diagnostics/dataflow/dead_stores.rs`) misses the legacy `trace variable` that model 1 sees; W210 treats any registration as defining the variable, while the registry says a registration only vivifies an undefined cell; W214 ignores the callback arity the signature scan already computes (`signature_scan/command_prefix.rs`); and no model attaches the callback's own effects to the access or call that fires it, which is the cause of both wrong rewrites (`tr22`: an execution trace on `puts`; `tr23`: a write trace on another variable).

## Design

A trace is a **deferred body bound to an event**, and the event is an access or a call. That puts it exactly where the [design integration](design-integration.md) decisions already point:

- **Registry effects** (fault 9): `TraceTransition::Add|Remove` stays the one effect descriptor. Extend the callback descriptor with its frame (one level below the access, `info level` + 1, `uplevel 1` is the accessing frame — `tr12`) and its appended arity, which already exists, so W214 and frames read it instead of guessing.
- **Name world** (decision 2): trace registrations are name-world state per point, keyed by variable place and `CommandRef` identity, not by string. `trace add` adds a registration; `trace remove`, `unset` of the variable and deletion of the command remove it; an unknown target or operation list makes "every place may be traced" from that point, per name where possible, never per world.
- **Potential execution** (decision 8): the callback is a body unit with a deferred entry (decision 3). Its frame-effect summary (the `cfg_builder` owner of [call frames](frame-design.md)) is the effect of the trace. Then:
  - an access to a place that may carry a trace of that operation is a **call** to its callbacks: their summarised writes clobber, their unknowns are unknown;
  - a call to a command that may carry an execution trace is followed by the callbacks' effects (enter before, leave after; enterstep and leavestep apply to every command in the body);
  - a write trace may change the value written (`set` returns the post-trace value, `tr03`), so value analysis of a traced place yields "unknown" unless the callback summary proves otherwise.
- **`Answer<T>` and receipts** (decision 7): W210 asks "is this place defined here?". A registration answers "vivified but undefined"; only a read-trace callback that may write the place turns the answer into unknown. So W210 can keep firing for `trace add variable y write cb; puts $y`.
- **Whole-program facts:** "no trace can observe this place" is an absence proof, so it belongs to the potential-execution owner and needs its coverage (dynamic targets, `source`, packages). The module name set (model 1) becomes a projection of the per-point registrations joined over the deferred point, not an independent scan.
- **Engines:** add script-trace registration, removal, listing and firing order to `tcl_runtime_api` (beside `Frames`), with conformance rows generated from these fixtures; the two engines may keep their storage but answer one contract.

## Plan

- **Stop the two wrong rewrites now.** In `optimiser/propagation/load_forwarding.rs` and `store_to_load_forwarding.rs`, treat a call to any command in `Module::traced_commands` (or any call when `has_dynamic_trace`) as clobbering every name the module's trace callbacks may write, and an access to a traced variable as clobbering the same set; take that set from `deferred_writes` with procedure callbacks followed in every mode, not only standalone. Done when `tr22` and `tr23` produce no rewrite and their `.optimised.tcl` files are deleted; owner: optimiser.
- **Fix the VM 8.4 read-trace error path.** A read-trace error on a global inside `catch` must become `can't read "x": msg` as on 8.5+ (`rust/tcl-vm/src/interp/variables/traced_reads.rs` / `variable_trace_outcomes.rs`; cause not traced here). Done when `tr04` and `tr21` match tclsh 8.4 in `engines.tsv`; owner: VM.
- **One trace fact owner in binding.** Replace models 1–6 with per-point trace registrations in the name world (`command_binding`), keyed by place and `CommandRef`, built from `TraceTransition`; model 1 becomes its deferred-point projection, models 2–6 read it. Done when `var_observability`'s `TRACED` flag, `variable_observation_names` and `outcome_defs::is_traced` are deleted or are views of the owner, and the four refactor checks in `tcl-lsp-core/src/refactor` ask the owner.
- **Trace firing as a call.** Lower each callback as a deferred body unit and apply its `cfg_builder` frame-effect summary at every access or call that may fire it (enter, leave, enterstep, leavestep; read, write, unset, array). Done when `tr02`, `tr03`, `tr14`, `tr15`, `tr22` and `tr23` give the right optimiser answer from the summary, not from a blanket decline, and a trace whose callback provably writes nothing allows folds of other names; owner: interprocedural summaries.
- **W210 reads the registration's operations.** A registration vivifies; only a read-capable callback that may write the place makes it defined-unknown. Done when `trace add variable y write cb; puts $y` reports W210 and `tr13` still does not; owner: W210 (`analyser/diagnostics/dataflow/read_before_set.rs`).
- **W220/W211 see legacy traces.** Route the analyser's dead-store and unused-variable checks (`analyser/diagnostics/dataflow/dead_stores.rs`, `unused_bindings.rs`) through the same trace owner as the optimiser, so `trace variable x w cb` suppresses them. Done when `tr08` is silent bar W144 (8.6) and W002 (9.0).
- **W214 reads callback arity.** Excuse a formal that the registry's appended arity for the callback position supplies (`signature_scan/command_prefix.rs` already computes it). Done when every W214 in `traces/analyser-mcp.out` is gone; shared with the frame page's W214 item.
- **References include `trace info`.** `find_references` and rename must count `trace info execution f` and `trace info command f` as references to `f`. Done when `tr10` gives six references.
- **Engine trace contract.** Add script-trace registration, removal, `trace info` order and firing order to `tcl_runtime_api`, implemented by both engines, with the 23 fixtures turned into conformance rows for every release and for Jim (expected: `invalid command name "trace"`). Done when the rows run in both engines' suites; whether the engines share firing code is a separate decision.
- **Conformance for analysis.** Run the trace fixtures through W210, W211, W214, W220, the optimiser (execute before and after, as the constants study did) and navigation in the probe harness. Done when `analyser-mcp.out` is generated, not hand-written.

## Status on the current source

Confirmed on the current source: W214 on every trace callback and the `tr20` fold of a traced builtin (wrong output). Not reproduced: the `tr22`/`tr23` folds and the `tr08` W211/W220 (cached-build artefacts; kept as regression rows). Details: [audit summary](tcl-areas-audit.md) and [`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/). Plan items 153–199.
