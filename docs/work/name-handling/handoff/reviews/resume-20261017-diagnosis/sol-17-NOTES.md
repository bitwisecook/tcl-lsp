# Sol-17 v2: row `lsp-package-standalone-control-role-residue` (worklist #10)

This revision follows Luna-16's review (`../luna-16/REVIEW.md`). The verdicts were:
- **ADMIT:** A, B, the dead variant, R.
- **HOLD:** C and D.

Work was static only. Nothing was compiled or executed in `/home/user/tcl-lsp`, and the repository was not modified.

The only things run were:
- the `nice tclsh8.6/9.0` oracle probes `probe.tcl` and `probe2.tcl`;
- `rustfmt --check` on a scratch tree;
- `git apply`, in a scratch `git init` copy of `HEAD:rust`.

HEAD is `75c75665f`. None of the touched files changed since `db5a1ba9a`.

## Deliverables

| File | sha256 | Applies to |
|---|---|---|
| `sol-17-admissible.patch` (144 lines) | `6ef76858d2f9fe380409d87eb28b171779e98786f45523a7c18d6bedb7d99f1a` | HEAD. A + B + dead `Condition::TclSatisfies` + R. |
| `sol-17-CD-v2.patch` (342 lines) | `2126a3b7cc11828486e99ae386be5a982323718d2e27fd2d290ff03f20cef26c` | HEAD **with the admissible patch applied**. C-v2 + D-v2 + tests. |

How they were checked:
- `git apply --check` passes for the admissible patch against HEAD.
- In a scratch copy, the admissible patch was applied and then `--check` passed for CD-v2.
- With both applied, `rustfmt --edition 2024 --check` is clean on every touched file, child modules included.
- No `#[allow]` was added.
- The v1 `sol-17.patch` and `parts/` are kept in `superseded/` and are withdrawn.

## Catalogue row (verbatim, `repository-consumer-audit-final.json` `/units/633`)

- **purpose:** "Standalone package-index survey with declared default grammar; live legacy package database scans call this source."
- **site:** `rust/tcl-lsp-core/src/package_resolver/reachability.rs:436` `ScriptWalker::walk_script / walk_if / body_may_terminate` — `let written_head = canonical_written_command(word_raw(text, &words[0]));`
- **api:** "Shared lexer/original static word decoder; Registry role lookup and version owner."
- **required_change:** "Still selects control/body/terminator meaning via canonical_written_command + registry.get(report head), parse_vsatisfies fixed package/provide/vsatisfies vocabulary and reconstructed script data. Needed source-only conditional control owner already exists in original sibling; keep fixed survey purpose but do not let nominal labels prune actually shadowed/aliased/unknown source paths or imply loader availability. body_may_terminate recursion-limit false and omitted dynamic bodies also cannot close negative inventory."
- **remaining** (the recorded open obligation): "Actual original compatibility controls and unentered/decoded unsupported geometry remain unvalidated; superseded deadcode cleanup and broad gates pending."

## 1. Admissible patch (A + B + dead variant + R)

### A — `realm.rs` `CommandBindingRealm::executed_script_for_word`

The lookup tries the exact key first. On a miss it retries with the shared lexer widening `tcl_lexer::word_span_at`.

**Cause.** A segmented argv token keeps the lexer's inner-end extent, so a quoted word's span stops before its closing `"` (#527). The interpreter keys the entered carrier by the `WordExpr` span instead (`command_binding.rs:11817`). For quoted and compound words that is the whole written word (`word_expr.rs:321/340`). Braced and bare words have equal spans under both conventions.

**Callers — correction to v1 (A-1).** v1 NOTES said the realm-level method had only two callers: the survey and the `registry_invocation` tests. That was wrong. It also has production analyser callers, which this patch changes:
- `analyser/handlers.rs:2257`, the proc-body param-trait scan (`head_identities`). Quoted and `${v}` proc bodies now obtain an `executed_source`.
- `analyser/commands.rs:446-449`, `analyse_list_quoted_body`. A `TokenType::Cmd` `body_tok` (inner-end `[…`) now hits the carrier recorded under the bracketed component span. List-built bodies such as `if 1 [list …]` therefore gain an `evaluated_body_invocation`.

Both are behaviour changes nobody has validated yet. Add these to the run:
- the analyser `param_traits` tests;
- the list-quoted-body tests;
- `registry_invocation.rs:12042/12083`.

**Lowering inconsistency (A-2, follow-up, not fixed).** Lowering still calls `SourceCommandBindings::executed_script_for_word` with raw token spans:
- `lowering/mod.rs:4274`, `5222`, `6281`
- `lowering/structured.rs:1525`, `1592`

So for quoted, `[…]` and `${}` body words the analyser now sees a carrier that lowering does not.

The better owner-level fix is to normalise once, in `origin_inventory.rs` `record_executed_word` or `executed_script_for_element`. That would keep the compiler test that compares against the realm working. It was left out to keep the admissible scope minimal.

### B — `reachability.rs` `Condition::holds`

A `SourceTarget` requirement that `tcl_dialect::validate_requirement_for(req, target)` rejects returns `Ternary::Inert`, which means Conditional:
- examples: `{{8.6}}` or `"8.6"` on 8.6 and 9.0; `9-` on 8.4;
- real Tcl raises in each case;
- without this, `satisfies_ternary` would read the malformed requirement as No and give a false Available.

**Follow-up (B-1, not fixed).** Tcl 8.4's `package vsatisfies` takes exactly one requirement. `vsatisfies [package provide Tcl] 8.4 8.5` raises `wrong # args` under 8.4.20, but the survey still evaluates it. It should become Inert when `!target.has_package_requirements() && requirements().len() != 1`, ideally derived from the registry's `VSATISFIES_FORMS` / `Arity::exact(2)`.

### Dead code

- Deleted `Condition::TclSatisfies`. It was never constructed outside the cfg(test) `Guard::condition`, and `Walker::guard` already maps `Guard::TclSatisfies` to `Unknown`. The test arm now yields `Undecidable`.
- The patch-level W123 rationale moved to the `SourceTarget` arm.
- **Still present, blocked on a user exception:** the test-only lexical probes:
  - `original_data_word`, `script_of`, `command_source`, `bracketed_body`, `parse_vsatisfies`, `is_tcl_version_subject`, `is_requirement_word`;
  - the cfg(test) `Guard::TclSatisfies` and its imports.
  - Their only callers are assertions in `reachability/tests.rs` 489-527 and 567-587.

### R — `commands/tcl/package_.rs`

| Subcommand | Declared `world_effects` | Basis |
|---|---|---|
| `provide` | `PACKAGE_REGISTRATION_EFFECTS` (PackageState ReadWrite, callback NONE, same as `ifneeded`) | `tclPkg.c` 8.4 to 9.1: `PKG_PROVIDE` is a hash lookup or `Tcl_PkgProvideEx`, with no `Tcl_NRAddCallback` |
| `vsatisfies` | `EMPTY` | `tclPkg.c`: `PKG_VSATISFIES` only parses and compares |

Luna checked these against the C sources for 8.4, 8.5, 8.6, 9.0 and 9.1, with no drift. Jim's `provide` is unchanged and stays conservative.

### Expected flips: admissible patch alone (not executed)

| Selector | Expectation |
|---|---|
| original test-174 `configured_body_values_and_command_children_keep_distinct_original_grammar` | pass (A). Medium-high: the decoded child survey's control depends on the derived-origin advice tape. |
| additive21 test-046 `decoded_registration_retains_native_value_original_words` | pass (A), medium-high |
| compiler `retained_body_preserves_actual_script_mapping_and_input`, `retained_control_keeps_actual_body_and_parent_model_lineage` | pass (A), medium |
| additive21 test-047, original test-178, compiler `logical_target_guard_uses_actual_queries_without_native_string_protocol` | **still fail.** They need C-v2: the `if` condition marker still taints the guard's children. |
| additive21 test-045 | **still fails.** It needs D-v2. |

Re-run because the result may change:
- analyser `param_traits` and list-quoted body tests (A);
- optimiser, GVN and effect-SSA tests involving `package provide` / `vsatisfies` (R);
- `trusted_package_transfer.rs:259` (R);
- `workspace_index.rs:11643` `a_conditional_provide_does_not_make_a_document_the_provider` (R).

## 2. CD-v2 patch (separate part; on top of the admissible patch)

### C-v2 — ordered conditions (`body_effects.rs` `expression_commands`, and `clause_grammar.rs`)

**C-2 fixed.** The ordering predicate is now registry data: a new `ClauseGrammarSpec::evaluates_conditions_once_before_bodies()`. It requires `FirstMatch` *and* head, rows and tail all `ClauseTiming::Selected`. Only `if` qualifies today:
- `try` has a Protected head;
- loops are `All` with `PerIteration` / `LoopFixture` rows;
- a SpecTcl spec that defaults to `FirstMatch` but has loop-timed rows is excluded.

**C-3 fixed.** The expression is also re-parsed with the shared `tcl_syntax::expr::OriginalSourceExpression::capture`. If it fails to parse, or it contains *any* math-function call, it is treated as unordered and the parent marker is kept. This is stricter than Luna's "non-registry-closed" suggestion: a builtin name can still be a redefined `tcl::mathfunc` proc, which tclsh confirms.

**What changes.** Only when both predicates hold, the parent uncertainty marker is recorded for variable reads alone, not for command substitutions. Those children are then threaded: each starts from the graph after the earlier siblings' possible effects were merged. Every other case keeps the exact old path (a single pre-loop clone, plus the marker).

**C-1 closed together with D-v2.** Commands created inside a condition substitution become possible occupants (below). They are not dropped any more.

### D-v2 — possible occupants (replaces v1 D, which set new cells to `Shadowed`)

**What the patch adds:**
- A new `AdviceGraph::possible_occupants: BTreeSet<ByteCommandSlot>`.
- In `retain_possible_effects`: a key present in `branch.cells` but not in `original.cells` is added to the set, and the branch's own set is merged in. Such a key is **never** inserted as a cell.
- A new helper, `falls_through_possible_occupant`. It is true when the coordinate `lookup_head_key` selected was reached past a candidate in that set. This applies to Logical candidates and to the procedure-local coordinate. The C-source path is unchanged.
- When the helper is true:
  - `blocks_registry_source` returns true. That bars registry advice and the body/prefix catalogue receipts, and `readonly_logical_source_candidate` refuses because it checks this.
  - `resolve` returns None.

**Unchanged, which addresses D-1:** `lookup_head_key`, the `cells` map, `logical_coordinates` and `external_header_candidates_at_selected`. So procs defined inside `namespace eval`, `if` or `catch` bodies keep their navigation, definition and workspace header candidates. They are never a cell; at most they block a fall-through.

If the set is empty, the helper returns immediately. It is empty on the nested-wrapper path (180 s budget), which makes no branch merge.

### Tests added (static; tclsh oracle results in `probe2.tcl`)

| Test | Location | Covers |
|---|---|---|
| `only_selected_first_match_grammars_evaluate_conditions_once_before_bodies` | `tcl-registry/src/clause_grammar.rs` | C-2: `if` is true; `while`, `for`, `foreach`, `try` and `catch` are false; a synthetic FirstMatch grammar with a PerIteration row is false. |
| `selected_condition_children_leave_no_parent_debt_unless_unwalked_phases_remain` | `body_effects.rs` | C-3 and C-2: `if {[package vsatisfies 8.6 8.5]} {set y 1}` gives the body no `UnknownEarlierMutation`. The same condition with `&& abs(1)`, with `&& $x`, or as a `while` keeps it. |
| `definitions_from_entered_conditions_and_bodies_keep_namespace_shadows` | `reachability.rs` | C-1: Luna's counterexample `if {[namespace eval ::ns2 {proc package args {}}; list 1]} {namespace eval ::ns2 {package ifneeded viacond 1.0 {}}}` visits nothing (tclsh 8.6 and 9.0 register nothing). Control: an unrelated `::ns::other` leaves `apply … ::ns` registering `kept` as Conditional (tclsh registers `kept 1.0`). |
| `possible_branch_definitions_keep_external_header_candidates` | `logical_navigation.rs` | D-1: `namespace eval A {proc helper {} {}}; namespace eval A {proc caller {} {helper ONE}}` keeps two coordinates and two external headers. The v1 Shadowed version would have returned None here. |
| existing `logical_lambda_namespace_is_its_own_original_source_owner` (T045) | — | D-v2. Oracle: explicit `::ns` lambda registers nothing; default lambda registers. |

`probe2.tcl` also shows that a `tcl::mathfunc::abs` override which defines `::ns4::package` suppresses the `viamath` registration. That is why C-3 keeps the marker for any function call.

### Expected flips: admissible + CD-v2 (not executed)

| Selector | Expectation |
|---|---|
| additive21 test-047 `fixed_source_guard_retains_real_query_and_advertisement_dependencies` | pass (C+R+B), medium |
| original test-178 `the_body_descent_leaves_top_level_declarations_alone` | pass (C+R+B), medium |
| compiler `logical_target_guard_uses_actual_queries_without_native_string_protocol` | pass (C+R), medium |
| additive21 test-045 `logical_lambda_namespace_is_its_own_original_source_owner` | pass (D-v2), medium-high |
| the four new tests above | pass, medium |
| outside the capture selection: `a_vsatisfies_early_return_gates_the_declaration_1017`, `the_tcllib_head_guard_admits_both_supported_releases`, `any_registry_terminator_ends_the_index`, `an_elseif_chain_selects_one_arm_per_release`, `the_real_tcllib_try_index_resolves_per_release`, the written-head version tests | expected pass |

Medium means not every gate of `original_package_index_model_at` for commands inside an expression was traced to completion.

Run these before admitting CD-v2:
- the full original and additive Core selections: Logical advice after `if {[cmd]}` now carries less uncertainty;
- `workspace_index` provider/bystander tests (`:11474-11700`);
- Logical navigation, definition and references tests;
- `tcl-compiler` `logical_navigation`, `source_relation_provider`, `project_procedure_roles` and `body_effects` tests;
- `reachability/tests.rs:187` `an_unmodelled_guard_leaves_the_declaration_conditional`.

## Budget

The 180 s nested-wrapper path is untouched:
- **A** adds at most one O(1) widening on a miss.
- **C** needs an `Expr` role, which `ifneeded` does not have.
- **D-v2**'s helper returns immediately when the set is empty, and the nested `[list …]` substitutions make no branch merge.
- **R** does not apply to `list`.

## Closure claim

**Not fully closed.**

If both patches land and the expected flips are confirmed by execution, they address:
- the five recorded reachability failures;
- the decoded-geometry part of the open obligation.

Still owed:
- running the original compatibility controls and the compiler owner controls;
- your exception for the test-only helper assertions;
- the follow-ups: A-2 (normalise the carrier key at the owner) and B-1 (8.4 `vsatisfies` arity);
- broad gates: `make rust-check` (clippy was not run), `make prep-pr`, the full selections and the 180 s budget run.

Unentered non-braced decoded source still abstains by design. No catalogue status change is justified yet.
