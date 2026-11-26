# Call frames: one design or several

How call frames, levels and their namespaces are handled across execution (VM, runtime, codegen) and analysis (compiler, analyser, LSP, registry). It extends the frame findings of [variable lookup is not centralised](variable-lookup-centralisation.md) and feeds step 1 of [tidying the architecture](architecture-tidy-plan.md) ("one frame value").

**Verdict:** execution behaves correctly almost everywhere, but through two different frame models with copied selection code; analysis has many frame types, three interprocedural frame-effect models and no frame conformance. The level-word grammar is the only part that is mostly shared.

## Measured behaviour

The 24 fixtures in [`oo-dispatch-probe/frames-semantics/`](handover/oo-dispatch-probe/frames-semantics/) cover `uplevel` and `upvar` at every level form, `info level`, `namespace eval`, `namespace inscope`/`code`, `apply`, `tailcall`, coroutines, TclOO methods with `my`, `next`, `nextto` and chained constructors, `interp alias`, variable traces, `-errorstack`, `return -level`, invalid levels and level-word ambiguity. Each prints deterministic output. They were run under tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0, then under the VM (`tclvm --tcl-version`) and the Rust runtime (`run_script --quiet --tcl-version`) for 8.6 and 9.0, on `9b16e406`.

**Execution:** of the 18 general fixtures, every runtime run matches tclsh and 34 of 36 VM runs do (the two misses are the TclOO `next` case on both releases); six narrowing fixtures then isolate the VM's differences:

| Case | tclsh, runtime | VM |
| --- | --- | --- |
| superclass method reached by `next`: `info level`, and `uplevel 1` sees the original caller | `2`, yes | `3`, no (it sees the subclass method's frame) |
| same through `nextto`, and through a chained constructor | level `2` | level `3` |
| `filter` in an `oo::class create` definition script | accepted | `invalid command name "filter"` |

A plain method and `my` are correct in the VM. The runtime keeps a `next`-chained method at the original level with `push_same_level`; the VM's level is always its stack index (`rust/tcl-vm/src/interp/frames.rs`), so it has no equivalent. The VM also does not implement `info frame` (pinned as an error in `rust/tcl-vm-it/tests/it_2/cmd_info_prefix_e2e.rs`); the runtime does.

**Analysis** (the same correct programs; any diagnostic is false):

| Case | Report |
| --- | --- |
| `x` set in the caller by a callee's `uplevel 2` | W210 read before set |
| `x` set through `interp alias {} al {} setter` where `setter` does `upvar 1` | W210 |
| `o` set by `catch {…} r o` | W210 |
| `next`, `nextto`, `my` in entered methods | W123 unresolved |
| variable-trace callback formals | W214 unused |

## Representations

- **VM:** `CallFrame` (`rust/tcl-vm/src/frame.rs:49`; level is the stack index; a separate `ns_eval` marks `namespace eval` frames), the bytecode activation `exec::Frame`, `VarTableOwner`, `SelectedFrameRestore`, coroutine `ParkedFlow`, `OoFrame`.
- **Runtime:** `Frame`/`FrameStack` (`runtime/rust/src/frame.rs`; level is *logical*, with an `active_level` like C's `varFramePtr`, and `push_same_level`), `VarHome`, `CmdFrame` for `info frame`, `ProcFrame`/`MethodFrameWhat`.
- **Shared by both engines:** the `tcl_runtime_api::Frames` trait (push, pop, current, link, enumeration — no selection or `uplevel` operation), `FrameId`, the error-stack frame types.
- **Analysis:** `VariableExecutionFrame` (eight variants), `VariableFrameKind`, `ResolveContext`, `SourceOriginalVariableFrame`, `ResolvedFrameBody`, `BodyFrame`, `Scope`/`ScopeKind` (`Uplevel` covers only `#0`), `FrameTarget`/`FrameContext` (`cfg_builder/frame_effects.rs`), `ExecutionNamespace::selected_frame`, the IR's `UpFrame { frame_shift: i32, absolute: bool }`, `EntryProcedureFrame`, `ExistenceFrame`; in `tcl-lsp-core`, `CallerFrameBinding`, a bare span from `source_frame_extent` and its own `OoFrame`.
- **Registry:** `FrameLevel` (relative, absolute, dynamic), `FrameLevelWord`, `FrameArgLayout`, `FrameEffectSpec`, `NativeFrameLevelProtocol`, `VariableAliasFrame`, `CallerFrameSelection`, `DeferredBodyFrame`, `CapturedLifecycleSpec`, `DeclaredFrameEffect` (stub `-frame`), `EventVariableFrame`.
- **Name collisions:** two unrelated `NativeCompilationFrame` enums (`tcl-runtime-api` and `tcl-registry`), two `OoFrame`s, two `FrameKind`s, two `restore_execution_frame`s.

No frame or activation type is shared between execution and analysis.

## Level words

The intended single reader is `tcl_syntax::frame_level` (`parse_supplied_level`, `leading_word_is_level`), reached through the registry's `FrameLevel` and `NativeFrameLevelProtocol`. Outside it:

- `tcl_cmd_core::frame` (`frame_level`, `bad_level`) is a third `TclObjGetFrame`, with no callers (*read*).
- `upvar_compiles_level_word` (`tcl-registry/src/native_compilation/variable_and_list_grammars.rs`) has its own per-release table.
- The runtime's compiled-locals scan (`runtime/rust/src/compiled_locals.rs`) treats only all-digit words as levels, which differs from the registry table for words such as `+1` or `0x1` (inferred from the two tables).
- The VM's `select_execution_frame` formats its own `bad level` text outside `NativeFrameLevelFailure`.
- Analysis decodes levels with two fallback policies (`registry_invocation/normal_transfer.rs` returns nothing without a dialect; `analyser/handlers/scope_aliases.rs` falls back to `Dynamic`).
- `LiteralLevel` holds a `u32` where C reads an `int`; `uplevel_operands` re-rejects values above `i32::MAX` itself.
- The engines' selection wrappers (`tcl-vm/src/command/frame_level.rs`, `runtime/rust/src/cmd_eval.rs`) are line-for-line copies.

## Frame selection

Both engines implement the same contract by different mechanisms: the VM splits the frame stack above the target and restores it; the runtime moves `active_level` and switches namespace, in two duplicated functions (`eval_uplevel`, `eval_uplevel_obj`). The contract and the runtime frame model still cite `Vm::enter_level`, which no longer exists (*read*).

Analysis chooses the frame and namespace for a body in at least nine places: the source walk's native target, `ResolveContext::in_frame`/`for_function`, lowering's inline choice, `ExecutionNamespace::selected_frame`, `unit_scope/extra_scan_contexts.rs`, `FrameContext`, `BodyFrame`, `ScopeKind::Uplevel` and the LSP's `source_frame_extent`. Two disagree on `uplevel #0`: `selected_frame` gives an exact `::`, `extra_scan_contexts` asks the source binding and otherwise gives runtime-selected (*read*). They also disagree on what a `namespace eval` body is: `BodyFrame` calls it a fresh frame with formal parameters; the VM, the runtime and the resolver treat it as a namespace frame. A level is converted from `FrameLevel` to the IR's `(i32, bool)` and back again.

## Frame effects of a called procedure

Three independent models on the analysis side:

1. `cfg_builder` frame-effect summaries (`UpvarInfo`, `GlobalWriteInfo`), closed over call edges, transitive for level 2 and beyond, pinned to tclsh transcripts; `uplevel #N` with N > 0 is unknown.
2. `analyser/param_traits/caller_frame.rs`, which re-scans body text and handles only `upvar 1` or an omitted level; it feeds `ProcDef` caller-frame parameters and the LSP's caller-frame navigation, and sees no `uplevel` writes and no level-2 effects.
3. `var_escape` summaries (`frame_needed`, `upvar_source_names`).

The false W210s above follow: the `uplevel 2` and alias-then-`upvar` writes are invisible to the model W210 consults. Coroutines have no analysis model; `tailcall` is a control exit and a frame barrier; `next` keeping the level is modelled only by the runtime.

## Conformance

`tcl-syntax/tests/data/native_frame_levels/*.jsonl` (a C probe over 8.4–9.1 and Jim) drives only the VM. `native_frame_reference/*.tsv` drives the VM and the runtime. `variable_resolution_vectors.txt` drives only its own test. The analysis side has hand-pinned transcripts; no vector drives the analyser, the IR, codegen inlining or the LSP's frame choice.

## Plan

1. **Fix the measured bugs:** the VM runs `next`, `nextto` and chained constructors at the original level (same-level push, as the runtime does) and accepts `filter` in class definition scripts; add the fixtures above to the VM and runtime end-to-end suites. Implement `info frame` in the VM or record it as an explicit non-goal.
2. **One level reader:** route `upvar_compiles_level_word`, the runtime compiled-locals scan, the VM's error text and both analysis fallback policies through `tcl_syntax::frame_level` and `NativeFrameLevelFailure`; delete `tcl_cmd_core::frame`; make `LiteralLevel` C-`int` ranged.
3. **One selection contract for both engines:** add frame selection (`uplevel`, `upvar` target, same-level push, namespace switch) to `tcl_runtime_api::Frames`, implement it once per engine, and delete the copied wrappers and the runtime's duplicate `eval_uplevel` path. Whether to converge the two internal models (stack index versus logical level) is a separate decision; the trait makes it possible without changing callers.
4. **One analysis frame value** (the static `Frame` of [design integration](design-integration.md); the alias table below is per-point name-world state), shared with command and variable lookup ([architecture plan](architecture-tidy-plan.md) step 1): frame kind (global, namespace activation, procedure, method with receiver, coroutine, lambda), level relation to the caller including same-level for `next`, namespace, alias table. Keep `FrameLevel` through the IR instead of `(i32, bool)`; choose a body's frame once, in binding; delete `BodyFrame`'s parameter-frame view of `namespace eval` and the LSP's span heuristic.
5. **One frame-effect model:** keep the `cfg_builder` summaries as the owner, extend them to `uplevel` writes into the caller (the `uplevel 2` case), aliases that resolve to procedures, and `catch`'s result and options variables through the registry's effect descriptors; make the parameter-trait and escape summaries projections of it.
6. **Frame conformance for every consumer:** run the frame fixtures and the existing frame vectors through the analyser (W210, W211), the LSP (definition, references) and codegen inlining, as well as both engines.
7. **Housekeeping:** rename the colliding types, and update the contract and runtime model documents that cite `Vm::enter_level` and the non-existent `frame_depth_stash`.
