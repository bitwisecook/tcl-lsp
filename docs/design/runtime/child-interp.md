# Child interpreters

Per-interpreter state in the WASM runtime — namespace tree, hidden-commands
table, children registry — plus `interp create` / `eval` / `exists` /
`slaves` / `delete`, and the child-path-aware forms of `interp alias` /
`hide` / `expose` / `invokehidden`. Builds on
[`namespace-tree.md`](namespace-tree.md),
[`rename-alias.md`](rename-alias.md), and
[`command-introspection.md`](command-introspection.md).

Reference Tcl 9 sources: `tmp/tcl9.0.3/generic/tclInterp.c`
(`ChildCreate`, `ChildEval`, `InterpObjCmd` dispatch,
`OPT_{CREATE,EVAL,EXISTS,SLAVES,DELETE}` branches) and
`tmp/tcl9.0.3/generic/tclBasic.c` (`Tcl_CreateInterp` /
`DeleteInterp`).

## 1. Scope

In:

- `interp create ?-safe? ?--? ?path?` — auto-named and explicit-
  named child creation, including multi-level paths (`interp
  create {a x1}` creates `x1` as a child of `a`).  `-safe` is
  accepted anywhere before `--`, matching C's historical rule.
- `interp eval path script ?script ...?` — concat-with-spaces,
  dispatch inside the resolved child's namespace tree.
- `interp exists ?path?` — pure lookup, non-raising.
- `interp slaves ?path?` / `interp children ?path?` — enumerate
  the resolved interp's direct children.
- `interp delete ?path ...?` — remove each named child from its
  parent, deferring teardown when the child is mid-eval (§9).
- `interp issafe ?path?` / `interp marktrusted path`.
- `interp hide` / `expose` / `hidden` / `invokehidden`, each
  path-aware, with the safe-interpreter permission checks.
- `interp recursionlimit path ?newlimit?`, `interp limit path
  limitType ?-option value …?`, `interp bgerror path ?cmdPrefix?`,
  and `interp debug path ?-frame ?bool??`.
- Child-as-command dispatch: `<child> eval script` resolves
  through `Command::ChildInterp` and routes into the child.
- Per-interp `InterpState` carrying the namespace tree, frame
  stack, `children`, `hidden`, `parent`, and `is_safe`.
- Child→parent aliases (`interp alias childPath name {} target`),
  covered in [`rename-alias.md`](rename-alias.md) §4.5.
- Conservative compiler-side command-binding distrust when registry-declared
  interpreter or command-table transitions cannot be bounded.
- tclsh-style error wording for the ensemble: `bad option "X": must be
  alias, aliases, bgerror, cancel, children, create, debug,
  delete, eval, exists, expose, hide, hidden, issafe,
  invokehidden, limit, marktrusted, recursionlimit, share,
  target, or transfer`; `bad option "-X": must be -safe or --`
  for `interp create`.

Known gaps:

- `interp target`, `interp share`, `interp transfer`, and
  `interp cancel` are not implemented.  They appear in the
  `bad option` list (which is the verbatim tclsh wording) but have
  no dispatch arm, so invoking one reports `bad option` naming a
  subcommand the same message lists as valid.
- The Safe Base (`safe.tcl`) is not present.  `-safe` does the
  `Tcl_MakeSafe` half of the job (§6); what is missing is the Tcl-level
  access-path virtualisation that re-aliases `source` / `load` / `file`
  / `glob` onto token-mapped parent commands, so a safe child cannot
  load packages the way a real safe interpreter can.
- `$child <sub>` for a subcommand with no arm reports
  `interp subcommand "X" is not supported in this runtime` rather than
  the tclsh `bad option` list that the `interp <sub>` path emits.
- Querying a child alias (`interp alias childPath name`) reports
  `querying a child alias is not yet supported`.
- Cross-interp aliases only run child→parent.  A non-empty *target*
  path is refused (`rename-alias.md` §4.5).

## 2. The interpreter handle

`interp.rs` owns the one-interpreter-per-`InterpState` model. The handle
is `Interp(Rc<InterpState>)`: cheap to clone, and every clone shares one
state. There is no interpreter registry, no `Interp*` raw pointer, and
no module-global "current interp" the runtime reads behind the caller's
back — the interpreter you are running in is the receiver you called
through.

The child-interpreter fields of `InterpState` are:

| Field | Owns |
|---|---|
| `namespaces` | This interp's whole namespace arena, whose element 0 is its `::`. Each interp has its own; nothing is shared with the parent. |
| `hidden` | This interpreter's hidden-command table, `BTreeMap<Vec<u8>, Command>` (`interp hide`). |
| `children` | Direct children, `BTreeMap<Vec<u8>, Interp>` — simple name to handle. The parent owns its children through this map. |
| `parent` | `RefCell<Weak<InterpState>>` — set for the duration of a call *into* this interp and restored afterwards. `Weak`, so parent→child ownership has no cycle. |
| `is_safe` | `interp create -safe` / `interp issafe` / `interp marktrusted`. |
| `interp_counter` | Sequence for auto-generated names (`interp0`, `interp1`, …). |
| `eval_active` | How many of this interp's evals are currently on the stack. |
| `pending_delete` | A delete requested while `eval_active > 0`, applied when the last eval unwinds. |
| `recursion_limit` | Per-interp `interp recursionlimit`; a child's is independent of its parent's. |
| `limits` | `interp limit` configuration (the `time` limit is enforced by the loop commands; `commands` is stored for query/set). |
| `bgerror` / `bg_queue` | The background-error handler prefix and the queue `update` drains. |

Every field is interior-mutable (`RefCell` / `Cell`) and borrowed only
for the span of a single operation — **never across a sub-eval**. That
is what makes re-entrancy safe: a re-entry re-borrows freshly instead of
aliasing a `&mut`, and a discipline slip is a clean panic rather than
undefined behaviour. Single-threaded throughout: `Rc` + `RefCell`, no
locks.

A child created by `create_child` inherits three things from its creator
and nothing else: the runtime dialect version (a child is another
interpreter of the *same* Tcl build, not a different release — issue
#1328), the predefined startup globals (`tcl_platform` and friends, for
a non-safe child), and the `-frame` debug flag when the creator's
`env(TCL_INTERP_DEBUG_FRAME)` is set (C's `Tcl_CreateChild`). Variable
*resolution* runs against the child's own global namespace; the rule is
inherited, the variables are not.

## 3. Entering / leaving a child

There is no global state to swap. Running something in a child is a
plain nested native call through the child's handle:

```rust
self.with_child(name, |child| child.eval_str(script))
```

`with_child` clones the child's handle out of the `children` map (an
`Rc` bump) and releases the map borrow *before* running the closure.
That release is the whole mechanism: the closure may re-enter `self`
(a child→parent alias calling back up), and may even re-enter the same
child through a fresh handle, because the shared `InterpState` is
reached through the `Rc` plus per-field interior mutability rather than
an aliased `&mut`.

Around the closure, `with_child` does three things:

1. Installs `self` as the child's `parent` `Weak` and restores the
   previous value afterwards — so a `ParentAlias` invoked during the
   call can upgrade to the right parent, and nesting unwinds correctly
   because each call saves the value that was live when it ran.
2. Increments `eval_active` and decrements it on the way out.
3. On the way out, if `pending_delete` is set and `eval_active` has
   returned to zero, drops the handle clone and *then* removes the
   child from the map and its command from the table — never while a
   re-entrant eval of it is still on the stack.

`eval_in_child` wraps that with the result handshake: run `eval_str` in
the child, copy the child's result bytes and completion code back into
the caller, and raise `could not find interpreter "X"` when the name
does not resolve.

## 4. Resolving paths

An interpreter path is a Tcl list of child names descending from the
current interpreter. `interp_path` parses the argument as a list
(falling back to a single-element path when it is not well-formed
list syntax, and to the empty path when it is empty).

`with_child_path(path, f)` walks it: an empty path is this interp
itself, a one-element path is `with_child`, and a longer one recurses —
so each hop gets its own `parent` wiring and its own `eval_active`
accounting, and the whole chain unwinds in order. It returns `None` if
any name in the chain is not a child of its predecessor.

Call sites turn that `None` into the tclsh error via `not_found_path`,
which renders the path back as a Tcl list: `could not find interpreter
"a b"`.

## 5. Per-interp hidden commands

The hidden table is a field of `InterpState`, so `hide` / `expose` /
`hidden` / `invokehidden` naturally address whichever interpreter the
path resolved to — there is no interpreter argument to thread through.

- `hide_command(name)` resolves `name` at the global namespace, removes
  the binding, and moves the `Command` into `hidden`.
- `expose_command(name)` is the inverse: remove from `hidden`,
  re-`register` in the command table.
- `invoke_hidden(name, argv)` clones the handle out of `hidden` and
  `invoke`s it, or raises `invalid hidden command name "X"`.
- `hidden_names()` lists the table (a `BTreeMap`, so the listing is
  sorted and deterministic).

Two structural restrictions are enforced at the command layer, matching
C: the hidden-command token may not carry namespace qualifiers
(`cannot use namespace qualifiers in hidden command token (rename)`),
and only global-namespace commands may be hidden
(`can only hide global namespace commands (use rename then hide)`).

The permission checks are on the **executing** interpreter, not the
target: a safe interpreter may not hide, expose, or invoke hidden
commands in itself or in any of its children
(`permission denied: safe interpreter cannot hide commands`,
`not allowed to invoke hidden commands from safe interpreter`).

## 6. `-safe` and cross-interp alias

`interp create -safe` calls `make_safe`, which does the `Tcl_MakeSafe`
work:

- Hides the host-reaching commands: `exec`, `exit`, `cd`, `pwd`,
  `glob`, `open`, `socket`, `source`, `load`, `file`, `fconfigure`,
  `encoding`. `after` and `vwait` are deliberately **not** on that list
  — pinned against real tclsh 8.6.14, where `interp create -safe s; s
  eval {info commands after}` shows `after` present and callable.
- Unsets the host-revealing `tcl_platform` elements (C unsets
  `os`/`osVersion`/`machine`/`user`; this runtime also drops its own
  backend-introspection keys), leaving the portable subset:
  `byteOrder`, `engine`, `pathSeparator`, `platform`, `pointerSize`,
  `wordSize`.
- Unsets `env`, `tcl_library`, `tclDefaultLibrary`, and `tcl_pkgPath`.
- Installs `clock` as a `ParentAlias`, so date/time formatting works in
  the child without it reaching the timezone files.

`interp marktrusted` clears the flag; a safe interpreter cannot call it
on a child (`permission denied: safe interpreter cannot mark trusted`).

Cross-interp aliasing is the child→parent direction:
`interp alias childPath name {} target ?arg…?` installs a
`Command::ParentAlias` in the child, and invoking it upgrades the
child's `parent` `Weak` and dispatches through the parent handle. The
mechanism, its two guards, and the unimplemented directions are in
[`rename-alias.md`](rename-alias.md) §4.5.

## 7. Compiler: registry-driven command-binding proof

The compiler does not recognise `interp` by name. `tcl-registry` describes
interpreter-state and command-table transitions for each form. Common semantic
analysis projects those descriptors into world-state and dispatch-dependency
facts at each invocation.

The canonical WASM pipeline consumes those facts first. A direct or intrinsic
plan is eligible only when the relevant command binding and world dependency
are proved at that program point. Child creation, cross-interpreter eval,
aliasing, hiding, deletion, or a dynamic transition can therefore make the
selector abstain without any command-specific WASM branch. Generic argv
dispatch remains sound because it uses the live runtime command table.

General structured lowering in the sole emitter also uses the generic
`scan_module_command_mutations` summary. `ModuleCommandMutations::trusts`
guards builtin assumptions, and `trusts_proc_binding` guards direct procedure
calls. A literal transition can distrust only the affected name; an unbounded
transition sets the wildcard state and distrusts every binding.

Procedure function indices remain deterministic module-layout and diagnostic
metadata. They are not a second callable map or a public code-generation API.
Whether a call may use an index is a semantic binding proof, not a mutation of
the metadata table. All module emission still enters through
`tcl_compiler::codegen::wasm::compile_wasm`.

## 8. Dispatch interplay

Cross-interp dispatch moves no globals, so there is no cache to
poison and no invariant to hold across the boundary. Each interpreter
resolves commands in its own `Namespaces` arena and variables in its own
frame stack; "which interpreter am I in" is the receiver, and it changes
only by making a call through a different handle.

The two directions compose:

- **Parent into child** — `with_child` / `with_child_path` (§3).
- **Child into parent** — `dispatch_parent_alias` upgrading the
  `parent` `Weak` (§6).

Both are ordinary nested native calls on one Rust stack, mirroring C's
`Tcl_EvalObjv(targetInterp, …)`. The Safe Base's child→parent→child
cycle therefore works: a child's aliased command calls into the parent,
which calls `interp invokehidden $child …` back into the *same* child
while the child's outer eval is still on the stack. The only bound on
that recursion is `MAX_CROSS_INTERP_DEPTH`, which exists to cap
native-stack growth, not to preserve any invariant.

## 9. Deletion

`interp delete path ...` resolves each path to `(parent, leaf)` and
calls `delete_child(leaf)` on the parent. That does two things:

1. Deletes the `<child>` command from the parent's table, so a
   post-delete `<child> eval ...` surfaces as an ordinary
   `invalid command name` from the normal dispatch path.
2. Removes the child from the parent's `children` map — *unless* the
   child is mid-eval (`eval_active > 0`), in which case it sets
   `pending_delete` and leaves the map entry alone. The command binding
   still goes immediately, so the name stops dispatching at once; the
   handle is dropped by `with_child` when the last eval of that child
   unwinds. This is C's deferred `Tcl_DeleteInterp`, and it is what
   makes a self-deleting child safe — the child whose aliased `exit`
   calls `interp delete` on itself.

There is no cascade to write and no reclaim to defer. Dropping the
child's handle drops its `Rc<InterpState>`, which drops its `children`
map, which drops its own children, and so on down the tree; each
`InterpState` drop releases its namespace arena, `VarTable`s (which
release every object they own, `memory-management.md` MM-B), hidden
table, and frames. A cross-interp alias pointing into a deleted child
cannot outlive it either: the child held its parent as a `Weak`, and a
parent-side reference to a deleted child is simply gone from the
`children` map, so the name no longer resolves.

A path that does not resolve raises `could not find interpreter "a b"`
and stops — earlier paths in the same command have already been deleted.

## 10. Dialect compatibility

| Command | 8.4 | 8.5 | 8.6 | 9.0 |
|---|---|---|---|---|
| `interp create` | ✓ | ✓ | ✓ | ✓ |
| `interp eval` | ✓ | ✓ | ✓ | ✓ |
| `interp delete` | ✓ | ✓ | ✓ | ✓ |
| `interp exists` | — | ✓ | ✓ | ✓ |
| `interp slaves` | ✓ | ✓ | ✓ | ✓ (deprecated alias for `children`) |
| `interp children` | — | — | — | ✓ |

`interp exists` was added in Tcl 8.5; 8.4 scripts either use
`catch {interp eval path {}}` or probe through the children
list.  We ship the 8.5+ form; the 8.4-style probe keeps working
through the `catch` fallback.

## 11. Test coverage

- **`runtime/rust`** is covered by unit tests co-located with the
  implementation: `cmd_alias.rs`'s `mod tests` drives `interp create` /
  `eval` / `exists` / `children` / `delete` and the child-as-command
  path, plus `hide` / `expose` / `invokehidden` and `interp create
  -safe`, directly against a live interpreter. The upstream
  `interp.test` file is not run against this engine.
- **`tcl-vm`** is gated against the upstream `interp.test` /
  `safe.test` files by the tcltest sweep (`rust/xtask/src/tcltest_sweep.rs`),
  whose per-stem numbers are the
  [tier scoreboard](rust-vm-tier-parity.md). Child interpreters are the
  [Tier 8](tcl-test-tiers.md) group ("Interpreters": `interp`, `safe`,
  `safe-stock`, `safe-stock86`).

## 12. Implementation map

| Piece | Where |
|---|---|
| `InterpState`'s child fields, `with_child` / `with_child_path`, `create_child` / `delete_child` / `eval_in_child`, `make_safe`, the hidden-table ops, `dispatch_child`, `dispatch_parent_alias` | `runtime/rust/src/interp.rs` |
| The `interp` ensemble's argument handling — `interp_create`, `interp_delete`, `interp_alias`, `interp_aliases`, `interp_hidectl`, `interp_invokehidden`, `interp_limit`, `interp_marktrusted`, `interp_debug`, `interp_path`, `not_found_path` | `runtime/rust/src/cmd_alias.rs` |
| `Command::ChildInterp` / `Command::ParentAlias` | `runtime/rust/src/interp.rs` |
| Per-interp namespace arena (`Namespaces`) | `runtime/rust/src/namespace.rs` |
| Compiler-side command-binding proof (`scan_module_command_mutations`, `trusts`, `trusts_proc_binding`) | `rust/tcl-compiler/src/command_binding.rs` |

## 13. The two engines

The two Rust execution engines implement the same contract with different
architectures:

- **`runtime/rust`** (tree-walk runtime — native and wasm32-wasip1) is what
  §2–§12 above describe: `Interp(Rc<InterpState>)` with a `children` map, a
  `hidden` table, a `parent` `Weak`, and an `is_safe` flag, with `with_child`
  as the re-entrancy guard and child→parent aliases (§6). `interp
  create`/`eval`/`delete`/`exists`/`children`/`issafe`/`marktrusted`/`hide`/
  `expose`/`hidden`/`invokehidden`/`recursionlimit`/`limit`/`bgerror`/`debug`
  are implemented; the Safe Base and `interp target`/`share`/`transfer`/
  `cancel` are the gaps listed in §1.

- **`tcl-vm`** (bytecode VM) is an engine (`Vm`) driving a tree of
  interpreters in one arena: each interpreter's state (`InterpState`) lives
  in a slot addressed by a stable `InterpId` (never reused), and the
  currently-executing interpreter's state is held directly on `Vm`
  (`Deref<Target = InterpState>`, one pointer hop, no lookup on the hot
  path). Cross-interp evaluation — `interp eval`, an `interp alias` crossing
  in *any* direction (parent→child, child→parent, sibling→sibling routed
  through the shared parent, arbitrary-depth grandchild paths),
  `invokehidden` — is `Vm::in_interp`: a plain nested native call that swaps
  which arena slot is current and swaps back on return, mirroring C's
  `Tcl_EvalObjv(targetInterp, …)` on one shared C stack. This makes the
  interp tree fully re-entrant: a parent-target alias works from any native
  re-entry (coroutine resume, `lsort -command`, trace/event callbacks), and a
  parent alias target may re-enter the child that called it, a sibling, or a
  grandchild, because every interpreter's state stays live in its arena slot
  whether or not it is "current" ([issue #946](https://github.com/bitwisecook/tcl-lsp/issues/946)).
  An `InterpSlot` tracks liveness (`dying`) and an in-flight-evaluation count
  (`active`), so `interp delete` on a currently-executing interpreter defers
  teardown until the last nested call unwinds (C's
  `Tcl_Preserve`/`Tcl_Release`), and deleting a target sweeps cross-interp
  aliases that pointed at it out of their source interpreters. `create
  ?-safe?`/`eval`/`delete`/`exists`/`children`/`issafe`/`marktrusted`/`hide`/
  `expose`/`hidden`/`invokehidden`/`recursionlimit`/`limit` are implemented;
  `-safe` hides the host-reaching commands into a per-interp hidden table;
  `share`/`transfer` are accepted so top-level channel wiring does not abort
  a test file. Remaining gaps: the Safe Base (`safe.tcl` access-path
  virtualisation), `target`, and `debug` beyond the `-frame` switch.
