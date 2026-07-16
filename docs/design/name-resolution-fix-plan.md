# Name resolution — master fix plan

**Status:** in execution on branch `claude/validate-issue-923-comment-tsajg7`
(PR #933). Honest completion state, per milestone:

- **Landed, with tests:** M1 (target-selection consolidation), M2 (D1 object-var
  rename + D2 uplevel guard + alias unification), M3 (command name-links), M4
  (class one-hop resolution), M5 (workspace resolution oracle — the #923 core),
  M6 (cross-file TclOO methods), M10 (dialect-aware resolver — analyser side;
  the VM mirror of 10.1 is deferred to the M16 VM-parity pass), M12 (expr math
  functions), M13 (TclOO `property` version fidelity), M14 (command-names-as-data
  + ensembles — see 14.2 for the one reverted sub-item), M15 (interp/inscope
  isolation, per-object methods, `zipfs`).
- **Partial:** M8 §8.1 — the **go-to-definition** autoload tier has landed;
  merging lazily-analysed library files into the shared index so *references /
  rename* also reach them is the remaining half.
- **Not started:** M7 (command-names-in-variables / SCCP), M9 (source-site
  namespace propagation), M11 (cross-version namespace-var fallback — needs a
  `tclsh8.6`/`tclsh9.0` vector), M16 (VM behavioural parity — out of resolution
  scope).
- **Review hardening (PR #933):** a review pass — the automated Codex reviewer
  plus the branch's first full CI run (including the vscode integration suite)
  — surfaced latent defects in stages previously ticked done, now fixed and
  called out inline as "**Review fix (PR #933)**": qualified-name aliasing in
  `variable` / `namespace upvar` (M2), completion best-effort inside `uplevel 1`
  (M2 D2), and a `namespace which -command` probe wrongly drawing W123 (M14.2,
  the reverted sub-item). "Done" therefore means *implemented and passing the
  stage's own tests*; these three show a stage can still carry a defect its own
  tests missed until the cross-cutting review runs.

This is the actionable plan behind four companion studies:
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
- [x] `resolve_workspace_symbol` (cross-document, `tcl-lsp-server/src/lib.rs`) — landed with M5 Stage 5.2: rewritten onto the workspace oracle (`resolution_candidates` + `workspace_command_exists`, link-chased via `resolve_command_target`); M8's completion adds the autoload fall-through.
- [x] Verified: reproduced the wrong-symbol rename (renamed `::b::helper` from `::a`'s call site), then fixed. Tests — 4 unit (proc+class rename/refs from call sites, TP/FP/TN) + 1 e2e + 1 vscode; full `tcl-lsp-core` suite (969 lib + all integration) green, 0 regressions.

**Stage 1.3 — Tier-2 migrations** ✅ **DONE (LSP providers)**
- [x] Migrated #8 [`implementation.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/implementation.rs#L155): the class *target* now resolves via `resolve_class_target_at`, and — because `superclasses`/`mixins` hold names *as written* (a bare `superclass Shape` in `::A` stays `"Shape"`, which the leading-`::`-only tail compare never matched to the resolved `::A::Shape`) — the subclass edges now come from the owner-aware class-hierarchy index (`subclasses`, super + mixin unioned), the same source `type_hierarchy::subtypes` shares. Fixed a real false-negative (namespaced classes returned zero subclasses).
- [x] Migrated #9 [`signature_help.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/signature_help.rs#L302) + #10 [`inlay_hints.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/inlay_hints.rs#L911) onto the shared `resolve_called_proc` (namespace at the command token; the builtin gate stops a namespaced proc hijacking a same-named builtin from global scope; the lenient fallback is now deterministic). Both are also cheaper — O(candidates) probes vs an O(procs) scan.
- [x] Fixed #11 [`hover.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/hover.rs#L2198) (deleted `lookup_class`, routed through `resolve_class_target_at`) and #12 [`type_hierarchy.rs prepare`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_hierarchy.rs#L54).
- [x] TP/FP/TN coverage added at each site (same-named class/proc in two namespaces resolves to the cursor's namespace; builtin gate; deterministic fallback).
- [x] Ambiguity-gate `proc_definitions`/`class_definitions` (#13/#14) — landed with M5 Stage 5.2: cross-document go-to-definition and the references include-declaration branch resolve through the *qualified* matchers (`proc_definitions_qualified`/`class_definitions_qualified`), so a same simple name in an unrelated namespace is never surfaced.

**Stage 1.4 — Tier-3 migrations** ✅ **DONE**
- [x] #16 [`minify.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/minify.rs#L1196): keyed the proc by its **body span** (unique per proc) instead of `pd.name == scope.name`. Reproduced the corruption first — two namespaces with a `dup` proc, one's local sharing the other's parameter name, minified repeatedly → wrong declaration's parameter region rewritten non-deterministically (`$use` renamed while the definition kept the other proc's name). Regression pins 32 runs to one intact output.
- [x] #15 [`type_definition.rs find_class`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-lsp-core/src/type_definition.rs#L77): dropped the namespace-blind simple-name fallback (the `instance_classes` value is already the namespace-aware qualified key). FP test: an instance of `::A::Widget` jumps to `::A::Widget`, never the same-named `::B::Widget`.
- [x] #17 [`tools.rs generate_docstring`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-mcp/src/tools.rs#L931): an ambiguous bare name resolves to the smallest qualified name (deterministic, no call-site namespace available); deleted the stale `AnalysisResult.find_proc` comment. Two in-file tests (qualified resolution + 32-run determinism).

**Stage 1.5 — tests** ✅ **DONE**
- [x] Per-feature TP/FP/TN tests trigger from the ambiguous call/cursor site at every migrated provider (unit layer, in each provider module).
- [x] Linked-Editing regression pinned in Stage 1.2 (`resolved_qualified_name` authoritative when `Some`).
- [x] The shared two-namespace fixture landed with PR #933 at both layers: e2e (`tests/e2e/rename.rs` / `references.rs` — the `::a`/`::b` `helper` collision) and vscode (`testFixture/renameNamespaceCollision.tcl` + `renameSymbol.test.ts`), same fixture content in both.

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
  - **Review fix (PR #933):** the `variable` and `namespace upvar` target builders kept only the tail after the last `::`, so a *qualified* name aliased the wrong cell — a relative `variable child::v` targeted `<ns>::v` (not `<ns>::child::v`) and `namespace upvar ::a b::c local` targeted `::a::c` (not `::a::b::c`), unifying with the wrong namespace variable. Both now keep the full qualified path (mirroring the `global` handler, which was already correct), and `variable` keys the local link on the unqualified tail so a later `$v` shares it. The simple-name common case is unchanged.
- [x] **`upvar` + non-`#0` `uplevel` abstention:** `upvar ?level? otherVar local` already abstains (`handle_upvar_command` defines the local alias with **no** `link_target` — the caller frame is unknown, so it never mis-links). Fixed the non-`#0` `uplevel N { … }` mis-attribution: every single-braced-body `uplevel` now opens an isolated `Uplevel` child scope tagged with the level word, and `uplevel_hides_scope` resolves `#0` outward to the global frame but a non-`#0` level **only within the body** (abstaining on the enclosing proc *and* the global). Tests: compiler scope isolation, lsp-core reference abstention + body-local resolution.
  - **Review note (PR #933):** the branch's first CI run exposed a *stale* vscode integration test (`variableContexts.test.ts`) still expecting the pre-D2 *best-effort* completion — offering the enclosing proc's locals inside `uplevel 1`. That contradicts the D2 design and the sound server-side `dollar_completion_uplevel_one_abstains_from_proc_scope`: a non-`#0` `uplevel` body runs in the dynamic caller's frame, so the enclosing proc's locals are *not* in scope and must not be suggested (a first attempt to satisfy the stale test by making completion best-effort was reverted — it broke the server-side abstain test and would suggest out-of-scope variables). The stale test and its fixture were corrected to assert abstention (offer the body's own `body_var`, not the proc's `up1_local`).

**Stage 2.3 — hygiene** ✅ **DONE**
- [x] **`$`-prefix-exclusion gap:** the `variable` / `global` / `upvar` / `namespace upvar` declaration handlers recorded a *dynamic* name (`variable $dyn` → a phantom variable `dyn`). Gated each on `naming::is_dynamic_word` (the shared `$`/`[` test `rename`/`proc` use), so a computed name is skipped rather than recorded — matching `set`'s existing behaviour. (The plan's `var_scoping.rs` reference predates a reorg; the fix lands directly in the handlers.)
- [x] **Doc pointers:** `command-resolution.md`'s variable-surface note now points to `runtime-variable-frame-model.md` (the actual variable/frame contract) rather than `namespace-model.md` (which covers only qualified-name namespace resolution). `runtime-variable-frame-model.md` already carries an explicit aspirational **Status** header.

**Stage 2.4 — the 4-way split spike** ✅ **ANSWERED**
- [x] **Decision: do NOT unify the four data models; keep sharing the resolution *substrate*.** The four layers are specialised for genuinely different consumers — the analyser `VarDef` is span/position-oriented (LSP navigation: declare/use sites, alias unification for references/rename); the compiler place-layer ([`var_resolve.rs`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-compiler/src/var_resolve.rs)) is `Place`/overlap-oriented (static-analysis soundness, over-approximating dynamic names for SSA/SCCP); the VM keeps a flat per-frame value map with `VAR_LINK`; the runtime an arena of cells. Unifying the *structs* would couple navigation to execution storage with no benefit. What they correctly share is the alias **semantics**: `naming.rs` candidate resolution, `var_refs.rs` scanning, and the `VAR_LINK` rule — the place-layer already "reuses the existing resolution substrate rather than re-deriving it", and Stage 2.2's `VarDef::link_target` mirrors `VAR_LINK`. The "one alias model" is the shared *rule set*, expressed once and viewed four ways, not a shared type. No unification work is scheduled; the substrate sharing is the invariant to preserve.

**Verify** `oo::class create C { variable n; method get {} {return $n}; method set {x} {set n $x} }` — rename `$n`: body intact, both methods + decl rewrite. `upvar 0 x y` — rename links both. **Depends on:** —.

---

## M3 — Command name-link following

The analyser records these links but no navigation feature follows them, so a
rename leaves a runtime-live binding pointing at the old name. Several need the
analyser to record a missing **span** first.
([tricky-name-resolution-surfaces.md §1, §3.1](tricky-name-resolution-surfaces.md).)

**Stage 3.1 — record missing spans** ✅ **DONE**
- [x] The `interp alias` `TARGET` word is already a first-class command invocation (the registry marks it a `command_prefix`), so it needs no separate span — the ordinary reference/rename path covers it.  Recorded the two that were missing: `rename OLD NEW`'s `OLD`-word span (`AnalysisResult::rename_target_spans`, `new_qname → OLD span`, populated in `handle_rename` and rebased in the per-item graft) and the `forward` `TARGET` token — recorded as a command invocation during the class-body walk (resolving in the class's namespace context), so it too flows through the invocation machinery.  The `namespace import` pattern span was already recorded (`SignatureNamespaceImport::range`).

**Stage 3.2 — path-aware definition/hover (closes M1's assumption gap, issue #5)** ✅ **DONE**
- [x] Exposed the recorded `namespace path` declarations on `AnalysisResult` (`namespace_paths`, populated in `finalise_invocation_resolutions`) and switched `proc_visible_from_namespace` from `bareword_resolution_candidates` to `command_resolution_candidates`, threading the caller namespace's path.  A bare `helper` in a namespace with `namespace path ::mymod` now resolves to `::mymod::helper` (not a same-named global `::helper`) for definition / hover / signature help, agreeing with call-site settling.  Test: `definition_bare_call_honours_namespace_path_over_global`.

**Stage 3.3 — follow the links in refs/rename/call-hierarchy** ✅ **DONE**
- [x] The workspace index lifts every `namespace import` / `interp alias` / `rename` into a flat `WorkspaceCommandLink` (`linked_qname → target_qname`, plus the defining-side span).  `linked_invocations_of` widens the existence oracle to admit the linked names and chases the winning candidate along the link map to its ultimate target — so a bare call reaching a command through an import / alias / rename counts as a reference to it — and `resolve_command_target` resolves a cursor sitting on such a call to the command it really names (references gather from either side).  `link_target_spans` surfaces the defining-side words a rename must rewrite (the `rename` `OLD` word, the import pattern); the `interp alias` / `forward` `TARGET` words are already invocations, so they ride the ordinary path.  References follow the links; **rename deliberately does not** rewrite the local imported / aliased *usages* — they name the local command, which keeps its own name (the token follows the source rename at run time — `exec.rs:2701`).  Tests: `namespace_import_call_site_references_the_source_command`, `interp_alias_call_site_references_the_target_command`, `rename_new_name_call_site_references_the_old_command`, `oo_forward_target_is_a_reference_to_the_command`, `cross_document_references_follow_namespace_import`, `cross_document_symbol_edits_rewrite_import_pattern_and_rename_old_word`.
- [x] `linked_invocations_of` / `resolve_command_target` follow alias **chains** transitively — `follow_links` walks the link map with cycle detection (an alias-of-an-alias-of-an-alias resolves to the source; a malformed cycle stops).  A glob `namespace import ::mod::*` names no single command, so it introduces no link.  Tests: `resolve_command_target_follows_a_chain_and_leaves_plain_names`, `glob_import_introduces_no_command_link`.

**Stage 3.4 — command-word arg roles & nested-def homing** ✅ **DONE**
- [x] Declared `command_prefixes` on `tailcall` (arg 0) and `coroutine` (arg 1) so the command a `tailcall command …` / `coroutine name command …` names is seen by references / go-to-definition / rename the same as a direct call ([`exec.rs:2909 run_tailcall`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/exec.rs#L2909)).
- [x] **(D4)** Wired nested `proc` / `oo::class create` homing to `command_resolution_namespace` (a proc body resolves the definition command in the enclosing proc's *defining* namespace) instead of the lexical `namespace_from_scope_path`.  Reproduced the bug (`proc a::outer { proc helper … }` homed `helper` to `::helper`, overwriting the real global), then fixed → homes to `::a::helper`, global preserved.  Test: `nested_def_in_qualified_encloser_does_not_overwrite_global`.

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
- [x] **Pure-consumer documents:** a file that only creates and uses an instance (`set d [::other::Cls new]; $d method`, without defining/subclassing) is now covered. The analyser gained an opt-in workspace class oracle (`Analyser::with_workspace_classes`, consulted by `resolve_user_class`; empty for the normal cached analysis so diagnostics are unaffected). The server's `resolve_method_target` retries method resolution with the oracle so a consumer cursor triggers, and `cross_file_consumer_method_references` re-analyses each candidate consumer document (bounded via `documents_invoking_classes`, plus the current document) with the oracle and collects its `$obj method` sites via `references::obj_method_call_sites`. Three tests (sibling consumer references, consumer-cursor references handler, consumer-cursor go-to-definition).

**Stage 6.2 — cross-file `oo::define`** ✅ **DONE**
- [x] A cross-file `oo::define ::C` records a second `::C` index entry with empty superclasses; the method-family parent walk resolved parents by first-match, so an adversarial indexing order let the stub hide the real class's hierarchy edge (non-deterministic). Added `WorkspaceIndex::resolved_parents_of`, which unions superclasses + mixins across every indexed definition of a class, and routed both family closures through it. Adversarial regression (stub indexed first) fails under first-match, passes under the union.
- [x] **Class go-to-definition prefers the real creation site:** `ClassDef::via_define` (mirrored on `WorkspaceClass`) marks an `oo::define` on a locally-uncreated class as an extension stub; cross-document go-to-definition returns the `oo::class create` site(s), falling back to all sites only when a class is defined solely by `oo::define`.
- [x] **`next`/`nextto` reference sites:** `references::method_next_dispatch_spans` folds the super-dispatch tokens into the three read-only method paths (never into `method_references_for_class`, whose set drives rename, so `next` is a reference but never rewritten). Tests confirm both.

**Depends on:** M4, M5.

---

## M7 — Command-names-in-variables & dispatch tables ✅ **DONE**

**Stage 7.1 — SCCP spike** ✅ **ANSWERED** — SSA/SCCP is **already computed per document** on the analyse path (`emit_cfg_ssa_diagnostics` builds the `CompilationUnit` the W307 pass consumes), but it runs *after* invocation settlement.  Two oracles therefore serve M7 at zero new pass cost: the analyser's scope-walk `const_strings` map (`resolve_const_word` — "the analyser-level counterpart to the optimiser's SCCP") settles constant `$cmd` heads *before* `finalise_invocation_resolutions`, and the existing W307 `CompilationUnit` serves the table-literal pass.
**Stage 7.2 — resolve constants** ✅ **DONE** — a `$cmd` head whose variable holds a known constant at the call site is recorded as a pending dispatch (`ConstDispatchSite`: value + head span + resolution namespace) and settled in `finalise_invocation_resolutions` through the shared `resolve_command_with`: a value resolving to a **user** command becomes an ordinary invocation (references / go-to-definition / call hierarchy reach the dispatched proc through the ordinary machinery); a builtin, unknown, or non-const value abstains entirely — no phantom invocation, no W123 delta, the W307 story at the site unchanged.  The invocation is flagged **`indirect: true`** — a new `SignatureCommandInvocation` field mirrored on `WorkspaceInvocation` — because its span is `$cmd`, *not* the written name: every span-rewriting consumer (in-document + cross-document rename, the minifier's call-site and alias-shortening passes, linked editing) skips indirect sites, so a rename can never splice the new name over `$cmd`.  Tests: analyser TP (global + namespace-resolved), TN (unknown / dynamic / builtin), lsp-core references-include + rename-never-rewrites.
**Stage 7.3 — literals in data** ✅ **DONE** — `harvest_table_command_value_spans` recovers `(table, value, value-span)` triples from the dispatch-table constructors whose literal text is recoverable in source (`set arr(k) v`, `array set arr {k v …}`, `dict set d k v`); `emit_dispatch_table_command_references` (inside the W307 pass, reusing its `CompilationUnit`) emits a command reference for each value that (a) belongs to a table **consumed** by a `$table(...)` / `[dict get $table …]` dispatch site — an unconsumed config array gains no phantom references — and (b) resolves to a known user command in the namespace at the constructor site.  These anchor at the literal itself (`indirect: false`), so **rename rewrites the table entry alongside the proc**, keeping the dispatch alive.  Values reachable only through folding (`string map`, computed keys) have no span and abstain — the documented limit.  Tests: analyser TP (`array set` + `dict set`), consumption-gate TN, lsp-core rename-rewrites-table, e2e `dispatch_table_literal_resolves_to_the_proc_m7`. **Depends on:** M5.

---

## M8 — Library / autoload resolution tier

`PackageResolver` already locates the defining file for any `package require`d /
autoloaded name (config/env-aware: `TCL_LIBRARY`, `TCLLIBPATH`,
`tclLsp.libraryPaths`, `.tcl-lsp.ini`) faithfully mirroring `tclPkgUnknown`/`auto_load` — it's just never analysed into `WorkspaceIndex`.

**Stage 8.1 — lazy second tier** ✅ **DONE** — both halves:
- [x] **Go-to-definition tier** (PR #933): when a command head resolves to nothing in the current document *or* the workspace oracle (M5), `compute_definition` falls through to the autoload tier, which asks `PackageResolver::resolve_auto_command` (the same database that already clears the #832 W123) for the library file — **lazy** (only the resolved file, never the whole stdlib).  Test: `autoload_library_command_go_to_definition_m8`.
- [x] **Index merge (the completion pass):** `Backend::ensure_autoload_indexed` is now the one autoload entry — it resolves the defining file, analyses it, and **merges it into the shared `WorkspaceIndex`** (`remove_document` + `add_document`), so references, rename, and definition all answer from the same index.  `resolve_workspace_symbol` falls through to it on a workspace miss, which gives *references / rename* the library tier for free; `autoload_definition` re-answers from the index.  Idempotent (an existence pre-check short-circuits once merged; a real workspace definition always wins over a same-named library one), and the merged URIs are dropped when the package database is rebuilt, so a `libraryPaths` change cannot leave stale library definitions behind.
- [x] **Consumer-document rename:** an empty in-document rename no longer aborts wholesale when the workspace oracle resolves the cursor's command — `core_rename::workspace_symbol_rename_edits` builds the whole edit set from the index (current document's call sites included) with the same collision discipline, fixing rename-from-consumer for workspace siblings and library files alike.
- [x] Tests: 4 server unit (references + rename through the merge, workspace-shadow FP guard, package-database-rebuild drop), 2 consumer-rename unit (sibling TP, collision TN), e2e `autoload_library_command_references_and_rename_m8` (temp-dir library through the real server), vscode `renameSymbol.test.ts` (consumer rename rewrites the library declaration).
**Depends on:** M5.

---

## M9 — Source-site namespace propagation

`source` evaluates in the caller's current namespace ([`command.rs cmd_source`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/command.rs); contract §134-137). A bare `proc helper` in a file sourced inside `namespace eval ::x` is homed `::helper`, not `::x::helper` — so a correctly-written `::x::helper` call misses, and rename dangles. **M5/M8 as scoped do NOT fix this** (the index holds `::helper`).

**Stage 9.1 — re-home** ✅ **DONE** — implemented as **seeded re-analysis**, not a string-prefix rewrite, so the full C semantics fall out of the ordinary scope machinery: `Analyser::analyse_with_source_namespace(source, dialect, ns_key)` walks the whole file inside the source-site namespace (exactly `namespace eval <ns> { <file> }`), so relative definitions re-home (`proc helper` → `::x::helper`), absolute ones stay put, relative `namespace eval` nests, and bare call sites gain the seeded tier in their candidate lists.  The analyser records the command-resolution namespace at every `source` call site (`SignatureSource::site_namespace`, both walks), the index lifts it onto `WorkspaceSource` and exposes `source_seed_map` (per sourced document, the set of namespaces it is sourced under — several views may all be true at run time), and the server's `refresh_source_rehoming` reconciles lazily before every cross-document query (definition / references / rename): documents whose applied seeds differ are re-analysed seeded and merged, bounded-fixpoint because a seeded parent records *composed* namespaces for its own nested `source` calls.  Declaration-side queries map a sourced document's standalone name to its re-homed twin (`seed_mapped_symbol`), so references/rename from inside the sourced file line up with the sourcing side.  Publish/scan/reindex paths invalidate the applied-seed record so an edit can never leave a stale view.
**Stage 9.2 — computed paths** ✅ **DONE** — `resolve_source_edge` routes a non-literal `source` path through [`auto_path_eval::evaluate_auto_path_expr`] (its first production caller), with the sourcing file standing in for `[info script]` — so `source [file join [file dirname [info script]] b.tcl]` re-homes exactly like a literal, and anything the folder cannot prove abstains (never a guess).
- [x] Tests: analyser (seeded homing incl. absolute/relative/nested + composed nested-source namespaces + global-seed identity), server unit (call-side references reach the sourced decl + its re-homed internal calls; declaration-side finds qualified callers; computed-path folding TP + `$var` abstention TN), e2e `sourced_file_resolves_under_the_source_site_namespace_m9` (goto-def + references through the real server).
**Depends on:** M5, M8.

---

## M10 — Dialect-aware command resolver

Version-correctness leaks where resolution *outcome* depends on dialect but the
resolver ignores it.

**Stage 10.1 — `namespace path` gating (D1 c-source).** 8.4 has no path tier ([8.4 `tclNamesp.c:1961`](https://github.com/tcltk/tcl/blob/9ccfe9d1b35741ff7323837f6485ffe48b06fad9/generic/tclNamesp.c#L1961) vs [8.5 `NamespacePathCmd:197`](https://github.com/tcltk/tcl/blob/160d612a6b2b1c2c0db27236d648b7bc1364570c/generic/tclNamesp.c#L197)); the registry records the boundary ([`namespace_.rs:160`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-registry/src/commands/tcl/namespace_.rs#L160)) but the resolver never consulted it.
- [x] **Analyser** ✅ **DONE** — gated the path *at the recording site* (`handle_namespace_path_command`): a `namespace path` under a pre-8.5 dialect records no path entry, so the empty path naturally makes command resolution, definition, and hover all skip the path tier (one gate covers every consumer, rather than threading the dialect into each — `finalise_invocation_resolutions` already reads the recorded `namespace_paths`).  Matches the `namespace path` subcommand's own `TCL85_PLUS` gate (which already flags the command W002 there).  A bare `helper` under `namespace path ::mymod` gains `::mymod::helper` as a candidate from 8.5 on, never under 8.4.  Test: `bare_call_honours_namespace_path_only_from_8_5`.
- [ ] **VM mirror** — the VM's dispatch ([`interp.rs:1658`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L1658)) is runtime-execution parity, separate from LSP navigation; left for the VM-parity pass (M16-adjacent).

**Stage 10.2 — `namespace unknown` re-gate (D9).** ✅ **DONE** — re-gated the `namespace unknown` subcommand from `NON_IRULES_OPERATORS` (which admitted 8.4) to `TCL85_PLUS`, matching its sibling 8.5 feature `namespace path`.  An 8.4 document now gets W002 (disabled-in-dialect), not silent acceptance.  Test: `namespace_unknown_is_dialect_gated_to_8_5_plus`.  (The vendor-shell subcommand-gating model — a single `DialectSet::parse` bit, so an 8.5-base vendor shell is treated like `namespace path` — is the deliberate existing design the expr-operator `expr_grammar_base_version` fix left untouched, so this stays consistent with `path`.) **Depends on:** —.

---

## M11 — Cross-version variable semantics

**Fixes D4 (c-source)** — the *only* resolution-semantics change 8.4→9.1. 8.4/8.5/8.6 fall back to global for an unqualified undefined var at namespace scope; **9.0 removed it** ([8.6 `tclVar.c:757`](https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/generic/tclVar.c#L757) keeps it vs [9.0 forces `TCL_NAMESPACE_ONLY`, `tclVar.c:935`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclVar.c#L935); [9.0 `changes.md:189`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/changes.md#L189)). Rust hardcodes 9.0 for all dialects ([`interp.rs:666`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L666); [`vars.rs:107`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/runtime/rust/src/vars.rs#L107)).

**Stage 11.1 — gate the fallback** Keep it for `TCL8X`, drop for `TCL90_PLUS`, in the runtime resolver, VM, and analyser var path.
**Stage 11.2 — pin** Vector both behaviors against `tclsh8.6` and `tclsh9.0`. **Depends on:** M2.

---

## M12 — Expr-function fidelity (`::tcl::mathfunc`)

**Stage 12.1 — dialect gating (D7 c-source).** ✅ **DONE** — `tcl_syntax::expr::mathfunc::added_in` is now the single source of truth for which names are `expr` functions and the release each first appeared in (8.4 fixed table / 8.5 TIP 232 `min`/`max`/`isqrt`/… / 9.0 TIP 521 `is*` classification / 9.1 TIP 745 C99).  The const-folder declines to fold a function newer than the dialect's expr-grammar base version (`min(…)` folds nothing under 8.4 — the runtime would error, not yield a constant), and the analyser emits W002 (disabled-in-dialect) on the function token for the same case.  Both read one shared `math_func_ceiling_for_dialect`.  Tests: `math_functions_fold_only_from_their_introducing_release`, `expr_function_before_its_release_is_disabled_in_dialect`.
**Stage 12.2 — proc linking (D8).** ✅ **DONE** — `ExprNode::function_calls` walks the expression AST for every math-function application; the analyser records each as an invocation whose written head is the bare tail and whose resolved name is `::tcl::mathfunc::<f>`, so a user `proc ::tcl::mathfunc::f` gets go-to-definition, references, rename, and arity, and is no longer flagged unused (`finalise_invocation_resolutions` recovers the `::tcl::mathfunc` namespace from that pair, so rename rewrites only the tail token).  Tests: `expr_function_call_records_a_mathfunc_invocation`, `expr_nested_function_calls_are_both_recorded`. **Depends on:** —.

---

## M13 — TclOO version fidelity (`property`)

**Stage 13.1 — gate `property` (D10).** ✅ **DONE** — `MemberSpec` gained an optional `dialects` gate; the `property` member carries `TCL90_PLUS`, so a document using it under an older core draws W002 (disabled-in-dialect) on the member keyword and records no property, while 9.0+ accepts it.  When the enclosing definer is itself disabled (`oo::configurable`, also 9.0+), the member gate is bypassed so the body still resolves structurally and the version-only construct draws a single diagnostic rather than a cascade.  Tests: `property_member_is_gated_to_9_0`, `disabled_definer_nested_in_catch_reports_w002_once_without_cascade`.
**Stage 13.2 — accessors (N3).** ✅ **DONE** — a configurable class (the `oo::configurable` metaclass, or any class carrying `property` declarations) answers `configure`/`cget` for its properties, so `known_methods` folds those accessor words in even though no `method` body defines them — `$obj configure -x …` / `$obj cget -x` no longer look like unknown methods.  Test: `configurable_class_knows_configure_and_cget`. **Depends on:** M4.

---

## M14 — Dynamic reference roles (`ArgRole::CommandName`)

Introduce a `CommandName` reference role (there is only `CommandPrefix` today) and
apply it wherever a command name appears as a data argument — merging the
C-source trace finding (D11) with the dynamic-surface introspection/ensemble
findings.

**Stage 14.1 — the role** ✅ **DONE** — added `ArgRole::CommandName` (the whole word is a bare command name held as data, introspected not invoked — distinct from `CommandPrefix`, which appends args and carries a callback arity).  The analyser's `record_command_name_invocations` records each such literal argument as a `command_invocation` (`argc`/`callback_arity` both `None` — a reference with no arity to check), so find-references / go-to-definition / rename / call-hierarchy reach the named command through the ordinary invocation machinery.  A dynamic word (`info body $p`) is skipped.
**Stage 14.2 — apply it** ◐ **MOSTLY DONE** — applied `CommandName` to `info args` / `info body` / `info default PROC` (proc introspected by name), `namespace origin NAME` (command resolved by name), and `trace add`/`trace remove` `command`/`execution NAME` (the traced command — via `trace_add_arg_roles`'s existing per-type resolver, leaving the trailing callback its separate `CommandPrefix`).  Tests: `references_proc_named_in_info_body_include_the_introspection_site`, `references_proc_named_in_trace_add_execution_include_the_trace_site`, `arg_indices_for_role_trace_add_command_name_reference`.  `namespace which ?-command? NAME` carries a flag-conditional `arg_role_resolver`, but only the `-variable` form's `VarRead` is applied: an initial `CommandName` on the `-command` form was **reverted** in review — `namespace which` is an existence *probe* (returns `""` for an unknown command), so feeding the name into the W123 unresolved-command pass wrongly flagged a legitimate `[namespace which -command foo] eq ""` check.  Navigating a *probed command* needs a reference role that records the link without asserting existence, which the model does not have yet, so the `-command` form contributes no role.  Tests: `namespace_which_command_probe_does_not_flag_unknown`, `namespace_which_command_probe_is_not_a_reference`.  The `coroutine NAME` created command is already handled by `defines_command_at`.
**Stage 14.3 — ensembles** ✅ **DONE** — `handle_namespace_ensemble` now parses `namespace ensemble create -map {sub target …}` (each odd *target* element is a command reference, resolved in the ensemble's namespace) and `-subcommands {a b …}` (each name maps to the command `<ns>::a`), recording both as command references so navigation reaches the implementing procs.  Element spans are located inside the list-word token via a shared `list_word_elements` helper; a dynamic element is skipped.  The four command-reference recorders (`forward` target, expr math function, `CommandName` argument, ensemble target) now share one `push_command_reference` sink.  Test: `namespace_ensemble_map_and_subcommands_record_command_references`. **Depends on:** M3.

---

## M15 — Interpreter/scope isolation & coverage

**Stage 15.1 — cross-interp isolation** ✅ **DONE** — `interp eval CHILD SCRIPT` now opens an isolated child scope (new `AnalyserHookId::InterpEval`, stamped on `interp`'s `eval` subcommand; `handle_interp_eval_command` mirrors `handle_namespace_eval_command` but homes the child's definitions under the interpreter path — `::<child>::foo` — so they never merge with the parent's `::foo`).  A parent `rename foo` no longer reaches a child `proc foo`, and the child's own calls still resolve within the block.  An empty path (`interp eval {} script`) targets the current interpreter, so it falls through to the ordinary in-scope walk; a multi-word / dynamic script keeps the pre-existing handling; a dynamic path (`$i`) stays conservatively isolated (its scope name can't collide with a real namespace).  ([`interp.rs:369`](https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/rust/tcl-vm/src/interp.rs#L369) is the runtime isolation ground truth.)  Tests: `interp_eval_child_isolates_definitions_from_the_parent`, `interp_eval_empty_path_is_the_current_interpreter`, `rename_parent_proc_does_not_edit_a_child_interp_body`.
**Stage 15.2 — inscope/code** ✅ **DONE** — `namespace inscope NS SCRIPT` now shares the `namespace eval` analyser hook (its `[subcmd, ns, body]` shape is identical), so the body is walked in `NS`'s scope — a bare `proc foo` homes to `::NS::foo`, not the caller's `::foo`.  `namespace code SCRIPT` gained an `ArgRole::Body` on its script: the captured script runs in the *current* namespace when the callback fires, so it is analysed in this scope (its references / definitions were previously invisible).  Tests: `namespace_inscope_runs_the_body_in_the_named_namespace`, `namespace_code_analyses_the_script_in_the_current_namespace`.
**Stage 15.3 — per-object methods** ✅ **DONE** — `handle_oo_objdefine` now walks the per-object definition instead of only recording the object variable.  It shares the `oo::define` member grammar, so the body / inline forms are parsed with the same helpers into a *throwaway* `ClassDef` — deliberately **not** registered in `all_classes` (a per-object extension is not a class and must never leak into class listings, hover, rename, or completion), homed under a private synthetic `@objdefine@…` name so the duplicate detector never confuses a per-object `greet` with the class's own.  Two effects follow: (1) each method **body** is walked into the scope tree, so in-body diagnostics and variable/command resolution light up exactly as inside an `oo::define` method (previously the whole `oo::objdefine` body was unparsed); the handler now returns `true` to own that walk, mirroring `handle_oo_define_command`.  (2) the method **declarations** are recorded in a new `AnalysisResult.object_methods` map (keyed by the object's simple name); go-to-definition consults it *ahead* of the class via `lookup_object_method`, so `$obj m` resolves the per-object override, not a same-named class method — matching TclOO's per-object-method layering.  Tests: `oo_objdefine_body_methods_are_analysed`, `references_proc_called_inside_oo_objdefine_method_body`, `definition_instance_method_prefers_per_object_override`. **Depends on:** M6.
**Stage 15.4 — 9.0 `::tcl::` reorg (N4)** ✅ **DONE** — the major 9.0-added `::tcl::` sub-namespaces were already modelled with dialect gating (`tcl::process` → `TCL90_PLUS`, `tcl::mathop` → `TCL85_PLUS`, `tcl::build-info`, `tcl::idna`, `tcl::unsupported::corotype`).  The outstanding gap was `zipfs` (TIP 430, shipped in 9.0), entirely unmodelled — a `zipfs mount …` in 9.0 code drew a bogus unknown-command W123.  Added a `zipfs` command spec (both the public `zipfs` and the fully-qualified `::tcl::zipfs` ensemble forms, mirroring `tcl::process`), gated `TCL90_PLUS`, with the full 9.0 `zipfs.n` subcommand set (`mount`/`mountdata`/`unmount`/`mkzip`/`mkimg`/`lmkzip`/`lmkimg`/`mkkey`/`exists`/`info`/`list`/`find`/`canonical`/`root`) and their argument counts transcribed from the manual page.  A 9.0 call now resolves (no W123, a bogus subcommand is W001), and under 8.6 the command is W002 (“disabled in the active dialect profile”) — pointing the user at the version instead of a bare unknown-command.  Test: `zipfs_is_a_9_0_ensemble_gated_out_of_earlier_dialects`.  (Removals from `::tcl::` internals are not modelled: they are undocumented `unsupported` names with no authoritative per-version availability oracle in-repo, and speculatively gating them risks false negatives.) **Depends on:** M3, M5.

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
