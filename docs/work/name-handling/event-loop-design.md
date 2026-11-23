# The event loop and deferred code

How `after`, `vwait`, `update`, `fileevent`/`chan event`, `bgerror` and `interp bgerror` behave in the engines, and how analysis treats code that runs later: `after` scripts, channel handlers, background-error handlers and Tk `-command` and `bind` scripts. It tests decision 3 of [design integration](design-integration.md) ("deferred bodies read a deferred point") and the potential-execution owner of decision 8 against the code.

**Question and method:** do the engines order and frame deferred scripts as C Tcl does, and which analysis consumers see what deferred scripts read and write? Seventeen short, deterministic fixtures in [`oo-dispatch-probe/event-loop/`](handover/oo-dispatch-probe/event-loop/) cover:

- timer and idle ordering;
- the frame and namespace an `after` script runs in;
- `after info` and `after cancel`;
- `update` against `update idletasks`;
- `bgerror` and `interp bgerror` handlers;
- late definition of a callback command;
- variables read by braced and list-quoted callbacks;
- `fileevent` and `chan event`;
- the name `vwait` waits on inside `namespace eval`;
- idle re-entry, `vwait` servicing idle callbacks, and timers that are not yet due;
- a Tk `-command`/`bind` script.

Each fixture waits with `vwait` on a flag with a timeout fallback. They ran under tclsh 8.4.20–9.1.0, Jim, the VM and the Rust runtime for 8.6 and 9.0 on `afe2f2d8` ([`engines.tsv`](handover/oo-dispatch-probe/event-loop/engines.tsv)); the Tk case ran under Tk 9.0.3 through `tk/run_tk.py`. The analyser, references, definition, rename and the optimiser ran through the MCP tools, a cached build that may lag `afe2f2d8` slightly ([`analyser-mcp.out`](handover/oo-dispatch-probe/event-loop/analyser-mcp.out)).

**Verdict:** the engines are **right**. They share one event queue (`tcl_cmd_core::event`), and every semantic fixture matches tclsh. The two VM differences are the debug build's slow event turn, not semantics, and channel events are unavailable in both engines. Analysis **already owns the fact decision 3 needs**: `DeferredWrites` records the names every callback script writes and reads. SCCP and the optimiser's dead-store pass honour it, but **W210 and W220 do not**, so a global set by an `after` script and read after `vwait` or `update` is a false "read before it is set" (9 such reports in 5 fixtures) and a variable read only by a callback is a false "never read". The converse, a braced callback that reads a procedure local that does not exist at global level, is not reported. Which commands defer a script is stated four different ways in the registry, and only `after` (and `package ifneeded`) gets a walked future body in binding. Navigation and rename through callbacks are correct.

## Measured

**Engines** (tclsh 8.4–9.1 agree except where noted):

| Fixture | tclsh | VM | Runtime | Jim |
| --- | --- | --- | --- | --- |
| ev01 timers and idle: `after 200`, `after idle`, two `after 0` | `t0a t0b idle t200` | same | same | `idle t0a t0b t200` (idle is a zero timer) |
| ev02 `after` script frame from a proc and from `namespace eval` | level 0, `::`, no locals | same | same | same |
| ev03 `after info`, cancel by id and by script text, multi-word script | as C | same | same | ids start at `after#1` |
| ev04 `update idletasks` runs idle only; `update` runs due timers only | `1 0`, `1 0` | same | same | `update idletasks` also runs timers |
| ev05 `bgerror` procedure | `boom 1 ::` | same | same | same |
| ev06 `interp bgerror {} ns::h` (8.5+) | `boom2 1 1 ::ns` | same | same | no `interp bgerror` |
| ev07 callback names a procedure defined after registration, then redefined | `first second` | same | same | same |
| ev08 list-quoted callback captures; braced callback reads a global set later and fails on a local | as C | same | same | same |
| ev09 `fileevent` on `open \|echo` | `hello 1` | **pipelines not supported** | **couldn't open "\|echo hello"** | `hello 1` |
| ev09b `chan event` on `chan pipe` (8.6+) | `hello 1` | **no `chan` command at all** | **"chan pipe is not supported under the WASM runtime"** | no `chan` |
| ev10 `vwait v` inside `namespace eval ns` | waits on `::v` (`global woke ns`) | same | same | same |
| ev11 idle callback registering another idle | one `update idletasks` runs both | same | same | same |
| ev12 braced `after` script reading a proc local | `can't read "local"` through `bgerror` | same | same | same |
| ev13 Tk `-command` invoked from an `after` script | `.b 1` (Tk 9.0.3) | — | — | — |
| ev14 `vwait` services idle callbacks | `idle-ran` | same | same | same |
| ev15 `update` does not wait for a timer not yet due | `0 1` | same | same | same |
| ev16 a 20 ms timer during `update` | `a0` | **`a0 late`** | `a0` | `a0` |

ev16 is kept to document a cost, not a bug: the debug VM takes about 35 ms for `after 0 …; update` (measured with `clock milliseconds`: tclsh 0 ms, runtime 2 ms, VM 36 ms), so a 20 ms timer is already due. The first versions of ev01 (10 ms) and ev04 (20 ms) failed in the VM for the same reason and pass with wider delays. The VM's loop is the same as the runtime's (below).

**Analysis** (any diagnostic on these correct programs is false unless marked):

| Fixture | Report | Correct answer |
| --- | --- | --- |
| ev02 `after 0 {set ::r1 …}`; `vwait done`; `puts $r1` (and `r2`) | **W210 ×2** | nothing |
| ev03 `after 0 set ::joined ok`; `vwait`; `$joined` | **W210** | nothing |
| ev08 `$cap`, `$err`, `$msg`, `$seen` after `vwait` | **W210 ×4** | nothing |
| ev08 `set g late-global` read only by `after 0 {set ::seen $g}` | **W220 never read** | nothing |
| ev10 `$order` after `vwait stop` | **W210** | nothing |
| ev11 `$o` after `update idletasks` | **W210** | nothing |
| ev05, ev06, ev09, ev13: the same pattern, but the callback names a **procedure** that writes the global | nothing | nothing (correct) |
| the `vwait` variable itself (`done`, `flag`, `stop`) | nothing | correct: `vwait`'s `VarWrite` role |
| ev12 `proc arm {} {set local 5; after 0 {puts $local}}` | W220 on `set local` only | W220 is right; the callback's read of a global `local` nobody sets is a **missed** W210-class report |
| ev07 `after 0 late` before `proc late` | nothing | correct: the callback runs after the definition |

Five further W210s in ev03, ev04 and ev15 are on `info exists` two substitutions deep (`puts [list [info exists x] …]`). They are not specific to the event loop: `puts [list [info exists zz] 1]` alone is W210, and `puts [info exists zz]` is not (recorded in `analyser-mcp.out`).

**Navigation:**

- References to `late` include `after 0 late`, `after 0 {late}`, `after 0 [list late]` and `interp bgerror {} late`, and rename edits all of them.
- References to `onread` include `fileevent … [list onread $f]`.
- Definition from a `-command {handler .b}` script and from `bind .b <Button-1> {handler %W}` reaches the procedure.
- The optimiser keeps `set g late-global` in ev08, because its dead-store pass reads the callbacks' reads.

## Census

**Engines:**

- Shared queue: `tcl_cmd_core::event::EventQueue` (`rust/tcl-cmd-core/src/event.rs`; `begin_turn` `:155`, `pop_turn` `:171`) owns timer and idle order, turn generations and cancellation for both engines. This is the one shared kernel, and it is right.
- Two copies of the driver: `rust/tcl-vm/src/cmd_event.rs` (667 lines) and `runtime/rust/src/cmd_event.rs` (636 lines) each implement `after`, `vwait`, `update`, `service_ready`, `process_one` and background-error reporting. `service_ready` and `process_one` are line-for-line the same (VM `:492-516`, runtime `:521-548`), including the 50 ms sleep cap.
- Release differences come from the registry's `NativeEventProtocol` (`rust/tcl-registry/src/native_event.rs:37`: Jim's idle-as-timer, `update idletasks` running timers, global-value `vwait`), which both drivers read.
- A third `after` for iRules TMM timers is in `rust/tcl-vm/src/retained_activation.rs:251`.
- `bgerror` and `interp bgerror` state is kept separately in each engine (VM `interp.rs`, `command/child_interpreters.rs`; runtime `interp/errors.rs`, `interp/child_interpreters.rs`).
- The VM registers no `chan` and no `fileevent` command; the runtime has them but refuses pipes and `chan pipe`.

**Registry — "this script runs later" is stated four ways:**

1. `body_execution: DeferredGlobalScript` on `after ms` and `after idle` (`commands/tcl/after_.rs:102`, `:169`) and `DeferredGlobalScriptForFamily(Tcl)` on `package ifneeded`. `BodyExecutionSpec::deferred_entry_frame` (`body_execution.rs:820`) turns it into `DeferredBodyFrame::Global`.
2. `Traits::DEFERS_BODY` on `after`, `fileevent`, `chan event` (`chan_.rs`), `bind` (`tk/bind.rs:59`), `trace` (`trace.rs:1184`), `timer`, Tk `text`, `canvas`, `ttk::treeview`, and tcllib and snit callbacks. `script_timing` turns it into `ScriptTiming::Deferred` (`registry/script_timing.rs:310`), and so does `script_body_flow`.
3. `body_kind: BodyKind::Structural` on `after` and `fileevent` (`fileevent.rs`) and `chan event`, documented as "SSA must not treat the script as sharing the caller's frame".
4. `callback_taint_inputs` (`script_timing.rs:394`) for taint.

Only (1) states the frame, and only `after` and `package ifneeded` carry it, although `fileevent`, `chan event`, `bind`, `-command` and `interp bgerror` handlers also run at global level (ev09, ev13). `commands/tcl/fileevent_.rs` is a second, older `fileevent` spec that `commands/tcl/mod.rs:99` (`mod fileevent;`) does not include: it is dead. `vwait` gives its operand `ArgRole::VarWrite` (`vwait.rs:276`); `update` is `CREATES_DYNAMIC_BARRIER` (`update.rs:109`).

**Analysis and LSP:**

- **Binding walks only (1).** `register_future_body` (`command_binding/future_bodies.rs:176`) creates a `DeferredSourceBody` with a global `future_frame`, and `walk_future_body` (`:249`) walks it from a cloned incoming world with contents cleared. The walk runs "after the document has loaded" (`deferred_entries.rs`, the comment before `walk_deferred_entry`), which is decision 3's end-of-script world in practice (*inference*: the incoming world is the completed root state; not traced through every caller). Scripts deferred by (2) alone, such as `fileevent`, `bind` and trace callbacks, get no future body.
- **`DeferredWrites`** (`tcl-compiler/src/deferred_writes.rs`, `scan_module` `:90`; type at `ir/module.rs:60`). It is whole-module and position-independent, built from `callback_script_indices`, so it covers (2) as well. It holds the names callbacks write (`names`, `any`) and read (`reads`, `reads_any`), and follows a callback that names a module procedure through `cfg_builder::global_write_info`.
  - Readers: SCCP (`sccp/solver.rs:115`, `sccp/original_value_facts.rs:378`), the optimiser's dead-store elimination (`optimiser/elimination.rs:1259`, `may_read`), loop termination (`analyser/bounds_checks.rs`), taint, the value-transfer context key, interprocedural exit values and the dataflow graph.
  - **Not read by** W210 (`analyser/diagnostics/dataflow/read_before_set.rs:424`) or W220 (`analyser/diagnostics/dataflow/dead_stores.rs`). That is the measured fault. Procedure callbacks escape it, presumably because W210 already reads the interprocedural global-write summary while inline scripts are only in `DeferredWrites` (*inference*).
- Other deferred-timing readers:
  - `interprocedural/eager.rs:515`;
  - `command_binding/source_transition_advice/logical_navigation.rs:738`;
  - `command_binding/source_declared_command/logical_body.rs:170`;
  - `registry_invocation/source_structure/source_load/frontier.rs:283`;
  - `analyser/handlers/classes/control_arms.rs:261`;
  - `tcl-lsp-core/src/executable_regions.rs:676`;
  - the LSP source graph's `ScriptScheduling::Deferred` (`source_graph/sequence_capture.rs`).

  So about eight consumers ask "is this deferred?" through two registry queries (`script_timing`, `script_body_flow`). One consumer asks for the frame (`deferred_entry_frame`).
- Nothing in analysis or the LSP names `after`, `vwait`, `update` or `bind` in production code. The registry rule holds.

## Design

- **One registry descriptor for deferral** (decision 6, registry layer). Fold the four statements into the `body_execution` descriptor: `DeferredGlobalScript` (with family) becomes the single fact, carrying the entry frame (`Global`, or `TraceFrame` for traces, which run in the frame of whatever fires them) and the trigger (timer, idle, channel, error handler, Tk event, trace). `DEFERS_BODY`, `BodyKind::Structural` and `ScriptTiming::Deferred` are derived from it. `fileevent`, `chan event`, `bind`, widget `-command`, `interp bgerror` and `bgerror` gain it.
- **Deferred point** (decision 3). Every deferred body is a body unit entered at the module's *deferred point*: the join of the worlds after registration. The cheap and sound version is the end-of-module world joined with Unknown for whatever unproven calls could change. Binding's future-body walk is already that for `after`; it should be the one path for every deferred descriptor.
- **Effects at the wait points.** `vwait`, `update`, `update idletasks` and a bare return to the event loop are where deferred bodies run. Their effect descriptor should be "runs pending deferred bodies": `DeferredWrites` as a call effect at that point, written to the deferred names, rather than a whole-module "externally mutable" flag. After `vwait done` or `update`, a variable written by any registered callback is maybe-set, so there is no W210; before the first wait point it is unaffected, so W210 stays accurate there. Rule 7's answer type expresses it: `Answer::Unknown(reason: deferred write)` before the wait, a bounded set after it.
- **Reads by deferred bodies are uses.** The reverse also holds: a store read by a callback is live (W220), and a callback read of a global nothing writes is a W210-class report in the deferred body unit (ev12).
- **Potential-execution owner** (decision 8). Deferred bodies are *possibly reached*, never *reached* at a program point. They count for absence proofs (W123 completeness, rename completeness) and never for straight-line facts.
- **Engines.** One driver behind an engine trait (the frame page's selection-contract pattern): `service_ready`, `process_one`, `vwait`'s wait loop and background-error dispatch move into `tcl-cmd-core` or `tcl-runtime-api` over a small host interface (evaluate script in the global frame, set and observe a variable, report a background error, sleep). The queue is already shared.
- **Conformance.** The fixtures become vectors for both engines (with delays wide enough for debug builds) and for W210, W220, references and rename.

## Plan

- **W210 and W220 read the deferred facts.** Owners: `rust/tcl-compiler/src/analyser/diagnostics/dataflow/read_before_set.rs` and `dead_stores.rs`, reading `crate::ir::DeferredWrites`. A wait point (`vwait`, `update`, registry effect) makes every name in `DeferredWrites::names` maybe-set (all names when `any`), and `DeferredWrites::may_read` keeps a store live for W220. Done when ev02, ev03 (`joined`), ev08, ev10 and ev11 report no W210, ev08 reports no W220, and `set x 1; after 0 {set ::y 2}; puts $y` without a wait still reports W210.
- **Deferred reads of missing names.** Owner: binding's future-body walk (`command_binding/future_bodies.rs`) and W210. Analyse each deferred body unit at the deferred point, and report a read of a global that nothing at module level or in any callback writes. Done when ev12 reports the `puts $local` read and ev07 and ev08 stay clean.
- **One deferral descriptor.** Owner: `rust/tcl-registry/src/body_execution.rs` with the specs `after_.rs`, `fileevent.rs`, `chan_.rs`, `tk/bind.rs`, `trace.rs`, `interp.rs` (`bgerror`) and `bgerror.rs`. Make `BodyExecutionSpec::DeferredGlobalScript` (with entry frame and trigger) the only stated fact, and derive `DEFERS_BODY`, `BodyKind::Structural`, `script_timing` and `script_body_flow` from it. Done when every spec with `DEFERS_BODY` states a `body_execution`, a registry test asserts the derivation, and `future_bodies.rs` walks `fileevent`, `bind` and trace callbacks too.
- **Delete the dead `fileevent` spec.** Owner: `rust/tcl-registry/src/commands/tcl/fileevent_.rs`. Remove the file (not in `commands/tcl/mod.rs`). Done when it is gone and the registry builds and its tests pass.
- **One event-loop driver.** Owners: `rust/tcl-vm/src/cmd_event.rs`, `runtime/rust/src/cmd_event.rs`, `rust/tcl-cmd-core/src/event.rs`. Move `service_ready`, `process_one`, the `vwait` loop and background-error dispatch into the shared crate behind a host trait. Done when both engines call it, both `cmd_event.rs` files shrink to registration and host glue, and every event-loop fixture passes on both.
- **Channel events in the VM, or a recorded non-goal.** Owners: `rust/tcl-vm` (no `chan`, `fileevent`) and `runtime/rust` (no pipes or `chan pipe` on the native host). Implement `chan` and `fileevent` in the VM, or record their absence in [backend constraints](../../design/runtime/backend-constraints.md) with the reason. Done when ev09 and ev09b pass in the VM, or the constraint is documented and the fixtures are marked expected-unsupported in the vector runner.
- **Debug VM event-turn cost.** Owner: `rust/tcl-vm/src/cmd_event.rs` and `eval_original_event` (`interp/native_event_context.rs:28`). Profile why one `after 0` plus `update` costs about 35 ms in the debug VM against 2 ms in the debug runtime. Done when ev16 matches tclsh in the debug VM, or the cost is explained and accepted in the VM's performance notes.
- **Nested `info exists` is not a read.** Owner: W210's read collection (`read_before_set.rs`) and the nested-substitution walker it uses. Done when `puts [list [info exists zz] 1]` reports nothing (ev03, ev04, ev15 lose their `info exists` W210s).
- **Conformance vectors.** Owner: engine end-to-end suites and an analyser vector runner. Turn the 17 fixtures and `engines.tsv` into vectors for both engines (8.6 and 9.0) and for W210, W220, references and rename, including the Tk row through `tk/run_tk.py`. Done when a regression in any measured row fails a test.

## Status on the current source

Not reproduced on the current source: the W210s and W220 on globals written by `after` scripts (`ev02`, `ev08` and the rest); they stay as regression rows. `ev12`'s missed read stands. Details: [audit summary](tcl-areas-audit.md) and [`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/). Plan items 153–199.
