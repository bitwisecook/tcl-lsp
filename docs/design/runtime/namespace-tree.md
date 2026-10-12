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

Source of truth for the C semantics we're mirroring is Tcl 9.0.4. All C-file
citations in this doc use that release's source.

Jim uses a separate flat command table and retained canonical namespace
objects. Each active context has an `Owned` original Jim object and a stable
arena token; its counted name is not parsed into C parent/child edges.
`namespace canonical` returns the original root-relative operand, creates a
fresh absolute suffix string, or duplicates and appends to the actual current
object. `namespace eval` retains the original name and single body object;
multiple body operands use the selected native concat owner. Namespace helper
frames own local variables, while qualified globals use the root flat table.
`namespace which -variable` constructs the canonical qualified name even when
no variable exists. Procedure namespace objects derive from the counted flat
publication key. Jim command enumeration and imports use the shared native
name/glob owners and do not require a C namespace node to exist.

## 1. Goal & non-goals

### Goal

The C Tcl namespace tree has parent / child links, per-namespace command and
variable tables, explicit path and export lists — so command and variable
resolution matches Tcl 9 semantics (`tclNamesp.c:Tcl_FindCommand`,
`tclVar.c:TclObjLookupVarEx`) rather than any flat fully-qualified-name
keying.

Correctness first: a tcltest-shaped bundle (`proc $varName {args}
body` created inside a factory, then invoked by FQN or unqualified
name, possibly through `namespace import`, possibly from inside
`namespace eval`) resolves the same way `tclsh 9.0` does.

### Non-goals

- **Not** a general OO / Itcl scaffold. This document covers the `namespace`
  built-in, `global`, `variable`, `upvar`, and the resolution paths those use.
  TclOO is its own subsystem (`cmd_oo.rs`) layered on the same tree.
- Runtime cache ownership belongs to the original-object owners. This tree
  supplies namespace incarnations, command nodes and invalidation epochs to
  `interp/native_command_names.rs` and the chosen procedure body's
  `interp/native_body_artifact.rs`. A vocabulary entry alone supplies neither
  an installed handler nor compiler admission.
- **Not** custom resolvers. Ensembles, traces, safe interpreters, and
  namespace deletion are all implemented, but elsewhere — see §4 for where.
- **Not** a change to the compiler's FQN mangling. Compiled procs keep their
  `::ns::name` mangled symbols; the tree is what `Namespaces::resolve` walks at
  runtime.

## 2. C Tcl 9 reference model

Only the fields that shape *resolution* and *storage* are called out
here. Original-object references, traces, deletion handlers and ensembles
have their own retained owners described in §3–4; custom resolver plug-ins
remain unsupported.

### `Namespace` (`tclInt.h:278`)

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
`earlyDeleteProc`, `nsId`, `interp`, `refCount`,
`resolverEpoch`, `cmdResProc`, `varResProc`, `compiledVarResProc`,
`exportLookupEpoch`, `ensembles`, `unknownHandlerPtr`.

### `NamespacePathEntry` (`tclInt.h:402`)

One ordered entry in a namespace's `commandPathArray`.  Two pointers
each:

- `nsPtr` — target ns (may be `NULL` if invalidated).
- `creatorNsPtr` — ns whose path contains this entry.
- `prevPtr` / `nextPtr` — doubly-linked list, hanging off the target
  ns's `commandPathSourceList`.  This is the back-list that lets a
  ns invalidate every path entry pointing *at* it when its own
  commands change.

### `Command` (`tclInt.h:1840`)

One per proc / built-in / imported redirect.  Mirror fields:

| Field | Type | Purpose |
|---|---|---|
| `nsPtr` | `Namespace *` | home ns (entry lives in `nsPtr->cmdTable`) |
| `objProc` | `Tcl_ObjCmdProc *` | C entry point |
| `objClientData` | `void *` | type-dependent payload — for imports, `ImportedCmdData *` |
| `importRefPtr` | `ImportRef *` | head of back-list of importing cmds |
| `flags` | `int` | `CMD_DYING` / `CMD_DEAD` / `CMD_VIA_RESOLVER` / `CMD_REDEF_IN_PROGRESS` |

The `Command` enum of §3 carries handler-specific payloads. Native binding
identity is the retained command node and generation, independently of its
map key or reported name. The native registration owner supplies compiler-hook
presence; an original command-name cache retains node/reference epochs and
cannot infer that hook from the handler vocabulary. Legacy string-based C
entry points and custom resolver callbacks are not supplied by this enum.

### `ImportRef` + `ImportedCmdData` (`tclInt.h:1807`, `:1826`)

Pair that implements `namespace import`:

- `ImportedCmdData { realCmdPtr, selfPtr }` — the `objClientData` of
  the redirect command in the importing ns.  `realCmdPtr` is the
  source, `selfPtr` is the redirect itself (needed to splice out of
  the back-list on delete).
- `ImportRef { importedCmdPtr, nextPtr }` — singly-linked list anchored
  on `realCmdPtr->importRefPtr`.  Each node points back at a redirect
  so deleting the source can walk the list and remove every redirect.

### `Var` (`tclInt.h:644`)

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
`VAR_ARGUMENT`, `VAR_TEMPORARY`, `VAR_IS_ARGS` (compiled-local bits — a
WASM-compiled proc's locals are native locals, and an interpreted proc's
are ordinary `Var` cells in the frame's `VarTable`, so neither needs a
compiled-local classification on the cell).

### `CallFrame` (`tclInt.h:1278`)

Per-proc-invocation frame.  `runtime/rust/src/frame.rs` covers the
local-var + alias slice (`Var::Link` stands in for `VAR_LINK`).  The one
field mirrored from `CallFrame` is `nsPtr` — which namespace is "current"
while this frame is on the stack — carried as `Frame::ns` (an `NsId`).

### Key resolution entry points (`tclNamesp.c`)

- `TclGetNamespaceForQualName` (`:2272`) — given a (possibly
  qualified) name + context ns, walk child tables until we hit the
  simple name.  Returns `(containing_ns, simple_name)` pair *plus* an
  alternate "search-from-global" pair.  `Namespaces::home_of`'s qualifier
  walk (§5.2) is the analogue, with the alternate search folded into the
  same fall-through chain.
- `Tcl_FindCommand` (`:2631`) — unqualified lookup order.  This is what
  `Namespaces::resolve` mirrors.
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

The Rust realisation is an **arena**: `Namespaces` owns `arena: Vec<Namespace>`
and every reference between namespaces is an `NsId` (an index), never a
pointer. The global namespace `::` is always index 0 (`GLOBAL`). No `Rc`, no
parent pointers, no address arithmetic — which is both `wasm32`-friendly and
what makes the borrow discipline tractable, since a namespace can refer to
another while the arena is borrowed.

### `Namespace`

```rust
struct Namespace {
    /// Simple (unqualified) name; the global namespace's is empty.
    name: Vec<u8>,
    parent: Option<NsId>,
    children: BTreeMap<Vec<u8>, NsId>,
    /// The child table's retained `TCL_STRING_KEYS` bucket order.
    child_order: TclStringHashOrder,
    /// Entries + the command table's retained bucket order + a per-entry
    /// token generation.
    commands: CommandTable,
    /// `namespace path` — namespaces searched for unqualified commands.
    path: Vec<NsId>,
    /// `namespace export` patterns, matched with `string match` glob.
    exports: Vec<Vec<u8>>,
    /// The namespace's own variable table.
    vars: VarTable,
    /// `namespace unknown` handler (a command prefix); `None` ⇒ inherit the
    /// interpreter default.
    unknown: Option<obj::Owned>,
    /// The name a retained token keeps after its parent edge is gone (C keeps
    /// `fullName` when the deferred deletion nulls `parentPtr`).
    retained_fqn: Option<Vec<u8>>,
}
```

`Namespaces` carries the token's lifecycle counters beside the arena:
`activations: Vec<u32>` — C's `Namespace.activationCount`, bumped for every
call frame — and `deferred: BTreeSet<NsId>`, the tokens a non-zero count is
keeping alive (§4).

Namespace representation and cache coherence use these owners:

- **`fullName`** is not stored. `qualified_name(ns)` walks the `parent` chain
  and builds it on demand — a namespace's FQN is derivable, and caching it
  would need invalidation that nothing else here needs.
- **`cmdRefEpoch`** is represented by `command_reference_epoch`. The retained
  reverse namespace-path entries invalidate their creators when a searched
  namespace changes. `native_resolver_epoch` and `ensemble_export_epoch`
  remain separate compiler and ensemble dependencies. Original command-name
  objects validate these dependencies and the exact live command node before
  reusing a cache.
- **`flags`** (`NS_DYING` / `NS_DEAD` / `NS_TEARDOWN`) is absent as a word:
  the states it names are the `dying`, `dead` and `deferred` sets on
  `Namespaces`, keyed by token id.

The maps provide deterministic storage independently of native enumeration.
Variable tables retain selected entry ledgers for dynamic birth, undefined
shells and array generations; compiled local index order remains separate.
C's retained hash bucket order also governs `namespace children` and namespace
**teardown**. Both read a retained
[`TclStringHashOrder`] kept beside the map — one per child table, one per
command table — because Tcl quadruples the bucket array at a 3:1 load factor,
never shrinks it, and reverses chains on every rebuild, so the order cannot be
reconstructed from the live entries alone. `TclDeleteNamespaceChildren` and
`TclTeardownNamespace` snapshot exactly that order before deleting each token,
and delete traces make it observable.

`CommandTable` also mints a **generation** per entry. It is the one piece of
C's `Command` identity the table needs, and two things read it. A teardown
snapshot names `(tail, generation)`, so a token a delete callback deleted or
redefined is skipped rather than confused with whatever now holds the name —
C's `CMD_DYING` early return, which leaves the replacement to the next
snapshot. And every command-trace registration is stamped with it, so a
deletion frees the dying token's trace list and no one else's
([`trace-implementation.md`](trace-implementation.md)).

The bytecode VM reaches the same behaviour from a different shape: its command
table is one flat map keyed by canonical FQN, so it keeps the order owners in
`ns_command_order`, keyed by each command's holder namespace, and its teardown
snapshot pairs each key with the `command_generations` entry. One caveat is
the VM's alone: it registers its builtins in its own bootstrap order rather
than `Tcl_CreateInterp`'s, so the **global** holder's order — what
`namespace delete ::` would enumerate — is ours, not C's. Every per-namespace
user table is exact.

[`TclStringHashOrder`]: ../../../rust/tcl-cmd-core/src/namespace.rs

### `Command`

A table entry is the `Command` enum from `interp.rs`, not a struct with a flags
word. Its variants and their dispatch are tabulated in
[`rename-alias.md`](rename-alias.md) §2. Two consequences shape this document:

- **The home namespace is not stored on the command.** A command's namespace
  *is* the arena slot whose `commands` map holds it, so `rename` moving an
  entry between maps is the whole of "changing its home". (One place notices:
  a `Command::Proc` carries its own `ns` and `fqn`, fixed at definition time
  for `info frame` provenance — see §8.1.)
- **`namespace import` is a `Command::Imported { source }` entry** holding the
  source's FQN as bytes, re-resolved at global on every dispatch. There is no
  `ImportedCmdData` / `ImportRef` pair and no back-list of importers. The
  operations C uses the back-list for are done by scanning instead: a source
  rename walks the tree rewriting matching `source` fields
  (`retarget_imports`), and `namespace forget` finds redirects with
  `imported_in(ns)` and drops them with `remove_in`. Both are cold paths.

### `Var`

The variable cell is the `Var` enum in `frame.rs` — `Scalar` / `Array` /
`Link` — described in [`memory-management.md`](memory-management.md) MM-B and
[`../contracts/runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md).
The C flag bits map onto the enum discriminant (`VAR_ARRAY` → `Array`,
`VAR_LINK` → `Link`) rather than onto a flags word; `VAR_CONSTANT` is a
separate `consts` set on the table, and `VAR_IN_HASHTABLE` /
`VAR_ARRAY_ELEMENT` / `VAR_DEAD_HASH` describe C's hash-entry mechanics and
have no analogue.

A `Link` carries `{ home: VarHome, name, elem }` for selected lookup, plus
its actual native target owner. C scalar and element aliases retain the
original entry and array generation, so deleting and recreating a spelling
does not retarget an old alias. Jim retains the original recursive target-name
object and a weak frame-incarnation receipt; its selected lookup follows that
name. A byte label alone cannot identify a C captured cell or a live Jim frame.

### Call frames

`Frame` carries `ns: NsId` — the namespace that was current when the frame was
pushed — alongside its `VarTable`, its logical `level`, its invoking `words`,
and an `is_proc` flag distinguishing a proc call frame from a `namespace
eval` / `inscope` frame. `InterpState::current_ns` is the live current
namespace; `uplevel` and `namespace eval` both save it, set it, and restore it
around the body, and a frame pop restores it from the frame beneath.

## 4. Structures C carries that this tree does not

"Omit" means the structure is unreachable from the supported surface, not that
it is pending. Where behaviour C attaches to one of these structures *is*
implemented, it is implemented differently — the pointer below says where.

### Namespace lifecycle bookkeeping (**omit the bookkeeping, not the behaviour**)

C Tcl uses `refCount`, `activationCount`, `NS_DYING` / `NS_DEAD`,
`deleteProc`, `earlyDeleteProc`, `VarInHash.refCount`, `Command.refCount`, and
`Command.cmdEpoch` to sequence a namespace teardown that can be re-entered by
the very code it is tearing down. `refCount`, `deleteProc` and
`earlyDeleteProc` are not carried, and there is no `flags` word on either
`Namespace` or `Command`, because Rust ownership sequences the teardown
instead — a removed entry is moved out of its map, so nothing can observe a
half-dead one.

The one exception is what `CMD_DYING` + `Command.cmdEpoch` together decide:
*is the thing at this name still the token I snapshotted?* `CommandTable`
answers that with its per-entry generation, so `delete_namespace_token` can
run C's loop — snapshot `cmdTable` in hash order, delete each snapshotted
token in turn while its entry is still visible to its own delete trace, and
re-snapshot while the table is non-empty. A callback that deletes a
snapshotted sibling, or redefines the entry the loop is on, changes that
entry's generation and the loop skips it; a command the callback *created*
is torn down by the next snapshot.

`activationCount` **is** carried, as `Namespaces::activations`. Every call
frame — proc, `apply`, TclOO method, `namespace eval`/`inscope` — counts an
activation against the namespace it runs in, and gives it back at the matching
pop. `namespace delete` on a token whose count is non-zero takes C's other
branch (`activationCount > (nsPtr == globalNsPtr)`, `tclNamesp.c:1012`): it
retires the token's owned ensembles and its `namespace unknown` handler, drops
the parent's child edge — so `namespace exists` and every absolute name stop
resolving at once, and the spelling is free for a wholly separate token — and
then stops. Commands, variables, children and exports stay exactly as they
are, reachable only from a frame that *holds* the token: a relative name
resolves through it, `namespace current` answers from the name it kept
(`retained_fqn`, C's `fullName` surviving a nulled `parentPtr`), and a
definition without a qualifier lands in it. The ordinary teardown then runs
from the pop that drops the last activation, exactly as C's
`Tcl_PopCallFrame` (`tclNamesp.c:491`) simply calls `Tcl_DeleteNamespace`
again — same per-token hash-ordered loop, same traces, same recursion into
children, which each re-enter the count check and may defer in turn.

A coroutine is the one place C never gets there: deleting a suspended
coroutine frees its parked frames without `Tcl_PopCallFrame`, so a namespace
they retained is abandoned rather than torn down, and no trace fires. Both
runtimes match, by discarding the parked stack rather than popping it.

The bytecode VM keeps the same counters (`NamespaceDeferral`) but cannot leave
a retained token's tables in place: its `commands` map is flat and keyed by
canonical name, so a retained `N::q` and the `N::q` of a namespace recreated
under that spelling would be one entry. The retained subtree therefore moves
into a `RetainedNamespace` record filed under the token's id, which only the
frames holding that token reach — resolution consults it for the
current-namespace candidate and refuses the flat map's entries in that
subtree, `info commands` / `info procs` / `namespace children` read it by
token id, and a relative definition is absorbed into it. Its final teardown
splices each token back into the live map one at a time, so the shared command
lifecycle runs unchanged. Variable tables remain separately owned by the
actual namespace token in `ns_vars`; `VarTableOwner::Namespace` selects that
token, so a same-path recreation receives a distinct table. Command-trace
sidecars remain in the separate binding map, with each registration stamped
by command generation. Retirement removes the dying token's registrations
without removing a replacement token's registrations.

`namespace delete` **is** implemented (`cmd_namespace.rs::ns_delete`, mirroring
C's `NamespaceDeleteCmd`): it deletes each named namespace with its children,
commands, and variables, errors on a missing namespace, and destroys any object
living in that namespace — running its destructor while the namespace is still
intact, as C's `ObjectNamespaceDeleted` does. Deletion goes **by namespace id**
(`delete_namespace_by_id`, over `descendant_ids`) rather than by name,
specifically so variable unset traces fire as the namespace is torn down.

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
interpreter, reached through the `Command::Ensemble` table entry.

### Child and safe interpreters

Implemented — see [`child-interp.md`](child-interp.md) for the per-interpreter
state (its own namespace arena, hidden-command table, children map) and
[`rename-alias.md`](rename-alias.md) for `interp alias`. The interpreter
carries a `hidden` command table, which is what a safe interpreter hides the
dangerous commands into.

### Per-namespace `unknown` (`unknownHandlerPtr`)

`namespace unknown` stores the accepted original object root, after the selected
native List Length getter succeeds. The setter and getter return that object,
without reconstructing its bytes or members. Zero elements clear the stored
handler even when its original string contains whitespace. A malformed prefix
leaves the previous root in place.

The selected native fallback policy chooses the caller or lookup namespace's
handler; an unset handler falls back to the global original root. Handlers are
not inherited by child namespaces. The global default is installed lazily.
Dispatch converts the stored original root through the selected List getter,
retains its original prefix members through the reached invocation, and resolves
the original head object in the retained lookup namespace. Handler replacement
or namespace deletion releases the stored root outside namespace-table borrows.

### Custom resolvers

Native custom resolver callbacks (`Tcl_SetNamespaceResolvers`,
`Tcl_AddInterpResolver`, and compiled-variable resolver hooks) are not exposed
by these Runtime namespace tables. Resolution uses the shared native name
protocol and the interpreter's actual command and variable tables.

### Compiled-local `Var`s (**omit by construction**)

The `VAR_ARGUMENT` / `VAR_TEMPORARY` / `VAR_IS_ARGS` / `VAR_RESOLVED`
flags describe compiled procedure locals, which in C live in a
per-frame `Var *` array.  WASM-compiled procs use native locals bound to
name-addressable cells through `tcl_codegen_local_bind`; interpreted procs use
the frame's `VarTable` directly. Neither carries a flags word, so these bits
are never read or written.

### `nsId` / `interp` (**omit**)

`nsId` is a monotonic counter used by debug tooling and `[namespace code]`
serialisation; the arena index serves the same addressing purpose and the FQN
is regenerable from the `parent` chain, so it has no consumer here.  `interp`
is the enclosing `Tcl_Interp *`; the runtime's per-interpreter state is held on
the interpreter itself ([`child-interp.md`](child-interp.md)), and each
interpreter owns a whole arena rather than each namespace pointing back at one.

### `resolverEpoch` / bytecode invalidation (**omit**)

C Tcl bumps this when resolution rules change so existing bytecode
re-compiles.  Compiled code here is AOT WASM — recompilation happens
by re-running the toolchain, not at runtime.

### `clientData` / `deleteProc` on namespaces (**omit**)

User-ns state hooks.  No in-tree user.

### Export lookup and ensemble dependencies

Export matching for `info commands` reads live `Namespace::exports` through
`is_exported` / `exported_commands`. The separate `ensemble_export_epoch`
tracks actual ensemble dependencies. A live vocabulary query cannot provide
an ensemble binding, prefix root or compiled dispatch-cache receipt.

### `cmdRefEpoch` / `commandPathSourceList`

The namespace owner retains reference epochs and reverse path dependencies.
`resolve_original_command_at` selects the actual engine's command-name
protocol on the same original object. A valid cached node bypasses string
lookup; a stale cache returns to the selected ordinary lookup boundary. C's
command-name cache and Jim's commandObj cache are separate primaries. Jim
retains its original namespace object and procedure epoch; a C receipt cannot
supply either role.

## 5. Resolution algorithms

Two resolvers, one for commands and one for variables. They are deliberately
*not* the same function: commands fall through a search chain until they find
an existing binding, variables commit at namespace level. Both are modelled on
`tclNamesp.c` / `tclVar.c`.

### 5.1 Name splitting

`tcl_syntax::naming` supplies the shared written-name split used by both
resolvers: `qualifier_segments` splits on `::` runs, `is_qualified` reports
whether a name contains one, and `ends_with_separator` catches the edge case
that shapes both resolvers —

**a name ending in a separator run names the empty-string `{}` entry** in the
qualified namespace, with *every* segment treated as a namespace component.
With `proc {} {} {}` defined, both `::` and `:::` dispatch it (tclsh 8.6/9.0
pinned), and `rename foo x::` binds `::x::{}`. Handling this in the
shared splitter is what keeps `home_of`, `var_home`, `rename`, and the ensemble
name split agreeing about what a written name means.

### 5.2 Command resolution — `Namespaces::home_of` / `resolve`

`resolve(current, name)` is the single command resolver (the command-binding
contract's A1/A2). It calls `home_of(current, name)` for the
`(namespace, simple name)` the name binds in, then clones the handle out of
that namespace's table.

`home_of` splits the name into a simple tail and zero or more qualifier
segments (§5.1), then defines one probe:

```rust
// Walk the qualifier segments down from `base`, then require the command.
let find_under = |base: NsId| -> Option<NsId> {
    let mut ns = base;
    for part in ns_parts {
        ns = *self.arena[ns].children.get(*part)?;
    }
    self.arena[ns].commands.contains_key(simple).then_some(ns)
};
```

and applies it in order:

1. **Absolute** (`name` starts with `::`) — `find_under(GLOBAL)` only. No
   fall-through: an absolute name either binds where it says or misses.
2. Otherwise, **the current namespace**: `find_under(current)`.
3. Then **each namespace on `current`'s `namespace path`**, in order.
4. Then **the global namespace**, when `current` is not already global.
5. Miss — the caller raises `invalid command name` (and `unknown` runs later).

Two things follow that are worth stating plainly, because they differ from a
naive reading of C:

- **The chain is existence-checked at every step**, not namespace-checked.
  `find_under` only reports a hit when the command is actually present, so a
  qualifier that resolves to a real namespace with no such command keeps
  searching rather than committing to that namespace.
- **A relative qualified name uses the same chain.** `a::b::cmd` from `::ctx`
  is tried as `::ctx::a::b::cmd`, then under each `namespace path` entry, then
  as `::a::b::cmd`. C reaches the global namespace for this case through
  `TclGetNamespaceForQualName`'s alternate-search slot rather than through the
  path; the runtime folds both into the one chain, which additionally lets a
  `namespace path` entry serve a partially-qualified name.

`resolve_fqn(current, name)` is the same walk returning the canonical
fully-qualified name instead of the handle — the key that command and
execution traces are registered under, so a trace addresses the same binding
`resolve` (and `rename` / `delete`) hits.

A **retained** token (§4) is reachable by no public path: its parent edge is
gone, so no absolute name and no `namespace path` entry finds it. Only the
frames whose current namespace *is* that token reach its tables, and they reach
them the way C does — `TclGetNamespaceForQualName` walking from
`varFramePtr->nsPtr`. An absolute name from such a frame is still rooted at the
global namespace, where a same-named recreation lives, so the two tokens never
answer for each other.

Once a recreation exists, **a namespace spelling is not a namespace identity**,
and every place that enters or reads a namespace has to say which token it
means:

- entering a **procedure** uses the token its binding lives in — C hands
  `Tcl_PushCallFrame` the `Namespace *` from `procPtr->cmdPtr->nsPtr`, and the
  VM carries the same thing as `ProcDef::ns_id`. A retained procedure called
  after its namespace was recreated still runs in the retained token; one
  reached by absolute name in the recreation runs in that one;
- `namespace eval` / `inscope` follow the qualifier: a **relative** name walks
  from the frame's own namespace and so reaches the retained children, an
  **absolute** one is rooted at the global namespace and builds a fresh tree
  under that spelling;
- `namespace path` and `namespace export` hang off the `Namespace`, so they
  follow the token; `unknownHandlerPtr` does not survive at all, because
  `Tcl_DeleteNamespace` frees it before it looks at the activation count;
- a body recompiled on demand (a step-capable trace forces one at the next
  entry) is memoised back into the table its binding came from, never into the
  flat map under a spelling a recreation owns.

Custom `CMD_VIA_RESOLVER` callbacks remain unsupported (§4). Native original
command-name caches retain the selected command node and reference context,
including a valid cached absolute name into a retained child. Literal and
dynamically constructed inputs must both be tested without erasing their
cache differences. The chosen original procedure body can own an admitted
Bytecode artifact with its literal array, local layout and entry guards;
ordinary header duplication follows the selected string-only body rule.
Future declaration relocation and an already active frame's namespace remain
independent owners.

### 5.3 Variable resolution

The variable resolver is `vars.rs`, modelled on `tclVar.c:TclLookupSimpleVar`.
It is one classification plus one link walk.

**Byte operand protocol.** `Interp::name_policy_protocol` selects the actual
engine's `NativeNameProtocol`, or an explicitly installed logical name provider.
`interp/variable_names.rs` retains that recipe in the namespace owner before
fresh variable lookup. An unavailable recipe produces a typed host refusal;
byte `VarStore` reads propagate that refusal instead of reporting absence.

Complete names use `combined_variable_input`. Root and element arguments use
`separate_variable_input`; a second element is never concatenated into a fresh
name. These purposes have different NUL extents. Root qualification uses the
original selected projection, so a separator after NUL cannot turn a counted
unqualified scalar into a qualified namespace address. `global` and `variable`
use their own byte local-name selectors. Trace registration and query apply
their selected text extents before resolving the physical receiver.

Jim namespace variables use the original constructed namespace bytes and
`jim_global_variable_key_bytes`. Neither namespace components nor scalar or
element keys pass through a Unicode replacement conversion. Procedure locals
remain in their activation's byte table. Captured array targets and followed
links retain their physical generation; name policy is not reapplied to those
already selected cells.

**Classification.** A name is a *namespace variable* when it is qualified
(contains `::`) **or** there is no active proc frame — the global scope, or a
`namespace eval` body. Otherwise it is a *frame-local* variable. So:

1. **Qualified** (`::a::b::x`) → namespace `::a::b`, simple tail `x`, absolute
   when `::`-led and relative to the current namespace otherwise. The
   namespace must already exist: a write into a missing one raises `can't set
   "…": parent namespace doesn't exist`, while a read or unset simply misses.
2. **Unqualified, inside a proc** → the current frame's local table.
3. **Unqualified, at global or `namespace eval` scope** → the current
   namespace's variable table. The global frame and the global namespace share
   one table, so `set x` and `set ::x` at top level are the same variable, and
   a level-0 frame target is canonicalised to `VarHome::Namespace(GLOBAL)` at
   the link site.

`Namespaces::var_home` performs the qualified split, and it is deliberately
**not** `home_of`: variable resolution has no existence-checked fall-through,
so it commits to the first namespace the qualifier resolves to rather than
continuing to search. That asymmetry is the C behaviour.

**The link walk.** `global`, `variable`, and `upvar` all install the same
shape: a `Var::Link { home, name, elem }` in the current home table. Resolution
follows links to a concrete `Place` (a table, a simple name, and an optional
array element), bounded by `LINK_LIMIT` so a pathological alias cycle cannot
spin forever. Because the link stores a *path* rather than a pointer, the
target may be created after the link (C's lazy `upvar` behaviour), and a target
table reallocating cannot dangle it.

`variable` also uses a self-link as its declared-but-undefined marker: a `Link`
whose `name` equals its own key and has no element is exactly the undefined
`Var` that the first write turns into a real `Scalar` or `Array`.

**One dialect switch lives here.** Tcl 8.x resolves an unqualified variable at
namespace scope to the *global* variable when the namespace has none but the
global namespace does; 9.0 removed that fallback (TIP 278,
`TCL_NAMESPACE_ONLY`). `Namespaces::ns_var_global_fallback` defaults to the 9.0
behaviour and is flipped by an 8.x embedding through
`Interp::set_runtime_version`.

## 6. Implementation map

| Concern | Where |
|---|---|
| `Namespace` / `Namespaces`, the arena, tree lookup and create (`ensure_namespace`, `find_namespace`, `children`, `parent`, `qualified_name`) | `runtime/rust/src/namespace.rs` |
| Current namespace (`InterpState::current_ns`) and its save/restore around `namespace eval` / `inscope` / `uplevel` | `runtime/rust/src/interp.rs` |
| Per-frame namespace, `VarTable`, `Var`, `Link`, `VarHome` | `runtime/rust/src/frame.rs` |
| The command resolver (`home_of`, `resolve`, `resolve_fqn`, `command_home_ns`) | `runtime/rust/src/namespace.rs` |
| The variable resolver (classification + link walk) | `runtime/rust/src/vars.rs`, with `var_home` / `var_table` in `namespace.rs` |
| The `namespace` ensemble's subcommands | `runtime/rust/src/cmd_namespace.rs` |
| `variable` / `global` / `upvar` / `array` | `runtime/rust/src/cmd_var.rs`, `cmd_array.rs` |
| Export patterns, import redirects, `namespace forget` | `runtime/rust/src/namespace.rs` (`export`, `is_exported`, `exported_commands`, `imported_in`, `remove_in`, `retarget_imports`) |
| `namespace path` | `runtime/rust/src/namespace.rs` (`set_path`, `path`) |

Two invariants that fall out of the layout and are easy to break:

- **The global frame's variable table *is* the global namespace's**, so a
  level-0 frame target is canonicalised to `VarHome::Namespace(GLOBAL)` at the
  link site rather than kept as a frame reference.
- **Original command-object caches validate against live tables.** Cached
  command allocations retain lookup-context and namespace token ownership;
  command deletion, replacement and namespace-path changes invalidate their
  corresponding lookup epochs. Text-only queries do not manufacture a cache.

The compiled `tcl-runtime` WASM artefact is not checked in; it is built from
`runtime/rust/` on demand, so a fresh checkout picks up the right binary
without anyone committing one.

## 7. Rust API surface

Everything lives in `runtime/rust/src/namespace.rs`, on `impl Namespaces`.
`NsId` is a `usize` arena index; `GLOBAL` is `0`. Names are `&[u8]` throughout
— the runtime's strings are bytes, not `str`.

### Core tree

```rust
pub fn new() -> Namespaces;
pub fn ensure_namespace(&mut self, current: NsId, qualified: &[u8]) -> NsId;
pub fn find_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId>;
pub fn parent(&self, ns: NsId) -> Option<NsId>;
pub fn children(&self, ns: NsId) -> Vec<NsId>;
pub fn descendant_ids(&self, ns: NsId) -> Vec<NsId>;
pub fn qualified_name(&self, ns: NsId) -> Vec<u8>;
pub fn delete_namespace(&mut self, current: NsId, qualified: &[u8]) -> bool;
pub fn delete_namespace_by_id(&mut self, ns: NsId);
```

### Command table

```rust
/// Register under a (possibly qualified) name, rooted at global,
/// creating intermediate namespaces.
pub fn register(&mut self, name: &[u8], command: Command);
/// Register under a simple name in a specific namespace.
pub fn bind(&mut self, ns: NsId, name: &[u8], command: Command);
/// The one resolver (§5.2) — returns a *clone* of the handle.
pub fn resolve(&self, current: NsId, name: &[u8]) -> Option<Command>;
/// The same walk, returning the canonical FQN (the trace key).
pub fn resolve_fqn(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>>;
/// Rebind in place where the name actually resolves (ensemble reconfigure).
pub fn rebind_resolved(&mut self, current: NsId, name: &[u8], command: Command) -> bool;
pub fn delete(&mut self, current: NsId, name: &[u8]) -> bool;
pub fn rename(&mut self, current: NsId, old: &[u8], new: &[u8]) -> RenameOutcome;
pub fn command_names(&self, ns: NsId) -> Vec<&[u8]>;
pub fn which_command(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>>;
pub fn command_origin(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>>;
/// The teardown snapshot: live `(simple name, generation)` slots in
/// `Tcl_FirstHashEntry` order.
pub(crate) fn command_hash_order(&self, ns: NsId) -> Vec<(Vec<u8>, u64)>;
/// The generation currently bound at `(ns, name)` — the snapshot's
/// still-the-same-token check.
pub(crate) fn command_generation(&self, ns: NsId, name: &[u8]) -> Option<u64>;
pub(crate) fn command_fqn_at(&self, ns: NsId, name: &[u8]) -> Vec<u8>;
```

### Variable table

```rust
pub(crate) fn var_home(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)>;
pub(crate) fn var_table(&self, ns: NsId) -> &VarTable;
pub(crate) fn var_table_mut(&mut self, ns: NsId) -> &mut VarTable;
pub(crate) fn var_names(&self, ns: NsId) -> Vec<Vec<u8>>;
pub(crate) fn const_names(&self, ns: NsId) -> Vec<Vec<u8>>;
pub(crate) fn which_variable(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>>;
```

Link following is not here — it crosses tables (frames and namespaces both) and
so belongs to the `vars.rs` coordinator, which borrows both owners from the
interpreter.

### Import / export

```rust
pub fn export(&mut self, ns: NsId, pattern: &[u8]);
pub fn clear_exports(&mut self, ns: NsId);
pub fn exports(&self, ns: NsId) -> &[Vec<u8>];
pub fn exported_commands(&self, ns: NsId) -> Vec<Vec<u8>>;
pub fn is_exported(&self, ns: NsId, name: &[u8]) -> bool;
/// `(local name, source FQN)` for every `Command::Imported` in `ns`.
pub fn imported_in(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)>;
pub fn remove_in(&mut self, ns: NsId, name: &[u8]) -> bool;
/// Follow a source rename through every redirect that named it.
pub fn retarget_imports(&mut self, old_fqn: &[u8], new_fqn: &[u8]);
```

`namespace import` itself lives in `cmd_namespace.rs::ns_import`: it splits the
pattern into a source-namespace qualifier and a glob tail, resolves the source
namespace (`unknown namespace in import pattern "…"` on a miss), refuses
importing a namespace into itself, and installs a `Command::Imported` for each
exported command that matches. Re-importing the *same* command from the *same*
source is a silent no-op (C's `TclGetOriginalCommand` re-import check); only a
clobber of a *different* command is a conflict.

### Path and unknown handler

```rust
pub fn set_path(&mut self, ns: NsId, path: Vec<NsId>);
pub fn path(&self, ns: NsId) -> &[NsId];
pub(crate) fn unknown_handler(&self, ns: NsId) -> Option<*mut TclObj>;
pub(crate) fn set_unknown_handler(&mut self, ns: NsId, handler: Option<obj::Owned>) -> Option<obj::Owned>;
```

### Iteration

There are no visitor helpers: `command_names`, `var_names`, `proc_names`,
`const_names`, `children`, and `descendant_ids` each return an owned or
borrowed listing directly. Two listings are ordered by the retained Tcl hash
table instead of the map: `children_hash_order` (which
`TclDeleteNamespaceChildren` and `namespace children` need) and
`command_hash_order` (which `TclTeardownNamespace` needs). Command and procedure listings retain sorted map order rather than C's
native hash order, so unsorted `info commands` / `info procs` remain a visible
ordering difference. Variable inventories use the selected entry ledger and
defined/declaration flags; compiled local index order is independent of dynamic
hash-table birth order.

## 8. Resolution invariants

### 8.1 Compiler-side FQN vs runtime resolution

A qualified name given to `register` is rooted at global and creates its
intermediate namespaces, so a compiled `::tcltest::test` lands in `::tcltest`
under the simple key `test`. It is **not** additionally inserted into the
global table under a mangled name: callers either pass an FQN (which resolves
through the qualifier walk) or a simple name plus context (which resolves
through the chain of §5.2). There is exactly one entry per command.

A procedure command retains its actual declaration independently of the
binding's current name. The selected relocation protocol updates future entry
namespace and reported provenance through that declaration; an already active
frame retains its original namespace incarnation. Jim's original procedure
namespace object is a separate owned role. Dispatch resolves the live binding,
rather than parsing a procedure's reported name (see
[`rename-alias.md`](rename-alias.md) §7).

### 8.2 Per-frame vs per-interp current namespace

Both. `InterpState::current_ns` is the live value every resolver reads, and
`Frame::ns` records what was current when each frame was pushed. `namespace
eval` / `inscope` and `uplevel` save `current_ns`, set it, and restore it
around the body; a frame pop restores it from the frame beneath. Nothing reads
the current namespace from a position where the frame has already popped.

### 8.3 Unqualified name containing `::` (partial qualification)

Settled in favour of the uniform chain — see §5.2. `a::b` from `::ctx` is tried
under `::ctx`, then under each `namespace path` entry, then under global, and
each attempt requires the command to exist rather than merely the namespace.

### 8.4 Chained `namespace import`

`Command::Imported` stores the source FQN as written at import time, and
dispatch re-resolves it. Importing an already-imported command therefore
produces a redirect naming the *intermediate*, which re-resolves through the
intermediate's own redirect on each call — the chain is followed at dispatch
rather than collapsed at import. `command_origin` walks the chain to report
`namespace origin`.

### 8.5 `variable` on an FQN

`variable ::a::b::x` addresses `::a::b` through `var_home`, which requires the
namespace to exist (§5.3), stores into that namespace's `VarTable`, and
installs a `Var::Link` with `home: VarHome::Namespace` in the current frame.

### 8.6 `upvar` into a namespace variable

The same `Link { home, name, elem }` shape covers it, with `VarHome::Namespace`
as the home. Because the link is a path rather than a pointer, the target
variable need not exist when `upvar` runs — it is created by the first write
through the alias, which is C's lazy behaviour.

### 8.7 `namespace current` inside `uplevel`

`eval_uplevel` sets `current_ns` to the *target frame's* namespace for the
duration of the body and restores it afterwards, so `namespace eval ::a {
uplevel #0 { namespace current } }` reports `::`.

### 8.8 `namespace which`

Implemented — `Namespaces::which_command` / `which_variable`, described in
[`command-introspection.md`](command-introspection.md) §5.
