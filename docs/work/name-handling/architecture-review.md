# Architecture review: earlier decisions and the new design together

**Question:** do this branch's earlier design decisions make architectural sense, and does the new design ([design integration](design-integration.md), the [architecture plan](architecture-tidy-plan.md), [validation](name-resolution-validation.md), [plan.md](plan.md) items 66–95) agree with them?

**Method:**
- An independent review read the earlier decisions: the [naming API guide](naming-api-guide.md), the owner decisions and coordinator rulings in [current-state.md](current-state.md), the [fixing guide](fixing-guide.md), the [parser plan](parser-crate-plan.md), the [structured-procedure plan](structured-procedure-plan.md), the release and branch-cut plans, and the main naming contracts in `docs/design/contracts/`. It read them against the new design.
- Its key claims were checked, and the code was measured for sprawl that the earlier decisions themselves produce.

## Verdict

The **core of the earlier design is sound and should bind the new design**:

- the representations are kept apart and display text is never parsed (D1);
- operation rules are chosen per interpreter and per purpose (`NativeNameProtocol`, D2 at the rules layer);
- authorities are separated (D3);
- receipts check currency (D4);
- bindings are positioned and refusals are terminal (D5);
- there is one pipeline, LSP-first (D6);
- the source is parsed once, compiled code must equal interpreted code, and variables go frame → name → cell (D7);
- display answers are kept apart from proof answers (D8);
- each grammar has one owner (D9).

The **way several of those ideas were carried out has become sprawl**, which the new design must reverse rather than build on:

- "purpose" came to mean "consumer feature";
- authorities became type-name prefixes;
- receipts multiplied per consumer;
- migration became a catalogue of caller moves (D10).

The review also found **six contradictions** between the new design and the earlier decisions. They are resolved below. Two of them, the migration strategy and the release scope, need the owner's decision.

## Measured sprawl from the earlier decisions

- **Type families.** 2,079 of the 8,546 struct and enum names in `rust/` are `Original*` (679), `Native*` (880), `Source*` (355), `Logical*` (119) or `Retained*` (46): 131 types about commands, 110 about variables and 93 about namespaces. Only 27 suffixes appear under more than one prefix. So these are not mirrored copies but a very large vocabulary: one type per question per consumer. 896 of them are in `tcl-compiler` and 136 in `tcl-registry`.
- **Positioned query entry points.** About 30 public functions answer a command, namespace or variable question at a position. Examples: `invocation_at_source`, `source_logical_command_candidates_at`, `source_logical_absolute_command_candidate_at`, `source_callback_procedure_target_at`, `source_class_proc_call_at`, `source_constructor_call_at`, `source_original_procedure_call_at`, `command_is_absent_at`, `diagnostic_command_slot_presence_at`, `implicit_command_namespace_path_at`, `original_variable_symbol_at_offset` and `variable_access_at`. Each is a separate owner for a facet of one question: what does this word name here?
- **A vestigial key.** `invocation_at_source(head: &str, offset)` ignores `head` in both implementations (`realm.rs:354`, `command_binding/site_queries.rs:471`). It has 403 call sites.
- **Contract prose.** `docs/design/contracts/name-handling.md` is 4,611 lines in 120 sections, many named after one consumer ("Configured W308 method proposal receiver", "Hover whole-word presentation geometry"). The owner manifest has 1,567 table rows and the consumer matrix 687 lines. Most of their claims are negative clauses ("grants no Native … authority") with no vector or oracle behind them.

## Contradictions and their resolution

1. **Annotated command versus `SourceInvocationBinding`.** Design integration said the annotated command *replaces* `SourceInvocationBinding`; the validation and item 66 said answers *extend* it; the guide's central API is built on it.

   **Resolved:** the annotated command *is* `SourceInvocationBinding` evolved. It is returned by `invocation_at_source(offset)`, with the unused `head` parameter removed. The roughly 30 positioned queries become facets of that record, each keeping the authority it has today. The guide's example keeps working.
2. **"Abstain, never compute a private answer" versus the display fallback (D8).**

   **Resolved:** answers carry an explicit **Display** authority beside Logical, Native, normal completion and Edit. Display features may use a Display answer built from written words. Proof consumers (diagnostics that claim, rewrites, folds, renames) require Logical or stronger. Hover, completion and tokens therefore keep their fallbacks inside the model, and fault 11's "every refusal has a reason" applies to proof consumers.
3. **Who issues body-unit IDs.** Design integration said binding; the structured-procedure plan says lowering, from the provenance owner.

   **Resolved:** lowering issues them. Today the binding walk runs inside the lowerer's walk (the realm is "produced once by the Lowerer"), so the IDs come from the provenance owner during that walk, as the structured-procedure plan says.
4. **Owner of the variable-reference grammar.** Item 70 said `tcl-lexer`; the naming contract names `tcl_core_types::naming::scan_var_name_end_with`; the parser plan gives naming to `tcl-syntax::naming`.

   **Resolved:**
   - The name-end rule stays in `tcl_core_types::naming`, the lowest layer.
   - The word-level reference grammar (braces, index, release rules) is `tcl_lexer::word_parts::scan_var_ref`, built on that rule.
   - `tcl-syntax::naming` re-exports both for consumers.
   - The element splitter is one function in the lexer's `variable_names`, taking the selected protocol.
5. **Migration by catalogue versus the phases (D10).** *Decided 23 November 2026: freeze and re-map (plan item 96).* The [plan.md](plan.md) header still directs execution of 2,158 catalogued changes (1,321 caller migrations), while phases 3–5 delete many of the walkers and heuristics those migrations target. Running both builds adapters that are then thrown away.

   **Recommended (owner decision):**
   - freeze the catalogue;
   - re-map each unit to *keep*, *delete with phase N* or *re-target to the phase-N owner*;
   - stop migrating callers whose owner is scheduled for deletion.
6. **Release scope.** *Decided 23 November 2026: no release target; `work` is a long-running design branch (plan item 96).* [current-state.md](current-state.md) "Scope and publication" limits the release to the LSP and the paths it uses, and defers VM, BIG-IP and ancillary work. The new order schedules engine work alongside phases 3–5, plus the crate split and the licence change.

   **Recommended (owner decision):**
   - ship the LSP release from the current scope;
   - in the release branch, take only phase 0's correctness backlog items that touch the LSP;
   - mark engine convergence, the crate split, salsa rework and the licence change as post-release.

   Superseded for the crate split and the salsa rework (26 November 2026): `work` has no release target (item 96), and build and iteration time now come first. The split is plan item 90 ([design](../../design/compiler/crate-split-design.md)), scheduled ahead of every other stage with items 117 and 118, and it ends by deleting the hand-written memos. The caching tiers (items 101 and 103, and 46–49 and 73) come late, after phases 4 and 5.

## Reconciliations (not conflicts, but must be stated)

- **The realm is sealed after the fixpoint.** `retained-positioned-command-realm` says an earlier retained producer cannot authenticate a changed module. With decision 1's binding and value loop, the realm snapshot is the post-fixpoint result and is sealed once the loop ends. Spike 94 also checks that the loop gives identical results fresh and incrementally.
- **Specialised summaries stay conditional.** Context-insensitive frames (decision 2) and O103's specialised procedure summaries (item 62) fit together because specialisation lives only in contract instantiation (decision 6). Instantiated results keep the condition they were computed under, as the Jim O103 ruling requires ("valid within that assumption").
- **Deferred-point answers are Logical only.** The worlds of decision 3 support diagnostics and display, never a rewrite, fold or rename, as the uncalled-methods ruling requires.
- **The parser plan feeds structured words.** Item 82 takes the lexer's word parts (the parser plan's first interface, "return each original word of this nested script") as its input. The parser plan covers reading; item 82 carries the result into the IR.
- **"Owner per purpose" and "one semantic model" are compatible.** Purpose means the operation rule (lookup, publication or rename in `NativeNameProtocol`). The semantic model is the query layer over those rules. Neither means one owner per consumer feature.

## Keep, revise, retire

**Keep, binding the new design:**
- D1;
- D2 at the rules layer;
- D5, sealed after the fixpoint;
- D6, D7, D8 (through the Display authority) and D9;
- the 19 November owner decisions and the branch cut.

**Revise:**
- **D3:** the authorities become one small fixed enum carried by receipts: Display, Logical, Native, normal completion, Edit. The `Original*`/`Logical*`/`Native*`/`Source*` type families shrink as consumers move to the annotated command and the semantic model. A new type with one of those prefixes needs a reason in the owner manifest.
- **D4:** one receipt type per authority, with one shared currency key (source image, configuration, registry generation, namespace generation) replacing the per-module custody checks.
- **Contracts:** `name-handling.md`, the consumer matrix and the owner manifest are reorganised by question, not by consumer. Per-consumer sections are deleted as their consumers migrate. Each remaining claim is tied to a vector row or test (fault 13).

**Retire:**
- D10 as the strategy (above);
- the fixing guide's rule that every consumer reads names through `tcl_syntax::naming` (its successor: name text through the syntax owners, resolution through the binding owners);
- the "canonical implementation" status of the string candidate family in `command-resolution.md` (item 69);
- proof records with no oracle or vector as acceptance evidence.

## Top risks, old and new together

1. **Two strategies at once** (contradiction 5): the most waste. Settle it first.
2. **Receipt sprawl carried into `Answer<T>`.** Cap the authority enum and the receipt types at the type level, and gate new ones.
3. **The binding and value fixpoint:** termination, cost, determinism between fresh and incremental runs, and sealing (spike 94).
4. **Memory of per-point worlds** with about 230 MB already retained (#2543, spike 92). If it fails, use sparse deltas or persistent maps before going further.
5. **Precision loss** if the Display authority is not built before consumers move onto `Answer<T>`.
6. **Two live owners during phases 3–5.** Resolution 1 removes the main case: one record, evolved in place.
7. **Scope creep against the release** (contradiction 6).
8. **Governance:** the cross-cutting changes (identity layer, structured words across 143 files) need one owner each. The fixing guide's areas are feature-sliced, and the naming owner has no session. Assign an owner per phase.

## Plan

Items 96–99 in [plan.md](plan.md); decisions 4 and 7 of [design integration](design-integration.md) and items 66 and 70 are amended to match.
