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

### The `rename` trace window

C's `TclRenameCommand` (`tclBasic.c` 9.0.4) creates the destination hash
entry, fires the `rename` traces, and only *then* deletes the source entry.
Both entries reference the one `Command`, and the traces hang off that, not
off either entry. So for the callbacks' duration the vacating name **is** the
destination command:

- both names resolve and are callable, and the command is already re-homed
  (`cmdPtr->nsPtr = newNsPtr` happens before the firing), so a body invoked
  through the vacating name reports the *destination's* `namespace current` —
  and likewise a TclOO object, an ensemble, an import redirect and a live
  coroutine all still dispatch through the vacating name, because C reaches
  each of them through the one `Command` (in `runtime/rust`,
  `Namespaces::rehome_command` re-points the identity our `Command` variants
  carry in its stead);
- `trace info command <old>` and `… <new>` answer the same list, and a
  `trace add` or `trace remove` through either name edits it;
- a `rename` or a delete through *either* name moves or destroys that one
  command — and C's `CMD_TRACE_ACTIVE` keeps the pass's remaining callbacks
  from re-firing when it does.

**Both** runtimes reproduce that window rather than the naive "mutate, then
fire", by the same three steps: publish the destination, move everything the
command carries to the destination key and fire from there — passing the old
fully-qualified name for the callback's first word — and only afterwards drop
the source's table entry, which is `Tcl_DeleteHashEntry(oldHPtr)`: a plain
removal that fires no `delete` trace.

| | publish | fire from | retire the source |
|---|---|---|---|
| `runtime/rust` | `Namespaces::publish_rename_destination` (writes the re-homed command into *both* slots) | `Interp::move_bound_command`, after moving the trace list, the import redirects, the TclOO registration and the coroutine | `Namespaces::retire_rename_source` |
| `rust/tcl-vm` | `cmd_rename` registers the destination | `on_command_renamed_traces`, after moving the sidecars | `retire_renamed_command_source` |

Neither has a shared command object to hang that equivalence on, so an open
window is recorded as a source→destination pair — `TraceTable::rename_windows`
in the tree-walker, `rename_windows` in the VM, a stack either way with one
frame per nested rename. `Interp::renamed_cmd_key` / `renamed_command_key`
resolves a name through it: used by `trace add`/`remove`/`info`, by the
tree-walker's execution-trace lookup and coroutine registry, and by
`Interp::rename_command` / `prepare_command_rename`, which is what makes a
callback's `rename <old> <third>` and `rename <old> {}` act on the
destination. A nested rename would otherwise strand that state on the key it
just vacated, so `relocate_rename_state` retargets every enclosing window
*and* every `firing_cmd_traces` record — the name-addressed stand-in for
`CMD_TRACE_ACTIVE`, which C gets for free from the `Command` being one object.

Two residues of that same missing shared identity are not emulated by either
engine, both because C's own behaviour there is a torn-state artefact rather
than a contract: re-*creating* the vacating name from a callback
(`proc <old> {} …`) leaves C deleting a freed hash entry and killing the
destination with it, where both engines leave the command standing under its
new name; and 8.6 and 9.0 disagree with each other on what
`info commands <old>` reports after a callback deletes the command.


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

Every firing loop walks **live** state rather than a snapshot: it collects the
registrations' identities up front, in the order above, and re-checks each one
immediately before running it. That is C's `active.nextTracePtr` / `nextPtr`
walk, which `Tcl_UntraceVar2` and `Tcl_UntraceCommand` rewrite as they unlink a
record — so a trace a callback removes does not fire in the same pass, while
one it adds waits for the next access (C prepends, behind the walk).

*What* the re-check consults differs by kind, because C's walks hold different
things:

| walk | holds | a callback that redefines the traced command |
|---|---|---|
| variable | the cell's list | — |
| execution (`enter`/`leave`/step) | the list of whatever command the name holds now | stops the rest of the walk |
| command (`rename`/`delete`) | the dying **token's** own list | does not stop it |

So the execution loops re-read the name-keyed table, while the command loops
consult a per-registration "untraced" mark that only `trace remove` sets: a
callback's `proc foo …` takes the table entry over without touching the list
the walk is following, and the remaining callbacks still run. Once a
replacement holds the name, a `trace remove` inside a later callback reaches
*its* list and so cancels nothing. All three shapes are pinned against tclsh
8.6.16 and 9.0.4.

The token stamp above and this untraced mark answer different questions and
both are needed: the stamp says which registrations a **deletion frees** once
the walk is over, the mark says which the **walk skips** while it is still
running. A delete callback that rebinds the name exercises both at once — the
rebinding must not cut the walk short (the mark's job), and the dying token's
list must still go when the walk ends, without taking the replacement's own
registrations with it (the stamp's job).

An unset is the variable-side exception, and for the same reason: it takes the
variable's own list out of the table before firing (C moves it to a dummy
`Var`), so nothing can remove those callbacks any more, and a variable a
callback revives carries no traces.

Re-entrancy is suppressed per scope: a variable trace pushes its scope onto
`active_var_scopes` for the duration of the callback, so a callback touching
the same variable does not re-fire itself. Command-trace firing is gated the
way C gates it — **per command**, on the command whose traces are running
(`CMD_TRACE_ACTIVE`, and `CMD_DYING` for a deletion), not interpreter-wide: a
callback that deletes a *different* command still fires that command's own
delete traces, nested inside the first. The interpreter result is preserved
across every callback (held with an explicit `+1` and restored afterwards), so
a trace cannot clobber the result of the operation it observed.

The interpreter-wide gate (`INTERP_TRACE_IN_PROGRESS`) belongs to **execution
traces alone**. C sets it in exactly one place — `TraceExecutionProc`
(`tclTrace.c` 9.0.4:1765), around an `enter`/`leave`/`enterstep`/`leavestep`
callback — and reads it in exactly one place, `TclCheckInterpTraces` (:1426),
the step machinery. So an execution callback's own dispatches are never
step-observed, while `CallCommandTraces` sets nothing at all: a command a
`rename`/`delete` callback dispatches is traced like any other — its
`enter`/`leave` traces fire, and an enclosing step scope steps both the
callback's invocation and the commands its body runs. Both runtimes used to
raise their stand-in (`TraceTable::exec_firing`, and the VM's
`trace_in_progress`) for command callbacks too, which silently untraced
everything such a callback dispatched.

Both stand-ins are *read* exactly where C reads its flag and nowhere else:
`runtime/rust` gates only the step firing in `Interp::dispatch_traced`, and the
VM only its `step_scopes_to_fire`. So a command dispatched from inside an
execution callback fires its own `enter` and `leave` traces, the way
`TclCheckExecutionTraces` fires them; only an `enterstep`/`leavestep` scope
goes quiet for the callback's duration. Both engines used to read the gate at
the whole traced-dispatch fast path instead, and fired neither.

What bounds a callback that invokes the command it traces is C's **per-trace**
`TCL_TRACE_EXEC_IN_PROGRESS` (:1655), not the interpreter-wide flag — and per
*trace* is observably different from per command: a second `enter` trace on the
same command fires for that inner call, and so does a `leave` trace while an
`enter` one is running. `runtime/rust` carries that as
`TraceTable::firing_exec_traces` over `CmdTrace::id`; the VM carries it per
entry as `CmdTraceEntry::firing`.

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
| `runtime/rust/tests/trace_semantics.rs` | tclsh-pinned transcripts for the tree-walker, including the `rename` window |
| `rust/tcl-vm/tests/command_traces_e2e.rs` | tclsh-pinned command/execution/variable trace vectors, including firing order |
| `rust/tcl-vm/tests/legacy_variable_traces_e2e.rs` | tclsh-pinned cross-version vectors for `trace variable`/`vdelete`/`vinfo` |
