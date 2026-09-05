# Trace dispatch — as built

Both Rust interpreters implement the full `trace` surface: variable traces
(`array`/`read`/`unset`/`write`), command traces (`rename`/`delete`), and
execution traces (`enter`/`leave`/`enterstep`/`leavestep`). This document
describes how each runtime stores and fires them. The *semantic* contract the
firing must satisfy — ordering, error reshaping, re-entrancy, and
introspection coherence — is
[`../contracts/variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md).

Reference: `tmp/tcl9.0.4/generic/tclTrace.c` (`Tcl_TraceObjCmd`,
`TraceVariableObjCmd` / `TraceCommandObjCmd` / `TraceExecutionObjCmd`,
`TclCallVarTraces`) and `tclVar.c`, cross-checked against `tmp/tcl8.6.16/`
for the deprecated forms 9.0 dropped.

## What is shared, and what is not

`trace` is heavily stateful — each runtime owns its trace tables, and the
firing is wired into that runtime's own variable, command, and dispatch
chokepoints. Only the **argument decoding** is shared, in
`tcl_cmd_core::trace`:

- `resolve_option` resolves `trace`'s first word (`add`/`info`/`remove`, plus
  8.x's `variable`/`vdelete`/`vinfo`) with C's `Tcl_GetIndexFromObj` rule —
  exact match first, then a unique prefix — against the option set the caller
  passes, and produces the `bad option` / `ambiguous option` text enumerating
  exactly those options;
- `TraceKind` (`Variable` / `Command` / `Execution`) selects the valid
  operation names in C's `opStrings[]` table order, which is the order the
  bad-operation error enumerates them in;
- the op-list parser validates `{read write}`-shaped specs and produces the
  canonical `bad type` / `bad operation` messages;
- `parse_legacy_variable_ops` does the same for the 8.x `rwua` letter string,
  `legacy_ops_letters` renders a stored set back to letters for `trace vinfo`,
  and `callback_op_word` supplies the single letter an old-style trace's
  callback receives.

**Op sets are stored in `TraceKind::info_order`** — the order each C
`TRACE_INFO` arm tests the stored flag bits, which is *not* the `opStrings[]`
table order: `array read write unset` for variable and `rename delete` for
command (execution alone agrees with its table). Because Tcl keeps the
selection as a bitset, the reported order never depends on how the op list was
spelled, so a runtime that stores the canonical set renders `trace info`
correctly by construction. Each runtime converts those names into its own
representation: the bytecode VM keeps the name list, the WASM runtime folds
them into an op bitset (and back through the same fixed order on the way out).

### The release boundary is the registry's

Tcl 9.0 dropped `trace variable`, `trace vdelete`, and `trace vinfo` (C
compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES`). Neither runtime
carries a list of its own: both filter the `trace` spec's subcommands by the
pinned profile's point, so the three 8.x-only subcommands
vanish at 9.0+ — the forms stop working *and* the `bad option` enumeration
shortens to `add, info, or remove`, in one step. A spec edit moves both
runtimes with it.

## Chokepoints

A trace is only correct if **every** mutation or dispatch path routes through
one firing site. Both runtimes funnel accordingly.

### `runtime/rust` (tree-walking runtime)

| Trace kind | Storage | Fired from |
|---|---|---|
| variable | keyed by resolved variable identity — home namespace *or* local call-frame level, plus the simple name | `Interp::fire_var_trace` / `fire_var_trace_resolved`, called from the scalar and array-element read/write/unset paths |
| command | keyed by resolved FQN (`Interp::resolve_cmd_fqn`) | `Interp::fire_cmd_trace`, from the `rename` and command-delete paths |
| execution | keyed by resolved FQN | `Interp::dispatch` — `enter`/`leave` around the traced command's own invocation, `enterstep`/`leavestep` around every command executed while a step-traced command is on the stack |

Because the key is the *resolved* identity rather than the written name, a
trace follows the variable or command through `upvar`/`global` links and
through a `rename` (`move_cmd_traces`), and is dropped when the command is
deleted (`remove_cmd_traces`) or its frame is popped
(`clear_frame_var_traces`).

### `rust/tcl-vm` (bytecode VM)

The VM keeps the same three tables (`cmd_traces` / `exec_traces` on the
interpreter) keyed by resolved FQN, plus one extra piece of state that the
tree-walker does not need: because the VM executes compiled proc bodies,
installing a new `enterstep`/`leavestep`-capable trace has to invalidate the
compiled bodies that would otherwise skip the per-command step hook. An
epoch counter, bumped on every `trace add|remove execution … enterstep`,
drives that invalidation.

Behaviour is pinned end-to-end in `rust/tcl-vm/tests/command_traces_e2e.rs`:
names arrive fully qualified, an enter-trace error aborts the command, a
leave-trace error replaces its result, `rename`/`delete` callback errors are
ignored, traces follow a `rename`, and a redefinition fires the `delete`
trace.

## Callback shape and firing order

A callback is evaluated as a script: the verbatim command prefix with the
trace's arguments appended as list elements.

| Kind | Appended words |
|---|---|
| variable | `name1 name2 op` — `name2` is the element key for an array access, empty for a scalar; `op` is the full word (`read`/`write`/`unset`/`array`), or the single `rwua` letter when the trace was installed by 8.x's `trace variable` (C's `TCL_TRACE_OLD_STYLE`) |
| command | `oldName newName rename`, or `oldName {} delete` |
| execution | the command's own words, plus the op word |

**Firing order is uniform across the kinds** because C prepends every new
registration (`TraceVarEx`, `Tcl_TraceCommand`) and each firing loop walks the
list head→tail: the **newest** trace fires first for variable `read`/`write`/
`unset`/`array`, for command `rename`/`delete`, and for execution
`enter`/`enterstep`. The single exception is C's explicit reverse scan for
`leave`/`leavestep`, which run **oldest-first**. Both runtimes push
newest-last, so each firing site iterates reversed; `trace info` also lists
newest-first, so it iterates reversed too.

From Tcl 9.0 an access whose spelling names no element but whose resolved
variable *is* one (`upvar #0 a(k) e; set e 5`) recovers the containing array —
so its traces run as well — and the element's key, reported as `name2`; an
element unset named by the one-part `a(k)` spelling likewise reports that whole
spelling as `name1`. 8.4/8.5/8.6 do neither. The release axis is the dialect
fact `TclVersion::traces_recover_linked_array_element`, which both engines read
rather than comparing versions beside their trace loops.

For an array-element access the two lists are walked as **groups**: the
containing array's traces first, then the element's own (`TclCallVarTraces`
runs its `arrayPtr` loop before its `varPtr` loop). Registration order does
not decide which group runs first — only the order within a group.

The same head-first rule decides **which** registration a `trace remove`
deletes: C breaks at the first match, so among identical duplicates the newest
goes. Every removal site therefore searches from the newest end (`rposition`
over our oldest-first Vecs), as do the teardown paths that collect a
namespace's unset and command-delete traces before firing them.

Every firing loop walks **live** state rather than a snapshot: it collects the
registrations' ids up front, in the order above, and re-finds each one in the
table immediately before running it. That is C's `active.nextTracePtr` /
`nextPtr` walk, which `Tcl_UntraceVar2` and `Tcl_UntraceCommand` rewrite as
they unlink a record — so a trace a callback removes does not fire in the same
pass, while one it adds waits for the next access (C prepends, behind the
walk). An unset is the exception, and for the same reason: it takes the
variable's own list out of the table before firing (C moves it to a dummy
`Var`), so nothing can remove those callbacks any more, and a variable a
callback revives carries no traces.

Re-entrancy is suppressed per scope: a variable trace pushes its scope onto
`active_var_scopes` for the duration of the callback, so a callback touching
the same variable does not re-fire itself, and command-trace firing is gated on
`exec_firing`. The interpreter result is preserved across every callback (held
with an explicit `+1` and restored afterwards), so a trace cannot clobber the
result of the operation it observed.

A read or write callback that errors reshapes the operation's result
(`can't read "NAME": …` / `can't set "NAME": …`) and stops further firing —
C's abort-the-chain-on-first-error. An `unset` or `array` callback's error is
discarded and the remaining traces still fire. Command-trace
(`rename`/`delete`) callback errors are ignored outright, matching C's "we
ignore errors in these traced commands".

**The mutation is never gated on the callback's outcome.** C stores the new
value before calling the write traces (`TclPtrSetVarIdx` swaps it in, then
jumps to `cleanup` on a trace error without restoring the old one), so a
failed `set`/`append`/`incr`/`lappend` — on a scalar, an array element, or a
variable the write itself created — still leaves the new value in place. Only
the command's *result* changes.

Ordering the contract fixes but the implementations reach differently is
authoritative in
[`../contracts/variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md),
not here.

## Key files

| File | Role |
|---|---|
| `rust/tcl-cmd-core/src/trace.rs` | shared option/op-list parsing, canonical op order, legacy letter rendering, error catalogue |
| `rust/tcl-registry/src/commands/tcl/trace.rs` | the `trace` spec — the option set per release, including the 8.x-only legacy forms |
| `runtime/rust/src/cmd_trace.rs` | the `trace` command for the tree-walking runtime |
| `runtime/rust/src/interp.rs` | `fire_var_trace`, `fire_cmd_trace`, the dispatch-side execution hook |
| `runtime/rust/src/frame.rs` | per-frame variable cells the variable-trace key resolves against |
| `rust/tcl-vm/src/cmd_trace.rs` | the `trace` command for the bytecode VM |
| `rust/tcl-vm/src/interp.rs` | VM trace tables, the step-trace epoch, `cmd_trace_entries` |
| `rust/tcl-vm/tests/command_traces_e2e.rs` | tclsh-pinned command/execution/variable trace vectors, including firing order |
| `rust/tcl-vm/tests/legacy_variable_traces_e2e.rs` | tclsh-pinned cross-version vectors for `trace variable`/`vdelete`/`vinfo` |
