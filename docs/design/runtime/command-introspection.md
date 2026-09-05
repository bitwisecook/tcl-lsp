# Command manipulation + introspection

The WASM runtime's command-table introspection and mutation surface, built on
the namespace tree ([`namespace-tree.md`](namespace-tree.md)) and the
rename/alias layer ([`rename-alias.md`](rename-alias.md)):

- ``interp hide`` / ``interp expose`` / ``interp hidden`` — a
  per-interpreter hidden-commands table sitting beside the
  namespace tree.
- ``namespace which -command`` / ``namespace which -variable``
  probes, plus ``namespace current``.
- ``info commands ?pattern?`` / ``info procs ?pattern?`` listings that
  consume the same command tables dispatch resolves against.
- ``info body`` / ``info args`` / ``info default proc arg var``
  for interpreted procs.
- ``info level`` / ``info script``.

Reference Tcl 9 sources: ``tmp/tcl9.0.4/generic/tclBasic.c``
(``Tcl_HideCommand`` ``:2270``, ``Tcl_ExposeCommand`` ``:2439`` — both
live here, not in ``tclInterp.c``, which holds only the ``ChildHide`` /
``ChildExpose`` / ``ChildInvokeHidden`` argument wrappers),
``tmp/tcl9.0.4/generic/tclNamesp.c`` (``Tcl_NamespaceWhichObjCmd``,
``Tcl_GetCommandFullName``), and ``tmp/tcl9.0.4/generic/tclCmdIL.c``
(``InfoCommandsCmd``, ``InfoProcsCmd``, ``InfoDefaultCmd``).

## 1. Scope

In:

- ``interp hide`` / ``expose`` / ``hidden`` / ``invokehidden``, each
  addressing the interpreter named by a (possibly nested) path.
- ``namespace which`` (find-only) + ``namespace current``.
- ``info commands`` / ``info procs`` listings over the ns tree.
- ``info body`` / ``info args`` / ``info default`` for
  interpreted procs, following ``namespace import`` redirects to the
  underlying proc.
- ``info level`` in both forms — the no-arg depth and the ``info level
  N`` call-words form.
- ``info script ?filename?`` — reads, and optionally sets, the
  interpreter's current script name.

Covered by sibling documents:

- Command and execution traces —
  [`trace-implementation.md`](trace-implementation.md).
- ``info frame`` and the call-stack axis —
  [`proc-call-and-stack-traces.md`](proc-call-and-stack-traces.md).
- Child interpreters, path resolution, and the safe-interpreter
  permission model — [`child-interp.md`](child-interp.md).

## 2. Hidden commands table

### 2.1 Storage

`interp.rs` owns the hidden-commands table as an `InterpState` field:
`hidden: RefCell<BTreeMap<Vec<u8>, Command>>`, one per interpreter. It
is a flat map of simple names to command handles, deliberately not a
namespace tree — hidden commands have no namespace parent, and
qualified hidden names are rejected up front.

`BTreeMap` (rather than a hash table) makes ``interp hidden`` listings
sorted and deterministic, the same choice the namespace command tables
make.

There is nothing to invalidate downstream: no namespace path targets
the hidden table, and command resolution never probes it.

The per-interpreter placement, and the path-addressing that follows
from it, are described in [`child-interp.md`](child-interp.md) §5.

The internal API is five methods on `Interp`:

| Method | Purpose |
|---|---|
| ``hide_command(name, hidden_name) -> CommandVisibilityOutcome`` | Move `name` out of the command table into `hidden` under `hidden_name`. |
| ``expose_command(hidden_name, name) -> CommandVisibilityOutcome`` | The inverse: `hidden_name` leaves `hidden` and is registered as `name`. |
| ``finish_command_visibility(op, source, destination, outcome)`` | Turn either outcome into Tcl's message + ``-errorcode``. |
| ``invoke_hidden(name, argv) -> Code`` | Dispatch a hidden command. |
| ``hidden_names() -> Vec<Vec<u8>>`` | The sorted listing. |

Neither move reports a bare success flag: `CommandVisibilityOutcome` is
`Moved` / `Missing` / `NonGlobal` / `Collision`, and
`finish_command_visibility` is the single seam that words each one, so
the `interp hide|expose path …` form and the `$child hide|expose …`
shorthand cannot drift apart.

### 2.2 `interp hide` semantics

`cmd_alias.rs`'s `interp_hidectl` drives both directions. Before the
move it enforces the structural rules and the permission check:

| Condition | Message | `-errorcode` |
|---|---|---|
| the executing interp is safe | `permission denied: safe interpreter cannot hide commands` | — |
| the hidden token contains `::` | `cannot use namespace qualifiers in hidden command token (rename)` | `TCL VALUE HIDDENTOKEN` |
| the source command does not resolve | `unknown command "X"` | `TCL LOOKUP COMMAND X` |
| the source command is not in the global namespace | `can only hide global namespace commands (use rename then hide)` | `TCL HIDE NON_GLOBAL` |
| the token is already hidden | `hidden command named "X" already exists` | `TCL HIDE ALREADY_HIDDEN` |

Those rows are in C's order, and the order is observable when more than
one applies: `Tcl_HideCommand` tests the token's qualifiers, then
resolves the source, then rejects a non-global source, then refuses an
occupied token. So `interp hide kid ns::nosuch tok` is `unknown command
"ns::nosuch"` (not the non-global error), and `interp hide kid nosuch
takentok` is `unknown command "nosuch"` (not the collision error).

The move itself is `hide_command`: resolve the name at the global
namespace, delete the binding, and insert the `Command` handle into
`hidden` under the token. Nothing else has to be maintained — there is
no importer cascade to deactivate (an `Imported` redirect re-resolves
its source by name on every dispatch and simply starts failing), no
stored name slot to rewrite, and no lookup cache to flush.

**Hiding a command that does not exist raises `unknown command "X"`, matching
C Tcl's `Tcl_HideCommand` and preventing a typo from being silently swallowed
while configuring a security-sensitive command surface.** A child path that
does not resolve still reports the interpreter-path error.

### 2.3 `interp expose` semantics

The inverse — but *not* a mirror image, because C's checks differ:

| Condition | Message | `-errorcode` |
|---|---|---|
| the executing interp is safe | `permission denied: safe interpreter cannot expose commands` | — |
| the destination name contains `::` | `cannot expose to a namespace (use expose to toplevel, then rename)` | `TCL EXPOSE NON_GLOBAL` |
| the token is not hidden | `unknown hidden command "X"` | `TCL LOOKUP HIDDENTOKEN X` |
| the destination is already bound | `exposed command "X" already exists` | `TCL EXPOSE COMMAND_EXISTS` |

Again the order is C's and is observable. Two asymmetries against §2.2
are worth stating because they look like bugs and are not:

- `Tcl_ExposeCommand` has **no** token-qualifier check. A qualified
  *token* is simply a token that is not in the hidden table, so
  `interp expose kid ::tok plain` is `unknown hidden command "::tok"`.
- Its destination test is a raw "contains `::`", so a **leading** `::`
  fails it too: `interp expose kid tok ::plain` is
  `cannot expose to a namespace …`. Hide's source test is the opposite —
  a leading `::`, or any run of colons, is fine there, because that
  lookup is global-anchored anyway.

`expose_command` removes the entry from `hidden` and re-`register`s it
in the command table under the exposed name. Exposing a command that is
not hidden raises, and an occupied destination is refused rather than
overwritten.

### 2.4 `interp hidden`

`interp hidden ?path?` (and `$child hidden`) returns
`hidden_names()` of the addressed interpreter as a Tcl list of simple
names, sorted by the `BTreeMap` key order.

### 2.5 `interp invokehidden`

`interp invokehidden path ?-opt …? cmd ?arg …?` looks `cmd` up in the
addressed interpreter's hidden table, builds the argv
``[cmd, caller_args…]``, and `invoke`s the handle there — the same
`Interp::invoke` every other command goes through, so a hidden proc,
builtin, alias, or ensemble all dispatch uniformly. A missing target
raises ``invalid hidden command name "X"``; a safe executing
interpreter is refused with ``not allowed to invoke hidden commands
from safe interpreter``.

The option flags are honoured. `-global` and `-namespace ns` set the
addressed interpreter's current namespace for the duration of that one
call (saved and restored around it), `--` ends the option list, and any
other leading `-…` word is a hard ``bad option "-x": must be -global,
-namespace, or --`` rather than a silent skip. `-namespace`'s namespace
is resolved from the **global** namespace whatever the caller's current
one is, and is created if unknown — C's ``TCL_GLOBAL_ONLY |
TCL_CREATE_NS_IF_UNKNOWN`` (tclsh-pinned: `-namespace bar` from inside
`::foo` still names `::bar`).

Two clarifications, both measured on 8.6.16 and 9.0.4:

- **There is no `cannot use -global option and -namespace option
  together` error.** Earlier revisions of this document, and issue
  #1412's own item 5, asserted C rejects the pair; it does not, on
  either release. `-global` is simply spelled `-namespace ::`
  internally, so the **last** option given wins:
  `-global -namespace foo` lands in `::foo`, and
  `-namespace foo -global` lands in `::`.
- Option matching here is exact, where C's `Tcl_GetIndexFromObj`
  accepts unambiguous abbreviations (`-g`, `-n`). That gap is part of
  the wider option-prefix sweep (#1607), not of this surface.

## 3. `info commands` / `info procs`

The listing logic is a shared **Family-B core**,
`tcl_cmd_core::info::command_list`, written against the `Namespaces`
and `ValueOps` traits and used by both Rust engines.
`runtime/rust/src/cmd_info.rs` is a thin adapter: check arity, call the
core, set the result.

### 3.1 The two shapes

**Unqualified pattern** (`info commands`, `info commands foo*`):

1. the current namespace's commands, then
2. the global namespace's commands as well, when the current namespace
   is not global and the listing is `info commands` (`info procs` stays
   in the current namespace).

Names are returned simple and glob-filtered against the pattern.

**Namespace-qualified pattern** (`info commands ::foo::*`): the
qualifier is split off, the named namespace is resolved, and its
commands are listed **re-qualified absolute** with the trailing
component as the pattern. A namespace that does not resolve yields an
empty list rather than an error.

The `namespace path` search list is **not** consulted by either shape.

### 3.2 What is filtered out

- `info procs` lists only `Command::Proc` entries
  (`Namespaces::proc_names`), so aliases, imports, ensembles, child
  interpreters, OO objects, and builtins never appear.
- `info commands` lists every binding in the table, with one filter:
  a `Command::Builtin` whose name the active dialect does not expose is
  omitted (`builtin_command_visible_for_surface`, driven by
  `tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version`).
  That is what keeps a 8.4-dialect interpreter from advertising a
  command that release never had.
- Glob matching is the shared `tcl_syntax::glob::string_match`.

There are no tombstones or dead redirects to skip: a `BTreeMap` removal
is a real removal, and a dangling `Imported` redirect is a live table
entry that fails at dispatch, so it is listed (as C lists it too).

### 3.3 Hidden commands are not listed

Hidden-table entries never appear in ``info commands`` — tclsh
treats hidden commands as invisible to the resolver.  The
listing consults only the namespace command tables and never probes
the hidden side-table.

## 4. Compiled procs and rename

There is no export-name sidecar, and rename needs no compiled-proc
special case, because nothing dispatches a proc by a compile-time
export name.

An AOT-emitted module registers its procedures through
`tcl_codegen_proc_register`, which parses the parameter list and calls
`define_proc` — producing an ordinary `Command::Proc` in the ordinary
command table, indistinguishable from one `proc` created. Generic
invocation from generated code goes through `tcl_invoke_argv`, which
resolves `argv[0]` through `Interp::dispatch` at call time. A rename
therefore takes effect for compiled and interpreted procs alike, with
no anchoring to maintain and no divergence between what dispatch sees
and what `info commands` reports.

Where a compiled module *does* call a procedure directly by function
index, the soundness condition is a compile-time proof rather than a
runtime indirection: `ModuleCommandMutations::trusts_proc_binding` must
show that no command-table transition in the module can have rebound
that name. Procedure function indices are deterministic module-layout
and diagnostic metadata, not a second callable map (§8.1).

## 5. ``namespace which`` / ``namespace current``

`ns_which` in `cmd_namespace.rs` dispatches:

- ``namespace which name`` — defaults to ``-command``.
- ``namespace which -command name`` — the shared
  `tcl_cmd_core::namespace::which_command` core. Imports are **not**
  unwrapped: tclsh's ``namespace which`` returns the redirect's FQN,
  not the source's.
- ``namespace which -variable name`` — `Namespaces::which_variable`
  against the current namespace. This one is runtime-local rather than
  a Family-B core. The ``namespace path`` is **not** consulted (Tcl's
  path is commands-only).

Both flags accept unambiguous prefix abbreviations (`-var`, `-com`),
as Tcl's option table does.

Every miss returns the empty string — ``namespace which`` is a
probe, not an error-raising lookup.

``namespace current`` returns the current namespace's qualified name:
``::`` for the global namespace, ``::path::to::here`` otherwise.

The `namespace` ensemble as a whole resolves its subcommand by exact
name or unambiguous prefix, so `namespace exist` reaches `exists`; an
ambiguous or unknown prefix reports ``unknown or ambiguous subcommand
"X": must be children, code, current, delete, ensemble, eval, exists,
export, forget, import, inscope, origin, parent, path, qualifiers,
tail, unknown, upvar, or which``.

### 5.1 ``info level`` / ``info script``

``info level`` and ``info level N`` are both implemented, over the
shared `tcl_cmd_core::info::level` core. The no-arg form returns the
current logical call level — 0 at the top level, ≥1 inside a proc body.
The numeric form returns the command words that entered that frame:
`N > 0` is absolute, `N <= 0` is relative to the current call, so
`info level 0` is the current invocation. The words are kept per frame
as `Frame::words` (owned byte copies of the invoking argv), which is
what C's `CallFrame.objv` holds.

``info script`` reads the interpreter's current script name — the
`source` stack's top, empty when nothing is being sourced.
``info script filename`` sets it and returns the new value, matching
C's setter form.

## 6. ``info default``

``info default proc arg varName`` is the shared
`tcl_cmd_core::info::default` core plus a runtime-local store. The core
walks the proc's parameter list, matches ``arg`` by name, and returns a
``(value, has_default)`` pair — the default and 1 on a match with a
default, an empty value and 0 on a match without one (matching Tcl 9's
``InfoDefaultCmd``).

The **store** deliberately stays in the runtime rather than the core,
because writing `varName` is trace-aware: a write trace that errors, or
an array-typed target, makes the whole command fail with the variable
error verbatim (`can't set "a": variable is array`).

Error shapes from the core:

* A name that is not an interpreted procedure — ``"<name>" isn't a
  procedure``. `namespace import` redirects are followed first (bounded
  at 64 hops), so `info default` works on an imported proc, as
  `info args` / `info body` do.
* An argument that is not a parameter of the proc — ``procedure "X"
  doesn't have an argument "Y"``.

## 7. Test coverage

Two layers:

1. **Unit tests co-located with the implementation** — `cmd_info.rs`,
   `cmd_namespace.rs`, and `cmd_alias.rs` each carry a `mod tests`
   exercising the listings, `namespace which`, `info level` /
   `info default`, and hide / expose / hidden / invokehidden directly
   against a live interpreter, each wrapped in a leak-free assertion
   (`counters::finalize() == 0`).

2. **Upstream `.test` coverage** — ``interp.test`` (hide/expose and alias
   sections) and ``info.test`` (the `info commands` / `info procs` /
   `info default` sections) — runs through the tcltest harness against
   the bytecode VM; see [`tcl-test-tiers.md`](tcl-test-tiers.md) and
   [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md).

## 8. Known limitations

### 8.1 Compiler direct-call invalidation

Command introspection and mutation make a direct call unsound unless the
binding is proved stable at that program point. The Rust compiler obtains the
relevant command-table and interpreter-state transitions from
`tcl-registry`; it does not recognise `rename`, `interp hide`, or `interp
expose` in the WASM emitter.

Common command-binding analysis tracks literal changes precisely and widens to
an unknown wildcard after an unbounded mutation. The canonical semantic WASM
plan uses world-state and dispatch-dependency evidence. General structured
lowering in the same emitter uses `scan_module_command_mutations`, with
`ModuleCommandMutations::trusts` and `trusts_proc_binding` as the generic
guards. Affected calls use live runtime dispatch; an unrelated, proved-stable
procedure can retain direct-call specialisation.

Procedure indices remain deterministic module-layout and diagnostic metadata.
They are not an independently filtered callable map. All WebAssembly emission
enters through `tcl_compiler::codegen::wasm::compile_wasm`, and a missing
binding proof produces a typed decline rather than a second backend.

### 8.2 tcltest reachability from a bundled counter test

Running tcllib's `counter.test` as a bundle traps with
``unknown command: test``: ``::tcltest::test`` is not reachable from the
bundle's invocation site even though tcltest's first-stage sourcing completes.
The cause is the interaction between tcltest's initialisation and the
namespace-path resolver, not the introspection surface described here.

### 8.3 Divergences in the hidden-command surface

One behavioural gap remains, and it is shared with the rest of the
`interp` ensemble rather than special to this surface: option words are
matched **exactly**, where C accepts any unambiguous abbreviation
(`interp invokehidden {} -g …`, `interp hid …`) and answers the empty
option word with `ambiguous option ""`. Converting the ensemble's
hand-spelled option lists to the shared prefix matcher is issue #1607;
until it lands, only the full spellings are accepted.

The three divergences this section used to list — silent hide/expose
misses, an overwriting `expose` destination, and `invokehidden` flags
parsed and discarded — are fixed, on both the `interp <op> path` form
and the `$child <op>` shorthand. §2.2, §2.3 and §2.5 describe what the
runtime now does.

## 9. Implementation map

| Piece | Where |
|---|---|
| The `hidden` table and `hide_command` / `expose_command` / `invoke_hidden` / `hidden_names` | `runtime/rust/src/interp.rs` |
| `interp hide` / `expose` / `hidden` / `invokehidden` / `target` argument handling and permission checks, and the `hidectl_in` / `invokehidden_in` owners the `$child` shorthand shares | `runtime/rust/src/cmd_alias.rs` |
| The `info` ensemble's adapters | `runtime/rust/src/cmd_info.rs` |
| The listing / `info level` / `info body` / `args` / `default` cores | `rust/tcl-cmd-core/src/info.rs` |
| `namespace which` / `current` and the ensemble's prefix resolution | `runtime/rust/src/cmd_namespace.rs` |
| `which_variable`, `proc_names`, `command_names` | `runtime/rust/src/namespace.rs` |
| Dialect visibility filter for builtins | `runtime/rust/src/interp.rs` (`builtin_command_visible_for_surface`) |
