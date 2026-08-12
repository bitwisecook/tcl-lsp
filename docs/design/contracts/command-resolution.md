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
| Cross-document settlement (`settle_call_against_workspace`, `tcl-lsp-server`) — the **one** lookup shared by go-to-definition, find-references and the diagnostics path (W123 suppression + cross-file E002/E003) | replays the site's recorded `resolution_candidates` in priority order against the workspace index's existence oracle, applying the forced-import, nested-shadow and pending-indirection gates | `tcl-lsp-server/tests/e2e/issue1331_crossfile_diagnostics.rs` + `lib.rs` unit tests; see [cross-file-diagnostics.md](cross-file-diagnostics.md) |
| Optimiser interprocedural identity (`resolve_internal_call` / `resolve_call_target`), O103 folding (`resolve_proc_qname`) | `resolve_command_with` over the unit's proc table | unit tests in `interprocedural.rs` |
| `uplevel` passthrough inliner (`inline_uplevel.rs`) | `resolve_command_with` over the candidate map | existing inliner tests |
| Bytecode VM dispatch (`tcl-vm`: `lookup_command` = `resolve_command_fqn` + fetch; also `rename`'s source lookup, alias global anchoring, TclOO forward object-ns anchoring, `namespace unknown` chain, expr mathfunc dispatch) | `resolve_command_with` over the live command table, with the namespace's real `namespace path` | `tcl-vm/tests/command_resolution_conformance.rs` (compiles + executes every vector) + `tcl-vm/tests/tricky_resolution_e2e.rs` (tclsh-pinned alias/forward/mathfunc/unknown/rename interactions) |
| WASM runtime dispatch (`runtime/rust`: `Namespaces::home_of`) | structural mirror (its store is a namespace *tree*, not a flat map) — same base order, command-existence-checked per base | `cmd_namespace.rs::dispatch_matches_every_conformance_vector` (executes every vector) |
| eBPF backend (`bpf-tcl-*`) | **N/A** — no user procs, no namespaces; the 24-verb DSL rejects anything else as a hard compile error, so there is nothing to resolve |
| Compiler codegen | **no static binding** — proc calls emit runtime name dispatch (`invokeStk`), inheriting the *runtime's* conformance via eval-delegation (the WASM runtime row above; the bytecode VM's row covers `tcl-vm` execution) |

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
  The export gate is a **snapshot taken at the import site**, not a
  standing subscription: the import binds the names exported when it runs,
  and neither a later `namespace export -clear` (which does **not** revoke
  the alias — pinned) nor a later `namespace export` (which does **not**
  add one retroactively — pinned) reaches back. A second, later import
  takes its own snapshot and may bind names the first could not (pinned).
  `namespace export` is **additive** across calls, and `-clear` empties the
  list before the same call's own patterns are appended (both pinned).
  Export patterns are glob *patterns* matched against a command's tail
  name, never `::`-qualified (`namespace export ::foo` is an error —
  pinned), and are not references to commands: `namespace export p` before
  `proc p` still exports it (pinned). A non-glob `namespace import` of an
  unexported name is a silent no-op, not an error (pinned) — so the **exact**
  form is snapshot-gated exactly like the glob one, including a pattern rooted
  at the global namespace (`namespace import ::p`, whose source namespace is
  `::`, not "none" — pinned); importing onto
  an existing command errors unless `-force`, whichever of the two spellings
  bound the name first (pinned). Both subcommands
  consume at most **one** leading flag word: a second `-clear` is an ordinary
  export pattern (and `-clear` is then a genuinely importable command name),
  a second — or trailing — `-force` is an import pattern that aborts the
  script, and neither flag abbreviates (`-c` / `-f` are patterns, not
  options) — all pinned, and declared as
  `SubCommand::max_leading_option_words`. Statically the
  analyser records exports as an ordered event log
  (`SignatureNamespaceExport`, with `-clear` tombstones) and both LSP tiers
  answer through one shared decision function
  (`tcl_lsp_core::namespace_import::exported_at_import_site`), under one
  shared execution-order rule (`analyser::indirection::in_effect` /
  `in_effect_within`). Issue #1027.

  The import edge is a **link with a lifecycle**, not a standing
  name-visibility fact (issue #1103; every row below oracle-pinned on tclsh
  9.0.4 + 8.6.14, byte-identical):

  - **`namespace forget` removes the alias.** `namespace forget ::src::p`
    empties `info commands ::dst::*` of it and a later bare call is `invalid
    command name`. Both `Tcl_ForgetImport` pattern shapes hold: a *qualified*
    pattern names the source namespace whose imports are dropped, a *simple*
    one matches the forgetting namespace's own imported command names whatever
    their origin. Forgetting a name that was never imported is a silent no-op;
    only an unknown namespace in a qualified pattern errors. A later re-import
    reinstates the alias.
  - **`-force` decides the conflict.** Importing onto a name the target
    namespace already holds aborts (`can't import command "p": already
    exists`) and installs nothing — the existing command survives and
    `namespace origin` still answers it; with `-force` the import silently
    replaces it and `namespace origin` answers the source. "Already holds"
    covers a local `proc`/class **and a live alias from a different source**:
    with `::dst` importing `::A::*`, a later unforced `namespace import ::B::*`
    raises the same error and leaves `origin` at `::A::p`, while `-force`
    makes it `::B::p` and a preceding `namespace forget` lets the unforced one
    through (all pinned). Re-importing from the *same* source is a silent
    no-op, not a conflict (pinned). Which of the two ran first is **load
    order**, not written order: a body-local `namespace import ::B::x` loses to
    a top-level `namespace import ::A::*` written below it, because the file
    loads before any body runs — `namespace origin ::dst::x` → `::A::x` and the
    body's import raises `already exists` (pinned). Statically the "installed
    nothing" / "replaced what was there" halves are both modelled; the error's
    *control-flow* consequence (nothing after it in that script runs) is
    deliberately out of scope.

    Whether a `-force` import deleted the local command is a **whole-program**
    question even when everything else about it is local, so the
    single-document tier takes a whole-program **export oracle**
    (`tcl_lsp_core::namespace_import::NamespaceExportOracle`, implemented by
    `WorkspaceIndex::export_snapshot`) rather than reading one file (issue
    #1116 item 1). Two programs whose importing document is *byte-identical*
    disagree, decided entirely by another file (pinned, tclsh 8.6.14 + 9.0.4):
    with `::src` holding `proc helper`, `proc other` and `namespace export
    other` in that document, a `-force` import of `::src::*` over a local
    `::app::helper` reaches `::src::helper` when some other file also declares
    `namespace eval ::src {namespace export helper}`, and the local
    `::app::helper` when nothing does. The oracle's answer is three-valued
    (`ExportVerdict`) and the two questions abstain in opposite directions:
    "what does this call reach?" installs only on a proven export, "did
    `-force` delete the local command?" installs on anything but a proven
    *non*-export — answering with a command the import may have removed is
    worse than answering nothing and letting the cross-document tier resolve
    it. The oracle is **optional**: a host with no workspace index (the `tcl`
    CLI, a single-document test, an unindexed buffer) keeps the document-only
    rule, under which absence of an export is evidence only where that
    document holds some export record for the namespace. It is carried as one
    context (`tcl_lsp_core::definition::CallResolution`) through
    `resolve_called_proc` and every provider that calls it, never as a global.
  - **Redefining the imported name ends the alias.** A `proc ::dst::p` written
    after the import silently recreates `::dst::p` as an ordinary command:
    no error, `::dst::p` runs the new body, `namespace origin ::dst::p`
    becomes `::dst::p`, and `::src::p` is untouched (pinned). It is an
    ordered event like every other — a call written *between* the import and
    the redefinition still reaches the source.
  - **A source *delete* kills the alias, a source *rename* does not.**
    `rename ::src::p {}` makes `::dst::p` an `invalid command name`, while
    `rename ::src::p ::src::pp` leaves it working and merely moves the origin
    — the alias holds the command *object*, the same
    rename-captures-object-identity rule `analyser::indirection` already
    models. A redefinition of the source is seen straight through the link.
  - **Chains follow.** `::A` importing `::B::*` where `::B` imported `::C::*`
    makes `::A::p` run `::C`'s body, with `namespace origin ::A::p` → `::C::p`;
    a forget anywhere along the chain kills the whole thing (deleting an
    imported command deletes the commands imported from it). Both LSP tiers
    follow the chain while every hop is provable, bounded by
    `analyser::indirection::MAX_COMMAND_NAME_HOPS`, and abstain otherwise.

  - **The install is ordered against the call, too.** A bare call written
    *before* its own `namespace import` reaches nothing (`invalid command
    name`); the same call written after it works (pinned). That order is the
    ordinary load-time one, so a call inside a **proc body** still resolves
    through an import written later in the same file — the whole file loads,
    imports included, before any body runs (pinned: procs first, `namespace
    import` last, the call works) — while an import written after the call in
    that *same* body has genuinely not run yet (pinned). Issue #1104 item 1.

  Statically the removals join the same ordered event log: `namespace forget`
  as `SignatureNamespaceForget`, a destroying `rename OLD {}` /
  `interp alias {} NAME {}` as `AnalysisResult::destroyed_commands` (a *re*name
  is deliberately absent from it), and `-force` as
  `SignatureNamespaceImport::forced` — read, like `-clear`, as "the declared
  leading option word was consumed", never as a `-force` name match. Both tiers
  answer through one shared decision function
  (`tcl_lsp_core::namespace_import::alias_live_at`), which gates **every**
  ordered event — installs as much as removals — under the same
  `in_effect` / `in_effect_within` order rule as the export snapshot, and it
  gates the **exact**-import link the same way it gates a glob lookup.
  Byte offsets order events only *within* one document, so a cross-file event
  — an install as much as a removal — is ordered only where the workspace
  **proves an order** (`tcl_lsp_core::source_graph::RunOrder`, issues #1104
  item 3 and #1279): sourcing a file inlines its whole body at the `source`
  statement's position, so the DFS of the `source` forest is the run order and
  two events in one tree reduce to positions in their deepest common document,
  where the ordinary single-document rule applies.  A `package require`
  additionally proves that the file `package provide`ing that package has
  *already* run — one-sidedly, since a require of an already-loaded package
  evaluates nothing, so it never proves the provider has not run yet. An export sourced *before* an import
  counts; one sourced *after* it is not retroactive; a `namespace forget`
  written beside a `source` revokes what that `source` installed (pinned, both
  tiers and end-to-end). Everywhere else the order abstains — different trees,
  a file reachable from two `source` sites or on a cycle, a `source` path the
  host cannot prove statically — and removals then abstain toward *keeping*
  the alias: one that cannot be ranked revokes nothing, and one written inside
  a proc/class body is conditional and is not published at all. The one
  exception is **destroying the source command**, which is not a slot event on
  a timeline but the disappearance of the command object workspace-wide: it
  revokes a link regardless of where it is written.
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
- **Execution traces** observe dispatch; they never alter it. Both
  backends key command and execution traces by the resolved FQN and fire
  them from their dispatch chokepoint (`tcl-vm`'s `cmd_trace.rs` /
  `interp.rs`, `runtime/rust`'s `Interp::fire_cmd_trace` /
  `Interp::dispatch`).
- **`args` variadics and alias prepended words** are an *arity* surface:
  the alias prepend happens before the target's own arity check
  (prepend-then-invoke, matching C).
- **`upvar` / `variable` / `namespace upvar`** are the *variable*
  resolution surface, which is deliberately **not** this rule — a
  qualified variable write commits to the first namespace its qualifier
  resolves to, with no existence-checked fall-through.  The variable /
  call-frame model (`VAR_LINK` aliasing, `upvar` to a statically-unknown
  caller frame, the `#0` vs non-`#0` `uplevel` frame distinction) is its own
  contract, [runtime-variable-frame-model.md](runtime-variable-frame-model.md).

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
- Command names held in **variables** (`set cmd helper; $cmd …`) settle
  against the compiler's flow-sensitive value model
  (`value_provenance::const_contributors`) when every reaching definition is
  a written literal: each may-target of a branch join is referenced, the
  `$cmd` head is navigation-only and the contributing literals carry the
  rename edit. Anything unprovable — a computed value, a parameter, an
  `upvar`/trace write, an opaque `catch` body — abstains, and a contributor
  with no writable span refuses the whole rename. See
  [name-resolution.md](../name-resolution.md) §3.4.
- **`interp eval` bodies** get their own synthetic
  `@interp@<path>[#<epoch>]` scope domain, so definitions made inside one
  (`proc`, `oo::class`, variables) never merge into the parent namespace
  and two evals into two different children never merge with each other.
  Command-table *mutations* written inside such a body — `rename`, and an
  `interp alias` with an empty (`{}`) interpreter path, both of which act on
  "the interpreter I am running in" — are likewise scoped to that child:
  `rename` abstains from the file-wide rename/deletion maps rather than
  making the parent's own builtin look deleted, and an empty-path
  `interp alias` homes under the child's domain
  (tclsh 9.0.4-verified in both directions; issue #1141's flaw class).
  What is still approximate: the *content* of a child's command table is
  not modelled as a separate universe, so a diagnostic that would depend on
  a rename having happened **inside** the child is not emitted at all
  (silence, never a wrong answer).  A dynamic (`interp eval $handle {…}`)
  target that cannot be resolved to a tracked interpreter keeps its own
  domain but is marked unresolved, and consumers widen across it.
- **`source` call-site namespaces** are propagated by seeded re-analysis
  (`Analyser::analyse_with_source_namespace` — the static
  `namespace eval <ns> {<file>}`), reconciled lazily before every
  cross-document query, and a document sourced under several namespaces
  carries one runtime identity per seed. A `source` path the folder cannot
  prove abstains rather than guessing. See
  [name-resolution.md](../name-resolution.md) §3.2.
- **`expr` function names** are routed through command resolution: each
  `f(…)` application is recorded as an invocation carrying
  `is_mathfunc_call`, settled with the `namespace path`-aware candidate
  builder over `tcl::mathfunc::f`, so a user `proc ::tcl::mathfunc::f` — a
  namespace-local override included — is linked to its `expr` uses. The
  per-release function set is gated by
  `tcl_syntax::expr::mathfunc::added_in`. See
  [name-resolution.md](../name-resolution.md) §7.
