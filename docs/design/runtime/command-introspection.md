# Command manipulation + introspection

Status: **shipped** in the command-introspection wave.  Extends the
namespace-tree and rename/alias waves with:

- ``interp hide`` / ``interp expose`` / ``interp hidden`` — an
  interpreter-wide hidden-commands table sitting beside the
  namespace tree.
- ``namespace which -command`` / ``namespace which -variable``
  probes, plus ``namespace current``.
- ``info commands ?pattern?`` / ``info procs ?pattern?`` walkers
  that consume the same ``cmd_table`` tree used by dispatch.
- ``info body`` / ``info args`` / ``info default proc arg var``
  for interpreted procs.
- ``OFF_EXPORT_NAME_BUCKET`` sidecar on the ``Command`` struct
  so compiled-proc renames stop needing a special case.

Reference Tcl 9 sources: ``tmp/tcl9.0.3/generic/tclInterp.c``
(``HiddenObjCmd``, ``ExposeObjCmd``, ``Tcl_HideCommand``),
``tmp/tcl9.0.3/generic/tclNamesp.c`` (``Tcl_NamespaceWhichObjCmd``,
``Tcl_GetCommandFullName``), and ``tmp/tcl9.0.3/generic/tclCmdIL.c``
(``InfoCommandsCmd``, ``InfoProcsCmd``, ``InfoDefaultCmd``).

## 1. Scope

In:

- Single-interp ``interp hide`` / ``expose`` / ``hidden`` /
  ``invokehidden``.
- ``namespace which`` (find-only) + ``namespace current``.
- ``info commands`` / ``info procs`` walkers over the ns tree.
- ``info body`` / ``info args`` / ``info default`` for
  interpreted procs.
- ``info level`` (no-arg frame-depth form) and ``info script``
  (empty string — no filesystem source path in the WASM
  sandbox).
- Compiled-proc rename completeness via
  ``OFF_EXPORT_NAME_BUCKET``.

Out (deferred to later waves):

- Command-trace machinery — still no trace infrastructure
  in the runtime.
- ``info level N`` (per-frame argv retrieval) — the zero-arg
  depth form ships here, but the ``info level N`` form would
  require per-frame argv tracking the runtime doesn't keep.
  Callers supplying a numeric level get the tclsh
  ``bad level "N"`` error.
- ``info frame`` — a separate introspection axis (source
  location + stack detail), out of scope for this wave.

Child interpreters moved to
[`child-interp.md`](child-interp.md) — the single-interp scope
here has been lifted, and cross-interp
``interp invokehidden`` / ``hide`` / ``expose`` / ``alias`` now
honour real child paths.

## 2. Hidden commands table

### 2.1 Storage

`interp.rs`
owns the hidden-commands table — one per `Interp`.  Same shape
as a namespace's ``cmd_table`` (12-byte header + 4-byte value =
16-byte bucket).  Qualified hidden names are rejected (they'd
violate the "hidden is a flat namespace" rule).

Pre-child-interp this lived as a module-global in
`namespace.rs`; the move to
per-interp storage is described in
[`child-interp.md`](child-interp.md) §5.

Public API:

| Function | Purpose |
|---|---|
| ``hidden_put(interp, name_ptr, name_len, value)`` | Insert / update. |
| ``hidden_find(interp, name_ptr, name_len) -> u32`` | Lookup; 0 on miss. |
| ``hidden_clear(interp, name_ptr, name_len) -> bool`` | Tombstone. |
| ``hidden_table_buf(interp) / hidden_table_cap(interp)`` | Iterator surface. |

No ``cmd_ref_epoch`` cascade — no namespace paths target the
hidden table, so there's nothing to invalidate downstream.

### 2.2 `interp hide` semantics

`cmd_alias.rs`
drives the move.  ``HideResult`` captures the four outcomes:

| Result | Tclsh message |
|---|---|
| ``ok`` | — |
| ``not_found`` | `unknown command "X"` |
| ``qualified_name_rejected`` | `can't use namespace qualifiers as hidden command token (rename)` |
| ``hidden_name_taken`` | `hidden command named "X" already exists` |

The move:

1. Reject qualified source or qualified hidden name up front.
2. ``ns_cmd_find`` the source; ``hidden_find`` the destination.
3. Deactivate every importer pointing at the source Command
   (same cascade as rename-to-empty).
4. ``hidden_put`` the Command, then ``ns_cmd_clear`` the source
   bucket.
5. Rewrite the Command's stored name slot to the hidden name
   (plain, unqualified — hidden commands have no namespace
   parent).
6. ``lru_invalidate_all`` on the proc-lookup cache.

### 2.3 `interp expose` semantics

`cmd_alias.rs`
is the inverse.  The destination is always the current namespace
(C Tcl raises
`cannot expose to a namespace (use expose to toplevel, then rename)`
for qualified destinations; we match the error string).

1. Reject qualified destination.
2. ``hidden_find`` the source; refuse if absent.
3. Refuse if the destination slot is live
   (``target_exists`` result).
4. ``ns_cmd_put`` the Command in the destination ns; then
   ``hidden_clear`` the hidden slot.
5. Rewrite the Command's stored name slot to
   ``ns_build_fqn(dest_ns, simple_name)`` so dispatchers and
   ``info commands`` see the exposed identity.
6. ``lru_invalidate_all``.

### 2.4 `interp hidden`

``eval_interp_hidden`` in
`interp.rs` walks the
hidden table with a two-pass fill, producing a space-separated Tcl
list of simple names.  Bucket-traversal order is the same shape
``interp aliases`` uses — not stable across grow events but
matches tclsh's `hiddenCmdTable` iteration.

### 2.5 `interp invokehidden`

``eval_interp_invokehidden`` dispatches a hidden command by name
with caller-supplied arguments.  Mirrors
``InvokeHiddenObjCmd`` in ``tclInterp.c`` trimmed to the
single-interp subset:

| Form | Dispatch namespace |
|---|---|
| ``interp invokehidden {} cmd ?arg…?`` | Global (root). |
| ``interp invokehidden {} -global cmd ?arg…?`` | Global (root). |
| ``interp invokehidden {} -namespace ns cmd ?arg…?`` | Resolved ``ns``. |

Combining ``-global`` + ``-namespace`` is rejected with
``cannot use -global option and -namespace option together``
(verbatim Tcl wording).  Missing targets raise ``invalid hidden
command name "X"``.

The implementation looks ``cmd`` up in the interpreter-wide
hidden table (no cmd_table move), builds a fresh argv
``[hidden_name, caller_args…]``, swaps ``current_ns`` to the
target, and recurses through ``eval_proc_call_bucket``.  The
Command's stored name slot is already the hidden name (set by
``hide_command``), so compiled-proc host-bridge dispatch and
interpreted-body calls both work uniformly — no special-casing
of the hidden vs. exposed path.

## 3. ``info commands`` / ``info procs`` walker

`cmd_info.rs` carries
the walker.  Two-pass sizing + filling keeps allocation O(1)
beyond the output buffer — critical because ``tcltest`` calls
``info commands`` on every ``testConstraint`` lookup.

### 3.1 Shared `CmdWalkCtx`

One context struct drives both passes.  Fields:

- ``kind``: ``commands`` (every live command) or ``procs``
  (interpreted-only filter).
- ``pat_ptr`` / ``pat_len`` + ``has_pattern``: the glob pattern
  applied to simple names.  For qualified patterns the prefix
  is consumed by the ns resolver and only the trailing component
  survives as the pattern.
- ``total`` / ``count``: sizing pass accumulators.
- ``buf`` / ``off``: filling pass write cursor (``buf == 0``
  means "we're still sizing").

### 3.2 Filter predicate

``entry_matches`` excludes:

- Dead import redirects (``CMD_IMPORTED`` with
  ``ImportedCmdData.real_cmd == 0`` — post-``namespace forget``).
- Aliases + imports + compiled procs when ``kind == .procs``.
- Procs whose ``body_obj == 0`` when ``kind == .procs`` (matches
  tclsh's "only interpreted procs").
- Entries whose simple name doesn't glob-match when a pattern
  was supplied.

Tombstones (buckets with zero ``OFF_HANDLE``) are skipped
automatically because ``cmd == 0`` short-circuits
``entry_matches``.

### 3.3 Walk shapes

Unqualified patterns (``info commands`` / ``info commands foo*``):

1. ``scan_ns_cmd_table`` of the current ns.
2. Each ``namespace path`` entry (skipping the path entry that
   points back at the context).
3. ``scan_ns_cmd_table`` of the root ns if ``current != root``.

Qualified patterns (``info commands ::foo::*``):

- ``ns_resolve_qualified`` splits the prefix; the resolved
  ``target_ns`` is scanned once with the trailing component as
  the simple pattern.  Misses (resolver yields ``target_ns ==
  0``) short-circuit to an empty list.

Emitted names are the Command's **stored FQN**, not the
``cmd_table`` key.  That's the live identity ``rename`` and
``expose`` maintain, so introspection stays consistent with
``namespace which -command`` and compiled-proc host-bridge
lookups.

### 3.4 Hidden commands are not listed

Hidden-table entries never appear in ``info commands`` — tclsh
treats hidden commands as invisible to the resolver.  The
walker consults only ``Namespace.cmd_table`` and never probes
the hidden side-table.

## 4. Compiled-proc rename: `OFF_EXPORT_NAME_BUCKET`

### 4.1 The old problem

Compiled procs dispatch through the host-bridge
(``tcl_dispatch.dispatch``) by name — the embedder turns a
TclObj string into a wasmtime callable.  That name has to match
the WASM export name, which is set at compile time and can't
change.  Renaming a compiled proc used to preserve the
Command's stored name slot to keep dispatch anchored, at the
cost of ``proc_get_name_ptr`` / introspection parity with the
renamed ``cmd_table`` key.

### 4.2 The sidecar

``Command[36..39]`` (previously reserved) now holds
``OFF_EXPORT_NAME_BUCKET``: a u32 pointer to an 8-byte record
``{ptr: u32, len: u32}`` carrying the registration-time export
name.  ``proc_register_compiled`` populates it on fresh
registrations; re-registrations reuse the existing sidecar so
the name never drifts.  Interpreted procs, aliases, and imports
leave the slot zero.

### 4.3 Dispatch path

``tcl_dispatch.dispatch`` reads
``proc_get_export_name(bucket)`` first.  If the sidecar is
populated (``ptr != 0``) it uses those bytes as the host-bridge
lookup key; otherwise it falls back to
``proc_get_name_ptr`` / ``proc_get_name_len`` (the Command's
live name slot).  This keeps the interpreted-proc path
single-branch — one extra ``read_i32`` on the compiled-proc
hot path is the only cost.

### 4.4 Rename path

``rename_command`` in
`cmd_misc.rs` no
longer special-cases ``func_idx != 0``.  Every Command's live
name slot is rewritten to its new FQN, uniformly.  The sidecar
keeps host-bridge lookups stable for compiled procs; for
interpreted procs the sidecar is zero and nothing touches it.

Observable consequence: ``info commands`` and
``namespace which -command`` report the renamed name for both
compiled and interpreted procs, with no asterisk.

## 5. ``namespace which`` / ``namespace current``

``eval_namespace_which`` dispatches:

- ``namespace which name`` — defaults to ``-command``.
- ``namespace which -command name`` — walks
  ``ns_find_command`` and returns the Command's stored FQN.
  Imports are **not** unwrapped — tclsh's ``namespace which``
  returns the redirect's FQN, not the source's.
- ``namespace which -variable name`` — walks
  ``ns_resolve_qualified`` / ``ns_var_find`` with the same
  primary + alt path probe used for variables elsewhere.  The
  ``namespace path`` is **not** consulted (Tcl's path is
  commands-only).

Every miss returns the empty string — ``namespace which`` is a
probe, not an error-raising lookup.

``namespace current`` returns
``tcl_ns.ns_full_name(ns_current())`` — ``::`` for the root ns,
``::path::to::here`` otherwise.

## 5.1 ``info level`` / ``info script``

``info level`` (no arg) returns the current call-frame depth as
an integer TclObj — thin wrapper around
``tcl_frames.frame_get_depth()``.  Inside a proc body the
depth is ≥1; at the top level it's 0.

``info level N`` would return the argv that entered frame
``N`` in Tcl 9 (``CallFrame.objv``), but our runtime doesn't
retain per-frame argv today.  Callers supplying a numeric
level get the tclsh ``bad level "N"`` error instead of a
fabricated result.  The no-arg form covers the common case
(depth reporting for error messages and tcltest's nesting
checks).

``info script`` returns the empty string — our single-unit
compiled runtime has no filesystem source path to report.
Consumers that use ``info script`` to resolve relative paths
already treat empty as "no location available"
(``testutilities.tcl`` in tcllib is the canonical example).

## 6. ``info default``

``info default proc arg varName`` walks the proc's params
TclObj (a Tcl list where each element is either a bare name or
a ``{name default}`` pair), matches ``arg`` against the name of
each pair, and — on a match with a default present — writes the
default into ``varName`` and returns 1.  On a match with no
default, writes the empty string into ``varName`` and returns 0
(matching Tcl 9's ``InfoDefaultCmd``).

Error shape for non-interpreted-proc targets:

* Missing / alias / hidden / compiled proc (``func_idx != 0``)
  — raises ``"<name>" isn't a procedure`` via the shared
  ``resolve_interpreted_proc`` gate.  Compiled procs don't
  retain their params TclObj (``proc_register_compiled`` clears
  it), so the ``resolve_interpreted_proc`` filter rejects them
  there — they never reach the params-walk branch.
* Arg isn't a parameter of the proc — raises ``procedure "X"
  doesn't have an argument "Y"``.

The implementation operates on the stored params TclObj
directly (``obj.list_element_at``), so it works for any
interpreted proc registered via ``proc_register``.

## 7. Test coverage

Three layers:

1. **Runtime direct tests** drive the primitives through
   dedicated WASM exports (``tcl_test_hide``, ``tcl_test_expose``,
   ``tcl_test_info_commands``, ``tcl_test_info_procs``,
   ``tcl_test_namespace_which``, ``tcl_test_hidden_exists``,
   ``tcl_test_export_name_ptr`` / ``_len``):
   - `tests/runtime/test_tcl_hide.py`
   - `tests/runtime/test_tcl_info.py`
   - `tests/runtime/test_tcl_rename.py::test_rename_compiled_proc_preserves_export_name`

2. **End-to-end Tcl → WASM → runtime tests** in
   `tests/test_wasm_execution.py`:
   - ``TestInterpHideExpose`` — hide/expose/hidden round-trip.
   - ``TestInfoIntrospection`` — ``info commands`` +
     ``namespace which`` end-to-end.

3. **Upstream tests** (``interp.test`` sections 5.* for
   hide/expose, 18.* for aliases; ``info.test`` sections for
   ``info commands`` / ``info procs`` / ``info default``) remain
   scoped out of the ship criteria — the direct + E2E layers
   pin the semantics; upstream coverage is the next natural
   expansion along with child-interp support.

## 8. Known limitations

### 8.1 Compiler direct-call invalidation (resolved)

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

### 8.2 `TestCounterBundle` stays xfail

The tcllib counter-bundle end-to-end (``tests/external/run_tcllib_test.py``)
hits an ``unknown command: test`` trap at counter.test line 4905
— ``::tcltest::test`` isn't reachable from the bundle's
invocation site despite tcltest's stage-1 sourcing completing.
Root cause is the tcltest-init / namespace-path resolver
interaction; unrelated to this wave's rename / alias / hide /
info additions.  The xfail marker's reason string captures the
concrete trap for follow-up.

## 9. Ship summary

- Hidden-command logic in ``cmd_alias.rs``.
- Four new ``namespace.rs`` helpers: ``hidden_put`` /
  ``hidden_find`` / ``hidden_clear`` / ``hidden_table_buf`` +
  ``hidden_table_cap``.
- Rewritten ``cmd_info.rs`` with a real walker + pattern
  parser, plus ``info procs`` / ``info default``.
- New ``OFF_EXPORT_NAME_BUCKET`` slot on ``Command`` + sidecar
  read in ``tcl_dispatch.dispatch``.
- Removed the compiled-proc rename exception from
  ``cmd_misc.rs``.
- New built-in branches in ``interp.rs``:
  ``interp hide`` / ``interp expose`` / ``interp hidden`` /
  ``namespace which`` / ``namespace current``, plus the ``info``
  dispatch wiring for no-arg / three-arg forms.
