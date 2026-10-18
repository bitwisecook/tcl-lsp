# Luna-17 review: sol-15 row-14 patch and sol-16 receiver-collection patch

Base: `/home/user/tcl-lsp` branch `work`, HEAD `75c75665f`. No Rust file has changed since `db5a1ba9a` (sol-15's base) or `a146f2608` (sol-16's base).

Method: static reading only. The repo was not modified, and no cargo or tests were run. One tclsh 8.6/9.0 probe was run (`probe/p1.tcl`). For rustfmt, patched copies were extracted with `git archive` into `tree1/` and `tree2/`.

| Patch | `git apply --check` | rustfmt `--edition 2024 --check` | `#[allow]` | Verdict |
|---|---|---|---|---|
| sol-15 `row14-lsp-linked-logical-name-geometry.patch` | OK | clean | none | **HOLD** (one-predicate fix) |
| sol-16 `sol-16-receiver-collection.patch` | OK | clean | none | **HOLD** (one-line fix), then admissible |

---

## 1. sol-15 row-14: `retain_positive_logical_procedure_call`

### The diagnosis is correct for the case it targets

- The catalogue walk (`body_effects.rs:446-458`) and the immediate Logical `namespace eval` walk (`body_effects.rs:610-627`, `inventory: None` for Structural) each write into their own new tape. Neither ever calls `retain_logical_navigation`, which runs only when `inventory` and `logical_scope` are both set (`body_effects.rs:1182`). So `logical_variable_words` is always empty in those tapes, and a **bare** head (`recur`, `p`) can never have a recipe there. Their `None` at a bare head is pure absence.
- `merge_receipt` (`source_class.rs:637-650`) turns `Some` against `None` into `None` regardless of order. That is how `LogicalBodyWalk`'s genuine `Some` (`logical_body.rs:778`) gets erased. Fixtures 628 (`proc recur {} {recur}`) and 629 (`proc p {} {puts p; p}`) are bare self-calls, so the expected flips are plausible.
- 626 (`::N::café` inside `namespace eval`) is correctly not claimed. Only `LogicalBodyWalk` reaches a deferred proc body: the catalogue walk does not enter `proc` (`DEFERS_BODY`), and neither does the namespace walk.
- Two kinds of conflict are still withdrawn:
  - Two different `Some`s.
  - A `None` from a walk with both `inventory` and `logical_scope` set.
- A positive-only `Some` cannot overturn an earlier withdrawal: `previous None != Some` keeps `None`.
- The Native route is unchanged. Both functions return early unless `graph.policy` is `Logical` (`logical_procedure_invocation.rs:186,208`), and `owns_logical_recipe` has no effect under Native.

### Blocking finding: the walk-shape predicate is not "owns a recipe"

The patch decides positive-only by walk shape: `owns_logical_recipe = inventory.is_some() && logical_scope.is_some()` (patched `body_effects.rs:1212`). But `capture_logical_procedure_call` has a second recipe, the **Absolute** recipe (`logical_procedure_invocation.rs:214-220`). Any walk with a syntax body retains it for a literal `::`-prefixed head (`body_effects.rs:1166-1181`, `logical_navigation.rs:504-518,417-447`). One such walk is the top-level immediate Plain-body walk:

- `apply_immediate_body_effects` is called from `apply_schema_transition(..., None)` (`source_transition_advice.rs:2862 → 2480`).
- For `if`, `while`, `foreach` and similar at document root under Logical, that walk has `inventory = Some` (Plain), `source_body = Some` (`body_effects.rs:774`) and `logical_scope = None`.
- It runs on the **actual** graph, after the condition expressions' effects have been applied: `expressions()` mutates `graph`, and `retain_possible_effects` marks changed cells `Shadowed` (`body_effects.rs:884-903, 1035-1076`).
- No `LogicalBodyWalk` visits a top-level `if` body, because `logical_namespace_scope` needs `DECLARES_NAMESPACE`.

So the most precise walk for that body becomes positive-only. Its `None` comes from a recipe plus the evidence of a shadowed or deleted target, and that `None` is now discarded. The catalogue walk runs on the pre-`if` graph and reads only the bodies, not the condition (`body_effects.rs:422-433`). It still produces a stale `Some`, and that `Some` now survives.

Counterexample (tclsh 8.6.18 and 9.0.4: `invalid command name "::foo"`; see `probe/p1.tcl`):

```tcl
proc foo {} {}
if {[rename ::foo {}] eq ""} { ::foo }
```

- At HEAD, the catalogue `Some(foo)` merges with the immediate `None` and the result is withdrawn, which is correct.
- With the patch, the immediate walk's `None` is not recorded. A `source_logical_procedure_call_at` receipt for `::foo` is issued against the deleted proc.

The variant `if {[proc ::foo {a b} {}] eq ""} { ::foo }` gives a receipt for the old 0-formal header, while tclsh reports `wrong # args: should be "::foo a b"`. That suppresses the true arity diagnostic and links references, rename and inline edits to the wrong header.

Consumers do not gate on `has_unresolved_applicability()`:

- `references.rs:173,737`
- `linked_authoring.rs:58`
- `definition.rs:397,505,564,1686`
- `hover.rs:405`
- `signature_help.rs:167`
- `call_hierarchy/logical_source.rs:298,607,748`
- `refactor/inline_proc.rs:144`, which offers an edit
- W143 suppression, `validity.rs:2357`

The arity subject (`diagnostic_subject.rs:366-371`) only adds `UnknownEarlierMutation`.

Secondary, narrower case: the catalogue walk of a `namespace eval` body uses the **outer** namespace, because it calls `walk.script` with no `prepare_region_branch`, and `publication_key` uses `graph.logical_namespace` (`source_transition_advice.rs:1682`). So `namespace eval N { proc p {} {}; ::p }` gives a catalogue `Some` for `::p`, even though tclsh says `invalid command name "::p"`.

- At HEAD, the immediate namespace walk's blanket `None` also hides this wrong positive.
- With the patch, only `LogicalBodyWalk::namespace` withdraws it. If that walk's `parent_scope` fails, its whole tape is discarded (`logical_body.rs:318-337`), and the wrong `Some` would survive.

### Required change: gate on whether the walk had a recipe at this head, not on walk shape

Make "positive-only" mean "this walk held no recipe at this head". Every walk records its `None` whenever it held a Named or Absolute recipe there. One way:

```rust
// in retain_positive_logical_procedure_call (or fold into retain_logical_procedure_call)
let has_recipe = self.original_logical_source_word_context(input, original).is_some()
    || self.logical_absolute_navigation.get(&head.span().start()).is_some_and(Option::is_some);
if !has_recipe { return; }            // absence, not evidence
merge_receipt(.., self.capture_logical_procedure_call(graph, original));
```

- This still flips 628 and 629, because bare heads in the catalogue have no recipe.
- It keeps the immediate walk's `Shadowed` evidence.
- It removes the `owns_logical_recipe` proxy, or keeps it as `owns || has_recipe`.

Residual, conservative and not a regression: absolute self-calls inside a Logical proc body are still withdrawn. The catalogue's Absolute `Some` or `None` always differs from `LogicalBodyWalk`'s Named `Some`, because the recipe variants and `earlier_unknown_mutations` differ.

Add regression tests:

- `logical_procedure_invocation::tests`: the two `if`-condition cases above must give `source_logical_procedure_call_at(... "::foo" ...)` → `None`.
- `namespace eval N {proc p {} {}; ::p}` → `None`.
- `proc recur {} {recur}` → `Some`.

### Tests whose results may change (run all of them)

- Targets:
  - `linked_editing_range::tests::{linked_spelling_lifts_lone_cr…(628), linked_spelling_rejects_surrogate…(629), logical_linked_spelling…(626, no flip expected)}`
  - `signature_help` 512
- Compiler:
  - `command_binding::source_transition_advice::logical_procedure_invocation::tests::*`
  - diagnostics tests for Logical arity (`SourceUserCallAritySubject`) and W143 (`diagnostics/tests.rs` near 18553)
- LSP:
  - `references` and code-lens reference counts (Logical, in-body calls)
  - `definition::tests::logical_*` (4740-4800)
  - `hover`, `call_hierarchy::logical_source`, `refactor::inline_proc`, `original_command_rename::linked_authoring`, `signature_help`
- The interprocedural, inliner and optimiser consumers are **not** affected. They use `original_logical_procedure_calls_for_module` (declaration-layout advice, `materialized_footprint.rs:79`), not this tape.

---

## 2. sol-16: typed collection receiver

### Correct parts

- Registry-only: `element_extraction` reads `ReturnElements::ElementOf` and exact argv from the schema. No command names are used.
- The scalar-text `ObjectClassMap` lookup and `scalar_reference_name` are removed. The local map still feeds `augment_loop_var_handles` (`semantic_tokens.rs:4340-4372`), so there is no dead field.
- Element agreement is enforced by the owner (`project_class_cards.rs:480-501`):
  - Any element whose constructor does not resolve aborts with `?`. Registry-factory elements such as `[listbox .l]` therefore abstain instead of being skipped.
  - Factory pointer and metadata must agree across elements.
  - An empty union gives `selected?` → `None`.
- `provides_method()` reads the original selector word (`project_class_cards.rs:245`).
- The `collection_source().is_none()` guard ensures only the collection branch marks.
- `original_collections` was `None` in Logical non-project mode. For `[C new] ping` heads, setting it to `Some` is harmless: `for_receiver` → `read_at` → `source_registry_words_at(C)` is `None` because of the SourceClass barrier, so control falls through to the constructor branch.
- Match exhaustiveness is fine: `ReceiverSourceCandidates` is matched only at `semantic_tokens.rs:1958`.
- No Native change: `insert_object_method_overrides` runs only when `ctx.lexical` (`semantic_tokens.rs:3513`). Native tcl9.0 fixtures do not enter it.

### Blocking finding: project-mode refusal bypass

At HEAD, `insert_object_method_overrides` first asks `insert_project_method` with the real `providers`. That call returns `false` exactly when the owner refused, for example on cross-file same-name class ambiguity (`finish`, `project_class_cards.rs:306-355`). After that, HEAD sets `classes = None` in project mode ("Typed project mode cannot recover a refused source class", `semantic_tokens.rs:1916-1917`). In HEAD's `ClassNames` path, only the registry branch can still mark.

The patch routes the same head to `insert_collection_method`. That calls the owner again with `providers = &[]` (patched `receiver_source.rs:203-208`). With no foreign providers, `finish` sees no ambiguity and returns the local class. It then marks `Method` and configure options that the project-aware owner had just refused. The same bypass applies when `classes` is `None` in project mode, where the project owner is never consulted.

**Fix:**

- Pass `ctx.project_classes.unwrap_or(&[])` as `providers` in `insert_collection_method`, or return early when `ctx.project_classes.is_some()` because the project path already owns the answer.
- Add a project-mode negative test: a provider with a conflicting `::C` plus `dict set items k [C new]; [dict get $items k] ping` → no `Method`.

### Non-blocking findings

1. **Absence-as-completeness (inherited, not new).** `OriginalObjectCollectionSources::capture` skips writers whose value is not a single command substitution: `lappend objs $x` hits `continue` at `original_collection_sources.rs:317-320`. Non-`VarElementsEffect` writers (`set objs …`) are never represented. The owner then selects the class of the represented writers only. `has_unknown_residual()` is constant `true` (`:98`) and is never consulted.
   - Example: `lappend objs $foreign; lappend objs [C new]; [lindex $objs 0] ping` would now be painted as `C::ping` in Logical non-project mode.
   - This matches the existing project consumer and is presentation-only. NOTES' sentence "Absence is never read as completeness" overstates it. Record it as a residual, or add an owner-side refusal when an unrepresented writer targets the same place.
2. **Scope is wider than NOTES says.** Registry-class collection elements (`object_collection_classes` element classes that are registry specs) lose their Method token in **both** Logical non-project and project fallback modes. HEAD's step-1 `registry.instance_method_at` (`semantic_tokens.rs:1977-1985`) used to cover them in both. NOTES mentions only non-project. No in-repo test covers this; the scans in `tcl-lsp-core` src and tests and in `tcl-lsp-server/tests/e2e` found none.
3. **Coupling.** The collection half of test-474 (`receiver_source/tests.rs:105-123`) now depends on the same `original_recorded_command()`/`capture_class` reborrow (`project_class_cards.rs:418-420`). NOTES suspects that reborrow causes 473 and 475-478. HEAD's scalar path did not depend on it. So fixing line 87 alone will not flip 474 unless that gate is also fixed.
4. **Redundant routing.** `element_extraction` repeats the owner's own exact-argv/`ElementOf` check (`original_collection_sources.rs:459-470`). Both outcomes are "no token" when the owner abstains. Returning `CollectionElement` unconditionally in that branch would drop the duplicate rule. Optional.
5. **Cost.** There is one extra `OriginalObjectCollectionSources::capture` per token request for Logical non-project documents with a CU. No timing-sensitive semantic-token test covers that path.

### Admissibility without flipping a selector

Once the project-mode fix is in, it is admissible as obligation closure for "Legacy collection map occurrence/frame identity" at this consumer. That is subject to `make rust-check` (clippy pedantic was not run) and the token suites.

Passing tests at risk (static):

- No passing Logical+CU+collection token test exists. The only one is in test-474, which is already failing.
- Run as smoke:
  - `semantic_tokens::tests::{tcloo_dispatch_pattern_fixture, collection_dispatch_resolves_user_configurable_method, dict_for_loop_var_dispatch_resolves, direct_constructor_dispatch_resolves_method, malformed_braced_object_handle_does_not_resolve_a_method}` (tcl9.0; they should be unaffected)
  - `semantic_tokens::tests` project-mode tests around 10560-10690 (`project_classes: Some`)
  - `semantic_tokens::original::*`
  - `receiver_source::tests::*`
  - `tcl-lsp-server` e2e `semantic_tokens` and `semantic_tokens_reference_client`
