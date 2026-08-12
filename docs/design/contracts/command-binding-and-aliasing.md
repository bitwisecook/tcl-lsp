# Contract: command binding, aliasing & resolution

> The one resolution model all command-name indirection shares —
> `rename`, `interp alias`, `namespace import`/`export`/`forget`,
> `namespace path`, ensembles, and the `::tcl::mathop` / `::tcl::mathfunc`
> operator commands — and how an AOT compiler may snapshot it safely. The
> as-built mechanics are [runtime/rename-alias.md](../runtime/rename-alias.md),
> [runtime/namespace-tree.md](../runtime/namespace-tree.md), and
> [runtime/command-introspection.md](../runtime/command-introspection.md); the
> LSP-side alias tracking is
> [command-alias-resolution.md](command-alias-resolution.md). This is the
> command-layer parallel of
> [runtime-variable-frame-model.md](runtime-variable-frame-model.md).

## Why resolution is a runtime function

A command name does not name a fixed implementation. `rename` moves a binding,
`interp alias` adds a redirect with frozen prefix args, `namespace import`
installs a transparent redirect, an ensemble maps a subcommand to a target,
`namespace path` adds search fallback, and a plain `proc` can shadow a builtin
— all at runtime, often mid-eval. An AOT compiler that resolves `+` or `dict
for` or an aliased `=` *once* and bakes in the answer is correct only until any
of those mutate the binding. Resolution is a runtime function with a guarded
compile-time cache, never a compile-time fact.

**Design rule:** there is exactly one way to reach a command —
`resolve(ns, name) → target`, evaluated against the command tables *as they
are at the moment of the call*. Compile-time resolution
(`canonical_command`) is a snapshot of that function behind a binding-stability
guard; it is never a parallel source of truth.

## The resolution function (the contract)

`resolve(currentNs, name) → *Command | unknown`:

1. **Parse the name.** Leading or embedded `::` ⇒ *qualified* (absolute if
   leading `::`, else relative to `currentNs`); otherwise *unqualified*.
2. **Qualified `::a::b::cmd`:** resolve namespace `::a::b`, look up `cmd` in
   its command table directly. No path search, no import fallback.
3. **Unqualified `cmd`**, in order:
   1. `currentNs`'s command table,
   2. each namespace on `currentNs`'s **`namespace path`** (in declared
      order),
   3. the global namespace `::`,
   4. the **`unknown`** command (auto-load, ensemble-unknown, or the
      `invalid command name "cmd"` error).
4. **Follow the binding.** A resolved Command may itself be a *redirect*
   (`namespace import` → unwrap transparently to the terminal source) or an
   *alias* (`interp alias` → do **not** unwrap; the trampoline re-resolves the
   stored target by name on dispatch). An ensemble Command maps a *subcommand*
   to a further target (step 6).

Resolution is by table state at call time — never memoised across an eval,
trace, or sourced-file boundary.

## The binding forms

| Form | What it installs | Mutation trigger | Unwrapped by lookup? |
|---|---|---|---|
| `proc` / builtin | a terminal Command | redefinition shadows | — |
| `rename old new` | moves Command old→new; `new=""` deletes | rename/delete | — |
| `interp alias {} new {} tgt ?pre…?` | redirect to `tgt` with frozen prefix `pre` | create/delete; lazily sees `tgt` deletion | **No** (queryable) |
| `namespace import ns::pat` | `CMD_IMPORTED` redirect → source | source rename/delete, `namespace forget` | **Yes** (transparent) |
| `namespace export pat` | export *gate* only | re-export | n/a (creates nothing) |
| `namespace path {ns…}` | command search fallback | path change | n/a (pure search) |
| ensemble (`dict`, `string`, …) | subcommand→target map | `namespace ensemble configure -map` | n/a (maps, see §6) |
| `::tcl::mathop::+` etc. | real operator commands | overridable like any command | — |

### rename
`rename old new` moves the Command between namespace command tables;
`rename old ""` deletes and must splice the command out of every importer's
redirect list and deactivate those redirects. Self-rename is a no-op.
Built-ins are protected (`return`, `error`) and refused verbatim
(`can't rename "X": …`). Renaming a command does **not** chase existing
`interp alias` targets — an alias stores the target *name*, so after a rename
the stored name simply stops resolving (matches C).

### interp alias
The alias trampoline re-resolves the stored target **by name on every
dispatch, anchored at the global namespace**, and prepends the frozen prefix
args. It therefore lazily observes target *deletion* (the alias then errors)
but does **not** follow target *rename*. Aliases are **not** unwrapped by
lookup (so `interp alias {} new` can introspect the redirect), whereas imports
**are**. Chains (`a→b`, `b→expr`) are literal and resolved per call — never
pre-flattened.

### namespace import / export / forget
`export` only *gates* what `import` may pull; it installs nothing. `import`
installs a transparent `CMD_IMPORTED` redirect that lookups unwrap to the
terminal source; it is a **snapshot** of matching exported commands at import
time (commands added to the source later are not retroactively imported).
Renaming/deleting the source, or `namespace forget`, splices the redirect out.

### namespace path & ::tcl::mathop / ::tcl::mathfunc
`namespace path` is pure search fallback (step 3.2), not a redirect. The
operator commands live in `::tcl::mathop` and math functions in
`::tcl::mathfunc`; as *commands* they are reachable only by qualification or
by putting `::tcl::mathop` on the path (`namespace path ::tcl::mathop; + 1 2`).

**Do not conflate `expr`'s internal dispatch with the command path.**
`expr {1+2}` does not consult the command table; `+ 1 2` (as a command) does.
But `::tcl::mathfunc::foo` *is* overridable and `expr`'s function-call path
resolves it through the command table — model that single hook, not two.

## Ensembles (§6)

An ensemble command maps `ens sub …` → a target: default `::ens::sub`, or via
`-map`. The subcommand is resolved by unambiguous prefix-abbreviation unless
`-prefix 0`; `-subcommands` restricts the set; `-unknown` handles misses.
Treat ensemble subcommand dispatch as a **resolution step declared in the
registry**, so `string cat`, `dict get`, `info exists` resolve uniformly. The
current compiler already rewrites `dict for`/`dict map` to the canonical
`::tcl::dict::for` / `::tcl::dict::map` (an interpreter barrier) — that
rewrite *is* the ensemble alias; generalise it rather than special-casing each.

## AOT compiler implications — the binding lattice

`canonical_command` is a **lowering-time snapshot** of `resolve` (alias /
import / qualified spelling / ensemble rewrite collapsed to `::ns::cmd`). It is
sound only while the binding cannot have changed by call time. The
stability assumption is violated by `rename`, `interp alias` create/delete,
`namespace import`/`forget`, a `namespace path` change, and `proc`
redefinition.

* **Binding lattice (the guard).** Track a binding state per command name:
  *pristine-builtin → user-proc → aliased → renamed-away → shadowed*. **Only
  `pristine-builtin` may take an inline fast path**; any rebinding op demotes
  the name and forces dispatch through the live runtime command table. The
  as-built `builtin_is_trusted` check is the seed of this — make it
  first-class and monotonic.
* **Keep both spellings.** Match optimiser/registry patterns on
  the *canonical* form, but retain the *source* spelling for the eval-fallback:
  the user's bare name resolves through the live scope walk (a namespace-local
  proc), whereas an eagerly-globalised `::name` would miss it.
* **Invalidate coarsely.** Resolution caches (proc LRU, path-resolution,
  ensemble maps) must be epoch-invalidated on every rebinding op.
  Coarse-but-correct (wipe the LRU on any rename) beats a clever partial
  invalidation.
* **Barriers for the unresolvable.** When a call's binding can't be proven
  stable, lower it to an interpreter barrier (a real invoke), not an inlined
  builtin — the same discipline the qualified-`foreach` fallback needs (see
  [compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md)).

## Hazards to design in (not patch)

* **Re-entrancy.** An alias / rename / import may be created mid-eval (inside a
  trace, an ensemble `-unknown`, or a sourced file). Resolution must always be
  against current table state.
* **Cycles & chains.** `alias→alias`, import-of-import, `rename a b; rename b
  a`. Resolve by-name per call (lazy) so creation can't loop; detect
  self-cycles where C does.
* **Aliasing ≍ traces.** This is the command-layer parallel of variable traces
  ([variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md)):
  both are re-entrant, both invalidate compile-time assumptions, both must
  funnel through one resolver/dispatcher.

## Contract vs. incompatible-by-design

| Behaviour | Class | Notes |
|---|---|---|
| Unqualified resolution order: current ns → path → global → unknown | **Contract** | Path makes bare `+` resolve via `::tcl::mathop`. |
| `rename` move/delete, importer splice, built-in protection | **Contract** | `can't rename "X": …` verbatim. |
| `interp alias` frozen prefix, by-name target re-resolve, no rename-follow | **Contract** | Lazily sees target deletion only. |
| `import` transparent unwrap; snapshot-at-import; `forget`/source-rename splice | **Contract** | Imports unwrapped, aliases not. |
| Ensemble subcommand prefix-match, `-map`/`-unknown`/`-subcommands` | **Contract** | `dict for`→`::tcl::dict::for` rewrite. |
| `::tcl::mathfunc::X` overrides seen by `expr`; `::tcl::mathop` as commands | **Contract** | Don't conflate with `expr`'s internal op dispatch. |
| Error strings (`invalid command name`, `can't import …`, ambiguous subcmd) | **Contract** | Tested verbatim. |
| Internal Command struct layout, redirect-list pointers, LRU contents | **Incompatible-by-design** | Object-rep / refcount probes never match. |
| Which calls C bytecode-compiles vs invokes | **Internal** | Mirror the observable dispatch result, not C's codegen choice. |

## See also

- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) — the
  variable-layer parallel (cell/frame indirection, aliasing, re-entrancy).
- [compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md) —
  the binding lattice's sibling for *variable* scope and barrier lowering.
- [namespace-model.md](namespace-model.md) — namespace scope tracking.
- [runtime/rename-alias.md](../runtime/rename-alias.md),
  [runtime/namespace-tree.md](../runtime/namespace-tree.md),
  [runtime/command-introspection.md](../runtime/command-introspection.md) —
  as-built dispatch, redirect lists, and the rename sidecar.
- [command-alias-resolution.md](command-alias-resolution.md) — LSP/analyser
  `interp alias` tracking (the static, editor-facing slice).
