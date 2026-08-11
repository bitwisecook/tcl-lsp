# Child interpreters

Status: **shipped** in the child-interpreter wave.  Extends the
namespace-tree / rename-alias / command-introspection waves with
per-interpreter state (root namespace, hidden-commands table,
children registry) and the minimum-viable primitives `interp
create` / `eval` / `exists` / `slaves` / `delete`, plus a promotion
of the previously-shipped single-interp `interp alias` / `hide` /
`expose` / `invokehidden` to honour real child-interpreter paths.

Reference Tcl 9 sources: `tmp/tcl9.0.3/generic/tclInterp.c`
(`ChildCreate`, `ChildEval`, `InterpObjCmd` dispatch,
`OPT_{CREATE,EVAL,EXISTS,SLAVES,DELETE}` branches) and
`tmp/tcl9.0.3/generic/tclBasic.c` (`Tcl_CreateInterp` /
`DeleteInterp`).

## 1. Scope

In:

- `interp create ?-safe? ?--? ?path?` — auto-named and explicit-
  named child creation, including multi-level paths (`interp
  create {a x1}` creates `x1` as a child of `a`).
- `interp eval path script ?script ...?` — concat-with-spaces,
  dispatch inside the resolved child's namespace tree.
- `interp exists ?path?` — pure lookup, non-raising.
- `interp slaves ?path?` / `interp children ?path?` — enumerate
  the resolved interp's direct children.
- `interp delete ?path ...?` — full cascade: mark the target and
  every descendant `INTERP_DELETED`, tombstone every children-
  table bucket along the way, remove the parent's top-level
  `<child>` command, and flush the proc-lookup LRU.
- `interp issafe ?path?` — reads the `INTERP_SAFE` bit.
- Child-as-command dispatch: `<child> eval script` resolves
  through `CMD_INTERP_CHILD` and routes into the child.
- Per-interp `Interp` struct carrying `root_ns`, `hidden_cmd_table`,
  `parent`, `name_*`, `children`, `flags`.
- Path-aware promotion of `interp alias` / `hide` / `expose` /
  `invokehidden`.
- Conservative compiler-side command-binding distrust when registry-declared
  interpreter or command-table transitions cannot be bounded.
- Full tclsh-style error wording: `bad option "X": must be
  alias, aliases, bgerror, cancel, children, create, debug,
  delete, eval, exists, expose, hide, hidden, issafe,
  invokehidden, limit, marktrusted, recursionlimit, share,
  target, or transfer`; `bad option "-X": must be -safe or --`
  for `interp create`.

Out (deferred to later waves):

- `-safe` semantics enforcement.  The flag is recorded on
  `Interp.flags` but file / exec / package / env / load access
  isn't gated — matches the runtime's "no fs / exec anyway"
  stance.
- `interp bgerror` — no event loop to pump background errors.
- `interp limit` / `interp marktrusted` — limit enforcement is
  cooperative in C Tcl and we have no event loop.
- `interp target` — arity validated but the operation surfaces
  the ``unsupported command: interp target`` stub.
- `interp share` / `interp transfer` — channel sharing is
  entangled with the non-existent channel infrastructure.
- `<child> alias ...` / `<child> hide ...` / `<child> eval` is
  shipped; the other per-child subcommands (`invokehidden`,
  `expose`, `aliases`, …) surface as `bad option` errors from
  the child-as-command dispatcher.
- Per-interp call-frame stack.  The shared stack is safe for
  single-threaded, non-nested eval; truly nested `interp eval`
  on a stack-depth-sensitive proc would observe stale depth.
  Flagged here, not fixed.
- Full bump-allocator reclaim.  After `interp delete`, the
  child's `Interp` struct, namespace subtree, cmd tables, and
  hidden table stay live in bump memory — the `INTERP_DELETED`
  flag and parent-bucket tombstone prevent every live code
  path from reaching them, but the bytes aren't freed.

## 2. The `Interp` struct

`interp.rs`
owns the one-Interp-per-interpreter state.  The Rust runtime
realises this as the `Interp(Rc<InterpState>)` handle (§13); the
field table below is the design model each field maps onto.

| Field | Owns |
|---|---|
| `root_ns` | This interp's root (`::`) `Namespace*` — for the root interp this equals `tcl_ns.root_addr`; for children, a fresh namespace allocated via `tcl_ns.ns_alloc_root`. |
| `hidden_cmd_table` | Interpreter-wide hidden table (used to live as a module-global `tcl_ns.hidden_cmd_table`; moved here so cross-interp `hide` / `expose` can target the right interp). |
| `parent` | Parent `Interp*` — zero only for the root. |
| `name_ptr` / `name_len` | Simple name in the parent's `children` table (zero-length for root). |
| `children` | Registry of direct children — key = simple name, value = `Interp*`. |
| `flags` | `INTERP_SAFE` (`0x1`) and future flag bits. |

The singleton is lazily allocated the first time `interp_root()`
is called.  The root interp adopts `tcl_ns.ns_root()` as its
`root_ns` so the very first access — from anywhere in the runtime
— gets the same `Namespace*` it always did.  `current_interp` is
set at the same moment so `interp_current()` always returns a
valid address.

## 3. Entering / leaving a child

`interp eval` (and every other cross-interp dispatch path —
`invokehidden`, cross-interp alias create / query / dispatch)
goes through:

```
const save = interp_reg.enter(target);
defer interp_reg.leave(save);
// ... eval in child ...
```

`enter` saves the previous `current_interp`, `tcl_ns.root_addr`,
and `tcl_ns.current_ns` into an `EnterSave` record, then swaps
the target's slots in.  `leave` restores the trio.  Nesting —
`interp eval child1 { interp eval child2 {...} }` — unwinds
correctly because each `enter` captures the state that was live
when it ran, not the root-level state.

`tcl_ns.root_addr` is the mechanism the rest of the runtime uses
to see the correct root namespace: every caller of `tcl_ns.ns_root()`
reads this global.  Swapping it means `proc_register`,
`global_set` / `global_get`, `info commands`, and every other
path that reaches into the root namespace automatically lands in
the child's tree — no per-call routing through an "interp
context" argument.

## 4. Resolving paths

`resolve_path(base, path_ptr, path_len)` parses the path bytes as
a Tcl list via `obj.list_count_elements` / `list_element_at` and
walks the chain of children starting from `base`.  Empty path
returns `base` unchanged; any missing component returns 0.

Call sites (`eval_interp_hide`, `eval_interp_expose`,
`eval_interp_invokehidden`, `eval_interp_eval`, …) wrap that with
`resolve_interp_path(words[path_idx])` which also raises the
tclsh error `could not find interpreter "X"` on miss.

## 5. Per-interp hidden commands

Previously the hidden table lived as a module-global in
`namespace.rs` — one table for the whole runtime.  That matched the
single-interp scope but broke the moment we needed cross-interp
hide / expose.

Post-child-interp:

- The `hidden_cmd_table: HiddenTable` field is embedded inside
  `Interp`.
- `hidden_put` / `hidden_find` / `hidden_clear` / `hidden_table_buf` /
  `hidden_table_cap` now take an explicit `interp_addr`
  parameter.  Callers that used to pass implicit single-interp
  state now pass `interp_reg.interp_current()` (e.g. the
  `tcl_test_hide` / `tcl_test_expose` scaffolding exports).
- `tcl_hide.hide_command` / `expose_command` gained a leading
  `target_interp` argument.
- `eval_interp_hide` / `_expose` / `_hidden` / `_invokehidden`
  resolve the path argument and route to the target interp's
  slot.

## 6. Cross-interp alias

`interp alias` takes two paths:

1. `childPath` — where the alias source is registered.
2. `parentPath` — where the target command lives at dispatch.

For single-interp (both paths empty), both collapse to the
current interp.  For cross-interp, the alias redirect Command is
inserted into `childPath`'s cmd_table, and the parent interp is
stashed in `AliasRec.parent_interp`.

`dispatch_alias` reads that slot on every invocation: when non-
zero and different from the current interp, it calls
`interp_reg.enter(parent_interp)` before `proc_lookup` so the
target resolves against the parent's cmd_table, then
`interp_reg.leave` restores on return.

`parent_interp` used to live in the Command's
`OFF_IMPORT_REF_HEAD` slot on the theory "aliases never have
importers so the slot is free" — but that assumption doesn't
hold in the presence of `namespace import` of an alias: the
importer-list tracking in `link_import_ref` would clobber the
stashed `Interp*`.  Moving the handle onto `AliasRec` keeps
the Command's import-machinery slots independent.

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

When cross-interp dispatch happens (a child alias's target
resolves via `proc_lookup` in the parent, or a cross-interp
`invokehidden` reaches into a child's hidden table), two module-
globals move in lockstep:

1. `current_interp` (the registry's tracker).
2. `tcl_ns.root_addr` (the ns tree's root pointer).

Both are swapped by `enter` and restored by `leave`.  The
proc-lookup LRU is keyed on `(ns, hash, len, first_byte)`, so
cached entries from the parent interp automatically miss when
the child interp's `ns` pointer differs.  No cross-interp
cache poisoning is possible without a deliberate
`tcl_ns.root_addr` rewrite that skips the `enter` / `leave` pair.

## 9. Deletion cascade

`interp delete path` runs the full teardown:

1. **Recursive descent** — `mark_deleted_subtree` walks the
   children table depth-first, flagging every descendant
   `INTERP_DELETED` and tombstoning the grandchildren buckets
   along the way.
2. **Top-level command removal** — the parent's `<child>`
   Command (registered with `CMD_INTERP_CHILD` at
   `interp create` time) is cleared via `ns_cmd_clear`, so a
   post-delete `<child> eval ...` surfaces as a clean
   "unknown command" from the normal dispatch path.
3. **Parent-registry tombstone** — `child_delete` writes 0 into
   the parent's children-table bucket value slot so
   `resolve_path` / `child_lookup` miss.
4. **LRU flush** — proc-lookup cache is invalidated so stale
   cross-interp entries don't linger.

Cross-interp aliases (parent-side alias whose target lives in
the deleted child) stash the deleted interp's `Interp*` in the
redirect Command's `OFF_IMPORT_REF_HEAD`.  `dispatch_alias`
reads it on every invocation and, when `is_deleted(parent) ==
true`, raises "unknown command: <target>" without entering the
dead interp.  This matches the observable behaviour of "target
of an alias got renamed away" which `dispatch_alias` already
handles cleanly.

What's *not* part of the cascade: bump-allocator reclaim of the
`Interp` struct, its namespace subtree, its command table
buckets, and its hidden-commands table.  Those byte regions stay
live in linear memory — the bump allocator's "never frees"
contract hasn't changed.  Every live dispatch path consults the
`INTERP_DELETED` flag and/or the parent's tombstoned bucket
before routing, so the unfreed bytes are observationally unreachable
from Tcl.

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

Three layers:

1. **Runtime direct tests** drive the registry primitives through
   dedicated WASM exports (`tcl_test_interp_create`,
   `tcl_test_interp_lookup`, `tcl_test_interp_delete`,
   `tcl_test_interp_eval_script`, `tcl_test_hidden_find_in`):
   - `tests/runtime/test_tcl_interp_children.py`
2. **End-to-end Tcl → WASM → runtime tests**:
   - `TestInterpChildren` in
     `tests/test_wasm_execution.py`
     covers create + eval + exists + slaves + delete, plus the
     cross-interp promotions for `alias` / `hide` /
     `invokehidden`.
   - `TestInterpTestPort` in the same file hand-ports
     ``tmp/tcl9.0.3/tests/interp.test`` sections 1.* / 2.* /
     3.* / 4.* / 5.* / 6.* (options, create, exists, children,
     delete, consistency, eval).  Upstream bundle compilation
     is still blocked by tcltest features unrelated to this
     wave (``::tcltest::normalizePath`` etc.), so the
     individual ``test interp-N.M`` bodies are ported
     verbatim instead.
3. **Upstream `interp.test` whole-file bundle** — blocked by
   tcltest harness features outside the child-interp scope.
   Revisit once the surrounding primitives land.

## 12. Ship summary

- Child-interp registry in `interp.rs` carrying the
  `Interp` struct + `enter` / `leave` + `alloc_child_command` +
  recursive delete helpers.
- `namespace.rs` changes: `root_addr` made public for swap; new
  `ns_alloc_root` helper; hidden-table surface removed (moved
  into the registry).
- `cmd_alias.rs` API gained a leading `target_interp` argument.
- `cmd_proc.rs` gained `CMD_INTERP_CHILD` (`0x200`).
- `interp.rs` gained `create`, `eval`, `exists`,
  `slaves` / `children`, `delete`, `target`, `issafe` branches;
  path promotion for `hide` / `expose` / `hidden` /
  `invokehidden` / `alias`; `dispatch_interp_child` for the
  child-as-command path; `emit_bad_option` for tclsh-parity
  error wording.
- `dispatch_alias` reads the parent-interp slot on the alias
  Command, gates on `is_deleted`, and `enter` / `leave`s around
  the lookup.
- Registry-declared interpreter and command-table transitions feed the common
  command-binding proof; literal mutations distrust affected names, and
  unbounded mutations distrust every binding.
- New test scaffolding exports:
  `tcl_test_interp_root` / `_current` / `_create` / `_lookup` /
  `_delete` / `_eval_script` / `_root_ns` / `_hidden_find_in`.
- Upstream `interp.test` sections 1–6 ported verbatim as
  `TestInterpTestPort` (59 cases, all passing).

## 13. Rust ports

The two Rust execution engines implement the same contract, mirroring the
design above:

- **`runtime/rust`** (tree-walk runtime — native and wasm32-wasip1) carries the
  faithful port: `Interp(Rc<InterpState>)` with a `children` map, `hidden`
  table, `parent` `Weak`, and `is_safe` flag, the `enter`/`leave`-style
  `with_child` re-entrancy guard, and cross-interp aliases (`§6`). `interp
  create`/`eval`/`delete`/`exists`/`children`/`issafe`/`marktrusted`/`hide`/
  `expose`/`hidden`/`invokehidden`/`recursionlimit` are implemented; the Safe
  Base (`safe.tcl` access-path virtualisation) and `interp limit`/`target`/
  `debug`/`share` are the remaining gaps.

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

Both are gated against the upstream `interp.test`/`safe.test` suites through the
[tier scoreboard](rust-vm-tier-parity.md); child interpreters are the
[Tier 7](tcl-test-tiers.md) feature.
