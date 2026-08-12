# Runtime namespace tree

The namespace tree in the WASM runtime (`runtime/rust/src/namespace.rs`):
parent/child links, per-namespace command and variable tables, explicit path
and export lists — so that command and variable resolution matches Tcl 9
semantics. Section numbers here are cited from the runtime source
(`namespace.rs`, `cmd_namespace.rs`, `cmd_var.rs`, `vars.rs`, `frame.rs`), so
keep them stable.

The *contract* the tree implements is
[`../contracts/command-resolution.md`](../contracts/command-resolution.md)
(commands, conformance-vector gated) and
[`../contracts/runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md)
(variables).

Source of truth for the C semantics we're mirroring is Tcl 9.0.3 in
`tmp/tcl9.0.3/.git` — read with `git -C tmp/tcl9.0.3 show HEAD:generic/<file>`.
All C-file citations in this doc use that source.

## 1. Goal & non-goals

### Goal

Give the runtime a real namespace tree — parent / child links, per-
namespace command and variable tables, explicit path and export lists
— so command and variable resolution matches Tcl 9 semantics
(`tclNamesp.c:Tcl_FindCommand`, `tclVar.c:TclObjLookupVarEx`) instead
of the current FQN-string fallbacks in `runtime/rust/src/cmd_proc.rs`
(suffix-scan in `proc_lookup`) and `runtime/rust/src/namespace.rs`
(flat FQN-keyed hash; globals live in the root namespace's variable
table).

Correctness first: a tcltest-shaped bundle (`proc $varName {args}
body` created inside a factory, then invoked by FQN or unqualified
name, possibly through `namespace import`, possibly from inside
`namespace eval`) should resolve the same way `tclsh 9.0` does.

### Non-goals

- **Not** a general OO / Itcl scaffold. We cover only the `namespace`
  built-in, `global`, `variable`, `upvar`, and the resolution paths
  those use.
- **Not** any bytecode-level caching (`ResolvedCmdName`,
  `resolverEpoch` on compiled bodies). Our bodies are either compiled
  to WASM (resolution happens at lowering time) or re-parsed per call
  (the parse cache). The `cmdRefEpoch` analogue is purely for path-cache
  invalidation of runtime lookups — no bytecode invalidation.
- **Not** custom resolvers. Ensembles, traces, safe interpreters, and
  namespace deletion are all implemented, but elsewhere — see §4 for where.
- **Not** a change to the compiler's FQN mangling. Compiled procs keep their
  `::ns::name` mangled symbols; the tree is what `proc_lookup` walks at
  runtime.

## 2. C Tcl 9 reference model

Only the fields that shape *resolution* and *storage* are called out
here.  Refcounting, traces, deletion handlers, resolver plug-ins, and
ensembles are listed in §4 and skipped from the Rust mirror.

### `Namespace` (`tclInt.h:271`)

Per-namespace storage and metadata.  Fields we mirror:

| Field | Type | Purpose |
|---|---|---|
| `name` | `char *` | simple (unqualified) name; `""` for root |
| `fullName` | `char *` | `::`-prefixed FQN |
| `parentPtr` | `Namespace *` | enclosing ns; `NULL` for root |
| `childTable` | `Tcl_HashTable` | simple-name → `Namespace *` children |
| `cmdTable` | `Tcl_HashTable` | simple-name → `Command *`, including imported redirect entries |
| `varTable` | `TclVarHashTable` | simple-name → `Var *` for ns-scoped variables |
| `exportArrayPtr` + `numExportPatterns` / `maxExportPatterns` | `char **` + `Tcl_Size` | `namespace export` patterns matched by `namespace import` |
| `cmdRefEpoch` | `Tcl_Size` | bumped when a cmd add/delete invalidates cached path lookups |
| `commandPathArray` + `commandPathLength` | `NamespacePathEntry *` + `Tcl_Size` | ordered `namespace path` search list |
| `commandPathSourceList` | `NamespacePathEntry *` | back-list for invalidation when `self` changes |
| `flags` | `int` | `NS_DYING` / `NS_DEAD` / `NS_TEARDOWN` |

Fields we skip (tracked in §4): `clientData`, `deleteProc`,
`earlyDeleteProc`, `nsId`, `interp`, `activationCount`, `refCount`,
`resolverEpoch`, `cmdResProc`, `varResProc`, `compiledVarResProc`,
`exportLookupEpoch`, `ensembles`, `unknownHandlerPtr`.

### `NamespacePathEntry` (`tclInt.h:396`)

One ordered entry in a namespace's `commandPathArray`.  Two pointers
each:

- `nsPtr` — target ns (may be `NULL` if invalidated).
- `creatorNsPtr` — ns whose path contains this entry.
- `prevPtr` / `nextPtr` — doubly-linked list, hanging off the target
  ns's `commandPathSourceList`.  This is the back-list that lets a
  ns invalidate every path entry pointing *at* it when its own
  commands change.

### `Command` (`tclInt.h:1837`)

One per proc / built-in / imported redirect.  Mirror fields:

| Field | Type | Purpose |
|---|---|---|
| `nsPtr` | `Namespace *` | home ns (entry lives in `nsPtr->cmdTable`) |
| `objProc` | `Tcl_ObjCmdProc *` | C entry point (WASM: runtime function index) |
| `objClientData` | `void *` | for imports: `ImportedCmdData *`; for compiled procs: our `proc_buf` bucket base |
| `importRefPtr` | `ImportRef *` | head of back-list of importing cmds |
| `flags` | `int` | `CMD_DYING` / `CMD_DEAD` / `CMD_VIA_RESOLVER` / `CMD_REDEF_IN_PROGRESS` |

Skipped: `hPtr` (we key by bucket, not hash-entry pointer), `refCount`,
`cmdEpoch`, `compileProc` (our compile is ahead-of-time), `proc`
(string-based), `clientData` (string-based), `deleteProc` + `deleteData`,
`tracePtr`, `nreProc`.

### `ImportRef` + `ImportedCmdData` (`tclInt.h:1804`, `:1823`)

Pair that implements `namespace import`:

- `ImportedCmdData { realCmdPtr, selfPtr }` — the `objClientData` of
  the redirect command in the importing ns.  `realCmdPtr` is the
  source, `selfPtr` is the redirect itself (needed to splice out of
  the back-list on delete).
- `ImportRef { importedCmdPtr, nextPtr }` — singly-linked list anchored
  on `realCmdPtr->importRefPtr`.  Each node points back at a redirect
  so deleting the source can walk the list and remove every redirect.

### `Var` (`tclInt.h:637`)

Union by `flags`:

- Default (scalar): `value.objPtr` is a `Tcl_Obj *`.
- `VAR_ARRAY`: `value.tablePtr` is a `TclVarHashTable *` of element vars.
- `VAR_LINK`: `value.linkPtr` is another `Var *` (target).  Set by
  `global`, `variable`, `upvar`, and by the namespace machinery when
  a local name resolves to a namespace-scoped var.

Flag bits we care about (all from `tclInt.h:757-790`):

| Flag | Value | Meaning |
|---|---|---|
| `VAR_ARRAY` | `0x1` | `value.tablePtr` is valid |
| `VAR_LINK` | `0x2` | `value.linkPtr` is valid — follow it |
| `VAR_IN_HASHTABLE` | `0x4` | entry lives in a ns varTable or array element table |
| `VAR_DEAD_HASH` | `0x8` | hash entry already removed |
| `VAR_CONSTANT` | `0x10000` | writes rejected (TIP 645) |
| `VAR_NAMESPACE_VAR` | `0x80` | declared via `variable`; persists across resets |
| `VAR_ARRAY_ELEMENT` | `0x1000` | entry is one element of an array |

Skipped: every `VAR_TRACED_*` bit, `VAR_SEARCH_ACTIVE`, `VAR_RESOLVED`,
`VAR_ARGUMENT`, `VAR_TEMPORARY`, `VAR_IS_ARGS` (compiled-local bits —
the Rust runtime handles locals via frame-local alias slots, not
`Var` structs).

### `CallFrame` (`tclInt.h:1275`)

Per-proc-invocation frame.  Our existing `runtime/rust/src/frame.rs`
already covers the local-var + alias slice (`ALIAS_GLOBAL`,
`ALIAS_EXT` descriptors stand in for `VAR_LINK`).  The one field we
carry is `nsPtr` — which namespace is "current" while this
frame is on the stack.  Today we simulate it with `ns_set`/`ns_restore`
around an FQN string; the tree saves / restores a `Namespace *`.

### Key resolution entry points (`tclNamesp.c`)

- `TclGetNamespaceForQualName` (`:2272`) — given a (possibly
  qualified) name + context ns, walk child tables until we hit the
  simple name.  Returns `(containing_ns, simple_name)` pair *plus* an
  alternate "search-from-global" pair.  This is what `ns_resolve_qualified`
  in §5 mirrors.
- `Tcl_FindCommand` (`:2631`) — unqualified lookup order.  This is
  what `proc_lookup` resolves through.
- `Tcl_Export` (`:1454`) — record patterns on `exportArrayPtr`.
- `Tcl_Import` / `DoImport` (`:1653` / `:1793`) — walk source
  `cmdTable`, match patterns, create redirect cmd in importer,
  splice an `ImportRef` onto source's `importRefPtr` list.
- `Tcl_ForgetImport` (`:1939`) — inverse of import; used by
  `namespace forget`.
- `TclSetNsPath` (`:4213`) — populate `commandPathArray` + link
  each new entry onto the target's `commandPathSourceList` for
  invalidation.

## 3. Runtime analogue

The design model below is what the Rust runtime realises in
`runtime/rust/src/namespace.rs` (`Namespace` / `Namespaces`),
`cmd_proc.rs`, and `frame.rs`.  The original sketch was written against
a WASM-linear-memory allocator that never frees, with every sub-table a
12-byte-header hash table; treat the `u32` handles below as the design's
addressing model (the Rust port uses owned data structures for the same
shape).

### `Namespace` struct

```
// design realised in runtime/rust/src/namespace.rs

// child_table, cmd_table, var_table are hash tables keyed by simple name

/// child_table, cmd_table, var_table bucket sizes.  All three use the
/// same 12-byte header + 4-byte value (a u32 handle into
/// ns_arena / cmd_arena / var_arena).  Keeping them all 16 bytes means
/// the three ``Table(16)`` instantiations share the same layout.
const NS_BUCKET_SIZE: u32 = 16;
const ChildTable = ht.Table(NS_BUCKET_SIZE);
const CmdTable = ht.Table(NS_BUCKET_SIZE);
const VarTable = ht.Table(NS_BUCKET_SIZE);

pub const Namespace = extern struct {
    /// Simple (unqualified) name.  Pointer + length into ns_arena.
    /// Zero-length for the root ns.
    name_ptr: u32,
    name_len: u32,

    /// ``::``-prefixed FQN, lazily materialised from the parent chain
    /// on first read.  Zero until computed.
    full_name_ptr: u32,
    full_name_len: u32,

    /// Enclosing namespace.  Zero for root.  All other Namespace*
    /// fields store absolute byte addresses (u32) matching our
    /// ``alloc`` contract in obj.rs.
    parent: u32,

    /// Sub-tables.  Each Table manages its own backing buffer.
    child_table: ChildTable,
    cmd_table: CmdTable,
    var_table: VarTable,

    /// ``namespace export`` patterns.  Pointer into ns_arena to an
    /// array of (pattern_ptr, pattern_len) u32 pairs.  `namespace export` fills
    /// this in.  Zero while unused.
    export_patterns: u32,
    export_pattern_count: u32,

    /// ``namespace path`` — array of Namespace* (u32).  the `namespace path` builtin fills
    /// this in.  Zero while unused.
    path_array: u32,
    path_len: u32,

    /// Bumped whenever a command is added, deleted, or imported into
    /// this ns.  Path lookups cache ``(target_ns, target_ns.cmd_ref_epoch)``
    /// and re-resolve when the recorded epoch no longer matches.
    cmd_ref_epoch: u32,

    /// Head of the back-list of ``NamespacePathEntry`` nodes whose
    /// target is this ns.  When cmd_ref_epoch bumps, we walk this
    /// list to bump every dependent namespace's epoch too.
    path_source_head: u32,

    /// NS_DYING | NS_DEAD | NS_TEARDOWN.  P1 leaves this at 0 (no
    /// deletion path yet).
    flags: u32,
};
```

Child / cmd / var bucket payload is a single `u32` that is one of:
- a `*Namespace` handle (child table)
- a `*Command` handle (cmd table)
- a `*Var` handle (var table)

### `Command` struct

```
pub const Command = extern struct {
    /// Home namespace.  Zero only during construction.
    ns: u32,

    /// For compiled procs: the bucket base in cmd_proc.rs's
    /// ``proc_table``.  For imported redirects: a ``*ImportedCmdData``
    /// (discriminated by ``flags & CMD_IMPORTED``).
    client_data: u32,

    /// WASM function index (compiled procs) or 0 (interpreted / imported).
    func_idx: u32,

    /// Head of the back-list of importers.  ImportRef node chain.
    import_ref_head: u32,

    /// CMD_DYING | CMD_DEAD | CMD_IMPORTED.  We reuse the C bit values
    /// for ease of porting; CMD_IMPORTED is our synthetic flag in the
    /// unused-by-C 0x80 slot.
    flags: u32,
};

pub const CMD_IMPORTED: u32 = 0x80;
pub const CMD_DYING: u32 = 0x01;
pub const CMD_DEAD: u32 = 0x40;
```

### `Var` struct

```
pub const Var = extern struct {
    /// VAR_ARRAY | VAR_LINK | VAR_CONSTANT | VAR_NAMESPACE_VAR etc.
    /// Matches C bit values (tclInt.h:757-790).
    flags: u32,

    /// Tagged by flags.  For scalars: TclObj handle.  For VAR_ARRAY:
    /// *ArrayVarTable (reuses hash_table.Table(16)).  For VAR_LINK:
    /// absolute address of target Var.
    value: u32,
};

pub const VAR_ARRAY: u32 = 0x1;
pub const VAR_LINK: u32 = 0x2;
pub const VAR_IN_HASHTABLE: u32 = 0x4;
pub const VAR_NAMESPACE_VAR: u32 = 0x80;
pub const VAR_ARRAY_ELEMENT: u32 = 0x1000;
pub const VAR_CONSTANT: u32 = 0x10000;
```

### `ImportRef` + `ImportedCmdData`

```
pub const ImportedCmdData = extern struct {
    real_cmd: u32,   // *Command — the source
    self_cmd: u32,   // *Command — the redirect (this command)
};

pub const ImportRef = extern struct {
    imported_cmd: u32, // *Command — redirect in importing ns
    next: u32,         // *ImportRef — singly-linked list head at realCmd.import_ref_head
};
```

### `NamespacePathEntry`

```
pub const NamespacePathEntry = extern struct {
    target_ns: u32,    // *Namespace — what we resolve through
    creator_ns: u32,   // *Namespace — whose path_array[i] this is
    prev: u32,         // *NamespacePathEntry — doubly-linked on target_ns.path_source_head
    next: u32,
};
```

### Call frames

`runtime/rust/src/frame.rs` gets one new field appended to its
per-frame header (outside the bucket array): `ns: u32` — the
`*Namespace` that was current when the frame was pushed.  The frame header carries
this; until then frames carry no ns pointer and the current-ns state
lives in the compiler-emitted `tcl_ns_set` / `tcl_ns_restore` global.

## 4. Structures C carries that this tree does not

"Omit" means the structure is unreachable from the supported surface, not that
it is pending. Where behaviour C attaches to one of these structures *is*
implemented, it is implemented differently — the pointer below says where.

### Namespace lifecycle bookkeeping (**omit the bookkeeping, not the behaviour**)

C Tcl uses `refCount`, `activationCount`, `NS_DYING` / `NS_DEAD`,
`deleteProc`, `earlyDeleteProc`, `VarInHash.refCount`, `Command.refCount`, and
`Command.cmdEpoch` to sequence a namespace teardown that can be re-entered by
the very code it is tearing down. `flags` is carried on `Namespace` and
`Command` for structural parity, but the `NS_DYING` / `CMD_DYING` bits stay
zero.

`namespace delete` **is** implemented (`cmd_namespace.rs::ns_delete`, mirroring
C's `NamespaceDeleteCmd`): it deletes each named namespace with its children,
commands, and variables, errors on a missing namespace, and destroys any object
living in that namespace — running its destructor while the namespace is still
intact, as C's `ObjectNamespaceDeleted` does. Deletion goes **by namespace id**
rather than by name, specifically so variable unset traces fire as the
namespace is torn down.

### Variable and command traces

Implemented, but not as `VAR_TRACED_*` bits on the cell. Both runtimes keep
interpreter-level trace tables keyed by the *resolved* variable or command
identity and fire from their access and dispatch chokepoints — see
[`trace-implementation.md`](trace-implementation.md) and the firing contract in
[`../contracts/variable-trace-dispatch-and-introspection.md`](../contracts/variable-trace-dispatch-and-introspection.md).

### Ensembles

Implemented in `runtime/rust/src/ensemble.rs` — the `ens sub …` → target
redirect with `-map`, `-prefix`, `-subcommands`, and `-unknown`, modelled on
C's `tclEnsemble.c`. `EnsembleConfig` and the subcommand-resolution and
error-wording rules are the pure half; the dispatch trampoline lives on the
interpreter.

### Child and safe interpreters

Implemented — see [`child-interp.md`](child-interp.md) for the per-interpreter
state (root namespace, hidden-command table, children registry) and
[`rename-alias.md`](rename-alias.md) for `interp alias`. The interpreter
carries a `hidden` command table, which is what a safe interpreter hides the
dangerous commands into.

### Per-namespace `unknown` (`unknownHandlerPtr`)

Implemented — `namespace unknown` is a real subcommand
(`cmd_namespace.rs::ns_unknown`), and handlers are per-namespace and **not**
inherited by children; the global namespace's handler is the interpreter-wide
default and beats the plain `::unknown` proc. See
[`../contracts/command-resolution.md`](../contracts/command-resolution.md).

### Custom resolvers (**omit**)

`Tcl_SetNamespaceResolvers`, `Tcl_AddInterpResolver`, `cmdResProc`,
`varResProc`, `compiledVarResProc`, `ResolverScheme`,
`resolverEpoch`.  No test in the current corpus uses them.  If we
need them later the hook point is at the top of `ns_find_command`
before the context-ns table check — same position as C's
`Tcl_FindCommand:2678`.

### Compiled-local `Var`s (**omit by construction**)

The `VAR_ARGUMENT` / `VAR_TEMPORARY` / `VAR_IS_ARGS` / `VAR_RESOLVED`
flags describe compiled procedure locals, which in C live in a
per-frame `Var *` array.  Our WASM-compiled procs use native
locals; interpreted procs use `frame.rs` alias slots.  Neither
goes through a `Var` struct, so these bits are never read or
written.

### `nsId` / `interp` (**omit**)

`nsId` is a monotonic counter used by debug tooling and `[namespace code]`
serialisation; it is regenerable from the `parent` chain plus the name, and has
no consumer here.  `interp` is the enclosing `Tcl_Interp *`; the runtime's
per-interpreter state is held on the interpreter itself
([`child-interp.md`](child-interp.md)), not on each namespace.

### `resolverEpoch` / bytecode invalidation (**omit**)

C Tcl bumps this when resolution rules change so existing bytecode
re-compiles.  Our compiled code is AOT WASM — recompilation happens
by re-running the toolchain, not at runtime.

### `clientData` / `deleteProc` on namespaces (**omit**)

User-ns state hooks.  No in-tree user.

### `exportLookupEpoch` (**omit**)

Cache-coherence counter for TIP-112 `info commands` filtering.  We
don't implement TIP-112 enumeration at runtime; `info commands` goes
through `each()` on the cmd table.

### `commandPathSourceList` on the *source* ns (**partial**)

We keep the back-list but use it only for `cmd_ref_epoch`
invalidation, not for actual path-entry rewriting.  Path entries
point at their target by `u32` address and the target is
never freed, so there's no dangling-pointer problem to solve.

## 5. Resolution algorithms

Two primitives, both modelled on `tclNamesp.c`.  Pseudocode uses
snake_case names; treat `*Namespace` / `*Command` as `u32`
handles throughout.

### 5.1 `ns_resolve_qualified(cxt, name) -> (target_ns, simple_name, alt_ns)`

Mirror of `TclGetNamespaceForQualName` (`tclNamesp.c:2272`).  Given
a possibly-qualified name like `::a::b::cmd` or `a::b::cmd` or just
`cmd`, return the *containing* namespace and the simple trailing
name.  The `alt_ns` slot is the C code's "alternate search from
global" — populated only when `cxt != root` and the name is not
`::`-anchored.

```
fn ns_resolve_qualified(cxt: *Namespace, name: []const u8)
    -> (target_ns: *Namespace, simple_name: []const u8, alt_ns: ?*Namespace)
{
    // 1. Anchor.  ``::``-prefixed starts at root; else at cxt.
    var ns = cxt;
    var s = name;
    if (s starts with "::") {
        ns = ns_root();
        s = strip_leading_colons(s);          // skip all subsequent ':'
        if (s.len == 0) {
            return (ns_root(), "", null);     // ``::`` alone is root itself
        }
    }
    var alt: ?*Namespace = if (ns == ns_root()) null else ns_root();

    // 2. Walk child tables.  Each iteration consumes one ``::``
    //    separator; the trailing component becomes simple_name.
    while (true) {
        const (head, rest) = split_once(s, "::");
        if (rest == null) {
            return (ns, head, alt);           // head is the simple name
        }
        // Descend one level in the primary path.
        ns = ns.child_table.find(head) orelse return (null, head, null);
        // Mirror the descent on the alt path — but only while alt is
        // still valid.  Once alt misses, drop it.
        if (alt) |a| {
            alt = a.child_table.find(head);
        }
        s = rest;
    }
}
```

Notes:
- The C code also has flags (`TCL_FIND_ONLY_NS`, `TCL_CREATE_NS_IF_UNKNOWN`,
  `TCL_GLOBAL_ONLY`, `TCL_NAMESPACE_ONLY`).  We expose `find-only` vs
  `create` via two wrappers (`ns_resolve_qualified` / `ns_resolve_qualified_creating`)
  rather than a flag word.
- We drop `altNsPtrPtr` mid-walk as soon as the alt path diverges,
  matching `Tcl_FindCommand`'s "alt only useful if it still points
  somewhere real" contract.

### 5.2 `ns_find_command(cxt, name) -> ?*Command`

Mirror of `Tcl_FindCommand` (`tclNamesp.c:2631`).  This is the
function `proc_lookup` in `cmd_proc.rs` becomes once P2/P5 are
done.

```
fn ns_find_command(cxt: *Namespace, name: []const u8) -> ?*Command {
    // A custom-resolver hook would go here (not implemented — §4).

    // A. If qualified OR name starts with ``::``, do the FQN walk.
    if (name starts with "::" or name contains "::") {
        const (target, simple, alt) = ns_resolve_qualified(cxt, name);
        if (target) |t| {
            if (t.cmd_table.find(simple)) |c| return c;
        }
        if (alt) |a| {                        // only when cxt != root
            if (a.cmd_table.find(simple)) |c| return c;
        }
        return null;
    }

    // B. Unqualified: context ns first.
    if (cxt.cmd_table.find(name)) |c| return c;

    // C. commandPathArray walk.
    for (entry of cxt.path_array[0..cxt.path_len]) {
        const t = entry.target_ns orelse continue;
        if (t.cmd_table.find(name)) |c| return c;
    }

    // D. Root ns fallback.
    if (cxt != ns_root()) {
        if (ns_root().cmd_table.find(name)) |c| return c;
    }
    return null;
}
```

Per-step C precedent:
- A: `Tcl_FindCommand:2668-2790` — the `commandPathLength!=0` branch
  plus the else-branch that does `TclGetNamespaceForQualName` with
  both `nsPtr[0]` and `nsPtr[1]`.
- B / D: same function, the two-slot `nsPtr[2]` search array.
- C: the `for (i=0; i<cxt->commandPathLength; i++)` loop in between.

Deliberate C-parity gaps:
- We don't check `NS_DEAD` — nothing dies in our world (§4).
- We don't bump `CMD_VIA_RESOLVER` — no resolvers (§4).
- We don't rehash on `cmdEpoch` — compiled procs have stable bucket
  addresses and interpreted bodies re-resolve every call anyway.

### 5.3 Variable resolution 

Follows exactly the same shape — `ns_resolve_qualified` to find the
containing ns, then walk into `var_table`.  The frame-local alias
bit already present in `frame.rs` (`ALIAS_GLOBAL` / `ALIAS_EXT`)
maps cleanly to C's `VAR_LINK`: a local whose value is the absolute
address of a `Var` in some ns's `var_table` is a `VAR_LINK`
equivalent, which `variable` / `global` populate
exact shape.

## 6. Implementation map

| Concern | Where |
|---|---|
| `Namespace` struct, root namespace, tree lookup/create | `runtime/rust/src/namespace.rs` |
| Current-namespace stash (`tcl_ns_set` / `tcl_ns_restore`), per-frame `ns` | `runtime/rust/src/interp.rs`, `frame.rs` |
| Qualified-name resolution (`ns_resolve_qualified`) | `runtime/rust/src/namespace.rs` |
| Command lookup through the tree, the path, and the root (`ns_find_command`, `Namespaces::home_of`) | `runtime/rust/src/namespace.rs`, `cmd_namespace.rs` |
| Per-namespace variable tables; `variable` / `global` / `upvar` links | `runtime/rust/src/vars.rs`, `frame.rs` |
| Export patterns, import redirects, `namespace forget`, import back-links | `runtime/rust/src/namespace.rs` |
| `namespace path` and the command-reference epoch that invalidates cached lookups | `runtime/rust/src/namespace.rs` |

Two invariants that fall out of the layout and are easy to break:

- **The global frame's variable table *is* the root namespace's**, so a level-0
  frame target is canonicalised to the root namespace at the link site rather
  than kept as a frame reference.
- **A command-reference epoch is bumped on every add/remove and cascaded
  through the path back-links**, and any resolution cache is keyed partly on
  it. Coarse-but-correct invalidation beats a clever partial one.

The compiled `tcl-runtime` WASM artefact is not checked in; it is built from
`runtime/rust/` on demand, so a fresh checkout picks up the right binary
without anyone committing one.

## 7. Rust API surface

Everything lives in `runtime/rust/src/namespace.rs`.  Function signatures
use `u32` for `*Namespace` / `*Command` / `*Var` (linear-memory
addresses, consistent with the rest of the runtime).  `[]const u8`
means `(ptr, len)` pair — the slice ABI the other runtime
modules already use.

### Core tree 

```
/// Return the root (global) namespace.  Always non-zero after the
/// module is loaded.
pub fn ns_root() u32;

/// Find a direct child of ``parent`` with simple name ``name``.
/// Returns 0 if not found.
pub fn ns_lookup(parent: u32, name: []const u8) u32;

/// Find-or-create.  Creates ``name`` as a child of ``parent`` if
/// missing; otherwise returns the existing one.  New namespaces
/// start with empty child/cmd/var tables.
pub fn ns_create(parent: u32, name: []const u8) u32;
```

### Current-ns stash 

```
/// Current namespace for the innermost active frame — equivalent to
/// ``iPtr->varFramePtr->nsPtr`` in C.  Defaults to ``ns_root()``.
pub fn ns_current() u32;

/// Compiler-emitted prologue: save the current ns and switch to
/// ``target``.  Returns the saved handle so ``tcl_ns_restore`` can
/// restore it.  Replaces the string-based stash that's there today.
#[no_mangle] pub extern "C" fn tcl_ns_set(target: u32) -> u32;

/// Restore the previously-stashed ns.
#[no_mangle] pub extern "C" fn tcl_ns_restore(saved: u32);
```

### FQN walker 

```
pub const QualifiedResult = struct {
    target_ns: u32,            // 0 if any child-table step missed
    simple_ptr: u32,           // start of the trailing simple name
    simple_len: u32,
    alt_ns: u32,               // 0 if no alt path (cxt == root, or diverged)
};

pub fn ns_resolve_qualified(
    cxt: u32,
    name_ptr: u32,
    name_len: u32,
) QualifiedResult;

/// Same but creates any missing intermediate namespaces.  Used by
/// ``namespace eval ::new::inner { ... }``.
pub fn ns_resolve_qualified_creating(
    cxt: u32,
    name_ptr: u32,
    name_len: u32,
) QualifiedResult;
```

### Command table 

```
/// Insert or update a command in ``ns.cmd_table``.  Bumps
/// ``cmd_ref_epoch`` on ns and cascades through ``path_source_head``
/// Returns the bucket base.
pub fn ns_add_command(ns: u32, name: []const u8) u32;

/// Walk the full resolution chain: context → path → root.
/// Returns 0 if not found.
pub fn ns_find_command(cxt: u32, name: []const u8) u32;
```

### Variable table 

```
pub fn ns_var_find(ns: u32, name: []const u8) u32;
pub fn ns_var_create(ns: u32, name: []const u8) u32;

/// Follow VAR_LINK chains to the terminal Var.
pub fn var_resolve_link(v: u32) u32;
```

### Import / export 

```
pub fn ns_export(ns: u32, pattern: []const u8) void;
pub fn ns_import(dest: u32, src_pat: []const u8) void;
pub fn ns_forget(dest: u32, src_pat: []const u8) void;

/// True if ``name`` matches any of ``ns.export_patterns`` using
/// ``string match`` semantics (we already have a Tcl glob
/// implementation in value_ops.rs).
pub fn ns_export_matches(ns: u32, name: []const u8) bool;
```

### Path 

```
pub fn ns_set_path(ns: u32, targets: []const u32) void;
pub fn ns_get_path(ns: u32) []const u32;

/// Called by ns_add_command to invalidate every ns that lists
/// ``ns`` on its path.  Internal to namespace.rs.
fn cmd_ref_epoch_bump(ns: u32) void;
```

### Iteration helpers

The existing `hash_table.Table(N).each(ctx, visit)` already covers
`info commands` / `info vars` enumeration.  `namespace.rs` exposes
thin wrappers so callers don't need to know the table layout:

```
pub fn ns_each_command(ns: u32, ctx: anytype, comptime visit: fn (@TypeOf(ctx), u32) void) void;
pub fn ns_each_variable(ns: u32, ctx: anytype, comptime visit: fn (@TypeOf(ctx), u32) void) void;
pub fn ns_each_child(ns: u32, ctx: anytype, comptime visit: fn (@TypeOf(ctx), u32) void) void;
```

### The ABI compiled code depends on

`proc_register` / `proc_register_compiled` / `proc_lookup` and the
`proc_get_*` accessors are used verbatim by compiler-emitted code, so their
signatures are fixed regardless of how the lookup is implemented underneath.

## 8. Open questions

Points where the C source is ambiguous, where we diverge, or where
the design is under-specified and a later phase will have to settle.

### 1. Compiler-side FQN vs runtime resolution

The compiler already mangles compiled procs to their FQN (e.g. a
`proc` inside `namespace eval ::tcltest { ... }` emits a symbol for
`::tcltest::test`).  Once P2 lands, `ns_add_command` will insert
that same proc into `::tcltest.cmd_table` under simple name `test`.

Question: do we also insert the mangled symbol into `root.cmd_table`
for backward-compat with existing callers that pass the FQN
directly?  The resolution chain handles `::tcltest::test` correctly
(FQN walk hits the right ns), so we shouldn't need to — but it
means the current flat-table "find by mangled name" fast path no
longer exists.  Every caller must either pass a
FQN (which resolves through the walker) or a simple name + context
ns (which resolves through B/C/D of §5.2).

### 2. Per-frame vs per-interp current-ns

C Tcl stores the current ns on `varFramePtr->nsPtr` — it's part of
the call frame and is restored naturally when the frame pops.  Our
current `tcl_ns_set` / `tcl_ns_restore` pair is compiler-emitted and
*surrounds* the frame (not inside it), which breaks cleanly only if
the runtime never needs the current ns between frame pop and
`ns_restore`.  The pointer lives on the frame header
alongside the alias slots — verify no code reads `ns_current()`
from a position where the frame has already popped.

### 3. Unqualified name contains `::` (partial qualification)

Given `a::b` called from `::ctx`, C starts the FQN walk at `::ctx`
and does *not* fall back to global if the walk misses — `Tcl_FindCommand`
uses `TCL_NAMESPACE_ONLY` for the context probe, then separately
tries global.  Our §5.2 pseudocode matches that (we walk from both
`cxt` and `root`), but the interaction with `commandPathArray` is
subtle: the path walks should also use `TCL_NAMESPACE_ONLY`
anchoring on each path member.  Needs a fixture test
that `path a; path a::b` beats `path a; b` when both exist.

### 4. Back-list invariants across `namespace import`

When a source cmd is imported into N namespaces, we accumulate N
`ImportRef` nodes on the source's `import_ref_head` list.  If the
source itself is an import (a redirect), what does `namespace import
::mid::x` do?  In C, `DoImport` follows the `ImportedCmdData.realCmdPtr`
chain to the original and threads the new redirect into *that* list.
The import path does the same — but note that
chained imports always point at the terminal real cmd, not the
intermediate.

### 5. `variable` on an FQN

`variable ::a::b::x` declares (and optionally initialises) a
namespace variable in `::a::b`, regardless of the current ns.  The link path
must use `ns_resolve_qualified_creating` (not `_creating` would be
wrong — the ns must exist), then insert into that target's
`var_table`, then create the `VAR_LINK` entry in the current frame.
Confirm against `tclVar.c:Tcl_VariableObjCmd`.

### 6. `upvar` into a namespace var

`upvar 1 ::ns::v local` creates a frame-local alias pointing at a
ns-scoped var.  Our `frame.rs` `ALIAS_EXT` descriptor
currently supports `KIND_GLOBAL_NAMED` (global by name) and
`KIND_FRAME_VAR` (another frame).  The link path may need a
`KIND_NS_VAR` (target ns + simple name) if the ns var doesn't yet
exist at `upvar` time — or we can follow C's approach and lazily
create the ns var on `upvar`, keeping only the `VAR_LINK`-to-Var
shape on the frame side.

### 7. `[namespace current]` inside `uplevel`

`uplevel` temporarily makes an outer frame's ns the current ns for
the duration of the body.  Today `frame_depth_stash` / `frame_depth_restore`
handle the frame-depth part; they also need to adjust
`ns_current()`.  This needs a test that
`namespace eval ::a { uplevel #0 { namespace current } }` returns
`::`.

### 8. `namespace which`

`namespace which -command foo` exposes the exact resolution the
runtime would do.  Once §5.2 is live, `namespace which` becomes a
one-liner (`ns_find_command` → build the FQN from `target.ns` +
simple name).  Not scheduled for P1-P5 but trivial to add once the
tree is in; flag for P5 closeout.
