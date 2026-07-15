# Name resolution — master fix plan

**Status:** execution plan. No code here has landed. This is the actionable
plan behind four companion studies:
[name-resolution-centralization.md](name-resolution-centralization.md) (the
duplication audit), [cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md)
(#923 cross-file work), [name-resolution-tcl-version-and-c-source.md](name-resolution-tcl-version-and-c-source.md)
(the C-source-grounded 8.4→9.1 conformance study, findings D1–D11 / N1–N8), and
[tricky-name-resolution-surfaces.md](tricky-name-resolution-surfaces.md) (the
dynamic-surface navigation-link audit). Read those for the *why*; this document
is the *how*, *in what order*, with a task checklist and direct source link per
step.

Structure: **16 milestones**, each split into **stages** (two levels, no
deeper). Milestones are ordered by (severity of what they fix) × (independence
from un-built work). Every stage is a checklist.

## How the source links work

Every link is a **commit-pinned** GitHub permalink so line numbers never drift.

- **This repo** → the **v2.1.9** release commit
  [`6a6bc87`](https://github.com/bitwisecook/tcl-lsp/commit/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63)
  (the validated code; this branch adds only docs on top).
- **C Tcl** → `tcltk/tcl` at the exact release tags studied:
  8.4.20 [`9ccfe9d1`](https://github.com/tcltk/tcl/tree/9ccfe9d1b35741ff7323837f6485ffe48b06fad9),
  8.5.19 [`160d612a`](https://github.com/tcltk/tcl/tree/160d612a6b2b1c2c0db27236d648b7bc1364570c),
  8.6.16 [`874e4fe4`](https://github.com/tcltk/tcl/tree/874e4fe4264a40c00c4db5115afba9600f9f368d),
  9.0.4 [`c655b477`](https://github.com/tcltk/tcl/tree/c655b4770b1d6d32a8cbffd6cef59db6029fe19e),
  9.1b0 [`fbe83207`](https://github.com/tcltk/tcl/tree/fbe83207a70634a5031c70bdce3d59071920f6da).

> **⚠ URGENT — confirmed source corruption; ship Stage 2.1 before anything
> else.** Renaming a **TclOO instance variable** rewrites the *entire method
> body*: `method get {} { return $n }`, rename `$n`→`w`, becomes `method get {} w`.
> Verified by hand:
> [`oo.rs:326`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/oo.rs#L326)
> (object var seeded with `def_span = None`) →
> [`scope.rs:696`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/scope.rs#L696)
> (`unwrap_or(tok.span)` → the body span) →
> [`rename.rs:631`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/rename.rs#L631)
> (replaces `var_def.definition_span` with just the name).

---

## Milestone index

| M | Title | Danger class | Depends on |
|---|---|---|---|
| **M1** | Target-selection consolidation (~17 sites) | wrong-target edit (silent) | — |
| **M2** | Variable resolution & VAR_LINK (incl. D1 corruption) | source corruption / wrong target | — |
| **M3** | Command name-link following (alias/rename/import/forward) | dangling rename (silent) | M1 |
| **M4** | Class-name resolution → one-hop rule | wrong inheritance edge (silent) | — |
| **M5** | Workspace-scoped resolution oracle (#923) | missed cross-file refs | — |
| **M6** | TclOO cross-file methods & `oo::define` merge | missed refs / stub class | M4, M5 |
| **M7** | Command-names-in-variables & dispatch tables | missed refs | M5 |
| **M8** | Library / autoload resolution tier | missed refs | M5 |
| **M9** | Source-site namespace propagation | wrong FQN cross-file | M5, M8 |
| **M10** | Dialect-aware command resolver (`namespace path`) | false resolve in 8.4 | — |
| **M11** | Cross-version variable semantics (9.0 fallback) | false (non-)resolve | M2 |
| **M12** | Expr-function fidelity (`::tcl::mathfunc`) | false resolve / missed | — |
| **M13** | TclOO version fidelity (`property`) | false resolve in 8.6 | M4 |
| **M14** | Dynamic reference roles (`ArgRole::CommandName`) | missed refs | M3 |
| **M15** | Interpreter/scope isolation & coverage | wrong edit (interp) / missed | M3, M5 |
| **M16** | VM behavioural parity | none (behavioural) | — |

---

## The duplication map (M1 work list)

The canonical, namespace-aware resolver already exists:
[`definition.rs:748 resolve_called_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L748)
→ [`:728 proc_visible_from_namespace`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L728)
→ [`naming.rs:455 command_resolution_candidates`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-syntax/src/naming.rs#L455).
Only `hover.rs`'s proc path uses it. Each site below re-derives its own
namespace-blind `all_procs.iter().find(name == word)` scan:

| # | Site | Breaks | Tier |
|---|---|---|---|
| 1 | [`rename.rs:654 rename_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/rename.rs#L654) | Rename → wrong same-named proc | 1 |
| 2 | [`rename.rs:735 rename_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/rename.rs#L735) | Rename → wrong same-named class | 1 |
| 3 | [`call_hierarchy.rs:127 find_proc_for_item`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/call_hierarchy.rs#L127) | Call-hierarchy wrong node | 1 |
| 4 | [`references.rs:388 proc_references`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/references.rs#L388) | Find-refs from call site → wrong proc | 1 |
| 5 | [`references.rs:340 class_references`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/references.rs#L340) | Find-refs from call site → wrong class | 1 |
| 6 | [`lib.rs:3631 resolve_workspace_symbol`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-server/src/lib.rs#L3631) | Cross-document rename/refs → wrong symbol in another file | 1 |
| 7 | [`linked_editing_range.rs:148 matches_self_call`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/linked_editing_range.rs#L148) | Live-links unrelated call site (OR-bug) | 1 |
| 8 | [`implementation.rs:155 strip_colons`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/implementation.rs#L155) | Go-to-impl misses bare `superclass` to namespaced base | 2 |
| 9 | [`signature_help.rs:302 lookup_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/signature_help.rs#L302) | Signature help on wrong proc | 2 |
| 10 | [`inlay_hints.rs:911 lookup_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/inlay_hints.rs#L911) | Inlay hints from wrong proc (dup of #9) | 2 |
| 11 | [`hover.rs:2198 lookup_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/hover.rs#L2198) | Hover wrong class docs | 2 |
| 12 | [`type_hierarchy.rs:54 prepare`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_hierarchy.rs#L54) | Type-hierarchy wrong class | 2 |
| 13 | [`workspace_index.rs:633 proc_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L633) | Cross-doc goto-def, ungated (also M5) | 2 |
| 14 | [`workspace_index.rs:645 class_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L645) | Cross-doc goto-def classes, ungated (also M5) | 2 |
| 15 | [`type_definition.rs:77 find_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_definition.rs#L77) | Go-to-type-def (narrow) | 3 |
| 16 | [`minify.rs:1196`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/minify.rs#L1196) | Wrong param list on collision | 3 |
| 17 | [`tools.rs:931 generate_docstring`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-mcp/src/tools.rs#L931) | MCP docstring arbitrary pick (+ stale comment) | 3 |

---

## M1 — Target-selection consolidation

Route the 17 sites through the existing correct resolver. Ground truth: it
already matches C's `Tcl_FindCommand`
([8.6 `tclNamesp.c:2528`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclNamesp.c#L2528),
[9.0 `:2640`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclNamesp.c#L2640)). No new algorithm.

**Stage 1.1 — shared entry points** ✅ **DONE**
- [x] Added `definition::resolve_proc_target_at` (decl-cover, else namespace-aware `resolve_called_proc`, returns the `all_procs` key + `ProcDef`) — never a namespace-blind `p.name == word` scan.
- [x] Added the class analogue `definition::resolve_class_target_at` (a class name is a command name → same `bareword_resolution_candidates` order).

**Stage 1.2 — Tier-1 migrations (silent-corruption bugs)** — same-file sites ✅ **DONE**
- [x] Migrated `rename_proc`, `rename_class`, `find_proc_for_item` (call-hierarchy fallback → namespace-aware from the item's own location), `proc_references`, `class_references` onto the shared resolvers.
- [x] Fixed the Linked-Editing OR-bug: `resolved_qualified_name` is now authoritative when `Some` (only falls back to `matches_self_call` when `None`), so a nested-namespace call to a different same-named proc no longer links.
- [ ] **Remaining:** `resolve_workspace_symbol` (cross-document, `tcl-lsp-server/src/lib.rs` — cross-crate re-export; deferred to land with M5's cross-doc work).
- [x] Verified: reproduced the wrong-symbol rename (renamed `::b::helper` from `::a`'s call site), then fixed. Tests — 4 unit (proc+class rename/refs from call sites, TP/FP/TN) + 1 e2e + 1 vscode; full `tcl-lsp-core` suite (969 lib + all integration) green, 0 regressions.

**Stage 1.3 — Tier-2 migrations** ✅ **DONE (LSP providers)**
- [x] Migrated #8 [`implementation.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/implementation.rs#L155): the class *target* now resolves via `resolve_class_target_at`, and — because `superclasses`/`mixins` hold names *as written* (a bare `superclass Shape` in `::A` stays `"Shape"`, which the leading-`::`-only tail compare never matched to the resolved `::A::Shape`) — the subclass edges now come from the owner-aware class-hierarchy index (`subclasses`, super + mixin unioned), the same source `type_hierarchy::subtypes` shares. Fixed a real false-negative (namespaced classes returned zero subclasses).
- [x] Migrated #9 [`signature_help.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/signature_help.rs#L302) + #10 [`inlay_hints.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/inlay_hints.rs#L911) onto the shared `resolve_called_proc` (namespace at the command token; the builtin gate stops a namespaced proc hijacking a same-named builtin from global scope; the lenient fallback is now deterministic). Both are also cheaper — O(candidates) probes vs an O(procs) scan.
- [x] Fixed #11 [`hover.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/hover.rs#L2198) (deleted `lookup_class`, routed through `resolve_class_target_at`) and #12 [`type_hierarchy.rs prepare`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_hierarchy.rs#L54).
- [x] TP/FP/TN coverage added at each site (same-named class/proc in two namespaces resolves to the cursor's namespace; builtin gate; deterministic fallback).
- [ ] **Deferred (lands with M5):** ambiguity-gate [`proc_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L633)/[`class_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L645) (#13/#14). These already return a `Vec` of *all* cross-doc candidates (no arbitrary single pick), so they are not silently corrupting; the namespace-aware gate needs M5's cross-doc namespace resolution.

**Stage 1.4 — Tier-3 migrations** ✅ **DONE**
- [x] #16 [`minify.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/minify.rs#L1196): keyed the proc by its **body span** (unique per proc) instead of `pd.name == scope.name`. Reproduced the corruption first — two namespaces with a `dup` proc, one's local sharing the other's parameter name, minified repeatedly → wrong declaration's parameter region rewritten non-deterministically (`$use` renamed while the definition kept the other proc's name). Regression pins 32 runs to one intact output.
- [x] #15 [`type_definition.rs find_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_definition.rs#L77): dropped the namespace-blind simple-name fallback (the `instance_classes` value is already the namespace-aware qualified key). FP test: an instance of `::A::Widget` jumps to `::A::Widget`, never the same-named `::B::Widget`.
- [x] #17 [`tools.rs generate_docstring`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-mcp/src/tools.rs#L931): an ambiguous bare name resolves to the smallest qualified name (deterministic, no call-site namespace available); deleted the stale `AnalysisResult.find_proc` comment. Two in-file tests (qualified resolution + 32-run determinism).

**Stage 1.5 — tests**
- [x] Per-feature TP/FP/TN tests trigger from the ambiguous call/cursor site at every migrated provider (unit layer, in each provider module).
- [x] Linked-Editing regression pinned in Stage 1.2 (`resolved_qualified_name` authoritative when `Some`).
- [ ] **Remaining:** one *shared* two-namespace fixture consumed by e2e + vscode layers (the unit coverage above is per-module); lands alongside the M5 cross-doc fixture.

**Risk:** low. **Depends on:** —.

---

## M2 — Variable resolution & VAR_LINK

Five variable contexts; details in
[tricky-name-resolution-surfaces.md §2](tricky-name-resolution-surfaces.md).
Ground truth: C's `VAR_LINK` — the alias's `Var.value.linkPtr` points at the
target ([9.0 `tclVar.c:4737`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclVar.c#L4737)) and every lookup follows the chain ([8.6 `tclVar.c:757`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclVar.c#L757)); identical 8.4→9.1. Runtime/VM model it; the analyser and place-layer do not.

**Stage 2.1 — URGENT, ship first (source corruption / wrong target)** ✅ **DONE**
- [x] **(D1)** Seed TclOO object-variable decls with a real name span. Implemented via a new `collect_var_decl_spans` helper (`oo.rs`) that maps each `variable v` name → its declaration name-token span, threaded through `walk_method_body` and `DeferredBody` (both the inline and per-item/incremental passes); the seeding fallback is now a zero-width span at the body start, never the whole-body span. Verified: rename of `$n` now edits `variable n` (decl) + `$n` (use) with the body intact (was: body replaced with the new name).
- [x] **(D2, one-line)** Applied the `in_uplevel && kind == Proc { continue }` guard from `visible_variable_names` to `lookup_var_in_scope_chain` (`definition.rs`), so an `uplevel #0` body's `$g` resolves to the global, not an invisible proc-local.
- [x] Tests: 6 unit (TP/FP/TN/FN for D1 + D2) in `tests/references_rename.rs`; 2 e2e (`tests/e2e/rename.rs`, `.../definition` — real server / incremental path). Regression: oo (37) + per_item (27) + references_rename (56) green.

**Stage 2.2 — the analyser link model** — instance vars + namespace/global aliases ✅ **DONE**
- [x] TclOO instance variables unify across methods: `definition::linked_var_reference_spans` walks the scope tree and unions the uses of every `VarDef` sharing one `variable v` declaration span, wired into `references`, `rename`, and `document_highlight`.
- [x] Namespace/global aliases unify: added `VarDef::link_target` (the qualified cell name), populated by `handle_global_command` (`::v`), `handle_variable_command` (`<current-ns>::v`), and `handle_namespace_upvar_command` (`<ns>::otherVar`); the same union helper links every alias of a cell — across procs, plus the namespace-level declaration — into one variable. Survives the incremental graft (`merge_one_var` keeps the link). Verified: `variable count` across procs + the namespace-level decl unify; `global g` across procs unify; **FP guard** — `variable count` in `::a` vs `::b` stay distinct; unit + e2e + rename.
- [ ] **Remaining:** `upvar ?level? otherVar local` (caller frame statically unknown) and non-`#0` `uplevel N { … }` — these need explicit **abstention** (no target to link, must not mis-attribute), a smaller follow-up.

**Stage 2.3 — hygiene**
- [ ] Stop [`handlers.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/handlers.rs#L185) re-deriving `var_scoping.rs`'s decl-index grammar inline (fixes the `$`-prefix-exclusion gap).
- [ ] Fix doc pointers: `command-resolution.md`→`namespace-model.md` doesn't cover variables; mark `runtime-variable-frame-model.md` aspirational.

**Stage 2.4 — the 4-way split spike**
- [ ] Can the analyser `VarDef` model and the compiler place-layer ([`var_resolve.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/var_resolve.rs)) share one alias model (D3), or are the VM flat-frame map and runtime arena-tree irreducible? Answer before committing to unification.

**Verify** `oo::class create C { variable n; method get {} {return $n}; method set {x} {set n $x} }` — rename `$n`: body intact, both methods + decl rewrite. `upvar 0 x y` — rename links both. **Depends on:** —.

---

## M3 — Command name-link following

The analyser records these links but no navigation feature follows them, so a
rename leaves a runtime-live binding pointing at the old name. Several need the
analyser to record a missing **span** first.
([tricky-name-resolution-surfaces.md §1, §3.1](tricky-name-resolution-surfaces.md).)

**Stage 3.1 — record missing spans**
- [ ] Alias target word, `rename OLD NEW` arg spans, `namespace import` pattern span, `forward` target token span.

**Stage 3.2 — path-aware definition/hover (closes M1's assumption gap, issue #5)**
- [ ] Make `resolve_called_proc`/[`proc_visible_from_namespace`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L728) consume `namespace path` (today they use the path-free variant, so def/hover can jump to an unrelated same-named `::helper`).

**Stage 3.3 — follow the links in refs/rename/call-hierarchy**
- [ ] Consult `command_aliases`, `renamed_commands`, `namespace_imports`, `forward` targets: a followed link is a reference; rename rewrites the defining-side spans. Ground truth: [`exec.rs:2701`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/exec.rs#L2701) (alias re-resolved from `::`), contract §104-117 (rename/import). **Do not** text-rewrite an import tail — the token follows the source rename.
- [ ] Follow alias **chains** transitively (bounded hops + cycle detection, mirroring signature-help's existing `resolve_alias_chain`).

**Stage 3.4 — command-word arg roles & nested-def homing**
- [ ] Declare `CommandPrefix` on `tailcall` arg 0 and `coroutine` arg 1 ([`exec.rs:2909 run_tailcall`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/exec.rs#L2909)).
- [ ] **(D4)** Wire nested `proc`/`oo::class create` definition homing to the existing [`command_resolution_namespace`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/scope.rs) helper instead of `namespace_from_scope_path` (which skips proc scopes) — else a nested def under a qualified-name encloser homes to the wrong FQN and can overwrite a same-named global in `all_procs`.

**Depends on:** M1.

---

## M4 — Class-name resolution → the verified one-hop rule

Fixes the wrong ancestor-walk in
[`class_hierarchy.rs:258 resolve_class_name`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/class_hierarchy.rs#L258).
Ground truth: C resolves a bare `superclass`/`mixin` relative to the
`oo::define` **call-site** namespace, two scopes only (current→global, +path in
8.5+), via `GetClassInOuterContext` ([8.6 `tclOODefineCmds.c:61`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclOODefineCmds.c#L61)) — no ancestor walk. The VM already does this at [`cmd_oo.rs:199 resolve_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/cmd_oo.rs#L199).

**Stage 4.1 — pin it** ✅ (unit vectors)
- [x] Pinned the one-hop rule with 5 direct TP/FP/TN/FN tests on `resolve_class_name` (ancestor-abstain, same-ns, global, cross-file unique-tail, absolute) — mirrors the VM's `cmd_oo::resolve_class` and C's `GetClassInOuterContext`.

**Stage 4.2 — replace** ✅ **DONE**
- [x] Replaced the ancestor-walk in [`resolve_class_name`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/class_hierarchy.rs#L258) with `crate::naming::bareword_resolution_candidates` (current-ns → global, no intermediate ancestors); kept the sound-by-abstention unique-tail fallback for the cross-file `namespace import` idiom. Reproduced the bug (`superclass Base` in `::a::b::Sub` wrongly linked to ancestor `::a::Base`), then fixed → abstains.
- [ ] **Deferred (hygiene, not a bug):** the duplicate `canonicalise_class_name` (W308) is a *simpler* heuristic without the ancestor walk, and `class_lattice.rs`'s copy is an unwired experiment — retiring them is dedup, not a correctness fix; left for a follow-up.

**Stage 4.3 — regression** ✅ **DONE**
- [x] Green with 0 regressions: compiler lib **4305**, integration `analyser` (196, +2 new full-`analyse`→`is_subtype` tests), `mro_lattice_adversarial` (9), and lsp-core `lsp_navigation`/`lsp_providers`/`references_residual`/`hover_residual`. vscode layer deferred (no type-hierarchy provider in the TS harness; go-to-implementation awaits the M1 Tier-2 `implementation.rs` fix to be a clean vehicle).

**Risk:** moderate — mitigated: unit vectors + 2 integration tests through the real pipeline landed and pass. **Depends on:** — (before M6).

---

## M5 — Workspace-scoped resolution oracle (#923) ✅ **DONE**

Retired the bespoke textual matcher `workspace_index.rs invocations_of` for
the canonical resolver widened to the workspace. Detail in
[cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md).

**Stage 5.1 — record candidates** ✅ **DONE**
- [x] Added `SignatureCommandInvocation::resolution_candidates` (the full ordered candidate list — caller namespace, each `namespace path` entry, then global) and populated it in `finalise_invocation_resolutions` for every resolvable call (absolute → itself; the unusual-spelling branch records the settled name rather than nothing, so the list is never empty for a real call). Carried into `WorkspaceInvocation`.

**Stage 5.2 — the oracle** ✅ **DONE**
- [x] Added `WorkspaceIndex::workspace_command_exists` over a `defined_command_names` set. Rewrote `invocations_of(qualified_name, exclude_uri)` (dropped the `simple_name` arg) as pure candidate resolution: a call is a reference iff the first of its candidates defined anywhere in the workspace is the target. Deleted the exact-literal special case, the `resolved_qualified_name` fallback, the bare-name ambiguity gate, `simple_name_defined_elsewhere`, and the now-unused `WorkspaceInvocation::resolved_qualified_name` field — one rule, no heuristics.
- [x] Rewrote `resolve_workspace_symbol` (returns the qualified name only): a declaration name under the cursor, else the invocation's candidates resolved against the current document and the workspace — replacing a namespace-blind `name == word` current-doc scan (M1 site #6) and an arbitrary same-simple-name sibling `.first()` pick. Gated cross-document go-to-definition and the references "include declaration" branch on the *qualified* matchers (`proc_definitions_qualified`/`class_definitions_qualified`), so a same simple name in an unrelated namespace is no longer surfaced as this symbol's definition (issue #923 false-positive #1, sites #13/#14).

**Stage 5.3 — tests** ✅ **DONE**
- [x] Reproduced the confirmed #923 trigger first (bare call reaching a namespaced proc via `namespace path`, with a same-simple-name collision elsewhere → 0 references), then fixed → 1. Coverage: five workspace-index unit tests (namespace-path resolution, collision FP guard, all three call spellings, no-path bare call resolves to nothing, existence oracle) + three server-layer tests (references follow the path, don't cross-link the collision, go-to-definition follows the path). **Depends on:** —.

---

## M6 — TclOO cross-file methods & `oo::define` merge

**Stage 6.1 — cross-file method references + definition** ✅ **DONE**
- [x] Mirrored the existing cross-file method *rename* path for references: `references::method_reference_spans_in_document` (reference analogue of `rename::method_spans_in_document`, honouring `include_declaration`) + the server's `cross_file_method_references`, wired into the references handler via `method_rename_target`. Gathers `$obj method` / `my method` sites (and the declaration when requested) across the method's workspace-wide override family + pure-inheritor classes, in sibling documents (the current document is covered by the single-document provider). Resolves from either cursor shape — a method declaration in a class body, or an `$obj method` / `my method` call.
- [x] Cross-file go-to-definition: `cross_file_method_definition` walks the override family and returns the method's declaration span in the defining sibling document, wired into `compute_definition` before the command-head gate (a method call's head is `$obj`, not the method token). Resolves the common inherited-method-declared-in-another-file case.
- [x] Coverage is bounded the same way rename is — a `$obj method` site resolves only in a document the index knows defines or inherits the family class (a pure-consumer document that merely holds `[::Base new]` needs the cross-file instance-inference tier, a documented follow-up). Tests: five references (override family, decl include/exclude, inheritor-only document, unrelated-method empty, end-to-end handler) + two definition (family lookup, end-to-end inherited jump).

**Stage 6.2 — cross-file `oo::define`** Dedup the cross-file `oo::define ::C` **stub** ClassDef against the real one; honor a late cross-file `superclass`; add `next`/`nextto` reference sites (go-to-def already handles them). Same-file split `oo::define` already works ([tricky §3.5](tricky-name-resolution-surfaces.md)).
**Depends on:** M4, M5.

---

## M7 — Command-names-in-variables & dispatch tables

**Stage 7.1 — SCCP spike** Is SSA/SCCP already computed per-document, or new cost? (The analyser walk and SSA/SCCP are separate passes today.)
**Stage 7.2 — resolve constants** If cheap: resolve a constant `$cmd` head through [`resolve_command_with`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-syntax/src/naming.rs#L516); abstain on non-const (matches the documented limit).
**Stage 7.3 — literals in data** Emit a reference for a proc-name literal held as a dict/`array set`/`string map` **value** dispatched via `{*}[dict get …]` — the W307 heuristic already recognizes these but only to suppress a diagnostic. **Depends on:** M5.

---

## M8 — Library / autoload resolution tier

`PackageResolver` already locates the defining file for any `package require`d /
autoloaded name (config/env-aware: `TCL_LIBRARY`, `TCLLIBPATH`,
`tclLsp.libraryPaths`, `.tcl-lsp.ini`) faithfully mirroring `tclPkgUnknown`/`auto_load` — it's just never analysed into `WorkspaceIndex`.

**Stage 8.1 — lazy second tier** On an oracle miss (M5), ask `PackageResolver` for the defining file, lazily analyse, memoise, merge into the oracle. Never eagerly parse the whole stdlib.
**Depends on:** M5.

---

## M9 — Source-site namespace propagation

`source` evaluates in the caller's current namespace ([`command.rs cmd_source`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/command.rs); contract §134-137). A bare `proc helper` in a file sourced inside `namespace eval ::x` is homed `::helper`, not `::x::helper` — so a correctly-written `::x::helper` call misses, and rename dangles. **M5/M8 as scoped do NOT fix this** (the index holds `::helper`).

**Stage 9.1 — re-home** Re-home a sourced file's global-scope defs under the namespace active at the literal `source` call site.
**Stage 9.2 — computed paths** Route source paths through the existing [`auto_path_eval`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/auto_path_eval.rs) folding for `[file join …]` forms.
**Depends on:** M5, M8.

---

## M10 — Dialect-aware command resolver

Version-correctness leaks where resolution *outcome* depends on dialect but the
resolver ignores it.

**Stage 10.1 — `namespace path` gating (D1 c-source).** 8.4 has no path tier ([8.4 `tclNamesp.c:1961`](https://github.com/tcltk/tcl/blob/9ccfe9d1b35741ff7323837f6485ffe48b06fad9/generic/tclNamesp.c#L1961) vs [8.5 `NamespacePathCmd:197`](https://github.com/tcltk/tcl/blob/160d612a6b2b1c2c0db27236d648b7bc1364570c/generic/tclNamesp.c#L197)); the registry records the boundary ([`namespace_.rs:160`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-registry/src/commands/tcl/namespace_.rs#L160)) but the resolver never consults it.
- [ ] Thread dialect into [`scope.rs finalise_invocation_resolutions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/scope.rs#L327); skip the path tier when the dialect excludes `TCL85_PLUS`; mirror in the VM ([`interp.rs:1658`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L1658)).

**Stage 10.2 — `namespace unknown` re-gate (D9).** Re-gate to `TCL85_PLUS` (currently admits 8.4). **Depends on:** —.

---

## M11 — Cross-version variable semantics

**Fixes D4 (c-source)** — the *only* resolution-semantics change 8.4→9.1. 8.4/8.5/8.6 fall back to global for an unqualified undefined var at namespace scope; **9.0 removed it** ([8.6 `tclVar.c:757`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclVar.c#L757) keeps it vs [9.0 forces `TCL_NAMESPACE_ONLY`, `tclVar.c:935`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclVar.c#L935); [9.0 `changes.md:189`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/changes.md#L189)). Rust hardcodes 9.0 for all dialects ([`interp.rs:666`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L666); [`vars.rs:107`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/runtime/rust/src/vars.rs#L107)).

**Stage 11.1 — gate the fallback** Keep it for `TCL8X`, drop for `TCL90_PLUS`, in the runtime resolver, VM, and analyser var path.
**Stage 11.2 — pin** Vector both behaviors against `tclsh8.6` and `tclsh9.0`. **Depends on:** M2.

---

## M12 — Expr-function fidelity (`::tcl::mathfunc`)

**Stage 12.1 — dialect gating (D7 c-source).** 8.4 has no `::tcl::mathfunc`; functions are a fixed C table ([8.4 `tclExecute.c:3934`](https://github.com/tcltk/tcl/blob/9ccfe9d1b35741ff7323837f6485ffe48b06fad9/generic/tclExecute.c#L3934)) lacking `min`/`max`/`is*`; 8.5+ adds the namespace scheme ([8.6 `tclCompExpr.c:2276`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclCompExpr.c#L2276)). Add a dialect-aware allowlist in the shared evaluator + const-folder.
**Stage 12.2 — proc linking (D8).** In [`collect_expr_substitutions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/commands.rs#L2225), emit a `::tcl::mathfunc::<f>` invocation head per expr function call so a user `proc ::tcl::mathfunc::f` gets goto-def/refs/arity and isn't flagged unused (codegen already resolves it, [`expressions.rs:325`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/codegen/expressions.rs#L325)). **Depends on:** —.

---

## M13 — TclOO version fidelity (`property`)

**Stage 13.1 — gate `property` (D10).** Properties are 9.0+ ([`tcl9.0.4/generic/tclOOProp.c`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclOOProp.c); absent 8.6). Gate the `property` body member `TCL90_PLUS` + configurable-family.
**Stage 13.2 — accessors (N3).** Fold property accessors / configure-cget into `known_methods`. **Depends on:** M4.

---

## M14 — Dynamic reference roles (`ArgRole::CommandName`)

Introduce a `CommandName` reference role (there is only `CommandPrefix` today) and
apply it wherever a command name appears as a data argument — merging the
C-source trace finding (D11) with the dynamic-surface introspection/ensemble
findings.

**Stage 14.1 — the role** Add `ArgRole::CommandName`; make refs/rename treat it as a reference.
**Stage 14.2 — apply it** `trace add command/execution NAME` ([8.6 `tclTrace.c:507`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclTrace.c#L507)); `namespace which -command` / `namespace origin`; `info args/body/default PROC`; the `coroutine NAME` created command.
**Stage 14.3 — ensembles** Parse `namespace ensemble create -map/-subcommands` into a subcommand→target map; emit a reference for each `<ns>::sub` and each `-map`/`-unknown` target literal. **Depends on:** M3.

---

## M15 — Interpreter/scope isolation & coverage

**Stage 15.1 — cross-interp isolation** Open a child-interp scope for `interp eval CHILD SCRIPT` so a child `proc foo` and its calls don't merge into the parent namespace (today a rename of the parent `foo` edits the child body — a false-positive wrong edit; [`interp.rs:369`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L369) is the isolation ground truth).
**Stage 15.2 — inscope/code** Give `namespace inscope`/`namespace code` scripts the `ns` scope (like `namespace eval`).
**Stage 15.3 — per-object mixins** Per-object symbol store for `oo::objdefine method`/`mixin` so `$obj m` resolves the per-object override, not a same-named class method.
**Stage 15.4 — 9.0 `::tcl::` reorg (N4)** Model the removed/added `::tcl::` sub-namespaces as dialect-gated specs. **Depends on:** M3, M5.

---

## M16 — VM behavioural parity (out of resolution scope)

**Stage 16.1** Alias-loop prevention (`TclPreventAliasLoop`, near [8.6 `tclInterp.c:225`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclInterp.c#L225)).
**Stage 16.2** Cross-interp aliases child→parent (analyser already refuses the link — no false cross-interp reference).
**Stage 16.3** Fire command/execution traces (accepted no-op today).
**Stage 16.4** Command-name epoch cache (C caches on the name object; VM re-resolves every dispatch at [`interp.rs:1658`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L1658) — perf only). **Depends on:** —.

---

## Danger-class summary

Silent *wrong* output (not merely missed), highest priority first:

1. **M2 Stage 2.1 (D1)** — rename **destroys** a method body. Ship first, standalone.
2. **M1 Tier-1** — rename/call-hierarchy act on the **wrong** same-named symbol.
3. **M2 Stage 2.1 (D2), Stage 2.2** — variable navigation to the **wrong** var (uplevel), and split/incomplete edits (upvar/object-var links).
4. **M4** — **wrong** inheritance edge (ancestor-walk).
5. **M3 (D4)** — nested-def homing overwrites a same-named symbol.
6. **M10 / M11 / M12 / M13** — version-conditioned **false** resolution (8.4/8.6 dialects).
7. **M15 Stage 15.1** — `interp eval` merges child/parent → wrong cross-interp edit.

Everything else is *missed* refs/edits (under-delivery, not corruption).

## Drift prevention (once M1 lands)

- [ ] `xtask` lint (grep-shaped to start) flagging any new `.iter().find(|…| …name == word…)` scan over `all_procs`/`all_classes` outside [`tcl_syntax::naming`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-syntax/src/naming.rs) and the sanctioned `definition.rs` helpers. The "add a vector" discipline only protects consumers already inside the contract — which is why 17 sites drifted.
- [ ] Contract-doc corrections: `command-resolution.md`'s WASM-codegen line (inherits the *runtime's* conformance via eval-delegation, not "the VM's"); `tcloo-implementation.md` (stale pre-Rust-port Python modules).
