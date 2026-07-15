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

**Stage 1.1 — shared entry points**
- [ ] Widen [`resolve_called_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L748) / [`proc_visible_from_namespace`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L728) to `pub(crate)`; add a re-export for [`lib.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-server/src/lib.rs#L3631).
- [ ] Build a class analogue `resolve_referenced_class` sharing the same candidate walk (classes are commands).

**Stage 1.2 — Tier-1 migrations (silent-corruption bugs)**
- [ ] Migrate #1–#6 ([`rename_proc`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/rename.rs#L654), [`rename_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/rename.rs#L735), [`find_proc_for_item`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/call_hierarchy.rs#L127), [`proc_references`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/references.rs#L388), [`class_references`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/references.rs#L340), [`resolve_workspace_symbol`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-server/src/lib.rs#L3631) same-doc tier), preserving each existing exact `name_span`-containment fast path.
- [ ] Fix the Linked-Editing OR-bug at [`matches_self_call`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/linked_editing_range.rs#L148): when `resolved_qualified_name` is `Some`, it is authoritative — only consult `matches_self_call` when it is `None`.

**Stage 1.3 — Tier-2 migrations**
- [ ] Migrate #8–#14: replace [`strip_colons`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/implementation.rs#L155) with the class resolver; dedupe the byte-identical [`signature_help.rs:302`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/signature_help.rs#L302) / [`inlay_hints.rs:911`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/inlay_hints.rs#L911) into one helper then route it; fix [`hover.rs:2198`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/hover.rs#L2198), [`type_hierarchy.rs:54`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_hierarchy.rs#L54); ambiguity-gate [`proc_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L633)/[`class_definitions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L645) (finished in M5).

**Stage 1.4 — Tier-3 migrations (follow-up)**
- [ ] Migrate #15–#17: [`type_definition.rs:77`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_definition.rs#L77), [`minify.rs:1196`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/minify.rs#L1196), [`tools.rs:931`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-mcp/src/tools.rs#L931) (+ delete the stale "mirrors `AnalysisResult.find_proc`" comment — no such method exists).

**Stage 1.5 — tests**
- [ ] One shared fixture (two namespaces, same-named proc + same-named class); a per-feature test triggering from the **ambiguous call site**; the Linked-Editing regression (`proc ::a::greet {} { namespace eval ::b { greet } }` with a separate `::b::greet` — assert not linked).

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

**Stage 2.2 — the analyser link model (VAR_LINK)**
- [ ] Add a link/target field to [`VarDef`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/types.rs#L285) (or a scope alias-edge map).
- [ ] Populate from every alias site: [`handle_global_command:185`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/handlers.rs#L185), [`handle_variable_command:208`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/handlers.rs#L208), [`handle_upvar_command:1483`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/handlers.rs#L1483), [`handle_namespace_upvar_command:1509`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/handlers.rs#L1509), and the TclOO object variable (link `variable v`/`my variable v` across every method to one cell).
- [ ] Make [`lookup_var_in_scope_chain`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/definition.rs#L552) follow the link so refs/rename/hover unify alias↔target and `$v` across sibling methods.
- [ ] For non-`#0` `uplevel N { … }` (target frame statically unknown) **abstain** rather than mis-attribute body vars (D3).

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

**Stage 4.1 — pin it**
- [ ] Class-name conformance vectors (same machinery as `command_resolution_vectors.txt`): base one level up not in an ancestor (the bug); does `superclass`/`mixin` honor a `namespace import`ed name? Pin against real `tclsh`.

**Stage 4.2 — replace + retire**
- [ ] Replace the ancestor-walk + unique-tail fallback in [`resolve_class_name`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/class_hierarchy.rs#L258) with a call into [`naming.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-syntax/src/naming.rs#L455).
- [ ] Retire the duplicate `canonicalise_class_name` (W308) and `class_lattice.rs`'s parallel `resolve_class_name` per the import answer.

**Stage 4.3 — regression**
- [ ] Full MRO / class-hierarchy / type-hierarchy / W308 / cross-file class suites (this resolver feeds all of them).

**Risk:** moderate — vectors must land+pass before the swap. **Depends on:** — (before M6).

---

## M5 — Workspace-scoped resolution oracle (#923)

Retire the bespoke matcher [`workspace_index.rs:758 invocations_of`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L758) for the canonical resolver widened to the workspace. Detail in [cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md).

**Stage 5.1 — record candidates** Extend [`finalise_invocation_resolutions`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/analyser/scope.rs#L327) to keep the full candidate list per invocation.
**Stage 5.2 — the oracle** Add `WorkspaceIndex::workspace_command_exists`; rewrite [`invocations_of`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/workspace_index.rs#L758) to run candidates through [`resolve_command_with`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-syntax/src/naming.rs#L516); gate `proc_definitions`/`class_definitions`.
**Stage 5.3 — tests** Multi-file conformance-vector format; the confirmed #923 repro. **Depends on:** —.

---

## M6 — TclOO cross-file methods & `oo::define` merge

**Stage 6.1 — method index** Extend `WorkspaceClass.defined_methods` into a queryable method table; teach `resolve_workspace_symbol` + [`cross_document_references`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-server/src/lib.rs#L3682) to resolve method names and gather `$obj method`/`my method` sites cross-file. Reuse `class_lattice.rs`'s `NsContext` (consistent after M4).
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
