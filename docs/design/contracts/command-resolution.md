# Contract: command-name resolution — one algorithm, everywhere

Every component that answers "which command does this call dispatch?" must
implement **C Tcl's** rule, and must prove it by passing the shared
conformance vectors. This contract names the algorithm, its single Rust
home, every consumer, and the anti-drift gates.

## The algorithm (C Tcl 9, `Tcl_FindCommand`, `generic/tclNamesp.c`)

Given a call to `name` from current namespace `ns` whose
`namespace path` is `P1 … Pn`:

1. **Absolute** (`::`-led) name: resolve from the global namespace only.
   The path is never consulted.
2. **Relative** name — bare (`helper`) *or* qualifier-carrying
   (`inner::p`), identically: try, in order, `ns`, then `P1 … Pn`, then
   the global namespace as the base; the call dispatches the **first base
   under which the command exists**. A *relative* path entry is
   **current-namespace-relative only**: `namespace path inner` inside
   `::outer` means `::outer::inner`, never `::inner` — the set errors
   (`namespace "inner" not found in "::outer"`) when `::outer::inner`
   does not exist, even with `::inner` present. Namespace names have no
   global fallback, unlike command names (tclsh-pinned).
3. Existence means the *command* exists. A qualifier namespace merely
   existing does not commit resolution: `inner::p` from `::outer`
   dispatches `::inner::p` even when the namespace `::outer::inner`
   exists but holds no `p`.
4. No implicit ancestor walk: `helper` inside `::a::b` never reaches
   `::a::helper` unless `::a` is on the path.
5. Resolution happens at **call time**: a candidate defined later in the
   file (or dynamically) wins when the call finally runs.

Behaviour pinned against real tclsh 8.6.16 and 9.0.4 (they agree on every
case, including all edge cases above).

## Canonical implementation

[`tcl_syntax::naming`](../../../rust/tcl-syntax/src/naming.rs):

| Helper | Role |
|---|---|
| `command_resolution_candidates(ns, path, name)` | Candidate list in priority order (absolute → single candidate; relative → `ns`, path entries, global; deduped). Roots a relative path entry against `ns` (the current-namespace-relative-only rule above), so callers may pass entries as written. |
| `bareword_resolution_candidates(ns, name)` | Path-free wrapper for consumers that do not model `namespace path`. |
| `resolve_command_with(ns, path, name, exists)` | The full rule: first candidate for which `exists` is true, `None` = `invalid command name`. |
| `naming::conformance` | The shared vector table (`tests/data/command_resolution_vectors.txt`), its parser, and `vector_script` (renders a vector as runnable Tcl). |

## Consumers and their conformance gates

| Consumer | How it conforms | Gate |
|---|---|---|
| Analyser call-site settlement (`Analyser::finalise_invocation_resolutions`, feeds `resolved_qualified_name` for references / rename / call hierarchy / code lens / symbol graph / minifier) | calls `resolve_command_with` post-walk (call-time semantics: whole-file definitions count) with the namespace's statically-recorded `namespace path` (`handle_namespace_path_command` tracks literal declarations; each replaces the whole path, as in C Tcl) | `tcl-compiler/tests/command_resolution_conformance.rs` (every vector, path-carrying included) |
| Analyser shadow/arity checks (`resolve_proc_call`), W-code validity (`qualify_candidates`) | shared candidate helper | unit tests in `handlers.rs` / `validity.rs` |
| Optimiser interprocedural identity (`resolve_internal_call` / `resolve_call_target`), O103 folding (`resolve_proc_qname`) | `resolve_command_with` over the unit's proc table | unit tests in `interprocedural.rs` |
| `uplevel` passthrough inliner (`inline_uplevel.rs`) | `resolve_command_with` over the candidate map | existing inliner tests |
| Bytecode VM dispatch (`tcl-vm`: `lookup_command` = `resolve_command_fqn` + fetch; also `rename`'s source lookup, alias global anchoring, TclOO forward object-ns anchoring, `namespace unknown` chain, expr mathfunc dispatch) | `resolve_command_with` over the live command table, with the namespace's real `namespace path` | `tcl-vm/tests/command_resolution_conformance.rs` (compiles + executes every vector) + `tcl-vm/tests/tricky_resolution_e2e.rs` (tclsh-pinned alias/forward/mathfunc/unknown/rename interactions) |
| WASM runtime dispatch (`runtime/rust`: `Namespaces::home_of`) | structural mirror (its store is a namespace *tree*, not a flat map) — same base order, command-existence-checked per base | `cmd_namespace.rs::dispatch_matches_every_conformance_vector` (executes every vector) |
| eBPF backend (`bpf-tcl-*`) | **N/A** — no user procs, no namespaces; the 24-verb DSL rejects anything else as a hard compile error, so there is nothing to resolve |
| Compiler codegen | **no static binding** — proc calls emit runtime name dispatch (`invokeStk`), inheriting the VM's conformance |

The vector table itself is pinned to C Tcl by
`tcl-syntax/tests/command_resolution_conformance.rs::vectors_match_real_tclsh`,
which executes every vector under a real tclsh (`TCL_LSP_TCLSH`, else
`tclsh9.0` / `tclsh8.6` / `tclsh` on `PATH`; skips when none is
installed). **Adding a resolution behaviour = adding a vector**: every
implementation then has to pass it or fail its own suite — drift is a test
failure, not a code review hope.

## What changes which input, not the algorithm

These features interact with resolution by changing its *inputs* — they do
not change the rule, and must not grow bespoke resolution logic. Every
"pinned" fact below was verified on **both** tclsh 8.6.16 and 9.0.4
(they agree on all of them):

- **`namespace eval` / proc bodies** select `ns`: a body resolves in its
  *defining* namespace, not its caller's. Plain **`eval`** keeps the
  current namespace (pinned).
- **TclOO method / constructor / destructor bodies** run with the
  **object's instance namespace** current (`::oo::ObjN`, whose
  `namespace path` is `::oo::Helpers` — home of `next`/`self`; `my` is
  an object-namespace command). Bare *and* relative-qualified names
  resolve object-ns → Helpers → **global**; the class's defining
  namespace is **never** searched (pinned: a helper proc in the class's
  defining namespace is unreachable unqualified; the global one wins).
  `forward` targets resolve the same way at call time (pinned even when
  the caller's namespace has a same-named proc). Statically the object
  namespace is unknowable, so the analyser approximates TclOO method
  bodies as **global-only** (`Scope::oo_global_resolution`); the VM runs
  method bodies in the object namespace and anchors `forward` there.
  snit / itcl members genuinely resolve in the type / class namespace
  and keep the defining-namespace rule.
- **`uplevel`** selects the *frame* whose namespace is current: the
  script resolves in the target frame's namespace (`#0` → global) —
  pinned; both execution backends switch namespace with the frame
  (`tcl-vm::eval_at_level`, `runtime`'s `eval_uplevel`).
- **`interp alias`** names are ordinary table entries (any namespace);
  the **target** is re-resolved **by name, from the GLOBAL namespace, at
  every call** — late-bound (a target defined later works; a deleted
  target errors lazily; a renamed target is not followed), with the
  caller's *frame* kept (an alias to `set` writes the caller's locals) —
  all pinned. A miss still reaches `unknown` (pinned). See
  [command-alias-resolution.md](command-alias-resolution.md) and
  [command-binding-and-aliasing.md](command-binding-and-aliasing.md).
- **`rename`** mutates the table: the *source* is found by the **full
  rule** (`namespace path` included — pinned), the *destination* roots
  at the current namespace and **auto-creates** missing parent
  namespaces (pinned). Builtins are renameable like anything else
  (pinned: `rename ::return ::myreturn` succeeds) — and a builtin
  renamed away no longer resolves under its old name (the analyser's
  settlement honours this).
- **`namespace import`** installs real entries gated by `namespace
  export`; in C they hold the source's command *token*, so a source
  **rename is followed** (the import keeps working; `namespace origin`
  reports the new name) while a source **delete dangles** (both pinned).
  The WASM runtime retargets its by-name redirects on rename to match;
  the VM's clone-model import (a later redefinition of the origin is not
  seen) is a **known divergence**, documented here.
- **`unknown` / `namespace unknown`** fire only after the full candidate
  walk (path included) misses. `namespace unknown` handlers are
  **per-namespace, NOT inherited** by children; the global namespace's
  handler is the interp-wide default; a namespace's own handler beats
  the plain `::unknown` proc (all pinned). Both execution backends
  implement this chain; statically only the global `unknown` proc's
  shape gates W123.
- **`::tcl::mathfunc` / `::tcl::mathop`** are ordinary commands under
  this rule: `expr` resolves a function `f` as the **relative** name
  `tcl::mathfunc::f` from the current namespace (namespace-local
  shadowing works — pinned, and vectored in the shared table), a
  missing function reports the command miss (`invalid command name
  "tcl::mathfunc::f"` — pinned), and TIP 232 `proc`-defined functions
  dispatch like builtins. `expr` *operators* compile natively and never
  consult `::tcl::mathop`; the mathop commands exist for direct calls
  (typically via `namespace path ::tcl::mathop`).
- **`source`** evaluates the file in the **caller's current namespace**
  (pinned: `namespace eval ::ns { source f.tcl }` lands `f.tcl`'s bare
  `proc`s in `::ns`). Both backends do this; the static cross-file model
  treats each file as its own global-rooted unit (documented gap below).
- **Safe / sub-interpreters** are entirely separate command tables
  (pinned: nothing crosses the boundary; safe interps *hide* commands).
  Both backends model children as separate interpreters with a hidden
  table. Statically, an `interp eval` script argument is still walked in
  the parent's context for injection diagnostics — its name bindings are
  a **known approximation**, documented below.
- **Execution traces** observe dispatch; they never alter it (both
  backends key traces by the resolved FQN; the VM accepts but does not
  yet fire command/execution traces — a completeness gap, not a
  resolution one).
- **`args` variadics and alias prepended words** are an *arity* surface:
  the alias prepend happens before the target's own arity check
  (prepend-then-invoke, matching C).
- **`upvar` / `variable` / `namespace upvar`** are the *variable*
  resolution surface, which is deliberately **not** this rule — a
  qualified variable write commits to the first namespace its qualifier
  resolves to, with no existence-checked fall-through (see
  `runtime/rust`'s `var_home` and
  [namespace-model.md](namespace-model.md)).

## Known modelling gaps (static side)

- The analyser tracks **literal** `namespace path {…}` declarations only:
  a dynamic list (`namespace path $entries` / `[…]`) is statically
  unknowable and keeps the conservative empty path. Path knowledge is
  whole-file (call-time semantics, like the rest of the settlement) —
  the lexically-last declaration per namespace wins, with no
  between-declarations ordering within one namespace.
- The **walk-time** heuristics (shadow/arity `resolve_proc_call`, W-code
  `qualify_candidates`) still resolve pathlessly — a path declaration may
  lexically follow the site they fire at. The settled
  `resolved_qualified_name` (what references / rename / call hierarchy /
  code lens consume) is the path-aware, conformance-gated value.
- **Custom command resolvers** (`Tcl_SetNamespaceResolvers`, C-level) and
  `namespace unknown` handlers are out of scope for static resolution;
  the runtime's `unknown` fallback fires only after this rule misses.
- Command names held in **variables** (`set cmd helper; $cmd …`) are
  statically undecidable and stay unresolved (pinned by LSP tests as a
  documented limitation).
- **`interp eval` bodies** are walked in the parent interpreter's
  context (for the injection diagnostics); a child interpreter's
  separate command table is not modelled, so name bindings inside those
  scripts are approximate.
- **`source` call-site namespaces** are not propagated cross-file: the
  static model analyses each file as its own global-rooted unit, so a
  file sourced inside `namespace eval` is not re-homed (cross-file
  matching is by qualified-name candidates and tails).
- **`expr` function names** are statically modelled as expression
  stubs, not routed through command resolution — a user
  `proc ::tcl::mathfunc::f` is not linked to `f(…)` uses inside `expr`
  (both execution backends resolve them correctly at run time).
