# Contract: variable & call-frame resolution model

The semantic model both Rust interpreters implement for variable access: how a
name reaches a *cell*, what a frame is, and how `upvar` / `global` / `variable`
aliasing, arrays, and traces interact. The as-built mechanics live in
[runtime/memory-management.md](../runtime/memory-management.md),
[runtime/refcount-contract.md](../runtime/refcount-contract.md), and
[runtime/namespace-tree.md](../runtime/namespace-tree.md); trace firing is
[variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md).

## Why the indirection is the base case

In Tcl a variable name resolves to a cell through a chain that is only fully
known at run time: `upvar`, `global`, `variable`, `namespace eval`, array
elements, and traces can all redirect a plain name to a cell in a different
frame or namespace. A compiler that assumes "a proc local is a machine slot"
is wrong the moment any of those appear.

**The rule:** there is exactly one way to reach a variable — through a
*frame → name → cell* indirection. Treating a proven non-aliased, non-traced
local as a raw slot is an optimisation layered on top behind a guard, never
the base case. The compiler-side mirror of this is
[compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md).

## Original variable names and physical owners

The variable resolver selects the table and follows links. Native name primaries
accelerate that lookup only under a matching actual engine and native name policy.
An authored name simulation does not issue a physical C variable-name cache.
`NativeVariableNameProtocol` owns release-specific parsing and retirement rules;
the concrete object descriptors retain the original parts and actual owners.
Snapshots report those descriptors and never provide receiver authority.

A parsed scalar name retains no variable or namespace. A parsed array name owns
its original root name and either an allocated C8 element buffer or the original
C9 element object. C8 duplicates and string updates use the element's CString
extent; C9 element objects retain counted bytes. The selected table key and the
reported name remain separate projections.

An indexed C8.4 local-name primary retains the actual procedure. Later releases
retain a canonical original name or an explicit self-cache. Canonical names belong
to `NativeLocalNameTable`, shared by the procedure and activation through its
header. Cloning that header does not acquire another native member reference.
`NativeLiteralWorld` owns global registrations; header destruction releases each
global registration before its local member owner.

Original scalar/root hash keys belong to table entries, independently of retained
variable cells and lookup caches. C8.4 entries own byte keys; later C entries can
retain their original name objects. Unqualified Jim births retain the original key,
while absolute births own a separate stripped key. Removing a Jim entry retires its
key even when a static alias retains the old variable cell. C operations that hold
a root entry through a callback retire its key when that retained entry expires.

Use `read_original_named_variable` when the operand is an original name object.
Use `assign_original_named_variable` for assignment without command result
publication. `store_original_named_variable` provides the command assignment and
result publication door. Byte-only coordinator operations remain appropriate for
already selected addresses; they do not authenticate an original name object.

## Source analysis cell identities

`ResolveContext` carries `SourceNamespaceKey` through namespace bodies,
procedure frames, selected callers and aliases. A native key retains the
interpreter, namespace incarnation and exact component boundaries. A display
such as `::a:::b` can describe different paths; it cannot select either table.
Jim namespace selection uses the retained original flat namespace-object bytes.

`VariableCellKey` keeps namespace, activation, retained wrapper, lifetime and
array-member identities separate. `VariableCellTable` and `VariableCellSet`
accept text queries only for authored entries. Lifetime and element wrappers
remain structural even for authored keys, so written labels cannot impersonate
those addresses. Array membership compares the original root lifetime.

The retained current namespace and namespaces available to a new address lookup
have separate inventories. Deleting an active native namespace detaches its
address but preserves its cells until the last referring frame exits.
`PendingNamespaceRetirement` captures the original namespace incarnations;
pop-time retirement cannot select a replacement created at the same path.
Trace and contents facts follow those exact cells after links resolve.
Authored static publication uses its independently captured worker namespace
and outward observer receipt; an ordinary global does not inherit those effects.

## The two stacks and the two contexts

Tcl conflates them in surface syntax but they are distinct:

| Concept | What it is | Who changes it |
|---|---|---|
| **Call frame** | A variable table for the *currently executing* body (proc locals, or the global table at script top level). | A proc call pushes one; `apply`, methods, `uplevel`, coroutines, `[interp] eval` reshape the active one. |
| **Call stack / level** | The numbered chain of frames. `info level` reads it. | Proc calls push; `tailcall` replaces the top; coroutines suspend/resume sub-stacks. |
| **Current namespace** | Context for resolving *command* names and *qualified* variable names. | `namespace eval`, and a proc's *defining* namespace on entry. |
| **Namespace variable tables** | Per-namespace variable storage (`::`, `::foo`, …). NOT the same as call frames. | `variable`, `set ::ns::x`, `namespace eval`. |

The classic trap: the current namespace is **not** the call frame. Inside a
proc defined in `::foo`, an *unqualified* `set x` writes a frame **local**, not
`::foo::x`. You only reach the namespace var by qualifying (`::foo::x`) or by
declaring `variable x` (which installs a local alias to it). At script top
level, by contrast, the call frame *is* the global namespace table, so
unqualified `x` is `::x`.

## The cell

Both runtimes model C's `tclInt.h` `Var` union — `{ scalar | array | link }` —
as a Rust enum in a per-table map:

* `runtime/rust/src/frame.rs` — `Var::Scalar(*mut TclObj)`,
  `Var::Array(BTreeMap<Vec<u8>, *mut TclObj>)`, `Var::Link(Link)`.
* `rust/tcl-vm/src/vars.rs` — `VarState::Undefined`, `VarState::Scalar(Value)`,
  `VarState::Array(VarTable)`, `VarState::Link(VarId)`, and dialect-selected
  `VarState::NameLink`. A byte-keyed `VarTable` owns bindings and original native
  keys; the interpreter's `VarArena` owns the cells those ids identify.

Three representation decisions are load-bearing and are contract, not detail:

* **Lookup storage and native iteration have separate owners.** Byte-keyed
  maps store the cells. Persistent `NativeEntryLedger` records supply the selected
  native hash order, bucket history and entry lifetime for `info vars` and array
  enumeration. Map ordering does not authenticate native iteration.
* **Links preserve their selected dialect's target ownership.** VM C links name
  stable cells. Portable-runtime C links retain their selected table home and any
  captured array generation. Jim links retain the original target-name object
  and a weak receipt for its actual frame incarnation. The resolver follows the
  appropriate cell, selected home or original getter. Removing a binding never
  reuses its stable identity for a replacement. Link traversal is cycle-safe.
* **Observers attach to stable storage identities.** Interpreter trace
  registries retain the resolved variable generation independently of diagnostic
  names. A trace reached through an alias observes the selected cell. Unset and
  reinstallation distinguish the dying registration from a newly created cell;
  an equal spelling does not transfer trace ownership.

The VM additionally carries `VarState::Undefined` — a materialised but unset
cell, which is what `variable` and `trace add variable` create: invisible to
`info exists`, yet a later scalar or array write defines it with the
appropriate shape. `const`-declared names (TIP 677) are an interpreter-level
`const_vars` set of cell ids rather than a per-cell flag; destroying a cell
drops its id from the set, so a proc-local constant lasts one activation.

## Resolution algorithm

Modelled on `tclVar.c:TclLookupSimpleVar`; the coordinator is
`runtime/rust/src/vars.rs`. Given a name and the current `(frame, namespace)`
context:

1. **Parse the name.** Split a trailing `(index)` → array element (the index is
   itself substituted first). A `::` anywhere in the remainder makes the name
   *qualified*.
2. **Qualified `::a::b::x`** → namespace `::a::b` (absolute if `::`-led, else
   relative to the current namespace), simple tail `x`. The namespace must
   exist: a write into a missing one raises *parent namespace doesn't exist*,
   a read simply misses. Intermediate namespaces are never auto-created for
   variables.
3. **Unqualified, inside a proc frame** → the current frame's local table. It
   does **not** fall through to the namespace; that is what `global` and
   `variable` are for.
4. **Unqualified, with no active proc frame** — global scope, or a
   `namespace eval` / `namespace inscope` body → the current namespace's var
   table. So `set x` and `set ::x` at top level are the same global, and an
   unqualified name in a `namespace eval` body is a *namespace* variable. The
   VM records this on the frame as `ns_eval`, which is also what makes
   `uplevel`/`upvar` into a `namespace eval` body resolve to namespace
   variables.
5. **Array element `a(k)`** → resolve `a` by 2–4 to a cell, require it be an
   array (auto-vivify on write, error on read of a missing element), then the
   element.

`upvar L other my`, `global v`, and `variable v` all reduce to: resolve `other`
in the target frame or namespace, then install a `Link` under `my` in the
current frame. The alias is created even if the target is currently unset.
Because the global frame's table *is* the global namespace's table, a level-0
frame target is canonicalised to the global namespace at the link site.

### Frame addressing for upvar/uplevel

* `#N` — absolute, counting from the global frame (`#0`).
* `N` — relative, N frames up from the current (default `1`).
* The target frame must exist (`upvar 3` from level 1 is an error).
* A relative-qualified target name is resolved in the **target frame's**
  namespace, not the active callee's. Thus `::B::inner` executing
  `upvar 1 rel::x y` for a caller in `::A` links `y` to `::A::rel::x`; the
  same rule applies to `rel::array(key)` and to deeper numeric levels.
* `uplevel L {script}` sets the *active call frame* to frame `L` for the
  duration of `script`, then restores it. Both backends switch the current
  namespace with the frame (`tcl-vm::eval_at_level`, `runtime`'s
  `eval_uplevel`).

## Interactions that are modelled, not patched

* **`unset` through a link** unsets the *target* cell (firing its unset traces),
  then removes the local alias name. Unsetting a plain alias does not orphan
  the target's other aliases.
* **Traces are re-entrant interrupts.** A write trace runs arbitrary Tcl
  *during* the write, may itself read/write/unset (including the same variable
  through another alias), may error (the error reshapes the operation's
  result), and fires through `upvar` links in the frame where the access
  occurred. Ordering, error policy, and the re-entrancy guard are
  [variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md).
* **Storage lifetime is refcount-disciplined.** A scalar cell or array element
  owns +1 of its object; storing retains, overwriting/unsetting/dropping
  releases; links own nothing. `VarTable` releases on `Drop`, so a dropped
  frame or namespace cannot leak and every refcount move stays visible to the
  leak counters.
* **`tailcall`** replaces the current frame's pending continuation; it must not
  leave a dangling frame or double-pop.
* **Coroutines** capture and resume a *slice* of the call stack; `upvar` and
  `info level` inside a coroutine see the coroutine's stack, not the resumer's.
* **`rename`/aliases at the command layer** are the command-table parallel of
  this and share the re-entrancy hazards
  ([command-binding-and-aliasing.md](command-binding-and-aliasing.md),
  [runtime/rename-alias.md](../runtime/rename-alias.md)).

## Contract vs. incompatible-by-design

| Behaviour | Class | Notes |
|---|---|---|
| Unqualified name = frame local (not namespace var) inside a proc | **Contract** | The single most common semantic surprise; tests depend on it. |
| Unqualified name = namespace var at global / `namespace eval` scope | **Contract** | The `TclLookupSimpleVar` "no active proc frame" rule. |
| `upvar`/`global`/`variable` aliasing, link-following on read/write/unset | **Contract** | Including an alias to a not-yet-existing var. |
| `#N` vs `N` frame addressing, errors on out-of-range level | **Contract** | |
| Deterministic `info vars` / `array names` ordering | **Contract** | Oracle-diffed against tclsh. |
| Trace fire order, fire-through-link, error-replaces-result | **Contract** | Re-entrancy must terminate and match C Tcl ordering. |
| Exact wording of `can't read "x": no such variable` etc. | **Contract** | Error strings are tested verbatim. |
| `info level` / `info frame` *line and PC tables* | **Incompatible-by-design** | Tied to C Tcl's bytecode; a from-scratch codegen cannot match them byte-for-byte. Match the *structure* (`info level` command lists), classify the line tables. |
| Internal cell layout, refcount values exposed via test hooks | **Incompatible-by-design** | Object-rep probes never match. |

## See also

- [namespace-model.md](namespace-model.md) — the dialect namespace models.
- [command-resolution.md](command-resolution.md) — the *command*-name rule,
  which this is deliberately not.
- [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — where `uplevel`/`eval` hand off to the interpreter.
- [runtime/trace-implementation.md](../runtime/trace-implementation.md) —
  as-built trace dispatch.
