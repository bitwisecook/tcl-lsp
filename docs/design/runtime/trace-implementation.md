# Trace dispatch — as built

Both Rust interpreters implement the full `trace` surface: variable traces
(`array`/`read`/`unset`/`write`), command traces (`rename`/`delete`), and
execution traces (`enter`/`leave`/`enterstep`/`leavestep`). This document
describes how each runtime stores and fires them. The *semantic* contract the
firing must satisfy — ordering, error reshaping, re-entrancy, and
introspection coherence — is
[`../contracts/variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md).

Reference: `tmp/tcl9.0.3/generic/tclTrace.c` (`Tcl_TraceObjCmd`,
`TraceVariableObjCmd` / `TraceCommandObjCmd` / `TraceExecutionObjCmd`,
`TclCallVarTraces`) and `tclVar.c`.

## What is shared, and what is not

`trace` is heavily stateful — each runtime owns its trace tables, and the
firing is wired into that runtime's own variable, command, and dispatch
chokepoints. Only the **argument decoding** is shared, in
`tcl_cmd_core::trace`:

- `TraceKind` (`Variable` / `Command` / `Execution`) selects the valid
  operation names in C's `opStrings[]` table order, which is the order the
  bad-operation error enumerates them in;
- the op-list parser validates `{read write}`-shaped specs and produces the
  canonical `bad type` / `bad operation` messages.

Each runtime converts the canonical op names into its own representation: the
bytecode VM keeps the name list, the WASM runtime folds them into an op
bitset.

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
| variable | `name1 name2 op` — `name2` is the element key for an array access, empty for a scalar; `op` is the full word (`read`/`write`/`unset`/`array`) |
| command | `oldName newName rename`, or `oldName {} delete` |
| execution | the command's own words, plus the op word |

Whole-array traces fire before element traces; within a list, newest-first.
A per-record active flag makes a callback that re-enters the same variable
terminate. A read or write callback that errors reshapes the operation's
result (`can't read "NAME": …` / `can't set "NAME": …`) and stops further
firing; an unset callback's error is discarded and the remaining unset traces
still fire. The mutation itself is never gated on the callback's outcome.

## Key files

| File | Role |
|---|---|
| `rust/tcl-cmd-core/src/trace.rs` | shared op-list parsing + error catalogue |
| `runtime/rust/src/cmd_trace.rs` | the `trace` command for the tree-walking runtime |
| `runtime/rust/src/interp.rs` | `fire_var_trace`, `fire_cmd_trace`, the dispatch-side execution hook |
| `runtime/rust/src/frame.rs` | per-frame variable cells the variable-trace key resolves against |
| `rust/tcl-vm/src/cmd_trace.rs` | the `trace` command for the bytecode VM |
| `rust/tcl-vm/src/interp.rs` | VM trace tables, the step-trace epoch, `cmd_trace_entries` |
| `rust/tcl-vm/tests/command_traces_e2e.rs` | tclsh-pinned command/execution trace vectors |
