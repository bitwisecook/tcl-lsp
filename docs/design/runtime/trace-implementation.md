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

For an array-element access the two lists are walked as **groups**: the
containing array's traces first, then the element's own (`TclCallVarTraces`
runs its `arrayPtr` loop before its `varPtr` loop). Registration order does
not decide which group runs first — only the order within a group.

The same head-first rule decides **which** registration a `trace remove`
deletes: C breaks at the first match, so among identical duplicates the newest
goes. Every removal site therefore searches from the newest end (`rposition`
over our oldest-first Vecs), as does the teardown path that collects a
namespace's unset traces before firing them.

Namespace teardown fires **command**-delete traces one command at a time, in
the order `TclTeardownNamespace` snapshots `nsPtr->cmdTable` — the retained
`TCL_STRING_KEYS` bucket order, not registration or lexical order (issue
#1752). Each token's traces fire while its entry is still in the table, then
its imports retire depth-first, then the loop moves to the next entry; the
table is re-snapshotted while it is non-empty, so a command a callback creates
is torn down in a later pass.

A namespace deleted while a call frame was still running in it fires none of
this at delete time: the token is retained and the whole loop runs from the pop
that drops its last activation instead (issue #1751,
[namespace-tree.md](namespace-tree.md) §4). Traces registered against the
retained tokens are addressed by the exact `(namespace, tail)` slot rather than
by re-resolving the name, because a retained `::N::q` and the `::N::q` of a
namespace recreated under the same spelling are two tokens with one spelling,
each firing only its own list.

A deletion drops exactly the traces the **dying token** carried, which is what
C's `Tcl_DeleteCommandFromToken` frees when it releases `cmdPtr->tracePtr`.
Both registries are keyed by command *name*, so each registration is stamped
with the generation of the token it was made against, and the deletion keeps
only the stamps that are later than the dying one:

- a trace a delete callback adds to the command **being deleted** attaches to
  that same dying token, so it never fires — not in the walk in progress
  (`CallCommandTraces` follows `active.nextTracePtr`), and not for a later
  command that takes the vacated name;
- a trace it registers on a **replacement** it bound at that name belongs to
  the new token and survives, list intact;
- a trace on a command in a namespace recreated at a **retained** token's
  spelling belongs to that recreation, and the retained token's own teardown
  neither fires nor drops it (generations are minted interpreter-wide, so the
  two never compare equal).

A hide, expose or rename moves the list with its token and re-stamps it,
because C moves the `Command` itself rather than creating a new one.

Re-entrancy is suppressed per scope: a variable trace pushes its scope onto
`active_var_scopes` for the duration of the callback, so a callback touching
the same variable does not re-fire itself. Command-trace firing is gated the
way C gates it — **per command**, on the command whose traces are running
(`CMD_TRACE_ACTIVE`, and `CMD_DYING` for a deletion), not interpreter-wide: a
callback that deletes a *different* command still fires that command's own
delete traces, nested inside the first. Execution and step traces keep the
interpreter-wide gate (`INTERP_TRACE_IN_PROGRESS`), so a callback's own
dispatches are never step-observed. The interpreter result is preserved across
every callback (held with an explicit `+1` and restored afterwards), so a trace
cannot clobber the result of the operation it observed.

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
