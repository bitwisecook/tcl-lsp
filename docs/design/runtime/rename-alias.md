# Runtime rename + interp alias

Status: **shipped** in the rename-alias wave.  Extends the namespace
tree from
[`namespace-tree.md`](namespace-tree.md) with the Tcl 9 semantics for
``rename`` and ``interp alias`` (single-interp scope).  Source of
truth for the C semantics is ``tmp/tcl9.0.3/generic/tclBasic.c``
(``TclRenameCommand``) and ``tmp/tcl9.0.3/generic/tclInterp.c``
(``AliasCreate`` / ``AliasDelete``).

## 1. Scope

In:

- ``rename`` — same-ns, cross-ns, imported-redirect, rename-with-
  importers, rename-to-empty (delete).
- ``interp alias`` — create / query / delete, with frozen prefix
  args.  Single-interp: target interp path is always ``{}``.
- Alias dispatch trampoline wired into the proc-first fast path so
  aliases are transparent to callers.  Resolution is by *stored
  target name* on each dispatch, anchored at the global namespace
  — this lazily observes deletion of the target but does NOT
  follow ``rename`` of the target (the stored name stops
  resolving).  Matches C Tcl's semantics.

Out (deferred):

- Child interpreters + cross-interp aliases (``interp alias child
  newName parent oldName``).  Needs child-interp infrastructure.
- ``interp hide`` / ``interp expose``.  Shipped in the
  command-introspection wave — see
  [`command-introspection.md`](command-introspection.md).
- Command trace invalidation on rename — no trace infra yet.
- Command deletion handlers — same.

## 2. Command struct reuse

The existing 40-byte ``Command`` struct in
[`tcl_procs.zig`](../../../runtime/zig/interp/tcl_procs.zig) already
reserved the ``OFF_PARAMS_OBJ`` slot at offset 12 for type-dependent
payloads:

| flags                     | params_obj payload            |
|---------------------------|-------------------------------|
| (interpreted proc)        | ``TclObj*`` holding params    |
| ``CMD_IMPORTED`` (0x80)   | ``*ImportedCmdData``          |
| ``CMD_ALIAS`` (0x100)     | ``*AliasRec``                 |

The dispatcher discriminates by ``flags``:

- ``proc_lookup`` unwraps ``CMD_IMPORTED`` to the terminal source
  Command before returning — imports stay transparent to callers.
- ``eval_proc_call_bucket`` checks ``CMD_ALIAS`` before falling
  into the generic proc-body path.  Aliases are NOT unwrapped so
  queries like ``interp alias {} foo`` can introspect the redirect.

## 3. ``rename``

### 3.1 Semantics mirror

| Form                              | Behaviour                                                                                     |
|-----------------------------------|-----------------------------------------------------------------------------------------------|
| ``rename old new``                | Move the Command from ``old_ns.cmd_table`` to ``new_ns.cmd_table`` under the simple ``new``.  |
| ``rename old ""``                 | Delete ``old``.  For ``CMD_IMPORTED`` Commands, also splice out of the source's ImportRef list.  For non-imported Commands, walk the Command's ``import_ref_head`` list and deactivate every redirect. |
| ``rename foo foo``                | No-op (self-rename).                                                                          |
| ``rename return X``               | Hardcoded built-in — rejected with ``can't rename "return": built-in command``.               |

### 3.2 Hash-table tombstones

The open-addressed table primitive in [`hash_table.zig`](../../../runtime/zig/valtypes/hash_table.zig)
doesn't support bucket removal (adding tombstones would require
probe-chain rewriting).  Both ``rename`` and ``namespace forget``
use the same trick: keep the bucket populated but zero its
``OFF_HANDLE`` value, so subsequent ``ns_cmd_find`` calls return 0
without breaking the probe chain for adjacent entries.

[`tcl_ns.ns_cmd_clear`](../../../runtime/zig/interp/tcl_ns.zig) is the
canonical entry point; ``rename_command`` uses it to retire the
source name after inserting at the target.

### 3.3 Compiled-proc caveat (retired)

Earlier waves special-cased compiled procs: ``rename_command``
detected ``func_idx != 0`` and skipped the Command's name-slot
update so ``tcl_dispatch.dispatch`` could continue reading the
registration-time export name.  The trade-off was that
``proc_get_name_ptr`` consumers saw the original name while
``info commands`` saw the renamed key — a minor parity gap.

The command-introspection wave replaced the exception with a
sidecar at ``Command[36..39]`` (``OFF_EXPORT_NAME_BUCKET``):
``proc_register_compiled`` stashes the registration-time FQN
there, and the host-bridge dispatcher reads the sidecar first
(falling back to the live name slot when it's zero).  Rename
now rewrites the live name slot uniformly for both interpreted
and compiled procs; the sidecar keeps dispatch anchored.  See
[`command-introspection.md`](command-introspection.md) §4 for
the full sidecar contract.

### 3.4 Built-in protection list

A small hardcoded set is refused: ``return`` and ``error``.
Matches the spirit of Tcl 9's ``TclProtectedCommandsList`` without
wiring trace machinery.  Rationale: tcltest occasionally
``rename error`` as a local shim — the error message is the most
user-visible way to surface "we don't support this yet".

### 3.5 Invalidation

``ns_cmd_put`` and ``ns_cmd_clear`` each bump the target ns's
``cmd_ref_epoch`` and cascade through ``path_source_head``, so
path-based invalidation is covered by the existing P5.3 wiring.
The proc-lookup LRU in ``tcl_procs.zig`` is additionally wiped
wholesale on any rename via ``lru_invalidate_all``; this is coarse
but rename is cold-path enough that the 4-entry wipe is free.

## 4. ``interp alias``

### 4.1 AliasRec layout

```zig
pub const AliasRec = extern struct {
    target_name_ptr: u32,   // FQN of target command (heap-copied)
    target_name_len: u32,
    n_prefix: u32,
    prefix_args_addr: u32,  // packed array of TclObj* (u32 each)
};
```

Stored via ``Command.params_obj`` (offset 12).  Flag bit
``CMD_ALIAS = 0x100`` distinguishes alias redirects from
``CMD_IMPORTED`` (0x80) and interpreted procs (flags = 0).  The
0x100 slot was chosen because no C Tcl Command flag occupies it;
future slots stay free for other redirect types (hide, ensemble,
…) as we need them.

### 4.2 Dispatch

[`tcl_interp.dispatch_alias`](../../../runtime/zig/interp/tcl_interp.zig)
is called from ``eval_proc_call_bucket`` when a resolved Command
has ``CMD_ALIAS`` set.  The trampoline:

1. Synthesises a new argv ``[target_name, *prefix_args, *caller_tail]``.
2. Caps total length at ``parse.MAX_WORDS`` — overlong argv
   triggers an explicit error rather than truncation.
3. Resolves the stored target name on each dispatch, anchored at
   the global namespace (``TCL_EVAL_INVOKE``-style).  By-string
   resolution observes *deletion* of the target lazily (next
   dispatch after ``rename target {}`` raises "unknown command:
   <target>"), but it does NOT track ``rename`` of the target —
   the stored name stops resolving once the Command has moved to
   its new cmd_table key.  Matches C Tcl semantics.
4. Recurses through ``eval_proc_call_bucket`` with the resolved
   target bucket, so compiled-proc / host-bridge / interpreted-body
   paths all work uniformly.

A missing target (deleted after the alias was created) surfaces
as ``unknown command: <target>`` at dispatch time, matching Tcl's
"alias is lazily bound" behaviour.  A cleared alias (``AliasRec``
with ``target_name_len == 0``) surfaces as ``unknown command:
<alias_name>`` so the failure attributes back to the caller's
site rather than the alias target.

### 4.3 Create / query / delete

Built-in dispatcher in ``tcl_interp.eval_interp`` recognises:

| Form                                           | Action                                            |
|------------------------------------------------|---------------------------------------------------|
| ``interp alias {} new {} target ?arg…?``       | Allocate AliasRec, insert CMD_ALIAS Command      |
| ``interp alias {} new``                        | Return ``target ?arg…?`` as a space-separated list |
| ``interp alias {} new {}``                     | Clear AliasRec + ``ns_cmd_clear`` the slot       |
| ``interp aliases {}``                          | Tcl list of every ``CMD_ALIAS`` command in the tree |

Other ``interp`` subcommands (``create``, ``hide``, ``expose``,
``eval``, ``slaves``, …) fall back to the trapping stub in
``tcl_env_stubs.zig`` (``unsupported command: interp``).

### 4.4 Performance

Alias dispatch is warm-path (tcltest's cross-test thunks all hit
it).  Current cost:

- One ``read_i32`` to check ``CMD_ALIAS``.
- One ``ns_find_command`` walk to resolve the target by name.
- One ``memcpy`` of caller argv tail into ``new_words``.
- One recursion into ``eval_proc_call_bucket``.

If ``bench_wasm_runtime.py`` flags the by-name resolution as a
hotspot, the ``AliasRec`` can grow a cached target-Command slot
+ ``target_ns.cmd_ref_epoch`` snapshot for O(1) dispatch on the
fast path.  Not shipped yet — waiting on a real bench regression.

## 5. Compiler surface

The compiler codegen now routes both ``rename`` and ``interp``
through the eval fallback rather than the pre-wired stub table:

- ``rename`` was previously in ``_SCOPE_NOP_COMMANDS`` (compiled as a
  no-op).  Removed — it now emits an ``eval`` call the runtime
  handles via the ``rename`` built-in.
- ``interp`` was previously in ``_CMD_RUNTIME`` pointing at the
  trapping ``tcl_cmd_interp_cmd`` stub.  Removed — the interpreter's
  ``interp`` built-in handles the alias forms; other ``interp``
  subcommands still trap via ``tcl_env_stubs``.

These are the only compiler-side changes in this wave.

## 6. Test strategy

Three layers, each live:

1. **Direct runtime tests** exercise the primitives through dedicated
   WASM exports ``tcl_test_rename`` / ``tcl_test_alias_create``:
   - [`tests/runtime/test_tcl_rename.py`](../../../tests/runtime/test_tcl_rename.py)
   - [`tests/runtime/test_tcl_alias.py`](../../../tests/runtime/test_tcl_alias.py)

2. **End-to-end Tcl → WASM → runtime tests** in
   [`tests/test_wasm_execution.py::TestRenameAndAlias`](../../../tests/test_wasm_execution.py)
   cover the compile-to-WASM + interpret-at-runtime integration.

3. **Upstream tests** (``rename.test``, ``interp.test`` single-
   interp subset) are out of scope for this wave's initial drop —
   the direct + E2E layers pin the semantics; upstream coverage
   is the next natural expansion.

## 7. Ship summary

- Two new Zig modules: ``tcl_rename.zig`` (146 LOC) and
  ``tcl_alias.zig`` (150 LOC).
- One extension to ``tcl_ns.zig``: ``ns_cmd_clear`` helper +
  exposure of ``bump_cmd_ref_epoch`` / ``link_import_ref`` /
  ``unlink_import_ref``.
- New flag bit ``CMD_ALIAS = 0x100`` on ``Command``.
- Wired dispatch trampoline in ``tcl_interp.eval_proc_call_bucket``.
- Wired ``rename`` / ``interp alias`` built-ins in
  ``tcl_interp.zig``.
- Compiler fallback updates in
  ``compiler/codegen/wasm/_imports.py``.
- Zig 0.16 build-compat fixes (``build.zig`` new Module API +
  ``callconv(.C)`` → ``callconv(.c)``).
