# Name resolution — full fix plan

**Status:** execution plan. No code here has landed. Turns the findings in
[name-resolution-centralization.md](name-resolution-centralization.md) and
[cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md)
into an ordered, concrete sequence of milestones. Each milestone lists the
files to touch, the specific change, and how it gets verified. Read the two
linked docs first for the *why*; this doc is the *how* and *in what order*.

Milestones are ordered by (severity of what they fix) × (how little they
depend on anything not yet built). M0 fixes live silent-correctness bugs
with no new architecture. Later milestones build genuinely new machinery
and depend on earlier ones being in place.

## M0 — collapse target-selection duplication onto the existing canonical resolver

**Fixes:** the Tier 1/2 items in the centralization doc's Part A — wrong-
symbol Rename, wrong-node Call Hierarchy, cross-file wrong-symbol Rename/
References, the Linked Editing Range OR-bug, plus the lower-severity Tier 2
sites. No new algorithm needed — `definition.rs::resolve_called_proc` /
`proc_visible_from_namespace` is already correct; the fix is routing
everyone else through it.

1. **Establish the shared entry points.**
   - Confirm `resolve_called_proc`/`proc_visible_from_namespace`
     (`tcl-lsp-core/src/definition.rs`) are visible to every intra-crate
     caller (`rename.rs`, `call_hierarchy.rs`, `references.rs`,
     `implementation.rs`, `hover.rs`, `type_hierarchy.rs`,
     `signature_help.rs`, `inlay_hints.rs`, `linked_editing_range.rs`,
     `workspace_index.rs`) — likely already `pub(crate)`, just unused by
     them; if any are private, widen to `pub(crate)`, not `pub`.
   - `tcl-lsp-server::resolve_workspace_symbol` lives in a different crate
     (`tcl-lsp-server`, not `tcl-lsp-core`) — its same-file tier needs a
     `pub` (crate-external) re-export or a thin `tcl-lsp-core` wrapper it
     can call; check what's already exported from `tcl-lsp-core`'s
     `lib.rs` before adding new surface.
   - Build the class-oriented analogue. Check first whether `definition.rs`
     already has one for classes (go-to-definition on a class name works
     today, so *something* resolves it) — if it exists, promote it to the
     same shared-helper status as `resolve_called_proc`; if it's itself one
     of the bespoke `.iter().find()` scans, fix it in place here rather
     than adding a second thing to migrate later.

2. **Migrate Tier 1 sites**, each a small diff replacing a `.or_else(||
   analysis.all_procs.iter().find(...))`-shaped fallback with a call to the
   shared resolver, keeping each file's existing "cursor is exactly on the
   declaration's `name_span`" fast path untouched (that tier is already
   correct):
   - `rename.rs`: `rename_proc`, `rename_class`, `prepare_rename`.
   - `call_hierarchy.rs`: `find_proc_for_item`, `prepare`.
   - `references.rs`: `proc_references`, `class_references`.
   - `tcl-lsp-server/src/lib.rs`: `resolve_workspace_symbol`'s same-document
     loop (leave the workspace-index fallback below it for M1/M2 — it's a
     different bug, gated on `WorkspaceIndex` which M2 is fixing anyway).
   - `linked_editing_range.rs`: this one isn't a missing-helper problem,
     it's a boolean-logic bug (`matches_self_call`'s result is OR'd with
     the resolved check instead of only being consulted when
     `resolved_qualified_name` is absent). Fix: when
     `inv.resolved_qualified_name` is `Some`, it is authoritative — only
     fall back to `matches_self_call` when it's `None`. Add the regression
     test from the centralization doc's repro (`::a::greet` calling into a
     `::b::greet` via a nested `namespace eval` block) directly.

3. **Migrate Tier 2 sites**, same shape, lower urgency but do in the same
   PR since the diff pattern is identical and it's a lot cheaper to do
   while the helper is fresh in context:
   - `implementation.rs`: replace `strip_colons`'s leading-colon-only strip
     with a call to the shared class resolver (fixes the "bare superclass
     reference to a namespaced base" miss directly, rather than patching
     `strip_colons` in isolation).
   - `signature_help.rs` + `inlay_hints.rs`: first dedupe the byte-identical
     `lookup_proc` into one function (either promote one to
     `tcl-lsp-core`'s shared utilities or have one call the other), then
     route that through `resolve_called_proc`.
   - `hover.rs`: `lookup_class`, `alias_hover_text`.
   - `type_hierarchy.rs`: `prepare`.
   - `workspace_index.rs`: `proc_definitions`/`class_definitions` need an
     ambiguity gate at minimum (they currently have none, unlike
     `invocations_of`'s `bare_is_safe`) — full fix folds into M2 since it's
     the same function family the oracle work touches; if M2 isn't
     immediately following, add the gate here as a standalone stopgap.

4. **Tier 3 — defense in depth, lower priority, can slip to a follow-up
   PR**: `minify.rs`, `graphs.rs` (harden the dead fallback anyway),
   `type_definition.rs`, `tcl-mcp/src/tools.rs`'s `generate_docstring`
   (also fix the stale "mirrors `AnalysisResult.find_proc`" doc comment —
   that method doesn't exist), `class_lattice.rs`'s `resolve_class_name`
   (superseded by M1, see below).

5. **Test strategy**: add one shared test fixture — a canonical "two
   namespaces define a same-named proc" and "two namespaces define a
   same-named class" Tcl snippet pair — under a common test-support module,
   and add one test per migrated feature (rename, call hierarchy,
   references, linked-editing, hover, signature-help, inlay-hints,
   implementation, type-hierarchy) that triggers the feature from the
   *ambiguous call site* and asserts it resolves to the correct symbol.
   Reusing one fixture across all of them means the same shape is pinned
   everywhere in one place, instead of each test file inventing its own
   (and potentially missing the exact shape that mattered).

**Risk**: low. Every change replaces a wrong/inconsistent lookup with a
call to an already-shipping, already-correct function. No behavior change
for any call site that was already unambiguous (the overwhelming majority).

## M1 — consolidate class-name resolution onto the verified-correct rule

**Fixes:** the confirmed-wrong `class_hierarchy.rs::resolve_class_name`
(ancestor-walk vs. real Tcl's one-hop rule), and removes two more duplicate
implementations (`class_lattice.rs`, `var_command.rs::canonicalise_class_name`).

1. **Pin the correct algorithm first.** Before touching `class_hierarchy.rs`,
   add conformance vectors for class-name resolution to (or alongside) the
   existing `command_resolution_vectors.txt` machinery — since resolving a
   class name is resolving a command name, the *shape* of the pinning
   should match: current-namespace-relative, then global, no ancestor walk,
   verified against real `tclsh`. Include the specific "namespace 2+ levels
   deep, base only exists one level up, not in an ancestor" vector that
   exposes the bug, plus **the open question the class-lattice experiment
   raises**: does `superclass`/`mixin` resolution honor a `namespace
   import`ed name the same way ordinary command resolution does? Pin that
   against real tclsh explicitly rather than assuming either answer — this
   determines whether `class_lattice.rs`'s `NsContext` (which tracks
   imports) is capturing real behavior the one-hop rule is missing, or is
   itself over-engineered relative to what TclOO actually does.
2. **Replace `class_hierarchy.rs::resolve_class_name`** with a call into
   `tcl_syntax::naming` (`bareword_resolution_candidates`/
   `resolve_command_with`) using the *analyser's own* namespace-tracking
   the same way command-invocation resolution does, rather than a fourth
   hand-rolled walk. Feed the corrected resolver back through
   `resolve_super_name`/`build_supers_mixins_maps` unchanged in shape.
3. **Retire the duplicates**: delete `var_command.rs::canonicalise_class_name`
   and route W308's constructor-object-type harvesting through the
   consolidated resolver; either delete `class_lattice.rs::resolve_class_name`
   or — if step 1's import-tracking question resolves in favor of needing
   it — fold `NsContext`'s import-prefix tracking *into* the consolidated
   resolver instead of leaving it as a parallel, separately-invoked
   implementation.
4. **Regression sweep**: run the full MRO (`analyser/mro.rs`), class
   hierarchy, `type_hierarchy.rs`, W308, and `workspace_index.rs` cross-file
   class test suites — this resolver feeds all of them, so a behavior
   change here has the widest blast radius of any milestone in this plan.
   Treat any test failure as a signal to re-examine, not to special-case
   around.

**Risk**: moderate — this is the one milestone that changes behavior for
code that isn't obviously buggy today (the ancestor-walk fallback
"worked," just not per real Tcl semantics, for any codebase that happened
to write `superclass` bareword references matching its own ancestor-walk
assumption). Mitigate with the conformance vectors in step 1 landing and
passing *before* the resolver swap, and a full regression run after.

## M2 — workspace-scoped resolution oracle (cross-file reference enumeration)

This is [cross-file-command-resolution-lattice.md](cross-file-command-resolution-lattice.md)'s
phase 1, sequenced here as its own milestone because M0/M1 both produce
consumers that benefit from it existing. No change to that doc's technical
content; restating the steps for sequencing clarity:

1. Extend the analyser's `finalise_invocation_resolutions` to record the
   full priority-ordered candidate list per invocation (`Vec<String>`,
   additive alongside the existing collapsed `resolved_qualified_name`).
2. Add `WorkspaceIndex::workspace_command_exists` — an `exists`-shaped
   oracle over the merged workspace (every indexed file's procs/classes/
   aliases/renames).
3. Rewrite `invocations_of` to run the recorded candidate list through
   `resolve_command_with` against that oracle instead of its four bespoke
   clauses; retire those clauses.
4. Fold in the ambiguity gate for `proc_definitions`/`class_definitions`
   noted in M0 step 3, if not already done as a stopgap there.
5. Add the multi-file conformance vector format described in the companion
   doc's Testing section.

**Depends on**: nothing from M0/M1 structurally, but M1's corrected class
resolver should land first if M3 (TclOO cross-file) is coming next, so M3
isn't built on the ancestor-walk bug.

## M3 — TclOO cross-file method references

Companion doc's phase 3. Depends on M1 (correct class-name resolution) and
M2 (the workspace oracle) both being in place — extend `WorkspaceIndex`
with a method table per class, wire `resolve_workspace_symbol` and
`cross_document_references` to check it, reuse `class_lattice.rs`'s
`NsContext` (now consistent with M1's consolidated resolver) for the
class-name half of `$obj method` resolution.

## M4 — variable resolution: spike, then targeted fixes

**Do not** attempt a direct command-resolution-style unification here
without the spike — the centralization doc's Part C found four genuinely
different data models (VM flat-frame map, runtime arena-tree, analyser
scope-tree, compiler SSA/place), not four implementations of one
algorithm.

1. **Spike deliverable**: a short doc answering whether *any* shared
   abstraction (even just a common `exists`-oracle-shaped interface, not a
   shared candidate-list algorithm) is viable across the VM and runtime's
   genuinely different structures, or whether variable resolution is
   correctly two-backends-plus-two-static-consumers by nature and the
   actionable fix is narrower than "centralize."
2. **Ship regardless of the spike's outcome** (low-risk, same-crate):
   - Stop `analyser/handlers.rs` from reimplementing `var_scoping.rs`'s
     declaration-index grammar inline; call the existing helper (fixes the
     `$`-prefix-exclusion gap noted in the audit as a side effect).
   - Fix `namespace-model.md`'s broken pointer (it doesn't actually cover
     variable resolution) and mark `runtime-variable-frame-model.md`
     explicitly as aspirational/not-current in its own header if it isn't
     already unambiguous about that.
3. **After the spike**: scope any cross-backend work as its own follow-up
   plan — out of scope for this document to pre-commit to an approach.

## M5 — SCCP-backed command-name-in-variable resolution

Companion doc's phase 4, unchanged. Still gated on the open question there
(whether SSA/SCCP is already computed per-document as part of the existing
diagnostics pipeline, or would be new cost) — resolve that as a short spike
before committing to the design in detail.

## M6 — library/package lazy resolution tier

Companion doc's phase 5, unchanged: lazy-analyse a `PackageResolver`-located
file on an oracle miss, memoise, merge into the same oracle M2 builds.
Depends on M2 existing (same oracle interface).

## Cross-cutting, can happen anytime

- **Contract-doc corrections** (cheap, do early, no code risk):
  `command-resolution.md`'s WASM-codegen line (inherits the *runtime's*
  conformance via eval-delegation, not "the VM's"); `tcloo-implementation.md`
  (describes stale pre-Rust-port Python modules).
- **Drift-prevention lint**: once M0 lands and the sanctioned helpers are
  the obviously-right thing to call, add an `xtask` check (grep-shaped is
  fine to start) that flags new `.iter().find(...)`-style name-equality
  scans over `all_procs`/`all_classes` outside `tcl_syntax::naming` and the
  handful of sanctioned LSP-side helpers. This is what actually prevents
  the next version of the #923 regression — the existing "add a vector"
  discipline only protects consumers that already opted in, which is
  exactly how 17 places went unnoticed.

## Summary table

| Milestone | Fixes | New architecture? | Depends on |
|---|---|---|---|
| M0 | Wrong-symbol Rename/Call-Hierarchy/Linked-Editing, ~15 duplicate lookups | No | — |
| M1 | Wrong MRO/superclass resolution | No (reuses `tcl_syntax::naming`) | — (M0 optional but do first) |
| M2 | #923 cross-file references regression | Yes (workspace oracle) | — |
| M3 | TclOO cross-file method references | Extends M2's oracle | M1, M2 |
| M4 | Variable resolution duplication | Spike first | — |
| M5 | Command names held in variables | Yes (SCCP integration) | M2 |
| M6 | References into libraries/packages | No (wires existing pieces) | M2 |
