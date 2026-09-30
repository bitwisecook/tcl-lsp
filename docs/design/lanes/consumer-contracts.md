# Lane: consumer contracts — steps 1–6 landed, step 7 in progress; the plan for steps 2–10

## Goal

Step 1 of [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
§ *Build order*, documents only: take the four rulings (and the fifth,
narrower one on the pack-authored `state_transitions` resolver) as decided,
and repair every document whose stated rule the rulings replace. No `rust/`,
`runtime/`, `editors/`, or `art/` file is touched by this lane.

## Decisions taken

- § *Rulings still open* becomes § *Rulings*. Each subsection keeps its
  statement, its rationale, and its consequences, and gains one **Decided.**
  sentence naming the build-order steps that land it. The four
  "If the owner decides the other way" paragraphs and the fifth ruling's
  "*The other way*" clause are dropped: a decided ruling has no other way,
  and leaving them would make the page argue against its own status box.
- The fifth ruling's lead-in read "emitting alias facts only" while its own
  contract sentence allows `VariableCellAliasTransition` **and**
  `NamespaceTransition` facts. The two families are the reading carried
  through, in the page and in `spec-packs.md`.
- Every repaired rule is stated as the design has it, in the page's own
  voice, with one present-tense "today" sentence wherever the tree still
  does the thing the ruling replaces. Each such sentence was checked against
  the tree with grep before it was written; no sentence claims behaviour
  that does not exist.

## Document inventory

| Document | Section | Status |
|---|---|---|
| `compiler/registry-consumer-contracts.md` | § Rulings, § The two hook bodies that remain, § Codegen…, § C Tcl extensions, § Build order, § Related docs | done |
| `compiler/command-registry.md` | § Authoring a spec without Rust | done |
| `contracts/dialect-stubs.md` | § Flags, § Stubs are declarations | done |
| `registry/spec-packs.md` | § Workspace trust…, § What a pack still cannot say | done |
| `runtime/c-extension-shim.md` | § The implemented subset, § Out of scope | done |
| `runtime/c-extension-abi.md` | § 7 Header scope | done |
| `runtime/tclvm-opcode-status.md` | note 5, `startCommand` | done |
| `runtime/rename-alias.md` | § 3.5 Invalidation | done |
| `compiler/aot-command-priority.md` | § 5 | done |
| `docs/GLOSSARY.md` | C extension shim | done |
| `design/README.md`, `design/compiler/README.md` | the index entries | done |
| `docs/kcs/` | — | checked, no note states a replaced rule |

## Open uncertainties

- `docs/kcs/kcs-qa-what-is-the-c-extension-shim.md` and the glossary entry
  describe compiling against `tclshim.h`. That is what the tree does today,
  so both keep it; the glossary entry names the authored header as the
  contract beside it. The KCS note is left alone rather than made to promise
  a header the tree does not contain.
- `aot-command-priority.md` § 5 groups `load`/`unload` in one cell. The
  WASM runtime registers `load` on `unsupported_cmd`
  (`runtime/rust/src/cmd_misc.rs`) and does not register `unload` at all;
  the repair changes only the "fixable?" answer and leaves the grouping.

## Step 2 — progress

Items in the order they land (CC2.6 promoted to follow CC2.1, because the
value-transfers lane's VT5.8 builds on it). The sonnet-class items (CC2.7,
CC2.10, CC2.13, CC2.15) are dispatched separately after the opus items.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC2.1 the per-axis lint and its ledger | landed | `wip(consumer-contracts): step 2 — the registry-axes gate and its baseline` | 7831 vocabulary words; 1089 sites pinned across 163 files; `CLEAN_FILES` empty; D2.12, D2.13 |
| CC2.6 `OptionEffect` in the registry, and the four clients | landed | `wip(consumer-contracts): step 2 — option effects` | `option_effect.rs`; `OptionSpec::effect` on 1373 literals; `subst`, `lsearch`, `regexp`, `switch` declare effects and families; `substitution_resolver`, `subst_substitutions`, `lsearch_pattern_args` and the five `CaseListSpec` switch fields gone; D2.14–D2.21 |
| CC2.2 `ClauseGrammarSpec` in the registry | landed | `wip(consumer-contracts): step 2 — clause grammars as data` | `clause_grammar.rs` (the page's types, the walk, `clause_keywords`, `owner_of_keyword`); ten grammars (`if`, `try`, `catch`, `for`, `while`, `foreach`, `lmap`, `dict for`/`dict map` (one), `dict update`, `array for`); `if_arg_roles`, `check_if_shape`, `walk_if`, `try_arg_roles`, `foreach_arg_roles`, `lmap_arg_roles` gone; the two clause-keyword tables derived; E004 through `clause_shape_defect`; D2.22–D2.30 |
| CC2.3 `clause_grammar` in the loader, renderer and studio | landed | same checkpoint as CC2.2 | loader builds the registry type (`ClauseGrammar`/`ClauseWalk` gone; subcommand grammars; derivations recorded, no placeholders); studio schema/draft/coverage/help/examples/render (Rust and `SpecTcl`)/store and a read-only form view; `if.tclspec` gains timings and its default clause, `foreach.tclspec` its grammar; 24 goldens and the callback inventory regenerated |
| CC2.4 `MemberEffect` in the registry | landed | `wip(consumer-contracts): step 2 — member effects and the studio round trip` | the page's types verbatim plus `WrapperShift` (D2.31); `MemberSpec::effect` required on every constructor; every TclOO, snit, itcl, `SpecTcl` and `SslicTcl` member states one; `DefinitionBodyGrammar::member_row` (D2.32), `MemberArity::parse`, `MemberEffect::natural_receiver`, the `.tclspec` spellings; sweep `every_member_carries_an_effect_that_agrees_with_its_roles` (D2.33) and `no_member_effect_names_a_family`; eleven `definer.rs` unit rows |
| CC2.5 `definition_body` and `semantic_operation` leave `GAPS` | landed | same checkpoint as CC2.4 | loader `-effect` / `-shift` on `member`, `member_option` rows, `family SpecTcl\|SslicTcl` (D2.35); studio seeds a shipped grammar by name — by data, not pointer (D2.34) — or the whole block, `semantic_operation` as `{kind, detail}`; both renderers write both; the two `GAPS` rows deleted; `DefinitionBody` / `SemanticOperation` field kinds, catalogues, help, examples, form (D2.36); `snit-type.tclspec` gains `-effect` on every row; the `oo-class` / `snit-type` golden hashes regenerated |
| CC2.8 the derived-query layer | landed | `wip(consumer-contracts): step 2 — the derived-query layer` | `CommandRegistry::invocation(words, ctx)`; `ResolvedInvocation` carries its `SurfaceQuery` and selection; `arg_roles`, `pattern_args`, `case_invocation`, `frame_effect`, `return_type`, `effects` added, `clause_plan` / `option_effects` / `substitutions_performed` lose the `dialect` parameter; one rule per query shared with the by-name functions (`arg_roles_in`, `command_prefixes_in`, `pattern_args_in`, `layout_is_proven_in`); `derived_queries_agree_with_the_by_name_answers`; D2.37–D2.43 |
| CC2.14 the `state_transitions` resolver family in the loader | landed | `wip(consumer-contracts): step 2 — the state-transition resolver family` | `HookFamily::StateTransitionResolver` (thirteenth family: `alias LOCAL TARGET ?-level LEVEL?`, `namespace-variable NAME`, silence "no transitions", field `state_transitions.resolver`); `PackTransition`, the thunk and `STATE_TRANSITION_RESOLVER_NATIVE`; `alias_pairs_resolver` for `from-frame-effect`; the loader reads every `state_transitions` row; 21 corpus notices and the two port goldens move; D2.44–D2.49 |
| CC2.9 clause consumers in the compiler | landed | `wip(consumer-contracts): step 2 — lowering and the analyser read the clause plan` | `lower_if` / `lower_try` from `ResolvedInvocation::clause_walk` (values; the inert reading on abstention); `TryHandler::kind: HandlerMatch` (IR, inlining, `executable_ir.rs`, `cfg_lower.rs`'s `on ok` through the completion-code parse, the diagram's wire spelling); `handle_try_command` walks the plan; `handle_for_command` gone — the generic body walk reads timings; `owner_of_keyword` for the stray-keyword report; `signature_scan/walker.rs` reads clause bodies; the registry's `try_control_invocation` parses the plan; `a_try_handler_walk_reads_timing_not_keywords` (and its negative); four files in `CLEAN_FILES`, `structured.rs` 19 → 8 and `handlers.rs` 20 → 9 pinned; the deprecated `effect_footprint` alias removed; D2.50–D2.62 |
| CC2.11 member consumers | landed | `wip(consumer-contracts): step 2 — members by effect` | every member statement read through its `MemberRow`: one `match` on the effect (`member_landing`) routes `apply_oo_subcommand_in` and the snit and itcl walkers — `apply_oo_private`, `apply_oo_self`, the sided-effects helper, the `filter` keyword test, `apply_oo_ctor_or_dtor`, the snit `type` prefix and the itcl keyword maps gone; `CallableRole::Procedure` for snit's `proc` (D2.63); `MethodKind::from_effect` in `ir.rs`, `from_str_lossy` and `member_method_kind` gone, the lowering's frame read off the row; the providers read the recorded member (`MethodDef::is_declared_by_keyword`, `kind`); the per-item path's `PackDefiner` fallback (D2.70); `a_pack_declared_member_spelling_reaches_every_provider` and its negative; `oo.rs` 43 → 5, `lowering/mod.rs` 25 → 18, `ir.rs` 5 → 2, `hover.rs` 6 → 4, `references.rs` 6 → 1, `definition.rs` and `workspace_index.rs` to `CLEAN_FILES`; D2.63–D2.71 |
| CC2.12 transitions, roles, special variables and the package layout | landed | `wip(consumer-contracts): step 2 — transitions, roles and special variables` | `apply_state_transitions` applies every `VariableCellAliasTransition` from the dispatch tail, over the call's source words (`resolve_invocation_words_in_context`, `SourceCall`); `VariableCellAliasTransition::words: AliasWords` (D2.72); the `global`, `variable`, `upvar`, `namespace upvar`, `dict for`, `dict update`, `incr` and `append` / `lappend` handlers gone — one binder reads `LoopVarList` / `VarWrite` roles (D2.74), `foreach` binds through it; `package require` / `provide` / `ifneeded` read roles (`package_require_arg_roles`, `-exact` exact-spelt) and `-exact` from the option run (D2.76); `interp create` reads `InterpreterTransition::Create`, direct and nested, with the resolver on the C option scan (D2.77); `special_var` pack statement, `CommandRegistry::special_vars()` door, the two `auto_path` arms one registry read (D2.78–D2.80); witness `alias_and_binding_resolvers_abstain_on_a_dynamic_word`; `a_pack_declared_scope_alias_binds_its_local` and `a_pack_declared_special_variable_is_readable_at_startup`; the six `until step 2` waivers gone (`handlers.rs` stays at 9 pinned); `package require`'s dynamic role pinned in the callback baseline; D2.72–D2.82 |
| — the registry-axes ledger after the rebase | landed | `wip(consumer-contracts): step 2 — the registry-axes ledger after the rebase` (`132fd5d3`) | mechanical: the rebase onto the value-transfers lane's slice 5 moved `analyser/commands.rs`'s four slice-8 waivers down thirteen lines, and `docs/generated/registry-axes.md` was regenerated; no pin or waiver changed |
| CC2.7 `option -effect` and `option_effect_family` in the loader, renderer and studio | landed | `wip(consumer-contracts): step 2 — option effects in the loader, renderer and studio` | loader: `option` rows read `-effect {disables\|selects AXIS VALUE}`, `{suppresses-role ROLE}`, `{reserves-trailing-words N}` or bare `ends-options`, and `-family NAME`; command/subcommand-scope `option_effect_family NAME { base all-on\|all-off\|{only AXIS VALUE} combine accumulate\|last-wins ?-introduced V? }`; new spelling helpers `EffectAxis::axis_word`/`value_word`/`from_words`, `SubstitutionKind`/`CaseMatchMode::spelling`/`from_spelling`, `OptionEffectKind`/`FamilyBase::kind_spelling`, `FamilyCombine::spelling`/`from_spelling` (`option_effect.rs`), reusing `PatternType::as_str`/new `from_str_tag` for `pattern-language`'s values rather than a second vocabulary; `-introduced V` resolves through `available::from_texts` (the shared availability algebra), never a hand-rolled window; `checked_option_effect_families` drops an `-effect` whose `-family` names no row this command/subcommand declares, with a notice, at both scopes; studio: `OptionSpec::effect` is `Surface::Key("effect")` (`coverage.rs`'s `OPTION_EFFECT_PENDING` gone), seeded as a flat tagged JSON object (`draft.rs`'s `option_effect`/`insert_axis`) and written back by `render_spectcl.rs`'s `option_effect_block`; `option_effect_families` (already a schema/draft `RustExpr` field since CC2.6) gains its generator (`option_effect_families_expr`) and reverse parser (`option_effect_family_rows`), its `GAPS` row gone; `render_rs.rs` gains the `.rs`-export mirror (`option_value_expr` split out to stay under the line budget, `option_effect_expr`); the option-row web form gains a kind/axis/value/role/n control cluster and a `family` text control (`optionEffectEditor`, `editors.ts`), `option_effect_edits_are_marked_non_structural` and `switching_the_effect_kind_still_rebuilds`; `switch.tclspec` gains its three families and six options' effects (its `spectcl_ports.rs` `__unrenderable` entry now `NONE`); a new `subst.tclspec`, the twelfth port — the design page's own `command subst { … }` example, with the one adaptation its own surrounding prose calls for (the illustrated `option_conflict {A} {B}` does not parse — a relation row takes one term-list word — so the built form is three `option_forbids` rows, matching `subst_.rs`'s own `RELATIONS`) — is the round-trip fixture for `the_subst_port_answers_the_same_kinds_as_the_shipped_spec` over `option_effect.rs`'s own corpus; `an_effect_naming_an_undeclared_family_is_a_notice` (negative, `eval_loader.rs`); every "eleven ports" count in prose and the `the_eleven_port_fixtures_…` test renamed to twelve; 25 pack goldens regenerated (`subst`, `switch`); no `D2.NN` — the plan's own CC2.7 paragraph cites none |
| CC2.10 clause consumers in the editor and tool tiers | landed | `wip(consumer-contracts): step 2 — clause consumers in the editor and tool tiers` | `if_to_switch.rs`, `refactor/datagroup.rs` and `tcl-mcp/datagroup.rs` read `resolve_call`/`clause_plan`/`CaseListSpec` (already landed going into this item); this item's own six files — `analyser/recovery.rs` (`recover_missing_open_brace`'s switch-option scan reads `OptionEffectKind::EndsOptions` and `CaseListSpec::value_options_require_regex`/`fallthrough_body` off `switch`'s own spec instead of hardcoding `--`/`-matchvar`/`-indexvar`/`-`; `looks_like_switch_case` takes the fallthrough marker as a parameter), `formatting/keywords.rs` and `minify.rs` (`subcommand_abbreviation`'s hand-kept `string`/`info`/`clock` abbreviation tables replaced by a genuine `CommandSpec::subcommand_table`/`minimal_unique_prefix` read, generalising to every ensemble; `minify_case_list`'s clause-flag scan reads `CaseListSpec::clause_end_options_flag` instead of hardcoding `--`), `semantic_tokens.rs` (`definer_class_name_idx`'s `oo::class`/`configurable`/`abstract`/`singleton` vs `oo::define` dispatch reads whether the command's own registered subcommands include `create`, fixing a latent gap where `oo::objdefine` was never recognised; the inline `oo::define`/`objdefine` wrapper check reads `MemberKind::Wrapper` instead of the literal `"self"`; snit's bare-construction check reads its metaclass's own registered manufacturer keyword instead of hardcoding `"create"`) — go from a temporarily-zeroed ratchet pin to `CLEAN_FILES` (now 16 files); 33 sites resolved, 25 by a genuine registry-driven fix or dispatch-guard restructuring and the rest by an `irreducible` waiver (36 irreducible waivers total in the ledger, up from 1) for a coincidental vocabulary collision (`^`/`$`/`\|` ARE metacharacters and `::tcl::mathop` operators, same pattern as `eq`/`ne`; `"body"`/`"args"`/`"procs"`/`"variables"` spelling coincidences) or a genuinely irreducible fact this item's own files document in place (the switch-only brace-recovery dispatch guard; `OptionEffectKind::EndsOptions` populated on only 2 of 23 commands' `"--"` rows, so a per-command/per-scope read would regress the other ~21 — the same gap in `formatting/keywords.rs`'s pre-existing `scan_options`; `oo::object`'s `destroy` being the one exported instance method by the registry's own doc comment, with no trait yet distinguishing instance- from class-level subcommands; `self`/`this` as one unmarked entry of a broader `implicit_vars` list; `METHOD_BODY_HELPER_SUB_KEYWORDS`'s pre-9.0 no-`CommandSpec` gap); `docs/generated/registry-axes.md` regenerated (7831 words, 16 clean files, 40 waived sites, 896 pinned across 147 files); `gen-tmlanguage-keywords --check` green with no diff. Tests: `if_to_switch_reads_the_clause_plan` (`refactor/if_to_switch.rs`) — a `CommandRegistry::insert`-built pack overlay redeclaring `if`'s own grammar (deliberately not the shipped shape: no `then` noise, no `else` tail) still converts; negative: an `if` overlay with `clause_grammar: None` does not. `cargo test -p tcl-lsp-core` (2346), `-p tcl-mcp` (108) and `-p tcl-compiler --lib` (6500) green; `cargo clippy -p tcl-lsp-core -p tcl-mcp -p tcl-compiler --all-targets --no-deps -- -D warnings` and `cargo fmt` clean. Deviations: the item's spec names "six files" for the registry-axes gate and a `formatting/`-directory entry without listing which files in it; the tree's actual ratchet debt attributed to this item's files, as pinned when this item started, was ten files (the three already landed plus these seven) — proceeded against the tree the gate actually enforces rather than the prose count, since a stale count is what "adapt and say so" is for. The item's file list also flags `recovery.rs`/`formatting/` for "the `then` distinction read from the slot's `noise` through `clause_noise_keywords()` where a literal `"then"` remains" — greeping `"then"` in both found none in `recovery.rs` and exactly one in `formatting/engine.rs`, inside `#[test] fn then_in_the_else_body_slot_is_not_a_keyword` as deliberate fixture data (the production code the test exercises, `arg_indices_for_role`, already reads keyword *position* from the registry, not a literal-"then" scan); already satisfied by earlier work, so no change was made for it. No `D2.NN` — the plan's own CC2.10 paragraph cites none |
| CC2.13 hook retirement and the re-baseline | landed | `wip(consumer-contracts): step 2 — hook retirement and the re-baseline` | Retired eleven `AnalyserHookId` variants whose handler's only command-specific knowledge was a position or a keyword a descriptor now states — `Try`, `For`, `DictFor`, `DictUpdate`, `Incr`, `Append`, `Lappend`, `Upvar`, `NamespaceUpvar`, `Global`, `Variable` — deleting the variants and their doc comments from `hooks.rs` (43 → 32 variants) and the `analyser_hook: Some(…)` field from the nine spec files that carried the twelve — actually thirteen — stamp rows (`dict for`/`dict update` each carry two: the subcommand and its `::tcl::dict::*` qualified spelling): `variable_.rs`, `global_.rs`, `for_.rs`, `try_.rs`, `upvar_.rs`, `append_.rs`, `lappend_.rs`, `incr_.rs`, `namespace_.rs` (`upvar` subcommand), `dict.rs` (`for`/`update` subcommands, whose qualified spellings inherit the field's new `None` verbatim, the same way they already inherit everything else — `qualified_specs_carry_the_subcommand_analysis_contract` updated to assert the absence). Ten of the eleven were already a `=> false` no-op arm in `dispatch_analyser_hook` (CC2.9/CC2.12); only `Try` still called a real handler. Retiring it deleted `handle_try_command` and the now-unreachable `analyse_selected_body`, `resolved_analyser_hook_plan` and `ResolvedAnalyserHook::clause_plan`; `try` now falls through to the same generic tail `for` already used — `body_depths` (not `handle_try_command`'s own `conditional_depth`-only counting) gives every `Selected` handler body both `conditional_depth` and `control_flow_body_depth`, a behavioural delta (below). The handler's `{resultVar optionsVar}` variable list does *not* carry through `handle_var_binding_command`'s flat role table — `clause_grammar.rs`'s own module doc: a repeating clause's `LoopVarList` is "bound per clause …, which the flat `LoopVarList` role … cannot say" — so `dispatch_body_arguments` gained a small generic step reading `ClausePlan::clauses` directly for every command reaching the tail, carrying the fact exactly rather than dropping it (verified: `handle_try_walks_on_handler_body`/`_trap_handler_body`, retargeted through `Analyser::analyse`, still see `result`/`options` bound). Three more consumers read a hook this item retired and were moved onto a fact that survives it: `signature_scan/walker.rs`'s `try` clause-body walk and its `AnalyserHookId::Try` disjunction now key on `LoweringHookId::Try` (already there, alongside `if`'s own `LoweringHookId::If` guard, now one combined arm), and its `lappend auto_path` detection reads `spec.var_elements_effect: Some(VarElementsEffect::AppendsListElements { .. })` instead of `AnalyserHookId::Lappend`; `references.rs`'s scope-alias detection reads `sub.creates_scope_alias` instead of `AnalyserHookId::NamespaceUpvar`. `analyser_hook_stamps_match_the_former_guard_list` re-baselined from 43 variants / 56 rows to 32 / 43 (`analyser_hook_stamps_are_disjoint_from_definer_families` and `_agree_across_dialect_twins` untouched); `resolve_analyser_hook_uses_registry_rooted_resolution`'s `dict for` example swapped for `dict with` (still stamped); `analyser_hook_selection_requires_binding_proof` (invariant I4) swapped its `try` example — no longer stamped at all — for `oo::define`, the same 8.6+ gate. The `.tclspec` DSL surface loses the eleven names too: `tcl-spectcl`'s `loader.rs` `ANALYSER_HOOKS` table (what `analyser_hook -native ID` matches against) and `catalogue.rs`'s parallel `ANALYSER_HOOKS`/`covered_analyser` (the studio's own listing and exhaustiveness witness); the one shipped pack declaring a retired hook, `docs/design/spec-dsl-examples/upvar.tclspec`'s stale `analyser_hook -native Upvar` (a leftover the field's own removal from `upvar_.rs` orphaned), is deleted, its golden re-recorded (`cargo xtask pack-goldens`, one snapshot). Docs: `docs/references/command-spec/fields.md` regenerated (`UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`), losing the eleven rows from the analyser-hook field's value table; `docs/generated/registry-axes.md` regenerated (line-number-only shift, same 16 clean files / 40 waivers / 896 pins). Tests: `cargo test -p tcl-registry --test analyser_hooks` (4, the re-baselined pinned set); `-p tcl-compiler --lib` (6497 — three `handle_try_command`-only tests dropped as meaningless post-retirement, the clause-timing structural test dropped as redundant with `clause_grammar.rs`'s own `try_grammar_agrees_with_the_retired_walk` and neighbours, four retargeted through `Analyser::analyse`); `-p tcl-lsp-core --lib` (2346), `-p tcl-mcp` (108), the rest of `-p tcl-registry` (924 lib + every integration binary), `-p tcl-spectcl` (188 lib + every integration binary, golden included) and `-p tcl-spec-studio` (198 lib + every integration binary, reference doc included) all green; `cargo clippy -p tcl-registry -p tcl-compiler -p tcl-spectcl -p tcl-spec-studio -p tcl-lsp-core -p tcl-mcp --all-targets --no-deps -- -D warnings` and `cargo fmt` clean. Deviations: none from the item's own spec — the "carry both or record the delta" question the plan's CC2.9/CC2.12 notes raised for `try`'s var-list resolved as "carry" (a small generic addition, not a registry change) after reading `clause_grammar.rs`'s module doc directly; the control-flow-depth widening is the one delta the notes anticipated, recorded below. The plan's own prose says "the twelve stamps"; the pinned test's own count is thirteen rows (`dict for`/`dict update` each two) for the same eleven variants — trusted the passing test over the prose count. Noted, not fixed (out of this item's scope): `cargo test -p tcl-lsp-server`'s `rename_safety::fp_namespace_variable_rename_refuses_beside_a_computed_alias_cell` fails both before and after this item (isolated by `git stash`-ing every CC2.13 change and re-running the one test against the unmodified CC2.10 landing, `74d46ccc`, where it fails identically) — a pre-existing gap unrelated to the hook retirement, most likely in how `namespace upvar`'s `otherVar` argument (the aliased cell's own name, as opposed to `myVar`, its declared `VarWrite`) reads a `VarRead`-shaped role `rename_safety.rs`'s hazard scan can see. No `D2.NN` — the plan's own CC2.13 paragraph cites none |
| CC2.15 the lint's roots shared, the ledger's expiries closed, and the step's documents | landed | `wip(consumer-contracts): step 2 — the lint's roots shared, the ledger's expiries closed, and the step's documents` | Rebased `cc-step2` onto the main branch's current head (`claude/spectcl-optimization-discussion-5qhf42` at `330b14bf`) immediately before this item, per the coordinator's mid-task instruction — the head sits on top of the value-transfers lane's slice 5 landing (and further, still-in-progress slice 8 checkpoints); CC2.7/CC2.10/CC2.13 replayed with no conflict, `cargo check --workspace` clean on the result. `rust/xtask/src/util.rs` gains `pub const ANALYSIS_TIER_ROOTS`, the ten analysis-tier crate roots `registry_axes.rs` and `value_transfers.rs` each declared as their own byte-identical `LINT_ROOTS`; both gates now read the one shared list — the merge the item's own text ties to the value-transfers lane's slice 2 commit, boundary B1, confirmed already on `main` well before slice 5. `registry_axes.rs`'s `LANDED` gains `"step 2"`; no production waiver anywhere in the tree named `until step 2` already (checked against the generated ledger and a source grep — `docs/generated/registry-axes.md` has zero "step 2" hits — so nothing needed retiring there), but three of the gate's own unit-test fixtures used `until step 2` as an example of a not-yet-landed expiry (`a_waiver_without_an_expiry_fails`'s `parse_waiver` assertions, the three-placement `line_comments`/`site_waiver` fixture, `an_enclosing_match_carries_its_arms_waiver`) and would have started asserting the opposite of what they test once `step 2` itself became landed; moved all three to `until step 3`. Three new KCS notes: `docs/kcs/compiler/kcs-qa-where-does-a-clause-shape-come-from.md` and `kcs-qa-what-does-a-member-effect-say.md` (Contributor Q&A — the grammar, the derived-query plan, and `clause_shape_check`/nothing as the escape hatch; the closed member-effect vocabulary and the three derivations that fold over `member_rows` generically — each linking out to the design doc rather than restating its type signatures, per STYLE.md rule 8) and `kcs-howto-declare-an-option-effect-in-a-tclspec-pack.md` (User How-To — kind/axis/family/`-effect` in plain terms plus the design doc's own `subst` family example verbatim), each indexed in `docs/kcs/README.md` / `docs/kcs/compiler/README.md`; `cargo xtask kcs-index-links` green. `docs/GLOSSARY.md`'s three existing *Clause grammar* / *Option effect* / *Member effect* entries — added by the earlier opus items while the three descriptors were still forthcoming — still read "proposed" / "Proposed as"; reworded to the built, landed state (`ClauseGrammarSpec` in `tcl_registry::clause_grammar`, `OptionEffect` in `tcl_registry::option_effect`, `MemberEffect` in `tcl_registry::definer`) and cross-linked to the three new notes. `docs/design/README.md` and `docs/design/compiler/README.md`'s `registry-consumer-contracts.md` index lines dropped the blanket **proposal** label and instead mark the description contract's three descriptors **built** in step 2, keeping **proposal** for the identity and backing contracts, which step 2 does not touch — the page itself still covers all three, so the whole line was reworded rather than only the word dropped. Gates: `kcs-index-links`; `cargo xtask registry-axes --check` (OK: 7831 vocabulary words, 16 clean files, 40 waived, 896 pinned across 147 files — unchanged by this item); `cargo xtask value-transfers --check` (OK: 20 clean files, 19 waived, 90 pinned across 36 files, 6607 inventory rows — unchanged); `cargo test -p xtask registry_axes` (9) and `value_transfers` (14) green; `cargo fmt --all --check` and `make rust-check` (fmt, `cargo clippy --workspace --all-targets -- -D warnings`, and every `xtask-check` gate, `kcs-index-links` and both drift gates among them) green — the one interim failure (`LANDED`'s array literal over rustfmt's line-length limit) fixed by wrapping it onto its own line, as rustfmt's own diff asked. Deviations: the item's own text lists no explicit lane-doc or step-landing requirement — this row, the step-2 landing note below, the title-line update, and the `docs/design/lanes/README.md` in-flight line are the standing lane-tracking-protocol obligation the orchestrating brief adds on top of every checkpoint in this lane, the same as CC2.7/CC2.10/CC2.13's own rows. No `D2.NN` — the plan's own CC2.15 paragraph cites none |
| Step 2 review fixes | landed | `wip(consumer-contracts): step 2 — review fixes` | The reviewer's "land with fixes", applied on `cc-step3` (branched from the step 2 merge, `b39e012b`). **B1** (blocking): CC2.12's `Analyser::alias_cell` abstained (`literal()?`) on a computed `namespace upvar` namespace or `otherVar` word, so the local got no `VarDef::link_target`, `variable_alias_links` no cell, the index no `WorkspaceVariableAlias`, and `documents_with_ambiguous_alias_of` no refusal — `rename_safety::fp_namespace_variable_rename_refuses_beside_a_computed_alias_cell` failed at the head ("the refusal must say why, got null"; CC2.13's row misread the cause as a missing role). The `Target::Namespace` arm now spells a computed half from its source word as the `CallerSelectedFrame` arm does (D2.83): `::$ns::v`, `::mypkg::$v`, a relative namespace against the command-resolution namespace, `fixed_frame: false`, the span still the `otherVar` word; the doc comment says why the marker is kept. The pinning test is inverted and renamed `namespace_upvar_with_a_computed_word_keeps_the_cell_as_written` (the computed local still binds nothing); the three `rename_safety::` tests pass (the `version` tail refuses, the `counter` tail does not, the literal alias in another namespace is edited). **S1**: the design page's analyser table (32 variants, the residue named, the clause row's consumers as built, "three descriptors were missing"), its CC2.9 as-built paragraph (CC2.10 and CC2.13 folded in), the two CC2.7 sentences and the file-path anchors flipped to the built state; `command-registry.md` § *Compound commands and subcommand dispatch* reads the descriptors first, and § *How registry feeds the compiler* gains CC2.13's residue paragraph (the 32 variants by class, the three on the ledger); `AGENTS.md`'s compound-command sentence likewise. **S2**: `for_start_runs_at_the_enclosing_depth_and_next_and_body_per_iteration` (D2.55 through `Analyser::analyse`, by `package_requires`' `conditional` / `control_flow`) and `a_rename_in_a_try_handler_is_not_a_straight_line_deletion` (`on` and `trap` record no deletion, `finally` and a top-level `rename` do); the stale `handle_try_command` / `analyse_selected_body` comments beside them corrected. **S3**: `incr $name` is pinned in `a_written_array_element_binds_its_array_and_a_computed_name_binds_nothing` beside the `append $name x` it already held; `var_binding_a_nested_loop_command_binds_its_loop_variables_in_the_enclosing_scope` (`diagnostics/tests.rs`: `[lmap …]`, `[foreach …]`, `[dict for …]` inside `set`); `tcloo_self_constructor_and_destructor_lift_no_unit` (`lowering/mod.rs`, with the instance spellings as the control; tclsh 8.6, 9.0 and 9.1 re-checked: `invalid command name "constructor"`). **S4**: `a_state_transition_resolver_is_reported_as_a_hook` (`tcl-mcp/src/spectcl.rs`) pins the wire spelling, verbs and silence; the delta is listed below. **S5**: § *Exit evidence* reads CC2.13's 32 variants on 43 rows; CC2.11's `lowering/mod.rs` is 25 → 18 (the ledger and `RATCHET`; the row's 26 → 19 predates the rebase onto slice 5, which retired one site). **S6**: the studio schema's `pattern_arg_resolver` hint is `Some(my_pattern_resolver)` (D2.86 — no live resolver exists to name). **S7**: `ResolvedClause::operands(role)` beside `operand` (now its first), `every_operand_of_a_role_is_named_in_word_order` over `catch`'s grammar; the generic tail binds every `LoopVarList` operand a clause fills; `a_clause_with_two_variable_lists_binds_both` (`tests/analyser.rs`, a `.tclspec` with `head {Body} -timing protected` and `tail {{LoopVarList optional} {LoopVarList optional}}`, no `arg` rows: `cmd {} a b` binds both, `cmd {} a` only `a`). **Notes**: the clause step's comment now says it fires beside the binder for `dict for` / `dict map` / `array for` and why that is idempotent (D2.84); the binder's opt-out documents why `SubCommand::creates_scope_alias` is not folded into the invocation's traits (D2.85); the dispatch comments no longer list `try` among the early-return hooks; this table gains `132fd5d3`'s row. Tests: `cargo test -p tcl-compiler --lib` (6506, 2 ignored), `--test analyser` (504), `-p tcl-registry --lib` (925), `-p tcl-mcp` (109), `-p tcl-lsp-core --lib` (2350), `-p tcl-spec-studio --lib` (198), `-p tcl-lsp-server --test e2e` (1599, 5 ignored — the whole binary, since `link_target` feeds every navigation provider); `cargo clippy -p tcl-compiler -p tcl-registry -p tcl-mcp -p tcl-spec-studio --all-targets --no-deps -- -D warnings` and `cargo fmt` clean; `registry-axes --check` OK (7831 words, 16 clean files, 40 waived, 896 pinned across 147 files — the ledger regenerated for a line shift only), `value-transfers --check` OK (20 / 19 / 90 across 36 / 6607), `kcs-index-links` green, `dialect-drift` at its 8 upstream sites. D2.83–D2.86 |

**Step 2 is landed.** All fifteen items above (CC2.1 through CC2.15 — the eleven opus items and the four sonnet-class items, CC2.7, CC2.10, CC2.13, CC2.15, dispatched separately as the table's lead-in notes) are `landed`. The ordering and checkpoints table's final gate, `make rust-check`, is green on the rebased tree at this commit. The title above and `docs/design/lanes/README.md`'s in-flight line are updated in this commit to read step 2 landed; step 3 (trust gates execution; stub flags reach their fields) has not started. The review's fixes landed after the merge, as the table's last row; step 3 is under way in § *Step 3 — progress*.

### Behavioural deltas accepted in step 2

- CC2.6: `subst -nocommands -variables …` at 9.1 draws W147 (the plan's
  delta); two negated switches with a positive one draw two (D2.17).
- CC2.6: `regexp -inline … v` draws W147 with tclsh's message; the roles were
  already upstream's #2222.
- CC2.6: `subst -novar x` answers `{backslashes, commands}` — Tcl resolves the
  unique prefix (tclsh 8.4–9.1 print `$x` for `subst -novar {$x}`) — where the
  retired resolver answered every kind; `subst -no x` (ambiguous) still
  answers every kind.
- CC2.6: a profile-bound registry below 9.1 answers every kind for
  `subst -variables x`: the positive switch is not an option there (tclsh
  9.0's `bad option "-variables"`), where the retired resolver ignored the
  release.
- CC2.6: an unknown switch before `regexp -inline` stops the scan, so the
  invalid call `regexp -bogus -inline a b v` keeps `VarWrite` on its
  trailing words (the retired name scan skipped the unknown word).
- CC2.2: a malformed `try` is walked by its grammar, which stops at the
  first defect: `try b foo on e m h` has no roles past `foo` (the retired
  scan skipped the unknown word and kept looking for `on`), and an
  incomplete handler (`try b on x y`, `try b finally`) highlights its
  keyword (the retired scan required the whole clause). Every well-formed
  `try` answers as before (`try_grammar_agrees_with_the_retired_walk`).
- CC2.2: the callback inventory's `dynamic-arg-role` rows for `if`, `try`,
  `foreach` and `lmap` now name a clause grammar as their source.
- CC2.5: a pack's `member` row without an `-effect` (or with one the
  vocabulary cannot read) is dropped with a notice; the one checked-in pack
  with member rows, `snit-type.tclspec`, gained its effects. `member_option`
  rows load (they were dropped with a notice), and `family SpecTcl` /
  `family SslicTcl` load (they fell back to `TclOo` with a notice).
- CC2.5: the studio's round-trip gap register lost `definition_body` and
  `semantic_operation`; every command of every dialect still round-trips,
  now with the `SpecTcl` and `SslicTcl` document grammars carried row by row.
  The `oo-class` and `snit-type` pack goldens' spec hashes moved (the
  `MemberSpec` literal gained two fields); no EDA pack declares a grammar.

- CC2.14: a pack's `state_transitions` block loads every row
  (`argument_shape`, `resolver`, `widen`, `covers`, `commit` beside
  `composition`), where each was dropped with "not yet loadable"; a value a
  row cannot read is dropped with its own notice. The `oo-class` port's
  three `resolver -native` ids now read `SCOPE::state_transitions.resolver`
  and name nothing this build ships (three notices); the `upvar` port's
  `from-frame-effect` derives. The corpus baseline loses 21 notices and
  gains those three; the `oo-class` and `upvar` goldens move.

- CC2.9: `lower_if` follows the grammar where the retired keyword walk
  did not. `if 0 {a} {b}` — a bare final body, valid Tcl (tclsh 8.6 runs
  `b`) — lowers to `Statement::If` with that `else` body, where the walk
  deferred it as "extra words"; a condition spelt like a keyword
  (`if else {a}`, `if elseif c {a}`, `if 0 {a} elseif else {b}`) is lowered
  as the expression it is (tclsh: `invalid bareword "else"`), where the
  walk skipped the keyword and mis-lowered `if elseif c {a}` as `if c {a}`
  and the third as an `else`; `if 1 {a} $w` lowers its bare final body's
  barrier as "if with non-literal body" rather than "if with extra words".
- CC2.9: `lower_try` defers `try … finally {…} on …` (tclsh: "finally
  clause must be last"), which the walk lowered with both clauses; a chain
  whose clause word is computed defers before its protected body is
  lowered (the walk lowered the body first, then deferred).
- CC2.9: `cfg_lower.rs`'s `on ok` edge reads the registry's completion-code
  parse, so `on 0` (and `+0`, `0x0`) is `on ok` too.
- CC2.9: `for` bodies take the generic body walk: an unbraced one draws W105
  as `while`'s does, and a bareword body (`for {} {$i<3} {incr i} step`) is
  processed as a call, where `handle_for_command` walked each word as a
  script. `start` still runs at the enclosing depth; `next` and the body
  at a control-flow depth, as before.
- CC2.9: the analyser's `try` walk and the signature scan stop at a chain's
  first defect (`try {a} bogus on error {} {b}` no longer walks `{b}`), and
  neither walks a braced `{-}` marker as a script.
- CC2.9: `[catch …]` nested in a substitution binds its result words when
  its head resolves to the `Catch` hook (a `::catch` spelling too), and
  `[list namespace unknown H]` is recognised through `BUILDS_COMMAND_PREFIX`
  and the `NamespaceUnknown` hook.

- CC2.11: a wrapped `TclOO` member lands by its row where the old arms
  ignored every wrapped spelling but `method` / `classmethod` and the sided
  words: `private variable x` declares an instance variable (tclsh 9.0.4:
  the class's methods read it back), `private forward f cmd` a private
  forward, `private superclass` / `private mixin` fold into the class's
  slots, `self private method m` (and `private self method m`) a private
  class method, `private constructor` the constructor. A class-object
  `forward`, `variable`, `superclass` or `mixin` stays unrecorded.
- CC2.11: `self constructor` / `self destructor` (tclsh 8.6.18 and 9.0.4:
  `invalid command name "constructor"`) no longer lift a lowering unit or
  collect a class-side body; a `foreach` installer's `self method $m` records
  class methods, where it recorded instance methods.
- CC2.11: a computed word where `TclOO`'s optional method word may stand
  (`method m $opt {} {…}`) makes the statement unreadable: the analyser
  records no member (it read the fixed layout) and the lowering marks the
  class unanalysed (it lifted the method under the fixed layout).
- CC2.11: an itcl `protected` member's `MethodDef::visibility` is
  `unexported`, the registry's spelling of the tier (it was `protected`);
  every consumer compares with `public` / `private`, so no answer moves.
  The IR's `MethodKind` for a snit or itcl `proc` body is `ClassMethod`
  (was `Method`); its one reader is `TclOO`'s `self class` fold.
- CC2.11: a document that invokes a definer only a workspace pack declares
  takes the per-item path's full-analysis fallback (`PackDefiner`), so the
  class it defines reaches the editor (the per-item shell reads the
  un-overlaid store, where the definer has no grammar); every fresh
  full-analysis fallback now carries the pack overlay, which it dropped.
- CC2.12: a scope alias binds through the registry's alias facts. `global
  {$x}` (braced) binds the local `$x` (it was skipped: the handler tested
  the brace-stripped text for `$`); a `namespace upvar` whose namespace or
  `otherVar` word is computed defines its local and links it to the cell as
  written, marker and all (`::$ns::v`), as the retired handler did — the
  workspace index reads that marker to refuse a rename beside an alias whose
  cell it cannot name (CC2.12 dropped the link; the step 2 review fixes
  restored it, D2.83); a `{*}` word
  in an `upvar`, `namespace upvar` or `variable` call leaves its layout
  unknown, so the call binds nothing (`upvar {*}$lvl a b` bound `b`); a
  pack command whose `state_transitions` resolver states alias facts binds
  as `upvar` does.
- CC2.12: one binder reads the roles. `lmap`, `dict map` and `array for`
  loop variables bind (nothing bound them); a nested `[lmap …]`, `[foreach
  …]` or `[dict for …]` binds its loop variables in the enclosing scope (the
  substitution path ran only the `VarWrite` binder); `foreach $names …` and
  `dict for $kv …` no longer define `names` / `kv` (a computed var list
  names nothing); a written array element binds its array for every
  `VarWrite` command (`lassign $l a(1)`, `regexp … m(1)`), as `incr` /
  `append` / `lappend` did, and a braced written name with a space binds
  (`lassign $l {a b}`); a computed written name (`incr $name`, `append
  $name x`) no longer defines `name`; `dict update d k {a b} body` binds `a`
  and `b` — the role is a list, where Tcl binds one variable `a b`, the
  reading the SSA harvester and `script_binds` already give it; `incr`'s
  `VarDef::warn_if_unused` is `false` (D2.75); a head the walk's context
  does not provide binds nothing (invariant I4).
- CC2.12: `package require -exact` with no package records nothing (tclsh
  8.4–9.0: `wrong # args`), where it recorded a package named `-exact`;
  `package require -e 1.0` records nothing — `-e` is a package name to Tcl,
  but the registry's option scan reads a dash word naming no option as a
  call Tcl rejects (D2.76).
- CC2.12: `interp create` follows the C option scan. `interp create -s x`
  records a safe `x` (it was unsafe); `interp create -bogus x`, `x --` and
  `-` create nothing (the scan recorded `x`, `x`, nothing); `set i [interp
  create a b]` and `set i [interp create n -bogus]` bind `i` to nothing
  (they bound an auto-named interpreter and `n`); `interp create $opt
  child` makes interpreter existence unknowable (it recorded nothing);
  `interp create {$x}` records an interpreter named `$x` (a braced literal,
  which the parse treated as dynamic).
- CC2.12: `set auto_path …` / `lappend auto_path …` record the search path
  only where the dialect provides `auto_path` writable, so iRules records
  nothing and no longer flips `has_dynamic_providers`; a nested `[lappend
  auto_path DIR]` records its directory (the substitution path never did).
- CC2.10: the minifier's iRules ensemble-subcommand abbreviation now reads
  `info`'s whole registered subcommand table rather than a three-command
  hand-kept copy, so `info exists $x` shortens to `info ex $x`, not the old
  table's `info e $x` — `info errorstack` (Tcl 8.6+) also starts with `e`,
  so the old table's `e` was an unproved, silently ambiguous abbreviation
  the new read no longer offers; `string`/`clock` and every other
  fixed-ensemble command abbreviate exactly as before (nothing else in the
  old table collided).
- CC2.13: retiring `handle_try_command` moves every `on` / `trap` handler
  body onto `body_depths`'s `Selected` reading, which raises both
  `conditional_depth` and `control_flow_body_depth` — the retired handler
  raised only the first. A `rename` inside a `try` handler is no longer read
  as a straight-line deletion (the same non-deletion treatment a `rename`
  inside any loop body already had); a `package require` inside a handler
  was already `conditional` before and after (unaffected — the pinned
  `package_require_conditionality_per_try_clause_kind` covers it), and now
  also carries `control_flow`, which nothing reads yet at that call site.
- CC2.14: `spectcl_check` reports a pack's `state_transitions` resolver
  body as a hook of the family `state_transitions.resolver`, with its two
  verbs and "no transitions" as its silence
  (`a_state_transition_resolver_is_reported_as_a_hook`); it landed by
  construction with the family (`family_key`) and had no pin until the
  review fixes.
- Step 2 review fixes: a clause whose grammar gives it two variable-list
  slots binds both (`ResolvedClause::operands`, D2.84), where the generic
  tail bound the first; a pack grammar in `catch`'s shape with no `arg`
  rows is the case that moves — no shipped command does, since `catch`'s
  own hook binds its result words and every other shipped clause has one
  such slot.

### CC2.12 — what the next items read

- **The alias consumer.** `Analyser::apply_state_transitions(&mut self,
  inv: &ResolvedInvocation, args, arg_tokens, scope_path)` applies every
  `VariableCellAliasTransition` the invocation states; the dispatch tail
  calls it for every command whose descriptor declares transitions, over
  `SourceCall` — the call's head and word facts (`source_invocation_word`:
  `{*}` expanded, braced or substitution-free literal, else computed) with
  the walk's registry and context, resolved by
  `tcl_registry::model::resolve_invocation_words_in_context` (I4).
  `dispatch_command_handlers` and `dispatch_analyser_hook` take
  `WordFacts { single, expanded }`.
- **The binder.** `handle_var_binding_command(cmd_name, args, arg_tokens,
  scope_path)` binds every `LoopVarList` (a static list, each name at its
  element's span) and `VarWrite` (`names_static_variable`) position the
  roles name, over the words' spellings (`read_spelled_invocation`, the
  literal view, D2.74). It also records `lappend auto_path DIR…` from the
  descriptor's `VarElementsEffect::AppendsListElements`.
- **The hook arms CC2.13 retires.** `Hook::For`, `Global`, `Variable`,
  `Upvar`, `NamespaceUpvar`, `Incr`, `Append`, `Lappend`, `DictFor` and
  `DictUpdate` are one `=> false` arm in `dispatch_analyser_hook`: no
  handler runs, so the variants carry only their stamps; `Try` keeps
  `handle_try_command` (the CC2.9 note stands). `Set` keeps
  `handle_set_command` and the `set auto_path` record
  (`record_search_path_write`), `Foreach` the literal-iteration simulation,
  `InterpCreate` `handle_interp_create_command(&StateTransitions)`, the
  three `Package*` arms their handlers — each now reading roles.
- **The registry.** `VariableCellAliasTransition::words: AliasWords {
  local, target }` (`AliasWords::same(i)` for a one-word alias);
  `CommandRegistry::{insert_special_var, special_vars, special_var,
  special_var_in_dialect, special_vars_for_dialect,
  is_readable_at_startup}`; the free `special_vars::*` functions read the
  shipped table only (D2.78). `package require` has
  `package_require_arg_roles` (`Name` then `Value`s after the option run)
  and `PrefixMatching::Strict`, as has `present`; `provide` and `ifneeded`
  declare `(0 Name) (1 Value)`. `interp create`'s resolver reads the C
  option scan (`create_reads_the_c_option_scan`).
- **Not yet in the editor's per-item path.** `per_item_setup` reads the
  un-overlaid store by design (D2.70), and the tail's `SourceCall` resolves
  against the walk's store, so a pack command's alias facts bind in a full
  analysis (`Analyser::analyse`, the CLI, the MCP server, the per-item
  path's `PackDefiner` fallback) but not in the server's memoised per-item
  walk; carrying the overlay into the per-body memo key is the question
  D2.70 left open.
- **The value-transfers lane (B1).** W210's startup read
  (`diagnostics/dataflow.rs`'s `startup_read_facts`, `helpers.rs`'s
  `StartupFacts::for_name`), the `[info exists]` fold (`sccp.rs`) and the
  taint seed (`taint.rs`) read the free functions; switching each to the
  registry door (`registry.is_readable_at_startup(name, dialect)`,
  `registry.special_vars_for_dialect(dialect)`) makes a pack's startup
  binding silence W210 there — one call per site, handed to that lane.

### CC2.11 — what the next items read

- **The routing.** `analyser/oo.rs`'s `MemberStatement::read(grammar,
  texts, argv, dialect)` reads one member statement through
  `DefinitionBodyGrammar::member_row` over structured words (a braced or
  substitution-free word is its literal, anything else computed — D2.64);
  `member_landing(grammar, member, row) -> MemberLanding` is the one
  `match` on `MemberEffect`, and `apply_oo_subcommand_in`, the snit walker
  (`dispatch_snit_member`) and the itcl walker all read it. Members are
  still recorded under their written spelling (`${m}` stays in the
  outline).
- **The IR.** `MethodKind::from_effect(role: CallableRole, receiver:
  MemberReceiver) -> Option<MethodKind>`; the lowering's `MemberFrame::of_row`
  adds the definition-time script rule (D2.65). `MethodDef::kind` (the
  analyser's) is `MethodKind::as_str()` or `forward`.
- **The vocabulary.** `CallableRole::Procedure` (`-role procedure`) for a
  procedure in the definition's own namespace; snit's `proc` states it,
  itcl's `proc` keeps `-role method` on the type object (D2.63).
- **The providers.** `MethodDef::is_declared_by_keyword(word)` answers
  whether a nameless member was declared by the keyword under the cursor;
  hover, definition, references and document symbols read `kind` from the
  member. `workspace_index.rs` spells the class side
  `MethodKind::ClassMethod.as_str()` and the visibilities through
  `DeclaredMemberVisibility`.
- **The ledger.** `CLEAN_FILES` gains `tcl-lsp-core`'s `definition.rs` and
  `workspace_index.rs`. Pinned: `analyser/oo.rs` 5 (the `property` flag
  words, three, until the 9.0 `property` accessor rows; snit's type-body
  implicit `type`, one; the unknown-proc walk's `default` arm, one — D2.68),
  `ir.rs` 2 (`SwitchMode`), `lowering/mod.rs` 18 (none on the member axis),
  `hover.rs` 4 (regex tokens), `references.rs` 1 (the `switch` marker).
  The value-transfer ledger loses `analyser/oo.rs`'s row and
  `lowering/mod.rs` drops to 1 (D2.71).

### CC2.9 — what the next items read

- **The plan's walks.** `ResolvedInvocation::clause_walk() ->
  Option<Result<ClausePlan, ClauseAbstention>>` walks the words' *values*
  (`InvocationWord::Literal`), so the fall-through marker is compared
  exactly; `Err` names the first computed word the walk compared and carries
  the *inert* plan (every computed word matching nothing) — never the call's
  plan, only how far its literal clauses go (D2.52). `clause_plan()` is
  `clause_walk()?.ok()`. The spelling walks (`ClauseGrammarSpec::walk`,
  `CommandSpec::clause_plan`, `CommandRegistry::clause_plan`, the new
  `ResolvedCall::clause_plan(args, dialect)`) keep the one-layer strip for
  callers holding source spellings (D2.50).
- **Reading a clause.** `ResolvedClause::operand(role)` (the first operand
  of a slot of `role`) and `ResolvedClause::handler()` (the pattern operand
  and its `HandlerMatch`); `ClausePlan::falls_through(index)` — a clause
  whose body word is the marker, target or not. A marker falls through to
  the next `Selected` clause only, never to `finally` (D2.51).
- **The compiler.** `TryHandler::kind: HandlerMatch` (re-exported as
  `tcl_compiler::ir::HandlerMatch`); `lower_if` / `lower_try` in
  `lowering/structured.rs` build from `clause_walk_of(seg)`; the dispatcher
  resolves in `structured_dispatch` (D2.60). The editor refactors
  (CC2.10) can ask either walk; `if_to_switch`'s shape is `lower_if`'s.
- **The analyser.** `ResolvedAnalyserHook::clause_plan` (walked at the
  document's authoring point) is what `handle_try_command` takes;
  `Analyser::clause_plan_in_context` answers the same plan for a head the
  generic body walk resolves, and `body_depths` maps each body's timing to
  its two depths (D2.55). `Hook::For => false` routes `for` to that walk: the
  `For` variant has no handler left for CC2.13 to delete, only its stamp.
  `Try` keeps `handle_try_command` — retiring it (CC2.13) moves `try` onto
  the generic walk, where a handler body is `Selected` (conditional *and*
  control-flow) while the handler walked it conditional only, and the
  var-list slot binds nothing: CC2.13 must carry both or record the delta.
- **The ledger.** `CLEAN_FILES`: `analyser/commands.rs`,
  `cfg_builder/cfg_lower.rs`, `executable_ir.rs`, `signature_scan/walker.rs`.
  Pinned: `lowering/structured.rs` 8 (the `switch` option parser, five; the
  `dict` subcommand routing, three), `analyser/handlers.rs` 9 (the ensemble
  configuration walk and its `dict merge` splice, eight; `$handle eval`,
  one), `tcl-diagram`'s `data.rs` 4 (D2.59). Waived until step 2, for
  CC2.12: `handlers.rs`'s `interp create` flag scan and nested
  `[interp create …]` (`InterpreterTransition::Create`), `package require
  -exact`, and the two `auto_path` arms. Waived until slice 8: the four
  `set VAR [CLASS new]` sites in `commands.rs` (VT8.9). Irreducible: W218's
  variadic `args`.

### CC2.6 — what the next items and the value-transfers lane read

- **The answer.** `tcl_registry::option_effect::OptionEffects { axes:
  Vec<(EffectAxis, bool)>, shifts: Vec<(ArgRole, i8)>, complete: bool,
  option_end: usize }`, with `value(axis)`, `substitution_kinds()`,
  `pattern_language()`, `selection()`, `suppresses(role)` and
  `reserved_trailing_words()`.
- **The walk.** `option_effect::option_effects(spec_options: &[OptionSpec],
  families: &[OptionEffectFamily], args: InvocationArguments<'_>,
  reserved_trailing_words: usize, dialect: Option<SurfaceQuery<'_>>) ->
  OptionEffects` (the plan's signature; unique prefixes resolve),
  `option_effects_with(…, prefix_matching)` for an exact-only table, and
  `option_effect::substitution_kinds(…same five…) -> SubstitutionKinds` (the
  projection with the operand-reach rule, D2.18).
- **Per command.** `CommandSpec::option_effects(args, dialect)`,
  `CommandSpec::substitutions_performed(args, dialect) ->
  Option<SubstitutionKinds>`, `CommandSpec::option_selects_pattern_language()`,
  `CommandSpec::option_selecting(axis) -> Option<&'static str>`.
- **Per resolution.** `ResolvedInvocation::option_effects()` and
  `ResolvedInvocation::substitutions_performed()`, over the selected
  table (`InvocationSemantics::option_scope`); `option_end` is post-head.
  CC2.8 dropped the `dialect` parameter: the resolution carries its query.
- **The registry projection.** `CommandRegistry::substitutions_performed(name:
  &str, args: &[&str]) -> Option<SubstitutionKinds>`, signature unchanged,
  under the registry's own profile.
- **`subst`'s declaration** (`commands/tcl/subst_.rs`): six rows with
  `effect`, families `negated {AllOn, Accumulate}` and `positive {AllOff,
  Accumulate, surface: TCL91}`, `reserved_trailing_words: 1`, and three
  `Forbids` relations (D2.17). VT5.8 adds only the `semantics` field; the
  `tp_*` / `fp_*` rows of the retired `substitution.rs` tests are in
  `option_effect.rs` under the same prefixes (D2.21).
- **State CC2.7 starts from.** The loader reads no `option -effect`,
  `-family` or `option_effect_family` yet; the studio draft carries
  `option_effect_families` as a presence-only `RustExpr` with a transient
  `DraftOpaque` `GAPS` row (`render_spectcl.rs`), and an option row's
  `effect` is `Surface::Excluded(OPTION_EFFECT_PENDING)` in `coverage.rs`
  and not drafted; `spectcl_ports.rs` documents `switch`'s
  `__unrenderable` difference. CC2.7 turns both into data, lands the loader
  spelling and the renderer, deletes the transient row, the pending reason
  and the port's `unequal` entry, and adds the `subst` port. The five retired
  `case_list` rows already load with a notice.


### CC2.2 / CC2.3 — what the next items read

- **The answer.** `tcl_registry::ClausePlan { clauses: Vec<ResolvedClause>,
  roles: Vec<(usize, ArgRole)>, defect: Option<ClauseShapeError> }`; a
  `ResolvedClause` carries `keyword_index`, `row: ClauseRowId`,
  `operands: Vec<(usize, ClauseSlot)>`, `timing`, `falls_through_to` (an
  index into `clauses`) and `is_default`. `roles` is the flat projection
  (D2.22) — a clause's binder list, a handler's pattern and a fall-through
  body are read from `clauses`.
- **The walks.** `ClauseGrammarSpec::walk(args, layouts)`, `walk_at(…,
  dialect)`, `walk_words(args, dynamic, layouts, dialect) -> Option<_>`;
  `CommandSpec::clause_plan(args, dialect)` /
  `SubCommand::clause_plan(args_after_sub, dialect)`;
  `CommandRegistry::clause_plan(name, args)` (post-head coordinates,
  subcommand resolved); `ResolvedInvocation::clause_plan()` (post-head
  coordinates, under the resolution's query since CC2.8; `None` on
  expansion or a computed word where a keyword could stand). `CommandSpec::clause_shape_defect(args, dialect)` /
  `CommandRegistry::clause_shape_defect(name, args)` is E004's source.
- **Vocabulary.** `ClauseTiming::spelling`/`from_spelling`,
  `ClauseSelection::spelling`/`from_spelling`,
  `clause_grammar::handler_spelling`/`handler_from_spelling`,
  `ClauseGrammarSpec::keywords`/`noise_words`/`all_rows`/
  `layout_depends_on_words`/`is_fallthrough_body`/`may_assign`;
  `clause_grammar::clause_keywords(reg)`, `owner_of_keyword(word)` (CC2.9's
  `orphaned_keyword_parent`), and `traits::clause_keywords_without_command_spec()`
  / `clause_noise_keywords()` (already called by `semantic_tokens.rs`,
  `minify.rs` and `gen_tmlanguage_keywords.rs`; CC2.10's remaining work there
  is the `then` literals and the refactors).
- **Still reading keywords by spelling** (CC2.9/CC2.10's; since CC2.9 only
  the editor refactors and `tcl-mcp`'s `datagroup.rs` remain): `lower_if`,
  `lower_try`, `handle_try_command`, `orphaned_keyword_parent`,
  `cfg_lower.rs`'s `on ok`, `signature_scan/walker.rs`, the editor refactors,
  `tcl-mcp`'s `datagroup.rs`, and the registry's own
  `parse_try_control_invocation` (`registry.rs`), which CC2.9 moves onto the
  plan. `tcl-compiler/src/value_transfer.rs`'s `view_of` (the value-transfers
  lane's) reads `arg_role_resolver` directly, so `if` / `try` / `foreach` /
  `lmap` operands carry only their static roles there until it reads
  `ResolvedInvocation::clause_plan` or CC2.8's `arg_roles`.

### CC2.4 / CC2.5 — what the next items read

- **The descriptor.** `MemberSpec::effect: MemberEffect` (every member),
  `MemberSpec::wrapper_shift: Option<WrapperShift>` (wrappers only: `self`
  → `receiver: TypeObject`; TclOO `private` → `visibility: Private`; itcl
  `public` / `protected` / `private` → `Public` / `Unexported` /
  `Private`). The effects are the plan's list; `proc` (snit, itcl) is a
  type-object `Callable`, `component` / `typecomponent` state per instance /
  per type, `option` `StateDeclaration { scope: Option }`, and every
  `SpecTcl` / `SslicTcl` row `Configuration` (D2.7).
- **The row.** `DefinitionBodyGrammar::member_row(keyword_index, words:
  InvocationArguments<'_>, dialect: Option<SurfaceQuery<'_>>) ->
  Option<MemberRow>` (D2.6, D2.32): `keyword_index` and `body` index `words`
  (the statement's words: 0 in a class body, 1 for `oo::define C method …`);
  a wrapper's prefix form answers the wrapped member's row with the shift
  applied, its block form (`self { … }`, exactly one argument) an
  `InitScript { body_slot: 0, AtDefinition }` row whose receiver and
  visibility the block's own members take — CC2.11 folds a block by
  recursing with them. `receiver` is after every shift
  (`MemberEffect::natural_receiver` before), `visibility` is option word,
  then wrapper, then `member_default_exported` (nameless rows `Public`),
  `arity` is `MemberArity::parse` of a literal list (positional), `slot_op`
  is `SlotSpec::split_call`'s answer (a computed first word abstains).
- **Vocabulary.** `MemberReceiver` / `CallableRole` / `StateScope` /
  `RelationSlot` / `InitTiming` each have `ALL`, `spelling`,
  `from_spelling`; `MemberEffect::KIND_SPELLINGS` / `kind_spelling`;
  `DeclaredMemberVisibility::ALL` / `from_spelling`; the prelude exports the
  member-effect types.
- **Loader and studio.** `tcl_spectcl::SHIPPED_DEFINITION_BODIES` (name ↔
  grammar, the loader's `definition_body NAME` table),
  `tcl_spectcl::semantic_operations()` and `semantic_operation_spelling(op)`;
  `draft::definition_body_block(grammar)` (public) and the draft keys
  `definition_body` (null / name / block) and `semantic_operation` (null /
  `{kind, detail}`).
- **Formerly keyed on member spellings** (CC2.11's, landed):
  `apply_oo_subcommand_in`'s arms, `apply_oo_private` / `apply_oo_self`,
  `MethodKind::from_str_lossy`, the snit / itcl body walkers and the
  `constructor` / `destructor` literals in `tcl-lsp-core` read `member_row`
  now; `extract_property_defs` keeps its flag words (D2.68).

### CC2.8 — what the next items read

- **The resolution.** `CommandRegistry::invocation(words: InvocationWords<'w>,
  ctx: &AnalysisContext) -> StructuredInvocationResolution<'r, 'w>` is
  `resolve_structured_invocation(words, ctx.surface_query())`;
  `AnalysisContext::surface_query()` answers the profile's
  `SurfaceQuery<'static>` (the one addition to `value_transfer/context.rs`,
  B1 — the value-transfers lane is to be told). `ResolvedInvocation::dialect:
  Option<SurfaceQuery<'w>>` is the query it resolved under, and a
  crate-private `selected` holds the spec, the subcommand (or instance
  method) and whether it is an instance call. `resolve_invocation`,
  `resolve_structured_invocation` and the instance pair take
  `SurfaceQuery<'w>` (D2.39).
- **The queries**, all `&self` with no `dialect` parameter:
  `clause_plan() -> Option<ClausePlan>`; `option_effects() -> OptionEffects`;
  `substitutions_performed() -> Option<SubstitutionKinds>`;
  `arg_roles() -> Option<Vec<(usize, ArgRole)>>` — post-head, sorted by
  position and then `ArgRole::ALL` order, `CommandPrefix` included, `None`
  for an expansion, a computed subcommand word, or a computed word where a
  resolver reads an option (D2.37); `pattern_args() -> Vec<PatternArg>`;
  `case_invocation() -> Option<(CaseInvocation, Vec<InlineCaseClause>)>`
  (D2.40); `frame_effect() -> Option<(FrameLevel, Vec<OperandId>)>` with
  post-head operands (D2.41); `return_type() -> Option<TclType>` (D2.41);
  `effects() -> EffectFootprint`, whose former name `effect_footprint` is a
  `#[deprecated]` alias for one checkpoint (D2.42). `member_rows` is
  `DefinitionBodyGrammar::member_row` (D2.6); `template_plan` is slice 5's
  (D2.4).
- **One rule each** (`registry.rs`, crate-private): `arg_roles_in(spec, sub,
  args, wanted, dialect, case_gate, option_patterns)` — the per-role
  `arg_indices_for_role` is its single-role filter — `command_prefixes_in`,
  `pattern_args_in`, `layout_is_proven_in`, `sort_role_table`,
  `sub_options_at`. The by-name methods pass their release-blind
  subcommand; the queries pass the resolution's (D2.38).
- **Nothing consumes the queries yet.** Every compiler, analyser and editor
  call site still asks the by-name functions; CC2.9–CC2.12 move them, and
  the by-name functions go as their last callers do.

### CC2.14 — what the next items read

- **The family.** `HookFamily::StateTransitionResolver` (index 12,
  `HOOK_FAMILIES: [HookFamily; 13]`): verbs `alias LOCAL TARGET ?-level
  LEVEL?` and `namespace-variable NAME`, word indices each; silence "no
  transitions"; not literal-only. `HookAnswer::Transitions(Vec<PackTransition>)`
  and `pack_hooks::state_transition_resolver_fn(slot)`; the thunk reads each
  index against the call's own `InvocationArguments`: a literal alias is a
  `VariableCellAliasTransition` (`CallerSelectedFrame` for `alias`, the
  current namespace's cell under its tail for `namespace-variable`), a
  computed word widens `VARIABLE_ALIAS_DOMAINS` (`VariableCells`,
  `VariableTraces`) instead, an index past the call names nothing (D2.44,
  D2.45).
- **The derivation.** `state_transition::alias_pairs_resolver(level_word)`
  — the `from-frame-effect` resolver for an `AliasPairs` frame effect, with
  the README's two abstentions, each widening (D2.46). The loader installs it
  at the command's seal (`derive_transitions_from_frame_effect`), for the
  command and its subcommands.
- **The loader.** `state_transitions_value` reads `composition`,
  `argument_shape`, `resolver none|from-frame-effect|-native ID|{words ctx}
  {…}`, `widen -operands EveryArgument|{Indices N …}|{Strided F S} -domains
  {…}`, `covers SOURCE -domains {…}`, `commit`; a body is a `HookDecl` with
  field `state_transitions.resolver` and the abstaining placeholder until
  `tcl_spectcl::hooks` binds its slot; a form binds no body (D2.47).
  `StateTransitionDomain::ALL` is the domain vocabulary.
- **What CC2.12 consumes.** A pack command's alias facts arrive through
  `ResolvedInvocation::state_transitions()` exactly as a shipped command's
  do; a dynamic level or name word in a pack resolver states no alias for
  that fact (CC2.12's negative test), where the shipped `upvar` states one
  with an unknown subject.

## Step 3 — progress

Items in the order they land. The opus items run first, CC3.1, CC3.3, CC3.5
— the order the lane's brief sets, where the plan's § *Ordering and
checkpoints* put CC3.5 first; nothing in CC3.5 reads CC3.1's or CC3.3's
types, so the order moves no work — and the sonnet items, CC3.2 and CC3.4,
are dispatched separately after them. CC3.3 lands before CC3.2 for the same
reason: it reads `MergedPack::provenance()`, which CC3.1 adds, and never the
predicate CC3.2 collapses.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC3.1 `WorkspaceTrust` through discovery, provenance and the cache key | landed | `wip(consumer-contracts): step 3 — workspace trust through discovery, provenance and the cache key` | `tcl_dialect::model::WorkspaceTrust { Trusted (default), Untrusted }` beside `Provenance`, with `workspace_provenance()` (D3.1); `DiscoveryOptions::workspace_trust` (default trusted); `Tier::trust_under` — the workspace tier reads the state, every other tier is trusted; the trust rides the load, not `PackFile` (D3.5): `pack::load_under`, `pack::load_in_memory_under`, `bundled::load_discovered_in(store, files, trust)`, with `load`, `load_in_memory`, `load_discovered` and `load_embedded` the trusted doors the CLI and MCP already call, so neither needed an edit; `MergedPack::trust` and `MergedPack::provenance()`; `PackEnvironmentTier::Workspace(WorkspaceTrust)` with `of(tier, trust)` and `label()` (D3.6), so registration (`register_pack_set`, the dialect and roster conversions, `untrusted_compiled_extension`) and `to_definition` carry the pack's own class; `register_pack_environments` / `register_environments` take the trust; `EvalOptions::trust`, `EvalSnapshotKey::trust` and the `tier` doc comment naming the trust state; `untrusted(PackEnvironmentTier)` gates E-R2 on the pair, and the refusal names "untrusted workspace" (D3.8); `cache::{key_for, evaluate_pack_cached, evaluate_pack_including, snapshot_memoised}` take the trust, `entry_key` and the pack-set key mix it (D3.7); the server passes `options.workspace_trust`, still the default until CC3.3 sets it. Tests: `rust/tcl-spectcl/tests/workspace_trust.rs` (new; shard `5 tcl-spectcl::workspace_trust`) — `an_untrusted_workspace_pack_is_workspace_untrusted_provenance` (and the user tier stays `User`), `an_untrusted_workspace_pack_still_declares_its_facts` (arity, `Body` / `Value` roles and a `Declared` semantics reach the installed registry, equal under both states), `an_untrusted_workspace_pack_cannot_override_a_compiled_name` (the negative: `-override lsort` refused naming "untrusted workspace", the shipped `lsort` standing; trusted, it loads), `a_client_that_reports_nothing_is_trusted` (the three defaults, and a discovered `.tcl-lsp/` pack loading as `WorkspaceTrusted` with its override), `the_snapshot_key_distinguishes_trust` (snapshot key, entry key and set key split for the workspace tier, not for bundled); `cache.rs`'s key and identity tests gain the trust rows; `surface_roster_trust.rs` pins both states; `loader.rs`'s environment test pins `to_definition` under both; `tcl-dialect`'s trust-class test pins the default and the map. Gates: `cargo test -p tcl-spectcl` (every binary: lib 188, `workspace_trust` 5, …), `-p tcl-dialect --lib` (153), `-p tcl-spec-studio --lib` (198); `cargo check --workspace --all-targets`; clippy (`-p tcl-dialect -p tcl-spectcl -p tcl-spec-studio -p tcl-lsp-server`) and `cargo fmt` clean; `verify-nextest-binary-shards.py --metadata-only` (327 targets) and `test-nextest-binary-shards.sh` green; `registry-axes` / `value-transfers --check` unchanged; `pack-goldens --check` (25) unchanged; `kcs-index-links` green; `dialect-drift` at its 8. Docs: `spec-packs.md` § *Workspace trust* (the loader maps by the state; the client wire and the hook gate still to come) and the cache bullet; the redesign's O9 narrowed, not closed (D3.9); the design page's status box moves `WorkspaceTrust` to built. Deviations: the brief's order runs CC3.1 before CC3.5 (the plan put CC3.5 first); the trust rides the load (D3.5) and the tier value (D3.6) rather than `PackFile` and a `provenance(trust)` parameter; no `cache::VERSION` exists to bump (D3.7); O9 closes with CC3.3 (D3.9); the MCP and CLI callers needed no edit (D3.5) |
| CC3.3 hook bodies gated; the dormant notice | landed | `wip(consumer-contracts): step 3 — dormant hook bodies` | `HookDecl::line` — the declaring row: the hook property, the `option` row carrying an `-arity-hook`, the `state_transitions` block's `resolver` row (`state_transitions_value` returns `(HookSource, u32)`), the `evaluate` statement (`semantics.rs`'s `Implementation::line`, carried through `declaration()` into `rebind`, so a later `facts` or `option -evaluate` rebind keeps it), the `clause_grammar` row (`CommandAcc::clause_grammar_line`) — D3.11; `hooks::hook_bodies_run(provenance)` (only `WorkspaceUntrusted` is dormant; the studio override runs, D3.12), `hooks::DormantHook { pack, command, field, file, line }` and `hooks::dormant_hooks(pack, commands, provenance)` over the private `bodies()` iterator `programs_of` also reads, so the bodies a trusted workspace runs are the ones an untrusted one reports; `plan_for` gives a dormant pack's bodies no slot (no `PackPrograms` at all) and lists them in `HookPlan::dormant()`, so `specialise` leaves the loader's abstaining placeholder and the host never sees the text; `pack::load_sources` pushes `PackNotice::dormant` for each, `Severity::Information`, context `command NAME`, the plan's message verbatim, at the hook's own line in the command's file; `golden::positionless` zeroes the lines the `hooks` digest reads (D3.13). Server: `Backend::workspace_trust`; `workspace_trust_in` reads a **top-level** `workspaceTrust` (`"trusted"` / `"untrusted"`, anything else no statement) from `initializationOptions` (in `apply_initialization_options`, before `initialized`'s first load) and from a `didChangeConfiguration` push, never from a `tclLsp` section (D3.10); a change reloads through `ReloadTrigger::Trust` and reschedules every open document; `spec_pack_discovery` sets `workspace_trust`. VS Code (`clientCore.ts`, both hosts): `initializationOptions` is a function returning `{ workspaceTrust }` from `workspace.isTrusted`, and `registerWorkspaceTrustGrant` pushes `{ settings: { workspaceTrust } }` on `onDidGrantWorkspaceTrust`, registered before `client.start()` (D3.14); `package.json`'s `untrustedWorkspaces` description and `INSTALL-editors.md` say the bodies stay dormant. Tests: `workspace_trust.rs` — `an_untrusted_pack_installs_no_hook_body_and_reports_each_as_dormant` (a `const_fold`, an `-arity-hook` and an `evaluate -implementation` body beside a `clause_grammar`: three information notices at lines 5, 11 and 21, none for the derivations; `plan_for` empty with the three in `dormant()`; the negative: trusted, no notice, three slots, and with a host on the thread the trusted install folds `abcde` to `5` while the untrusted one still abstains) and `only_an_untrusted_workspace_holds_its_bodies_dormant` (every `Provenance` but `WorkspaceUntrusted` runs; the bundled, user and studio tiers ignore the state through the load); server unit `workspace_trust_comes_from_the_client_and_never_from_a_setting` (absent is trusted; `initializationOptions` sets untrusted; the negative: `{"tclLsp":{"workspaceTrust":"trusted"}}` and the flat-dotted `tclLsp.workspaceTrust` change nothing; the top-level push grants; an unreadable value is no statement); e2e `spec_packs::granting_trust_reloads_and_installs_the_bodies` (an untrusted client: one information notice on the `const_fold` row, no `O129` fold; the top-level grant push clears the notice and the same call site folds to `5`); `hooks.rs`'s `a_declared_body_replaces_the_abstaining_placeholder` unchanged (the preserve). Gates: `cargo test -p tcl-spectcl --no-fail-fast` (lib 188 and all 19 binaries, `workspace_trust` 7, `golden_packs` 3), `-p tcl-lsp-server --lib` (594), `--test e2e spec_packs::` (31) and `config::` (26); `cargo check --workspace`; clippy (`-p tcl-spectcl -p tcl-lsp-server`) and `cargo fmt` clean; `tsc --noEmit -p editors/vscode`, eslint and prettier on the three changed `.ts` files; `pack-goldens` rewrote 8 snapshots (`foreach`, `if`, `oo-class`, `return`, `string`, `switch`, `upf`, `upvar` — the `hooks` digests only) and `--check` passes; `registry-axes` (7831 / 16 / 40 / 896 across 147) and `value-transfers --check` (20 / 19 / 90 across 36 / 6607) unchanged; `callback-inventory --check` unchanged (no new tier); `kcs-index-links` green; `dialect-drift` at its 8. Docs: `spec-packs.md` § *Workspace trust* (the wire, the gate, the notice, what is not a body); the design page's status box and the ruling's consequences (the abstention sits in `plan_for`, the host never learns the state); the redesign's O9 row removed (closed, D3.9), § 6.4 gains the execution bullet and its E-R2 bullet names the untrusted workspace; KCS `kcs-qa-why-is-my-pack-hook-dormant.md` (User, all-editors, a VS Code sub-heading), indexed. Deviations: the wire is top-level, not `tclLsp.workspaceTrust` (D3.10); the grant test is an e2e test, since no unit test drives `reload_spec_packs` — the unit test pins the wire and its forgery negative; `HookPlan::dormant` is `Vec<DormantHook>` (file and line beside the plan's `(pack, command, field)`), so the notice and the plan read one list; `npm test` not run (it downloads and launches a VS Code build) — the type-check, eslint and prettier stand in; `cargo test -p tcl-spec-hooks` not run (the crate depends on none of the changed crates, and `containment_e2e.rs` is untouched); `semantics.rs` (the value-transfers lane's slice-4 file, B1) gains three small hunks, no caller in `loader.rs` changed shape |
| CC3.5 the six `StubFlags` on their catalogue fields; nearest-wins | landed | `wip(consumer-contracts): step 3 — stub flags reach their fields` | `DeclaredCommand::traits: Traits` and `side_effects: Vec<SideEffect>`, `new` unchanged, builders `with_traits` / `with_side_effects` (D3.4); `StubCommandDef::to_declared_command` carries the flags through `declared_traits` (`-barrier` → `CREATES_DYNAMIC_BARRIER`, `-loop` → `HAS_LOOP_BODY`, `-pure` → `PURE`, `-unsafe` → `UNSAFE | SAFE_INTERP_HIDDEN`, `-scope_alias` → `CREATES_SCOPE_ALIAS`, `-mutator` → `READS_BEFORE_WRITE`) and `declared_side_effects` (`-mutator` → a `Variable` read and write, D3.15), and its "has never had a consumer" doc sentence goes; `DocumentCommandSurface` answers nearest-wins: `arg_indices_for_role` and `command_prefixes` read the declaration alone for a declared name (the union is gone), and the new `traits`, `invocation_traits` and `side_effects` (`Option<Cow<[SideEffect]>>`, D3.16) answer the declaration's facts under the security floor — a redeclared shipped command keeps `SecurityFloor::security_traits` and its effects beneath them (D3.17). Consumers moved onto the surface: `bounds_checks::loop_shape` / `loop_termination_diagnostics` take `Option<&DocumentCommandSurface>` (the analyser passes `command_surface`), a declared name's loop-ness is `HAS_LOOP_BODY` off `traits` and its shape its `Expr` / `Body` roles (D3.21); `unit_scope::note_surface_var_writes` reads `CREATES_SCOPE_ALIAS` off `surface.traits`; `lower_default`'s read-before-write asks `surface.invocation_traits`; `side_effects::classify_side_effects_in(surface, …)` with `classify_declared` (the catalogue's order; a declaration stating nothing is `fallback_unknown_write`), which the interprocedural call scan uses, and a declared name is not an unknown call there (D3.18); the analyser's `safe_interp_visibility_gate` reads `SAFE_INTERP_HIDDEN` off `command_surface(registry).traits`; the minifier's `find_rename_barriers` builds the surface from `analysis.stub_commands` and reads its command-level traits nearest-wins (D3.23). Tests: `stub_arg_roles.rs` gains `a_pure_stub_keeps_its_caller_pure` (O126 on `set a [label abc]` through a wrapper proc, beside `string length`; flagless keeps it, D3.19), `a_mutator_stub_keeps_the_store_it_reads` (no O109, beside `lappend`; flagless draws it), `a_barrier_stub_fences_its_scope_from_renaming` (the compacted output keeps `$local`, beside `vwait`; flagless compacts it, D3.20), `a_loop_stub_is_checked_as_a_loop` (W241 / W240, `break` clears W241, beside `while`; flagless draws neither, D3.21), `a_scope_alias_stub_aliases_its_local` (I230 withheld, beside `upvar`; flagless folds), `an_unsafe_stub_is_hidden_in_a_safe_interpreter` (W129, beside `exec`; flagless none) and `a_stub_that_redeclares_a_catalogued_command_answers_nearest_wins` (`stub after {ms script}` drops `main → on_row`, `{ms script:body}` keeps it); `declaration.rs` gains `a_redeclared_name_answers_nearest_wins` and `declared_traits_and_effects_answer_under_the_security_floor` (a redeclared `exec` keeps `UNSAFE` and its effects), and `one_door_answers_catalogue_and_document` loses its union assertions; `side_effects.rs` gains `a_declared_command_classifies_from_its_declaration` (the flagless declaration equals the undeclared answer; an undeclared name equals `classify_side_effects`); `bounds_checks.rs` gains `a_document_declaration_answers_the_loop_question_for_its_name` (a redeclared `while` without `-loop` is not a loop). Gates: `cargo test -p tcl-registry` (lib 927 and all 19 binaries), `-p tcl-compiler --lib` (6508, 2 ignored) and `--test analyser` (504), `-p tcl-lsp-core --test stub_arg_roles` (31), `--lib` (2350) and `--test minify_residual` (39); `cargo check --workspace`; clippy (`-p tcl-registry -p tcl-compiler -p tcl-lsp-core --all-targets`) and `cargo fmt` clean; `registry-axes` (7831 / 16 / 40 / 896 across 147) and `value-transfers` (20 / 19 / 90 across 36 / 6607) pins unchanged, both ledgers regenerated for moved lines; `pack-goldens --check` (25) unchanged; `kcs-index-links` green; `dialect-drift` at its 8. Docs: `dialect-stubs.md` § *Flags* (each flag's field and consumer; the flagless answer; the catalogue-only residue) and § *Stubs are declarations* (nearest-wins as built, the floor) — both "today" sentences gone — and its key-files table; KCS `kcs-howto-annotate-commands-with-stubs.md` gains § *Flags* and § *Stubbing a command tcl-lsp already knows*; the design page's status box and the stub ruling's consequences (the consumers built, `-mutator`'s shape, SSA and memory SSA left on the catalogue with the reason); the centralisation plan's R1 and the redesign's stub bullet, whose union sentences now state nearest-wins; four code comments that said a sidecar's roles "can only widen" (`param_traits.rs`'s `resolve_arg_roles`, `graphs.rs`'s `document_unit`, `hover.rs`, `tcl-lsp-db`'s compiler-diagnostics path). Deviations: `-mutator` is the read-modify-write shape, not a write-only effect (D3.15); `side_effects` returns `Cow` and `invocation_traits` joins it (D3.16); the `-pure`, `-barrier` and `-loop` tests are renamed to the findings the tree has (D3.19–D3.21); `ssa.rs` and `memory_ssa.rs` are not wired (D3.22) |
| CC3.2 one `untrusted` predicate | landed | `wip(consumer-contracts): step 3 — one untrusted predicate` (`fca87728`) | `tcl_registry::model::registration::untrusted(provenance: Provenance) -> bool` beside `provenance_label`, re-exported as `tcl_registry::model::untrusted` — a thin door onto `Provenance::is_untrusted`; `rust/tcl-spectcl/src/loader/eval.rs`'s private `fn untrusted(tier: PackEnvironmentTier) -> bool` shim deleted, its one call site in `replay()` now calling the registry's predicate over `tier.provenance()` directly. Tests: `registration.rs` unit row `the_one_untrusted_predicate_names_the_three_classes` (every `Provenance` variant checked against `is_untrusted` directly; the three untrusted classes and the four trusted ones each asserted). Gates: `cargo test -p tcl-registry` (928 lib and all 21 binaries), `-p tcl-spectcl` (every binary: lib 188, `workspace_trust` 7, `eval_loader` and the rest — the pinned `a_workspace_pack_may_still_override_a_shipped_command` and `an_untrusted_pack_declaring_dialect_axes_fails_with_the_provenance_error` stand unmodified); `cargo check --workspace --all-targets`; clippy (`-p tcl-registry -p tcl-spectcl --all-targets`) and `cargo fmt` clean; `registry-axes --check` OK (7831 / 16 / 40 / 896 across 147, unchanged) and `value-transfers --check` OK (20 / 19 / 90 across 36 / 6607, unchanged); `pack-goldens --check` (25) unchanged; `kcs-index-links` green; `dialect-drift` at its 8. Deviations: `provenance_violation` and `provenance_violation_in` keep their `Tier` / `PackEnvironmentTier` parameters rather than becoming `Provenance`-typed as the plan's prose reads (D3.24) — `reserved_name_for`'s `PackEnvironmentTier` argument and the pinned `"Spec Studio override"` notice text depend on the tier value, and the outward rename would ripple into `tcl-spec-studio/src/store.rs`'s two callers, which the item's own file list omits; `rust/tcl-mcp/src/spectcl.rs` needed no edit for this item — nothing there called the deleted shim or depends on `provenance_violation`'s (unchanged) signature; CC3.4, next, rewrites that file's tier/trust handling on its own terms. D3.24 |
| CC3.4 `spectcl_check`'s tier and trust | landed | `wip(consumer-contracts): step 3 — spectcl_check reports tier and trust` (`6e9668c8`) | `spectcl_check` gains `tier` (`bundled`/`user`/`workspace`/`studio-override`, default `workspace` — the tier a `.tclspec` file actually installs at) and `trust` (`trusted`/`untrusted`, default `trusted`); the pack is still always evaluated as trusted (D3.25 — the authority ruling; evaluating under an actually-untrusted pair would let E-R2 discard the whole pack transactionally, emptying the per-command report the tool exists for), so `tier`/`trust` instead parameterise two previews over that one snapshot: `untrusted_tier_refusal` keeps reading `tier` alone, unconditional, exactly as before (`trust` never gates it — the pinned `a_workspace_tier_refusal_is_reported_without_failing_the_check` and `an_ordinary_pack_carries_no_tier_refusal` stand unmodified); `dormant_hooks` (new) is `tcl_spectcl::hooks::dormant_hooks` over the pack's own commands and `PackEnvironmentTier::of(tier, trust).provenance()` (empty unless the pair is an untrusted workspace, D3.12); `provenance` (new, D3.26) names that same verdict as a label (`tcl_registry::model::provenance_label`), answering "the provenance verdict computed for that pair" as an output, not only an input to `dormant_hooks`. `hook_json` also reports `HookDecl::line`. `tools.rs` gains the two properties with closed JSON-Schema enums (`closed_string_schema`, beside `dialect_schema`); `spectcl_check`'s description names the two new fields. Tests: `spectcl.rs` gains `the_default_tier_and_trust_equal_an_explicit_workspace_trusted_call` (a bare call answers byte-identically to an explicit `tier=workspace`/`trust=trusted` one; `dormant_hooks` empty, `provenance` "trusted workspace") and `trust_untrusted_lists_the_hook_as_dormant_and_still_refuses_an_override` (a `const_fold` hook beside an `-override` of a compiled command, checked at `trust=untrusted`: the hook is dormant, the override still refused, both commands still load — neither preview costs an analysis fact). Gates: `cargo test -p tcl-mcp` (111, was 109); `cargo check --workspace --all-targets`; clippy (`-p tcl-mcp --all-targets`) and `cargo fmt` clean; `registry-axes --check` OK (7831 / 16 / 42 / 896 across 147 — two new `irreducible` waivers, the `Tier`/`WorkspaceTrust` MCP-argument spellings in `declared_tier` and `TIER_VALUES`, neither command-registry vocabulary); `value-transfers --check` unchanged; `pack-goldens --check` (25) unchanged; `kcs-index-links` green; `dialect-drift` at its 8. Docs: the redesign's § 11.1 O4 row removed (closed); `spec-packs.md`'s "CLI and MCP" bullet reworded from "what the workspace tier would refuse" to the general `tier`/`trust` preview. Deviations: `gen-ai-diagnostics --check`'s conditional does not apply — it generates `ai/shared/diagnostics.json` from the `DiagCode` catalogue, not from `tools.rs`'s `ToolDef` list, and no generator derives from the tool catalogue (checked); `docs/kcs/features/kcs-feature-mcp-server.md`'s conditional does not apply either — its tool table is one line per tool and does not enumerate any tool's arguments, `spectcl_check` included (checked); no KCS note lists `spectcl_check`'s arguments. D3.25, D3.26 |
| Gate fix — the surface test asks its owner | landed | `wip(consumer-contracts): step 3 — the surface test asks its owner` | CC3.5's `a_declared_command_classifies_from_its_declaration` (`rust/tcl-compiler/src/side_effects.rs`) built its declaration set by hand (`DeclaredSurface::new` and `declare`), two spellings of the type outside its owner files, which `retired-api-gate`'s one-oracle sweep (ruling R10) reports. It now states its three declarations the way a document does (`# tcl-lsp: stub my_pure {} -pure`, `my_mut {} -mutator`, `my_plain {}`) and takes the set from the analyser's one ingestion door, `analyser::utils::document_declared_surface` (`document_stub_declarations` ingested through `build_declared_surface`, the path the analyser builds its own set by; `bounds_checks.rs`'s declared-loop tests already take it) — so the test no longer names the type and needs neither a `one-oracle-ok` waiver nor a centralisation-ledger row. The facts are the ones the hand-built declarations stated (`-pure` is `PURE`; `-mutator` is `READS_BEFORE_WRITE` beside a `Variable` read and write, D3.15; an inline block is `Provenance::Document`), so every assertion stands unchanged. Gates: `cargo test -p tcl-compiler --lib a_declared_command_classifies_from_its_declaration`; `retired-api-gate` OK (0 hits, was 2); `cargo check --workspace`; clippy (`-p tcl-compiler --all-targets`) and `cargo fmt` clean; `registry-axes --check` (7831 / 16 / 42 / 896 across 147) and `value-transfers --check` (20 / 19 / 90 across 36 / 6607) unchanged; `dialect-drift` at its 8; `owner-resolution` OK (45). |
| Review fixes | landed | `wip(consumer-contracts): step 3 — review fixes` | The step 3 review ("land with fixes"), item by item. **B1** `provenance_violation(pack, tier)` answered "as if untrusted" at every tier, so `spectcl_check` at `tier: bundled` or `user` reported an E-R2 refusal the load never gives (`untrusted` is false for `BundledPack` and `User`); it now returns `None` unless `tcl_registry::model::untrusted(PackEnvironmentTier::of(tier, WorkspaceTrust::Untrusted).provenance())` — the load's own predicate — so the Studio's two callers (`Workspace`, `StudioOverride`) and the pinned `a_workspace_tier_refusal_is_reported_without_failing_the_check` / `an_ordinary_pack_carries_no_tier_refusal` answer as before; new `a_bundled_or_user_tier_draws_no_refusal` (both tiers over `command lsort -override`: refusal null, no `provenance` notice, `provenance` `bundled` / `user`; the negative: the workspace default still previews it). **B2** the gate-fix row above, with the reviewer's fixture (`my_mut {v:var} -mutator`, so the declaration's facts flow through `declared_traits` / `declared_side_effects` from a real argument); `retired_api_gate.rs`'s `DeclaredSurface` owners gain `rust/tcl-lsp-core/src/minify.rs` with a carrier comment (it holds `build_declared_surface`'s set only to hand to `DocumentCommandSurface`, D3.23), so that exemption is deliberate rather than textual. **S1** `AGENTS.md`: stub declarations "answer that same query nearest-wins through `DocumentCommandSurface`". **S2** the design page's status box takes `alias_of` out of the proposed list (step 4 has begun: a field with its loader row and studio surfaces, nothing reads it yet), and the rung table's rung-2 cell reads "`alias_of` is a field nothing reads yet". **S3** the page's intro: the three descriptors step 2 "gave the analyser", the option effect "retired" the two native resolvers; the `substitution.rs` / `patterns.rs` file-path anchor names the projections that replaced them. **S4** the page's stub-ruling consequences call the SSA and memory-SSA readers deferred residue, give the boundary reason (`compilation_unit.rs`, the value-transfers lane's), name the false W210 below, and state the intended wiring; D3.22 is rewritten with the corrected reasons. **S5** D3.27. **S6** `declared_tier` matches `Tier::{Bundled, User, Workspace}.label()` and the one MCP-only token `studio-override`; `TIER_ARGUMENTS` beside it is the schema's list (`tools.rs`'s `TIER_VALUES` reads it) and `Tier::label` became `const fn` for it; both `irreducible` waivers go (`registry-axes` waived 42 → 40, ledger regenerated). **S7** D3.29. **S8** D3.30. **S9** D3.28. **S10** the KCS how-to's § *Stubbing a command tcl-lsp already knows* says a stub has no subcommands, so it replaces what tcl-lsp knows about each (`stub dict {args}` and `dict set`'s read of `d`). Beside them, the Studio's `untrusted_tier_refusal` doc comment loses its stale O9 sentence (discovery is told the trust state since CC3.1). The reviewer's W210 question, confirmed by a probe (not committed): `# tcl-lsp: stub run_with {v:var script:body}` then `run_with x { puts hi }` and `puts $x`, in a proc and at top level, draws `W210 Variable 'x' is read before it is set` — `lower_default` keeps the call a `Statement::Barrier` (the `body` word is a same-invocation executable) and SSA's `registry_barrier_defs` asks the catalogue alone; without the `body` word the call lowers plain with its declared def and there is no W210. Reported for filing, not fixed: the fix threads the surface through `compilation_unit.rs` (D3.22). Tests moved: CC3.4's `trust_untrusted_lists_the_hook_as_dormant_and_still_refuses_an_override` split into `trust_untrusted_lists_the_hook_as_dormant` (the hook alone: dormant, no refusal) and `a_pack_the_untrusted_install_refuses_holds_no_dormant_hook` (the hook beside `-override lsort`: the conditional refusal and no dormant hook; the negative: trusted, the preview refusal and still none dormant). Gates: `cargo test -p tcl-mcp` (113, was 111), `-p tcl-spectcl --no-fail-fast` (lib 188 and all 19 binaries; `eval_loader` 25, the load's own E-R2 messages byte-identical), `-p tcl-spec-studio --lib` (198), `-p tcl-registry --lib` (928), `-p tcl-compiler --lib a_declared_command_classifies_from_its_declaration`, `-p xtask retired_api` (9); `cargo check --workspace --all-targets`; clippy (`-p tcl-spectcl -p tcl-registry -p tcl-mcp -p tcl-compiler -p tcl-spec-studio -p xtask --all-targets`) and `cargo fmt` clean; `tsc --noEmit -p editors/vscode`, eslint and prettier on `clientCore.ts`; `registry-axes --check` (7831 / 16 / 40 / 896 across 147), `value-transfers --check` unchanged, `pack-goldens --check` (25) unchanged, `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8. D3.22, D3.27–D3.30 |

**Step 3 is landed.** All five items above (CC3.1, CC3.2, CC3.3, CC3.4 and
CC3.5) are `landed`; the review checklist below is run against this tree and
its evidence recorded there. `cargo check --workspace --all-targets`,
`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --
-D warnings` are all clean; every `xtask-check` gate this step touches
(`registry-axes`, `value-transfers`, `pack-goldens`, `kcs-index-links`,
`dialect-drift`, `callback-inventory`, `audit-option-dialects`,
`number-drift`, `segmentation-drift`, `command-backing`) is green.
`make rust-check`'s own `xtask-retired-api-gate` step fails on two
pre-existing sites in `rust/tcl-compiler/src/side_effects.rs`
(`DeclaredSurface` constructed directly in CC3.5's own
`a_declared_command_classifies_from_its_declaration` test) — confirmed
present, byte-for-byte unchanged, at the step's own base commit `250edd5b`
(`git stash` against the clean CC3.2/CC3.4 tree reproduces the same two
hits), so it predates every step-3 sonnet item and is left for review
rather than fixed here, out of the assigned scope; the gate fix row above
closes it, and `retired-api-gate` is clean from that commit on. The title above and
`docs/design/lanes/README.md`'s in-flight line are updated in this commit
to read step 3 landed; step 4 (identity: `alias_of`, the alias target's
identity, the stamp rule, `SiteClaim`) has not started.

### Step 3 — review checklist, verified

The plan's § *Review checklist* (§ *Plan for steps 2–10* › *Step 3*), run
against the landed tree with evidence:

- **Authority is never gated.** `cargo test -p tcl-spectcl --test
  workspace_trust an_untrusted_workspace_pack_still_declares_its_facts`
  passes: an untrusted workspace pack's arity, roles and `Declared`
  semantics reach the installed registry, equal under both trust states.
- **Exactly one `untrusted` predicate exists.** `grep -rn "fn untrusted("
  rust/` returns exactly one hit —
  `rust/tcl-registry/src/model/registration.rs:172`. The checklist's own
  looser pattern (`"fn untrusted"`, no trailing paren) returns five,
  matching `untrusted_tier_refusal` (`tcl-spec-studio`),
  `untrusted_compiled_extension` (`tcl-spectcl`), a test fixture builder
  `untrusted_at_root` and a unit-test name `untrusted_head_declines`
  besides the predicate itself — none of the other four is a predicate,
  as the CC3.2 hand-off already flagged.
- **A dormant hook is reported once per hook at its own line, never once
  per call; `plan_for` allocates it no slot.** `cargo test -p tcl-spectcl
  --test workspace_trust
  an_untrusted_pack_installs_no_hook_body_and_reports_each_as_dormant`
  passes (one information notice per hook, at its own line; `plan_for`
  empty for the pack); `spectcl_check`'s own
  `trust_untrusted_lists_the_hook_as_dormant_and_still_refuses_an_override`
  (CC3.4) confirms the same mechanism surfaced through the MCP tool, one
  `dormant_hooks` row per hook.
- **The trust wire is one input; an absent value is trusted; the cache
  key includes it.** `cargo test -p tcl-lsp-server --lib
  workspace_trust_comes_from_the_client_and_never_from_a_setting` passes,
  and so do `cargo test -p tcl-spectcl --test workspace_trust
  a_client_that_reports_nothing_is_trusted` and `… --test workspace_trust
  the_snapshot_key_distinguishes_trust` (run separately — `cargo test`
  takes one test-name filter).
- **Nearest-wins changed a union to a replacement; the negative witness
  exists and the delta is recorded.** `cargo test -p tcl-registry --lib
  a_redeclared_name_answers_nearest_wins` and `… --lib
  declared_traits_and_effects_answer_under_the_security_floor` (run
  separately) both pass, and so does `cargo test -p tcl-lsp-core --test
  stub_arg_roles
  a_stub_that_redeclares_a_catalogued_command_answers_nearest_wins`; the
  delta is CC3.5's two bullets in § *Behavioural deltas accepted
  in step 3* below.
- **Risks.** The cache-key change is accepted, not a defect (D3.7; the
  "every on-disk compiled-pack cache entry key moved once" bullet below).
  `SAFE_INTERP_HIDDEN` reaching an unexpected diagnostic is exercised by
  the pinned `an_unsafe_stub_is_hidden_in_a_safe_interpreter` (CC3.5) with
  no other incident found. The VS Code client's trust event racing
  `initialized` is addressed by design (D3.14: `initializationOptions` is
  a function re-read on every start, and `registerWorkspaceTrustGrant` is
  registered before `client.start()`); this landing does not independently
  re-run `npm test` to confirm it (CC3.3's own item ran `tsc --noEmit`,
  eslint and prettier on the changed `.ts` files instead, for the reason
  its row already records).

### CC3.1 — what the next items read

- **The input.** `tcl_dialect::model::WorkspaceTrust` (`Trusted`, the
  default, and `Untrusted`); `workspace_provenance()` maps it onto
  `Provenance::WorkspaceTrusted` / `WorkspaceUntrusted`. It arrives on
  `DiscoveryOptions::workspace_trust`, which the scan never reads; the
  load is handed it (`bundled::load_discovered_in(store, files, trust)`,
  `pack::load_under`, `pack::load_in_memory_under`), and each file reads it
  through `Tier::trust_under`. The server's reload already passes
  `options.workspace_trust`, so CC3.3 only has to set it in
  `spec_pack_discovery`.
- **The pack's class.** `MergedPack::trust` (normalised by the winning
  tier) and `MergedPack::provenance()` — what CC3.3's `load_sources`
  notice and `hooks::plan_for` read (D3.3). `PackEnvironmentTier::of(tier,
  trust).provenance()` is the same answer for a bare `Pack`.
- **The key.** `EvalOptions::trust`, `EvalSnapshotKey::trust`,
  `cache::key_for(source, tier, trust)`, and the pack-set key, which moves
  when the state does — so a reload that only changed the trust state is a
  changed set, and `reload_spec_packs` re-installs.
- **For CC3.2.** The tree already decides the class in one place:
  `Provenance::is_untrusted` (`tcl_dialect::model`, #2139, `44c58a5b`),
  which `loader/eval.rs`'s private `untrusted(PackEnvironmentTier)` and
  `tcl_registry::model::registration` both call — the plan's two
  identical predicates were collapsed into one before step 3. What remains
  is the plan's API shape: a `tcl_registry::model::untrusted(provenance)`
  would be a second door onto `is_untrusted`, and `provenance_violation(pack,
  tier)` (whose hypothetical is now "as if untrusted", D3.8) still takes a
  tier. `grep -rn "fn untrusted(" rust/` answers one hit today (the
  loader's private shim); the checklist's looser `"fn untrusted"` also
  matches `untrusted_compiled_extension` and the studio's
  `untrusted_tier_refusal`, which are not predicates.

### CC3.3 — what the next items read

- **For CC3.4 (`spectcl_check`'s `dormant_hooks`).** The tool evaluates a
  single `Pack`, not a `PackSet`, so it cannot ask `HookPlan`; the one
  list is `tcl_spectcl::hooks::dormant_hooks(&pack.name, &pack.commands,
  provenance)` with `provenance = PackEnvironmentTier::of(tier,
  trust).provenance()` — empty unless the pair is an untrusted workspace.
  Each `DormantHook` carries `command`, `field` and `line` (its `file` is
  empty for an unmerged pack); `PackNotice::dormant(&hook).message` is the
  load's wording, if the tool wants the same sentence.
  `hooks::hook_bodies_run(provenance)` is the predicate. `HookDecl::line`
  is new, so `spectcl.rs`'s `hook_json` may report it.
- **For CC3.2.** Nothing CC3.3 added reads `untrusted` or
  `Provenance::is_untrusted`: dormancy is `provenance ==
  WorkspaceUntrusted`, deliberately narrower (D3.12), so collapsing the
  registration predicate moves no CC3.3 answer.
- **For the landing.** The server's trust state is `Backend::workspace_trust`,
  set only by `workspace_trust_in` (top-level key); `getEffectiveConfig`
  does not report it. The two design indexes' lines for the page
  (`docs/design/README.md`, `docs/design/compiler/README.md`) still read
  "built in step 2" and do not name the trust gate.

### CC3.5 — what the next items read

- **The one door.** `DocumentCommandSurface::{traits, invocation_traits,
  side_effects}` beside the role queries; a consumer that reads a trait or
  an effect of a command a document may declare asks the surface, never
  `registry.get(name).traits`. `DocumentCommandSurface::declares(name)` is
  how a consumer tells a declared name from a catalogued one where it must
  (the minifier's subcommand observability, `classify_side_effects_in`).
- **Residue, each with its reason in D3.19 and D3.22–D3.23.** The direct
  O108 / O126 gate in `optimiser/elimination.rs` and GVN's purity
  (`gvn.rs`) still call `classify_side_effects` over the catalogue, so a
  `-pure` call's unused result goes only through a wrapper procedure's
  summary — `classify_side_effects_in` is the door they move onto once
  `elimination.rs` is free of VT8.5 and the pass context carries the unit's
  `declared_commands`. `ssa.rs`'s `registry_barrier_defs` and
  `uses_in_barrier`, and `memory_ssa.rs`'s `is_clobber`, read the
  catalogue only — deferred until `compilation_unit.rs` is free, with the
  intended wiring in D3.22; meanwhile a stub declaring a `var` word beside
  a `body` word draws a false W210 on the variable (the review's probe).
  The loop-exit set (`TERMINATES_BLOCK`) and the interprocedural
  `INVOKES_USER_PROC` head read traits no flag states and stay on the
  catalogue.
- **For the landing.** The two design indexes' lines for the page still
  read "built in step 2"; they name neither the trust gate nor the stub
  flags.

### Behavioural deltas accepted in step 3

- CC3.1: every on-disk compiled-pack cache entry key moved once (the trust
  byte), so the cache rebuilds on the first load; unobservable (D3.7).
- CC3.3: in a VS Code workspace the user has not trusted, a workspace
  pack's hook bodies do not run — each field keeps its abstaining
  placeholder — and each is reported once as an information notice on its
  own row; the declarative facts are unchanged, and a grant re-installs
  with no restart. Every other editor sends no state and is trusted, so
  nothing moves there.
- CC3.3: the pack goldens' `hooks` digests moved once for the eight packs
  that declare hooks (`HookDecl` gained its line, and the digest reads it
  zeroed); no `spec` or `grammar` digest moved.
- CC3.3: VS Code's Restricted Mode description for the extension now says
  a workspace pack's hook bodies stay dormant until the workspace is
  trusted.
- CC3.5: each stub flag now has an effect — `-loop` draws W240 / W241 on a
  declared condition and body, `-pure` makes a procedure that only calls
  the command pure (so O126 can remove an unused call of it), `-mutator`
  keeps the store the command reads (no O109), `-unsafe` draws W129 inside
  a safe interpreter, `-scope_alias` makes the names it takes unknown to
  the call-site scan (I230 withheld), and `-barrier` fences the minifier's
  renaming of the scope it runs in. A stub with no flags classifies as an
  undeclared command does.
- CC3.5: a stub that redeclares a catalogued command answers alone for it —
  its roles, prefixes, traits and effects — beneath the shipped command's
  security traits and side effects: `stub after {ms script}` no longer
  makes `after`'s script a call-graph body.
- CC3.5: in the interprocedural summary a call of a stub-declared command
  is no longer an unknown call (`has_unknown_calls`, the Explorer's
  `hasUnknownCalls`); its purity is still decided by its classification,
  so a flagless stub leaves its caller impure as before.
- CC3.4: `spectcl_check` gains `tier` and `trust` arguments and two output
  fields, `provenance` and `dormant_hooks`; every existing field's value at
  the defaults (`tier: "workspace"`, `trust: "trusted"`) is unchanged.
- Review fixes: `spectcl_check` at `tier: "bundled"` or `"user"` no longer
  reports an `untrusted_tier_refusal` — the load never refuses either tier,
  so the report was false.
- Review fixes: an authoring tool's refusal preview (`spectcl_check`'s
  `untrusted_tier_refusal`, the Spec Studio's two refusal reports) reads
  "…, so the pack would not be loaded from the {class} tier (design
  E-R2)"; the load's own refusal notice is unchanged.
- Review fixes: `spectcl_check` for an untrusted pair that refuses the pack
  reports no `dormant_hooks`.
- Review fixes: the studio override's provenance label reads "Spec Studio
  override" (was "studio override") in `spectcl_check`'s `provenance` and
  in the two environment-registration refusals.
- Review fixes: in VS Code, granting Workspace Trust while the language
  server is stopped no longer leaves an unhandled promise rejection.

## Step 4 — progress

Item order follows § *Plan for steps 2–10* › *Step 4* § *Ordering and
checkpoints*: CC4.1 (sonnet) first, on its own; then the opus items in
order, CC4.2, CC4.3, CC4.4, each its own checkpoint.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC4.1 `alias_of` | landed | `wip(consumer-contracts): step 4 — alias_of` | `CommandSpec::alias_of: Option<&'static str>` beside `deprecated_replacement`/`deprecated_replacement_drop_in` (the same shape, so every one of the ~3888 existing `CommandSpec { … }` literals keeps compiling through `..CommandSpec::DEFAULT`), doc: "the shipped builtin this pack command is; the only admissible source of a builtin identity for a pack command; never inferred from a realm alias" — D4.1 confirmed exactly: a field only, no consumer. `rust/tcl-spectcl/src/loader.rs`'s `apply_command_stmt` gains `"alias_of" => spec.alias_of = Some(leak_str(&value))`, and `loader/eval.rs`'s `ROW_WORDS` gains `"alias_of"` so the Tcl-evaluated pack path captures the row too — the two halves the crate's own module doc calls "exactly one loader" share this per-row apply match. Studio surfaces: `schema.rs` (`IDENTITY` category, `FieldKind::OptText`), `draft.rs` (`opt_str(spec.alias_of)`), `render_spectcl.rs` (`text(out, ctx, draft, "alias_of")`, renders `alias_of NAME`), `coverage.rs`'s witness pattern and `Field` table (`Surface::Key("alias_of")`, no `GAPS` row) — plus two surfaces the plan's "the four surfaces" phrase did not name, each gated by its own completeness test that failed at compile or test time until filled (D4.6): `examples/fields_core.rs` (an Identity-section worked example over a pack command, `vendor::unpack`, since no *shipped* command can ever carry a pack-only field) and `relations.rs` (`STANDALONE`, "declared vocabulary only" — no sibling field exists to cluster with yet). `docs/references/command-spec/fields.md` regenerated (`UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`); `docs/design/spec-dsl-examples/README.md`'s keyword table gains the `alias_of NAME` row. Tests: `rust/tcl-registry/tests/registry_sweep.rs`'s `alias_of_names_a_shipped_command_of_the_same_family` — a live, forward-looking sweep over every `LOADABLE_DIALECTS` registry (0 shipped specs declare it yet, so this checks nothing today and everything the day one does) plus three synthetic `CommandRegistry::build_default()` + `.insert()` fixtures: a real target resolves, an unknown target does not, and a command real only in another dialect (`HTTP::header`) does not resolve in the plain Tcl family — the "of the same family" qualifier. `spectcl_roundtrip.rs` needed no new test: `every_command_in_every_dialect_round_trips_through_spectcl` and `the_twelve_port_fixtures_render_and_reload_as_themselves` already exercise every field generically once the loader and renderer speak it, and both passed unmodified. Gates: `cargo test -p tcl-registry` (928 lib and all 21 binaries, `registry_sweep` 39 — one more than the 38 baseline), `-p tcl-spectcl` (every binary, `golden_packs` included), `-p tcl-spec-studio` (198 lib and all binaries, `reference_doc` included), `-p tcl-compiler --lib` (6508, 2 ignored), `-p tcl-lsp-core --lib` (2350), `-p tcl-mcp` (111), `-p tcl-cli` (every binary); `cargo check --workspace --all-targets`; clippy (`-p tcl-registry -p tcl-spectcl -p tcl-spec-studio --all-targets`) and `cargo fmt` clean; `registry-axes --check` OK (7831 / 16 / 42 / 896 across 147, unchanged — the new test's string comparisons trip no site); `value-transfers --check` unchanged; `pack-goldens` rewrote all 25 snapshots (every command's `spec` digest moved once, since the digest hashes `CommandSpec`'s whole `Debug` text and every spec now carries `alias_of: None,` — `hooks`/`grammar` digests untouched, the same mechanical shape CC3.3's `HookDecl::line` addition moved 8 of them for) and `--check` passes; `kcs-index-links` green; `dialect-drift` at its 8; `retired-api-gate`'s two pre-existing hits (`side_effects.rs`, unrelated, unchanged since `250edd5b`) remain the only gate not clean, out of this item's scope (recorded at the step 3 landing). Deviations: the plan names `render_spectcl.rs` / `schema.rs` / `help.rs` / `draft.rs` / `coverage.rs` as "the four surfaces" (five names for four surfaces plus the witness); the tree's own completeness gates added two more, `examples.rs` and `relations.rs`, each already enforced for every other `CommandSpec` field and newly enforced for this one the moment it existed — filled rather than bypassed (D4.6). D4.1, D4.6 |
| CC4.2 the stamp rejection rule | landed | `wip(consumer-contracts): step 4 — the stamp rejection rule` | `rust/tcl-spectcl/src/stamps.rs` (new): `Stamp` (`Codegen`, `InlineCodegen`, `Intrinsic`), `StampSite` (`Command`, `Subcommand(name)`, `Form(name)`), `RefusalReason` (`TierGate`, `NoAliasOf`, `UnknownTarget`, `NotTheTargetsOwn`), `StampRefusal` with `message()`; `stamps_admitted_from(provenance)` (rule 2: `BuiltIn`, `BundledPack`), `carries_stamp(spec)`, `shipped()` (the lenient all-Tcl store, D4.7), `stamp_refusals(spec, provenance, shipped)` (pure — the previews read it) and the plan's `admit_codegen_stamps(command: &mut PackCommand, provenance, shipped) -> Vec<StampRefusal>`, which strips through a memo keyed by the original spec's address and the exact drops (D4.10). Rule 1 at every site a stamp can sit — the command, each subcommand, each `command_forms` entry — against the target's same-named site (D4.8); the refusal names the first shipped carrier by name, at the same site when one exists (D4.9). `pack::load_sources` applies it to every merged command at `MergedPack::provenance()` and pushes `PackNotice::stamp_refused` (the command's row, context `command NAME`, `Severity::Warning`) per refusal — the plan's message verbatim for a trusted workspace pack with no target: "`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace pack may not name a codegen catalogue member; the stamp would have to sit on `alias_of lassign`". `loader.rs`: the "names a codegen hook" `log.say` is gone (the rule's notice replaces it); the lowering-hook notice stays. `install.rs`: `debug_assert!` that no stamp survives from a provenance `stamps_admitted_from` refuses. Beyond the plan's files (D4.11): the Spec Studio assembles its own set (`store.rs`'s `merged()`), so it applies the rule to the world it installs — the document and its drafts keep the rows — and reports `PackStore::stamp_refusals` / `patch_stamp_refusals` as `stamp_refusals` in the store view (and its `patch` object); `spectcl_check` reports `stamp_refusals` for the pair's install beside `dormant_hooks`, both from a new `install_preview` helper (the function had crossed clippy's line limit), empty where the install refuses the pack (D3.27); the Studio's `relations.rs` files `alias_of` in a new "Builtin identity" cluster with the three stamp fields, its help text states the rule, and its worked example takes `lassign`'s word order (D4.12); `alias_of`'s own doc comment in `spec.rs` states the rule as built. Tests: `workspace_packs.rs` gains the plan's four — `a_workspace_stamp_without_alias_of_is_refused_and_names_the_target` (the plan's message, on the command's line, the stamp dropped), `a_workspace_stamp_with_alias_of_is_refused_by_the_tier_gate` (trusted and untrusted workspace, user and Spec Studio override, each naming its provenance; `alias_of` kept), `a_refused_stamp_costs_no_analysis_fact` (the stripped spec's `Debug` rendering equals the loader's with only `codegen_hook` cleared; installed arity, `alias_of`, `Value` and `VarWrite` roles survive), `a_bundled_stamp_on_an_alias_of_target_is_admitted` (through `bundled::load_from` on a temp `specs/`: no notice, the stamp installed; the negative: `alias_of lsort` refused by rule 1, naming `lassign`); `stamps.rs` unit tests for a subcommand stamp (admitted through `alias_of string`, refused without it, naming `string`), a form stamp with an unknown target, the gate and a stamp nothing ships, and the memo (a second strip returns the same pointer); `spectcl.rs` `stamp_refusals_preview_the_install_the_pair_describes`; `store.rs` `a_stamp_is_kept_in_the_document_and_dropped_from_the_installed_world`. Moved: `spectcl_roundtrip.rs`'s `is_policy_report` loses its dead codegen branch; `spec_corpus_baseline.txt` re-blessed — the upvar port's "names a codegen hook" line gone, six refusals added (the `return`, `string` and `upvar` ports copy their shipped specs' stamps at the workspace tier: `inline_codegen_hook Return`, `inline_codegen_hook String`, `semantic_operation {Intrinsic …}` on `string`'s `is`, `length` and `range`, `codegen_hook Upvar`), 30 lines in seven groups, the header's group text updated; `pack-goldens` rewrote `upvar.snap` only (the notice gone; no `spec` digest moved — the golden renders the loader's pack, stamps as written). Gates: `cargo test -p tcl-spectcl --no-fail-fast` (lib 192, was 188; `workspace_packs` 9, was 5; `spec_corpus` 5, `golden_packs` 3, `i6_security_floor` 2 and every other binary), `-p tcl-spec-studio --no-fail-fast` (lib 199, was 198; `reference_doc` with `fields.md` regenerated; `spectcl_roundtrip` and every other binary), `-p tcl-mcp` (114, was 113); `cargo check --workspace --all-targets`; clippy (`-p tcl-spectcl -p tcl-spec-studio -p tcl-mcp -p tcl-registry --all-targets`) and `cargo fmt` clean; `pack-goldens --check` (25), `spec_corpus` baseline, `registry-axes --check` (7831 / 16 / 40 / 896 across 147) and `value-transfers --check` unchanged, `retired-api-gate` / `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8. Docs: the design page's § *The loader's stamp rejection rule* states rules 1–3 as built (the site precision, `stamps_admitted_from`, the warning on the command's row, the previews; rule 4 stays step 6's), § *Codegen and the registry today*'s bullet, the rung-2 table cell and bullet, and the status box — every "today" sentence about the loader accepting stamps from any tier is gone; `spec-packs.md` § *What a pack still cannot say* gains the rule; `spec-dsl-examples/README.md`'s load-policy bullet and `alias_of` row; KCS `kcs-qa-why-was-my-pack-codegen-hook-refused.md` (User, all-editors), indexed, and linked from `kcs-howto-write-a-tclspec-pack.md`. Deviations: the rule covers subcommand and form stamps (D4.8); the refused-with-target remedy wording (D4.9); the memo (D4.10); the Studio and MCP previews (D4.11); the Studio relation, help and example (D4.12). D4.7–D4.12 |
| CC4.3 codegen records the alias target | landed | `wip(consumer-contracts): step 4 — codegen records the alias target` | `rust/tcl-registry/src/codegen_stamp.rs` (new): `CodegenStamp` and `StampSite` move here from CC4.2's `stamps.rs` (re-exported there), with `CommandSpec::codegen_stamps` and `CommandSpec::carries_codegen_stamp_at(site, stamp)` — the one "same stamp, same site" answer the loader's rule and codegen both ask — and `ResolvedCall::stamp_site(stamp)` (form over subcommand over command, `resolve_call`'s precedence) and `ResolvedCall::stamp_identity(registry, stamp)`: the `alias_of` target's name where the registry's spec for it carries the stamp at the same site, the resolved spec's own name otherwise (D4.13). `registry_codegen_hook` (`codegen/emitter/bytecoded.rs`) records `stamp_identity(ctx.registry, CodegenStamp::Codegen(hook))` and the inline path (`codegen/cmd_subst.rs`'s `inline_codegen_resolution`) `stamp_identity(self.registry, CodegenStamp::InlineCodegen(hook))`; `command_binding_matches` is unchanged (its one prefix-free alias hop already resolves the pack name to the builtin); lowering-hook sites (`codegen/mod.rs`'s `inline_lowering_hook`) and const-fold sites (`const_subst.rs`) keep the spec's own name (D4.13). Tests: `rust/tcl-spectcl/tests/codegen_stamps.rs` (new; shard row `5 tcl-spectcl::codegen_stamps`; dev-deps `tcl-vm`, `tcl-runtime-api`, both already in the crate's graph, `Cargo.lock` gains the two edges) — a bundled `specs/` pack declaring `vendor::unpack` as `alias_of lassign` with `codegen_hook -native Lassign`, compiled through the lowering → CFG → codegen pipeline against the installed registry: `an_admitted_alias_stamp_records_the_targets_identity` (the top level's `command_bindings` holds `CommandBindingIdentity::new("vendor::unpack", "lassign")` and nothing with identity `vendor::unpack`), `the_vm_admits_it_through_the_alias_hop` (a `tcl_vm::Vm` whose compile service counts plain-dispatch compiles: before the alias the module is refused and recompiled plain, erroring on the unknown name; after `interp alias {} vendor::unpack {} lassign` it runs as compiled — `1 2`, no plain compile) and `a_proc_at_the_pack_name_recompiles_plain` (a proc at `vendor::unpack`: refused, recompiled plain, the proc's `P Q` where the specialised code would have given `1 2`) (D4.14); a mutation check — `registry_codegen_hook` put back to `resolved.spec.name` — fails the first two, and the negative still passes. `codegen_stamp.rs` unit tests: the identity is the target only where the stamp is the target's own (`lassign` → `lassign`; `lsort` or none → `vendor::unpack`), `an_override_with_an_unrelated_alias_keeps_its_own_identity` (a `lassign` spec naming `alias_of lsort` records `lassign`), and `string length`'s intrinsic carried at `Subcommand("length")` only. Gates: `cargo test -p tcl-spectcl --test codegen_stamps` (3), `--test workspace_packs` (9), `--lib stamps` (4); `-p tcl-registry --lib codegen_stamp` (3); `-p tcl-compiler --test codegen --test codegen_integration` (164, 17); `-p tcl-vm --test command_mutation_deopt_e2e` (75); `-p tcl-spec-studio --lib` (199) and `--test reference_doc` (`fields.md` regenerated); `cargo check --workspace --all-targets`; clippy (`-p tcl-registry -p tcl-compiler -p tcl-spectcl -p tcl-spec-studio -p tcl-mcp --all-targets`) and `cargo fmt` clean; `verify-nextest-binary-shards.py --metadata-only` (328 targets) and `test-nextest-binary-shards.sh` green; `registry-axes --check` (7831 / 16 / 40 / 896 across 147), `value-transfers --check` unchanged, `pack-goldens --check` (25), `retired-api-gate` / `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8. Docs: the design page's status box, § *Codegen and the registry today* (the second witness exists), the rung-2 table cell and bullet (both corrections built, with the override reason), and the test anchors; `vm-compiled-artifact-provenance.md` (the alias-target identity and its admission; the new witness); `alias_of`'s doc comment; the Studio's help text and worked example. Deviations: the identity is conditional, not `alias_of.unwrap_or(name)` (D4.13); the test's word order, counting service and explicit namespace (D4.14). D4.13, D4.14 |
| CC4.4 site claims | landed | `wip(consumer-contracts): step 4 — site claims` | `rust/tcl-runtime-api/src/site_claim.rs` (new): `PackFactStamp { pack, content_hash: u64, vocabulary_version, overlay_generation, evaluator_revision }` — its doc comment names `content_hash` as the `u64` xxh3 the declaring file's `EvalSnapshotKey` interns (D4.3) — and `SiteClaim` with the two variants this step builds, `PackFacts(PackFactStamp)` and `BuiltinAlias { binding, facts }`, and `facts()`; no `Generic`, no `IdentityKind` (D4.15). `rust/tcl-bytecode/src/lib.rs`: `FunctionAsm::site_claims: Vec<SiteClaim>` (the literals in `format.rs`'s test and four `tcl-vm` test files gain `site_claims: Vec::new()`). `rust/tcl-registry/src/pack_origin.rs` (new): `PackOrigin { pack, content_hash, vocabulary_version }`; `CommandRegistry::insert_pack_origin` / `pack_origin`, a side table keyed by the installed spec's address (D4.4), which `project_for_profile` now carries with the overlay generation (D4.17); the registry's `Debug` counts it. `tcl-spectcl`: `PackCommand::content_hash`, set by the loader on both evaluation paths (`loader/eval.rs`'s `content_hash`, `pack_content_hash`, `with_content_hash` — the root file's hash, taken before the byte-order mark is stripped as `eval_snapshot_key` takes it, folded with each `include`d fragment's) (D4.16); `install::install_into` records every inserted spec's origin through `install::pack_origin`, which `PackSet::fact_stamps(evaluator_revision)` also builds from — one stamp per pack file the set installs, sorted and deduplicated, the facts an embedder hands the VM; `tcl-runtime-api` moves from CC4.3's dev-dependencies to the crate's own (`Cargo.lock` unchanged). `rust/tcl-compiler/src/site_claims.rs` (new): `pack_fact_stamp(origin, overlay_generation, evaluator_revision)`, the one stamp construction; `evaluator_revision()`; `pack_facts_claim(registry, spec)` (rung 1) and `builtin_alias_claim(registry, resolved, binding)` (rung 2, `None` where the binding's identity is the spec's own name). Codegen: `CodegenCtx::site_claim_requirements` becomes each function's `site_claims`; `stamped_binding` returns a `SiteBinding { binding, claim }` to `registry_codegen_hook` and `inline_codegen_resolution`, whose callers `require_site_binding`; `const_subst.rs`'s `ResolvedConstSubst::site_claims` (both fold returns, nested folds included) is required by `values.rs`'s `try_emit_constant_fold`; `trusted_inline_codegen_binding` declines a claimed site (D4.18). `rust/tcl-vm/src/interp.rs`: `Vm::set_pack_facts(stamps)`, which replaces the held facts and advances the compilation-deopt epoch, and `site_claims_hold` inside `function_command_bindings_match` (D4.19). No `rust/tcl-lsp-server` change: the server runs no VM (D4.20). Tests: `codegen_stamps.rs` gains the plan's `a_changed_pack_invalidates_the_site` (the VM holds the set's stamps with the content hash flipped: the module is refused and recompiled plain, `1 2` through the alias; the negative control, the set's own stamps, admits it with no plain compile) and, beyond the plan, `a_pack_fold_is_admitted_only_under_its_pack_s_facts` — rung 1 on its own: a bundled `llength -override` naming `llength::const_fold`, scoped `dialects tcl9.0` (D4.21), folds `[llength {a b c}]` to `3` and claims `PackFacts` beside `llength`'s own binding; a VM holding no facts refuses it and recompiles plain, and holding the set's facts runs it as compiled; `an_admitted_alias_stamp_records_the_targets_identity` asserts the site's `BuiltinAlias` claim carries the one stamp `fact_stamps` gives; `command_mutation_deopt_e2e.rs` gains the plan's `a_rung_zero_module_is_admitted_under_a_changed_pack_set` (a module with specialised bindings and no claims runs as compiled under no facts, one set's and a changed set's, with zero plain compiles); `site_claims.rs` unit tests `a_pack_fold_claims_the_pack_s_facts` (the same fold from an embedder-inserted spec claims nothing) and `an_inline_alias_site_claims_the_builtin` (the inline path's claim, on `alias_of lindex`). A mutation check — `site_claims_hold` answering `true` — fails exactly the two rung-1 witnesses. Moved: `the_vm_admits_it_through_the_alias_hop` and `a_proc_at_the_pack_name_recompiles_plain` set the set's facts before running — an admitted alias site now needs them, and without them the proc test's refusal would no longer isolate the proc; `pack_formatting.rs` normalises the new `content_hash` field in the `Debug` dump it compares (its doc's two differences become three). Gates: `cargo test -p tcl-spectcl --no-fail-fast` (lib 192; `codegen_stamps` 5, was 3; `workspace_packs` 9, `pack_formatting` 2, `golden_packs` 3 and every other binary); `-p tcl-compiler --lib` (6510, was 6508; 2 ignored), `--test codegen --test codegen_integration --test codegen_depth --test compiler_residual` (164, 17, 55, 59); `-p tcl-vm` lib 97, `command_mutation_deopt_e2e` 76 (was 75), `embed_api_e2e` 14, `opcode_c_parity` 86, `opcode_catch_parity` 17, `opcode_dispatch_coverage` 3, `run_script` 96 of 97 (`encoding_command` reads the system encoding, which the container's empty `LANG` makes `iso8859-1`; it passes under `LANG=C.UTF-8`, with or without this item); `-p tcl-bytecode` 33, `-p tcl-runtime-api` 29, `-p tcl-registry --lib` 931, `-p tcl-spec-studio` (lib 199 and every binary), `-p tcl-mcp` 114; `cargo check --workspace --all-targets`; clippy (`-p tcl-runtime-api -p tcl-bytecode -p tcl-registry -p tcl-compiler -p tcl-spectcl -p tcl-vm -p tcl-mcp -p tcl-spec-studio --all-targets`) and `cargo fmt` clean; `verify-nextest-binary-shards.py --metadata-only` (328 targets); `registry-axes --check` (7831 / 16 / 40 / 896 across 147) and `value-transfers --check` (20 / 19 / 90 across 36, 6607 rows) unchanged, `pack-goldens --check` (25), `retired-api-gate` / `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8. Docs: the design page's status box, § *Codegen and the registry today* (the optimise-path sentence corrected, D4.20), the rung table's state column for rungs 1 and 2, § *What the artefact records per rung* (rungs 1 and 2 built, `content_hash: u64`, the vocabulary version's source, no rung-0 variant), § *The admission checks, per rung* (rows 1 and 2 built), the rung 1 and 2 bullets, and the anchors; `vm-compiled-artifact-provenance.md` gains the claim, its invalidation row, and the witnesses; `spec-packs.md` § *What a pack still cannot say* states what an edited pack does to compiled sites. Deviations: the claim's two variants (D4.15); the stamp's sources (D4.16); the projection (D4.17); which sites claim (D4.18); the check's seat (D4.19); no server wiring (D4.20); the rung-1 witness beyond the plan, and its scoped override (D4.21). D4.15–D4.21 |
| Step 4 review fixes | landed | `wip(consumer-contracts): step 4 — review fixes` | The step 4 review ("land with fixes", none blocking), the mechanical fixes only — SF2 (the include-fragment `content_hash` test), SF3 (the `project_for_profile`/overlay-generation stamping test) and D3.22's SSA wiring are the opus implementer's, next. **SF1** `retired_api_gate.rs`'s `DeclaredSurface` owners drop `rust/tcl-lsp-core/src/minify.rs`: the file never spells the type, only calls `build_declared_surface` (line ~974), and the gate was already at 0 hits, so the entry was inert and would have silenced a future hand-built `DeclaredSurface::new()` there instead of catching it; the comment explaining why `minify.rs` is a carrier moves to sit beside the `build_declared_surface` call in `minify.rs` itself, and the retired-api-gate comment above the pattern is reworded to explain the omission instead of the inclusion. **SF4** `registry-consumer-contracts.md`'s file-path-anchors line now reads "the one `untrusted(…)` predicate" (was "the second"), matching CC3.2's collapse of the two into one (`rust/tcl-registry/src/model/registration.rs`'s `untrusted` is the only `fn untrusted(` in the tree). **SF5** `rust/tcl-spectcl/tests/workspace_packs.rs`'s AGPL header gains the "You should have received a copy…" paragraph its neighbour `codegen_stamps.rs` carries; it was truncated at "GNU Affero General Public License for more details." straight to the SPDX line. **Nit** `spectcl_check`'s tool description in `rust/tcl-mcp/src/tools.rs` (~2291) loses its doubled "and" before `dormant_hooks` (the `plus` list now reads `…, untrusted_tier_refusal (…), dormant_hooks (…), and stamp_refusals (…)`, one "and" before the last item). **Waiver count** the "Step 4 is landed" paragraph's `registry-axes` citation read `7831 / 16 / 40 / 896 across 147`, a figure CC4.1 inherited from a pre-step-3-review-fix branch position and every later step 4 item repeated unverified. Step 3's own review fix (`1b05e252`) is what actually dropped two irreducible waivers — `rust/tcl-mcp/src/spectcl.rs:274` and `rust/tcl-mcp/src/tools.rs:2352`, both `until never` — taking the ledger from 38 to 36 irreducible before any step 4 commit landed; no step 4 commit touched `docs/generated/registry-axes.md` again (`git log --follow` on the file stops at `1b05e252`), yet the checked-in file already reads 36 waived (all of it `irreducible`; every other axis 0) and 893 pinned — confirmed by running `cargo xtask registry-axes` fresh, which reproduces the checked-in file byte-for-byte bar this commit's own line-number shift in `minify.rs`'s two waiver rows (one at `:1893`, now `:1897`, from SF1's added comment). The landing paragraph is corrected to `7831 / 16 / 36 / 893 across 147`; CC4.1–CC4.4's own rows are left as each item's own historical record and are not rewritten. Gates: `cargo check --workspace --all-targets`; clippy (`-p xtask -p tcl-lsp-core -p tcl-spectcl -p tcl-mcp --all-targets --no-deps -- -D warnings`) and `cargo fmt --all -- --check` clean; `cargo xtask retired-api-gate` OK (0 hits, unchanged); `registry-axes --check` OK (7831 / 16 / 36 / 893 across 147, regenerated for the `minify.rs` line shift only); `value-transfers --check` unchanged; `owner-resolution` OK; `kcs-index-links` green; `dialect-drift` at its 8 (no new site). |
| Step 4 review fixes, continued | landed | `wip(consumer-contracts): step 4 — review fixes, continued` | The step 4 review's two remaining fixes; D3.22's SSA and memory-SSA wiring stays deferred until value-transfers slice 6 lands, because it shares `ssa.rs`. No source file changes. **SF2** D4.16's include fold: `rust/tcl-spectcl/tests/eval_loader.rs` gains `a_content_hash_is_the_snapshot_keys_and_follows_an_included_fragment`. The review named `eval.rs`'s tests or `workspace_packs.rs`; `eval.rs` has no test module of its own, and `eval_loader.rs` is the file that selects both drive paths (`EvalOptions::static_fast_path`) and already resolves fragments through `IncludeContext::new`, so the test sits beside `an_included_fragment_loads_identically_on_both_routes`. One test, both routes, three parts. (b) A root that includes nothing, with a leading byte-order mark, carries exactly `eval_snapshot_key(source).content_hash`, and the key of the mark-stripped text is a different number, so a hash taken after the strip cannot pass. (a) The same root over a fragment edited from `arity 2` to `arity 3` hashes differently, differs from the root's own key, hashes alike on the static route and the interpreter route for one fragment, and every command of one load, the root's and the fragment's, carries one value. A row the loader drops (an unresolvable fragment) folds nothing: the hash is the root's own key. Mutation checks: `pack_content_hash` answering the root's hash always fails (a); the same on the static route alone, and on the interpreter route alone, each fails (a); the root's hash taken after the strip fails (b). **SF3** D4.17's projection: `rust/tcl-spectcl/tests/codegen_stamps.rs` gains `a_site_compiled_through_the_service_stamps_the_bases_overlay_generation`. A bundled pack's registry (overlay generation `set.key`, asserted non-zero) is handed to `BytecodeCompileService::new` and compiled through `compile_for_profile` for `tcl9.0`, the seam `tcl-engine-tclvm`'s `with_registry` uses. The owned value handed to the service is `project_for_profile`'s own view of the cached registry: `CommandRegistry` is not `Clone` and `set_overlay` is crate-private, so a projection is the only owned form a pack registry with a generation has, and the service then projects it again for the profile. Both rungs: the alias site claims `BuiltinAlias { vendor::unpack → lassign, facts }` and a pack's `llength` fold claims `PackFacts(facts)`, each with `overlay_generation == set.key`, equal to the one stamp `PackSet::fact_stamps` gives. Mutation checks: `project_for_profile` not carrying `overlay` (the stamp reads `0`) fails it, and not carrying `pack_origins` (no claim at all) fails it. Gates: `cargo test -p tcl-spectcl --no-fail-fast` (lib 197; `eval_loader` 27, was 26; `codegen_stamps` 7, was 6; `workspace_packs` 10, `golden_packs` 3, `spec_corpus` 5 and every other binary); `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` clean; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8 sites (the gate exits 1 on them, as on the parent commit). Deviations: the SF2 test's file. |

**Step 4 is landed.** All four items above (CC4.1, CC4.2, CC4.3 and
CC4.4) are `landed`; the review checklist below is run against this tree
and its evidence recorded there. The plan's exit evidence holds. The
page's two witnesses: a refused stamp under the tier gate
(`workspace_packs.rs`'s `a_workspace_stamp_with_alias_of_is_refused_by_the_tier_gate`,
where CC4.2's item put it, and at the emitter `codegen_stamps.rs`'s
`a_workspace_stamp_is_refused_and_specialises_nothing`, which this landing
adds because the exit evidence places the refused stamp in that file: the
workspace-tier load keeps `alias_of`, drops the stamp, and the call
compiles to generic dispatch with no `lassign` identity and no claim, which
the VM runs as compiled), and an accepted one whose recorded identity the
VM's alias hop resolves (`the_vm_admits_it_through_the_alias_hop`).
`cargo test -p tcl-vm --test command_mutation_deopt_e2e` is green with the
rung-1 check (76). `spectcl_roundtrip` is green with `alias_of`
round-tripping (9, was 8) through `alias_of_survives_the_round_trip`, which
this landing adds: CC4.1's row said the whole-surface trips cover the
field, but no shipped spec declares it, so they never meet it — the gap
`arity_windows_survive_the_round_trip` closes for arity windows. The
landing also completes `codegen_stamps.rs`'s licence notice, which lacked
the full AGPL paragraph CONTRIBUTING.md asks for, and bumps
`registry_axes.rs`'s `LANDED` to hold `step 3` and `step 4` — D2.13's "a
lane landing a step or slice bumps it", which step 3's landing missed. No
waiver in the tree expires with either step; the gate's own fixtures that
used `until step 3` as a not-yet-landed expiry move to `until step 5`, as
CC2.15 moved them from `step 2` (`cargo test -p xtask registry_axes`, 9). Gates for the landing:
`cargo check --workspace --all-targets`, `cargo fmt --all -- --check`, and
clippy over the three crates it touches (`-p tcl-spectcl -p
tcl-spec-studio -p xtask --all-targets`) are clean; `cargo test -p tcl-spectcl --test
codegen_stamps` (6) and `-p tcl-spec-studio --test spectcl_roundtrip` (9)
pass; `registry-axes` (7831 / 16 / 36 / 893 across 147),
`value-transfers` (20 / 19 / 90 across 36, 6607 rows), `pack-goldens`
(25), `kcs-index-links`, `dialect-drift` (8), `retired-api-gate`,
`owner-resolution` (45), `callback-inventory --check`,
`audit-option-dialects --check` (114 probed options), `number-drift`,
`segmentation-drift` and `command-backing --check` (389 commands) are
green, and `verify-nextest-binary-shards.py --metadata-only` proves 328
targets. The step's KCS note,
`docs/kcs/kcs-qa-why-was-my-pack-codegen-hook-refused.md`, is indexed in
`docs/kcs/README.md` (CC4.2); CC4.3 and CC4.4 change nothing a user runs
(D4.20), so they add none. The plan drafts no landing message
for step 4, so this commit's follows step 3's landing. The title above, the
design page's status box, the two design indexes' lines
(`docs/design/README.md`, `docs/design/compiler/README.md`) and
`docs/design/lanes/README.md`'s in-flight line are updated in this commit
to read step 4 landed; step 5 (persisted guard identities, per-member
semantics keys, the Explorer record) has not started.

### Step 4 — review checklist, verified

The plan's § *Review checklist* (§ *Plan for steps 2–10* › *Step 4*), run
against the landed tree with evidence:

- **`alias_of` is the only source of a pack command's builtin identity;
  no code path reads `realm.rs`'s aliases to admit a site.** Outside tests
  and the Studio's surfaces, the field has one writer (the loader's
  `apply_command_stmt`, with `loader/eval.rs`'s `ROW_WORDS` entry) and two
  readers: `tcl_spectcl::stamps` (the rule's target) and
  `tcl_registry::codegen_stamp` (`ResolvedCall::stamp_identity`). Every
  `CommandBindingIdentity` construction in `rust/tcl-compiler/src` takes
  its identity from the registry: `stamp_identity` at the codegen stamps
  (`codegen/mod.rs`'s `stamped_binding`), the resolved spec's own name at
  lowering-hook and const-fold sites (`inline_lowering_hook`,
  `const_subst.rs`), the resolved invocation's `canonical_command` in
  `lowering_hooks.rs`, `lowering/hooks/control.rs` and `lowering/mod.rs`,
  and the literal `expr`. `grep -rn realm` over `codegen/`,
  `const_subst.rs`, `site_claims.rs`, `lowering_hooks.rs`,
  `codegen_stamp.rs` and `stamps.rs` finds only `stamps.rs`'s module doc
  saying that a realm alias is never a source.
- **A refusal drops the stamp and nothing else.** `cargo test -p
  tcl-spectcl --test workspace_packs` passes (9), including
  `a_refused_stamp_costs_no_analysis_fact`: the stripped spec's `Debug`
  rendering equals the loader's with only `codegen_hook` cleared, and the
  installed arity, `alias_of`, `Value` and `VarWrite` roles survive.
- **The identity recorded is the target's, never the pack command's own
  name.** `an_admitted_alias_stamp_records_the_targets_identity` passes:
  the module's `command_bindings` holds `vendor::unpack` → `lassign` and
  nothing with identity `vendor::unpack`, and its claim is `BuiltinAlias`
  with that binding. CC4.3's mutation check (the pack name recorded)
  fails it.
- **`content_hash` is the snapshot key's `u64`, named as such in
  `PackFactStamp`'s doc comment.** `rust/tcl-runtime-api/src/site_claim.rs`:
  "the `u64` xxh3 of its bytes, the value that file's `EvalSnapshotKey`
  interns, folded with every fragment an `include` row brought in". The
  root's value is `eval_snapshot_key`'s — `content_hash(source)`, taken
  before the byte-order mark is stripped; only a pack with `include` rows,
  which the snapshot cache never holds, folds its fragments in (D4.16).
- **Risks.** The clone-and-releak on refusal is memoised on the original
  spec's address and the drops (D4.10): a reload of an unchanged pack
  reuses the one clone, and only the packs the loader re-evaluates anyway
  — target-dependent ones, and ones with `include` rows — clone per load,
  at the rate they already leak. A module compiled with no pack claims
  nothing and admits: `a_rung_zero_module_is_admitted_under_a_changed_pack_set`
  passes, with no plain compile under no facts, one set's, or a changed
  set's.

### Step 4 — what the next steps read

- **For step 5.** The VM's admission predicate,
  `function_command_bindings_match`, is now bindings, procedure bindings
  and `site_claims_hold`. CC5.2 changes what `bump_cmd_epoch` clears; step
  4 never calls it — `set_pack_facts` advances `bump_trace_deopt_epoch`
  only — so pack facts leave the intrinsic guard table's lifetime as it
  was. `codegen_stamps.rs`'s harness (`PlainCounting`, a bundled `specs/`
  pack, the lowering → CFG → codegen pipeline) suits a VM row that must
  show admission rather than a result. The page's § *Codegen and the
  registry today* diagram row CC5.2 flips is untouched by step 4.
- **For step 6.** Rule 2's gate is `stamps::stamps_admitted_from`, the row
  the capability matrix takes over. Rule 4 is not built:
  `security_floor.rs` still protects only `codegen_hook` and
  `inline_codegen_hook`, and D4.13's conditional identity is what keeps an
  override's floored hook from being admitted for an unrelated `alias_of`
  target meanwhile. Once the compile service carries the overlay
  generation, `site_claims`'s stamp can read it there rather than from
  `CommandRegistry::overlay_generation` (D4.16).
- **For steps 7 and 8.** `SiteClaim` gains `ShippedImplementation`, with
  `IdentityKind` (D4.15), and `ReferenceBody`; `site_claims_hold` compares
  `claim.facts()` for every variant, so a new variant needs only its
  `facts()` arm. The manifest's `packs` is the deduplicated union of a
  unit's claims' facts. A lockfile integrity hash that is to equal
  `PackFactStamp::content_hash` must be the same per-file xxh3, folded over
  `include`d fragments (D4.16).
- **Reported, not fixed.** The VM does not create the namespace a
  qualified `interp alias` name lives in, as Tcl 8.4 to 9.1 do, and lacks
  the two-argument describe form (D4.14); an unscoped `-override` loses to a scoped shipped spec in every
  profile-aware lookup (D4.21); a stub declaring `{v:var script:body}` draws
  a false W210 (D3.22); `tcl-vm`'s `run_script` `encoding_command` test
  reads the host locale and fails under an empty `LANG`.

### CC4.1 — what the next items read

- **The field.** `CommandSpec::alias_of: Option<&'static str>` — `None`
  for a shipped command and for a pack command declaring no target; the
  loader's `apply_command_stmt` match arm and `ROW_WORDS` entry are the
  only two sites that write it outside the studio surfaces. Nothing reads
  it yet (D4.1): the catalogue's codegen-axis dispatch (`codegen_hook`,
  `inline_codegen_hook`, `semantic_operation Intrinsic(…)`) is unaffected,
  and `realm.rs`'s alias facts remain the only thing that infers an alias
  from script statements — a candidate the studio may one day seed a
  suggestion from, never a source `alias_of` reads or admits from.
- **For CC4.2 (the stamp rejection rule).** The registry sweep's own
  positive/negative fixtures
  (`CommandRegistry::build_default()` + `.insert(CommandSpec { alias_of:
  Some(…), ..CommandSpec::DEFAULT })`) are the pattern a
  `rust/tcl-spectcl/tests/workspace_packs.rs` test can reuse for a pack
  command that does or does not declare the stamped hook's own target.
  `leak_str` is how `apply_command_stmt` already turns a row's second word
  into the `&'static str` the field holds; `admit_codegen_stamps` (CC4.2's
  own, not yet written) reads `command.spec.alias_of` the same way a
  consumer reads any other `CommandSpec` field.
- **Not yet wired (residue, unaffected by this item).**
  `optimiser/elimination.rs`, GVN, `ssa.rs` and `memory_ssa.rs` still
  classify every command from the catalogue alone (D3.22, step 3);
  `alias_of` is a codegen-identity fact, not an analysis one, so nothing
  about that residue changes here.

### Behavioural deltas accepted in step 4

- CC4.2: a codegen-axis stamp (`codegen_hook`, `inline_codegen_hook`,
  `semantic_operation {Intrinsic …}`) in a user, workspace, or Spec Studio
  pack is dropped at load, with one warning on its command's row naming
  the provenance and the `alias_of` target it would have had to sit on;
  before, a `codegen_hook` loaded with a "names a codegen hook" warning and
  the other two loaded silently. A bundled pack's stamp is dropped the same
  way unless its command's `alias_of` names the shipped builtin carrying
  it (no shipped pack carries one). The command keeps every other fact.
- CC4.2: the loader's "names a codegen hook" notice is gone: the `upvar`
  port's golden loses it, and the corpus baseline trades it for the six
  refusals of the `return`, `string` and `upvar` ports.
- CC4.2: `spectcl_check` gains `stamp_refusals`, and the Spec Studio's
  store view gains `stamp_refusals` (and the same in its `patch` object);
  the Studio's installed world — its Test tab — drops the stamps a
  workspace load drops.
- CC4.2: in the Spec Studio, `alias_of` joins a "Builtin identity" field
  cluster with the three stamp fields, and its help text and worked
  example change; the generated field reference follows.
- CC4.3: a site specialised from a bundled pack's `alias_of` command
  records the target builtin's identity, so the VM admits it through the
  alias hop and runs it specialised; before, it recorded the pack name and
  every such site recompiled plain. No shipped command declares `alias_of`,
  so nothing else records a different identity.
- CC4.4: a compiled unit whose sites rest on a pack's facts — a constant a
  pack's `const_fold` computed, or a builtin reached through `alias_of` —
  claims them, and the VM admits it only while it holds the same facts
  (`Vm::set_pack_facts`): on a VM holding none, or holding a changed
  pack's, the unit recompiles plain. A unit claiming nothing admits as
  before. No production VM compiles against a pack, so nothing a user runs
  changes; the embedder's seam is `PackSet::fact_stamps`.
- CC4.4: a compile service's per-profile view of an embedder's registry
  (`project_for_profile`) keeps the base's overlay generation and pack
  origins, so its `AnalysisContextKey` carries the overlay generation
  where it carried none.

## Step 5 — progress

Item order follows § *Plan for steps 2–10* › *Step 5* § *Ordering and
checkpoints*: CC5.1 (sonnet) first, on its own; then the opus items, CC5.3
and CC5.2 last, each its own checkpoint.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC5.1 one semantics key per member | landed | `wip(consumer-contracts): step 5 — one semantics key per member` | `rust/tcl-registry/src/intrinsic.rs`: `const SEMANTICS_REVISION: [(IntrinsicId, u32); 28]`, one explicit row per member (every row `0`, matched by `stable_id`, never by declaration order; the array length is the catalogue's, so a member added without a row does not compile). `guard_semantics_key` is `semantics_key(release_variant(runtime), &SEMANTICS_REVISION)` — a key packs three disjoint fields, the member's own `stable_id` (bits 16–31), its revision (bits 2–15) and the release variant (bits 0–1, `RUNTIME_INVARIANT_SEMANTICS` for every member but `StringLength`, which keeps its three: 8.4/8.5 BMP, 8.6 UTF-16, 9.x scalar) (D5.2). `guard_semantics_variants` is an exhaustive per-member match with no wildcard, each arm a `const { &[…] }` block over the `member_keys!` macro, so it keeps its `&'static [u32]` signature and a new member must say which releases it is versioned across (D5.4); `revision_in` panics on a member with no row, at compile time through those blocks. The shared `INVARIANT_SEMANTICS` and `VERSIONED_STRING_SEMANTICS` slices are gone. `revision_in` and `semantics_key` take the revision table as a parameter so a test can bump one row of a copy. Every key is now non-zero, so every intrinsic's guard identity takes the packed form (the stable id in the high half of the identity's 64-bit value, the key in the low half), where a release-invariant member's used to be its bare stable id: each identity value moves once, and no persisted artefact holds one (D5.3). Tests (`intrinsic.rs`, 13 in the file, was 5): the plan's `every_member_has_a_distinct_semantics_key` (no key answered by two members on any of the five releases, no key on two members' variant lists, none zero) and `bumping_one_members_revision_moves_no_other_key` (for each of the 28 members, at revision 1 and at the field's maximum: every one of that member's keys moves, no other member's does), plus `the_revision_table_names_every_member_exactly_once`, `the_variants_are_exactly_the_keys_the_releases_answer` (pins `guard_semantics_variants` to `guard_semantics_key` across all five releases), `a_key_names_its_member_in_its_high_field`, `a_bumped_member_still_collides_with_no_other`, `a_revision_beyond_its_field_is_refused` (`should_panic`) and `string_length_answers_three_keys_across_five_releases`. The runtime and VM tests that call `guard_semantics_key` are unchanged and pass; the plan's "one literal `0x0306`-style assertion" does not exist as a key literal — the only one is `stable_ids_round_trip_without_ordinal_dependence`'s on `stable_id()`, which does not move (D5.5). Gates: `cargo test -p tcl-registry --no-fail-fast` (lib 941, was 933, and every binary), `-p tcl-vm --lib guard` (8), `-p tcl-compiler --lib mixed_region_plan` (8), and `runtime/rust`'s own `cargo test --lib -- guard intrinsic` (15; a standalone workspace, built with its own `target/`); `-p tcl-compiler --test wasm_tiers --test wasm_real_link` (6, 13) pass, but every real-link case skips loudly in this container — no `wasm32-wasip1` target, and no libtommath under the worktree's `tmp/` — so the identity check is covered by the runtime crate's `codegen_abi` tests and the planner's `mixed_region_plan` tests rather than by a linked module; `cargo check --workspace --all-targets`; clippy (`-p tcl-registry --all-targets --no-deps -- -D warnings`) and `cargo fmt -p tcl-registry` clean; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows — the value-transfers lane's own figures, moved by its merged commits), `pack-goldens --check` (25), `retired-api-gate`, `owner-resolution` (45), `kcs-index-links` green, `dialect-drift` at its 8. No KCS note: nothing a user runs changes. Docs: the design page's status box and its "intrinsic table splits by family" bullet, `docs/GLOSSARY.md` § *Guard identity*. D5.2–D5.5 |
| CC5.3 the Explorer observability record | landed | `wip(consumer-contracts): step 5 — the AOT decline record` | `select_native_i64_add_plan` returned `None` at the first premise that failed, so a compile that did not select the sealed native i64 addition fell to the generic or general plan and said nothing of why. The composition moves to `rust/tcl-compiler/src/codegen/wasm/native_add.rs` (new; the function and its six helpers leave `pipeline.rs`, D5.9) and is rebuilt as one derivation of selection and record: `native_add::select` evaluates every premise, records each one it finds wanting as a `NativeDecline`, and returns `Ok(selection)` only where none is rejected, so the two cannot disagree. **The types** are in `common_aot_plan.rs`, where the plan puts them: `NativePremise` (`SemanticPlans`, `Packaging`, `SealedProgram`, `Pass(id)`, `Coverage`, `DirectCall`, `DirectBody`, `ClosedProgram`, `Frame`, `Actuals`, `Boundary`, `NativeInteger`, `Operands`), `NativeDeclineReason` (27 variants; each wrapped typed reason keeps its own `as_str`) and `NativeDecline { premise, reason, sites }` (D5.7); `as_str` is added to the four typed declines that had none (`DirectProcBodyDecline`, `CommonAotCoverageDecline`, `ClosedProgramCoverageDecline`, `NativeIntegerDeclineReason`). **The plan** `WasmCodegenPlan::GenericInvoke` and `::General` gain `native_declines: Vec<NativeDecline>`, with the accessor `WasmCodegenPlan::native_declines()` (empty for `NativeI64Add`); `mixed_region_plan.rs` is untouched: the addition is a whole-program top-level selection that needs the unit, registry and options, which the region plan's builder does not have (D5.6). **Evaluation.** The options' premises (target plan policy, packaging, sealed environment, each of the five passes) are always evaluated; while none of the five passes is enabled the addition has not been asked for and no proof is built, so the record is those premises alone (D5.8: measured on a 300-statement, 150-procedure script in a debug build, `CommonAotProofPlan::build` costs 410 ms of `compile_wasm`'s 446 ms and about a fifth of the 1.9 s unit build, for every caller: `tcl compwasm`, the MCP tool, the fuzz harness). Once a pass is enabled the unit-level premises (excluded surfaces, the closed-program accounting) and, per direct call site, the direct call, its body, the closed-program shape, the frame's two flags, both actuals, the boundary, the integer proof (cached per callee) and the operands are each evaluated independently; a premise whose input another rejects is not evaluated. Entries dedupe on (premise, reason) and collect their sites, so the record is bounded by premises, not by calls; a unit with no call to its own procedure records `direct-call: no-direct-call`. **The Explorer.** `codegenPlan` gains `nativeDeclines` (`premise`, `reason`, `detail`, `sites`), and the plan the `wasm` header carries is repeated once, unchanged, as `data.aot`; the plan's "`aot` view" did not exist (the `wasm` text view prints only the WAT and function headers, `codegenPlan` was JSON-only, and `--show aot` matched no view), so `views.rs` gains the `aot` descriptor (`AOT Plan`, group `codegen`, `Tree`) and `view_tree.rs` its builder, which `render_all` and the TUI pick up from `tree_view_ids()` and the browser and editor panels show through their structured fallback (D5.10). `meta.views` is 36, was 35. Tests: `native_add.rs` (8) — `a_selected_addition_records_no_decline`, `an_addition_nobody_asked_for_records_the_options_premises_alone`, `a_hosted_compile_names_the_environment_the_passes_and_what_they_take_down` (five entries, the call named), `a_rejected_body_records_each_premise_it_takes_down` (the `native-integer` pass off: body, frame and the integer proof, each at the call), `a_surviving_statement_rejects_the_closed_program_and_the_actuals_it_hides`, `a_program_without_a_procedure_call_names_the_missing_call`, `a_second_call_to_the_callee_is_recorded_at_both_sites` and `the_target_and_its_packaging_are_premises_too`; the plan's `wasm_tiers.rs::a_failed_native_add_names_every_rejected_premise` (public API: the selected program rejects nothing, the same program hosted with two passes off names five premises and the call); the explorer text snapshots the plan asks for, `render.rs`'s `aot_text_lists_every_rejected_native_premise` (the six premises of an untouched Explorer) and `aot_text_follows_the_pass_selection` (seven, with the cascade a missing pass causes), `serialise.rs`'s `aot_view_carries_the_plan_and_every_rejected_native_premise` (the JSON, typed, with the site) and `view_tree.rs`'s `aot_view_names_a_selected_native_add`. Mutation checks: recording only the first rejected premise fails seven of the eight `native_add` tests (the selected case alone passes) and the `wasm_tiers` test; serialising `nativeDeclines` as `[]` fails both text snapshots and the JSON test. Moved: `serialise.rs`'s `meta_lists_all_dialects_views_and_severities` (36 views) and `wasm_view_exposes_common_native_i64_selection_evidence` (a selected plan's `nativeDeclines` is `[]`); every existing native add test passes unmodified (`pipeline.rs`'s four, the explorer's selection-evidence test, `wasm_codegen`, `wasm_execute`), and the real-link native rows skip loudly here (no `wasm32-wasip1` target), so the emitted module is covered by the pipeline tests that inspect its WAT. Gates: `cargo test -p tcl-compiler --no-fail-fast` (lib 6546, 2 ignored, was 6538; `wasm_tiers` 7, was 6; `wasm_real_link` 13, skipping loudly; `wasm_codegen` 45, `wasm_execute` 4, `codegen` 164, `codegen_integration` 17 and every other binary), `-p tcl-explorer` (lib 108, was 104), `-p tcl-cli --no-fail-fast` (lib 27, `cli` 50, `compile_verbs` 11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18, `value_transfers_cli` 8); `tcl-explorer-wasm` is outside the workspace (wasm32, its own lockfile) and reads only keys this item adds to; `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` clean, no new `#[allow]`; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged: the moved `ChannelWrite` and `Set` matches trip no site), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8 sites. Docs: `semantic-aot-optimisation.md` (the observability paragraph states the record as built), `wasm-explorer-view.md` (`nativeDeclines`, `data.aot` and the `aot` view), `kcs-feature-compiler-explorer.md` (an *AOT plan* section), the `compiler-explorer` skill's view catalogue. Deviations: the record sits on `WasmCodegenPlan`, not `MixedRegionPlan` (D5.6); `NativeDecline` gains `sites` (D5.7); the record is complete only once a pass is enabled (D5.8); the composition has its own module (D5.9); the `aot` view is created here (D5.10). D5.6–D5.10 |
| CC5.2 guard identities keyed by token generation | landed | `wip(consumer-contracts): step 5 — guard identities persist` | Both runtimes held a command's guard identities in a table keyed by its name, and every command-table mutation cleared it whole and moved the `CommandEnvironment`, `Namespace` and `UnknownHandling` domains (the VM's `bump_cmd_epoch`, the runtime's `invalidate_command_environment`, and the runtime's profile pin with it), so the first `proc`, `rename`, `interp alias`, `namespace import`, `interp hide` or `interp expose` after registration emptied the table for the interpreter's life: `prepare` answered `IdentityUnavailable`, every `check` answered false, and every guarded intrinsic of a linked module took generic dispatch from then on (the design page's "never repopulated" row). **The table** is `guarded_commands: HashMap<u64, BTreeSet<GuardIdentity>>` in the VM (the generation of `command_identity.generations`, read through `visible_command_generation`) and `BTreeMap<u64, BTreeSet<GuardIdentity>>` in the runtime (`Namespaces::resolve_generation`): a command's identities sit under its token generation, which both runtimes already carry through a rename and a hide and replace on a rebinding (D5.11). **Registration** goes through one funnel per runtime, `register_attested` (VM) and `bind_attested_builtin` (runtime): bind, attest the generation bound and drop the displaced generation's entry; `register_guarded_builtin` and `register_spec_builtin` use it, the VM's `register` returns the storage key it bound at, and the runtime's `register_builtin` shares a `bind_builtin` that returns the generation. **The read** is `attested_identities(name)` in each: it resolves the guarded name afresh, from the current namespace, to a key and a generation, requires the pinned surface to admit the command (`command_visible_for_surface_at` in the runtime, `resolve_command_fqn`'s filter in the VM) and answers the entry at that generation. `prepare_command_guard`, `check_command_guard` and the runtime's `check_command_guard_identity` (the second half of the ABI's `tcl_codegen_guard_check`, which recomputes the identity from the runtime version and re-resolves the argv head on every check) all read through it, so a guard follows its command through rename and hide, stops at a different command at the name, and a `proc string` in a namespace drops it for the calls made from that namespace alone. **What moves the domains** (D5.12). A command-table mutation moves none: the VM's `bump_cmd_epoch` keeps its resolution-memo clear and its counter and loses both guard effects; the runtime loses eleven `invalidate_command_environment` calls (the pin, `move_bound_command`, `delete_bound_command`, `delete_command`, `create_ensemble`, procedure definition, `bind_command_replacement`, the two OO command-retirement paths, hide and expose) and keeps eight (`namespaces_mut`, `ensure_namespace`, `ensure_command_owned_namespace`, `ensure_global_namespace`, `delete_namespace_by_id`, `create_child`, `with_child`, `delete_child`); the VM's new `invalidate_lookup_guards` is called from `ns_path_set`, `mark_interp_trusted`, `make_safe` and `delete_interp`. The Interpreter and ObjectDispatch domains and the trace domains are untouched, so a renamed or traced intrinsic still falls back; `tcl-runtime-api` is untouched (D5.13). **The pin** keeps the table in both runtimes, and the surface decides what a guard reaches (D5.14). Tests, in the runtime: `an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it` re-states `command_mutation_invalidates_guard_and_identity_attestation`, whose one assertion, that an unrelated registration stales the guard, is the behaviour the item removes — an unrelated `proc`, `rename` and `interp alias` keep the guard and a fresh `prepare`; `namespace eval other {namespace path ::}` stales the token and leaves the attestation; the pin keeps a `CommandEnvironment`-only token and `prepare`; `rename guarded moved` drops the guard at `guarded`, where `prepare` answers `IdentityUnavailable`, and `prepare("moved")` succeeds; renaming back restores the guard; hide drops it and expose restores it; a `proc guarded` drops it for good; `a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace` (the plan's risk row: a `proc guarded` in `ns` leaves the guard at `::`, drops it with the current namespace set to `ns` and restores it back at `::`); `an_unrelated_mutation_keeps_the_string_length_guard` (the spec-registered `string length` over the registry's base domains, through `check_command_guard_identity`: an unrelated `proc`, `rename` and alias keep it, `rename string moved` drops it, `rename moved string` restores it, `proc string` drops it); `a_pin_to_a_release_without_the_command_leaves_it_unattested` (a guarded builtin named `lassign`, a command Tcl 8.4 lacks, under 8.4 and back); and `codegen_abi.rs`'s `guarded_intrinsic_guards_survive_unrelated_command_mutation`, which makes the ABI call an emitted module makes (`tcl_codegen_guard_prepare`, `tcl_codegen_guard_check`) over the base domains: three unrelated scripts leave the check at 1, `proc string` moves it to 0. In the VM's `family_b_tests` (D5.16): the restated test and the shadowing test under the same names (the `interp` verbs run through `eval_value`, and a probe builtin checks the token from wherever it runs for the shadowing row), `a_profile_pin_keeps_the_spec_registered_string_attested`, `a_pin_to_a_release_without_the_command_leaves_it_unattested`, and the rename test gains its `prepare` assertion. The real-link case gains the plan's "unrelated proc keeps the fast path" row (D5.15). **Mutation checks.** In the runtime, `bind_command_replacement` moving the command domains again fails four of the 18 guard and pin tests — `guarded_intrinsic_guards_survive_unrelated_command_mutation`, `an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it`, `an_unrelated_mutation_keeps_the_string_length_guard` and the shadowing test; resolving the guarded name from `::` alone fails the shadowing test only; dropping the surface gate from the read fails the pin test only. In the VM, `bump_cmd_epoch` calling `invalidate_lookup_guards` fails the restated test and the shadowing test (2 of the 37 in `family_b_tests`); resolving from the empty namespace key alone fails the shadowing test only; reading through `resolve_command_fqn_raw`, which skips the surface filter, fails the pin test only; and `bump_cmd_epoch` clearing the table again fails the restated test, the shadowing test and both pin tests (4 of 37). **Moved.** `renaming_spec_registered_string_stales_its_intrinsic_guard`'s second assertion in the runtime, that the guard at the new name `moved` fails, contradicts D5.11, since the token at `moved` is the one the rename carried, and is replaced by a `prepare("string", …)` that answers `IdentityUnavailable`; the VM's twin keeps both assertions (its manual take and `register_command` mint a new generation) and gains the same `prepare` one; the runtime helper `assert_interpreter_guard_stale` asserts the attestation is present instead of re-registering the builtin, which only the old clearing made necessary. Gates: `cargo test -p tcl-vm --lib` (100, was 97), `-p tcl-vm --test command_mutation_deopt_e2e` (76, unchanged), `-p tcl-vm` in full under `LANG=C.UTF-8` (50 binaries, 1477 tests; `expanded_invoke_e2e` needs the `tmp` oracle tree, which was linked for its run); `runtime/rust`'s `cargo test --locked --lib` (707, was 703) and `--tests` (12 integration binaries, 160 tests), its own workspace, built with `TCL_TOMMATH_DIR`, under `LANG=C.UTF-8` (under an empty `LANG`, `cmd_misc.rs`'s `encoding_ensemble_resolves_like_tclsh` fails as `tcl-vm`'s `encoding_command` test does, issue #2271); `-p tcl-runtime-api` (29, unchanged); `-p tcl-compiler --test wasm_real_link --test wasm_tiers` (13 and 7, with `TCL_REQUIRE_WASM_LINK=1`, run for real: `wasm32-wasip1`, wasi-sdk 34.0 and wasmtime were installed for this item, so the 13 real-link cases execute and pass against the changed runtime, where CC5.3's row recorded them skipping); `-p tcl-registry` (21 binaries, 1276 tests), `-p tcl-explorer` (108); `cargo check --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` clean, and the runtime's own `cargo clippy --all-targets -- -D warnings` and `cargo fmt`, no new `#[allow]`; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8 sites. **Fuzz.** The plan's exit campaign: `tcl-fuzz run --subject runtime-rust --reference tclvm`, both engines built from this tree (`tclvm` debug, `runtime/rust`'s `run_script` release with `TCL_TOMMATH_DIR`) and hard-linked out of the build tree so a rebuild could not change them mid-run, one release at a time, findings in a scratch directory. Release 9.0: seed 5210000, 3000 iterations, 2998 matched, 2 findings, 1096 s. Release 8.6: seed 5310000, 800 iterations, 800 matched, no finding, 324 s. 23 minutes 40 seconds in all. The two 9.0 findings (seeds 5210167 and 5211533, both `stdout_mismatch`) are one `tclvm` compiler defect and no guard: `proc p {} {return \101}` answers `\101` where C Tcl 9.0 and `runtime/rust` answer `A`, because the compiler pushes `return`'s value word as source text when it holds a backslash escape and no `$` or `[` (`tcl dis` shows `push1 "\101"`; `emit_value`'s default arm in `rust/tcl-compiler/src/codegen/cmd_subst.rs`, reached from `emit_proc_return`). With the escaped word decoded by hand in each script, `tclvm`, `runtime/rust` and C Tcl agree, and `runtime/rust` matched C Tcl on both scripts as they stand; the code involved is unchanged since this step began and no guard is in its path. So the campaign is not finding-free and neither finding is this item's: it is reported, not fixed (*CC5.2 — what the next items read*). Docs: `rename-alias.md` § 3.5 (a rename changes what a guard's name resolves to and no longer clears anything), `tclvm-opcode-status.md` note 5, `value-transfers-review.md` R12, the design page (the guard-eligibility bullet, the diagram's "never repopulated" row, the paragraph after it, an *Intrinsic guard identities persist* bullet and the test anchor), `docs/GLOSSARY.md` § *Guard identity*. Deviations: keyed by generation and read live, not a name-keyed entry that moves (D5.11); the domains keep the lookup events only (D5.12); no `GuardDomain::CommandToken` (D5.13); the pin keeps the table and the read filters (D5.14); the real-link row is output-equal to the fallback (D5.15); the VM twin sits in `family_b_tests` (D5.16). D5.11–D5.16 |
| Step 5 review fixes | landed | `wip(consumer-contracts): step 5 — review fixes` | The step 5 review ("land with fixes"): two statements that the code no longer bears out, and two notes. No behaviour changes; the one Rust file that changes changes in its doc comments only. **S1** the guard-domain text after D5.12. Measured from the code first: the runtime's eight `invalidate_command_environment` sites (`namespaces_mut`, the three `ensure_*namespace` funnels, `delete_namespace_by_id`, `create_child`, `with_child`, `delete_child`) and the VM's four `invalidate_lookup_guards` sites (`ns_path_set`, `mark_interp_trusted`, `make_safe`, `delete_interp`); then by a probe in `runtime/rust`'s own test module — a token over each lookup domain prepared before, and checked after, each of 25 scripts; the test was added, run and reverted, and nothing of it is committed. The probe reproduced the reviewer's nine results and added the cases they lacked. The runtime moves `CommandEnvironment`, `Namespace` and `UnknownHandling` together, never one alone, on `namespace path`, `namespace export`, `namespace unknown`, `namespace delete`, a `namespace forget` that removes an imported command, `oo::class create`, `interp create` and `interp delete` (and, by the code, `oo::copy`'s namespace clone and an `interp invokehidden -namespace` that names a new namespace); it leaves them alone for a `namespace eval` that creates a namespace, `namespace import`, `interp alias`, `proc`, `rename`, and binding, renaming or deleting `unknown`. The VM moves them on `namespace path`, `interp marktrusted`, making an interpreter safe and `interp delete`, and on none of the rest, so it misses `namespace delete`, `namespace export` and `namespace unknown`; that gap is older than D5.12 and is issue #2292, and neither runtime's invalidation is widened here. `CommandEnvironment` moves on no definition, rename, alias or import in either. The reviewer's `namespace forget ::guarded` left the token valid because it removed nothing (below); a forget that removes an import moves the three. The fixes: `rust/tcl-runtime-api/src/guard.rs`'s `GuardDomain` doc comments name the events and the runtime difference and drop "imports"; the design page's guard paragraph (the sentence that said a `CommandEnvironment` guard "depends on its command's token and on no other", and the one that listed the events) and `rename-alias.md` § 3.5 say the same; `wasm-native-lowering-plan.md`'s runtime-limits item 1, which called the `CommandEnvironment` epoch "exactly the validation a direct-call handle would need", now names the token generation and the lookup epochs, since a rebinding no longer moves the epoch (a third instance, not named by the review); D5.12 is amended in place and *CC5.2 — what the next items read* says the same, citing #2292. **S2** `constant-folding-type-inference.md` named `selected_closed_native_coverage` in `pipeline.rs` as the WASM pipeline's one read of a `LatticeValue`; that function left with the composition (D5.9), and the read is `lattice_i64`, reached from `covered_shape`, in `codegen/wasm/native_add.rs`, the only `LatticeValue` match under `codegen/`. **N1** this table's CC5.3 row said `NativeDeclineReason` has 26 variants; `common_aot_plan.rs` declares 27. **N2** the two fuzz notes (*Step 5 — what the next steps read* and *CC5.2 — what the next items read*) now say that the `return` defect does not depend on the release (it reproduces at 8.6 and at 9.0), that the quoted form `return "\101"` answers `\101` too, and that it is issue #2291. **Reported, not fixed** `runtime/rust`'s `namespace forget` of a command imported from the global namespace removes nothing (`ns_forget` matches against `::::name`; tclsh 8.6 and 9.0 remove it), found by the S1 probe and recorded with its reproduction under *Step 5 — what the next steps read*. Gates: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean (no test suite ran: no code changed, only the doc comments of one file), and `cargo doc -p tcl-runtime-api --no-deps` warns once, on an unresolved `GuardToken` link in the crate's opening paragraph, which the edit did not touch; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8 sites (the gate exits 1 on them, as on the parent commit) and the shard-manifest verifier (329 targets). Deviations: the `wasm-native-lowering-plan.md` sentence is beyond the four items the review named, and D5.12 is amended in place rather than superseded. |

**Step 5 is landed.** All three items above (CC5.1, CC5.3 and CC5.2) are
`landed`; the review checklist below is run against this tree and its evidence
recorded there. The plan's exit evidence holds, with one qualification.
`command_mutation_invalidates_guard_and_identity_attestation` is re-stated as
`an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it`
(`runtime/rust/src/interp.rs`), and its VM twin,
`any_command_mutation_invalidates_guard_and_live_identity_attestation`, under
the same name (`rust/tcl-vm/src/interp.rs`, D5.16); both are green.
`guarded_boxed_intrinsic_runs_and_falls_back_against_the_real_runtime` gains
the "unrelated proc keeps the fast path" row and, unlike at CC5.3, runs for
real: `wasm32-wasip1`, wasi-sdk 34.0 and wasmtime are installed here, so the 13
real-link cases execute against the changed runtime (D5.15 says what the new
row can and cannot show). The qualification is the fuzz exit. The campaign over
the `tclvm`/`runtime-rust` pair ran — release 9.0, 3000 iterations from seed
5210000; release 8.6, 800 from seed 5310000; 23 minutes 40 seconds of wall
clock — and found two divergences at 9.0 and none at 8.6. Both (seeds 5210167
and 5211533) are one `tclvm` compiler defect in `return`'s value word, which
the runtime under test does not share, and neither involves a guard; so "no
new finding" holds for what this step changed and not for the pair. The defect
is reported under *what the next steps read* and is not fixed here.

The landing commit also changes the following. It appends `"step 5"` to
`registry_axes.rs`'s `LANDED` — D2.13's "a lane landing a step or slice bumps
it". No waiver in the tree says `until step 5`, so none expires and none needs
resolving; the gate's own fixtures that used `until step 5` as a not-yet-landed
expiry (five spellings in three tests) move to `until step 6`, as CC4's landing
moved `until step 3` to `until step 5` (`cargo test -p xtask registry_axes`, 9; the crate's 237 pass),
and `docs/generated/registry-axes.md` regenerates unchanged. It rewrites the
design page's status box as a description of what is built and of what is not,
and the two design indexes' entries for the page likewise: the repository
owner's standing rule for every page outside `docs/design/lanes/` is current
state only, with no step, item or decision numbers, so the boxes that said
"built in step 2", "step 4 builds" and "still proposed" now name what exists and
what does not, and read the value-transfers names as the types they have become
(`AnalysisContext`, `AnalysisInputs`, `PlanAnswer`, `OperandId`,
`TemplateWordPlan`, `EvalAnswer` and `HandlerPlan` are in
`tcl_registry::value_transfer`). Other passages of the page that say "(step 4)"
or "step 8's" are untouched and still carry that narrative; they are for the
owner's sweep. It updates the title above and `docs/design/lanes/README.md`'s
bullet. Gates for the landing: `cargo check --workspace --all-targets`, `cargo
fmt --all -- --check` and clippy over the crate it touches (`-p xtask
--all-targets`) are clean; `registry-axes --check` (7831 / 16 / 36 / 893 across
147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows,
unchanged), `pack-goldens --check` (25), `command-backing --check` (389),
`retired-api-gate`, `owner-resolution` (45), `kcs-index-links`,
`callback-inventory --check`, `audit-option-dialects --check` (114 probed
options), `number-drift` and `segmentation-drift` are green, `dialect-drift` is
at its 8 sites, and `verify-nextest-binary-shards.py --partition-count 5
--metadata-only` proves 328 targets (no test binary was added). The plan drafts
no landing message for step 5, so this commit's follows step 4's. Steps 6 and 7
are in progress, each on its own items.

### Step 5 — review checklist, verified

The plan's § *Review checklist* (§ *Plan for steps 2–10* › *Step 5*), run
against the landed tree with evidence:

- **Both runtimes change together, and the fuzz campaign is recorded with its
  seed count and release.** CC5.2's commit changes `runtime/rust/src/interp.rs`
  and `rust/tcl-vm/src/interp.rs` together, with a test pair in each and the
  ABI's own call checked in `runtime/rust/src/codegen_abi.rs`; the campaign's
  releases, seeds, iteration counts, wall clock and findings are in CC5.2's row.
- **A persisted identity is invalidated when its dependency changes: the
  `rename` and `trace` rows still fall back.** `wasm_real_link.rs`'s
  `renamed fallback` (`proc string` answers 99) and `traced fallback` rows run
  for real and pass; `renaming_spec_registered_string_stales_its_intrinsic_guard`,
  `command_trace_stales_the_string_intrinsic_guard` and the shadowing, rename,
  hide and pin rows of `an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it`
  hold it in both runtimes, and each of the seven mutations of the read or of the
  domains that CC5.2's row lists fails the rows that name it.
- **No identity is attached by name after registration.** `guarded_commands` has
  one writer in each runtime, `register_attested` (VM) and `bind_attested_builtin`
  (runtime), both inside registration, and one reader, `attested_identities`;
  `git grep guarded_commands` in the two interpreters finds no other use. CC5.2
  adds no attach path (D5.11).
- **Risk: a guard surviving a shadowing `proc string` for calls made from a
  namespace.** `a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace`,
  in both runtimes, asserts it: the read resolves the name from the calling
  namespace to the token generation it reaches, and a mutation resolving from
  the global namespace alone fails it in each.
- **CC5.1's own checks.** `guard_semantics_key` is one key per member, and
  `every_member_has_a_distinct_semantics_key` and
  `bumping_one_members_revision_moves_no_other_key` hold in `intrinsic.rs`
  (CC5.1's row); nothing in CC5.2 or CC5.3 touches it.

### Step 5 — what the next steps read

- **For step 6.** The compile service's one route to a registry carrying an
  overlay today is `BytecodeCompileService::new(registry.project_for_profile(profile))`:
  `project_for_profile` keeps the base's `overlay` and `pack_origins`, which is
  what `codegen_stamps.rs`'s
  `a_site_compiled_through_the_service_stamps_the_bases_overlay_generation` (step
  4's review fix, SF3) drives; `CommandRegistry::set_overlay` is `pub(crate)` and
  the registry is not `Clone`, so CC6.3's `RegistryTarget::Overlay` replaces that
  route rather than wrapping it. `BytecodeCompileService::new` builds its
  `config` with `LexerConfig::default()` (`compile_service.rs:165`), one of the
  eight `dialect-drift` sites, and stays that while the gate's baseline is 8.
  CC6.2's capability matrix gives a `Transitive` or `Development` pack nothing,
  which should include `runtime_backing` (CC7.1's field) beside `alias_of`, the
  stamps and a reference body; CC7.1's floor already keeps a shipped command's
  backing against an override from any tier, so the matrix only has to refuse
  what a pack *adds*.
- **For step 7.** CC7.2's attach path, the pin and the table's lifetime are in
  *CC5.2 — what the next items read*; CC7.3 raises a member's
  `SEMANTICS_REVISION` row when a family split moves its guard domain or trace
  behaviour, and CC7.4's `intrinsic_table_hash` can hash `(stable_id,
  guard_semantics_variants())` over `IntrinsicId::ALL` (*CC5.1 — what the next
  items read*). CC7.5's fuzz exit should replay seeds 5210167 and 5211533 first
  (`tcl-fuzz replay SEED --subject runtime-rust --reference tclvm --tcl-version
  9.0`): they diverge until the `return` defect below (issue #2291) is fixed,
  and a campaign that reports them again is the known pair, not a regression.
- **Real-link tests run here.** `wasm32-wasip1` (rustup), wasi-sdk 34.0 at
  `/opt/wasi-sdk` and wasmtime are installed, so `wasm_real_link` (13) and
  `wasm_tiers` (7) execute; `TCL_REQUIRE_WASM_LINK=1` turns a missing toolchain
  into a failure rather than a loud skip; the runtime's bignums need the Tcl
  9.0.4 source under `tmp/` or `TCL_TOMMATH_DIR`. An item that changes the
  runtime's guard, ABI or codegen contract should run them.
- **Reported, not fixed.** `tclvm` prints `\101` for `proc p {} {return \101};
  puts [p]`, where C Tcl 9.0 and `runtime/rust` print `A`, and likewise `\{\}`,
  `\x41` and `\n` (a length of 2): `emit_proc_return`
  (`rust/tcl-compiler/src/codegen/emitter/terminator.rs`) hands the value word to
  `emit_value`, whose default arm (`codegen/cmd_subst.rs`, `push_word_value`)
  pushes the source text when the word has no `$` or `[` and no escaped marker, so
  a backslash escape is never decoded; `tcl dis` shows `push1 "\101"`. The
  quoted form `return "\101"` answers `\101` too, and the defect does not
  depend on the release: it reproduces at 8.6 and at 9.0. It is the fuzz
  campaign's two findings, filed as issue #2291, and independent of the guard
  tables. The runtime crate's `cmd_misc.rs:210`
  (`encoding_ensemble_resolves_like_tclsh`) reads the host locale and fails under
  an empty `LANG`, as `tcl-vm`'s `encoding_command` test does (issue #2271);
  `scripts/dev/ensure-test-deps.sh:982` still says `wasi-sdk-25` in a comment
  where the pin is 34.0. The runtime's
  `namespace forget` of a command imported from the global namespace removes
  nothing: `ns_forget` (`runtime/rust/src/cmd_namespace.rs`, ~491) joins the
  source namespace's qualified name, `::` for the global one, to `::` and the
  tail, so it matches imports against `::::name` and finds none. `proc g {};
  namespace eval :: {namespace export g}; namespace eval n {namespace import
  ::g}; namespace eval n {namespace forget ::g}; info commands n::g` answers
  `::n::g` where tclsh 8.6 and 9.0 answer nothing. Found by the step 5 review
  fixes' probe; a forget from any other namespace removes the import.

### CC5.1 — what the next items read

- **For CC5.2.** The identity values are opaque to invalidation: both
  runtimes still build a command's identity set in `register_spec_builtin`
  from `guard_semantics_variants()`, and the guard check still compares
  `GuardIdentity` values, so a per-token guard table keys on whatever the
  set holds. Within one build an identity moves only when a member's
  `SEMANTICS_REVISION` row rises.
- **For CC7.3.** A family split that changes a member's guard domain or
  trace behaviour is a change to what a fast path may assume, so it raises
  that member's row; the keys of every other member stay put.
- **For CC7.4.** `intrinsic_table_hash` can hash `(stable_id,
  guard_semantics_variants())` over `IntrinsicId::ALL`: every revision bump
  moves it, and the accessors are already public. The tests' `bumped` and
  `keys_under` helpers show the shape of a table-parameterised check.

### CC5.3 — what the next items read

- **For CC5.2.** Nothing: the record concerns the AOT plan, and CC5.2's
  identities are the runtimes' guard tables.
- **For widening native selection (CC7.3 and later).** A new consumer of a
  common proof adds its premises where the addition's are: a `NativePremise`
  and the reasons it can be rejected for in `common_aot_plan.rs`, and an
  evaluation in `native_add.rs` that records the rejection instead of
  returning early. `native_add::select` returning `Ok` is the only selection,
  so a premise that is evaluated but not recorded is a bug the record's tests
  catch by name. The Explorer needs no change for a new premise: `aot` prints
  whatever `nativeDeclines` holds.
- **For the Explorer.** `data.aot` is the `wasm` header's plan; an options
  path that compiles sealed (the Explorer compiles hosted) would show
  `native i64 add: selected` there, a branch `aot_view_names_a_selected_native_add`
  covers with hand-built JSON.

### CC5.2 — what the next items read

- **For CC7.2.** The table is keyed by generation, so the sweep's attach is
  "resolve the name to its generation and insert under it"
  (`Namespaces::resolve_generation` in the runtime, `visible_command_generation`
  in the VM), and it must attest only a generation that is still the shipped
  builtin (the runtime's `registry_builtin_names`, by generation, and the VM's
  `builtin_identities`, by name, moving with a rename, record which), or a name a script has since
  redefined would gain an attestation for a `proc`. Registration attests through
  `register_attested` and `bind_attested_builtin`, which bind, attest the
  generation bound and drop the displaced one; the sweep may call them or do the
  same three things. The pin no longer empties the table (D5.14), so a sweep run
  again at the pin has nothing to restore: the surface filter already hides what
  the pinned release lacks.
- **For CC7.3 and CC7.4.** Nothing changes for them: the identity values,
  `guard_semantics_variants` and the ABI's guard functions are as CC5.1 left
  them, and a change to a member's guarded contract still raises its
  `SEMANTICS_REVISION` row, which moves its identity. A module compiled before
  this item links and runs unchanged (D5.13).
- **For a guard family added later.** A guard is valid while its token's domains
  have not moved *and* the name it guards resolves, from the namespace the call
  is made in, to a command attested for the guard's identity. A family whose
  guard depends on a command being one particular builtin is covered by the
  second condition at no cost. A family that depends on a command's *absence*
  has no identity to attest and is not covered; it needs a domain (the
  `GuardDomain` set has no per-command variant, D5.13). A family that depends on
  how names resolve, not on what a name is bound to, takes the lookup domains,
  which move together and only on the events D5.12 lists as amended: in the
  runtime `namespace path`, `namespace export`, `namespace unknown`,
  `namespace delete`, a `namespace forget` that removes an import, `TclOO`
  class and object creation and child interpreter creation and deletion; in
  the VM `namespace path`, `interp marktrusted`, making an interpreter safe and
  `interp delete` alone (the gap is issue #2292). A family that relies on any
  other event has no domain to take.
- **For the Explorer and the AOT record.** Nothing: the record concerns the
  plan, and the guard tables are the runtimes'.
- **Reported, not fixed.** `tclvm` prints `\101` for `proc p {} {return \101};
  puts [p]`, where C Tcl 9.0 and the runtime print `A`: `return`'s value word
  is pushed as source text when it holds a backslash escape and no `$` or `[`
  (`emit_value`'s default arm, `rust/tcl-compiler/src/codegen/cmd_subst.rs`, via
  `emit_proc_return` in `codegen/emitter/terminator.rs`). The quoted form
  `return "\101"` answers `\101` too, and the defect does not depend on the
  release: it reproduces at 8.6 and at 9.0. Found by this item's fuzz campaign
  (release 9.0, seeds 5210167 and 5211533), filed as issue #2291; it is
  independent of the guard tables.

### Behavioural deltas accepted in step 5

- CC5.1: every intrinsic's guard identity value changes once, because
  every key is now non-zero and a non-zero key is packed beside the stable
  id (D5.3). Both runtimes and the WASM planner derive the value through
  `guard_semantics_key`, so nothing a user runs differs; a module compiled
  by the previous compiler and linked against a runtime from this tree
  fails the identity check and takes generic dispatch.
- CC5.3: a compile that does not select the sealed native i64 addition
  records every premise it rejected on the plan (`WasmCodegenPlan::native_declines`,
  `codegenPlan.nativeDeclines`), where it recorded nothing; while none of the
  five passes the addition consumes is enabled, the record is the options'
  premises alone. `compile_wasm` builds the common proof plan when any one of
  the five is enabled, where it built it only with all five enabled, sealed and
  not standalone; a compile enabling one to four passes pays for the proofs.
- CC5.3: the Explorer gains an `aot` view (`AOT Plan`; `meta.views` is 36)
  and the `aot` payload, and `tcl explore --show aot --text` prints the plan
  and the rejected premises. `WasmCodegenPlan::GenericInvoke` and `::General`
  gain a field, which no pattern in the tree spelled out.
- CC5.2: defining, renaming, deleting, aliasing, importing, hiding or exposing
  a command other than the guarded one no longer stales a guard or empties the
  attestation table in either runtime, so `string length` and every other
  guarded intrinsic of a linked module keeps its fast path after a `proc foo`,
  where every guard failed for the rest of the interpreter's life. A guard
  follows its command through `rename` and `interp hide`: renaming the guarded
  command away drops the guard at its name, and renaming it back, or exposing
  it, restores it, where the table was never refilled. Replacing the command,
  or shadowing it by a definition in a namespace, drops the guard for the calls
  that reach the other command and for those alone.
- CC5.2: the profile pin no longer clears the attestation table in either
  runtime, and the runtime's pin no longer moves `CommandEnvironment`,
  `Namespace` or `UnknownHandling`; a VM pinned at construction can issue a
  guard, where it held none. A token over the `Interpreter` domain still goes
  stale at a pin, and a guard on a command the pinned release lacks answers
  false through the surface filter.
- CC5.2: the VM's `bump_cmd_epoch` no longer moves guard domains (it clears
  the resolution memo and advances the counter); `namespace path`,
  `interp marktrusted`, making an interpreter safe and deleting one move the
  lookup domains through `invalidate_lookup_guards`. No script observes any
  of this: the guards steer emitted code only, the ABI, the identity values and
  `tcl-runtime-api` are unchanged, and a module the previous compiler emitted
  links and runs.

## Step 6 — progress

Item order follows § *Plan for steps 2–10* › *Step 6* § *Ordering and
checkpoints*: CC6.1 (sonnet) first, on its own; then the opus items, CC6.2
and CC6.3, each its own checkpoint.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC6.1 the floor widens | landed | `wip(consumer-contracts): step 6 — the floor widens` | `rust/tcl-registry/src/security_floor.rs`: `SecurityFloor::apply` takes `lowering_hook`, `analyser_hook`, `semantic_operation`, `state_transitions`, `native_lowering` and `bpf_op` from the shipped command beside the two codegen hooks it already kept, through the same `take_shipped` (the shipped value wins wherever the shipped command has one; all six types are `Copy`, so no new merge shape), `MERGED_FIELDS` lists them, and `every_security_bearing_field_is_in_the_floor` names the six explicitly in a `matches!` beside its `taint` / `codegen` / `side_effect` / `credential` filter, since none of their names carries those words. The module and `apply` docs state the codegen and dispatch axis as a contract about the closed catalogues, not a trust gate on analysis facts (D6.4). Tests: `rust/tcl-spectcl/tests/i6_security_floor.rs` gains the plan's six rows, each first proving the override took effect (its own `arity 7..9` window installed, so what survives of the shipped spec is the floor's doing) — `a_workspace_override_cannot_swap_the_lowering_hook` (`while`'s `While` against `lowering_hook -native If`), `…_swap_the_analyser_hook` (`source`'s `Source` against `Rename`), `…_swap_the_semantic_operation` (`puts`'s `Intrinsic(ChannelWrite)` against `Invoke`), `…_swap_the_state_transitions` (`join`'s descriptor against a restated one, compared by `Debug` since the type has no `PartialEq`), `…_drop_the_native_lowering` (`break`) and `…_drop_the_bpf_op` (the `bpf` dialect's `pass`); the last two fields have no loader statement, so their rows lose them the only way a pack can, by replacing the command and saying nothing (D6.5). Two registry unit tests: `the_floor_takes_the_shipped_codegen_and_dispatch_axis` (all six, on a hand-built shipped spec) and `the_floor_adds_nothing_the_shipped_command_lacks` (the negative: a shipped command with none of the six leaves an override's own value alone, the registry-level twin of the unchanged `the_floor_does_not_invent_facts_for_a_new_command`). A mutation check — the six `take_shipped` lines commented out — fails all six integration rows and the unit test, and dropping `"bpf_op"` from `MERGED_FIELDS` fails the field scan. Nothing existing moved: no shipped or bundled pack overrides a command, and every test that installs an override passes unmodified. Gates: `cargo test -p tcl-spectcl --test i6_security_floor` (8, was 2), `-p tcl-registry` (lib 943, was 941, and every binary) and `-p tcl-spectcl` (lib 192 and every binary, `workspace_packs`, `codegen_stamps`, `golden_packs` included), `-p tcl-spec-studio` (lib 199 and every binary) and `-p tcl-mcp` (114); `cargo check --workspace --all-targets`; clippy (`-p tcl-registry -p tcl-spectcl --all-targets --no-deps -- -D warnings`) and `cargo fmt` clean; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows), `pack-goldens --check` (25), `retired-api-gate`, `owner-resolution` (45), `kcs-index-links` green, `dialect-drift` at its 8. Docs: `spec-packs.md` § *Workspace trust* floor sentence, the design page's rule 4 (now built for the six, `runtime_backing` step 7's) with its status box, the BPF bullet's tense and the anchors, and the pack howto `docs/kcs/kcs-howto-write-a-tclspec-pack.md` (an override keeps the shipped taint facts and compiler-side identity, silently). D6.4, D6.5 |
| CC6.2 `DependencyTier` and `CodegenCapability` | landed | `wip(consumer-contracts): step 6 — the dependency-tier capability matrix` | **The types.** `rust/tcl-dialect/src/model/environment.rs`: `DependencyTier` (`Root`, `Direct`, `Transitive`, `Development`, with `label`) beside `Provenance` and `WorkspaceTrust`, exported from `tcl_dialect::model` (D6.7). `rust/tcl-registry/src/model/capability.rs` (new): `CodegenCapability { tier, codegen_stamps, runtime_backing, builtin_alias, reference_body }` with `for_tier`, the page's matrix (`Root` everything; `Direct` a backing and an alias, no stamp, no body; `Transitive` and `Development` nothing), `DependencyTier` re-exported, and `ReferenceBodies { Forbidden, AnySource }` for the fourth field (D6.10). **The model crate.** `rust/tcl-pkg-model` (new workspace member): `errors.rs`, `json.rs`, `version.rs`, `manifest.rs` and `lockfile.rs` move from `tcl-pkg` with `git mv`, `tcl-pkg` re-exports the five (its own modules and `tcl-cli` keep their paths, as for `tcl-userdirs`), `LockFile::stamp`, the model's one use of the clock, becomes `tcl_pkg::stamp_lockfile` so the model has no `chrono`, `LockedPackage::required_names` is new, and `tier.rs` (new) is `dependency_tier(root_manifest, lockfile, package) -> Option<DependencyTier>` (D6.6, D6.8). The plan's `tcl-pkg` dependency was measured first and refused: `cargo tree` puts 33 more crates in the server's build (39 in the Explorer's and the Studio's) — `ureq`, `rustls`, `zip`, `tar`, `tcl-sandbox` among them — and `cargo check --target wasm32-unknown-unknown -p tcl-sandbox` fails (`wait-timeout` builds for `unix` and `windows` only), which would break `tcl-lsp-server-wasm`, `tcl-lsp-server-wasi`, `tcl-explorer-wasm` and `tcl-spec-studio-wasm` without `cargo check --workspace` seeing it, since none is a workspace member; the layout rules also keep a developer-tool crate out of the pack loader's graph. **Discovery.** `PackFile::dependency_tier: Option<DependencyTier>` (the last field, so the sort is unchanged; the 31 `PackFile` literals in nine crates' sources and tests gain `dependency_tier: None`), set by `discovery::assign_dependency_tiers` for the files of `Origin::BesideManifest` only. A file's package is the one whose manifest is nearest above it; the project root is the *outermost* directory inside the workspace folder holding both a `tclpkg.tcl` and a `tclpkg.lock` — not the plan's nearest, because an installed dependency's directory can hold a pair of its own and the nearest pair would let it name itself the root of its own graph; the package is `Root` when its directory is the project root, and otherwise the tier is `dependency_tier` of the project's manifest and lockfile for the package the manifest beside the file names. No lockfile, an unlisted package and a file that does not read leave `None` (D6.8). Read through the `SourceStore`, like the packs, so a browser host places its packs the same way. **The loader.** `PackCommand::dependency_tier`, set by the merge beside `file`; `pack::set_key` mixes each file's tier (a package moving in the graph is as much a change as an edit, or the cached registry would keep the old answer); `stamps.rs`: `RefusalReason::Capability(tier)` — a stamp must pass the provenance gate and then the capability gate (`stamps_admitted`, `capability_admits_stamps`), the provenance the reason named when both refuse — and the second gate for the two declarations, `Declaration` (`AliasOf`, `RuntimeBacking`), `DeclarationRefusal`, `declaration_refusals` and `admit_declarations`, run in `pack::load_sources` after the stamp rule so rule 1 still reads `alias_of`; each refusal is a warning on the command's row naming the tier ("`alias_of lassign` refused for `dep::unpack`: a transitive dependency's pack may not declare `alias_of`; only the workspace's own package and its direct dependencies may"), and only the declaration goes. The strip is memoised on the spec's address and a `Drops` set (stamps, `alias_of`, `runtime_backing`); `install_into`'s assertion asks both gates; `stamp_refusals` gains the tier parameter, and the Studio's and `spectcl_check`'s previews pass `None` (D6.9). Tests: `rust/tcl-pkg-model/src/tier.rs` (6): `a_package_the_root_requires_is_direct`, `a_package_reached_only_through_another_is_transitive`, `a_package_named_only_in_dev_require_is_development`, `a_regular_route_outranks_a_development_one`, `a_stale_entry_is_transitive_and_an_unlisted_package_has_no_tier` and `a_cycle_in_the_lockfile_terminates`. `rust/tcl-registry/src/model/capability.rs` (3): `the_matrix_is_the_pages` (every tier's row, cell by cell), `a_capability_names_its_own_tier` and `distance_never_widens_a_capability` (down the order a tier keeps or loses a right, never gains one). `rust/tcl-spectcl/src/discovery.rs` (6): `a_packs_package_is_placed_by_the_lockfiles_graph`, `no_lockfile_and_no_listing_mean_no_tier`, `a_dependency_shipping_its_own_lockfile_does_not_become_a_root` (the outermost-pair rule), `only_a_pack_beside_a_manifest_has_a_tier`, `an_unreadable_manifest_or_lockfile_leaves_no_tier` and `a_host_filled_store_places_its_packs_too` (an in-memory `SourceStore`, no disk). `rust/tcl-spectcl/src/stamps.rs` (7): `the_capability_gate_refuses_a_stamp_the_provenance_gate_admits`, `a_stamp_must_pass_both_gates`, `the_matrix_decides_which_declarations_a_command_may_keep`, `the_remedy_names_the_tiers_the_matrix_permits`, `a_refusal_names_the_declaration_the_command_and_the_tier`, `a_dropped_declaration_costs_no_other_fact` and `a_repeated_declaration_refusal_reuses_its_stripped_spec`. `rust/tcl-spectcl/tests/workspace_packs.rs` (4 new, 14 in all, was 10): the plan's `a_transitive_dependencys_alias_of_is_dropped` and `a_direct_dependency_keeps_alias_of_but_not_a_stamp` (its negative half loads the workspace's own package, `Root`, which keeps both declarations and loses the stamp to the provenance gate alone), `a_package_moving_in_the_graph_changes_what_its_pack_loads` (the lockfile edited between two loads; the set's key moves with it) and `a_pack_no_package_ships_is_not_narrowed`. `rust/tcl-lsp-server/src/lib.rs` (1): `a_manifest_or_lockfile_change_reloads_the_packs` (D6.11). **Mutation checks** (19, each one line or arm changed, the tests that must fail named beforehand; in 18 every named test failed, and the exception is the graph-walk mutation under `tier.rs`). `tier.rs`: a root requirement no longer `Direct`, a development requirement outranking a regular route, development reachability dropped and an unlisted package given a tier each fail their own test; the graph walk following no edge fails the two development tests and *not* the transitive test named for it, which cannot tell a package the walk reaches from one merely listed, since both are `Transitive` (D6.8): the transitive test pins the answer and the two development tests pin the walk. `capability.rs`: a direct dependency permitted a stamp fails `the_matrix_is_the_pages`; a transitive tier given a direct one's rights fails it and `distance_never_widens_a_capability`. `discovery.rs`: no tier assigned fails all six discovery tests; the nearest rather than the outermost project root fails `a_dependency_shipping_its_own_lockfile_does_not_become_a_root` alone; every origin placed fails `only_a_pack_beside_a_manifest_has_a_tier` alone. `stamps.rs`: the capability gate admitting every stamp fails the two stamp tests; refusing no declaration fails four; the stamp rule ignoring the command's tier fails `the_capability_gate_refuses_a_stamp_the_provenance_gate_admits` alone; an alias always permitted fails five. `pack.rs`: the merge not recording the tier fails `a_transitive_dependencys_alias_of_is_dropped` and `a_package_moving_in_the_graph_changes_what_its_pack_loads`; the key ignoring the tier fails the second alone; the load never running the declaration gate fails both. The server: `partition_watched_file_changes` no longer flagging a manifest or lockfile, and `is_package_metadata_file` forgetting the lockfile, each fail `a_manifest_or_lockfile_change_reloads_the_packs`. Nothing existing moved: no test expectation changed. `Cargo.lock` gains `tcl-pkg-model` and `tcl-pkg` loses `regex`, `tcl-lexer` and `tcl-syntax`; the shard table gains the new crate's lib row (329 targets, `verify-nextest-binary-shards.py` passing). Gates: `cargo test -p tcl-pkg-model` (39: the six new and the 33 that moved), `-p tcl-pkg` (lib 53, was 86 before the 33 moved out, and `manifest_env_drift` 2), `-p tcl-cli-support` (19), `-p tcl-registry` (lib 949, was 946, and every binary), `-p tcl-spectcl` (lib 210, was 197; `workspace_packs` 14, was 10; `codegen_stamps` 7, `i6_security_floor` 10, `golden_packs` 3, `spec_corpus` 5, `workspace_trust` 7 and every other binary), `-p tcl-spec-studio` (lib 199 and every binary), `-p tcl-mcp` (114), `-p tcl-lsp-db` (lib 103 and every binary), `-p tcl-lsp-core` (lib 2350), `-p tcl-dialect` (lib 153), `-p tcl-compiler` (lib 6546, `analyser` 505, `cfg` 17, `value_transfer_witnesses` 64, `codegen` 164), `-p tcl-lsp-server --lib` (595, was 594), `-p tcl-cli` (lib 27, `cli` 50, `compile_verbs` 11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18, `value_transfers_cli` 8) and `-p xtask` (237); `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean, no new `#[allow]`; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate` and `owner-resolution` (45) OK, `kcs-index-links` green, `dialect-drift` at its 8 sites. Docs: `spec-packs.md` (the workspace-trust section's second gate), the design page (the status paragraph and both not-built lists, rule 2, the packages table's tier row, the capability code block with `ReferenceBodies`, the composition paragraph with the outermost-lockfile rule and its binding residual, the anchors), `docs/GLOSSARY.md` (*Dependency tier and codegen capability*, and its index row), `docs/kcs/kcs-qa-why-was-a-declaration-dropped-from-my-dependencys-pack.md` (new, indexed in `docs/kcs/README.md`), `docs/kcs/kcs-howto-write-a-tclspec-pack.md`, `project-layout.md` (the model crate beside `tcl-userdirs`), `tclpkg/architecture.md`, `tclpkg-contracts.md` and `spec-dsl-examples/README.md`. Deviations: the model crate, not `tcl-pkg` (D6.6); `DependencyTier` in `tcl-dialect`, not `tcl-pkg` (D6.7); the outermost lockfile, not the nearest (D6.8); `ReferenceBodies`, not `Option<BodySource>` (D6.10); a server reload trigger beyond the plan's files (D6.11). D6.6–D6.11 |
| CC6.3 the overlay reaches the compile service; a miss fails closed | landed | `wip(consumer-contracts): step 6 — overlay misses fail closed` | **The door.** `rust/tcl-registry/src/model/assembly.rs`: `OverlayMiss { environment, overlay }` (`Display`, `Error`, exported from `tcl_registry::model`), and `registry_for_environment_if_built` returns `Result<Arc<ContextRegistry>, OverlayMiss>`; `ingress.rs`: `DocumentEnvironment::context_registry` returns the same, `plain_context_registry` is new and infallible (overlay `0` cannot miss) and `default_context_registry` goes through it, so nothing around the overlay lookup falls back (`grep unwrap_or` in `ingress.rs` finds the environment-name resolution, the library-version override and one test helper, none on the overlay lookup); `cache.rs`'s docs say the door's callers answer a miss themselves. **The consumers** (D6.12) are told apart by what a wrong answer costs. *Advising* consumers read the plain generation by a door named for it: the analyser's three overlay reads go through `environment_ingress::analysis_registry` and its per-item walk through `plain_context_registry`, and the two semantic-token queries through `token_registry` in `tcl-lsp-db`. *Compiling* consumers decline. `BytecodeCompileService` gains `RegistryTarget::Overlay { profile, overlay }` and `for_profile_with_overlay(profile, overlay) -> Result<Self, OverlayMiss>` (overlay `0` is `for_profile`): the generation is looked up again for every compile, every entry point returns a `CompileError` naming the overlay when it is gone, an explicit-profile compile looks the service's overlay up for the profile asked for, and `RegistryTarget::registry` and `registry_for_profile` return `Result` with a `?` at the four compile paths (D6.16). In `tcl-lsp-db`, `TclDb::registry_with_overlay` returns `Result`, `compilation_unit` and `proc_taint_solve` return `Option`, `compiler_check_diagnostics` answers no checks and no optimisations, `file_analysis_incremental` supplies no override, and `document_compilation_unit_for` is an `Option`; the abstention reads the overlay epoch — `tcl_registry::overlay_epoch` (`cache.rs`: it moves when an overlay generation is installed or a sweep retires one), mirrored as the salsa input `tcl_lsp_db::OverlayEpoch` that `unit_registry` reads for every non-zero overlay and the host moves with `set_overlay_epoch` (the server does, beside its key publish, in `sync_overlay_epoch`) — so an overlay installed later is found when the epoch moves, by the unit and by everything that read it; a first design that called salsa's `report_untracked_read` instead was measured to re-run the unit and leave the analysis on its memoised answer, and was replaced (D6.14). Each distinct miss is recorded once (`take_overlay_misses`) and reported by the server (`report_overlay_misses`, on stderr with its worker faults). The database holds the generations its queries resolve (`OverlayGenerations`, 64, process-wide) so a per-procedure query, which reads `nested_registry`, finds the generation its unit started with when the process cache retires the key (D6.15). The server's other change is `.flatten()` on the unit handle. **The plan's server file did not apply**: `optimise_document_command` builds no compile service, because it has none — it hands `optimise_under_policy` the registry `registry_for_dialect` installs the overlay into, which cannot miss (D6.13); the exit evidence for it is the two existing e2e tests that run the optimiser under a workspace pack, run here and green. Tests: `rust/tcl-registry/src/model/ingress.rs` `an_uninstalled_overlay_is_an_error_not_the_plain_generation` (the plan's; the old test's fallback half is its inversion, and `context_registries_carry_the_expected_stores` keeps the rest); `rust/tcl-compiler/src/compile_service.rs` (3) `a_service_for_an_uninstalled_overlay_is_not_built`, `a_service_over_an_installed_overlay_compiles_its_commands` (an overlay-only command is a known binding to the service and unknown to the plain one, for its own profile and another release's) and `a_service_whose_overlay_is_gone_declines_every_compile` (all eight compile entries); `environment_ingress.rs` `an_analysis_reads_the_plain_generation_until_its_overlay_installs`; `rust/tcl-lsp-db/tests/dialect_seam.rs` (2) `the_compilation_unit_sees_the_packs_commands` (the plan's: a real pack's `VarWrite` roles define `a` in the unit; the key set to `0`, the unit is rebuilt without it; set back, it sees it again) and `the_analysis_reads_the_packs_once_they_install`; `rust/tcl-lsp-db/tests/overlay_generations.rs` (new binary, 4, serialised because one of them fills the process caches) `an_uninstalled_overlay_yields_no_unit_and_no_findings` (with a control that the same document has a unit and rewrites under no overlay, and one record for two queries that missed), `an_abstention_runs_again_once_the_overlay_is_installed` (the unit and the checks and rewrites that read it, after the epoch moves and not before), `the_token_query_reads_the_plain_registry_for_a_miss` and `a_retired_generation_still_serves_the_queries_that_resolved_it`; `rust/tcl-registry/src/cache.rs` `installing_and_retiring_an_overlay_moves_the_epoch`; `rust/tcl-spectcl/tests/codegen_stamps.rs` `a_service_for_the_packs_overlay_compiles_against_the_generation_they_installed` (a real pack through the door: the alias site and the fold claim their pack facts and stamp the overlay generation, as the owned projection's do; a key nothing installed builds no service; a service for `tcl9.0`'s pack declines a compile for `tcl8.6`); `rust/tcl-lsp-server/src/lib.rs` (2) `an_overlay_miss_is_reported_with_its_key_and_what_waits` and `syncing_the_overlay_epoch_lets_an_abstained_unit_build`. **Mutation checks** (18, each one line or arm changed and the tests that must fail named beforehand, run over the final tree; in 14 exactly the named tests failed and in four — G5, H1, H4 and H6 — others failed as well, as listed). `ingress.rs`: the door answering a miss with the plain generation (F1) fails `an_uninstalled_overlay_is_an_error_not_the_plain_generation`. `environment_ingress.rs`: the analysis door with no fallback (G1) fails `an_analysis_reads_the_plain_generation_until_its_overlay_installs`. `compile_service.rs`: the service's default registry (G2) and its per-profile registry (G3) each falling back to the plain generation fail `a_service_whose_overlay_is_gone_declines_every_compile`; a service built over an overlay nothing installed (G4) fails `a_service_for_an_uninstalled_overlay_is_not_built`; the service looking overlay `0` up whatever its key (G5) fails all three service tests; a per-profile compile using the service's own profile instead of the one asked for (G6) fails `a_service_for_the_packs_overlay_compiles_against_the_generation_they_installed`. `tcl-lsp-db`: the unit falling back to the plain registry on a miss (H1) fails `an_uninstalled_overlay_yields_no_unit_and_no_findings` and `an_abstention_runs_again_once_the_overlay_is_installed`; an overlay lookup that reads no epoch (H2) fails the second and `the_analysis_reads_the_packs_once_they_install`; the database holding no generations (H3) fails `a_retired_generation_still_serves_the_queries_that_resolved_it`; every miss recorded again (H4) fails the first and `the_token_query_reads_the_plain_registry_for_a_miss`; the token query with no plain fallback (H5) fails its own test; the unit ignoring the overlay (H6) fails `the_compilation_unit_sees_the_packs_commands`, `an_uninstalled_overlay_yields_no_unit_and_no_findings` and three more; an epoch written whether or not it moved (E1) fails `an_abstention_runs_again_once_the_overlay_is_installed`. `cache.rs`: an install that does not move the epoch (R1) and a sweep that does not (R2) each fail `installing_and_retiring_an_overlay_moves_the_epoch`. The server: not syncing the epoch after a reload (S1) fails `syncing_the_overlay_epoch_lets_an_abstained_unit_build`, and a miss report that omits what waits (I1) fails `an_overlay_miss_is_reported_with_its_key_and_what_waits`. The first design of the abstention, salsa's `report_untracked_read`, is not among them: it was measured wrong before these were written (D6.14). Moved: `assembly.rs`'s `pack_overlays_thread_through_the_generation_door` asserts the `OverlayMiss` value where it asserted `is_none()`; `context_registries_carry_the_expected_stores` (`ingress.rs`) and its twin in `environment_ingress.rs` lose the fallback assertion (the named tests are its inversion); `spec_corpus.rs` (2) and the lib tests of `tcl-lsp-db` and `compile_service.rs` take an `.expect` where the door became fallible, `value_transfer_parity.rs` reads its units through `installed_unit`; the shard table gains `tcl-lsp-db::overlay_generations`. Gates: `cargo test -p tcl-registry` (lib 951, was 949; 1281 with every binary and the doctest), `-p tcl-compiler` (lib 6550, was 6546; all 66 integration binaries, 3271 tests, `analyser` 505, `cfg` 17, `codegen` 164, `value_transfer_witnesses` 64, `wasm_real_link` 13 and `wasm_tiers` 7 among them), `-p tcl-lsp-db` (lib 103; `dialect_seam` 5, was 3; `overlay_generations` 4, new; 135 in all), `-p tcl-spectcl` (lib 210; `codegen_stamps` 8, was 7; `i6_security_floor` 10; `workspace_packs` 14; `spec_corpus` 5; 365 in all), `-p tcl-lsp-server` (lib 597, was 595; `e2e` 1600, `preview_tickets_e2e` 24, `smoke` 14, `stdio_deadlock` 6), `-p tcl-mcp` (114) and `-p tcl-lsp-core --lib` (2350) green; `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean, no new `#[allow]`; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged), `pack-goldens --check` (25), `command-backing --check` (389), `retired-api-gate`, `owner-resolution` (45) and `kcs-index-links` green, `dialect-drift` at its 8 sites (`compile_service.rs`'s `LexerConfig::default()` moved from line 125 to 165), and `verify-nextest-binary-shards.py --partition-count 5 --metadata-only` proves 330 targets (`tcl-lsp-db::overlay_generations` is the one new row). Docs: the design page (the *Codegen and the registry today* bullet on the overlay, the runtime-pin bullet, the plumbing-gap row for the shared compilation unit, the file-path and test anchors), `dialect-and-package-registry-centralisation.md` (its F7 row goes, the fail-closed overlay row joins its tests, and the not-built sentence loses it), `value-evaluation.md` (a unit is not built against a fallback), `kcs-qa-is-the-command-registry-fixed-at-compile-time.md` (a miss is an `OverlayMiss` and what each consumer does with it). Deviations: the server file of the plan (D6.13); the analysis-versus-compile split of what the door's callers do with the error (D6.12); the overlay epoch, the generation holder and the once-only record with its stderr report, none in the plan, each what "diagnostic-free abstention, logged once" needs to hold when a query runs again (D6.14, D6.15). D6.12–D6.16 |

**Step 6 is landed.** All three items above (CC6.1, CC6.2 and CC6.3) are
`landed`; the review checklist below is run against this tree and its evidence
recorded there. The plan's exit evidence holds, with one qualification.
`cargo test -p tcl-spectcl --test i6_security_floor` is green (10 tests: the
six rows CC6.1 added, and the rows CC7.1 added for `runtime_backing`),
`cargo test -p tcl-registry` is green with the ingress test inverted
(`an_uninstalled_overlay_is_an_error_not_the_plain_generation`; lib 951)
and `cargo test -p tcl-lsp-db --test dialect_seam` is green (5, was 3). The
qualification is the last line of the exit evidence, "the server's optimise path
compiles under the workspace overlay": that path builds no compile service, so
the door has no server caller to compile through (D6.13). What runs is the two
e2e tests that put the optimiser under a workspace pack,
`a_pack_const_fold_body_folds_a_call_site_in_the_optimiser` and its negative
`without_the_pack_the_same_call_site_does_not_fold`, and the door's own witness
through a real pack, `a_service_for_the_packs_overlay_compiles_against_the_generation_they_installed`
(`codegen_stamps.rs`); all pass. The plan's own list of green suites is run over
the final tree: `cargo test -p tcl-registry`, `-p tcl-spectcl`, `-p tcl-lsp-db`
and `-p tcl-compiler` (every binary, `--test codegen` among them) are green,
with the counts in *Step 6 — review checklist, verified*.

The landing commit also changes the following. It appends `"step 6"` to
`registry_axes.rs`'s `LANDED` — D2.13's "a lane landing a step or slice bumps
it". No waiver in the tree says `until step 6`, so none expires and none needs
resolving; the gate's own fixtures that used `until step 6` as a not-yet-landed
expiry (five spellings in three tests) move to `until step 7`, as step 5's
landing moved `until step 5` to `until step 6` (`cargo test -p xtask
registry_axes`, 9; the crate's 237 pass), and `docs/generated/registry-axes.md`
regenerates unchanged. It rewrites the design page's status box to add the
overlay door and to say what is not built, and the two design indexes' entries
for the page likewise, under the repository owner's standing rule for every page
outside `docs/design/lanes/`: current state only, with no step, item or decision
numbers. Rule 4 of the page's § *The loader's stamp rejection rule* and the
sentence after it, which said the floor "protected" two hooks "and nothing else"
and marked rules "(step 4)", "(step 6)" and "(step 7)", now say what the floor
keeps and that the rules are built; other passages that carry that narrative are
untouched, for the owner's sweep. It updates the title above and
`docs/design/lanes/README.md`'s bullet. Gates for the landing: `cargo check
--workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D
warnings` and `cargo fmt --all -- --check` are clean, with no new `#[allow]`;
`registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged),
`value-transfers --check` (22 / 19 / 83 across 34, 6607 rows, unchanged),
`pack-goldens --check` (25), `command-backing --check` (389),
`retired-api-gate`, `owner-resolution` (45), `kcs-index-links`,
`callback-inventory --check`, `audit-option-dialects --check` (114 probed
options), `number-drift` and `segmentation-drift` are green, `dialect-drift` is
at its 8 sites (`compile_service.rs`'s `LexerConfig::default()` is one, now at
line 165), and `verify-nextest-binary-shards.py --partition-count 5
--metadata-only` proves 330 targets (CC6.2 added the `tcl-pkg-model` lib row and
CC6.3 the `overlay_generations` binary). The plan drafts no landing message for
step 6, so this commit's follows step 5's. Step 7 is in progress, on its own
items.

### Step 6 — review checklist, verified

The plan's § *Review checklist* (§ *Plan for steps 2–10* › *Step 6*), run
against the landed tree with evidence:

- **The floor is a codegen-axis contract, not a trust gate on analysis facts: an
  override still changes arity, roles, hover.** `security_floor.rs`'s module and
  `apply` docs say so (D6.4). Every row of `i6_security_floor.rs` first asserts
  the override took effect through its own `arity 7..9` window, a window no
  shipped command has, so a green row means the floor restored the value and not
  that the override never installed; `the_floor_adds_nothing_the_shipped_command_lacks`
  holds that an override's own value stands where the shipped command has none.
  The floor reads command-level fields only; a subcommand's or a form's hooks are
  outside it, and *what the next steps read* has the reproduction.
- **Both gates, provenance and capability, apply and a declaration passes both;
  the notices name which one refused.** `stamps_admitted` asks
  `stamps_admitted_from` (the provenance) and then `capability_admits_stamps`
  (the tier), and `admit_declarations` runs the capability's second gate over
  `alias_of` and `runtime_backing` in `pack::load_sources` after the stamp rule.
  `a_stamp_must_pass_both_gates` and `the_capability_gate_refuses_a_stamp_the_provenance_gate_admits`
  hold both directions; `stamp_refusals` names the provenance when both refuse
  and the tier when only the capability does, and
  `a_refusal_names_the_declaration_the_command_and_the_tier` holds the wording.
- **No silent fallback remains in `ingress.rs`.** `grep -n unwrap_or
  rust/tcl-registry/src/model/ingress.rs` finds three lines: the environment-name
  resolution to the lenient `tcl` (127), the library-version override (364) and a
  test's spelling of the retired ingress (671, in `mod tests`); none is on the
  overlay lookup. `context_registry` returns `Result<_, OverlayMiss>`, and its
  non-test callers outside `tcl-registry` are `compile_service.rs` (`?`, into a
  `CompileError`), `tcl-lsp-db`'s `unit_registry` (an `Err` the unit abstains
  on, which `token_registry` reads through a named plain fallback) and
  `environment_ingress::analysis_registry` (the analyser's named plain
  fallback): a fallback is a function whose name says so, at the consumer, and
  never an `unwrap_or` at the ingress (D6.12).
  `an_uninstalled_overlay_is_an_error_not_the_plain_generation` holds the ingress;
  `an_analysis_reads_the_plain_generation_until_its_overlay_installs` and
  `the_token_query_reads_the_plain_registry_for_a_miss` hold the two doors.
- **Risk: `tcl-pkg` as a `tcl-spectcl` dependency pulling `tcl-sandbox` into
  every server build.** Measured at CC6.2 and refused (D6.6): the manifest and
  lockfile live in `tcl-pkg-model`, and `cargo tree -p tcl-lsp-server` on the
  landed tree lists `tcl-pkg-model` and none of `tcl-pkg`, `tcl-sandbox`, `ureq`
  or `rustls`.
- **Exit evidence, run over the final tree.** `cargo test -p tcl-spectcl --test
  i6_security_floor` 10; `-p tcl-registry` (lib 951, 1281 with every binary);
  `-p tcl-lsp-db --test dialect_seam` 5 (the crate's 135 in all); `-p
  tcl-compiler --test codegen` 164 (lib 6550, and 3271 in the 66 integration
  binaries); `-p tcl-spectcl` 365; `-p tcl-lsp-server` 2241 (lib 597 and `e2e`
  1600, which hold the optimiser's
  `a_pack_const_fold_body_folds_a_call_site_in_the_optimiser` and
  `without_the_pack_the_same_call_site_does_not_fold`), `-p tcl-mcp` 114 and `-p
  tcl-lsp-core --lib` 2350; no failure and no test newly ignored.

### Step 6 — what the next steps read

- **For CC7.3 (the intrinsic families).** A family carries a `GuardDomain`, and
  the domain's docs in `rust/tcl-runtime-api/src/guard.rs` are what the code does
  today (the step 5 review fixes corrected them): `CommandEnvironment`,
  `Namespace` and `UnknownHandling` describe how a name reaches a command, and
  each runtime moves the three together and on no definition, rename, alias,
  import, hide or expose of a command; the runtime's events are `namespace
  path`, `export`, `unknown`, `delete`, a forget that removes an import, TclOO
  creation and child-interpreter creation and deletion, the VM's are `namespace
  path`, making an interpreter safe, `interp marktrusted` and deleting a child
  (issue #2292). `every_member_names_a_family` and `trace_firing_members_are_family_b`
  should take a member's domain from those docs. A split that moves a member's
  guard domain or trace behaviour raises its `SEMANTICS_REVISION` row (CC5.1).
- **For CC7.2 (the backing query and the identity sweep).** "From the pinned
  generation only" has a door: `DocumentEnvironment::plain_context_registry` is
  the shipped generation and cannot miss, while `context_registry(keyed, overlay)`
  is fallible and is what an overlay goes through; the sweep takes the first and
  never the second. A `runtime_backing` from a transitive or development
  dependency's pack never reaches a spec (CC6.2's load gate), and the floor keeps
  a shipped command's against any override from any tier, so `backing_report()`
  and the gate need no tier logic; a `Root` or `Direct` package's declaration
  does reach the registry, and the query sees it.
- **For CC7.4 (the manifest and the runtime context).** The pack-set field is
  `PackSet::key`, which mixes each file's `DependencyTier` as well as its
  content: a package moving in the lockfile's graph moves the key, so a unit
  compiled before the move is refused at rung 1 like any other changed pack.
  `RuntimeContext::overlay_generation` resolves at
  `DocumentEnvironment::context_registry`, and a miss is an `OverlayMiss` the pin
  takes as an error, as the plan says; `analysis_registry` is for what advises
  and runs again, and a pin is neither. The pin should hold the
  `Arc<ContextRegistry>` it resolved, not the key alone: the process cache retires
  overlay generations past 64 (`OVERLAY_LIMIT`), and the database holds them for
  the same reason (D6.15). A compile through `BytecodeCompileService::for_profile_with_overlay`
  looks the generation up at every compile (D6.16), so the manifest's pack facts
  are those of the compile, not of the service's construction.
- **For CC7.5 (the fuzz exit).** Nothing in this step touches either runtime; the
  seeds and the `return` defect in *Step 5 — what the next steps read* stand.
- **Reported, not fixed: the floor does not reach a subcommand's or a form's
  hooks (D6.4).** `SecurityFloor::apply` reads `CommandSpec` fields. A
  `SubCommand` carries its own `lowering_hook`, `codegen_hook`,
  `inline_codegen_hook`, `analyser_hook`, `semantic_operation` and
  `state_transitions`, a `CommandForm` its own `lowering_hook`, `codegen_hook`,
  `semantic_operation` and `state_transitions`, and an override of the command
  replaces every such row. The stamp rule refuses only a pack's own
  `codegen_hook`, `inline_codegen_hook` and `semantic_operation {Intrinsic …}`
  at those sites and restores nothing, so `lowering_hook`, `analyser_hook` and
  `state_transitions` on a subcommand or a form are open to a swap, and every
  such row to a drop. Measured on the landed tree with three temporary tests in
  the style of `i6_security_floor.rs`, using its `shipped` and `overridden`
  helpers, each asserting what the floor would guarantee and each failing:
  `a_workspace_override_cannot_swap_a_subcommands_lowering_hook` —
  `shipped("tcl8.6", "array")`'s `for` subcommand has `lowering_hook:
  Some(ArrayFor)`, and `overridden(..., "array", "subcommand for { lowering_hook
  -native If }")` installs `Some(If)` on it, and `lowering/mod.rs` chooses the
  translation by that id;
  `a_workspace_override_cannot_swap_a_forms_lowering_hook` — `incr`'s `implicit`
  form ships `Some(Incr)`, and `refine implicit { lowering_hook -native If }`
  installs `Some(If)`;
  `a_workspace_override_cannot_drop_a_subcommands_intrinsic` — `string length`
  ships `semantic_operation: Some(Intrinsic(StringLength))`, and an override
  that redeclares `subcommand length { arity 1 }` installs `None`. The tests are
  not committed. The fix is `apply` walking `subcommands` and `command_forms` by
  name and taking the shipped row's hooks the way it takes the command's, with
  `MERGED_FIELDS` and `every_security_bearing_field_is_in_the_floor` naming
  them; what a pack that removes or adds a subcommand should keep is the open
  part.

### CC6.1 — what the next items read

- **For CC6.2.** The floor and the capability matrix are separate gates
  and both apply: the floor is unconditional and silent (an override keeps
  the shipped value, with no notice, exactly as it has for taint), while
  CC6.2's capability drops `alias_of`, `runtime_backing` and reference
  bodies with a notice naming the tier. A declaration passes both.
- **For CC7.1.** `runtime_backing` joins the floor the way the six did:
  one `take_shipped` line in `apply`, `"runtime_backing"` in
  `MERGED_FIELDS` and in the field scan's `matches!`, and a seventh row in
  `i6_security_floor.rs`. The file's `shipped` and `overridden` helpers
  build a row in three lines.
- **Residue, reported.** The floor reads `CommandSpec` fields only: the
  same fields on a `SubCommand` or a form are not restored (D6.4).

### CC6.2 — what the next items read

- **For CC6.3.** Nothing in the item touches the overlay. The compile
  service's `LexerConfig::default()` (`compile_service.rs:165`) is still one
  of the eight `dialect-drift` sites.
- **For CC7.2 (the backing query).** A `runtime_backing` from a transitive or
  development dependency's pack never reaches a spec: the load drops it, with
  a warning naming the tier, before the merge is installed. The query needs no
  tier logic. A `Root` or `Direct` package's backing does reach the registry,
  and the floor still keeps a shipped command's against any override.
- **For CC8.x (reference bodies, the manifest `spec` directive).** The
  manifest and lockfile live in `rust/tcl-pkg-model/src/` now, not
  `rust/tcl-pkg/src/` (D6.6): `ManifestAst::spec` and the lockfile's pack hash
  are edits there, and `tcl_pkg` re-exports the modules. `tier::dependency_tier`
  is the derivation `SpecDirective::requested_tier` clamps against, and
  `DependencyTier` is `tcl_dialect::model::DependencyTier`. A reference body
  joins the load gate as one more `Declaration` variant and one more `Drops`
  field beside `alias_of` and `runtime_backing` in `stamps.rs`;
  `CodegenCapability::reference_body` is `ReferenceBodies` already. The
  lockfile hash of each pack is what closes D6.8's residual: today a pack's
  package is the one its manifest names, and a manifest that names a package
  the lockfile does not list gets no tier.
- **For anything that builds a `PackFile`.** The struct gained
  `dependency_tier`; a literal takes `dependency_tier: None` unless the caller
  is discovery. Every literal in the tree, 31 in nine crates, takes it in this
  commit.
- **Reported, not fixed.** The wasm hosts' own lockfiles
  (`rust/tcl-spec-studio-wasm/Cargo.lock` and the three beside it) are already
  stale at `dc7a138c` — `cargo metadata --locked` there fails, `tcl-spectcl`'s
  `tcl-runtime-api` edge from step 4 is missing — and this commit adds
  `tcl-pkg-model` to their graphs, so they need regenerating together; they
  are not workspace members and not this lane's. `spectcl_check` and the
  Studio's store view report stamp refusals for a `(tier, trust)` pair and do
  not show a capability refusal, since neither sees a lockfile. `cargo test -p
  tcl-spectcl --lib` failed once in three runs at `cache::tests::the_two_tiers_share_one_identity`
  (`cache.rs:997`, the entry count read 2 where it expects 1) and passed in
  every other; no lib test this item adds loads a pack, so it looks like an
  older race with a test that loads one without the cache lock.

### CC6.3 — what the next items read

- **For a host that compiles bytecode against a workspace's packs.**
  `BytecodeCompileService::for_profile_with_overlay(profile, key)` is the door,
  with `key` the `PackSet::key` the packs were installed under. The packs have to
  be installed for each profile the service is asked to compile for, and the
  service declines a compile for one they are not installed for (D6.16). No
  shipped host uses the door yet (D6.13): `tcl-engine-tclvm`'s `with_registry`
  takes an owned registry, which is what `codegen_stamps.rs`'s first service test
  still compiles through.
- **For CC7.5's runtime context pin.** The pin's overlay generation is looked up
  at `DocumentEnvironment::context_registry`, and a miss is an `OverlayMiss` to
  take as an error, as the plan says. `analysis_registry` is for what advises and
  runs again, and a pin is neither.
- **For a new query in `tcl-lsp-db` that reads an overlay registry.** A query
  that builds a unit resolves through `unit_registry` at its top, which reads
  the `OverlayEpoch` input, and abstains through `abstain` on a miss (D6.14); a
  per-procedure query reads `nested_registry` (D6.15). Do not put a plain
  fallback below the unit query: the result would be memoised under the pack's
  key. A host that installs an overlay after a query has missed on it moves the
  epoch with `set_overlay_epoch(db, tcl_registry::overlay_epoch())`.
- **For a caller of the unit queries.** `document_compilation_unit_for` and
  `compilation_unit` answer `None` when the packs are not installed; the server's
  handle flattens it, and the tests that install their overlay first `.expect`
  it. `document_compilation_unit` (no overlay) still answers an `Arc`.
- **Reported, not fixed.** In the window a miss leaves, the analyser's own
  CFG/SSA tail builds a unit against the plain registry, because the shared unit
  query declined to build one; that analysis is corrected when the host moves
  the overlay epoch, which runs the unit query again and so the analysis that
  reads it. The server's line for a miss goes to stderr, where its worker faults
  already go, not to `window/logMessage`. `OverlayGenerations` is first in first
  out, not least recently used; with 64 entries and one per
  `(environment, overlay)` the difference is not reachable. The compile
  service's `LexerConfig::default()`, one of the eight `dialect-drift` sites,
  moved to `compile_service.rs:165` with this item's additions, and the two
  earlier notes that cite its line are updated.

### Behavioural deltas accepted in step 6

- CC6.3: a pack overlay nothing has installed no longer gives the editor a
  compilation unit built against the plain registry. Until the packs are
  installed for the document's dialect, the compiler checks and the optimiser's
  hints for a document under that key are absent, the miss is logged once on
  the server's stderr, and analysis and highlighting read the plain registry as
  they did. The server installs the overlay before it publishes the key, so the
  window is a reload racing a pass at the superseded key; with the packs
  installed nothing changes. `BytecodeCompileService::for_profile_with_overlay`
  is new, and no shipped host builds one; a compile through it for a generation
  that is not installed is a `CompileError`, not a plain compile.
- CC6.2: a `.tclspec` beside a `tclpkg.tcl` whose package a `tclpkg.lock`
  lists as a transitive or development dependency loses its commands'
  `alias_of` and `runtime_backing` at load, each with a warning on the
  command's row; a direct dependency's pack keeps both and loses any codegen
  stamp (which every workspace-tier pack already lost); the workspace's own
  package's pack keeps everything the provenance gate leaves it. A pack found
  any other way, in a project with no lockfile, or for a package the lockfile
  does not list, is unchanged. The server reloads the packs when a
  `tclpkg.tcl` or `tclpkg.lock` changes. No shipped or bundled pack sits
  beside a manifest, so nothing a user runs today changes without a lockfile.
- CC6.1: an `-override` of a shipped command, from any tier, keeps the
  shipped command's `lowering_hook`, `analyser_hook`, `semantic_operation`,
  `state_transitions`, `native_lowering` and `bpf_op` wherever the shipped
  command has one; before, only the two codegen hooks and the taint,
  side-effect and credential facts survived. It changes arity, options,
  roles and hover as before, and the floor gives no notice. No shipped or
  bundled pack overrides a command, so nothing a user runs changes today;
  a workspace pack that did swap one of the six now finds the shipped value
  installed.

## Step 7 — progress

Item order follows § *Plan for steps 2–10* › *Step 7* § *Ordering and
checkpoints*: CC7.1 (sonnet) first, on its own; then the opus items, CC7.3,
CC7.2 and CC7.4 in that order, each its own checkpoint, and CC7.5 (the fuzz
exit) last.

| Item | State | Checkpoint | Notes |
|---|---|---|---|
| CC7.1 `RuntimeBacking` on the spec | landed | `wip(consumer-contracts): step 7 — RuntimeBacking on the spec` | `rust/tcl-registry/src/runtime_backing.rs` (new): `RuntimeBacking` (`ShippedBuiltin { identity }`, `TclBody { source }`, `HostNative`, `None` — the default) and `BodySource` (`PackageSource { relative_path }`, `PackText { text }` — the text is carried, D7.3), with `shipped`, `package_source` and `is_none`; `CommandSpec::runtime_backing`, `CommandSpec::DEFAULT` says `None`, `RuntimeBacking` and `BodySource` reach the prelude and the crate root. **The rows.** The report's 389 core Tcl commands each declare what the report says: 324 `ShippedBuiltin` (the 282 handler and native rows and the 42 known-gap rows, whose target state it is, D7.5), 11 `TclBody`/`PackageSource` (`init.tcl` 7, `package.tcl` 3, `parray.tcl` 1 — the stdlib rows) and 54 `None` (the not-required rows, by default). A mechanical pass placed the 128 standard spec literals; seven builders needed hand edits — `mathop_generated` (the two qualified spellings `ShippedBuiltin`, the bare operator word `None`), `mathfunc_generated` (both spellings), `dict::qualified_specs`, `oo_helpers::qualified_specs` (shipped where the bare twin is, so the `ooutil` twins declare none), and the `corotype`, `zipfs` and `list_math_91` factories; the identity is the spec's own name as the spec spells it, `::` kept (D7.4). **The floor.** `runtime_backing` joins `SecurityFloor::apply` (a non-`None` shipped backing wins), `MERGED_FIELDS` and the field scan (D6.1). **The statement.** `rust/tcl-spectcl/src/backing.rs` (new) — `BackingSyntax` with `parse`, `parse_spelling`, `spelling`, `from_backing` and `leak` — is the one spelling of the five `runtime_backing` statements (`none`, `host-native`, `shipped-builtin ID`, `tcl-body {-package-source PATH}`, `tcl-body {-pack-text {TEXT}}`) for the loader (`apply_command_stmt`'s new arm and `eval.rs`'s `ROW_WORDS`, the two halves of the one loader), the Studio's draft and both its renderers; a statement that does not read is dropped with a warning and claims nothing. `PackNotice::pack_text_backing` — an Information notice on the command's row, raised in `pack::load_sources` beside the stamp refusals — reports a `-pack-text` body at load (D7.7). **The Studio.** `FieldKind::RuntimeBacking` (`IDENTITY` category, sharing the `text` wire tag, so the front-end needs no new editor and `every_field_kind_has_a_front_end_editor` stays green), `draft.rs` (the spelling), `render_spectcl.rs`'s `runtime_backing_row` (re-spelled through the parser, so the row is always one the loader reads back), `render_rs.rs`'s `runtime_backing_expr`, `help.rs`, `coverage.rs`'s witness and `Field` row, `relations.rs` (the "Builtin identity" cluster) and `examples/fields_core.rs` (D7.6); `docs/references/command-spec/fields.md` regenerated. **The generator.** `gen_irule_test_data.rs` emits a stub only for a command whose backing is `None` or `HostNative`; the plan expected the output byte-identical, and it is not: 46 entries for shared Tcl core commands and `pkg::create` drop, all dead (D7.8). **The ports.** Nine ports of shipped core commands declare `runtime_backing shipped-builtin NAME` (D7.9). Tests: `registry_sweep.rs`'s `every_core_command_declares_a_backing` reads the committed report and holds the spec to it in both directions — the row's kind, the identity equal to the name, the stdlib file the note names — and requires every non-core spec in the Tcl table to declare nothing (a mutation check, one declaration removed, fails it naming the command); `i6_security_floor.rs` gains `a_workspace_override_cannot_swap_the_runtime_backing` (`lindex` against `host-native` and `none`) and `a_new_command_keeps_the_backing_it_declares`; `security_floor.rs`'s unit rows cover it; `eval_loader.rs`'s `runtime_backing_reads_each_shape_through_both_paths` (five shapes, an unstated one, two that do not read — through the static fast path and the interpreter); `workspace_packs.rs`'s `a_pack_text_backing_is_reported_at_load`; `spectcl_roundtrip.rs`'s `runtime_backing_survives_the_round_trip` (five backings, one with unbalanced braces, and `none` writing no row); `backing.rs`'s four unit rows. Moved: `spectcl_ports.rs`'s `every_port_loads_and_matches_its_shipped_spec` (the ports now declare it, D7.9); `_mock_stubs.tcl` (D7.8); `pack-goldens` rewrote all 25 snapshots (every `spec` digest moved once, as for `alias_of`; the nine edited ports' notice lines moved by one). Gates: `cargo test -p tcl-registry` (lib 946, was 943; `registry_sweep` 40, was 39; every other binary), `-p tcl-spectcl` (lib 196, was 192; `eval_loader` 26, was 25; `i6_security_floor` 10, was 8; `workspace_packs` 10, was 9; `codegen_stamps`, `golden_packs`, `spec_corpus` and every other binary), `-p tcl-spec-studio` (lib 199; `spectcl_roundtrip` 10, was 9; `spectcl_ports` 11; `reference_doc` regenerated; every other binary), `-p xtask -p tcl-mcp -p tcl-cli` (480), `-p tcl-irule-test` (28, against the regenerated stubs), `-p tcl-compiler -p tcl-lsp-core --lib` (6531, 2 ignored; 2350); `runtime/rust`'s `cargo check --tests` clean; `cargo check --workspace --all-targets`; clippy (`-p tcl-registry -p tcl-spectcl -p tcl-spec-studio -p xtask --all-targets --no-deps -- -D warnings`) and `cargo fmt` clean; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows), `pack-goldens --check` (25), `gen-irule-test-data --check`, `command-backing --check` (389 commands, unchanged), `callback-inventory --check`, `audit-option-dialects --check` (114), `retired-api-gate`, `owner-resolution` (45), `kcs-index-links` green, `dialect-drift` at its 8. Docs: the design page (status box, the rung table's row 4, the rung-4 `RuntimeBacking` block with its `PackText { text }`, the rung-4 bullet, rule 4, the registration bullet), `docs/GLOSSARY.md` § *Runtime backing*, `spec-packs.md` § *What a pack still cannot say*, `spec-dsl-examples/README.md` (the keyword row and the override bullet), `irule-test-framework.md` decision rule 1. No KCS note: nothing a user runs changes today. Deviations: the text `PackText` carries (D7.3); the identity (D7.4); the rows and their test (D7.5); one spelling for six consumers, and the Studio kind sharing the `text` tag (D7.6); the notice's seat (D7.7); the generator's output moves (D7.8); the ports (D7.9). D7.3–D7.9 |
| CC7.3 the intrinsic table by family | landed | `wip(consumer-contracts): step 7 — the intrinsic families` | `rust/tcl-registry/src/intrinsic.rs`: `IntrinsicFamily` (`Value`; `FamilyB { domain: GuardDomain, fires_traces }`, with the two constants `STORES` and `OBSERVES_AND_FIRES`), `IntrinsicId::family` (an exhaustive per-member match with no wildcard, arms grouped by family), `IntrinsicFamily::guard_domains` and `fires_traces`, `IntrinsicId::from_guard_identity` and `required_guard_domains`; the crate gains a direct dependency on `tcl-runtime-api` for `GuardDomain` (it was in its graph through `tcl-cmd-core`, so no crate joins any graph). **The table.** 14 `Value` members (`llength`, `lindex`, `lrange`, `lreplace`, `linsert`, `list`, `concat`, `dict get` and `string` `index`, `range`, `equal`, `compare`, `replace`, `length`) and 14 `FamilyB`, every one under `VariableTrace`: `lassign`, `lset`, the five `dict` updates, `string is`, `regexp`, `info exists`, the three array queries and `puts` (D7.10); `fires_traces` only on `info exists` and the array queries, which `tclsh` 8.4 to 9.0 and `tcl-vm` answer with a read trace and an array trace respectively, and `runtime/rust` answers for the array queries only (reported). **The consumers** (D7.11). `tcl-runtime-api/src/guard.rs`: `GuardDomains::union` and `covers`, `GuardIdentity::registry_stable_id` (inverts both identity forms), `GuardError::DomainsInsufficient`, and the `VariableTrace` doc names the family. `tcl-compiler/src/backend_registry.rs`: `guard_domains_for_intrinsic` (the dispatch domains plus the family's), read by `mixed_region_plan.rs` where a guarded plan is built and where it is validated. Both runtimes' `prepare_command_guard` refuse a request that does not cover the family's domains, before any other check. No `SEMANTICS_REVISION` row moves (D7.12). Tests: `intrinsic.rs`'s `every_member_names_a_family` (the two lists, spelled out), `trace_firing_members_are_family_b` (the four) and `a_guard_request_must_cover_its_members_family_domains` (both identity forms across the five releases, a foreign vocabulary, an unknown id); `guard.rs`'s `a_registry_identity_names_its_intrinsic_in_both_forms` and `a_set_covers_exactly_the_domains_it_contains`; `backend_registry.rs`'s `an_intrinsics_guard_domains_add_its_familys_to_its_dispatch_dependencies`; in each runtime `a_family_b_guard_request_must_cover_the_variable_trace_domain` (a `DictSet` identity refused without the domain, a `ListLength` one issued without it, the covered `DictSet` request refused while a variable trace exists and stale once one is added); `tcl-vm`'s `every_trace_firing_intrinsic_fires_a_trace_here`. Mutation checks, each reverted and each failing its test: `ListLength` moved to Family B (`every_member_names_a_family`), `ArraySize` unmarked (`trace_firing_members_are_family_b`), Family B requiring no domain (`a_guard_request_must_cover_its_members_family_domains`, the compile-side test, and both runtimes' test), `registry_stable_id` ignoring the packed form and `covers` answering on any overlap (the two `guard.rs` tests), the check removed from each runtime's `prepare_command_guard` (that runtime's test), `ListLength` marked as firing (`every_trace_firing_intrinsic_fires_a_trace_here` panics naming it). Moved: nothing; the lockfiles that list `tcl-registry` gain one edge (the root's and `runtime/rust`'s by cargo, `bigip-query-wasm`, `bigip-report-gen/wasm` and `tcl-vm-wasm` by `cargo metadata --offline`, `bigip-report-gen/python` by hand); the four host lockfiles that were already stale (`tcl-explorer-wasm`, `tcl-lsp-server-wasi`, `tcl-lsp-server-wasm`, `tcl-spec-studio-wasm`) are left for the regeneration step 6 reported. Gates: `cargo test -p tcl-registry` (lib 954, was 951; `registry_sweep` 40; every other binary), `-p tcl-runtime-api` (lib 31, was 29), `-p tcl-compiler` (lib 6551, 2 ignored, was 6550; `codegen_integration` 17, `wasm_codegen` 45; `wasm_real_link` 13 and `wasm_tiers` 7 against the real runtime with `TCL_REQUIRE_WASM_LINK=1`), `-p tcl-vm` (lib 102, was 100; every other binary, `command_mutation_deopt_e2e` 76), `runtime/rust` (lib 708, was 707; its 12 integration binaries); `cargo check --workspace --all-targets`; clippy (`--workspace --all-targets -- -D warnings`), `cargo fmt --all` and `runtime/rust`'s `cargo fmt` clean; `registry-axes --check` (7831 / 16 / 36 / 893 across 147, unchanged), `value-transfers --check` (22 / 19 / 83 across 34, 6607 rows), `pack-goldens --check` (25), `command-backing --check` (389), `owner-resolution` (45), `retired-api-gate`, `kcs-index-links`, `dialect-drift` at its 8. Docs: the design page's status box and its intrinsic-table bullet, `family-b-routing.md` § 3 (*Intrinsic families*) and § 4 (one row), `docs/GLOSSARY.md` § *Guard identity*. Deviations: the classification and the `puts` domain (D7.10); the consumers, where the plan names `codegen_abi.rs` and `tcl-vm` (D7.11); no revision bump (D7.12); § 4 had no gap row to close (D7.13). D7.10–D7.13 |

### CC7.1 — what the next items read

- **For CC7.2 (the backing query and the gate).** The declared rows are
  the report's rows and `every_core_command_declares_a_backing` holds them
  equal to the committed report. Until CC7.2 renders the report from the
  spec, the report (from the source scan and the gate's four lists) and the
  specs are two sources kept equal by that test; once the report is a
  rendering of `spec.runtime_backing` against `backing_report()` the test is
  a tautology and may be deleted with the lists. Match on the variant, not
  on a category string: `ShippedBuiltin` carries the identity the runtime
  reports (`spec.name` as spelled, so `tcl::mathfunc::abs` and
  `::tcl::mathfunc::abs` are two identities), `TclBody` a `PackageSource`
  file the embedded library defines, `None` a command nothing executes.
  The 42 known-gap rows declare `ShippedBuiltin` and stay on
  `KNOWN_UNBACKED` as the drift waiver (D7.1); a spec that declares `None`
  and is registered by a runtime is drift, and every not-required row
  declares `None` by default.
- **For CC7.4 and the manifest.** `BackingSyntax::spelling` is the canonical
  text of a backing and round-trips through `parse_spelling`; `BodySource::PackText`
  holds the body, which a rung-3 claim compares against the live procedure
  (CC8.1).
- **For CC6.2.** `runtime_backing` is one of the declarations the
  capability matrix must forbid a `Transitive` or `Development` pack: the
  loader reads the statement at every tier, the floor keeps a shipped
  command's backing, and nothing else gates the statement yet.
- **Reported, not fixed.** The Studio's Test tab and `spectcl_check` do not
  raise the `-pack-text` notice, which `pack::load_sources` raises for a
  pack the server loads (D7.7).

### CC7.3 — what the next items read

- **For CC7.4.** `IntrinsicId::family` is a `const fn` over a `Copy` enum, so
  `intrinsic_table_hash` hashes `(stable_id, family, guard_semantics_variants())`
  over `IntrinsicId::ALL` (D7.12). The family wants one stable byte spelling for
  that, written where the hash is built and tested there: `Value`, or `FamilyB`
  with the domain's discriminant and the `fires_traces` flag. `GuardDomain` is
  `#[repr(u8)]` with explicit discriminants, which are the ABI's `domains` bits'
  positions and so stable.
- **For CC7.2.** The sweep attests identities and never issues guards, so the
  family does not reach it. A sweep that attests every command whose spec
  declares an intrinsic (`string` only, today) widens no fast path: `execute_intrinsic`
  serves `StringLength` alone, and a guard request for any other member declines
  at the runtime's own check.
- **Reported, not fixed.** `runtime/rust`'s `info exists` runs no read trace:
  `proc note {n1 n2 op} {lappend ::fired $op}; set v 1; trace add variable v read
  note; info exists v; puts [set ::fired]` prints `read` under `tclsh` 8.4 to 9.0
  and `tclvm`, and nothing under the runtime (`cmd_info.rs`'s `info_exists`, over
  `VarStore::exists`, which takes `&self`). The array queries fire their array
  traces in all three.

### Behavioural deltas accepted in step 7

- CC7.1: `CommandSpec` gains `runtime_backing`, so every command's `Debug`
  text and all 25 pack goldens' `spec` digests move once; every core Tcl
  command declares its row of the WASM command-backing report; a pack may
  write `runtime_backing`, and a `-pack-text` body draws an Information
  notice at load.
- CC7.1: the iRule-test stub table no longer lists 46 shared Tcl core
  commands and `pkg::create` (D7.8). The harness never dispatched them;
  `_mock_stubs.tcl` reads 933 stub actions, was 979.
- CC7.1: the Spec Studio gains a "Runtime backing" field in the Identity
  group, edited as text in the statement's own spelling.
- CC7.3: a guard request for a Family-B intrinsic that omits the
  variable-trace domain is refused by both runtimes (`GuardError::DomainsInsufficient`);
  before, it was issued if the identity matched. No compiler emits one, so no
  module a user runs changes. `GuardError` gains a variant, `GuardDomains` two
  methods and `GuardIdentity` one, and `tcl-registry` a direct dependency on
  `tcl-runtime-api`.

## Plan for steps 2–10

Steps 2 to 10 of [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
§ *Build order*, planned against the tree at `f3f9390f` on
`claude/spectcl-optimization-discussion-5qhf42`. The page is the design;
this section is the sequence an implementer follows and the reviewer checks
a landed item against. Items are numbered `CC<step>.<n>`. Each names its
files, its Rust items, what stays byte-for-byte and what changes, its tests
(with the negative case), the gates it touches, the documents it updates, a
model class (`opus` for design judgement or semantics, `sonnet` for
mechanical work from a given shape), a size (S: under a day; M: one to
three days; L: more), and the items it follows. Every item is one
`wip(consumer-contracts): …` checkpoint or a named part of one; the tree
compiles at every checkpoint (`cargo check --workspace`), the item's own
test binaries are green, and the lane stages only its own files by path.
The page's identifiers are used verbatim wherever the tree lets them be;
every adaptation is in § *Where the tree differs from the page* and in the
item that makes it.

### Where the tree differs from the page

Verified at the plan's revision. Each row is an adaptation the items use.

- `../spec-dsl-examples/if.tclspec` resolves to
  `docs/design/spec-dsl-examples/if.tclspec`; the examples directory is
  under `docs/design/`.
- `HandlerMatch`, `HandlerPlan`, `TemplateWordPlan`, `ScriptRegion`,
  `OperandId`, `AnalysisContext` and `Budget` already exist in
  `rust/tcl-registry/src/value_transfer/` (slice 1 of the value-transfers
  lane). The clause-grammar descriptor reuses
  `value_transfer::answers::HandlerMatch`; `MemberRow::body` is a
  `value_transfer::inputs::OperandId`. Neither type is declared twice.
- `OptionSpec` is declared in `rust/tcl-registry/src/hover.rs` and
  re-exported from `spec.rs`; its fields are `name`, `value`, `detail`,
  `surface`, `aliases`, `lifecycle`, `min_abbrev`. `OptionRelation` is
  `Relation<OptionTerm>` in `spec.rs`; `RelationKind::MutuallyExclusive`
  and `Relation::evaluate` exist; the loader reads `option_conflict`.
- `ResolvedInvocation<'r, 'w>` (`resolved_invocation.rs`) carries `words`,
  `canonical_command`, `subcommand`, `form` and `semantics`, and already
  answers `state_transitions()`, `effect_footprint()`, `facts()` and
  `validate_literal_arguments()`. The page's `RegistryQueries` trait is
  pseudocode by its own note; the layer is inherent methods on this type
  (decision D2.5).
- `Provenance` is `tcl_dialect::model::environment::Provenance`, not a
  registry type. The two `untrusted(…)` predicates are
  `rust/tcl-spectcl/src/loader/eval.rs:1784` (over `Tier`) and
  `rust/tcl-registry/src/model/registration.rs:287` (over `Provenance`).
- `PackEnvironmentTier::provenance` (`loader/environment_block.rs:109`)
  maps `Workspace` to `WorkspaceTrusted` unconditionally; `DiscoveryOptions`
  (`discovery.rs:182`) has no trust input; the server builds it in
  `spec_pack_discovery` (`rust/tcl-lsp-server/src/lib.rs:20837`) and reads
  `initializationOptions` in `apply_initialization_options` (`lib.rs:11179`).
- Hook bodies reach the host through `tcl_spectcl::hooks::plan_for` /
  `publish` (`rust/tcl-spectcl/src/hooks.rs`) and
  `HookHost::install_pack_hooks` (`rust/tcl-spec-hooks/src/host.rs:134`);
  `HookInstallation::declined` is the existing "not installed" record;
  `HookDecl` (`loader.rs:735`) carries no line number.
- `StubFlags` and `StubCommandDef` are in
  `rust/tcl-compiler/src/analyser/types.rs`; `parse_stub_flags` is in
  `analyser/utils.rs:1289`; `DocumentCommandSurface::arg_indices_for_role`
  (`model/declaration.rs:349`) unions the catalogue's and the document's
  answers.
- The shim exports its 34 symbols with `#[unsafe(export_name = "Tcl_…")]`
  in `rust/tcl-cshim/src/ffi.rs`; `runtime/rust/src/capi.rs` has 18
  `#[no_mangle]` exports (the page's 17 plus `tcl_test_finalize`);
  `runtime/rust/include/` holds only `tcl_regex_capi.h`, so no authored
  `tcl.h` exists anywhere (the ABI's § 4.1 says the runtime ships it).
- `EvalSnapshotKey::content_hash` is a `u64` xxh3, not `[u8; 32]`
  (decision D4.3).
- `CompiledUnit` (`rust/tcl-vm/src/compiled.rs`) carries the three
  generations the page names; `ModuleAsm` (`rust/tcl-bytecode/src/lib.rs`)
  carries `profile`, `source`, `source_namespace`,
  `plain_command_dispatch`, `top_level`.
- The WASM emitter encodes modules itself
  (`rust/tcl-compiler/src/codegen/wasm/encoding.rs`); no `wasm-encoder`
  dependency exists.
- `command_backing.rs`'s four lists exist; the report
  `docs/generated/wasm-command-backing.md` classifies 389 commands (105
  handler, 177 native, 11 stdlib, 54 not-required, 42 known-gap).
- `runtime/rust` is the crate `tcl-runtime`; `xtask` depends on neither it
  nor `tcl-vm`.
- `tcl-pkg` depends on no registry crate and `tcl-spectcl` does not depend
  on `tcl-pkg`; discovery finds a pack beside a `tclpkg.tcl`
  (`Origin::BesideManifest`) without reading the manifest.
- `TclVersion::from_profile` (`rust/tcl-dialect/src/version.rs:181`)
  answers for the five plain names; `DialectProfile::runtime_base` carries
  a vendor profile's measured base (`f5-irules` is `V8_4`);
  `value_transfer::const_ops` reads the release through `from_profile`
  (`const_ops.rs:325`).
- `Engine` (`rust/tcl-engine-api/src/lib.rs:250`) has no `set_release`;
  slice 4 of the value-transfers lane adds it (boundary B3).
- The analyser consumes 43 `AnalyserHookId` variants in
  `dispatch_analyser_hook` (`analyser/commands.rs:1294`); a generic
  role-driven binding pass, `handle_var_binding_command`, already runs ahead
  of the hook dispatch (`commands.rs:1139`).
- `MethodKind::from_str_lossy` has one live call (`lowering/mod.rs:3931`),
  fed by `member_method_kind` (`lowering/mod.rs:4009`), a keyword match;
  `SwitchMode::from_str_lossy` (`ir.rs:1504`) is the case-list axis's twin
  and goes on the ledger, not into this lane.
- `CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC` and `CLAUSE_NOISE_KEYWORDS`
  (`traits.rs:1397`, `:1412`) have three consumers outside the registry:
  `rust/xtask/src/gen_tmlanguage_keywords.rs`,
  `rust/tcl-lsp-core/src/semantic_tokens.rs`,
  `rust/tcl-lsp-core/src/minify.rs`.
- The clause-carrying commands' native resolvers are `if_arg_roles` /
  `check_if_shape` (`if_.rs`), `try_arg_roles` (`try_.rs`),
  `foreach_arg_roles` (`foreach_.rs`), `lmap_arg_roles` (`lmap_.rs`);
  `catch`, `for`, `while`, `dict for`, `dict map`, `dict update`,
  `array for` use positional `arg_roles`. E004 reads
  `spec.clause_shape_check` at `analyser/commands.rs:1708`.
- The studio's coverage witness (`rust/tcl-spec-studio/src/coverage.rs`)
  destructures `CommandSpec`, `SubCommand` and `OptionSpec` exhaustively, so
  a new registry field breaks the studio's build until the witness, the
  `Field` table and the schema entry land in the same checkpoint.

### Step 2 — the description contract

**Goal.** The derived-query layer, the clause-grammar and member-effect
descriptors, the option-effect descriptor with its four clients (`subst`,
`lsearch`, `regexp`, `CaseListSpec`), the consumer migration across every
tier onto the generic operations, the per-axis lint with its generated
ledger, and `semantic_operation`, `definition_body` and `clause_grammar`
round-tripping through the studio with no `GAPS` row.

**Exit evidence.**

- `cargo xtask registry-axes --check` is green with `docs/generated/registry-axes.md`
  committed and every pin at or below its CC2.1 baseline; the files this
  step rewrites are held clean.
- `cargo test -p tcl-registry` green, including the new
  `clause_grammar`, `option_effect` and `definer` unit rows, the
  `registry_sweep` agreement rules, and `analyser_hooks` re-baselined to
  CC2.13's 32 variants on 43 stamp rows.
- `cargo test -p tcl-spec-studio --test spectcl_roundtrip --test spectcl_ports --test option_row_editing --test reference_doc`
  green with `GAPS` no longer naming `semantic_operation`,
  `definition_body`, `substitution_resolver` or `pattern_arg_resolver`, and
  `docs/references/command-spec/fields.md` regenerated.
- `cargo test -p tcl-compiler --test analyser --test cfg --test mro_lattice_adversarial --test codegen`
  green and byte-identical on every existing case; `cargo test -p tcl-lsp-server --test preview_tickets_e2e`
  carries the new-member-spelling witness.
- `cargo xtask pack-goldens --check`, `gen-tmlanguage-keywords --check`,
  `callback-inventory --check`, `value-transfers --check`, `owner-resolution`
  and `kcs-index-links` green; `make codegen` produces no diff except the
  files this step regenerates on purpose (none: the TextMate keyword lists
  are pinned byte-identical).

#### Work items

**CC2.1 — the per-axis lint and its ledger.**
Files: `rust/xtask/src/registry_axes.rs` (new), `rust/xtask/src/main.rs`
(`Command::RegistryAxes { check }` beside `ValueTransfers`), `Makefile`
(`xtask-registry-axes` target in `xtask-check`, after
`xtask-value-transfers`), `docs/generated/registry-axes.md` (new),
`docs/design/contracts/shared-utility-contracts-rust.md` (owner manifest
row: surface *command vocabulary outside the registry*, owner
`rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/clause_grammar.rs`;
`rust/tcl-registry/src/definer.rs`, entry points `CommandRegistry`,
`clause_keywords`, `DefinitionBodyGrammar`, axis "per dialect surface",
gate `xtask-registry-axes`).
Rust: `pub fn run(check: bool) -> Result<ExitCode>`; `const LINT_ROOTS`
copied from `value_transfers.rs` (the ten crates; the two lists are
merged into `rust/xtask/src/util.rs` by CC2.15 once the value-transfers
lane's slice 2 lands, boundary B1); `const AXES: &[&str] = &["command",
"subcommands", "clause_grammar", "definition_body", "options",
"special_vars", "irreducible"]` (the names of the migration plan's
§ *Debt on other axes* table, so a site waived for both gates names one
axis); `fn vocabulary(reg: &CommandRegistry) -> Vocabulary` built from the
`analyser_hooks.rs` `full_registry()` shape (every loadable dialect plus
`specs/`): every command and subcommand name, every `OptionSpec::name` and
alias at command, subcommand and form level, every `MemberSpec::keyword` of
every `DefinitionBodyGrammar` the specs reference, the clause keywords
(`CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC` ∪ `CLAUSE_NOISE_KEYWORDS` until
CC2.2 replaces them with the grammars' rows), every `SpecialVarSpec` name;
`fn scan(text) -> Vec<Site>` recognising a vocabulary word used as the
operand of `==` / `!=` / `matches!` / a `match` arm / `.eq(` / a `&[&str]`
constant table entry compared later — the same shapes `value_transfers::scan`
recognises, with the operand set widened from command heads to the
vocabulary; the waiver marker `// registry-axis-ok: <axis> — <reason>; until <step N | slice N | never>`
(the expiry the page requires; `never` only with `irreducible`), a file
waiver `// registry-axis-ok(file): …`; `RATCHET: &[(&str, usize)]` pinned
from the first write-mode run and `CLEAN_FILES` starting empty; `--check`
fails on a rising count, a stale pin, an unknown axis, a missing expiry,
or an expiry naming a step or slice that has landed (the landed set is a
`const` the lane bumps). The report renders the ledger: every waiver by
axis with its expiry, and the ratchet table.
Preserve: nothing observable changes. Tests: `rust/xtask/src/registry_axes.rs`
unit tests — `a_clause_keyword_compared_by_spelling_is_a_site`,
`a_member_keyword_in_a_match_arm_is_a_site`, `an_option_spelling_in_a_const_table_is_a_site`,
`a_waiver_without_an_expiry_fails`, `a_registry_owner_file_is_never_scanned`
(negative: `rust/tcl-registry/**` and every path under a `tests/` directory
and `#[cfg(test)]` module are excluded, exactly as `value_transfers.rs`
excludes them). Gates: the new gate; `owner-resolution` (the manifest row
must name a live gate). Docs: a `## The per-axis lint` paragraph in
`docs/design/compiler/registry-consumer-contracts.md` § *The per-axis
lint and ledger* replacing "The ledger is generated" with the marker and
the file; KCS `docs/kcs/kcs-issue-the-registry-axes-gate-reports-a-keyword.md`
(the shape of `kcs-issue-the-value-transfers-gate-reports-a-command-name.md`),
indexed in `docs/kcs/README.md` § Issues. Model: opus. Size: M. After: —.

**CC2.2 — `ClauseGrammarSpec` in the registry.**
Files: `rust/tcl-registry/src/clause_grammar.rs` (new), `spec.rs`
(`CommandSpec::clause_grammar: Option<&'static ClauseGrammarSpec>`,
`SubCommand::clause_grammar: Option<&'static ClauseGrammarSpec>`, both in
`DEFAULT` as `None`), `lib.rs` (`pub mod clause_grammar`), `registry.rs`
(`arg_indices_for_role` and `arg_indices_for_role_words` consult the
grammar first: the resolution order becomes `clause_grammar` →
`arg_role_resolver` → `arg_roles` → `assigns_variable_at`; `AGENTS.md`
§ *Command registry* gains the word), `resolved_invocation.rs`
(`ResolvedInvocation::clause_plan(&self) -> Option<ClausePlan>`),
`traits.rs` (the two clause tables become `pub fn clause_keywords_without_command_spec() -> &'static [&'static str]`
and `clause_noise_keywords()` derived once from the shipped grammars and
memoised, with a unit test pinning them equal to today's two lists),
`commands/tcl/{if_,try_,catch_,for_,while_,foreach_,lmap_,dict,array_}.rs`
(the ten grammars as `pub const … : ClauseGrammarSpec`; `if_arg_roles`,
`check_if_shape`, `walk_if`, `try_arg_roles`, `foreach_arg_roles`,
`lmap_arg_roles` deleted; `arg_role_resolver: None` on those four specs;
`arg_role_resolver_roles` stays as the closed set the walk may emit;
`if`'s `clause_shape_check: None`), `rust/tcl-spec-studio/src/coverage.rs`
(`clause_grammar: _` in `witness_command_spec` and `witness_sub_command`;
`f("clause_grammar", Surface::Key("clause_grammar"))` in `COMMAND_SPEC`
and `SUB_COMMAND`), `rust/tcl-spec-studio/src/draft.rs` (seeding writes the
grammar as a JSON value — plain data, never `UNRENDERABLE_KEY`),
`rust/tcl-spec-studio/src/schema.rs` (a `clause_grammar` field of a new
`FieldKind::ClauseGrammar`, cluster "Structure" in `relations.rs`),
`rust/tcl-spec-studio/src/help.rs` (long-form help with the `try` example
from the page), `rust/tcl-spec-studio/src/render_spectcl.rs`
(`clause_grammar_block` beside `object_class_block`, emitting the CC2.3
spelling; a transient `GapKind::LoaderGap` row for `clause_grammar` is
**not** added — CC2.2 and CC2.3 land as one checkpoint so the round trip
never sees the field without its reader).
Rust: the page's types verbatim — `ClauseGrammarSpec`, `ClauseRow`,
`ClauseRowShape`, `ClauseSlot`, `ClauseTiming`, `LoopPhase`,
`ClauseSelection`, `DefaultClause`, `ClausePlan`, `ResolvedClause`,
`ClauseRowId` — with two adaptations: `ClauseGrammarSpec::head` is a
`ClauseRow` (keyword `None`, `keyword_required: false`, shape `Once`) so
the head carries the timing the page's own DSL row `head {Body} -timing protected`
declares (D2.1); `ClauseSlot::handler: Option<value_transfer::HandlerMatch>`
(D2.2). `pub enum ClauseRowId { Head, Row(u8), Tail }`. The walk,
`ClauseGrammarSpec::walk(&self, args: &[&str], layouts: &[RepeatedArgLayout]) -> ClausePlan`,
is the loader's `ClauseGrammar::walk` (`rust/tcl-spectcl/src/loader.rs:3840`)
moved into the registry and extended: the head is filled positionally; at
each position a `Repeated` row whose keyword matches starts, else the next
unfilled keywordless `Once` row in declaration order, else the tail, else
`ExtraWords`; a `Group` row consumes `layout.stride` words at a time until
`args.len() - layout.exclude_trailing`; a `?noise?` slot consumes its word
when present; `fallthrough_body` sets `falls_through_to` on the clause
whose body word equals it (the next clause with a body); `defect` is the
existing `ClauseShapeError`. `pub fn clause_keywords(reg) -> …` is the
derivation the traits tables move onto. The ten grammars: `if` and `try`
as the page spells them; `catch` — head `Once{[Body]}@Protected`, tail
`Once{[LoopVarList optional, LoopVarList optional]}@Selected`; `for` —
head `Once{[Body]}@LoopFixture(Init)`, rows `Once{[Expr]}@Selected`,
`Once{[Body]}@LoopFixture(Next)`, tail `Once{[Body]}@PerIteration` (D2.1);
`while` — head `Once{[Expr]}@Selected`, tail `Once{[Body]}@PerIteration`;
`foreach` / `lmap` / `dict for` / `dict map` — head empty, rows
`Group{layout: 0}@PerIteration`, tail `Once{[Body]}@PerIteration`;
`dict update` — head `Once{[Value]}` (the dict variable), rows
`Group{layout: 0}@PerIteration` with `conditional_binding: true` on the
var-list slot, tail `Once{[Body]}@Selected`; `array for` — as `dict for`
with `surface: Some(TCL90_PLUS)` on the grammar. Each grammar's role
answers equal the retired resolver's on that command's whole test matrix.
Preserve: every `arg_indices_for_role` answer and every
`ClauseShapeError` for every call shape in `if_.rs`'s 24 tests and
`try_.rs`'s tests (moved into `clause_grammar.rs` as `const` corpora over
the shipped grammars); E004 positions; the semantic-token keyword set; the
TextMate keyword lists. Changes: nothing observable. Tests (new, in
`clause_grammar.rs`): `if_grammar_agrees_with_the_retired_walk_on_its_corpus`
(24 rows), `try_grammar_agrees_with_the_retired_walk` (including `on ok`,
`trap {POSIX ENOENT}`, a `-` body), `foreach_group_row_cites_layout_zero`,
`array_for_is_absent_below_9_0` (negative: under `tcl8.6` `clause_plan`
answers `None`), `a_keyword_never_matches_inside_a_slot` (`if else {a}`);
`rust/tcl-registry/tests/registry_sweep.rs` gains
`clause_grammar_group_rows_cite_a_real_layout` and
`conditional_binding_clause_slots_never_carry_var_write` (the sibling of
`repeated_arg_layouts_never_pair_conditional_binding_with_an_ssa_def_role`)
and `clause_slots_use_only_the_six_roles`; `traits.rs` gains
`derived_clause_keyword_tables_equal_the_pinned_lists`. Gates:
`gen-tmlanguage-keywords --check` (byte-identical), `registry-axes`
(CC2.1's pins unchanged), `pack-goldens --check`, `reference_doc`
regenerated. Docs: `docs/design/compiler/command-registry.md` § *CommandSpec
field reference* gains `clause_grammar`; `registry-consumer-contracts.md`
§ *The clause-grammar descriptor* "today" sentences flipped. Model: opus.
Size: L. After: CC2.1.

**CC2.3 — `clause_grammar` in the loader, renderer and studio.**
Files: `rust/tcl-spectcl/src/loader.rs` (`ClauseGrammar` and `ClauseWalk`
deleted; `clause_grammar_block` builds a `ClauseGrammarSpec` with `leak_slice`
/ `leak_str`; `PackCommand::clause_grammar: Option<&'static ClauseGrammarSpec>`;
the derivation hooks recorded as today with `HookSource::Derived`;
`abstain_arg_roles` / `accept_clause_shape` no longer installed for a
grammar-carrying command — the registry walk answers, so the two placeholders
stay only for `-native` escapes), the subcommand block (`clause_grammar`
on a `subcommand` body), `docs/design/spec-dsl-examples/README.md`
(§ *Clause grammars, declaratively* and the coverage matrix row: `head {slots} ?-timing T?`,
`repeated KEYWORD {slots} ?-timing T? ?-pattern completion-code|error-code-prefix? ?-optional-keyword? ?-available V?`,
`once ?KEYWORD? {slots} ?-timing T?`, `group N ?-timing T?`,
`tail ?KEYWORD? {slots} ?-timing T?`, `fallthrough_body WORD`,
`default_clause ROW|tail ?-final-only?`, `selection first-match|all`;
timings `selected|always|per-iteration|init|next|protected`; a slot word
`?noise?` as today, `Pattern` carries the row's `-pattern`, a slot
`-conditional` suffix spelt `LoopVarList?` is **not** introduced: the
`-conditional` flag sits on the row and applies to its var-list slot),
`docs/design/spec-dsl-examples/{if,foreach}.tclspec` (`if` gains
`-timing`; `foreach` gains `clause_grammar { group 0 -timing per-iteration; tail {Body} -timing per-iteration }`),
`rust/tcl-spec-studio/src/render_spectcl.rs` (the block writer),
`rust/tcl-spec-studio/src/store.rs` (the `clause_grammar` presence flag
becomes the field itself), `rust/tcl-spec-studio/src/examples/fields_behaviour.rs`
(the example), `rust/tcl-spec-studio/web/src/editors.ts` (a read-only
structured view of the rows is enough for this step; editing rows is an
open question Q2).
Preserve: every existing port loads to the same spec; `spectcl_roundtrip`'s
`every_command_in_every_dialect_round_trips_through_spectcl` passes with
no `clause_grammar` gap. Tests: `rust/tcl-spec-studio/tests/spectcl_ports.rs`
— `the_clause_grammar_derivation_agrees_with_the_shipped_walk` widened to
`if`, `foreach` (both from the examples) and, over the shipped grammars,
every grammar-carrying command (the corpora of CC2.2), asserting roles and
defect per call; `derivations_are_recorded_as_derivations` unchanged;
`rust/tcl-spectcl/tests/eval_loader.rs` gains `a_clause_grammar_row_with_an_unknown_timing_is_dropped_with_a_notice`
(negative). Gates: `pack-goldens --check` (the EDA packs declare no
grammar; no snapshot moves), `reference_doc`. Docs: the memo rows above;
`docs/design/contracts/command-spec-studio.md` needs no change (no `GAPS`
row). Model: opus. Size: M. After: CC2.2 (same checkpoint).

**CC2.4 — `MemberEffect` in the registry.**
Files: `rust/tcl-registry/src/definer.rs` (`MemberSpec::effect: MemberEffect`,
required; the page's `MemberEffect`, `MemberReceiver`, `CallableRole`,
`StateScope`, `RelationSlot`, `InitTiming` verbatim; `MemberRow`,
`MemberArity`; `DefinitionBodyGrammar::member_row(&self, keyword_index: usize, words: InvocationArguments<'_>, dialect: Option<SurfaceQuery<'_>>) -> Option<MemberRow>`
— one member statement at a time, the shape the analyser already asks
(D2.6); every constructor (`flat`, `all_refs`, `all_vars`, `keyword_only`,
`wrapper`, `wrapper_or_body`, `flag_keyed`) takes the effect or a builder
`.effect(…)` sets it; the TclOO, snit and itcl rows get the effects the page
lists — `method`/`classmethod`/`typemethod`/`proc`/`constructor`/`destructor`/
`onconfigure`/`oncget` `Callable` with `name_slot`, `params_slot`,
`body_slot` positioned by their `arg_roles`' `Name`/`ParamList`/`Body`;
`forward` `Forward{name_slot: 0, prefix_slot: 1}`; `variable`/`typevariable`/
`common`/`component`/`typecomponent` `StateDeclaration`; `option`
`StateDeclaration{scope: Option}`; `superclass`/`mixin`/`filter`/`inherit`
`Relation`; `export`/`unexport` `Visibility`; `deletemethod`/`renamemethod`
`Retraction`; `typeconstructor` `InitScript{body_slot: 0, timing: AtDefinition}`;
`initialise`/`initialize` `InitScript{…, AtDefinition}`; `self`/`private`/
`public`/`protected` (wrappers) and `definitionnamespace`/`delegate`/`expose`/
`property` `Configuration`; every SpecTcl and SslicTcl document-grammar
member `Configuration` (D2.7)), `rust/tcl-spec-studio/src/coverage.rs`
(a new `witness_member_spec` and `MEMBER_SPEC` table, wired into the
`definition_body` draft once CC2.5 seeds it).
Preserve: `MemberSpec::indices_for_call*`, `option_for*`,
`declared_visibility_for*` unchanged. Tests: `rust/tcl-registry/tests/registry_sweep.rs`
gains `every_member_carries_an_effect_that_agrees_with_its_roles` (the
page's five rules: a `Callable` row's three slots index `Name`,
`ParamList`, `Body` in `arg_roles`; a `Relation` row carries `slot`; a
`Retraction` row carries `retraction`; a `Visibility` row carries
`visibility_effect`; a `Forward` row's prefix slot is `CommandPrefix`),
and `no_member_effect_names_a_family` (negative: the enum's `Debug`
strings contain none of `TclOo`, `Snit`, `Itcl`); `definer.rs` unit rows
for `member_row` on `method m {a b} {…}`, `self method`, `superclass -append B`
(`slot_op`), `forward f ::x`, a dynamic name word (`name: None`).
Gates: none beyond the crate tests. Docs: `command-registry.md`
§ *CommandSpec field reference* (`definition_body` paragraph gains the
effect); `docs/design/contracts/tcloo-implementation.md` if it lists
member rows (the implementer checks and updates the row table). Model:
opus. Size: M. After: CC2.1.

**CC2.5 — `definition_body` and `semantic_operation` leave `GAPS`;
`-effect` on the member row.**
Files: `rust/tcl-spectcl/src/loader.rs` (`member_row` reads `-effect {callable -receiver R -role K ?-name N? ?-params N? ?-body N?}` / `{forward -name N -prefix N}` / `{state-declaration per-instance|per-type|option}` / `{relation superclass|mixin|filter}` / `visibility` / `retraction` / `{init-script -body N -timing at-definition|at-construction}` / `configuration`,
the page's spelling; a row without `-effect` is a notice and the member is
dropped), `rust/tcl-spec-studio/src/draft.rs` (seeding writes a shipped
grammar by name when the pointer is one of `TCLOO_GRAMMAR`,
`TCLOO_CONFIGURABLE_GRAMMAR`, `SNIT_GRAMMAR`, `SNIT_WIDGET_GRAMMAR`,
`ITCL_GRAMMAR` (`std::ptr::eq`) and the full block otherwise; seeding
writes `semantic_operation` as `{kind, detail}` from `kind_str` /
`detail_str`; both keys leave `UNRENDERABLE_KEY`),
`rust/tcl-spec-studio/src/render_spectcl.rs` (`definition_body NAME` or
the inline block with `-effect` on every member; `semantic_operation Invoke|{Intrinsic ID}|{StructuredLowering ID}`;
the two `GAPS` rows deleted), `schema.rs` / `help.rs` / `examples/` (the
member-row form gains the effect control; `semantic_operation` becomes a
closed-vocabulary field), `rust/tcl-spec-studio/web/src/editors.ts`
(member-row effect select), `docs/design/spec-dsl-examples/README.md`
(matrix rows; `snit-type.tclspec` and `oo-class.tclspec` gain `-effect`).
Preserve: every port loads to the same grammar. Tests:
`spectcl_roundtrip.rs` — `every_command_in_every_dialect_round_trips_through_spectcl`
with two fewer gap keys (the report's gap register shrinks);
`the_eleven_port_fixtures_render_and_reload_as_themselves` unchanged;
`spectcl_ports.rs` gains `a_member_row_without_an_effect_is_dropped_with_a_notice`
(negative); `rust/tcl-spectcl/tests/eval_loader.rs` gains
`semantic_operation_round_trips_through_the_renderer`. Gates:
`pack-goldens --check` (snapshots unchanged unless an EDA pack declares a
`definition_body`; the implementer regenerates and stages if one does),
`reference_doc`. Docs: `command-spec-studio.md` § *Fields that cannot
round-trip* loses the two rows; `registry-consumer-contracts.md` § *The
studio round-trip for `semantic_operation`* flips to past tense. Model:
opus. Size: M. After: CC2.4.

**CC2.6 — `OptionEffect` in the registry, and the four clients.**
Files: `rust/tcl-registry/src/option_effect.rs` (new: `OptionEffect`,
`OptionEffectKind`, `EffectAxis`, `SubstitutionKind`, `OptionEffectFamily`,
`FamilyBase`, `FamilyCombine`, `OptionEffects`, verbatim from the page;
`pub fn option_effects(spec_options: &[OptionSpec], families: &[OptionEffectFamily], args: InvocationArguments<'_>, reserved_trailing_words: usize, dialect: Option<SurfaceQuery<'_>>) -> OptionEffects`
— the walk of `lsearch_pattern_args` made generic: the scan ends at
`reserved_trailing_words` before the end, spellings resolve through
`resolve_available_option_prefix` (made `pub(crate)` for the module), an
unresolvable hyphenated word or a non-literal word makes `complete: false`
and every axis value `true`; `Accumulate` applies each effect,
`LastWins` keeps the last accepted; `EndsOptions` stops the scan;
`SuppressesRole` and `ReservesTrailingWords` are reported as `shifts`),
`hover.rs` (`OptionSpec::effect: Option<OptionEffect>`), `spec.rs`
(`CommandSpec::option_effect_families: &'static [OptionEffectFamily]`,
`SubCommand::option_effect_families`; `CaseListSpec` loses `regex_option`,
`exact_option`, `glob_option`, `nocase_option`, `end_options_option`;
`CaseListSpec::invocation` reads the command's `OptionEffects` — axes
`Selection(mode)` and `CaseSensitivity`, and `EndsOptions` — through a
new parameter `effects: &OptionEffects`; `substitution_resolver` deleted
from `CommandSpec`; `pattern_arg_resolver` stays), `substitution.rs`
(`SubstitutionResolver` and `subst_substitutions` deleted;
`SubstitutionKinds` stays; the three `tp_*` unit rows move to
`option_effect.rs` as rows of the generic derivation), `patterns.rs`
(`pattern_args(spec, inv) -> Vec<PatternArg>`: the resolver escape hatch
first, else the projection of `EffectAxis::PatternLanguage` onto the
pattern operand at `option_end + 1`), `registry.rs`
(`substitutions_performed` becomes the projection of `option_effects`
onto `SubstitutionKinds`; signature unchanged; the mixed-family call no
longer answers `ALL` silently — see the relation below),
`resolved_invocation.rs` (`ResolvedInvocation::option_effects(&self) -> OptionEffects`,
`pattern_args(&self)`), `commands/tcl/subst_.rs` (six rows gain `effect`,
two families `negated {AllOn, Accumulate}` and `positive {AllOff, Accumulate, surface: TCL91}`,
`reserved_trailing_words: 1`, and one `OptionRelation` of
`RelationKind::MutuallyExclusive` over the two term sets — W147 at the
call site, produced by the existing relation check in
`analyser/diagnostics/validity.rs:2127` with no new consumer code),
`commands/tcl/lsearch_.rs` (`lsearch_pattern_args` deleted;
`pattern_arg_resolver: None`; `-glob`/`-regexp` `Selects(PatternLanguage(…))`,
`-exact`/`-sorted` `Disables(PatternLanguage(Glob))`, family `match {Only(PatternLanguage(Glob)), LastWins}`;
`reserved_trailing_words: 2` stays), `commands/tcl/regexp_.rs`
(`-inline` `SuppressesRole(VarWrite)`, `-about` `ReservesTrailingWords(1)`;
`regexp_arg_roles` stays as the argument-role hook body of § *The two
hook bodies that remain* and reads `option_effects().shifts` for the
suppression and the reservation instead of assigning `VarWrite` to every
trailing word and assuming two reserved operands), `commands/tcl/switch_.rs`
(`-exact`/`-glob`/`-regexp` `Selects(Selection(…))`, `-integer`
`Selects(Selection(Other))`, `-nocase` `Selects(CaseSensitivity)`, `--`
`EndsOptions`, family `match {Only(Selection(Exact)), LastWins}`),
`commands/tcl/case_.rs` and the expect specs (their `CaseListSpec`s lose
the five fields; `case` has no options; expect's clause flags are not
options and stay on the descriptor), `rust/tcl-spec-studio/src/coverage.rs`
(`effect: _` in `witness_option_spec`; `f("effect", Surface::Key("effect"))`;
`option_effect_families: _` in the two spec witnesses; the deleted
`substitution_resolver` leaves `COMMAND_SPEC`, `schema.rs`, `help.rs`,
`draft.rs`, `relations.rs`, and its `GAPS` row; `pattern_arg_resolver`
keeps its schema entry and loses its `GAPS` row because no shipped spec
sets it — the round trip therefore never sees it set).
Preserve: every `substitutions_performed` answer for a single-family call
(the rows moved from `substitution.rs`), every `lsearch_pattern_args` case
including `lsearch -regexp -glob` searching the list `-regexp` for the glob
pattern `-glob`, every `regexp` role answer for calls without `-inline` or
`-about`, every `CaseListSpec::invocation` answer. Changes: a mixed-family
`subst` call answers `complete: false` and every kind on (W102 advice
unchanged) **and** draws W147; `regexp -inline … var` no longer assigns
`VarWrite` to the trailing words (they are an error the relation reports:
add `OptionRelation` `-inline` forbids positional 2+, message from
`tclsh`); `regexp -about exp` resolves its expression with one reserved
operand. Tests: `option_effect.rs` unit rows (`no_switches_runs_every_substitution`,
`a_negated_switch_turns_off_only_its_own_kind`, `a_positive_switch_turns_on_only_its_own_kind`,
`the_two_families_mixed_are_unreadable_and_every_kind_is_on`,
`an_abbreviation_the_table_cannot_resolve_is_unreadable`,
`a_computed_switch_word_is_unreadable`, `lsearch_last_style_wins`,
`lsearch_regexp_glob_scans_no_option_past_the_reserved_words`,
`switch_dash_dash_ends_options`); `registry.rs`'s
`substitutions_performed_answers_per_call_and_only_for_substituting_commands`
unchanged; `registry_sweep.rs` gains `every_option_effect_names_a_declared_family`
and `a_family_base_mentions_only_axes_its_options_mention`;
`rust/tcl-registry/tests/tcl91_dialect.rs` gains
`subst_positive_family_is_91_only` and
`subst_mixed_families_error_matches_tclsh91` (the differential row: runs
`subst -nocommands -variables x` on the `tclsh9.1` the value-transfers
lane's `differential_fold.rs` locates through `PATH`, asserts the error
message the relation's `-message` carries; skips with a note when no
`tclsh9.1` is on `PATH`, as that file does); `regexp_.rs` gains
`inline_suppresses_the_capture_var_writes` and `about_reserves_one_operand`.
Gates: `audit-option-dialects --check` (the option rows keep their
surfaces), `callback-inventory --check`, `pack-goldens --check`,
`reference_doc`. Docs: `command-registry.md` § *OptionSpec and option
terminators* gains the effect and the family; `registry-consumer-contracts.md`
§ *Options with semantic effects* "today" sentences flipped. Model: opus.
Size: L. After: CC2.1.

**CC2.7 — `option -effect` and `option_effect_family` in the loader,
renderer and studio.**
Files: `rust/tcl-spectcl/src/loader.rs` (`option_row` reads
`-effect {disables|selects AXIS VALUE} | {suppresses-role ROLE} | {reserves-trailing-words N} | ends-options`
and `-family NAME`; the pack-level and subcommand-level
`option_effect_family NAME { base all-on|all-off|{only AXIS VALUE} combine accumulate|last-wins ?-introduced V? }`
statement; axes spelt `substitution backslashes|commands|variables`,
`pattern-language glob|regex|…` (the `PatternType` names), `case-sensitivity`,
`selection exact|glob|regexp|other`), `render_spectcl.rs` (`option_row`
emits `-effect` / `-family`; a `family` writer), `schema.rs` / `help.rs` /
`examples/` / `relations.rs` (the option-row form gains the two controls;
the families a list field under "Options"), `rust/tcl-spec-studio/web/src/editors.ts`
(the controls patch non-structurally: `patch({effect: …}, false)`),
`docs/design/spec-dsl-examples/README.md` (§ *Option rows* and the matrix),
`docs/design/spec-dsl-examples/string.tclspec` or a new
`subst.tclspec` port carrying the page's `command subst { … }` example
verbatim (the port is the round-trip fixture).
Preserve: every existing pack loads unchanged. Tests:
`rust/tcl-spec-studio/tests/option_row_editing.rs` gains
`option_effect_edits_are_marked_non_structural` and
`switching_the_effect_kind_still_rebuilds`; `spectcl_ports.rs` gains
`the_subst_port_answers_the_same_kinds_as_the_shipped_spec` over the
`option_effect.rs` corpus; `eval_loader.rs` gains
`an_effect_naming_an_undeclared_family_is_a_notice` (negative). Gates:
`pack-goldens --check`, `reference_doc`. Docs: `command-spec-studio.md`
§ *Fields that cannot round-trip* loses `substitution_resolver` and
`pattern_arg_resolver`. Model: sonnet (the spelling is given by CC2.6 and
the page). Size: M. After: CC2.6.

**CC2.8 — the derived-query layer.**
Files: `rust/tcl-registry/src/resolved_invocation.rs` (inherent methods on
`ResolvedInvocation`: `clause_plan` (CC2.2), `option_effects` and
`pattern_args` (CC2.6), `case_invocation(&self) -> Option<(CaseInvocation, Vec<InlineCaseClause>)>`
re-keying `CaseListSpec::invocation` and `inline_clauses`,
`frame_effect(&self) -> Option<(FrameLevel, Vec<OperandId>)>` re-keying
`FrameEffectSpec` over the call's words, `arg_roles(&self) -> Vec<(usize, ArgRole)>`
re-keying `arg_indices_for_role_words` with the CC2.2 order,
`return_type(&self) -> Option<TclType>`, `effects(&self) -> EffectFootprint`
(the existing `effect_footprint`, renamed with a deprecated alias for one
checkpoint); `member_rows` is `DefinitionBodyGrammar::member_row` (D2.6);
`template_plan` is not added — slice 5's (D2.4)), `value_transfer/context.rs`
(`AnalysisContext::surface_query(&self) -> Option<SurfaceQuery<'_>>` from
`profile`), `registry.rs` (`CommandRegistry::invocation(&self, words: InvocationWords<'w>, ctx: &AnalysisContext) -> StructuredInvocationResolution`,
the page's `invocation(words, ctx)`: `resolve_structured_invocation(words, ctx.surface_query())`).
The three rules hold by construction: every answer is a value; every
answer carries its abstention (`None`, `complete: false`, `name: None`);
every answer is computed under the invocation's `SurfaceQuery`, which the
context fixes.
Preserve: every existing answer of the re-keyed functions; the old entry
points stay for one checkpoint and are removed by CC2.9–CC2.11 as their
callers move. Tests: `resolved_invocation.rs` unit rows — one per query on
a literal call, a computed-head call (`None`), and a call under a release
that lacks the command (`None`); `registry_sweep.rs`'s
`sweep_every_command_every_accessor` extended to call every query on every
command with a representative shape (no panic, no `unwrap`). Gates: none.
Docs: `command-registry.md` § *How registry feeds the compiler* gains a
paragraph naming the queries; the page's § *The derived-query layer*
"proposed" note flips. Model: opus. Size: M. After: CC2.2, CC2.6.

**CC2.9 — clause consumers in the compiler.**
Files: `rust/tcl-compiler/src/ir.rs` (`TryHandler::kind: HandlerMatch`
replaces the `String`; `IfClause` unchanged), `lowering/structured.rs`
(`lower_if`, `lower_try` build from `ResolvedInvocation::clause_plan`:
clauses with `timing == Selected` become `IfClause` / `TryHandler`,
`Always` becomes `finally_body`, `falls_through_to` the fall-through
marker; the `"elseif"` / `"else"` / `"then"` / `"finally"` / `"on"` /
`"trap"` / `"-"` literals are gone), `executable_ir.rs:3452`
(`handler.kind == HandlerMatch::ErrorCodePrefix`),
`cfg_builder/cfg_lower.rs:955` (`is_on_ok` becomes
`handler.kind == HandlerMatch::CompletionCode && tcl_registry::completion::CompletionCode::from_word(&handler.match_arg) == Some(CompletionCode::Ok)`
— the registry's own completion-code word parse at
`rust/tcl-registry/src/completion.rs:56`, made `pub` if it is not; the
page's `is_default` does not apply because `try` declares no
`default_clause` — see D2.8), `analyser/handlers.rs`
(`handle_try_command` walks the plan: `Protected` and `Selected` bodies
through `analyse_selected_body`, `Always` through `analyse_body`, the
var-list slot through `define_vars_from_list`, a `falls_through_to` clause
skipped; `handle_for_command` deleted — the generic body walk in
`dispatch_body_arguments` bumps `control_flow_body_depth` for a
`PerIteration` or `LoopFixture` body and `conditional_depth` for a
`Selected` one, reading the plan), `analyser/commands.rs`
(`orphaned_keyword_parent` becomes a lookup of the word in
`clause_keywords()`'s owner map — `clause_grammar::owner_of_keyword(word) -> Option<&'static str>`
added to CC2.2's module and used here; `:1708` reads
`clause_plan().and_then(|p| p.defect)` first, then the `clause_shape_check`
escape hatch), `signature_scan/walker.rs` (the two `if` walks and the two
`try` walks at lines 428–440, 487–498, 584–594, 622–626 read the plan
through a `SignatureScan::clause_plan_for(seg)` helper).
Preserve: every `rust/tcl-compiler/tests/{cfg,analyser}.rs` `if` / `try` /
loop case byte-identical; every `signature_scan.rs` test; E004 positions.
Changes: none observable. Tests: `lowering/structured.rs`'s existing
`try` tests updated to `HandlerMatch` (the `("on", "ok", true)` triples
become `(HandlerMatch::CompletionCode, "ok", true)`);
`rust/tcl-compiler/tests/cfg.rs` gains
`a_try_handler_walk_reads_timing_not_keywords` (a pack command declaring a
`try`-shaped grammar under a different keyword set lowers the same CFG —
the negative control: the same body without the grammar stays one opaque
call). Gates: `registry-axes` pins for `lowering/structured.rs`,
`executable_ir.rs`, `cfg_lower.rs`, `analyser/handlers.rs`,
`analyser/commands.rs`, `signature_scan/walker.rs` lowered to zero and
the files added to `CLEAN_FILES`; `value-transfers --check` (the
`lowering/structured.rs` and `analyser/commands.rs` value-transfer pins
are the other lane's — untouched, see B1). Docs: `docs/design/compiler/lowering-dispatch.md`
(the `if`/`try` lowering paragraph names the plan). Model: opus. Size: L.
After: CC2.8.

**CC2.10 — clause consumers in the editor and tool tiers.**
Files: `rust/tcl-lsp-core/src/refactor/if_to_switch.rs:179–199`,
`rust/tcl-lsp-core/src/refactor/datagroup.rs:445` (`parse_if_chain`),
`rust/tcl-mcp/src/datagroup.rs:422`, `rust/tcl-lsp-core/src/semantic_tokens.rs`
and `minify.rs` (the two tables' functions from CC2.2),
`rust/xtask/src/gen_tmlanguage_keywords.rs` (same), `rust/tcl-lsp-core/src/formatting/`
and `rust/tcl-compiler/src/analyser/recovery.rs` (the `then` distinction
read from the slot's `noise` through `clause_noise_keywords()` where a
literal `"then"` remains; the implementer greps `"then"` in both).
Preserve: every refactor, formatter, minifier and semantic-token test;
the generated TextMate lists byte-identical. Tests:
`rust/tcl-lsp-core/tests/lsp_providers.rs` or the refactor modules' own
tests gain `if_to_switch_reads_the_clause_plan` with a pack-declared
`if`-shaped command (negative: a command without a grammar is not
converted). Gates: `gen-tmlanguage-keywords --check`, `registry-axes`
pins lowered to zero for the six files. Model: sonnet. Size: M. After:
CC2.9.

**CC2.11 — member consumers.**
Files: `rust/tcl-compiler/src/analyser/oo.rs` (`apply_oo_subcommand_in`'s
eleven arms become one `match member_row.effect`: `Relation` folds through
`apply_slot_member` on the graph `RelationSlot` names; `Callable` keyed on
`CallableRole` and the resolved `receiver` — `Instance`+`Method` →
`methods`, `TypeObject`+`Method` → `class_methods`, `Constructor` →
`constructors`, `Destructor` → `destructor`; `StateDeclaration` →
`variables` through `apply_slot_member`; `Forward` → `apply_oo_forward`;
`Configuration` on a wrapper → `apply_oo_self` / `apply_oo_private`
selected by the wrapper's own shift (the side is the row's resolved
`receiver`); `property`'s flag-keyed accessors stay `extract_property_defs`
until they are `Callable` rows of their own — recorded on the ledger with
expiry "the 9.0 `property` accessor rows"); `dispatch_snit_member` and
`parse_itcl_definition_body` route through the same `match`, deleting the
snit and itcl prefix conventions (33 keyword literals);
`member_method_kind` in `lowering/mod.rs` is replaced by
`MethodKind::from_effect(role: CallableRole, receiver: MemberReceiver) -> Option<Self>`
in `ir.rs`, `from_str_lossy` deleted with its tests; `extract_one_member`
reads the row), `rust/tcl-lsp-core/src/{references,folding,oo_body,document_symbols,definition,hover,workspace_index,workspace_symbols}.rs`
(the `"constructor"` / `"destructor"` literals read `MethodDef::kind`).
Preserve: `rust/tcl-compiler/tests/mro_lattice_adversarial.rs`,
`analyser.rs`, `rust/tcl-lsp-core/tests/oo_mro_parity.rs`, every OO
provider test byte-identical. Changes: none observable for the shipped
grammars. Tests: `rust/tcl-lsp-server/tests/preview_tickets_e2e.rs` gains
`a_pack_declared_member_spelling_reaches_every_provider` — a workspace
pack declaring `definition_body { family Snit member mymethod -roles {0 Name 1 ParamList 2 Body} -effect {callable -receiver instance -role method} … }`
on a private definer; document symbols, go-to-definition and hover see
`mymethod` with no consumer edit; the file's comment that "a new definer
spelling needs an `apply_oo_subcommand` arm" is deleted; negative: the
same pack with `-effect configuration` yields no method. Gates:
`registry-axes` pins for `oo.rs`, `lowering/mod.rs` and the eight
provider files lowered; `value-transfers --check` untouched (the `oo.rs`
value-transfer pin is the other lane's, B1). Docs: `docs/design/contracts/tcloo-implementation.md`
(the arm table becomes the effect table); `AGENTS.md` § *The registry is
the source of truth* keeps its sentence. Model: opus. Size: L. After:
CC2.4, CC2.8.

**CC2.12 — scope, namespace and interpreter transitions; loop and bind
by role; the special-variable pack statement; the package layout.**
Files: `rust/tcl-compiler/src/analyser/handlers.rs` (one generic
`apply_state_transitions(&mut self, inv: &ResolvedInvocation, arg_tokens, scope_path)`
consuming `VariableCellAliasTransition` (the alias the handler recorded by
hand: `define_var(local, …)` plus the `link_target_span` unification the
`global` / `variable` / `upvar` / `namespace upvar` handlers perform today)
and `NamespaceTransition` (namespace variable declarations); the four
handlers `handle_global_command`, `handle_variable_command`,
`handle_upvar_command`, `handle_namespace_upvar_command` are deleted once
their unit tests pass through the generic consumer — `handle_global_defines_each_name`,
`handle_variable_defines_only_names_skipping_values`,
`handle_variable_single_name_no_value` and the `upvar` tests are retargeted;
`handle_dict_for_command`, `handle_dict_update_command`,
`handle_incr_command`, `handle_append_lappend_command` deleted — the
generic `handle_var_binding_command` binds every `LoopVarList` and
`VarWrite` position the roles name, with `warn_if_unused` false for a
command whose `semantics` resolves to a cell read-modify-write (the
value-transfers lane's derived descriptor on `append` / `lappend` /
`incr`; `incr` today passes `true`, a delta recorded below);
`handle_foreach_command` keeps only the literal-iteration simulation and
binds through roles; `handle_package_require` reads `-exact` through
`option_effects` (`package_.rs` gains `EndsOptions`-free rows as today
plus a `Selects(…)`-free read: the presence check is
`OptionFacts::options`), the index arithmetic replaced by the roles the
spec declares), `rust/tcl-registry/src/commands/tcl/package_.rs`
(`arg_roles` for `require` / `provide` / `ifneeded` where missing),
`rust/tcl-registry/src/special_vars.rs` and `rust/tcl-spectcl/src/loader.rs`
(a pack-level `special_var NAME -kind K -access A -origin O ?-dialects {…}? ?-startup B?`
statement building a `SpecialVarSpec`; `special_vars_for_dialect` reads
pack-declared rows from the overlaid registry — a
`CommandRegistry::special_vars()` door the loader fills; the `lappend auto_path`
arm and `set auto_path` arm become one read of `special_var("auto_path")`'s
`VarAccess`), `docs/design/spec-dsl-examples/README.md` (the row).
Preserve: every `handlers.rs` and `analyser.rs` test except the delta
below. Changes: `incr x` with `x` never read afterwards no longer draws
W211 (the target is read before it is written; `append` already reads so);
a `namespace upvar` with a dynamic local still skips it (the transition
abstains — the witness the page requires before the isolated per-item
pass consumes transitions: `alias_and_binding_resolvers_abstain_on_a_dynamic_word`
in `rust/tcl-registry/src/state_transition.rs`). Tests: the retargeted
handler tests; `rust/tcl-compiler/tests/analyser.rs` gains
`a_pack_declared_scope_alias_binds_its_local` (a pack command with
`frame_effect -level-word … -layout AliasPairs` and a `state_transitions`
resolver emitting alias facts binds like `upvar`; negative: the same
command with a dynamic level word binds nothing) and
`a_pack_declared_special_variable_is_readable_at_startup`. Gates:
`registry-axes` pins for `handlers.rs` lowered; `analyser_hooks`
re-baselined by CC2.13. Docs: `docs/design/registry/special-variable-registry.md`
gains the pack statement; `spec-packs.md` § *What a pack still cannot say*
loses "the `state_transitions` resolver … today" sentence once CC2.14
lands the resolver family. Model: opus. Size: L. After: CC2.8, CC2.14.

**CC2.13 — hook retirement and the re-baseline.**
Files: `rust/tcl-registry/src/hooks.rs`, the specs that carried the
variants, `rust/tcl-compiler/src/analyser/commands.rs` (`dispatch_analyser_hook`),
`rust/tcl-registry/tests/analyser_hooks.rs`.
Retire (the handler's only command-specific knowledge was a position or a
keyword a descriptor now states): `Try` (CC2.9), `For` (CC2.9), `DictFor`,
`DictUpdate`, `Incr`, `Append`, `Lappend`, `Upvar`, `NamespaceUpvar`,
`Global`, `Variable` (CC2.12) — eleven variants, and the twelve stamps that
carried them (`::tcl::dict::for` and `::tcl::dict::update` included).
Keep, documented at the call site as the page's residue: `Proc`, `OptProc`,
`Apply` (procedure definition and the `all_procs` table), `Uplevel` and
`NamespaceEval` (dynamic-target synthetic domains), `NamespaceEnsemble`,
`NamespaceImport`, `NamespaceExport`, `NamespaceForget`, `NamespacePath`,
`NamespaceUnknown` (export tombstone ordering and the namespace domain),
`Foreach` (literal-iteration simulation), `Switch` and `Catch` (case-list
walk and completion protocol), `InterpAlias`, `InterpEval`, `InterpCreate`,
`InterpDelete`, `InterpHide`, `InterpExpose` (the interpreter-domain
stack and value binding of a created interpreter), `Rename` (rename
epochs), `OoDefine`, `OoObjdefine` (member routing to `ClassDef` fields),
`PackageRequire`, `PackageProvide`, `PackageIfneeded`, `PackagePrefer`
(package-index bookkeeping), `Source`, `Load`. On the ledger (command-
specific, retired by a named slice): `Set` and `DictWith` (the value axis,
slices 5 and 8), `RegexPatternCapture` (slice 5). The pinned set in
`analyser_hook_stamps_match_the_former_guard_list` shrinks from 43
variants to 32 and the comment on each kept row names its residue class;
`analyser_hook_stamps_are_disjoint_from_definer_families` unchanged.
Tests: the re-baselined pinned set; `hooks.rs`'s own count assertion if
one exists. Gates: `registry-axes` (the ledger rows for `Set`, `DictWith`,
`RegexPatternCapture` carry `until slice 5|8`). Docs: `docs/design/compiler/command-registry.md`
§ *How registry feeds the compiler* (the residue list). Model: sonnet
(the table above is the design). Size: S. After: CC2.9, CC2.12.

**CC2.14 — the `state_transitions` resolver family in the loader.**
Files: `rust/tcl-registry/src/pack_hooks.rs` (`HookFamily::StateTransitionResolver`,
verbs `alias IDX-LOCAL IDX-TARGET ?-level W?` and `namespace-variable IDX`,
silence "no transitions", `field` `state_transitions.resolver`;
`HOOK_FAMILIES` becomes `[HookFamily; 12]`; the dispatch thunk returns a
`StateTransitions` holding only `VariableCellAliasTransition` and
`NamespaceTransition` facts — any other verb is not injected, so the
family cannot emit the four forbidden families; a dynamic target word
abstains and the abstention widens through `StateTransitionWidening`),
`rust/tcl-spec-hooks/src/emit.rs` (the two verbs), `rust/tcl-spectcl/src/loader.rs`
(`state_transitions_value` reads `resolver {words ctx} {…} | -native ID | none | from-frame-effect`,
`argument_shape`, `widen`, `covers`, `commit` — the drop list loses the
five rows; the comment "reference-only by design" deleted),
`rust/tcl-spec-studio/src/render_spectcl.rs` (`state_transitions`'s `GAPS`
row shrinks to the resolver's `-native` form: `DraftOpaque` stays for a
descriptor naming a native resolver, and a pack-authored body renders),
`docs/design/spec-dsl-examples/README.md` (the rows).
Tests: `rust/tcl-spectcl/src/loader.rs`'s `native_hook_tables_cover_their_catalogues`
covers the new family; `rust/tcl-spec-hooks/tests/families_e2e.rs` gains
`a_state_transition_body_emits_alias_facts_only` (negative: a body calling
`command-binding …` fails to compile because the verb is not defined in
its sandbox); `rust/tcl-spectcl/tests/spec_corpus.rs`'s baseline loses the
"`state_transitions` row … is not yet loadable" notices (regenerate
`spec_corpus_baseline.txt`). Gates: `callback-inventory --check` (a new
executable position tier: the resolver body), `pack-goldens --check`.
Docs: `spec-packs.md` § *What a pack still cannot say* (the "today"
sentence goes). Model: opus. Size: M. After: CC2.1.

**CC2.15 — the lint's roots shared, the ledger's expiries closed, and the
step's documents.**
Files: `rust/xtask/src/util.rs` (`pub const ANALYSIS_TIER_ROOTS` used by
both gates — lands only after the value-transfers lane's slice 2 commit,
B1), `rust/xtask/src/registry_axes.rs` (the landed-set constant gains
step 2; every `until step 2` waiver must be gone), KCS notes:
`docs/kcs/compiler/kcs-qa-where-does-a-clause-shape-come-from.md`
(Contributor; the grammar, the plan, the escape hatch),
`docs/kcs/compiler/kcs-qa-what-does-a-member-effect-say.md`,
`docs/kcs/kcs-howto-declare-an-option-effect-in-a-tclspec-pack.md`
(User/Contributor; the `subst` example), each indexed in
`docs/kcs/README.md` and `docs/kcs/compiler/README.md`; `docs/GLOSSARY.md`
entries for *clause grammar*, *member effect*, *option effect*;
`docs/design/README.md` and `docs/design/compiler/README.md` index lines
for `registry-consumer-contracts.md` drop "proposal" for the description
contract. Gates: `kcs-index-links`, `registry-axes --check`. Model:
sonnet. Size: S. After: every other CC2 item.

#### Ordering and checkpoints

1. CC2.1 → checkpoint `wip(consumer-contracts): the registry-axes gate and its baseline`.
   Green: `cargo check -p xtask`, `cargo test -p xtask registry_axes`,
   `cargo xtask registry-axes --check`, `owner-resolution`.
2. CC2.2 + CC2.3 → one checkpoint `… clause grammars as data`. Green:
   `cargo check --workspace`; `cargo test -p tcl-registry` (lib,
   `registry_sweep`, `analyser_hooks`); `cargo test -p tcl-spectcl --test eval_loader --test spec_corpus`;
   `cargo test -p tcl-spec-studio` (lib, `spectcl_roundtrip`, `spectcl_ports`,
   `schema_coverage`, `reference_doc`); `cargo test -p tcl-compiler --test cfg --test analyser`
   (unchanged behaviour); `gen-tmlanguage-keywords --check`; `pack-goldens --check`.
3. CC2.4 + CC2.5 → `… member effects and the studio round trip`. Green as
   in 2 plus `cargo test -p tcl-compiler --test mro_lattice_adversarial`.
4. CC2.6 + CC2.7 → `… option effects`. Green as in 2 plus
   `cargo test -p tcl-registry --test tcl91_dialect`, `audit-option-dialects --check`,
   `callback-inventory --check`.
5. CC2.8 → `… the derived-query layer`. Green: `cargo test -p tcl-registry`.
6. CC2.14 → `… the state-transition resolver family`. Green:
   `cargo test -p tcl-spec-hooks`, `cargo test -p tcl-spectcl`,
   `callback-inventory --check`.
7. CC2.9 → `… lowering and the analyser read the clause plan`. Green:
   `cargo test -p tcl-compiler` in full; `registry-axes --check`.
8. CC2.10 → `… the editor tiers read the clause plan`. Green:
   `cargo test -p tcl-lsp-core`, `cargo test -p tcl-mcp`,
   `gen-tmlanguage-keywords --check`.
9. CC2.11 → `… members by effect`. Green: `cargo test -p tcl-compiler`,
   `cargo test -p tcl-lsp-core`, `cargo test -p tcl-lsp-server --test preview_tickets_e2e`.
10. CC2.12 → `… transitions, roles and special variables`. Green:
    `cargo test -p tcl-compiler`, `cargo test -p tcl-registry`.
11. CC2.13 → `… retire eleven analyser hooks`. Green:
    `cargo test -p tcl-registry --test analyser_hooks`, `cargo test -p tcl-compiler --test analyser`.
12. CC2.15 → `… step 2 documents and the shared lint roots`. Green:
    `make rust-check`.

#### Review checklist

- The registry rule: no new `match cmd_name { "…" => … }` anywhere outside
  `rust/tcl-registry`; `registry-axes --check` and `value-transfers --check`
  both green with no pin raised.
- No new `#[allow]`; UK spelling in every new identifier and comment
  (`serialise`, `initialise`, `behaviour`, `catalogue`); the AGPL header on
  every new `.rs` (`clause_grammar.rs`, `option_effect.rs`,
  `registry_axes.rs`) and on no generated or fixture file.
- The page's identifiers verbatim: `ClauseGrammarSpec`, `ClauseRow`,
  `ClauseRowShape`, `ClauseSlot`, `ClauseTiming`, `LoopPhase`,
  `ClauseSelection`, `DefaultClause`, `ClausePlan`, `ResolvedClause`,
  `ClauseRowId`, `MemberEffect`, `MemberReceiver`, `CallableRole`,
  `StateScope`, `RelationSlot`, `InitTiming`, `MemberRow`, `MemberArity`,
  `OptionEffect`, `OptionEffectKind`, `EffectAxis`, `SubstitutionKind`,
  `OptionEffectFamily`, `FamilyBase`, `FamilyCombine`, `OptionEffects`; the
  two adaptations D2.1 and D2.2 and nothing else.
- Every descriptor moved all four surfaces in one checkpoint (registry,
  loader, renderer, studio form) or the `GAPS` table names it; the
  `spectcl_roundtrip` gap register lists exactly the rows the page leaves
  (`frame_effect`, `world_effects`, `state_transitions` (resolver only),
  `event_*`, `case_list`, `body_scope`, `bpf_op`, `data_collection`,
  `side_switch_target`, `irules_top_level_effect`, `result_stability`,
  `remote_method`, `variable_write_min_args`, `body_interpreter`,
  `completion`, `dispatch_dependencies`, `native_lowering`, `semantics`).
- The parity gates named per item are byte-identical, not "mostly":
  `cfg.rs`, `analyser.rs`, `mro_lattice_adversarial.rs`, `signature_scan.rs`,
  `lsp_providers.rs`, `oo_mro_parity.rs`, the TextMate lists.
- Risks: the walk's keyword rule (a keyword never matches inside a slot —
  `if else {a}` is the witness); `foreach`'s `Group` row and
  `exclude_trailing` off by one on a two-word call; `for`'s keywordless
  rows filled in the wrong order; the `regexp -inline` role change reaching
  W210/W211 through the SSA def roles (the `stub_arg_roles.rs` and
  `analyser.rs` regexp cases are the gate); `incr`'s W211 delta accepted
  and recorded; a retired hook whose handler had a second effect (the
  retargeted unit tests are the proof, never the dispatch table).

#### Behavioural deltas expected

- `subst -nocommands -variables …` draws W147 (mutually exclusive options)
  beside W102; no other output changes.
- `regexp -inline … a b` draws the new option relation's finding instead of
  binding `a` and `b`; `regexp -about exp` no longer reports a missing
  operand.
- `incr x` with `x` unread afterwards draws no W211.
- A workspace pack may declare a `state_transitions` resolver body,
  `clause_grammar` timings, `member -effect`, `option -effect`,
  `option_effect_family`, `special_var`, and `semantic_operation`, and the
  Spec Studio renders all of them; `spectcl_check` reports the resolver as
  a hook.
- Nothing else: every parity gate above is byte-identical.

### Step 3 — trust gates execution; stub flags reach their fields

**Goal.** `WorkspaceTrust` plumbed from the LSP client to discovery; the
two `untrusted` predicates collapsed into one exported from `tcl-registry`;
hook-body execution gated on trust with the dormant-hook abstention
reported on the pack file; `spectcl_check`'s tier and trust parameters;
the six `StubFlags` consumed on their catalogue fields with nearest-wins
role resolution.

**Exit evidence.** `cargo test -p tcl-spectcl` (in full) green including
the new `workspace_trust.rs` harness; `cargo test -p tcl-lsp-server`'s
spec-pack e2e subset green with the dormant notice published;
`cargo test -p tcl-lsp-core --test stub_arg_roles` green with one
positive and one negative witness per flag; `spectcl_check` reports
`dormant_hooks`; `cargo xtask callback-inventory --check` and
`kcs-index-links` green.

#### Work items

**CC3.1 — `WorkspaceTrust` through discovery, provenance and the cache key.**
Files: `rust/tcl-dialect/src/model/environment.rs` (`pub enum WorkspaceTrust { Trusted, Untrusted }`,
`Default` `Trusted`, beside `Provenance` — D3.1), `rust/tcl-spectcl/src/discovery.rs`
(`DiscoveryOptions::workspace_trust: WorkspaceTrust`; `PackFile::trust: WorkspaceTrust`,
set from the options for `Tier::Workspace` and `Trusted` for every other
tier), `rust/tcl-spectcl/src/pack.rs` (`MergedPack::trust`;
`MergedPack::provenance(&self) -> Provenance`), `loader/environment_block.rs`
(`PackEnvironmentTier::provenance(self, trust: WorkspaceTrust)`: `Workspace`
maps to `WorkspaceTrusted` or `WorkspaceUntrusted`; every caller passes
the pack's trust), `loader/eval.rs` (`EvalOptions::trust`,
`EvalSnapshotKey::trust`, the `tier` doc comment corrected to name the
trust state), `cache.rs` (`key_for(source, tier, trust)`; `entry_key`
hashes the trust), `install.rs` and `registration.rs` (the provenance
passed through), `rust/tcl-mcp/src/spectcl.rs` and `rust/tcl-cli`
callers (pass `Trusted` explicitly).
Preserve: with `Trusted` (the default) every answer is byte-identical;
every existing cache key for a trusted workspace pack is unchanged only if
the hash mixes `Trusted` as zero bytes — it does not: the entry key
changes for every pack and the on-disk cache rebuilds once (a
non-observable delta; `cache::VERSION` bumps). Tests:
`rust/tcl-spectcl/tests/workspace_trust.rs` (new harness, added to
`scripts/dev/rust-test-binary-shards.tsv` as
`5	tcl-spectcl::workspace_trust	test	tcl-spectcl	workspace_trust`):
`an_untrusted_workspace_pack_is_workspace_untrusted_provenance`,
`an_untrusted_workspace_pack_still_declares_its_facts` (arity, roles and
the `semantics` declaration reach the registry — the authority ruling),
`an_untrusted_workspace_pack_cannot_override_a_compiled_name` (E-R2, the
negative), `a_client_that_reports_nothing_is_trusted`,
`the_snapshot_key_distinguishes_trust`. Gates: none. Docs:
`spec-packs.md` § *Workspace trust* "today" sentence goes;
`docs/design/registry/dialect-and-package-registry-redesign.md` § 11.1
O9 closed. Model: opus. Size: M. After: —.

**CC3.2 — one `untrusted` predicate.**
Files: `rust/tcl-registry/src/model/registration.rs` (`pub fn untrusted(provenance: Provenance) -> bool`,
re-exported as `tcl_registry::model::untrusted`), `rust/tcl-spectcl/src/loader/eval.rs`
(`untrusted(tier)` deleted; `provenance_violation_in` takes the
`Provenance` and calls the registry's predicate; `provenance_violation(pack, tier)`
becomes `provenance_violation(pack, provenance)`), `rust/tcl-mcp/src/spectcl.rs`
(the caller). Tests: `registration.rs` unit row
`the_one_untrusted_predicate_names_the_three_classes`;
`workspace_trust.rs` asserts the loader and the registration layer agree
for every `Provenance` (the negative: `WorkspaceTrusted` and `User` are
not untrusted). Model: sonnet. Size: S. After: CC3.1.

**CC3.3 — hook bodies gated; the dormant notice.**
Files: `rust/tcl-spectcl/src/loader.rs` (`HookDecl::line: u32`),
`rust/tcl-spectcl/src/pack.rs` (`load_sources` pushes, for every hook of
a pack whose `provenance()` is `WorkspaceUntrusted`, a `PackNotice` at the
hook's line, context `command NAME`, message
"`FIELD` is dormant: the workspace is not trusted, so this hook body does
not run and the command keeps its declarative facts", `Severity::Information`),
`rust/tcl-spectcl/src/hooks.rs` (`plan_for` allocates no slot for a
dormant pack's programs, so `specialise` leaves the loader's abstaining
placeholder — the abstention "exactly as a declared-but-unbound hook
does"; `HookPlan::dormant: Vec<(pack, command, field)>` for `spectcl_check`),
`rust/tcl-lsp-server/src/lib.rs` (`apply_initialization_options` reads
`workspaceTrust: "trusted" | "untrusted"`; `did_change_configuration`
reads `tclLsp.workspaceTrust`; a change triggers `reload_spec_packs(ReloadTrigger::Trust)`;
`spec_pack_discovery` sets `workspace_trust`), `editors/vscode/src/`
(the client sends `workspace.isTrusted` in `initializationOptions` and
re-sends the setting on `onDidGrantWorkspaceTrust` — D3.2; the other
editors send nothing and are trusted).
Preserve: a trusted workspace installs every hook as today
(`hooks.rs`'s `a_declared_body_replaces_the_abstaining_placeholder`).
Tests: `workspace_trust.rs` — `an_untrusted_pack_installs_no_hook_body_and_reports_each_as_dormant`
(a pack with a `const_fold` body: `plan_for` has zero programs for it, one
notice per hook at its line; the negative: the same pack trusted installs
the body and folds), `granting_trust_reloads_and_installs_the_bodies`
(server unit test beside the `reload_spec_packs` tests);
`rust/tcl-spec-hooks/tests/containment_e2e.rs` unchanged (containment is
not trust). Gates: `callback-inventory --check` unchanged (no new tier).
Docs: `spec-packs.md` § *Workspace trust*; KCS
`docs/kcs/kcs-qa-why-is-my-pack-hook-dormant.md` (User; all-editors,
VS Code sub-heading for the trust prompt), indexed. Model: opus. Size: M.
After: CC3.1, CC3.2.

**CC3.4 — `spectcl_check`'s tier and trust.**
Files: `rust/tcl-mcp/src/spectcl.rs` (`tier` argument
`bundled|user|workspace|studio-override`, default `workspace`; `trust`
argument `trusted|untrusted`, default `trusted`; the provenance verdict
computed for that pair; a `dormant_hooks` array), `rust/tcl-mcp/src/tools.rs`
(the tool schema's two properties). Tests: `rust/tcl-mcp/src/spectcl.rs`
unit rows — the default equals today's output; `trust: untrusted` lists
every hook as dormant and refuses an `-override`. Gates: `gen-ai-diagnostics --check`
if the tool catalogue is generated from `tools.rs` (the implementer runs
`make codegen` and stages what moves). Docs: the redesign's § 11.1 O4
closed; `docs/kcs/features/` MCP note updated if it lists the tool's
arguments. Model: sonnet. Size: S. After: CC3.3, and after the
diagnostic-policy lane's MCP commit (B2).

**CC3.5 — the six `StubFlags` on their catalogue fields; nearest-wins.**
Files: `rust/tcl-registry/src/model/declaration.rs` (`DeclaredCommand::traits: Traits`,
`DeclaredCommand::side_effects: Vec<SideEffect>`; `DeclaredCommand::new`
keeps its signature and a builder `with_traits` / `with_side_effects`;
`DocumentCommandSurface::traits(&self, name) -> Option<Traits>` and
`side_effects(&self, name) -> Option<&[SideEffect]>` answering the
document's declaration where it speaks, else the catalogue's;
`arg_indices_for_role` and `command_prefixes` answer nearest-wins: the
declaration alone when it declares the name, the catalogue otherwise),
`rust/tcl-compiler/src/analyser/types.rs` (`to_declared_command` maps
`BARRIER` → `Traits::CREATES_DYNAMIC_BARRIER`, `LOOP` → `HAS_LOOP_BODY`,
`PURE` → `PURE`, `UNSAFE` → `UNSAFE | SAFE_INTERP_HIDDEN`, `SCOPE_ALIAS` →
`CREATES_SCOPE_ALIAS`, `MUTATOR` → `SideEffect { target: Variable, writes: true, .. }`;
the doc comment's "has never had a consumer" goes),
`rust/tcl-compiler/src/ssa.rs:625` and `:1693`, `unit_scope.rs:521`,
`memory_ssa.rs:517` (read `surface.traits(name)` — the surface is already
built in `unit_scope.rs`; `ssa.rs` and `memory_ssa.rs` receive it through
`UnitBuildOptions::declared_commands`, which reaches lowering already),
`rust/tcl-compiler/src/analyser/utils.rs` (`parse_stub_flags` unchanged),
`docs/design/contracts/dialect-stubs.md` § *Flags* and § *Stubs are
declarations* ("today" sentences go).
Preserve: a document with no stub is byte-identical; a stub that names a
command the catalogue lacks is unchanged. Changes: a stub re-declaring a
catalogued command narrows its roles to the declaration (nearest-wins
replaces the union); each flag has an effect. Tests:
`rust/tcl-lsp-core/tests/stub_arg_roles.rs` gains one pair per flag —
`a_pure_stub_in_statement_position_is_reported_unused` (O108 / the pure
statement finding; negative: without `-pure` it stays),
`a_mutator_stub_keeps_the_store_it_reads` (O109),
`a_barrier_stub_blinds_the_reads_after_it` (W210 silence; negative: no
flag, W210 fires), `a_loop_stub_bumps_the_loop_body_depth` (W240 family),
`a_scope_alias_stub_aliases_its_local` (the alias walk sees the local as
the target's cell), `an_unsafe_stub_is_hidden_in_a_safe_interpreter`
(the safe-interp diagnostic the catalogue's `UNSAFE` commands draw), and
`a_stub_that_redeclares_a_catalogued_command_answers_nearest_wins`
(negative: the catalogue role the stub omits is not assigned). Gates:
`registry-axes` (no change). Docs: `dialect-stubs.md`; KCS
`kcs-howto-annotate-commands-with-stubs.md` gains the flags' effects.
Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC3.5 first (independent; `wip(consumer-contracts): stub flags reach their fields`;
green: `cargo test -p tcl-registry`, `cargo test -p tcl-compiler --test analyser`,
`cargo test -p tcl-lsp-core --test stub_arg_roles`), then CC3.1 + CC3.2
(`… workspace trust reaches discovery`; green: `cargo test -p tcl-spectcl`,
`cargo test -p tcl-registry`, the shard manifest check
`bash scripts/dev/test-nextest-binary-shards.sh`), then CC3.3
(`… dormant hook bodies`; green: `cargo test -p tcl-spectcl`,
`cargo test -p tcl-spec-hooks`, `cargo test -p tcl-lsp-server` spec-pack
subset, `npm test` in `editors/vscode` for the client change), then CC3.4
(`… spectcl_check tier and trust`; green: `cargo test -p tcl-mcp`).

#### Review checklist

- Authority is never gated: under `Untrusted` every declarative fact of the
  pack reaches the registry and the analyser (the
  `an_untrusted_workspace_pack_still_declares_its_facts` witness).
- Exactly one `untrusted` predicate exists in the tree
  (`grep -rn "fn untrusted" rust/` returns one hit).
- A dormant hook is reported once per hook at its own line, never once per
  call; `plan_for` allocates it no slot.
- The trust wire is one input; the server treats an absent value as
  trusted; the cache key includes it.
- Nearest-wins changed a union to a replacement: the negative witness
  exists and the delta is recorded.
- Risks: the cache-key change invalidating every pack snapshot once (accepted);
  `SAFE_INTERP_HIDDEN` reaching a diagnostic the stub author did not expect;
  the VS Code client's trust event racing `initialized`.

#### Behavioural deltas expected

- In an untrusted VS Code workspace, pack hook bodies do not run and each
  is reported as an Information notice on the `.tclspec`; declarative facts
  are unchanged; an `-override` of a compiled command is refused with the
  provenance named.
- A stub's `-pure`, `-mutator`, `-barrier`, `-loop`, `-scope_alias` and
  `-unsafe` now change analysis as the table in `dialect-stubs.md` says.
- A stub re-declaring a catalogued command narrows its roles to the
  declaration.

### Step 4 — identity: `alias_of`, the alias target's identity, the stamp rule, `SiteClaim`

**Goal.** `alias_of` as the one admissible source of a pack command's
builtin identity; codegen recording the alias target's identity; the
loader's stamp rejection rule (target's own, tier gate, drop the stamp
only); `SiteClaim` and `PackFactStamp` recorded in the artefact and checked
at admission for rungs 1 and 2.

**Exit evidence.** The two witnesses the page names, in
`rust/tcl-spectcl/tests/codegen_stamps.rs`: a refused stamp under the tier
gate, and an accepted one whose recorded identity the VM's alias hop
resolves; `cargo test -p tcl-vm --test command_mutation_deopt_e2e` green
with the rung-1 stamp check; `spectcl_roundtrip` green with `alias_of`
round-tripping.

#### Work items

**CC4.1 — `alias_of`.**
Files: `rust/tcl-registry/src/spec.rs` (`CommandSpec::alias_of: Option<&'static str>`,
doc: "the shipped builtin this pack command is; the only admissible
source of a builtin identity for a pack command; never inferred from a
realm alias"), `rust/tcl-spectcl/src/loader.rs` (`alias_of NAME`),
`render_spectcl.rs` / `schema.rs` / `help.rs` / `draft.rs` / `coverage.rs`
(the four surfaces; a plain string field, no `GAPS` row),
`docs/design/spec-dsl-examples/README.md`. Tests: `registry_sweep.rs` —
`alias_of_names_a_shipped_command_of_the_same_family` (negative: a spec
naming an unknown target fails the sweep); `spectcl_roundtrip.rs` sees the
field round-trip. Model: sonnet. Size: S. After: —.

**CC4.2 — the stamp rejection rule.**
Files: `rust/tcl-spectcl/src/stamps.rs` (new: `pub fn admit_codegen_stamps(command: &mut PackCommand, provenance: Provenance, shipped: &CommandRegistry) -> Vec<StampRefusal>`;
rule 1: a `codegen_hook`, `inline_codegen_hook` or
`semantic_operation Intrinsic(…)` on a pack command is admitted only when
`alias_of` names a shipped spec carrying that same hook identity; rule 2:
only `BuiltIn` and `BundledPack` may carry one at all; rule 3: a refusal
clones the leaked spec, clears the stamp, re-leaks it, and returns the
refusal with the provenance's label and the target the stamp would have
had to name), `rust/tcl-spectcl/src/pack.rs` (`load_sources` calls it per
command after the merge and pushes one `PackNotice` per refusal at the
command's line, `Severity::Warning`: "`codegen_hook Lassign` refused for
`vendor::unpack`: a trusted workspace pack may not name a codegen
catalogue member; the stamp would have to sit on `alias_of lassign`"),
`rust/tcl-spectcl/src/loader.rs` (the existing "names a codegen hook"
notice is replaced by the rule's notice), `rust/tcl-spectcl/src/install.rs`
(`debug_assert!` that no stamp survives on a non-admitted provenance).
Preserve: a bundled pack's stamps (none of the shipped `specs/*.tclspec`
carries one; `jim.tclspec` is a core surface) load as today. Tests:
`rust/tcl-spectcl/tests/workspace_packs.rs` gains
`a_workspace_stamp_without_alias_of_is_refused_and_names_the_target`,
`a_workspace_stamp_with_alias_of_is_refused_by_the_tier_gate`,
`a_refused_stamp_costs_no_analysis_fact` (arity and roles survive), and
`a_bundled_stamp_on_an_alias_of_target_is_admitted` (loaded through
`bundled::load_from` on a temp `specs/` dir; negative: the same pack with
`alias_of lsort` is refused by rule 1). Gates: `pack-goldens --check`,
`spec_corpus` baseline. Docs: `spec-packs.md` § *What a pack still cannot
say* and `registry-consumer-contracts.md` § *The loader's stamp rejection
rule* ("today" sentences go). Model: opus. Size: M. After: CC4.1.

**CC4.3 — codegen records the alias target's identity.**
Files: `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`
(`registry_codegen_hook` builds the identity from
`resolved.spec.alias_of.unwrap_or(resolved.spec.name)`; the inline path
in `codegen/cmd_subst.rs` likewise), `rust/tcl-vm/src/interp.rs`
(`command_binding_matches` unchanged — the alias hop already resolves).
Tests: `rust/tcl-spectcl/tests/codegen_stamps.rs` (new harness; shard row
`5	tcl-spectcl::codegen_stamps	test	tcl-spectcl	codegen_stamps`):
`an_admitted_alias_stamp_records_the_targets_identity` (compile
`vendor::unpack {a b} $l` against a registry with the bundled pack; the
module's `command_bindings` names `lassign`'s identity), then
`the_vm_admits_it_through_the_alias_hop` (`interp alias {} vendor::unpack {} lassign`
in a `tcl_vm::Vm` running the module: admitted, result correct) and
`a_proc_at_the_pack_name_recompiles_plain` (negative). Gates: none.
Model: opus. Size: M. After: CC4.2.

**CC4.4 — `SiteClaim` and `PackFactStamp`.**
Files: `rust/tcl-runtime-api/src/site_claim.rs` (new: `SiteClaim`,
`PackFactStamp` with `content_hash: u64` (D4.3), `IdentityKind`;
`RuntimeBacking` and `BodySource` are step 7's — the `ReferenceBody` and
`ShippedImplementation` variants are added by CC7.1 and CC8.2, so this
item lands `Generic`, `PackFacts`, `BuiltinAlias`), `rust/tcl-bytecode/src/lib.rs`
(`FunctionAsm::site_claims: Vec<SiteClaim>`), `rust/tcl-registry/src/registry.rs`
(`CommandRegistry::pack_origin(&self, spec: &CommandSpec) -> Option<&PackOrigin>`
— a side table keyed by spec pointer; `PackOrigin { pack: String, content_hash: u64, vocabulary_version: String }`
in `rust/tcl-registry/src/pack_origin.rs`, filled by
`tcl-spectcl::install::install_into` — D4.4), `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`
(a site whose hook came through `alias_of` records `BuiltinAlias { binding, facts }`;
a constant fold emitted from a pack `const_fold` hook records
`PackFacts(stamp)`; `overlay_generation` and `evaluator_revision` from the
`AnalysisContext` the codegen context carries), `rust/tcl-vm/src/interp.rs`
(`Vm::set_pack_facts(&mut self, stamps: Vec<PackFactStamp>)`; admission
in `function_command_bindings_match` gains the rung-1 check: every
`PackFacts` / `BuiltinAlias.facts` stamp equals a stamp the VM holds on
`pack`, `content_hash`, `vocabulary_version`, `overlay_generation`,
`evaluator_revision`; a mismatch is plain dispatch or the admission error
`run_module` already produces), `rust/tcl-lsp-server` (the optimise path
sets the facts from the published pack set).
Tests: `codegen_stamps.rs` gains `a_changed_pack_invalidates_the_site`
(same module, VM holding a stamp with a different `content_hash`:
recompiled plain — negative control: the same stamp admits);
`rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` gains
`a_rung_zero_module_is_admitted_under_a_changed_pack_set`. Docs:
`docs/design/contracts/vm-compiled-artifact-provenance.md` gains the
claim; the page's § *What the artefact records per rung* "proposed" note
flips for rungs 1 and 2. Model: opus. Size: L. After: CC4.3.

#### Ordering and checkpoints

CC4.1 (`wip(consumer-contracts): alias_of`), CC4.2 (`… the stamp rejection rule`),
CC4.3 (`… codegen records the alias target`), CC4.4 (`… site claims`).
Green at each: `cargo check --workspace`; `cargo test -p tcl-spectcl`;
from CC4.3 `cargo test -p tcl-compiler --test codegen --test codegen_integration`
and `cargo test -p tcl-vm --test command_mutation_deopt_e2e`; the shard
manifest check after CC4.3.

#### Review checklist

- `alias_of` is the only source of a pack command's builtin identity: no
  code path reads `realm.rs`'s aliases to admit a site (grep
  `alias_of` and `CommandBindingIdentity` construction sites).
- A refusal drops the stamp and nothing else (the
  `a_refused_stamp_costs_no_analysis_fact` witness).
- The identity recorded is the target's, never the pack command's own name
  (the module's `command_bindings` assertion).
- `content_hash` is the snapshot key's `u64` (D4.3), named as such in
  `PackFactStamp`'s doc comment.
- Risks: the spec clone-and-releak on refusal leaking per reload (bounded
  by the pack-set cache; note it); the VM's rung-1 check refusing a module
  compiled with no pack at all (a module with no claims must admit).

#### Behavioural deltas expected

- A pack stamp at any non-bundled tier is refused with a Warning notice
  naming the provenance and the target; today it loads with a note.
- A specialised site from a bundled `alias_of` command is admitted through
  the alias hop and runs specialised; today it recompiles plain.
- A changed pack turns its sites plain on the next admission.

### Step 5 — persisted guard identities, per-member semantics keys, the Explorer record

**Goal.** Intrinsic guard identities persisted in both runtimes across
command-environment mutations and the profile pin, keyed by command-token
generation and following rename and hide; `guard_semantics_key` one key
per `IntrinsicId` member; the AOT observability record complete for every
rejected native premise.

**Exit evidence.** `command_mutation_invalidates_guard_and_identity_attestation`
re-stated as `an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it`
(runtime) and its VM twin green; `guarded_boxed_intrinsic_runs_and_falls_back_against_the_real_runtime`
gains the "unrelated proc keeps the fast path" row; one
`tcl-fuzz` campaign over the `tclvm`/`runtime-rust` pair with no new
finding (manual, recorded in this document).

#### Work items

**CC5.1 — one semantics key per member.**
Files: `rust/tcl-registry/src/intrinsic.rs` (`guard_semantics_key` reads a
per-member table `const SEMANTICS_REVISION: [(IntrinsicId, u32); 28]`
combined with the release variant for the versioned members; the two
`RUNTIME_INVARIANT_SEMANTICS` / `VERSIONED_STRING_SEMANTICS` constants
become per-member; `guard_semantics_variants` likewise). Tests:
`intrinsic.rs` — `every_member_has_a_distinct_semantics_key`,
`bumping_one_members_revision_moves_no_other_key`; the runtime and VM
tests that compute the key by calling the function are unchanged; the one
literal `0x0306`-style assertion is updated. Model: sonnet. Size: S.
After: —.

**CC5.2 — guard identities keyed by token generation, in both runtimes.**
Files: `rust/tcl-vm/src/interp.rs` (`guarded_commands: RefCell<HashMap<String, (u64, BTreeSet<GuardIdentity>)>>`
keyed with the command's `command_identity.generations` entry;
`bump_cmd_epoch` no longer clears it; `GuardDomain::CommandEnvironment`
invalidation becomes per token: a mutation of token T drops T's guards
and leaves the rest; rename and hide move the entry with the builtin
identity as `builtin_identities` moves; `set_dialect_profile` keeps the
entries whose spec is in the pinned generation), `runtime/rust/src/interp.rs`
(`invalidate_command_environment` the same way), `rust/tcl-runtime-api/src/guard.rs`
(a `GuardDomain::CommandToken(generation)` if the domain lattice needs the
finer key — the implementer decides whether the existing domain set
suffices and records it), `docs/design/runtime/rename-alias.md` § 3.5,
`docs/design/runtime/tclvm-opcode-status.md`.
Preserve: a renamed or traced intrinsic still falls back (the two rows in
`wasm_real_link.rs`); the interpreter and object-dispatch domains stay
poisoned. Changes: a `proc foo` after registration no longer disables
`string length`'s fast path. Tests: `runtime/rust/src/interp.rs`'s
`command_mutation_invalidates_guard_and_identity_attestation` re-stated
(three rows: unrelated proc keeps; `rename string x` drops; the profile pin
keeps); `rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` twin;
`wasm_real_link.rs` gains the "unrelated proc" row. Gates: `tcl-fuzz`
campaign (`cargo run -p tcl-fuzz -- run` over the two-backend pair per
`docs/kcs/kcs-howto-work-on-fuzz-findings.md`), recorded here. Docs: the
two runtime pages; the page's § *Codegen and the registry today* diagram
"never repopulated" row flips. Model: opus. Size: L. After: CC5.1.

**CC5.3 — the Explorer observability record.**
Files: `rust/tcl-compiler/src/mixed_region_plan.rs`, `common_aot_plan.rs`
(a failed native-add composition serialises every rejected premise as a
`NativeDecline { premise, reason }` list on the region's plan),
`rust/tcl-explorer/src/serialise.rs` and `view_tree.rs` (the `aot` view
renders them; `tcl explore --show aot --text` prints them). Tests:
`rust/tcl-compiler/tests/wasm_tiers.rs` gains
`a_failed_native_add_names_every_rejected_premise`; an explorer text
snapshot. Docs: `semantic-aot-optimisation.md`'s "That observability gap
must close" sentence goes. Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC5.1 (`wip(consumer-contracts): one semantics key per intrinsic`), CC5.3
(`… the AOT decline record`), CC5.2 last (`… guard identities persist`).
Green: `cargo test -p tcl-registry`; `cargo test -p tcl-runtime`;
`cargo test -p tcl-vm`; `cargo test -p tcl-compiler --test wasm_real_link --test wasm_tiers`;
`cargo test -p tcl-explorer`.

#### Review checklist

- Both runtimes change together (the page's "a design change to a tested
  contract in both runtimes"); the fuzz campaign is recorded with its seed
  count and release.
- A persisted identity is invalidated when its dependency changes: the
  `rename` and `trace` rows still fall back.
- No identity is attached by name after registration (the sweep of CC7.2
  is the only attach path; step 5 adds none).
- Risks: a guard surviving a `proc string` in a child namespace that
  shadows the builtin for calls made from that namespace (the token
  generation is per fully-qualified key; the binding check re-resolves the
  namespace — assert it in a test row).

#### Behavioural deltas expected

- After any `proc`, `rename` or `interp alias` of an unrelated command, a
  guarded intrinsic fast path stays live in both runtimes; today it is
  disabled for the rest of the interpreter's life.
- The Explorer's `aot` view lists rejected native premises.

### Step 6 — the take-shipped floor, the capability matrix, the overlay to the compile service

**Goal.** The take-shipped floor over the whole codegen and runtime axis;
`CodegenCapability` applied by `DependencyTier`; the overlay generation
delivered to the compile service; an overlay miss failing closed.

**Exit evidence.** `cargo test -p tcl-spectcl --test i6_security_floor`
green with the six new rows; `cargo test -p tcl-registry` green with the
ingress test inverted; `cargo test -p tcl-lsp-db --test dialect_seam`
green; the server's optimise path compiles under the workspace overlay.

#### Work items

**CC6.1 — the floor widens.**
Files: `rust/tcl-registry/src/security_floor.rs` (`apply` takes
`lowering_hook`, `analyser_hook`, `semantic_operation`, `state_transitions`,
`native_lowering`, `bpf_op` shipped; `MERGED_FIELDS` lists them;
`every_security_bearing_field_is_in_the_floor` names the six explicitly
beside its `taint` / `codegen` filter). Tests: `i6_security_floor.rs` —
one row per field: `a_workspace_override_cannot_swap_the_lowering_hook`
etc. (negative: `the_floor_does_not_invent_facts_for_a_new_command`
unchanged). Docs: `spec-packs.md` § *Workspace trust* floor sentence;
the page's rule 4 "today" goes. Model: sonnet. Size: S. After: —.

**CC6.2 — `DependencyTier` and `CodegenCapability`.**
Files: `rust/tcl-registry/src/model/capability.rs` (new: `DependencyTier`,
`CodegenCapability`, `CodegenCapability::for_tier(tier) -> Self` — the
page's matrix: `Root` everything; `Direct` `runtime_backing` and
`builtin_alias`, no stamps, no body; `Transitive` and `Development`
nothing), `rust/tcl-spectcl/Cargo.toml` (`tcl-pkg` dependency, data model
only — D6.2), `rust/tcl-spectcl/src/discovery.rs` (`PackFile::dependency_tier: Option<DependencyTier>`:
a pack beside a manifest that the nearest `tclpkg.lock` lists as a
dependency gets the tier the lockfile's graph gives it — in the root
manifest's `requires` `Direct`, reached only transitively `Transitive`,
`dev` `Development`; the workspace's own manifest `Root`; no lockfile or
not listed `None`), `rust/tcl-spectcl/src/stamps.rs` (a second gate after
CC4.2's: `alias_of`, `runtime_backing` (step 7) and a reference body
(step 8) are dropped with a notice when the capability forbids them;
both gates must pass), `docs/design/registry/spec-packs.md`.
Tests: `workspace_packs.rs` gains `a_transitive_dependencys_alias_of_is_dropped`
and `a_direct_dependency_keeps_alias_of_but_not_a_stamp` (negative:
`Root` keeps both). Model: opus. Size: M. After: CC4.2.

**CC6.3 — the overlay reaches the compile service; a miss fails closed.**
Files: `rust/tcl-registry/src/model/ingress.rs` (`context_registry` and
`registry_for_environment_if_built` return `Result<Arc<ContextRegistry>, OverlayMiss>`;
the test at `ingress.rs:796` asserting the fallback is inverted),
`rust/tcl-compiler/src/compile_service.rs` (`RegistryTarget::Overlay { profile, overlay: u64 }`;
`BytecodeCompileService::for_profile_with_overlay`), `rust/tcl-lsp-db/src/lib.rs`
(`compilation_unit` and `registry_with_overlay` propagate the error as a
diagnostic-free abstention: the unit is not built and the reason is
logged once), `rust/tcl-lsp-server/src/lib.rs` (`optimise_document_command`
builds the service with `spec_pack_key`), `rust/tcl-vm` (unchanged: the
service is injected). Preserve: every compile with an installed overlay
or no overlay. Changes: an uninstalled overlay is an error, not a silent
un-overlaid compile. Tests: `ingress.rs` unit `an_uninstalled_overlay_is_an_error_not_the_plain_generation`;
`rust/tcl-lsp-db/tests/dialect_seam.rs` gains
`the_compilation_unit_sees_the_packs_commands` (a pack command resolves in
the unit; negative: after the pack is retired the unit is rebuilt without
it, never with a stale overlay). Boundary: slice 4 of the value-transfers
lane lands `spec_pack_key` reaching `compilation_unit` first (B3); this
item lands after it. Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC6.1 (`wip(consumer-contracts): the take-shipped floor covers the codegen axis`),
CC6.2 (`… the dependency-tier capability matrix`), CC6.3 (`… overlay misses fail closed`).
Green: `cargo test -p tcl-registry`; `cargo test -p tcl-spectcl`;
`cargo test -p tcl-lsp-db`; `cargo test -p tcl-compiler --test codegen`.

#### Review checklist

- The floor is a codegen-axis contract, not a trust gate on analysis
  facts: an override still changes arity, roles, hover.
- Both gates (provenance, capability) apply and a declaration passes both;
  the notices name which one refused.
- No silent fallback remains in `ingress.rs` (grep `unwrap_or` around the
  overlay lookup).
- Risks: `tcl-pkg` as a `tcl-spectcl` dependency pulling `tcl-sandbox` into
  every server build (check the build graph; if it hurts, the lockfile
  parser moves to a `tcl-pkg-model` crate — record the decision).

#### Behavioural deltas expected

- A workspace override that swapped `lowering_hook`, `analyser_hook`,
  `semantic_operation` or `state_transitions` keeps the shipped value.
- An uninstalled overlay yields no compilation unit instead of an
  un-overlaid one.

### Step 7 — identities from the pinned generation, `runtime_backing`, the intrinsic families, the manifest, the runtime context

**Goal.** Identities attached after registration and after the pin from
the pinned shipped generation only; `runtime_backing` a registry fact
with `command_backing`'s lists as its rows and a query both runtimes
answer; the intrinsic table split by family; `ArtefactIdentityManifest`
on `CompiledUnit` and as a WASM custom section; the runtime pin a context;
a fuzz campaign as the exit.

**Exit evidence.** `cargo xtask command-backing --check` green with the
report generated from the query and `KNOWN_UNBACKED` the only list left;
`cargo test -p tcl-runtime` and `cargo test -p tcl-vm` green; the
manifest witnesses in `rust/tcl-vm/tests/command_mutation_deopt_e2e.rs`
and `rust/tcl-compiler/tests/wasm_real_link.rs`; one fuzz campaign over
the two-backend pair recorded here.

#### Work items

**CC7.1 — `RuntimeBacking` on the spec.**
Files: `rust/tcl-registry/src/runtime_backing.rs` (new: `RuntimeBacking`,
`BodySource` verbatim; `CommandSpec::runtime_backing: RuntimeBacking`,
default `None`), every core spec (`ShippedBuiltin { identity }` for the
282 handler and native rows, `TclBody { source: PackageSource { relative_path: "init.tcl" | "package.tcl" | "parray.tcl" } }`
for the 11 stdlib rows, `None` for the 54 not-required rows; the 42
known-gap rows declare `ShippedBuiltin` — the target state — and stay on
`KNOWN_UNBACKED` as the drift waiver, D7.1), `security_floor.rs`
(`runtime_backing` joins the take-shipped list), `rust/tcl-spectcl/src/loader.rs`
(`runtime_backing shipped-builtin ID | tcl-body {-package-source PATH} | tcl-body {-pack-text {…}} | host-native | none`;
`PackText` emits an Information notice at load), the four studio
surfaces, `rust/xtask/src/gen_irule_test_data.rs` (emits a mock only for
`None` or `HostNative`; every iRules command is `None`, so the output is
byte-identical). Tests: `registry_sweep.rs` — `every_core_command_declares_a_backing`;
`gen_irule_test_data.rs` — `committed_files_match_registry_generation`
unchanged. Gates: `gen-irule-test-data --check`, `pack-goldens --check`,
`reference_doc`. Model: sonnet (the rows are the report's rows). Size: M.
After: —.

**CC7.2 — the backing query in both runtimes; the gate asks it.**
Files: `runtime/rust/src/interp.rs` (`Interp::backing_report(&self) -> Vec<(String, RegisteredBacking)>`
over the handler table: `Builtin` for a registered fn, `Object` for a
TclOO bootstrapped object, `Stdlib` for a name the embedded library
defines, `Absent`; `register_spec_builtin` stops reading `build_default()`
— `cmd_string.rs:47` — and the identity sweep `attach_identities(&mut self, registry: &CommandRegistry)`
runs at the end of `register_builtins` and again in the profile pin,
from the pinned generation only), `rust/tcl-vm/src/interp.rs` (the same
two functions), `rust/xtask/Cargo.toml` (`tcl-runtime` and `tcl-vm`
dependencies — D7.1), `rust/xtask/src/command_backing.rs` (the source
scan, `HANDLER_EXTRA`, `STDLIB` and `NOT_REQUIRED` deleted; the report is
rendered from `spec.runtime_backing` against both runtimes'
`backing_report()`; a spec declaring `ShippedBuiltin` whose runtime report
is `Absent` fails unless it is on `KNOWN_UNBACKED`; a spec declaring
`None` that the runtime registers is drift), `docs/generated/wasm-command-backing.md`
(regenerated: the same 389 rows, the `backing` column now the declared
fact and a new `vm` column for `tcl-vm`). Tests: `command_backing.rs` unit
rows re-stated over the query (`a_declared_builtin_the_runtime_lacks_is_drift_unless_waived`,
`a_declared_none_the_runtime_registers_is_drift`); `runtime/rust/src/interp.rs`
gains `identities_come_from_the_pinned_generation_never_an_overlay`
(negative: an overlay-only spec attaches nothing). Gates:
`command-backing --check` (the report regenerated and staged). Docs:
`AGENTS.md` § *WASM command parity* (the gate's description), the page's
§ *Consequences for the runtimes* second bullet flips. Model: opus. Size:
L. After: CC7.1.

**CC7.3 — the intrinsic table by family.**
Files: `rust/tcl-registry/src/intrinsic.rs` (`IntrinsicId::family(self) -> IntrinsicFamily`
— `Value` for the shared-core functions, `FamilyB { domain: GuardDomain, fires_traces: bool }`
for the variable-store and channel operations; `info exists` and the
array operations `fires_traces: true`), `runtime/rust/src/codegen_abi.rs`
and `rust/tcl-vm` (the guard domain a registration derives comes from the
family, not from a per-call table). Tests: `intrinsic.rs` —
`every_member_names_a_family`, `trace_firing_members_are_family_b`.
Docs: `docs/design/runtime/family-b-routing.md` § 4 (the gap row closes).
Model: opus. Size: S. After: CC5.1.

**CC7.4 — `ArtefactIdentityManifest` and the runtime context pin.**
Files: `rust/tcl-runtime-api/src/manifest.rs` (new: `ArtefactIdentityManifest`
verbatim; `BuildProfileId` is `tcl_dialect::build_info`'s id),
`rust/tcl-vm/src/compiled.rs` (`CompiledUnit::manifest: Option<ArtefactIdentityManifest>`),
`rust/tcl-bytecode/src/lib.rs` (`ModuleAsm::manifest`), the compiler's
emitters (fill it from the `AnalysisContext` and the pack set),
`rust/tcl-compiler/src/codegen/wasm/encoding.rs` (a custom section
`tcl.manifest`, a length-prefixed field list in declaration order),
`rust/tcl-vm/src/environment.rs` and `runtime/rust/src/environment.rs`
(`RuntimeContext { environment, release, build, packages, overlay_generation }`
resolved through `tcl_registry::model::ingress` — the same ingress the
compiler uses — replacing the profile-only pin; `set_dialect_profile`
becomes `pin_context(&RuntimeContext)` with the profile form kept as a
constructor; the `namespace` and `trace` subcommand gates read the pinned
profile, not the release name), `rust/tcl-vm/src/exec.rs`
(`validate_module_profile` becomes the manifest check: per rung, a
disagreeing field refuses only the sites that rest on it), the WASM link
harness (`wasm_real_link.rs` helpers read the section and refuse on an
`abi_version` or `intrinsic_table_hash` mismatch). Tests:
`command_mutation_deopt_e2e.rs` — `a_manifest_disagreeing_on_packs_refuses_only_rung_one_sites`,
`a_rung_zero_unit_is_admitted_under_a_changed_pack_set`;
`wasm_real_link.rs` — `a_module_with_a_foreign_intrinsic_table_is_refused`
(negative: the matching hash links). Docs:
`vm-compiled-artifact-provenance.md`; the page's manifest "proposed" note
flips. Model: opus. Size: L. After: CC4.4, CC7.2.

**CC7.5 — the fuzz exit.** One `tcl-fuzz` campaign over the
`tclvm`/`runtime-rust` pair after CC7.4 (the skill `fuzz-findings`); seed
count, release and outcome recorded in this document's status section.
Model: sonnet. Size: S. After: CC7.4.

#### Ordering and checkpoints

CC7.1 (`wip(consumer-contracts): runtime_backing rows`), CC7.3
(`… intrinsic families`), CC7.2 (`… the backing query replaces the scan`),
CC7.4 (`… the artefact manifest and the runtime context`), CC7.5. Green:
`cargo test -p tcl-registry`; `cargo test -p tcl-runtime`;
`cargo test -p tcl-vm`; `cargo test -p xtask`; `command-backing --check`;
`gen-irule-test-data --check`; `cargo test -p tcl-compiler --test wasm_real_link --test wasm_codegen --test codegen_integration`.

#### Review checklist

- Registration stays a runtime-owned handler table in its documented order
  (TclOO overrides `variable`, the event loop replaces `update`, `string`
  last); the sweep attaches identities, it never registers.
- The gate's residue is one list (`KNOWN_UNBACKED`); the report is a
  rendering of the query, and `--check` fails on a changed row.
- The manifest is checked per rung, never as a global refusal.
- The `xtask` build time with the two runtime crates linked is measured
  and recorded (if it exceeds a minute, the query moves behind a
  `cargo xtask command-backing --from-runtime` build step — record the
  decision).
- Risks: the identity sweep running before `package` is registered (the
  first pin happens inside `register_builtins`); the custom section's
  byte layout drifting between emitter and reader without a shared
  encoder (one function pair in `tcl-runtime-api`, tested round-trip).

#### Behavioural deltas expected

- `docs/generated/wasm-command-backing.md` gains a `vm` column and reads
  its `backing` column from the spec.
- A bytecode module compiled under one pack set is refused for its rung-1
  sites under another; a WASM module with a different intrinsic-table hash
  is refused at link.

### Step 8 — reference bodies, `tcl spec test`, the manifest `spec` directive

**Goal.** A reference Tcl body as a declared implementation and as code;
`tcl spec test`; the manifest `spec` directive with its `DependencyTier`
and lockfile hash. `Engine::set_release` is slice 4's and is consumed,
not added (B3).

**Exit evidence.** `rust/tcl-spectcl/tests/codegen_stamps.rs` rung-3
rows green; `cargo test -p tcl-cli` with the `spec test` rows;
`cargo test -p tcl-pkg` with the directive and lockfile rows;
`cargo xtask pack-goldens --check`.

#### Work items

**CC8.1 — the reference body as code (rung 3).**
Files: `rust/tcl-runtime-api/src/site_claim.rs` (`SiteClaim::ReferenceBody`),
`rust/tcl-compiler/src/inlining/` and `inline_uplevel.rs` (a body inlined
for a command whose `runtime_backing` is `TclBody` records the claim with
`ProcedureBindingIdentity` built from the body source; `PackageSource`
bodies are read through the host filesystem seam — the value-transfers
lane's pinned provisioning path — never `std::fs`), `rust/tcl-vm/src/interp.rs`
(`procedure_binding_matches` gains the conjunct: the claim's backing is
`TclBody`; activation defines the proc only then), `rust/tcl-vm/src/command.rs`
(the `source` command's direct `std::fs::read_to_string` at `:307` routed
through `tcl_platform`'s host filesystem seam and honouring `-encoding`
— the page's § *Consequences for the runtimes* fifth bullet, landed here
because rung 3 is its first consumer). Tests: `codegen_stamps.rs` —
`a_tcl_body_backed_command_is_inlined_and_admitted`,
`a_host_native_backing_never_defines_a_proc` (negative: the same body with
`host-native` backing is neither inlined nor activated),
`a_pack_text_body_that_diverges_turns_the_site_plain`. Model: opus. Size:
L. After: CC7.1, CC7.4.

**CC8.2 — the reference body as a declared implementation and as a
derivation source.**
Files: `rust/tcl-registry/src/value_transfer/declaration.rs` (a
`TclBody`-backed command whose body the sandbox can express — no
`upvar`, `uplevel`, `global`, `variable`, channel or `exec` word, decided
by a static scan the registry owns — derives a `Declared` semantics with
`EvalRoute::Implementation`; the value-transfers lane's route machinery
runs it under `Engine::set_release`), `rust/tcl-spec-studio/src/infer.rs`
(`infer_from_body(body) -> InferredFacts { pure, side_effects, return_type, callback_slots }`
through `tcl_compiler`'s interprocedural summary; `tcl spec import` and
the `spec-author` skill's draft carry them as proposals), `ai/claude/skills/spec-author/SKILL.md`
(the "questions only the author can answer" list shrinks). Tests:
`rust/tcl-registry/tests/value_transfers.rs` gains
`a_reference_body_is_a_declared_implementation_when_the_sandbox_can_express_it`
(negative: a body with `upvar` derives nothing); `rust/tcl-spec-studio`'s
`infer` tests gain a body row. Gates: `value-transfers --check` (the
inventory gains the route; the pinned route set in `value_transfers.rs`
grows — coordinate with the lane, B1). Model: opus. Size: L. After:
CC8.1, and slice 4 of the value-transfers lane.

**CC8.3 — `tcl spec test`.**
Files: `rust/tcl-cli/src/cli.rs` (`SpecCommand::Test(SpecTestArgs { pack, tclsh, package })`),
`rust/tcl-cli/src/commands/spec.rs` (`run_test`: loads the pack, requires
the package in the named shell under `tcl-pkg`'s policy
(`rust/tcl-pkg/src/policy.rs`, opt-in), then for every command diffs the
declared facts against the implementation — arity windows against
`wrong # args`, `return_type` and each `hover.examples` line against the
shell's result, a `TclBody` reference body against the real command on
the same inputs, `pure` against a second run with traced globals —
printing one row per divergence; exit 1 on any), `docs/kcs/features/kcs-feature-tcl-verb-cli.md`.
Tests: `rust/tcl-cli/tests/` gains `spec_test_reports_an_arity_divergence`
over a fixture pack and a fixture Tcl package (skipped without a
`tclsh` on `PATH`). Model: opus. Size: M. After: CC8.1.

**CC8.4 — the manifest `spec` directive, the lockfile hash, the container generator.**
Files: `rust/tcl-pkg/src/manifest.rs` (`ManifestAst::spec: Option<SpecDirective>`;
`SpecDirective { packs: Vec<String>, requested_tier: DependencyTier }`
with `DependencyTier` defined in `tcl-pkg` (the page's enum; `tcl-registry`'s
`model::capability::DependencyTier` becomes a re-export of it once the
dependency exists — D6.2), parsed from `spec { packs {a.tclspec} tier direct }`;
data-only, never executed), `rust/tcl-pkg/src/lockfile.rs`
(`LockedPackage::spec_integrity: Option<String>` — the xxh3 of each pack
as hex, joined; `PackFactStamp::content_hash` is the same value),
`rust/tcl-pkg/src/resolver.rs` (the tier clamp: the declared tier is never
nearer than the resolution's), `rust/tcl-spectcl/src/discovery.rs`
(reads the directive's paths as the pack list beside a manifest instead of
globbing), `rust/tcl-cli/src/commands/docker.rs` (`tcl docker create`
computes the `HostNative` commands' extensions from the packs'
`runtime_backing` and passes `DockerfileSpec::native_extensions: Vec<String>`;
`rust/tcl-pkg/src/docker.rs` renders the install lines and stays
registry-free — D8.2), `docs/design/tclpkg/architecture.md` § Manifest and
§ Lockfile. Tests: `rust/tcl-pkg` unit rows — `the_spec_directive_is_data_only`,
`a_changed_pack_changes_the_lockfile` (negative: an unchanged pack keeps
the hash), `a_manifest_cannot_claim_a_nearer_tier`;
`rust/tcl-pkg/tests/manifest_env_drift.rs` unchanged. Model: opus. Size:
M. After: CC6.2, CC7.1.

#### Ordering and checkpoints

CC8.4 first (independent of the runtimes;
`wip(consumer-contracts): the manifest spec directive`), CC8.1
(`… reference bodies as code`), CC8.3 (`… tcl spec test`), CC8.2 last
(`… reference bodies as declared implementations`, after slice 4). Green:
`cargo test -p tcl-pkg`; `cargo test -p tcl-spectcl`; `cargo test -p tcl-vm`;
`cargo test -p tcl-cli`; `cargo test -p tcl-registry --test value_transfers`;
`value-transfers --check`.

#### Review checklist

- Rung 3's extra conjunct: no proc is ever defined for a `HostNative` or
  `None` backing (the negative witness).
- A `PackText` body is reported at load and turns its sites plain on the
  first mismatch.
- `tcl spec test` never runs at editor load; it is a CLI verb under the
  package manager's opt-in policy.
- The lockfile hash and the artefact stamp are one value.
- Risks: the sandbox-expressibility scan admitting a body that reads
  world state through a command the scan does not know (the scan is a
  whitelist of commands, never a blacklist — assert it).

#### Behavioural deltas expected

- A pack with `runtime_backing tcl-body {-pack-text …}` loads with an
  Information notice.
- `tcl docker create` lists a `HostNative` command's extension.
- A `tclpkg.lock` gains a `spec_integrity` field for packages that ship
  packs.

### Step 9 — the codegen axis versioned; evaluation points through the evidence gate

**Goal.** Codegen-axis facts versioned as arity is (ordered windows, first
covering row wins, selected at the primary, plain dispatch when a
declared target range disagrees with the primary); a vendor
environment's evaluation point fed through the evidence gate rather than
answered by name; package version windows carried on `SurfaceQuery`.

**Exit evidence.** `cargo test -p tcl-registry` with the window sweep;
`cargo test -p tcl-compiler --test codegen --test dialect_threading`
with the straddle row; the value-transfers lane's
`kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md` witnesses
updated for iRules.

#### Work items

**CC9.1 — versioned stamps.**
Files: `rust/tcl-registry/src/spec.rs` (`codegen_hook_windows: &'static [StampWindow<CodegenHookId>]`,
`inline_codegen_hook_windows`, `semantic_operation_windows`,
`native_lowering_windows` beside the unversioned fields, each a
`StampWindow<T> { lifecycle: Lifecycle, value: T }` mirroring `ArityWindow`
in `arity.rs`), `resolved_invocation.rs` (selection at the primary
release; a query whose release range straddles two windows or a window
edge answers `None` — plain dispatch), the loader (`codegen_hook -native ID -introduced V ?-deprecated V? ?-retired V?`
repeatable, as `arity` is), the four studio surfaces,
`docs/design/spec-dsl-examples/README.md`. Tests: `registry_sweep.rs` —
`stamp_windows_never_overlap` (the arity-window gate's twin);
`rust/tcl-compiler/tests/codegen.rs` — `a_stamp_declared_from_9_0_declines_under_a_profile_spanning_8_6`
(negative: pinned at 9.0 it specialises). Model: opus. Size: M. After:
CC4.2.

**CC9.2 — the evidence gate.**
Files: `rust/tcl-dialect/src/profile.rs` (`DialectProfile::evaluation_point(&self) -> Option<TclVersion>`:
`runtime_base` when the profile's row in `data/reference-toolchains.tsv`
or the profile's own measured-fork note marks it measured, else `None`),
`rust/tcl-dialect/src/version.rs` (`TclVersion::from_profile` delegates to
it), `rust/tcl-registry/src/value_transfer/const_ops.rs:325` (unchanged
call, new answer — the value-transfers lane owns the file; the change is
one line in `version.rs`, B1), `rust/tcl-spec-hooks/src/host.rs` (the
`tcl-version` context key for a vendor profile). Changes: under
`f5-irules` a versioned fold answers as Tcl 8.4 (`incr` over `010`
folds to 9; a bignum overflow declines) where today it answers the
unanimous subset. Tests: `profile.rs` — `a_measured_vendor_profile_has_an_evaluation_point`,
`an_unmeasured_profile_has_none` (negative); the value-transfers lane's
`differential_fold.rs` rows for iRules updated with it (B1). Docs: the
lane's KCS note; `docs/design/registry/dialect-profile-model.md` (the
gate paragraph). Model: opus. Size: M. After: —.

**CC9.3 — package version windows.**
Files: `rust/tcl-dialect/src/model/authored_surface.rs`
(`SurfaceQuery::packages: &'a [PackageFloor<'a>]`, `PackageFloor { name, version: Option<&str> }`;
`surface_admits` consults a package row's `Lifecycle` against the floor),
every `SurfaceQuery` constructor (`core`, the profile's `surface_query`),
`rust/tcl-registry/src/registry.rs` (`ambient_package_floor` feeds the
floor). Tests: `rust/tcl-registry/tests/library_axis.rs` gains
`a_package_row_introduced_after_the_floor_is_not_admitted` (negative: at
or above the floor it is). Model: opus. Size: M. After: —.

#### Ordering and checkpoints

CC9.2 (`wip(consumer-contracts): evaluation points through the evidence gate`),
CC9.3 (`… package version windows`), CC9.1 (`… versioned codegen stamps`).
Green: `cargo test -p tcl-dialect`; `cargo test -p tcl-registry`;
`cargo test -p tcl-compiler`; `cargo test -p tcl-spec-hooks`.

#### Review checklist

- "Per measured row, never by name": no `match profile.name` decides a
  release.
- A straddle is a decline to plain dispatch, never a silent choice of one
  row.
- Risks: the iRules fold delta reaching `tcl-irule-test` fixtures
  (`gen-irule-test-data --check`); `SurfaceQuery` is `Copy`-shaped and
  widely constructed — the compile fan-out is large but mechanical.

#### Behavioural deltas expected

- Under `f5-irules` and `f5-iapps`, versioned folds answer as Tcl 8.4.
- A codegen stamp with a window outside the profile's range specialises
  nothing.

### Step 10 — the extension legs

**Goal.** In the page's order: the conservative default fact; the C scan,
the probe and the bridge; the host opt-in `load`; the authored header
with its CI gate, `Tcl_CreateObjCommand` and the shared-table `Command`
variant; the engine interface's completion and variable doors;
WASM-hosted evaluation last.

**Exit evidence.** `rust/tcl-cshim/tests/pkga_e2e.rs` green against the
authored header on both legs (the same vectors); the CI gate compiling
`pkga.c` for `wasm32-wasi`; `sandbox_isolation.rs` unchanged; one
extension evaluated under WASM through the engine interface with fuel.

#### Work items

**CC10.1 — the conservative default fact.**
Files: `rust/tcl-registry/src/spec.rs` (`CommandSpec::extension_default(name: &'static str) -> Self`:
`Arity::any()`, `Traits::EVALUATES_CODE | CREATES_BARRIER | CREATES_DYNAMIC_BARRIER | UNSAFE | SAFE_INTERP_HIDDEN | TAINT_SINK | TAINT_SOURCE | ESTABLISHES_VARIABLE_TRACE`
(the implementer verifies each name in `traits.rs`), `side_effects`
Unknown reads and writes, `command_table_effect: Some(Unknown)`,
`completion` any code with a normal successor, `pure: false`,
`runtime_backing: HostNative`), `rust/tcl-registry/src/model/declaration.rs`
(`DeclaredCommand::extension(name)` seeded from it and narrowed by the
CC3.5 flags axis by axis), `rust/tcl-compiler/src/analyser/types.rs` (a
stub for a name a `load`-provided package declares starts from the
default: the `provides` directive / extra-commands path). Tests:
`registry_sweep.rs` — `the_extension_default_is_at_the_top_of_every_axis`
(negative: narrowing `-pure` clears `TAINT_SOURCE`? No — it clears only
purity; assert the other axes stay). Model: opus. Size: S. After: CC7.1,
CC3.5.

**CC10.2 — the C scan, the probe and the bridge.**
Files: `rust/tcl-spec-studio/src/infer.rs` (`scan_c_source(text) -> Vec<CDeclaredCommand>`
over `Tcl_CreateObjCommand` / `Tcl_CreateCommand` / `Tcl_NRCreateCommand`
name literals, `Tcl_PkgProvide(Ex)`, `Tcl_WrongNumArgs` usage strings,
`Tcl_GetIndexFromObj(Struct)` tables, variable and eval calls, factory
patterns; blind to the OO C API and C-built ensembles by declaration),
`rust/tcl-cli/src/commands/spec.rs` (`tcl spec import --c-source DIR`
and `--probe PACKAGE` (the sandboxed `package require` under `tcl-pkg`'s
policy, listing the `info commands` delta), each row carrying its
provenance `c-scan` / `probe`), `rust/tcl-cshim/src/lib.rs`
(`Loaded::declared_surface(&self) -> Vec<DeclaredCommand>` — the bridge:
every registered command at the default fact), `ai/claude/skills/spec-author/SKILL.md`.
Tests: `infer.rs` — `the_scan_finds_pkga_s_commands_and_provide`
(over `rust/tcl-cshim/tests/c/pkga.c`; negative: a `Tcl_CreateObjCommand`
with a computed name yields a row marked dynamic); `pkga_e2e.rs` —
`the_loaded_report_bridges_to_the_default_fact`. Model: opus (the scan),
sonnet (the bridge). Size: L. After: CC10.1.

**CC10.3 — the host opt-in `load` bridge.**
Files: `rust/tcl-engine-tclvm/src/lib.rs` (`TclVmEngine::enable_static_extensions(&mut self, table: &'static [(&'static str, tcl_cshim::InitProc)])`:
a host command `load` that resolves its name in the table and calls
`tcl_cshim::Interp::load_static` on a shim interpreter sharing this
engine — the shim's `Interp` gains a constructor over `&mut E` for the
duration of one load, `Interp::load_static_into(engine, init)`, so the
engine is not moved), `rust/tcl-cshim/src/lib.rs`, `rust/tcl-vm-cli`
(the opt-in flag). Constraints: trusted host code only; no pack word
reaches it; `sandbox_isolation.rs` stays green. Tests: `pkga_e2e.rs` —
`load_through_the_host_bridge_defines_the_commands` (negative: a name not
in the table is `couldn't load`). Model: opus. Size: M. After: CC10.1.
Open question Q6.

**CC10.4 — the authored header, the CI gate, `Tcl_CreateObjCommand`, the
shared-table variant.**
Files: `runtime/rust/include/tcl.h` (new, the ABI § 7 scope, with
`TCL_MAJOR_VERSION` switching `Tcl_Size` as `tclshim.h`'s
`TCL_SHIM_TCL_MAJOR` does; every declaration a host does not implement is
absent from that host's build through `TCL_HOST_NATIVE` /
`TCL_HOST_WASM` guards), `rust/tcl-cshim/include/tclshim.h` (deleted),
`rust/tcl-cshim/build.rs` (compiles `tests/c/pkga.c` against the authored
header), `rust/tcl-cshim/src/obj.rs` (`#[repr(C)] pub struct Obj` publishes
the § 4.2 layout), `runtime/rust/src/capi.rs` (`Tcl_CreateObjCommand`,
`Tcl_DeleteCommand`), `runtime/rust/src/interp.rs` (`Command::ObjCmd { proc: Tcl_ObjCmdProc, client_data: *mut c_void, delete_proc: Option<Tcl_CmdDeleteProc> }`
— a shared-table function index under WASM; `dispatch` calls it with the
prebuilt argv `tcl_invoke_argv` already routes), `.github/workflows/`
and `Makefile` (`check-c-extension-wasm`: wasi-sdk clang compiles
`pkga.c` for `wasm32-wasi` against the header and links it with the
runtime; in `xtask-check`'s CI mirror), `docs/design/runtime/c-extension-shim.md`
§ *The implemented subset* and `c-extension-abi.md` § 7 (the "today"
sentences go), `docs/GLOSSARY.md`, `docs/kcs/kcs-qa-what-is-the-c-extension-shim.md`.
Preserve: every `pkga_e2e.rs` expectation byte-for-byte. Tests:
`pkga_e2e.rs` unchanged; `rust/tcl-compiler/tests/wasm_real_link.rs`
gains `a_compiled_script_calls_an_extension_registered_command` (the § 12
seam: `Foo_Init` registers `foo`, a compiled script calls it; negative:
before registration the call is `invalid command name`). Model: opus.
Size: L. After: CC10.2.

**CC10.5 — the engine interface's completion and variable doors.**
Files: `rust/tcl-engine-api/src/lib.rs` (`HostCommand::invoke` returns
`Result<HostOutcome, EngineError>` with `HostOutcome { value, code: CompletionCode }`;
`Engine::variable(&self, name) -> Result<Option<Value>, EngineError>`,
`set_variable`, `unset_variable`; `Engine::eval_in_invocation(&mut self, script) -> Result<HostOutcome, EngineError>`),
`rust/tcl-engine-tclvm/src/lib.rs`, `rust/tcl-cshim/src/lib.rs:207–219`
(the `TCL_BREAK` / `TCL_CONTINUE` / `TCL_RETURN` narrowing goes;
`Tcl_ObjSetVar2`, `Tcl_GetVar2Ex`, `Tcl_EvalObjEx` exported through the
doors), `rust/tcl-spec-hooks/src/host.rs` (unchanged: a hook body's
completion is still an abstention). Tests: `rust/tcl-cshim/src/lib.rs`
unit rows — `a_c_command_returning_break_is_a_break_completion`;
`pkga_e2e.rs` gains a `Tcl_EvalObjEx` row; `rust/tcl-engine-tclvm`'s
tests gain the variable door. Model: opus. Size: M. After: CC10.4.

**CC10.6 — the WASM runtime implements `Engine`; WASM-hosted evaluation.**
Files: `runtime/rust/src/engine.rs` (new: `impl Engine for Interp` under a
`engine` feature — the hook host and the shim can target it),
`rust/tcl-vm-wasm` (the second WASM engine with the browser host, the
fallback profile, no filesystem — the same statements), `rust/tcl-spec-hooks/src/host.rs`
(a body tested on two engines), the extension evaluation route
(`EvalRoute::Implementation` with `EvaluatorCapability` naming the
extension artefact hash in the memo key; WASI imports for the clock,
filesystem, randomness, environment and arguments stubbed per side module
in the link harness; eligibility from a declared route on the command;
a per-extension differential vector against the real shell in
`rust/tcl-compiler/tests/wasm_real_link.rs` gating shipping). Tests:
`rust/tcl-spec-hooks/tests/families_e2e.rs` runs every family on both
engines; `wasm_real_link.rs` — `pkga_evaluates_under_wasm_with_fuel`
(negative: a body over budget is a `BudgetExceeded`, never a hang). Model:
opus. Size: L. After: CC10.4, CC10.5.

#### Ordering and checkpoints

CC10.1, CC10.2, CC10.3, CC10.4, CC10.5, CC10.6 in that order, one
checkpoint each (`wip(consumer-contracts): the extension default`,
`… describe an extension from three sources`, `… the host load bridge`,
`… one header, two hosts`, `… the engine's completion and variable doors`,
`… evaluation under WASM`). Green: `cargo test -p tcl-cshim` (native and,
where the toolchain exists, the wasm check), `cargo test -p tcl-runtime`,
`cargo test -p tcl-engine-tclvm`, `cargo test -p tcl-spec-hooks`,
`cargo test -p tcl-compiler --test wasm_real_link`; `make ensure-rust-deps`
before any WASM build on macOS.

#### Review checklist

- Extensions are recompiled, never binary-loaded; no pack word loads native
  code; a shimmed command is not a `-native` hook (`sandbox_isolation.rs`
  is the proof and stays untouched).
- A declaration outside a host's subset is absent from that host's leg of
  the header, never present and failing.
- C is never evaluated natively; the WASM route is gated by fuel, memory,
  a declared route, per-module WASI stubs and the differential vector.
- The engine interface carries the completion code before any hosted
  extension exercises the default fact.
- Risks: `Tcl_Obj` layout changes breaking `pkga_e2e.rs` byte-for-byte
  (the vectors are the gate); the `load` bridge's engine ownership
  (Q6); the WASI stubs leaking host time or randomness into an
  evaluation (assert determinism across two runs).

#### Behavioural deltas expected

- The shim compiles extensions against `runtime/rust/include/tcl.h`;
  `tclshim.h` is gone.
- `Command::ObjCmd` commands dispatch from compiled WASM code.
- `tcl spec import --c-source` and `--probe` produce draft packs with
  provenance rows.

### Boundaries with the other lanes

**B1 — value-transfers (slices 2–13).** Its slice 2 is at checkpoint
`f3f9390f` (not complete); its working tree holds uncommitted edits to
`rust/tcl-registry/src/value_transfer/*`, `rust/tcl-compiler/src/{sccp,value_transfer,static_loops,intervals,type_infer,taint}.rs`,
the optimiser, `rust/tcl-registry/tests/{value_transfers,differential_fold}.rs`,
`rust/xtask/src/value_transfers.rs`, and `docs/generated/value-transfers.md`.
This lane never edits any of those files. Shared files and their owners:

- `rust/tcl-registry/src/spec.rs`: the lane owns `semantics` on the three
  scopes; this lane adds `clause_grammar` (CC2.2), `option_effect_families`
  (CC2.6), `alias_of` (CC4.1), `runtime_backing` (CC7.1), the stamp
  windows (CC9.1) and deletes `substitution_resolver` and the five
  `CaseListSpec` option fields (CC2.6). Disjoint fields; whichever lands
  second rebases.
- `rust/tcl-registry/src/resolved_invocation.rs`: the lane owns
  `InvocationSemantics::value`; this lane adds the query methods (CC2.8).
  Disjoint impl blocks.
- `rust/tcl-registry/src/value_transfer/`: the lane's. This lane only
  *reads* `HandlerMatch`, `OperandId`, `AnalysisContext`; CC2.8's
  `AnalysisContext::surface_query` is the one addition, in `context.rs`,
  landed as a two-line commit the lane is told of.
- `rust/tcl-spectcl/src/loader.rs`, `render_spectcl.rs`, `schema.rs`,
  `draft.rs`, `help.rs`, `coverage.rs`: slice 4 adds the `semantics` /
  `evaluate` / `facts` spellings; this lane adds `clause_grammar`
  (CC2.3), `-effect` (CC2.5), `option -effect` (CC2.7), the resolver
  family (CC2.14), `alias_of` (CC4.1), `runtime_backing` (CC7.1), the
  windows (CC9.1). Each is its own statement or row flag; the `GAPS`
  table is edited by both (the lane removes `semantics`; this lane removes
  `semantic_operation`, `definition_body`, `substitution_resolver`,
  `pattern_arg_resolver`). Order: this lane's step 2 before slice 4 where
  possible; on conflict the second rebases and re-runs `spectcl_roundtrip`.
- `rust/tcl-spectcl/src/cache.rs`, `loader/eval.rs` (`EvalOptions`,
  `EvalSnapshotKey`): slice 4 adds target values; CC3.1 adds `trust`.
  Disjoint fields; the second rebases.
- `Engine::set_release` (`rust/tcl-engine-api`): slice 4's. CC8.2 waits
  for it.
- `rust/tcl-lsp-db/src/lib.rs` `spec_pack_key` → `compilation_unit`:
  slice 4's analysis half; CC6.3's compile-service half follows it.
- `subst`: slice 5 owns `template_plan` and the four template-word
  consumers (`dynamic_names.rs`, `push_substituted_commands`,
  `lowering/mod.rs`'s subst fold, `specialise_factories.rs`, extract-proc);
  this lane owns `subst_.rs`'s option rows and families, the
  `option_effects` derivation and the `substitutions_performed` projection
  (CC2.6). Slice 5 builds `template_plan` over `option_effects`.
- `rust/xtask/src/value_transfers.rs`, `docs/generated/value-transfers.md`,
  the migration plan's ledger: the lane's. CC2.1's gate is a sibling
  module; CC2.15 merges the roots after slice 2 lands. The ratchet pins in
  `value_transfers.rs` for files this lane rewrites (`lowering/structured.rs`,
  `analyser/commands.rs`, `analyser/oo.rs`, `analyser/handlers.rs`) are
  *not* lowered by this lane; the lane lowers them when it reviews those
  files, or this lane files a one-line request.
- `rust/tcl-dialect/src/version.rs` (CC9.2) changes what
  `const_ops.rs:325` answers for vendor profiles; the lane's
  `differential_fold.rs` rows for iRules move with it — CC9.2 lands only
  with the lane's agreement and its rows updated in the same checkpoint.

**B2 — diagnostic-policy (slices 4–7 at checkpoint `5bc40e95`).** It owns
`rust/tcl-lsp-core/src/diagnostic_policy.rs`, `diagnostic_report.rs`,
`config_ini*`, `code_actions.rs`, the diagnostic-code table, the xtask
catalogue generators, and the adapter edits in `rust/tcl-lsp-server/src/lib.rs`,
`rust/tcl-cli/src/commands/diag.rs`, `policy.rs`, `rust/tcl-mcp/src/tools.rs`.
This lane's server edits (CC3.3: `apply_initialization_options`,
`did_change_configuration`, `spec_pack_discovery`, `reload_spec_packs`)
and MCP edits (CC3.4: `spectcl.rs`, the `spectcl_check` entry in
`tools.rs`) are in different functions; they land after that lane's
adapter commits and rebase over them. This lane adds no diagnostic code
(W147 already exists); the option-relation finding CC2.6 adds is a
registry relation the existing producer reports. No new `DiagCode`, so no
catalogue regenerates.

**B3 — ordering across lanes.** Slice 4 (value-transfers) before CC6.3
and CC8.2; the diagnostic-policy adapter commits before CC3.3 and CC3.4;
everything else in this lane is independent of both.

### Decisions taken

- **D2.1** `ClauseGrammarSpec::head` is a `ClauseRow` (keyword `None`,
  `Once`), and keywordless `Once` rows are legal and entered in
  declaration order. Reason: the page's own DSL row carries
  `head {Body} -timing protected`, and `for`'s three fixture bodies need
  three timings that `&[ClauseSlot]` cannot carry.
- **D2.2** `ClauseSlot::handler` is `value_transfer::answers::HandlerMatch`;
  no second enum. Reason: the page says `HandlerPlan` carries "this page's
  `HandlerMatch`", and the type already exists.
- **D2.3** `clause_grammar` sits on `CommandSpec` and `SubCommand`.
  Reason: `dict for`, `dict map`, `dict update` and `array for` are
  subcommands.
- **D2.4** `template_plan` is not added in step 2; it is slice 5's.
  Reason: an answer with empty `script_regions` and `reads` would read as
  a fact; `substitutions_performed` as a projection of `option_effects` is
  what the page's four callers need until then.
- **D2.5** The derived-query layer is inherent methods on
  `ResolvedInvocation` plus `CommandRegistry::invocation(words, ctx)`;
  `CallWords` is `InvocationWords`; `ResolvedEffects` is `EffectFootprint`.
  Reason: the page calls `RegistryQueries` pseudocode and the type already
  answers three of the queries.
- **D2.6** `member_rows` is `DefinitionBodyGrammar::member_row` over one
  member statement; the analyser folds the body. Reason: a definer call's
  body is a script the compiler segments; the registry never re-segments.
- **D2.7** The SpecTcl and SslicTcl document grammars' members carry
  `MemberEffect::Configuration`. Reason: a DSL row declares data and opens
  no method frame; the vocabulary stays family-neutral.
- **D2.8** `cfg_lower.rs`'s `on ok` test reads `HandlerMatch::CompletionCode`
  plus the registry's completion-code word parse, not `is_default`.
  Reason: `try` has no default clause; `ok` is a value of the pattern
  word.
- **D2.9** The per-axis lint is a sibling gate (`registry-axes`) with its
  own marker carrying an expiry, the migration plan's axis names, and a
  ratchet. Reason: `value_transfers.rs` is the other lane's in-flight
  file; the page requires an expiry the existing waiver lacks.
- **D2.10** The two clause-keyword tables become derivations pinned equal
  to today's lists. Reason: the page's "read the slot's `noise` word"
  without changing a generated keyword list.
- **D2.11** The hook retirement table of CC2.13 is the step's residue
  list; `Set`, `DictWith` and `RegexPatternCapture` go on the ledger with
  their slices. Reason: the page's residue classes, applied to each
  handler's verified body.
- **D2.12** The per-axis lint scans the lexer's tokens (`rustc_lexer`,
  already an `xtask` dependency), not lines: a word in a comment, a doc
  string or a longer literal is never a site, a multi-line `matches!` or
  `match` is read whole, and a `#[cfg(test)]` attribute skips the item it
  guards (the value-transfer gate stops the whole file at the first one).
  Beside the plan's shapes it recognises an inline literal array searched
  with `.contains(` (the table shape written in place), and a `pub` string
  table counts as read. A waiver may wrap onto the standalone comment
  lines below it, and a waiver above an enclosing `match`, `matches!` or
  table covers its arms and entries. Reason: the vocabulary is
  7831 words, so a line-based scan's false positives (words in prose) would
  swamp the pins, and the plan's own shapes are multi-line in this tree.
- **D2.13** CC2.1's owner-manifest row names the sources that exist at
  CC2.1 — `registry.rs`, `definer.rs`, `traits.rs` (the two clause-keyword
  tables) and `special_vars.rs` (`SPECIAL_VARS`, the vocabulary's
  special-variable axis) — and CC2.2 swaps `traits.rs` for
  `clause_grammar.rs` and `clause_keywords` when they exist. The vocabulary
  keeps every word the registry declares, including the bare `mathop`
  operator commands (`+`, `eq`, `in`, …), so expression-operator sites are
  pinned rather than exempted; the report lists waivers and pins but not
  the vocabulary, which would churn with every spec change. `LANDED` holds
  step 1 and slices 1–4; a lane landing a step or slice bumps it.
  `owner_resolution.rs` learns the `registry-axes` dispatch name. Reason:
  `owner-resolution` rejects a manifest path or entry point that does not
  exist, and a hand-picked exemption list would be the per-name knowledge
  the gate exists to find.
- **D2.14** `OptionEffects` carries `option_end` beside the page's three
  fields, and `shifts` encodes a suppression as `(role, 0)` and a
  reservation as `(ArgRole::Option, n)`. A family's base applies to the axis
  values its options mention; with no option of any covering family present
  the first declared family's base decides; a `LastWins` option resets its
  family before it applies; two families covering one axis value used in
  one call make the answer unreadable. Reason: `lsearch`'s pattern sits at
  `option_end + 1` and a second walk to find it would be a second rule, and
  the page names the shifts' pairing but not its meaning.
- **D2.15** The walk has three doors: the plan's `option_effects` (unique
  prefixes), `option_effects_with` (the table's `PrefixMatching` — `regexp`
  is exact-only), and the crate-private `option_effects_over` the registry
  uses with its own profile-filtered option list, as the retired resolvers
  were given one. `resolve_available_option_prefix` (the prefix-enabled
  wrapper) is deleted; the `_with` form is the one resolver.
- **D2.16** `CaseListSpec::invocation` keeps its signature and classifies
  each resolved option by its declared effect inside the one walk that also
  finds the subject and the clause list, rather than taking an
  `&OptionEffects`. Reason: a second match mode is tclsh 8.5+'s `-exact
  option already found`, which the invocation abstains on today, and a
  `LastWins` answer keeps only the last mode; the positions need the walk
  anyway. `switch`'s `-integer` declares `Selects(Selection(Other))`, so
  `CaseListSpec::SWITCH.special_match_options` is empty (the field stays
  for packs). The Expect descriptor's `nocase_option` / `end_options_option`
  were unreachable — the clause-flag break precedes them — so its option rows
  gain no effect.
- **D2.17** `subst`'s family exclusion is three `Forbids` relations (each
  negated switch forbids the positive set, gated to 9.1, message tclsh
  9.1b0's `cannot combine positive and negative options`), not one
  `MutuallyExclusive` over two term sets. Reason: `Relation::terms` is one
  flat set, a `MutuallyExclusive` over all six switches would reject
  `-nocommands -novariables`, and a set-valued `OptionTerm` would need new
  analyser code in `relation_span`, which the plan rules out. A call mixing
  two negated switches with a positive one draws two W147s.
- **D2.18** The substitution projection
  (`option_effect::substitution_kinds`, `CommandSpec::substitutions_performed`)
  answers every kind when the option run stops before the reserved
  operands. Reason: `subst` rejects any non-option word before its operand,
  and a spelling-only caller (the analyser's W102, the taint gate) passes a
  computed switch's source text, which reads as a non-option word; the
  retired resolver answered every kind for both, and the generic leading-run
  rule alone would narrow `subst -nocommands $opt $x` to `{backslashes,
  variables}` — sound for the negated family but not for 9.1's positive one.
- **D2.19** `ResolvedInvocation::option_effects` and
  `substitutions_performed` take the `dialect` until CC2.8 fixes it in the
  resolution; `InvocationSemantics` gains `option_scope`
  (`OptionEffectScope`: the selected table's families, reservation, prefix
  policy and inherited release gate). `ResolvedInvocation::pattern_args`
  moves to CC2.8 with the other re-keyed queries: its escape hatch and
  static path need the spec's `pattern_arg_resolver` and `pattern_type`,
  which the resolution does not carry.
- **D2.20** CC2.6 lands before CC2.7 (the coordinator's order), so the
  studio's transitional state is explicit: `option_effect_families` is a
  schema field seeded presence-only with a transient `DraftOpaque` `GAPS`
  row, and an option row's `effect` is `Surface::Excluded` with a reason
  naming the step that drafts it. Reason: `schema_coverage` requires a schema
  field for every `CommandSpec` field, and a nested option-row key cannot
  have a `GAPS` row.
- **D2.21** The retired `substitution.rs` rows keep their `tp_` / `fp_`
  prefixes in `option_effect.rs` (`tp_no_switches_runs_every_substitution`,
  `fp_the_two_families_mixed_are_unreadable_and_every_kind_is_on`, …), and
  the five retired `case_list` rows load with a notice rather than as an
  unknown property. Reason: the value-transfers plan cites the `tp_*` /
  `fp_*` rows as its own preserves, and a pack that used the rows is told
  where the fact went.
- **D2.22** `ClausePlan::roles`, the flat projection the registry folds,
  holds each present introducing keyword and noise word as `Keyword` and each
  filled slot's role — except a `Value` slot (the unlisted default), a
  `LoopVarList` slot, a `conditional_binding` slot, a `Pattern` slot carrying
  a `HandlerMatch`, a fall-through body, and a `Group` row's words (its
  layout's, folded already). Reason: every existing `arg_indices_for_role`
  answer is preserved — `try`'s pattern and variable list, and `catch`'s
  result words (the page's `LoopVarList`), keep the flat meaning their own
  tables give them, since the flat `LoopVarList` is read as an iteration
  binding by SSA, semantic tokens and the refactors.
- **D2.23** The grammar is first in the documented order and *additive* in
  `arg_indices_for_role`: its flat roles join the resolver's (or the static
  table's), the repeated layouts and the option values.
  `arg_role_resolver_roles` stays the closed set the walk emits
  (`clause_grammar_roles_are_declared_capabilities`), and a grammar-carrying
  command's static table agrees with its walk
  (`clause_grammars_agree_with_their_static_role_tables`).
  `InvocationFacts::arg_roles` treats a grammar as the dynamic role source
  only where no static table exists — the four retired resolvers' place — so
  its roles, their order and `arg_roles_complete` (read by backend selection)
  are byte-identical. Reason: `catch`, `for`, `while`, `dict for`, `dict
  map` and `array for` keep the static tables the analyser's `CommandSig`
  reads, and `dict update`'s dictionary variable is `VarWrite` and `VarRead`
  at once, which one slot cannot say.
- **D2.24** Grammar shapes adapted from the plan: `catch`'s result words are
  `{LoopVarList optional}` slots (the page's role; `catch`'s `arg_roles` keep
  their `VarWrite`); `dict for` / `dict map` / `array for` bind one list, so
  their binder is a `{LoopVarList Value}` head rather than a `Group` row (they
  declare no `repeated_args` to cite) and `dict for` and `dict map` share one
  `DICT_LOOP_GRAMMAR`; `dict update` keeps `dict_last_arg_body` beside its
  grammar (D2.23) and its body is `Always` — it runs once whenever the call
  does — not the plan's `Selected`, which would read as conditional; `if`
  declares its tail the `default_clause` (`-final-only`); loops select
  `All`, chains `FirstMatch`.
- **D2.25** E004 moved in CC2.2, not CC2.9: `if`'s `clause_shape_check` went
  to `None`, so the analyser reads `CommandRegistry::clause_shape_defect` —
  the grammar's defect for a command carrying
  `Traits::STRUCTURALLY_CHECKED_ARITY`, else the escape hatch — and
  `control_arm_semantics` / `control_invocation_valid` read the grammar.
  Reason: E004 positions are a CC2.2 preserve, and without the trait gate
  every malformed loop and `try` would draw E004 beside E002/E003. CC2.9's
  `clause_plan().defect` read keeps the gate.
- **D2.26** A `Group` row needs one whole group; a partial last group is a
  recorded defect the walk passes, so the excluded trailing words still reach
  the tail. Reason: the retired `foreach` / `lmap` resolvers put the body on
  the last word from three words on and on nothing before that, including
  the off-by-one shapes (`foreach a b c body`).
- **D2.27** `ResolvedInvocation::clause_plan` takes the dialect (D2.19's
  reason) and abstains on a computed word where the walk compares a keyword,
  a noise word or the fall-through marker; a computed word in a positional
  slot is fine. Subcommand plans are reported in post-head coordinates
  (`ClausePlan::offset_by`).
- **D2.28** The `.tclspec` spelling beyond the plan's rows: a slot `{ROLE
  optional}` (the plan names no optional-slot spelling, and `catch` needs
  one), `-conditional` / `-pattern` on the row, `clause_grammar { … }
  -available V` for the grammar's releases, `selected` and `first-match` as
  the defaults a bare row or grammar reads as, and `ClauseRow::EMPTY_HEAD`
  for a grammar with no `head` row. The spellings are the registry's
  (`ClauseTiming::spelling`, …), shared by the loader, the draft and both
  renderers. The loader's `STRUCTURALLY_CHECKED_ARITY` advice is gone (the
  trait is an opt-in, and `try` and the loops carry grammars without it), and
  "needs a `head`" became "declares no clause". The new row words carry no
  vocabulary-version gate: the three rows the block had keep their meaning.
- **D2.29** Consumers that re-derived the role order by hand moved
  minimally in CC2.2 so nothing observable changed: `analyser/oo.rs`'s
  opaque-member test adds the clause walk; `package_resolver/reachability.rs`
  offers a command to its `if`-chain walk when its grammar selects
  `FirstMatch` (or it has a resolver, as before); `callback_inventory.rs`
  reports a grammar whose layout the words decide as a `dynamic-arg-role` row.
  The two clause-keyword tables became functions and their three consumers
  call them.
- **D2.30** The studio files `clause_grammar` under the existing "Clause
  grammars" and "Argument roles" clusters rather than a new "Structure"
  cluster, and the form shows the rows read-only (Q2's assumption).
- **D2.31** What a wrapper does to the member it wraps is
  `MemberSpec::wrapper_shift: Option<WrapperShift { receiver:
  Option<MemberReceiver>, visibility: Option<DeclaredMemberVisibility> }>`,
  and a wrapper's own effect is `Configuration`; the `.tclspec` spelling is
  `-shift {?-receiver R? ?-visibility V?}` in the `-effect` value's kebab
  spellings, and `-shift` on a non-wrapper is ignored with a notice.
  Reason: the page says a wrapper's side is "already `MemberKind::Wrapper`",
  but `Wrapper` does not say *which* shift — `self` moves the side,
  `private` and itcl's modifiers change the visibility — and the page's
  `MemberRow::receiver` is "after every wrapper shift".
- **D2.32** `member_row` specifics beyond the page: a nested wrapper's shift
  composes innermost-first and a wrapped member without a release set
  inherits the wrapper's (`private method` is 9.0+); the block form is a
  wrapper with exactly one argument (TclOO's own rule) and answers
  `InitScript { body_slot: 0, timing: AtDefinition }` (the page's reading of
  "a wrapper's bare block form"); the row abstains on a computed keyword, an
  unrecognised wrapped word, a recognised optional word the dialect lacks
  (the analyser skips that member), and a computed word at the optional
  position of a call long enough to hold it; a `StateDeclaration`'s name is
  its first `VarWrite` position unless it declares every argument; per-type
  and option state land on `Both` (itcl `common`, snit `typevariable` and
  `option`, the page's own examples) and a definition-time script on the
  type object; `MemberArity` counts positionally — tclsh 8.6 and 9.0:
  `proc p {{a 1} b}` → `wrong # args: should be "p ?a? b"` for one
  argument. Reason: each is the analyser's current reading, so CC2.11's port
  preserves behaviour.
- **D2.33** The sweep's agreement rules as built: a `Callable`'s slots are
  exactly the *first* `Name` / `ParamList` / `Body` positions (the loader's
  reading of an unwritten slot, so a shipped row's compact spelling is
  total); a `Forward`'s prefix slot is the target's `CommandName` (or a
  `CommandPrefix`); a `Relation` is a slot or, like itcl's `inherit`, a
  plain list of class references. Reason: `forward NAME TARGET ?arg …?`
  spreads its prefix over words, so re-typing the target `CommandPrefix`
  would turn a navigation reference into a call site with an appended-arity
  check the registry has no count for; itcl's `inherit` takes no slot
  operation words, so a `SlotSpec` would misread `-append` as an operation.
- **D2.34** Seeding names a shipped grammar when the grammar's *data*
  (`draft::definition_body_block`) equals it, not by `std::ptr::eq`, and the
  name list is `tcl_spectcl::SHIPPED_DEFINITION_BODIES`, which the loader's
  `definition_body NAME` reads too. Reason: the shipped grammars are
  `const`s, so every `&TCLOO_GRAMMAR` is its own promoted allocation and a
  pointer comparison is not reliable; by data the snit port (the inline
  spelling of `SNIT_GRAMMAR`) drafts as `snit`, so the port test needs no
  `unequal` entry for it.
- **D2.35** The loader grew three readers beyond `-effect`:
  `member_option KEYWORD POS VALUE -role R ?-visibility V? ?-dialects D |
  -available V?` (the README's spelling, `-visibility` in the Debug
  spelling as that row writes it), `-shift` (D2.31), and `family SpecTcl |
  SslicTcl`. Reason: every non-shipped grammar in the registry is a
  `SpecTcl` or `SslicTcl` document grammar, and `speclib`'s `command
  ?-override?` is an optional member word, so without them "the full block"
  could not round-trip; `every_shipped_grammar_spelt_inline_reloads_as_itself`
  holds the class families' inline spelling to the same bar.
- **D2.36** The studio's `definition_body` form picks a shipped grammar by
  name and shows an inline grammar's rows (with each effect) read-only —
  Q2's assumption, as D2.30 — and `semantic_operation` is a picker over the
  closed vocabulary keyed `KIND ?DETAIL?`. A form-level (`refine`)
  `semantic_operation` stays native: no shipped form sets one, and the
  refinement editor has no slot for it. The vocabulary and its `.tclspec`
  spelling live in the loader (`semantic_operations`,
  `semantic_operation_spelling`), which the studio renderer calls and
  `eval_loader`'s `semantic_operation_round_trips_through_the_renderer`
  reads back, since `tcl-spectcl` cannot depend on the studio.
- **D2.37** `ResolvedInvocation::arg_roles` answers
  `Option<Vec<(usize, ArgRole)>>`, not the plan's `Vec`: the function it
  re-keys, `arg_indices_for_role_words`, answers `None` for an expansion, a
  computed subcommand word, or a computed word where a resolver reads an
  option, and the page's second rule requires the abstention to travel with
  the answer. An empty call is read (`Some` of nothing), where the by-name
  function abstained on an empty ensemble call. Reason: a consumer moving
  onto the query must be able to tell "no roles" from "cannot tell".
- **D2.38** The queries read the resolution's own selection — its spec and
  the subcommand it selected under its release — where the by-name
  functions look the subcommand up release-blind; the two differ only for a
  subcommand the release lacks or a prefix whose uniqueness the release
  changes (`array d a s` at 8.6 selects `donesearch`, which a release-blind
  lookup finds ambiguous with 9.0's `default` —
  `a_prefix_the_release_makes_unique_selects_its_subcommand`), and
  `derived_queries_agree_with_the_by_name_answers` (every shipped command of
  every loadable dialect, literal words) and
  `arg_roles_agree_with_the_registry_role_answer_on_computed_words` (a
  corpus of computed words under 8.6 and 9.0) hold them equal everywhere
  else. Each rule exists once: the in-flight copy of the role, prefix,
  pattern and layout-proof bodies became the one body the by-name methods
  delegate to, parameterised by the caller's selection, and the role fold
  answers every wanted role in one pass (`arg_roles` asked the per-role
  core once per role, re-walking the grammar and the resolver 26 times).
  Reason: the page's third rule — every answer under the context's release
  — and one rule per axis.
- **D2.39** The resolution stores its query as `SurfaceQuery<'w>` (the
  words' lifetime) rather than adding a lifetime to `ResolvedInvocation`;
  the resolvers take `SurfaceQuery<'w>`, which no caller had to change
  (the query is covariant), and `resolve_invocation_in_context` borrows its
  context for `'w`. The option tables are the profile-less
  `option_specs(dialect)`; a profile-bound registry's `ProfileQueries`
  table can differ only for a profile whose query names no release but
  whose version ceiling excludes an option. Reason: the plan fixes the
  query as the key, and the growth (a query and two pointers) keeps the
  compiler's constrained-stack test (`the_source_walk_cap_fits_its_stack_budget`)
  green.
- **D2.40** `case_invocation` abstains when its reading depends on a
  computed word's value: it must hold whether each computed word is an
  operand or a dash word — in the option run always, and at a clause start
  only where the descriptor declares per-clause flags, since
  `inline_clauses` reads any dash word at a clause start as a flag attempt
  and `switch x $p {…}` would otherwise abstain. `arg_roles`' case-body
  gate keeps the by-name placeholder reading. Reason: the page's second
  rule for the new query, and parity for the re-keyed one (`case $x in {…}`
  keeps its body roles, as `arg_indices_for_role_words` gives them).
- **D2.41** `return_type` feeds a return-type hook a computed word spelt
  `$` — the hooks' own reading of a substituted source word
  (`switches_are_certain`) — and abstains under an expansion, so it answers
  what `return_type_for_call` answers over the source text (`lsearch $l
  $p` is `Int`); `frame_effect` parses the level with
  `FrameLevel::parse_for` under the query's Tcl release (`upvar 010 a b` is
  8 up at 8.6 and 10 at 9.0), where `FrameEffectSpec::resolve_for_version`
  parses release-blind, and a query naming no Tcl release (a vendor
  profile) answers `Dynamic` where the releases disagree. Reason: parity
  where a caller's answer exists, the context's release where the answer
  is new.
- **D2.42** `effect_footprint` stays one checkpoint as a `#[deprecated]`
  alias of `effects`; no caller outside the registry used it, and the
  registry's own tests call `effects`. Reason: the plan's one-checkpoint
  alias, for a concurrent lane mid-edit.
- **D2.43** The loader's `member_row` (CC2.5) was 103 lines, which the
  pedantic `too_many_lines` rejects. The coordinator's d89c7ea0 split its
  named flags into `member_named_flag` on the main branch while CC2.8 split
  the blank row out; after the rebase the CC2.8 split was dropped, so the
  main branch's fix is the only one. Reason: one fix per defect.
- **D2.44** The verbs name words by index, and the thunk, not the body,
  builds each fact from the call's own words. Reason: a body sees a
  computed word as the empty string, so only the host knows it is computed;
  reading the index there makes "abstain and widen on a dynamic word" the
  family's guarantee rather than each author's discipline, and the typed
  `PackTransition` has no variant for any other family.
- **D2.45** `namespace-variable NAME` states the alias `variable` states —
  a local under the name's tail bound to the current namespace's cell — and
  no verb builds a `NamespaceTransition`. Reason: the tree has no namespace
  transition for a variable declaration (`variable_state_transitions`
  states it as a `VariableCellAlias` with a `CurrentNamespace` target), and
  the ruling permits namespace facts without requiring them; a namespace
  verb is a later need's.
- **D2.46** `from-frame-effect` takes the README's pinned abstentions (a
  computed level word aborts the call; a computed pair member skips its
  pair) and widens for each, where the shipped `upvar_state_transitions`
  states the alias with an unknown subject. Reason: the page's resolver
  contract ("abstains … widens rather than producing a narrower fact") is
  the pack's, and CC2.12's negative witness needs a pack alias with a
  computed level word to bind nothing; the shipped resolver keeps its own
  answer. It is registry code (`alias_pairs_resolver`, one fn per level-word
  policy) because a resolver pointer cannot carry the frame effect.
- **D2.47** A `-native` resolver id is `SCOPE::state_transitions.resolver`
  (the `SCOPE::FIELD` rule the loader's other tables use, and the field
  `HookFamily::field` names), looked up in
  `STATE_TRANSITION_RESOLVER_NATIVE`, which ships empty like most families'
  tables; the `oo-class` port's ids were respelt to that form. A form's
  resolver may be `none` or `-native ID` only — forms bind no body and own no
  frame effect. Reason: one id rule; populating the table with the shipped
  resolvers is not this item's.
- **D2.48** The studio keeps `state_transitions` one `DraftOpaque` `GAPS`
  row; its spelling now lists the rows, and
  `a_state_transitions_resolver_body_survives_a_form_edit` proves the
  author's block — resolver body included — is carried forward verbatim.
  The plan's "the row shrinks to the resolver's `-native` form" is not
  taken: a draft holds no hook body for any family (every body survives by
  the pack store's carry-forward of its whole statement), so drafting the
  plain rows while the resolver stayed opaque would make a form edit write
  a block without the author's body. Reason: shrinking the row needs hook
  bodies in drafts, a studio change beyond this item — recorded as
  remaining.
- **D2.49** `callback-inventory --check` is unchanged: a pack resolver body
  is a hook body, not a Tcl callback position in a shipped spec, so it adds
  no inventory row or tier. The plan's "a new executable position tier" did
  not materialise.
- **D2.50** The value walk (`ClauseGrammarSpec::walk_words`, which is
  `ResolvedInvocation::clause_plan`'s) compares the fall-through marker
  exactly; the spelling walks keep the one-layer strip. Reason: an
  invocation word's literal is its Tcl *value*, so a value `{-}` (the source
  `"{-}"`) is a script, while a caller holding source spellings still reads
  the braced `{-}` as the marker. `arg_roles` keeps the spelling rule D2.38
  pins it to.
- **D2.51** A marker falls through to the next `Selected` clause only; a
  marker with none after it has no target and no new defect, and
  `ClausePlan::falls_through` names it from the flat projection, which
  leaves exactly such a body out. Reason: the walk linked a marker before
  `finally` to `finally`'s body, where Tcl raises "last non-finally clause
  must not have a body of `-`"; making that a `ClauseShapeError` would
  change the plan's defect vocabulary the page fixes, and every consumer
  already treats it by its own rule (the lowering keeps the handler's empty
  body, `try_control_invocation` rejects the chain).
- **D2.52** An abstaining walk goes on reading computed words as matching
  nothing and names the first one it compared
  (`ClauseAbstention { word, inert }`, `walk_words_or_abstain`,
  `ResolvedInvocation::clause_walk`); `clause_plan` stays the abstaining
  answer. Reason: the lowering must defer a `try` whose handler body is
  computed with the reason and after the steps the retired walk took
  (`try_dollar_var_handler_body_falls_through_to_barrier` pins "try with
  dynamic handler body"), and the inert plan says how far the literal
  clauses go without ever standing for the call.
- **D2.53** The lowering follows the grammar where the retired keyword walk
  differed (the behavioural deltas above), and with a defect lowers the
  clauses before it before deferring: `if` never lowers the clause the
  defect stopped in or the tail extra words follow, `try` lowers every
  clause that has its script word. Reason: the grammar is verified against
  C Tcl word for word (`if_.rs`), the walk's differences were
  mis-lowerings, and the pre-defect lowering keeps the nested procedures a
  deferred construct registered.
- **D2.54** E004 keeps `CommandRegistry::clause_shape_defect`. Reason: since
  CC2.2 (D2.25) it *is* the plan's defect behind the
  `STRUCTURALLY_CHECKED_ARITY` gate, then the escape hatch — the plan's
  `clause_plan().and_then(|p| p.defect)` at `:1708` would restate that gate
  in the analyser.
- **D2.55** The generic body walk takes each body's depths from its clause's
  timing where a plan places it — `Selected` conditional and control-flow,
  `PerIteration` and a `next` fixture control-flow, `Protected` conditional,
  `Always` and an `init` fixture neither — and from the traits otherwise.
  Reason: the plan's rule ("control-flow for a `PerIteration` or
  `LoopFixture` body, conditional for a `Selected` one") would bump `for`'s
  `start`, which runs once, and drop `if` bodies' control-flow depth; this
  mapping reproduces every depth the traits and `handle_for_command` gave
  the shipped commands.
- **D2.56** The analyser's hook resolution carries the invocation's plan
  (`ResolvedAnalyserHook::clause_plan`), and `handle_try_command` takes the
  plan, reading `analyse_selected_body` for `Protected` / `Selected` and
  `analyse_body` for the rest, in place of the traits. Reason: one
  resolution, as the traits were; `BRANCH_SELECTED_BODY` stays the generic
  walk's reading for a body no plan places.
- **D2.57** The diagram's `kind_handler` stays `on` / `trap`: the protocol's
  own two-word vocabulary, mapped from `HandlerMatch`. Reason: the editors
  read it (`TclLspActionsTest.kt`), and a respelt command's handlers are the
  same two kinds.
- **D2.58** `cfg_lower.rs`'s `on ok` test parses the pattern word with
  `completion::completion_code_selector` under the profile-less numeral
  grammar, as `executable_ir.rs`'s `try_handler_code` does. Reason: the
  plan's `CompletionCode::from_word` is that function; the IR carries no
  profile there.
- **D2.59** The six files' non-clause sites: migrated where a registry fact
  answers them (`[list namespace unknown H]` through
  `list_build_effective_command` and the `NamespaceUnknown` hook; nested
  `catch` through the `Catch` hook; the created name's `--` through the new
  `leading_option_run_is_terminated`; `switch`'s marker and `default`
  through `CaseListSpec`; the lambda's `list` through
  `BUILDS_COMMAND_PREFIX`), waived where a planned change retires them
  (six until step 2 for CC2.12, four until slice 8 for VT8.9, `args`
  irreducible), and pinned where none does: `structured.rs`'s `switch`
  option parser (`CommandRegistry::case_invocation` reads the same options,
  but moving the lowering onto it changes which shapes lower — unique
  prefixes, Tcl 8.5's two-word form, a repeated mode — a change this item's
  "none observable" excludes) and its `dict` routing (whether `dict map`
  collects has no descriptor: the value-transfer lane's iteration semantics
  are declared for `foreach` / `lmap` only; the value-transfer ledger names
  the same two sites `native_lowering` debt), and `handlers.rs`'s ensemble
  configuration walk (`NamespaceTransition::Ensemble` names only the
  namespace) and `$handle eval` (no descriptor models a child
  interpreter's command). Reason: a waiver names the change that retires
  its site, and these have none yet; `structured.rs` and `handlers.rs`
  therefore stay out of `CLEAN_FILES`, where the plan put all six.
- **D2.60** `try_dispatch_structured_hook` resolves the head in
  `structured_dispatch`, a frame of its own. Reason: the dispatcher stays on
  the stack while the lowerer recurses, and `depth_guard`'s
  `the_source_walk_cap_fits_its_stack_budget` had no headroom left — it
  failed on the unchanged tree once a test function was added beside it.
  The frame fell from 10,704 to 7,920 bytes a level (measured under gdb,
  dev profile); the deepest walk from about 1,576,000 to 1,401,208 bytes of
  the 1,572,864 budget.
- **D2.61** The value-transfer ratchet's `analyser/commands.rs` row (the
  orphaned-keyword table) is removed with its pin. Reason: its one site is
  gone; `value-transfers --check` requires the ledger row to match.
- **D2.62** The deprecated `ResolvedInvocation::effect_footprint` alias is
  removed, and the registry's `try_body_is_fallthrough` with it. Reason:
  D2.42's one checkpoint; no caller remained.
- **D2.63** `CallableRole` gains `Procedure` (`-role procedure`): a
  procedure in the definition's own namespace, reached by name and never
  dispatched, whose receiver is the side whose state its body sees. snit's
  `proc` states it (`-receiver type-object -role procedure`, in `definer.rs`
  and the `snit-type.tclspec` port, whose golden moves); itcl's `proc` keeps
  `-role method` on the type object. Reason: the page's vocabulary made snit's
  `proc` and `typemethod` one effect, so one `match` could not keep the
  analyser's reading of each (an ordinary proc; a class method) without the
  keyword, and the class-method reading is wrong for snit — snit 2.3.4
  (tcllib 2.0) on tclsh 8.6.18 and 9.0.4: `snit::type ::app::Dog { proc
  helper {a} {…} }` defines `::app::Dog::helper`, while `::app::Dog helper
  1` is a construction and an instance's `helper` an unknown subcommand, so
  a class-method reading would turn a construction into a typemethod call.
  itcl's `proc` keeps its reading because the providers' qualified
  `Class::proc` dispatch reads `class_methods` (`name_resolution.rs`'s itcl
  rows) and itcl is not installed here to oracle a change.
- **D2.64** The analyser reads a member statement's words structurally (a
  braced word, or one with no substitution, is its literal; anything else is
  computed — the boundary `has_substitution` draws for the walker's other
  static-word checks) and keeps recording members under their written
  spelling, a computed name included; such a member's visibility is the
  family's name rule read on the written spelling unless its option word or
  wrapper says otherwise. Reason: the row's `name` abstains on a computed
  word, and dropping `${m}` would move the outline; its `Public` answer for
  a nameless row would offer `${m}` to completion, where the analyser has
  always kept it unexported.
- **D2.65** An option accessor or mutator (snit's `oncget` /
  `onconfigure`) lands among the methods of its side though
  `MethodKind::from_effect` names no frame for it, and a definition-time
  script is a class-side method when the family runs member bodies in the
  defined entity's namespace (`MemberCurrentNamespace::DefinedEntity`: snit's
  `typeconstructor`) and the class's init script under `RuntimeReceiver`
  (`TclOO`'s `initialise`) — the same rule in the analyser's
  `member_landing` and the lowering's `MemberFrame::of_row`. Reason: each is
  the reading the walkers and the lowering had, now decided by registry data
  rather than the keyword; the namespace policy is what makes snit's script
  a nameable frame (`${type}::Snit_typeconstructor`) and `TclOO`'s not
  (`::oo::ObjN`).
- **D2.66** `MethodDef::is_self_method` is set for a class-side method a
  receiver-moving wrapper was crossed to reach (`self method`, `self
  classmethod`), read off the wrappers' shifts, not the receivers. Reason:
  `self classmethod`'s natural receiver is already the type object, and the
  existing test keeps it tagged.
- **D2.67** A forward, a state declaration, a superclass or mixin relation
  and flag-keyed accessors land only when their row is on the instances; a
  class-object spelling (`self forward`, `self variable`, `self mixin`)
  records nothing, and a type-object constructor or destructor is no member
  (`MethodKind::from_effect` answers none). Reason: `class_methods` holds
  the `classmethod`-kind entries the class-command dispatch reads, the
  class has no class-object variable or ancestry list, and those were the
  readings before; tclsh 8.6.18 and 9.0.4 reject `self constructor`.
- **D2.68** `analyser/oo.rs` keeps five pinned sites, unwaived: the three
  `property` flag words (`get`, `set`) until the 9.0 `property` accessor rows
  are `Callable` rows of their own, snit's type-body implicit `type`
  (`SNIT_TYPE_IMPLICIT`) until the grammar carries a type-body implicit
  list, and the unknown-proc walk's `default` switch arm. Reason: a waiver's
  expiry is a step or a slice, and no build-order step schedules either
  registry field, so the pin (which may only fall) is the honest record.
- **D2.69** The providers read the recorded member:
  `MethodDef::is_declared_by_keyword(word)` (a nameless member's synthetic
  `<keyword>` name) finds the constructor or destructor the keyword under the
  cursor declared, and its `kind` says which; `workspace_index.rs` spells
  the class side `MethodKind::ClassMethod.as_str()` and the visibilities
  `DeclaredMemberVisibility::{Public, Private}.as_str()`. Reason: the plan's
  "read `MethodDef::kind`" for a cursor on a keyword needs the member the
  keyword declared, and the synthetic name is the one place the analyser
  records it.
- **D2.70** The per-item analysis falls back to the full path
  (`PerItemFallback::PackDefiner`) when its shell walk meets a definer only
  the workspace's packs declare, and `fresh_full_analyse` carries the pack
  overlay. Reason: the plan's end-to-end test found that the server's
  memoised path never saw a pack-declared definer — `per_item_setup` reads
  the un-overlaid store, by design, while the per-body memo key carries no
  overlay — and that the fresh fallback dropped the overlay; a fallback is
  the one change that keeps the memo key and still gives the editor the
  class.
- **D2.71** The value-transfer ratchet's `analyser/oo.rs` row (the
  `definition_body` axis's keywords) is removed with its pin and
  `lowering/mod.rs`'s drops to 1 (the `namespace` body; the `self`
  comparison is gone). Reason: CC2.11 retired those sites, and
  `value-transfers --check` requires the ledger to match; the plan's
  "untouched" assumed they would survive.
- **D2.72** `VariableCellAliasTransition` gains `words: AliasWords {
  local, target }`, the post-head indices of the words spelling the local
  and the target variable (`AliasWords::same` where one word names both, as
  `global` / `variable` / `namespace-variable` do). Reason: a
  `TransitionSubject::Literal` keeps a value but not its place, and the
  analyser anchors the local's definition and the rename span of the cell
  at words; widening `TransitionSubject` itself would touch every family.
- **D2.73** `apply_state_transitions` takes `args` beside the plan's
  parameters, and reads a computed `upvar` target's source word for its
  array base alone (`::tk::FocusGrab($index)` links to `::tk::FocusGrab`,
  as the handler did); the plan's "`NamespaceTransition` (namespace
  variable declarations)" is the `CurrentNamespace` alias target, since the
  tree states a namespace variable as a `VariableCellAliasTransition`
  (`variable`, and a pack's `namespace-variable` verb, D2.45). The call is
  resolved over the source words by a new
  `model::resolve_invocation_words_in_context`, the structured twin of
  `resolve_invocation_in_context`. Reason: the shipped `upvar` resolver
  states an element target as unknown; the base is the analyser's variable
  grammar, not a value.
- **D2.74** The binder reads the roles over the words' spellings — the
  literal view `resolve_invocation_in_context` gives, then
  `ResolvedInvocation::arg_roles` — not over source facts. Reason: the
  structured `arg_roles` abstains when a computed word stands where an
  option a resolver reads could (`regexp $re $s m`), which would unbind
  names the editor has always bound; the literal view is what the by-name
  table answered. Both resolutions go through the walk's context (I4).
  A written name binds when it substitutes nothing or is an array element
  whose base is written literally (`incr hits($w)`); a loop var list binds
  when static, split as a list (the SSA harvester's and `script_binds`'
  reading, `dict update`'s single name included).
- **D2.75** Every `VarWrite` binding is `warn_if_unused = false`, `incr`'s
  included, and loop variables keep `true`. The plan's "`incr x` no longer
  draws W211" is moot in this tree: W211 is the SSA dead-store pass's and
  never read `VarDef::warn_if_unused` (`incr x` unread drew no W211 before
  this item either); only the flag moves.
- **D2.76** `package require`'s `exact` is "the option run is non-empty"
  (`option_effects().option_end > argument_offset`) — `require`'s one option
  is `-exact`, the reading `signature_scan`'s handler already gives — and
  its name and requirements are the `Name` / `Value` roles of
  `package_require_arg_roles` after the option run. `require` and
  `present` declare `PrefixMatching::Strict` (`PkgRequireCore`'s `strcmp`;
  tclsh: `package require -e Tcl` asks for a package `-e`). Reason: the
  plan's `EndsOptions`- and `Selects`-free rows; the presence check needs
  no option effect. A dash word naming no option leaves the layout
  unproven, so `package require -e 1.0` records nothing — the registry's
  option model has no "unknown dash word is an operand" policy.
- **D2.77** `interp create`'s registry resolver reads `Tcl_InterpObjCmd`'s
  option scan: `-safe` and `--` by unique prefix, `--` taking the next word
  as the path, a bad or ambiguous option or a second path creating nothing,
  and a computed word among live options leaving the path unknown (safety
  too, unless `-safe` came first). The nested `[interp create …]` is parsed
  as a list (`substitution_elements`) and resolved as a call
  (`substitution_call`), so no spelling is compared. Reason: the
  analyser's parse and the resolver disagreed in the error corners, and the
  analyser now reads the resolver; tclsh 8.4–9.1 agree on every row pinned.
- **D2.78** A pack's `special_var` rows live on the registry generation
  (`CommandRegistry::insert_special_var`, read through `special_vars()`,
  pack rows first and shadowing a shipped row of the same name); the free
  functions keep their signatures and read the shipped table. Reason: the
  free functions have no registry, their W210, `[info exists]` and taint
  consumers are the value-transfers lane's files (B1), and the plan's door
  is the registry. `a_pack_declared_special_variable_is_readable_at_startup`
  pins the answer the analyser's generation gives; the W210 switch is handed
  to that lane.
- **D2.79** The `set auto_path` / `lappend auto_path` arms are one
  `record_search_path_write`, reading `special_var_in_dialect("auto_path")`
  on the walk's generation and requiring `VarAccess::ReadWrite`; the
  append form is recorded by the binder from
  `VarElementsEffect::AppendsListElements`, so the `Lappend` arm has no
  handler left for CC2.13. Reason: the plan's "one read of
  `special_var("auto_path")`'s `VarAccess`"; the append trigger is a
  descriptor the registry already states.
- **D2.80** `special_var` is additive vocabulary, not gated (as step 2's
  other statements), classified Assistance (a dropped row degrades to a
  read-before-set, never to a stronger claim); `-dialects` narrows where
  the variable exists, like a command's `dialects` (it is not refused as
  `ambient_package`'s is: it places nothing in an environment); a row with
  no `-dialects` takes the pack's `default dialects`, else `ALL_TCL`; a
  pack golden prints `special_vars` only when a pack declares one. Reason:
  no shipped pack declares one, so no golden moves.
- **D2.81** `compile_service.rs`'s `RegistryTarget::Owned` boxes its
  registry. Reason: `CommandRegistry` grew by the `special_vars` field, and
  the owned variant crossed clippy's `large_enum_variant` threshold.
- **D2.82** `analyser/oo.rs` gains the `;` clippy asked for after CC2.11's
  `apply_relation_member` arm was reformatted into a block. Reason:
  pedantic clippy on the touched crate.
- **D2.83** A `namespace upvar` alias whose namespace or `otherVar` word is
  computed links its local to the cell spelt from the source word as
  written: `Analyser::alias_cell`'s `Target::Namespace` arm reads a
  `TransitionSubject::Literal` as its value and an `Unknown` subject as
  `args[argument_index]`, the way the `CallerSelectedFrame` arm already did
  (D2.73), resolves a namespace not starting `::` against the
  command-resolution namespace, and joins with `relative_to` (`::$ns::v`,
  `::mypkg::$v`); `fixed_frame` stays `false` and the rename span stays the
  `otherVar` word. Reason: the marker is a consumer contract, not an
  accident — `workspace_index.rs`'s `alias_cell_is_computed` reads it
  (`WorkspaceVariableAlias` from `VarDef::link_target`) and the rename
  tier refuses beside such an alias; the typed alternative (an "ambiguous
  cell" field on `VarDef`) would move the index and the analyser at once
  for no reader the marker does not already serve.
- **D2.84** `ResolvedClause::operands(role)` names every operand of a
  role in word order and `operand(role)` is its first; the generic tail's
  clause step binds each. The double binding of a list a static table also
  states (`dict for`, `dict map`, `array for`: the binder, then the clause
  step) is kept and documented, not removed. Reason: skipping it would
  resolve the flat roles a second time in the body walk to learn what the
  binder bound, and `define_var` is idempotent for one word.
- **D2.85** The binder's opt-out reads the command-level
  `Traits::CREATES_SCOPE_ALIAS` only; `SubCommand::creates_scope_alias` is
  not folded into `InvocationSemantics::traits`, and the reason is
  documented at the opt-out. Reason: `dict update`, `dict with` and `my
  variable` carry the subcommand flag for aliases no
  `VariableCellAliasTransition` states, so folding it would unbind their
  names; `namespace upvar`'s locals, bound by both the transition and the
  binder, are the same name at the same word.
- **D2.86** The studio schema's `pattern_arg_resolver` hint names a
  placeholder, `Some(my_pattern_resolver)`, as the other native-hook hints
  do (`my_resolver`, `my_prefix_resolver`). Reason: the brief asked for
  "the live resolver", and none exists — no shipped spec sets the escape
  hatch since CC2.6, and `patterns::option_selected_pattern_args`, the
  derivation `lsearch` now takes, is not a `PatternArgResolver`.
- **D3.1** `WorkspaceTrust` lives in `tcl_dialect::model::environment`
  beside `Provenance`; `Tier` is unchanged and the trust rides `PackFile`,
  `MergedPack`, `EvalOptions`, `EvalSnapshotKey` and the cache key.
  Reason: `Tier` is the discovery location and its ordering is
  precedence; trust is a second axis, and a snapshot evaluated under a
  different provenance registers differently (E-R2).
- **D3.2** The wire is `initializationOptions.workspaceTrust` and the
  setting `tclLsp.workspaceTrust`; VS Code sends `workspace.isTrusted`;
  an absent value is trusted. Reason: the page's "a client that does not
  report it is treated as trusted".
- **D3.3** Dormancy is decided from `MergedPack::provenance()` in two
  places that read one fact: `pack::load_sources` (the notice) and
  `hooks::plan_for` (no slot). Reason: the host never learns trust; an
  unallocated slot is exactly the declared-but-unbound abstention.
- **D3.4** `DeclaredCommand` gains `traits` and `side_effects`;
  `DocumentCommandSurface` gains `traits` and `side_effects` doors;
  `SubCommand::pure` is moot for a stub. Reason: the page's field map.
- **D3.5** The trust rides the load, not `PackFile`:
  `DiscoveryOptions::workspace_trust` is the one input, the load doors
  take it (`bundled::load_discovered_in(store, files, trust)`,
  `pack::load_under`, `pack::load_in_memory_under`), each file reads it
  through `Tier::trust_under`, and `MergedPack::trust` records the winning
  tier's. `load`, `load_in_memory`, `load_discovered` and `load_embedded`
  keep their signatures and load trusted — the CLI's and MCP's reading, an
  author's own files — so neither caller changed. Reason: `PackFile` is
  built by struct literal in 31 places across 20 files, three of them the
  value-transfers lane's (`tests/value_transfers.rs`,
  `value_transfer_witnesses.rs`, `value_transfer_parity.rs`; B1) and edited
  mid-slice, so a new field would break that lane's files; and the trust is
  one state per workspace window, never per file.
- **D3.6** `PackEnvironmentTier::Workspace(WorkspaceTrust)` carries the
  trust: `of(tier, trust)` builds the value, `provenance()` keeps its
  signature, and `label()` names the class a refusal prints ("untrusted
  workspace"; the discovery tier's own label otherwise, so the studio
  override's messages are unchanged). Reason: the plan's
  `provenance(self, trust)` would thread a second parameter through
  `to_definition`, `to_extension`, `reserved_name_for` and
  `to_dynamic_family` too, while the tier value is already "the trust
  class" its module doc says it is.
- **D3.7** The trust is normalised to what the tier reads
  (`Tier::trust_under`): a bundled, user or studio-override snapshot keys
  the same under either state and a workspace one splits. The on-disk
  entry key mixes the trust byte for every tier, so every entry key moved
  once and the cache rebuilds on the first load (disposable by contract,
  unobservable); `FORMAT` is not bumped, since no layout changed — the
  plan's `cache::VERSION` names no constant in the tree. The pack-set key
  mixes each file's trust, so granting or withdrawing trust re-installs.
- **D3.8** `provenance_violation(pack, tier)` answers "as if untrusted",
  so for the workspace tier it names the untrusted workspace; the load's
  own gate names the class it refused under. `register_pack_environments`
  and `register_environments` take the trust beside the tier. Reason: the
  step's expected delta — an `-override` refused "with the provenance
  named" — and CC3.2 changes the violation's signature anyway.
- **D3.9** The redesign's O9 is narrowed at CC3.1 and closed at CC3.3, not
  closed at CC3.1 as the plan's Docs line says. Reason: O9's resolution is
  "plumbing the LSP client's Workspace Trust state to `discovery`", and
  the client half — the server reading `initializationOptions` and the VS
  Code client sending it — is CC3.3's.
- **D3.10** The client's trust wire is `initializationOptions.workspaceTrust`
  and a **top-level** `workspaceTrust` in a `didChangeConfiguration` push —
  not `tclLsp.workspaceTrust`, which the plan and D3.2 name — and the
  server never reads the state from a pulled or synchronised `tclLsp`
  section. Reason: the VS Code client sets `synchronize.configurationSection:
  "tclLsp"`, so `vscode-languageclient` pushes the whole resolved `tclLsp`
  section on every change, and a `workspace/configuration` pull answers it,
  from every settings layer — the untrusted workspace's own
  `.vscode/settings.json` among them, and an undeclared key is not a
  restricted setting — so a nested key would let an untrusted workspace
  grant itself trust. No sync ever produces a top-level key besides
  `tclLsp`; only extension code writes one, from `workspace.isTrusted`.
  Refines D3.2; the negative is pinned by
  `workspace_trust_comes_from_the_client_and_never_from_a_setting`.
- **D3.11** `HookDecl::line` is the declaring row's line, and for a
  `state_transitions` resolver the `resolver` row's, not the block's; an
  `evaluate` hook carries its `evaluate` statement's line from
  `semantics.rs`'s `Implementation` through `declaration()` into `rebind`,
  so a later `facts` or `option -evaluate` statement that rebinds the hook
  does not move it. Reason: the notice goes where the author wrote the
  body; a caller-side `stmt.line` would name whichever statement rebound
  it last. The change to the value-transfers lane's slice-4 file (B1) is
  three hunks behind one tuple, and no `loader.rs` caller changed shape.
- **D3.12** A body is dormant exactly when its pack's provenance is
  `Provenance::WorkspaceUntrusted` (`hooks::hook_bodies_run`), not whenever
  `Provenance::is_untrusted` answers: a Spec Studio override is untrusted
  for registration (E-R2) but runs its bodies. Only `HookSource::Body`
  hooks are gated; a `-native ID` runs shipped code the pack only names, and
  a derivation is the loader's own. Reason: the ruling gates the one
  surface whose inputs the workspace chooses — a body run per query on the
  analysed document's words, from a folder the editor has not trusted; the
  studio override is the author's own live edit, and its answering is what
  the studio shows. One list, `hooks::dormant_hooks`, over the `bodies()`
  iterator `programs_of` also reads, feeds both the notice and
  `HookPlan::dormant` (D3.3's one fact).
- **D3.13** The golden `hooks` digest reads the hooks with their lines
  zeroed (`golden::positionless`). Reason:
  `every_shipped_pack_upgrades_to_2_0_and_loads_identically` compares a 2.0
  rewrite modulo positions (the command's `line` is its wildcard), and a
  line inside the digest broke that for every pack with a hook; the digest
  says which hooks a command declares, the `line` column says where.
- **D3.14** The VS Code client's `initializationOptions` is a function, so
  each start — the first and every restart — reads `workspace.isTrusted`
  afresh, and the grant listener (`registerWorkspaceTrustGrant`, in
  `clientCore.ts` for both hosts) is registered before `client.start()`.
  Reason: the checklist's race — a grant during start-up — is covered by
  the client holding a notification until its connection is up, and VS
  Code never withdraws trust within a session (withdrawing reloads the
  window, which restarts the server with the new state).
- **D3.15** `-mutator` lands as the read-modify-write shape `lset` and
  `lappend` state — `Traits::READS_BEFORE_WRITE` beside a `SideEffect` that
  reads and writes `SideEffectTarget::Variable` — not the plan's write-only
  `SideEffect`. Reason: a write alone is what a `var` role already states
  (lowering's def), so the flag would change nothing; the store before a
  mutator survives only because the command reads it, and lowering's
  `reads_own_defs` reads `READS_BEFORE_WRITE` off the invocation's traits.
- **D3.16** `DocumentCommandSurface::side_effects` returns
  `Option<Cow<'a, [SideEffect]>>`, not `Option<&[SideEffect]>`, and
  `invocation_traits(name, args, query)` joins `traits`. Reason: the floor
  unions a redeclared shipped command's effects beneath the declaration's,
  an owned slice when both are non-empty, and a surface is built per
  analysis, so the per-generation leak `union_leaked` uses has no place
  here; `invocation_traits` is what lowering's read-before-write asks, and
  a declaration, having no subcommands, answers it with `traits`.
- **D3.17** Nearest-wins holds under the floor: for a declared name
  `traits` is the declaration's ∪ `SecurityFloor::security_traits` of the
  shipped command it redeclares, and `side_effects` the declaration's ∪ the
  shipped command's; roles, prefixes and every other trait are the
  declaration's alone. Reason: invariant I6, applied exactly as a pack
  override meets it (`SecurityFloor::apply`'s set-valued union), so a stub
  can no more take `exec`'s `UNSAFE` away than a pack can.
- **D3.18** A declared command's side effects are classified from its
  declaration (`classify_side_effects_in` → `classify_declared`) in the
  catalogue's order — eval-like barrier traits, `PURE` (its stated effects
  read-only), the stated effects — and a declaration stating none is
  `fallback_unknown_write`, the undeclared answer. The interprocedural scan
  counts a declared command as known (`has_unknown_calls` stays false).
  Reason: the ruling makes a stub a workspace-authored fact; the preserve —
  a flagless stub is as conservative as before — holds because the
  classification, not the unknown-call bit, decides `local_pure`, and the
  compiler only reports `has_unknown_calls` (the Explorer and the db
  serialise it).
- **D3.19** The `-pure` test is `a_pure_stub_keeps_its_caller_pure` — O126
  on `set a [label abc]`, `label` a procedure returning `[mypure $x]` — not
  `a_pure_stub_in_statement_position_is_reported_unused`. Reason: the tree
  has no pure-statement finding even for a catalogued pure command, and the
  direct O108 / O126 gate on `set a [mypure …]` is `optimiser/elimination.rs`'s
  `classify_side_effects` over the catalogue — the value-transfers lane's
  in-flight file (VT8.5), left untouched, so the direct gate is residue
  (probed: the direct call keeps its `set` with or without `-pure`).
- **D3.20** The `-barrier` test is `a_barrier_stub_fences_its_scope_from_renaming`
  (the minifier), not W210 silence. Reason: W210 is not silenced by a
  catalogued `CREATES_DYNAMIC_BARRIER` command either (`vwait`, `uplevel 1
  $s`; probed — only `eval $s` silences it, through `EVALUATES_CODE`), so
  the plan's pair would have pinned a behaviour no catalogued command has.
  `find_rename_barriers` is `CREATES_DYNAMIC_BARRIER`'s one consumer in the
  tree.
- **D3.21** The `-loop` test is `a_loop_stub_is_checked_as_a_loop` (W240 /
  W241), not `a_loop_stub_bumps_the_loop_body_depth`. Reason: no
  loop-body-depth counter exists; `HAS_LOOP_BODY`'s analyser consumer is
  `bounds_checks::loop_shape`, the plan's "W240 family". A declared name
  answers the loop question from its declaration alone, so a stub
  redeclaring `while` without `-loop` is not a loop.
- **D3.22** `ssa.rs` (`registry_barrier_defs`, `uses_in_barrier`) and
  `memory_ssa.rs` (`is_clobber`) are not wired: deferred residue, and the
  reason that defers it is the boundary — threading a surface into
  `build_ssa_with_config` and `build_memory_ssa` runs through
  `compilation_unit.rs`, the value-transfers lane's file, held clean and
  edited in its slice 8 (B1). The two other reasons first given here were
  corrected by the step 3 review. `registry_barrier_defs` takes its
  `VarWrite` / `LoopVarList` roles from the catalogue's
  `arg_indices_for_role`, so the gap is not only a name the catalogue
  lacks: a redeclared catalogued name is walked with the catalogue's roles,
  and a stub-declared name gets no def at all — a stub declaring `{v:var
  script:body}` lowers to a `Statement::Barrier` (its `body` word is a
  same-invocation executable, `lower_default`), so `run_with x { puts hi }`
  then `puts $x` draws a false W210, confirmed by a probe at the review
  (without the `body` word the call lowers as a plain call with its
  declared def, and there is no W210); reported for filing, not fixed
  here. And `is_clobber` already answers "clobbers" for a name the
  catalogue lacks, so reading the surface need not lose conservatism.
  `uses_in_barrier` reads a *subcommand's* `creates_scope_alias`, which a
  declaration cannot state. The intended wiring once `compilation_unit.rs`
  is free: the barrier-def walk and its scope-alias discriminator read the
  document's surface, and memory SSA clobbers for a declared name unless
  its declaration states `PURE` — the reading `classify_declared` gives a
  declaration that states nothing (`fallback_unknown_write`). The
  coordinator schedules it after the value-transfers slice 8 merge.
- **D3.23** The minifier builds the document's surface from
  `analysis.stub_commands` (`build_declared_surface`), reads a head's
  command-level traits off it, and skips subcommand observability for a
  declared name. The loop-exit set (`TERMINATES_BLOCK`) and the
  interprocedural `INVOKES_USER_PROC` head stay on the catalogue. Reason:
  those traits are ones no flag states, so nearest-wins could only drop
  them from a redeclared `return` or `call` — a W241 false positive or a
  lost call edge — where the catalogue's reading is the conservative one.
- **D3.24** CC3.2 keeps `provenance_violation(pack: &Pack, tier: Tier)` and
  the private `provenance_violation_in(registrations, tier:
  PackEnvironmentTier)` exactly as CC3.1 left them, rather than
  reshaping either to take a bare `Provenance` as the plan's prose reads.
  The deleted shim's one call site, in `replay()`, now calls
  `tcl_registry::model::untrusted(tier.provenance())` directly. Reason:
  `provenance_violation_in` still needs the full `PackEnvironmentTier` for
  `environment_block::reserved_name_for` and for the class name its
  message prints — `PackEnvironmentTier::label()` and
  `tcl_registry::model::provenance_label` spell `StudioOverride`
  differently ("Spec Studio override" vs "studio override"), and the
  pinned `an_untrusted_pack_declaring_dialect_axes_fails_with_the_provenance_error`
  depends on the former; a `Provenance`-typed `provenance_violation` would
  also ripple into `tcl-spec-studio/src/store.rs`'s two callers, which the
  item's own file list omits.
- **D3.25** CC3.4's `spectcl_check` always evaluates the pack as trusted,
  never under the caller's `tier`/`trust`; the two parameters instead
  drive previews layered on that one snapshot (`untrusted_tier_refusal`
  from `tier` alone, exactly as before; `dormant_hooks` from the pair's
  provenance). Reason: E-R2 is transactional — evaluating under an
  actually-untrusted pair with a real violation would discard the whole
  pack, emptying `pack.commands` and regressing the tool's per-command
  report, which is what an authoring tool checks a pack *for*.
- **D3.26** CC3.4 adds a `provenance` output field
  (`tcl_registry::model::provenance_label` of
  `PackEnvironmentTier::of(tier, trust).provenance()`), reading the
  plan's "the provenance verdict computed for that pair" as a reported
  fact rather than only `dormant_hooks`'s internal input.
- **D3.27** `spectcl_check`'s `dormant_hooks` is empty when the `(tier,
  trust)` pair's provenance is itself untrusted and `untrusted_tier_refusal`
  is set: that install refuses the whole pack (E-R2 is transactional), and
  a pack that does not load holds nothing dormant — one report, one
  install. Chosen over stating in the tool description that
  `dormant_hooks` assumes the pack loads, which would describe an install
  that cannot happen. The refusal field stays `tier`'s preview of an
  untrusted install, so at a trusted pair it is still reported while the
  pack loads and its bodies run.
- **D3.28** An E-R2 message has two moods from one builder (`Verdict` in
  `loader/eval.rs`): the load's refusal keeps its wording byte-for-byte
  ("…, but this pack loads from the {class} tier; … so the pack is not
  loaded (design E-R2)"), and the preview `provenance_violation` answers
  for authoring tools says "…; an untrusted pack may not …, so the pack
  would not be loaded from the {class} tier (design E-R2)" — true at the
  default `workspace` / `trusted` preview, where the pack does load.
- **D3.29** The studio class is spelt "Spec Studio override" wherever a
  report names it: `provenance_label(Provenance::StudioOverride)` takes
  `Tier::StudioOverride.label()`'s spelling (it fed two
  environment-registration messages and `spectcl_check`'s `provenance`,
  none pinned). The `tier` argument keeps the token `studio-override`,
  since an argument cannot carry the label's prose; `spectcl.rs` owns it as
  `STUDIO_OVERRIDE_ARGUMENT`, and it needs no waiver.
- **D3.30** The trust-grant notification is guarded with a `.catch`, not a
  `client.isRunning()` test: the handler is registered before
  `client.start()`, and a grant during start-up is queued by the client
  until the connection is up — an `isRunning()` guard would drop exactly
  that grant. A stopped client's rejection is dropped deliberately: the
  next start's `initializationOptions` read the state afresh.
- **D4.1** `alias_of` is a `CommandSpec` field only. **D4.2** The stamp
  rule runs in `pack::load_sources` on the loaded command. **D4.3**
  `PackFactStamp::content_hash` is the `u64` xxh3 the snapshot key
  interns (the page's `[u8; 32]` would be a second hashing rule, which the
  page forbids for the lockfile's sake). **D4.4** A spec's pack origin is
  a registry side table, not a `CommandSpec` field (no studio surface, no
  `GAPS` row). **D4.5** `SiteClaim`, `PackFactStamp`, `IdentityKind` and
  the manifest live in `tcl-runtime-api`; `FunctionAsm` carries
  `site_claims`.
- **D4.6** The plan's "the four surfaces" (`render_spectcl.rs`,
  `schema.rs`, `help.rs`, `draft.rs`, over `coverage.rs`'s completeness
  gate) is what a new `CommandSpec` field must reach to be authorable and
  documented — but not the whole of what the tree enforces. Two more
  surfaces, each with its own completeness test, are equally mandatory
  and were not named: `examples/fields_core.rs` (`every_field_has_a_valid_example`
  — every schema field needs a worked snippet; `alias_of`'s uses a pack
  command, `vendor::unpack`, since no *shipped* command can ever carry a
  pack-only field, unlike every neighbouring example that shows a real
  one) and `relations.rs` (`every_field_is_clustered_or_declared_standalone`
  — `alias_of` is `STANDALONE`, since nothing else interacts with it
  until CC4.2 reads it). `alias_of`'s `schema.rs` category is `IDENTITY`,
  not `deprecated_replacement`'s `DEPRECATION`, matching the page's own
  three-contract framing (description, identity, backing) rather than the
  nearest existing field of the same shape.
- **D4.7** The rule's "shipped" registry is `stamps::shipped()` —
  `environment::lenient_store()`, the permissive all-Tcl view the E-R2 gate
  already treats as "a compiled command". A load is dialect-free, and the
  identity a stamp claims is the builtin's name in every release that has
  it; a release without the target never binds to it at run time, so the
  specialised site is refused there by the ordinary binding check.
- **D4.8** Rule 1 covers every site a stamp can sit: the command, each
  subcommand, and each `command_forms` entry (`CommandForm` carries
  `codegen_hook` and `semantic_operation`; `SubSubCommand` carries none). A
  site's stamp is admitted only as the target's own at the same site — the
  command itself, its subcommand of the same name, its form of the same
  name — which is where codegen's resolution (`resolve_call`: form over
  subcommand over command) would read it. The plan named the command-level
  fields only; a subcommand or form stamp would otherwise pass both rules.
  `semantic_operation` is policed for `Intrinsic(…)` only; `Invoke` and
  `StructuredLowering(…)` are the floor's business (rule 4, step 6).
- **D4.9** A refusal reads "`STAMP` refused for SITE: WHY; REMEDY". WHY is
  the tier gate with the provenance label (the plan's wording, with "an"
  before "untrusted workspace"), or rule 1's missing, unknown, or foreign
  `alias_of`. REMEDY names the shipped carrier — the first command by name
  that carries the stamp at the same site, else anywhere in its spec — as
  "the stamp would have to sit on `alias_of T`"; when the command already
  names that target (the tier gate refused it anyway) it reads "only a
  bundled pack may carry `alias_of T`'s own stamp", and when nothing
  shipped carries the stamp, it says so. The notice sits on the command's
  row (the plan's choice); a stamp row carries no line of its own.
- **D4.10** Stripping is memoised on the original spec's address and the
  exact stamps dropped. Every spec a `PackCommand` holds is `&'static` —
  leaked by the loader or compiled in, never freed — so its address is its
  identity for the process, and the drops alone decide the clone. The
  loader's snapshot cache returns an unchanged pack's original specs on
  every reload, so the plan's per-reload leak risk is closed rather than
  noted: a reload, or a Studio preview rebuilt per query, reuses the one
  clone. The exceptions are the packs the snapshot cache does not hold — a
  target-dependent pack, a pack with `include` rows — which the loader
  re-evaluates, and re-leaks, on every load anyway; their clones follow
  the same count.
- **D4.11** The rule reaches the two authoring previews, because the
  loader notice it replaces reached them. The Spec Studio assembles its
  installed set itself (`merged()`), not through `load_sources`, so it
  applies the rule there — without it, the install's assertion would fire
  on any authored stamp, and the Test tab would specialise what a real
  load drops — while the document and its drafts keep the rows (an editor
  must not delete what its author wrote); `PackStore::stamp_refusals`
  reports them in the store view. `spectcl_check` reports `stamp_refusals`
  for the pair's install beside `dormant_hooks`, empty where that install
  refuses the pack (D3.27).
- **D4.12** The Studio's `relations.rs` moves `alias_of` from `STANDALONE`
  (D4.6: nothing read it) into a new "Builtin identity" cluster with
  `codegen_hook`, `inline_codegen_hook` and `semantic_operation`, which the
  rule now reads together. The field's worked example takes `lassign`'s
  word order, the list first (`vendor::unpack $items first second`); the
  plan's `vendor::unpack {a b} $l` puts the names first, as `foreach` does.
- **D4.13** Codegen records the `alias_of` target's identity only where
  the target's own spec carries the stamp being specialised at the same site
  (`ResolvedCall::stamp_identity`), not the plan's bare
  `alias_of.unwrap_or(name)`. An `-override` of a shipped command keeps the
  shipped `codegen_hook` / `inline_codegen_hook` through the security floor
  whatever `alias_of` it declares — `command lassign -override { alias_of
  lsort }` installs a `lassign` spec carrying `CodegenHookId::Lassign` and
  `alias_of lsort` — and recording `lsort` there would let a runtime
  `interp alias {} lassign {} lsort` admit `lassign`'s specialised code for
  `lsort`. The condition is the loader's rule 1 asked again at the point of
  use, so the "same stamp, same site" predicate moved into `tcl-registry`
  (`codegen_stamp.rs`) as its one owner; `tcl_spectcl::stamps` re-exports
  `CodegenStamp` and `StampSite` and keeps what is the loader's (the tier
  gate, the refusals, the stripping). Lowering-hook sites keep the spec's
  own name (a lowering hook is not a stamp the rule polices, so an
  `alias_of` claim cannot vouch for it) and so do const-fold sites (rung 1,
  CC4.4's `PackFacts`).
- **D4.14** The `codegen_stamps.rs` harness: the plan's `vendor::unpack {a
  b} $l` is written `vendor::unpack $l a b`, `lassign`'s word order. The
  VM's compile service is the default one wrapped to count plain-dispatch
  compiles: the specialised module and a plain dispatch through the alias
  both answer `1 2`, so the count, not the result, shows admission, and
  the setup's `namespace eval` needs a service anyway. The setup creates
  namespace `vendor` before `interp alias {} vendor::unpack {} lassign`:
  Tcl 8.4 to 9.1 create it for a qualified alias themselves (`namespace
  exists vendor` is 1, `namespace which vendor::unpack` answers
  `::vendor::unpack`), and the VM does not — its `namespace which` answers
  empty, and admission cannot resolve the name. That divergence, and the
  VM's missing two-argument `interp alias srcPath srcCmd` describe form, are
  reported for filing, not fixed here (`tcl-vm`'s `interp_alias_create`).
- **D4.15** `SiteClaim` lands with the two variants step 4 builds,
  `PackFacts` and `BuiltinAlias`. No `Generic`: the page's own text has
  rung 0 record nothing, a unit with no claims is exactly the rung-0 case,
  and a variant nothing constructs is dead code (CONTRIBUTING.md, *Dead
  code*). No `IdentityKind` yet: D4.5 places it in `tcl-runtime-api`, but
  its one reader is rung 4's `ShippedImplementation`, so it lands with that
  variant in step 7. The page's code block now marks rungs 1 and 2 built,
  rungs 3 and 4 proposed, and drops the rung-0 variant.
- **D4.16** The stamp's fields and where each comes from.
  `overlay_generation` is the compiling registry's own
  (`CommandRegistry::overlay_generation`: the pack set's key the install
  was built with, `0` for a registry built without one) and
  `evaluator_revision` the compiling thread's
  `pack_hooks::evaluator_generation`, not the plan's "`AnalysisContext` the
  codegen context carries": `CodegenCtx` carries a registry and no
  analysis context, and `AnalysisContextKey::for_module` reads the same two
  values from the same two places, so a claim and the analysis key agree.
  `vocabulary_version` is the loader's `VOCABULARY_VERSION` — bumped when
  a word's meaning changes, and interned by the snapshot key — not the
  `speclib` version word the page names, which is part of the pack's text
  and so already inside its content hash. `content_hash` is per command
  (`PackCommand::content_hash`): the xxh3 of the declaring file's bytes
  before the byte-order mark is stripped, which is `eval_snapshot_key`'s
  value, folded — one xxh3 over the little-endian hashes, in inclusion
  order — with each `include`d fragment's when the file included any, so
  an edit to an included file moves it too. `install::pack_origin` builds
  the origin for the installer and for `PackSet::fact_stamps` alike, and
  `tcl_compiler::site_claims::pack_fact_stamp` is the one stamp
  construction, so a site's stamp and a VM's held facts agree by
  construction.
- **D4.17** The side table is keyed by the installed spec's address — the
  security-floor-merged spec the installer leaks once — as the plan's
  "keyed by spec pointer" and D4.4 have it. `project_for_profile` (a
  compile service's per-profile view of an embedder's registry) now
  carries the table and the overlay generation, which it used to drop:
  without the table a site compiled on the projection would claim nothing
  though its code rests on the pack exactly as on the base, and without
  the overlay generation it would stamp `0`, which no held fact matches.
  An `AnalysisContextKey` built on such a view now carries the base's
  overlay generation where it carried `None`.
- **D4.18** Which sites claim. Rung 1: every constant fold
  (`const_subst.rs`'s `fold_at_depth`, both returns, nested folds
  included) whose resolved spec has a pack origin. Rung 2: a stamped
  binding (`stamped_binding`, from `registry_codegen_hook` and
  `inline_codegen_resolution`) whose recorded identity is not the resolved
  spec's own name — exactly where `stamp_identity` answered the `alias_of`
  target. A pack override's codegen hook, kept by the floor and recorded
  under its own name, claims nothing: the emitted code is the shipped
  builtin's, and the binding check attests the builtin. Lowering-hook sites
  claim nothing (D4.13's reason: not a stamp the rule polices, recorded
  under the spec's own name). `trusted_inline_codegen_binding`, whose
  callers name shipped commands, declines a site that would carry a claim
  rather than drop it.
- **D4.19** The check sits in `function_command_bindings_match`, the one
  predicate every admission path already asks — `run_module`, procedure
  entry (`ensure_proc_traced`), `FunctionHandle`, and the stale-frame
  redispatch at a source-command boundary — so a claim is checked wherever
  a binding is, and a refusal takes a failed binding's path: plain
  dispatch when the unit carries source and a service is installed, an
  admission error otherwise. It is whole-stamp membership in the held list,
  the plan's five-field equality with no field the VM may skip.
  `set_pack_facts` advances the compilation-deopt epoch
  (`bump_trace_deopt_epoch`, not `bump_cmd_epoch`, which would also clear
  the intrinsic guard table for no command change), so a unit admitted
  under the old facts is checked again at its next entry or boundary.
- **D4.20** No `rust/tcl-lsp-server` wiring. The plan's "the optimise path
  sets the facts from the published pack set" has no VM to set them on:
  the server compiles no bytecode, and its optimise path is the
  source-to-source optimiser. No production VM compiles against a pack —
  the `tclvm` engine compiles through `build_default` and the debugger
  through the profile's shared generation, while `tcl compile` and the
  Explorer compile against the discovered set and run nothing — so
  `PackSet::fact_stamps` is the embedder's seam, exercised by the
  witnesses. The design page's sentence that the server's optimise path
  specialises an admitted stamp (carried from before step 4) is corrected.
- **D4.21** Beyond the plan's two tests, `codegen_stamps.rs` gains a rung-1
  witness on its own — a pack's `const_fold` constant admitted only under
  the pack's facts — because the plan's changed-pack test checks the
  facts on a rung-2 site. A pack's `const_fold -native` must name the
  command's own shipped folder, so the witness overrides a builtin
  (`llength`), and it scopes the override with `dialects tcl9.0`: a
  profile's lookup (`get_for_surface`, and `resolve_call` under the
  registry's own surface query) prefers a scoped spec to a catch-all in
  `best_visible`, and a pack command declares no surface unless it says
  `dialects`, so an unscoped `command llength -override { … }` replaces
  `llength` for `CommandRegistry::get` but loses to the shipped
  `ALL_TCL_AND_IRULES` row for every profile-aware consumer. That is
  older than this lane and wider than codegen (`install.rs`'s own
  override test asks `get` only); it is reported for filing, not fixed
  here.
- **D5.1** Guard identities are keyed by command-token generation and the
  `CommandEnvironment` domain invalidates per token; the interpreter and
  object-dispatch domains stay whole-domain.
- **D5.2** A guarded semantics key packs three disjoint fields: the
  member's own `stable_id` (bits 16–31), its `SEMANTICS_REVISION` row (bits
  2–15, so a revision up to 16383) and the release variant (bits 0–1). The
  identity, `GuardIdentity::registry_intrinsic_with_semantics`, already
  carries the stable id in its high half, so the key repeats it on
  purpose: the plan's `every_member_has_a_distinct_semantics_key` is a
  claim about the key alone, and a key read without its identity (a
  manifest field, an Explorer line, an entry of `guard_semantics_variants`)
  then names its member. A revision alone would start at `0` for every
  member and collide; a hash would not be readable. The widths are asserted
  where the key is built, at compile time for the live table (the constant
  blocks of `guard_semantics_variants` evaluate them), and the largest
  stable id today, `0x0801`, leaves the 16-bit field most of its room.
- **D5.3** Every intrinsic's guard identity value moves once. A
  release-invariant member's key was `0`, which
  `registry_intrinsic_with_semantics` folds to the bare stable id; every key
  is now non-zero, so every identity takes the packed form. No persisted
  artefact holds a value — `git grep` finds them computed only by the two
  runtimes, the WASM planner and their tests, all through
  `guard_semantics_key` — and a mismatch fails in the conservative
  direction: a module compiled by the previous compiler and linked against
  a runtime from this tree fails the identity check and takes generic
  dispatch. CC7.4's `intrinsic_table_hash` is what refuses such a module up
  front.
- **D5.4** `guard_semantics_variants` is an exhaustive per-member match with
  no wildcard, each arm a `const { &[…] }` block, so the lists stay
  `&'static [u32]` (the signature both runtimes already call) with no
  runtime table, and a member added to the catalogue must say which releases
  it is versioned across instead of inheriting the invariant contract.
  `guard_semantics_key` keeps a wildcard for the release variant
  (`release_variant`: `StringLength` by string character model, otherwise
  `RUNTIME_INVARIANT_SEMANTICS`), and
  `the_variants_are_exactly_the_keys_the_releases_answer` pins the two
  functions to each other across all five releases. A member with no
  `SEMANTICS_REVISION` row panics in `revision_in`, at compile time through
  those blocks.
- **D5.5** The plan's "the one literal `0x0306`-style assertion is updated"
  found nothing to update. The tree's only `0x0306` outside `stable_id`
  itself is `stable_ids_round_trip_without_ordinal_dependence`'s assertion
  on `stable_id()`, which does not move; no test asserts a key or an
  identity as a literal (`git grep` for `registry_intrinsic` and `<< 32`
  finds none in a test), so the runtime and VM tests stand as the plan says.
- **D5.6** The rejected-premise record lives on `WasmCodegenPlan`, in the
  two variants that did not select the addition (`native_declines`), not on
  `MixedRegionPlan`. `semantic-aot-optimisation.md` says the addition is a
  separate top-level selection that deliberately requires exact closed
  coverage of its whole script and that the region plan holds no native
  variant; the composition also needs the unit, the registry and the compile
  options, none of which `MixedRegionPlan::build` (an `ExecutableFunction` in,
  a plan out) or `mixed_plan_with_optimisations` has. The types are in
  `common_aot_plan.rs` as the plan says; `mixed_region_plan.rs` is untouched.
- **D5.7** `NativeDecline` is `{ premise, reason, sites }`, the plan's two
  fields and the direct call sites the premise was rejected at. A premise
  about a call is rejected per call, and a unit with two calls must say which;
  entries dedupe on (premise, reason) and collect their sites, so the record
  is bounded by the premises and not by the number of calls. `sites` is empty
  for a premise about the options or the unit.
- **D5.8** Every premise is evaluated independently, but the unit's proofs
  are built only once a pass the addition consumes is enabled. A premise whose
  input another premise rejects is not evaluated, since its obstacle is the
  one already recorded. All five passes exist only for the addition (the
  pass table in `semantic-aot-optimisation.md`), so while none is enabled it
  has not been asked for and the record is the options' premises alone.
  Building the common proof plan always would multiply `compile_wasm`'s own
  cost by about twelve on a 300-statement, 150-procedure script (a debug
  build: 410 ms of 446 ms, about a fifth of the 1.9 s unit build), for every
  caller, and its direct-call collection is the dominant part. So "every
  rejected premise" holds for a compile that asked for the addition; the
  options' premises are always complete. Before, the proofs were built only
  with all five passes enabled, sealed, semantic-first and not standalone;
  they are now built with any one enabled.
- **D5.9** The composition moves to `codegen/wasm/native_add.rs`. It is one
  responsibility (the premises the emitter's one native selection composes),
  and rebuilding it to record every premise rewrote it whole; keeping it in
  `pipeline.rs` would have grown that file by the record's bookkeeping. The
  selection is unchanged: the same premises in the same reading, and every
  existing native add test passes unmodified.
- **D5.10** The plan's "`aot` view" did not exist. The `wasm` text view prints
  only the module's WAT and per-function headers, `codegenPlan` was reachable
  only in `--json` and the browser panel, and `tcl explore --show aot` matched
  no view. The item adds it: descriptor `aot` (`AOT Plan`, group `codegen`,
  `Tree`), payload `aot` (the `wasm` header's `codegenPlan` on its own,
  serialised once and cloned so the two cannot disagree), and a builder in
  `view_tree.rs` that shows the plan kind, the semantic decline, each region's
  guarded candidates and the addition's selection or its rejected premises.
  `render_all` and the TUI take every `Tree` view from `tree_view_ids()`, and
  the browser and editor panels reconcile tabs from `meta.views` and show a
  view with no bespoke renderer through their structured fallback, so no
  front-end changes; `meta.views` is 36.
- **D5.11** The attestation is keyed by generation and read live; nothing
  moves on rename or hide. The plan's shape, a name-keyed table of
  `(generation, identities)` whose entry moves with `builtin_identities`, keeps
  two stores in step (the name and its generation) where one is enough: both
  runtimes already carry a command's token generation through a rename and a
  hide (the VM's `set_visible_command_generation`, the runtime's
  `insert_moved_binding`) and mint a new one for a rebinding. So
  `guarded_commands` is `generation → identities`, and `attested_identities`
  resolves the guarded name afresh, from the current namespace, to a key and a
  generation, and answers the entry at that generation. A guard follows its
  command wherever the name goes and stops at a different command at the name (a
  `proc string`, an alias, an import); a definition in a namespace that shadows
  the command reaches a different token only for calls made from that
  namespace, which is the plan's risk row. `register_attested` and
  `bind_attested_builtin` remove the entry of the generation a registration
  displaces; an entry for a command deleted by `rename x {}` stays in the table,
  unreachable, and the table grows only with registrations. D5.1 stands: the
  `CommandEnvironment` guard invalidates per token, by this read and not by an
  epoch per token.
- **D5.12** A command-table mutation moves no guard domain and clears no table;
  the events that change how names resolve keep moving the three lookup
  domains. Before, every definition, rename, deletion, alias, ensemble
  creation, OO retirement, hide, expose and profile pin cleared the table and
  moved `CommandEnvironment`, `Namespace` and `UnknownHandling`. The check's
  re-resolution now decides what a mutation staled (D5.11), so the domains stay
  for `namespace path`, namespace creation and deletion, and interpreter
  topology: the runtime's `invalidate_command_environment` keeps its eight
  calls (`namespaces_mut`, the three `ensure_*namespace` funnels,
  `delete_namespace_by_id`, `create_child`, `with_child`, `delete_child`) and
  loses its eleven at the command-table paths; the VM's `bump_cmd_epoch` keeps
  its resolution-memo clear and its counter and loses both guard effects, and
  `invalidate_lookup_guards` (new) is called from `ns_path_set`,
  `mark_interp_trusted`, `make_safe` and `delete_interp`. The Interpreter and
  ObjectDispatch domains and the trace domains are untouched, as D5.1 says.
  **Amended by the step 5 review.** The event list above ("namespace creation
  and deletion") is wider than either runtime's code, and the contract text
  that repeated it said "imports" as well. Measured from the code and by probe
  (a token per domain, checked after each script), the runtime moves the three
  domains together on `namespace path`, `namespace export`, `namespace
  unknown`, `namespace delete`, a `namespace forget` that removes an imported
  command, the creation of a `TclOO` class or object (and an `interp
  invokehidden -namespace` naming a new namespace) and `interp create` and
  `interp delete`; it leaves them alone when `namespace eval` creates a
  namespace, on `namespace import`, on `interp alias` and when `unknown` is
  bound, renamed or deleted. The VM moves them on `namespace path`, `interp
  marktrusted`, making an interpreter safe and `interp delete`, and on none of
  the rest: a `namespace delete`, `namespace export` or `namespace unknown`
  leaves a VM token valid. That narrower set is older than this decision, is
  filed as issue #2292, and neither runtime's invalidation is widened by the
  fix. `CommandEnvironment` moves on no definition, rename, alias or import in
  either runtime. The events are stated in `guard.rs`'s doc comments (a
  comment change, so D5.13's "untouched" is about code), the design page's
  guard paragraph and `rename-alias.md` § 3.5. One probe result needs its
  reason: a `namespace forget` of a command imported from the global namespace
  moved nothing, because the runtime's `ns_forget` builds `::::name` for a
  global-qualified pattern, finds no import and removes none (real Tcl 8.6 and
  9.0 remove it); the runtime does move the domains when a forget removes an
  import from another namespace. That defect is reported under *Step 5 — what
  the next steps read*, not fixed here.
- **D5.13** No `GuardDomain::CommandToken`. The plan left it to the implementer
  whether the domain lattice needs the finer key. It does not: a domain is an
  epoch a token snapshots at issue and compares at check, and the command's
  generation, which the check already reads to find the attestation, is that
  epoch for one command. A variant would put a second per-command counter beside
  the generation for the same job and add a bit to every token's
  `GuardDomains`, which the ABI's `domains` word, the registry's base sets and
  the emitter carry. `rust/tcl-runtime-api/src/guard.rs` is untouched, so the
  ABI, the identity values and a compiled module's guard sites are unchanged: a
  module compiled before this item links and runs against a runtime from this
  tree.
- **D5.14** The profile pin keeps the whole table, and the pinned surface decides
  what a guard reaches. The plan's "keeps the entries whose spec is in the
  pinned generation" filters at the pin; filtering at the read does the same for
  every release an interpreter is pinned to and back, with nothing to rebuild
  when the pin moves. An entry for a command the pinned release lacks is kept
  and unreachable (`command_visible_for_surface_at` in the runtime, the surface
  filter of `resolve_command_fqn` in the VM) and reachable again when the pin
  returns to a release that has the command
  (`a_pin_to_a_release_without_the_command_leaves_it_unattested`, `lassign`
  under 8.4, in both runtimes). A token whose domains include `Interpreter`,
  as the registry's base sets do, still goes stale at a pin, as an
  interpreter-policy change should; what the pin keeps is the attestation, so a
  guard can be issued after it, where a pinned VM used to hold none (its
  `bump_cmd_epoch` cleared the table).
- **D5.15** The real-link row cannot tell the fast path from the fallback. The
  harness reads the module's standard output, and a guarded intrinsic answers
  alike on either path: `renamed fallback` tells them apart because the
  mutation changes the command (`proc string` answers 99), and `traced
  fallback` answers 3 on both. The "unrelated proc keeps the fast path" row is
  added as the plan says and proves that an emitted module links and runs
  against an interpreter whose command table has moved; the proof that the guard
  survives is `guarded_intrinsic_guards_survive_unrelated_command_mutation`,
  which makes the exact `tcl_codegen_guard_check` call an emitted module makes.
  A row that told the paths apart needs a probe in the runtime, such as a
  fast-path counter, which is not this item's.
- **D5.16** The VM twin is re-stated where it lives. The plan names
  `command_mutation_deopt_e2e.rs`, which drives the compile epoch through
  scripts and holds no guard test (it does not mention a guard); the test the
  item re-states is `family_b_tests`'s
  `any_command_mutation_invalidates_guard_and_live_identity_attestation`, so
  the twin is re-stated in place under the runtime's new name. The e2e binary
  runs green unchanged, as a regression check on the compile epoch that
  `bump_cmd_epoch` still owns.
- **D6.1** CC6.1 covers the six existing fields; `runtime_backing` joins
  the floor in CC7.1. **D6.2** `DependencyTier` is defined in `tcl-pkg`
  and re-exported by `tcl-registry`'s `model::capability`; `tcl-spectcl`
  depends on `tcl-pkg` for the manifest and lockfile data model only.
  **D6.3** An overlay miss is an error at the ingress. (D6.2 is amended
  by D6.6 and D6.7 below.)
- **D6.4** The widened floor keeps the floor's existing shape and reach:
  command-level values only, and the shipped value wins wherever the
  shipped command has one. It reads `CommandSpec` fields, so the same fields
  on a `SubCommand` or a `CommandForm` (both carry `lowering_hook`,
  `analyser_hook`, `semantic_operation` and `state_transitions`) are not
  restored — the limitation the floor's comment already records for the
  codegen hooks. That is reported, not fixed: subcommands and forms merge by
  name with rows added and removed, a different shape from a single field,
  and the loader's stamp rule already covers the stamps among them. An
  override may still *add* a value to a shipped command that ships none,
  exactly as a new command may declare one
  (`the_floor_does_not_invent_facts_for_a_new_command`): the floor stops a
  fact going away, it neither invents nor forbids one. It is silent, as it
  is for taint; the pack howto says so.
- **D6.5** `native_lowering` and `bpf_op` have no loader statement, so a
  pack cannot swap them, only lose them by replacing a shipped command and
  saying nothing; their rows test that (`break`, and `pass` in the `bpf`
  dialect). The four fields a pack can spell are swapped for a different
  catalogue member. `semantic_operation` is swapped for `Invoke`, not for an
  `Intrinsic`: CC4.2's tier gate already strips an `Intrinsic` spelling from
  every tier but a bundled one, so it could not exercise the floor. Every row
  first asserts the override took effect (its `arity 7..9`, a window no
  shipped command has), so a green row means the floor restored the value
  rather than the override never having installed.
- **D6.6** `tcl-spectcl` reads the manifest and lockfile through a new leaf
  crate, `tcl-pkg-model`, not through `tcl-pkg` (D6.2's dependency). The
  plan's own risk clause said to check the build graph, and the graph
  answers it. `tcl-pkg` in `tcl-spectcl`'s dependencies puts 33 more crates
  in `tcl-lsp-server`'s build and 39 in `tcl-explorer`'s and
  `tcl-spec-studio`'s — `ureq`, `rustls`, `webpki-roots`, `zip`, `tar`,
  `xattr`, `tcl-sandbox` — and `cargo check --target
  wasm32-unknown-unknown -p tcl-sandbox` fails (`wait-timeout` compiles for
  `unix` and `windows` only, "cannot find module or crate `imp`"), so every
  wasm host that loads packs (`tcl-lsp-server-wasm`, `tcl-lsp-server-wasi`,
  `tcl-explorer-wasm`, `tcl-spec-studio-wasm`) would stop building — outside
  the workspace, where `cargo check --workspace` never sees it. The layout
  contract (`project-layout.md` rule 5) also keeps a developer-tool crate out
  of a pack loader's graph, and `tcl-userdirs` is the precedent for lifting
  the shared part into a leaf. What moves, by `git mv`: `errors.rs`,
  `json.rs`, `version.rs`, `manifest.rs` and `lockfile.rs`; `tier.rs` is new.
  `tcl-pkg` re-exports the five modules as it re-exports `tcl-userdirs`, so
  the package manager's own modules and `tcl-cli` keep their paths (the
  layout contract asks for direct imports; a re-export is what the
  precedent does, and it keeps this diff to the crate boundary).
  `LockFile::stamp`, the model's one use of the clock, becomes
  `tcl_pkg::stamp_lockfile`, so the model needs no `chrono`. The step 8
  plan's `rust/tcl-pkg/src/manifest.rs` and `lockfile.rs` are
  `rust/tcl-pkg-model/src/` files now.
- **D6.7** `DependencyTier` is defined in `tcl_dialect::model`, beside
  `Provenance` and `WorkspaceTrust`, and re-exported by `tcl-registry`'s
  `model::capability` (D6.2 said `tcl-pkg`). `tcl-registry` cannot depend on
  `tcl-pkg`, and a dependency on the model crate would put `regex` and
  `serde_json` under every crate; `tcl-pkg-model` needs the enum for
  `tier.rs` and step 8's `SpecDirective::requested_tier`, and `tcl-dialect`
  is the lowest crate all three reach.
- **D6.8** How discovery reads a tier. The plan says "the nearest
  `tclpkg.lock`"; the tier is read from the *outermost* directory, inside
  the workspace folder and at or above the package, that holds both a
  `tclpkg.tcl` and a `tclpkg.lock`. An installed dependency's directory can
  hold a manifest and a lockfile of its own (its tarball can carry any file),
  and the nearest pair would let a dependency name itself the root of its own
  graph — the one distance the matrix exists to deny it. A file's package is
  the one whose manifest is nearest above it; the package is `Root` when its
  directory *is* the project root (a fact about where a directory sits, never
  about a name a manifest claims), and otherwise the tier is
  `tier::dependency_tier` of the project's manifest and lockfile for the
  package the file's own manifest names. The graph rules: named in `require`
  is `Direct` whatever else requires it; reached through the `require` graph
  is `Transitive`; named in `dev-require` only, or reached only from there,
  is `Development`; listed but reached from neither (a stale entry) is
  `Transitive`, the least a listed package gets; not listed is `None`. No
  lockfile, an unlisted package, and a manifest or lockfile that does not
  read leave `None`, as the plan says, and a pack with no tier is not
  narrowed. Only a file found `BesideManifest` is placed: a pack named in
  `tclLsp.specPacks` is the user's own choice of it. Residual, recorded on
  the design page: a manifest names its own package, so a dependency that
  names itself a package the lockfile does not list is unlimited; the
  lockfile hash of each pack that step 8 adds is what binds a pack file to
  the package it claims.
- **D6.9** The gate's shape. Two functions in `stamps.rs`, run in
  `pack::load_sources` on every merged command after the merge has recorded
  the declaring file's tier on `PackCommand::dependency_tier`. Stamps first,
  through `stamp_refusals`, which gains the tier: the provenance gate is
  asked first and names itself when both refuse (an author reads it off where
  the pack sits; every pack that has a tier is a workspace-tier file, so in
  the product it is the reason named), then the capability gate
  (`RefusalReason::Capability`, "only the workspace's own package may"), then
  rule 1. Then `admit_declarations` drops `alias_of` and a `runtime_backing`
  other than `none`, after the stamp rule, so rule 1 still reads the
  `alias_of` a refused stamp would have needed. Each refusal is a warning on
  the command's row that names the declaration and the tier; the command keeps
  every other fact. The strip is one memo keyed by the spec's address and a
  `Drops` set. `pack::set_key` mixes a file's tier: a package moving in the
  lockfile's graph changes what its packs load, and the registry cache is
  keyed by the set's key. `install_into`'s assertion asks both gates. The
  Studio's and `spectcl_check`'s previews pass `None` — they see no lockfile —
  and so do not show a capability refusal.
- **D6.10** `CodegenCapability::reference_body` is `ReferenceBodies
  { Forbidden, AnySource }`, not the page's `Option<BodySource>`.
  `BodySource`'s variants carry data (a path, a text), so a capability
  cannot hold "from which source"; no tier is allowed one source and not the
  other; and a fourth `bool` trips `clippy::struct_excessive_bools`, which
  the plan's "no new `#[allow]`" forbids answering. A tier that gains one
  source without the other adds its variant. Nothing reads the field yet
  (reference bodies are step 8's); the matrix test holds its values.
- **D6.11** The server reloads the packs when a `tclpkg.tcl` or a
  `tclpkg.lock` changes (beyond the plan's files). Discovery now reads both,
  so a lockfile rewritten by `tcl pkg install` changes what a dependency's
  pack may declare with no `.tclspec` moving; without a trigger the old tiers
  would stand until the next reload, and a stale tier that is *nearer* than
  the graph now says is the unsafe direction. The manifest is a `.tcl` file
  the source watcher already reports, and it stays an indexed Tcl source; the
  lockfile gets a watcher of its own (`**/tclpkg.lock`). Both set the pack
  reload flag `partition_watched_file_changes` already had.
- **D6.12** The overlay miss is answered by each consumer of the door, not by
  the door. The plan: `context_registry` and `registry_for_environment_if_built`
  return `Result<Arc<ContextRegistry>, OverlayMiss>`, and the test at
  `ingress.rs:796` is inverted — done, as `an_uninstalled_overlay_is_an_error_not_the_plain_generation`
  (the old test's first half stays as `context_registries_carry_the_expected_stores`),
  and `plain_context_registry` is the new infallible door for overlay `0`. What
  the plan did not say is what each caller does with the error, and they are not
  alike. What produces something a host compiles or a user is offered as an
  edit — the compile service, `compilation_unit` and the checks and rewrites
  built on it — declines (D6.14, D6.16). What only advises and runs again when
  the packs arrive — the analyser's three overlay reads (`analysis_context`,
  `resolve_walk_environment`, the incremental re-analysis) and the two
  semantic-token queries — reads the plain generation through a door named for
  it (`environment_ingress::analysis_registry`, `token_registry` in `tcl-lsp-db`),
  so no `unwrap_or` sits at the ingress and the fallback survives only where it
  is right. The line is drawn by what a wrong answer costs: a diagnostic pass
  without the packs is corrected by the re-analysis a reload triggers (the e2e
  `the_bundled_eda_loadables_make_their_vendor_commands_known` waits for
  exactly that settle), while a unit built without them is memoised under the
  pack's key and is where the optimiser's rewrites come from. The analysis
  re-runs when the overlay epoch moves because `file_analysis_incremental` reads
  the unit query, which reads it (`the_analysis_reads_the_packs_once_they_install`).
- **D6.13** The server's optimise path builds no compile service. The plan's
  Files list has `optimise_document_command` build the service with
  `spec_pack_key`; there is no service there to build. The command hands
  `core_report::optimise_under_policy` the registry `Backend::registry_for_dialect`
  returns, which installs the workspace's overlay itself
  (`registry_for_dialect_with_packs`) and so cannot miss, and the language
  server compiles no bytecode (the design page's words since D4.20). So the
  item delivers the door — `RegistryTarget::Overlay`,
  `BytecodeCompileService::for_profile_with_overlay` — with its tests through a
  real pack in `codegen_stamps.rs`, and the exit evidence "the server's optimise
  path compiles under the workspace overlay" is the two e2e tests that already
  ran it, `a_pack_const_fold_body_folds_a_call_site_in_the_optimiser` and its
  negative `without_the_pack_the_same_call_site_does_not_fold`
  (`rust/tcl-lsp-server/tests/e2e/spec_packs.rs`), run and green. No shipped host
  builds a service through the door: the `tclvm` engine takes an owned
  registry and the debugger the profile's shared generation. The server's
  changes are at the two ends of the unit query: the handle takes `None` from
  `document_compilation_unit_for`, the compiler-checks pass reports each
  recorded miss, and a pack reload moves the overlay epoch beside its key
  publish (D6.14).
- **D6.14** How `compilation_unit` abstains, and how it is asked again. It
  returns `Option<Arc<CompilationUnit>>`; `proc_taint_solve` returns an `Option`
  too; `compiler_check_diagnostics` answers no checks and no optimisations;
  `file_analysis_incremental` supplies no override, and the analyser builds its
  own against the registry it reads; `document_compilation_unit_for` is an
  `Option`, and `document_compilation_unit` (no overlay, so no miss) keeps its
  `Arc`. The abstaining path reads nothing an installation of the packs moves,
  so it would stand for every revision that leaves `(file, cfg, overlay)`
  alone. The first design made it volatile with salsa's `report_untracked_read`
  and was measured wrong: the query itself ran again at the next revision, but
  a re-executed volatile query whose value changed keeps its old `changed_at`,
  so `file_analysis_incremental` and everything else that had read the `None`
  stayed on the answer memoised without the packs — an analysis of a pack
  command stayed a W123 after the overlay was installed and a revision ran. So
  the overlay is an input, as the evaluator epoch is (D104):
  `tcl_registry::overlay_epoch` moves whenever an overlay generation is
  installed and whenever a sweep retires one, `tcl_lsp_db::OverlayEpoch` is
  the salsa input that mirrors it, `unit_registry` reads it for every non-zero
  overlay, and a host installs the packs and then moves it with
  `set_overlay_epoch` — the server does, beside its key publish
  (`sync_overlay_epoch`). Compare-then-set, so a sync that finds nothing moved
  invalidates nothing, and everything that resolved an overlay — the unit, its
  per-procedure queries, the analysis and the tokens that read it — depends on
  it at any depth (`an_abstention_runs_again_once_the_overlay_is_installed`,
  `the_analysis_reads_the_packs_once_they_install`,
  `syncing_the_overlay_epoch_lets_an_abstained_unit_build`). Until the epoch
  moves the abstention stands: a database cannot see the cache change, only its
  inputs. "Logged once": `tcl-lsp-db` has no logger, and the server sends a
  worker's faults to stderr with `eprintln!`, so the database records each
  distinct `(environment, overlay)` miss once in a process-wide log,
  `take_overlay_misses`, and `compute_compiler_diags` reports what it finds.
- **D6.15** The queries hold the overlay generations they resolve (beyond the
  plan). A unit is built by per-procedure queries that each look the registry up
  again, and the process cache retires overlay generations past 64 entries,
  keeping only the key being built — so a reload that lands while a pass at the
  superseded key is running would hand the per-procedure queries a miss the unit
  never saw. `OverlayGenerations` (64 entries, first in first out, process-wide)
  keeps what `registry_with_overlay` resolved and answers from it first; a
  generation is content-addressed by its key, so serving one the cache has
  dropped is never stale (`a_retired_generation_still_serves_the_queries_that_resolved_it`).
  It is process-wide because the cache it backs up is, and because the crate's
  tests build `TclDatabase { storage }` by literal at 21 sites. The per-procedure
  queries read it through `nested_registry`, which stops with the miss in its
  message for a key no unit query resolved — a caller error no production path
  reaches, since a per-procedure query runs only inside a unit or a
  re-verification of one. The alternative, a plain registry inside a memoised
  per-procedure query, would put a lattice computed without the packs under the
  pack's key, which is the fault the item removes.
- **D6.16** `BytecodeCompileService::for_profile_with_overlay` returns
  `Result<Self, OverlayMiss>` and looks the generation up again at every
  compile. A built service can outlive its generation, and a compile through it
  then declines with a `CompileError` naming the overlay, on every compile entry,
  instead of compiling plain. A compile for another profile than the service's
  uses the service's overlay for the profile asked for: the packs are installed
  per profile, so a service for `tcl9.0` asked to compile for `tcl8.6` finds no
  generation and declines. Overlay `0` is `for_profile`. `RegistryTarget::registry`
  and `registry_for_profile` return `Result`, and the four compile paths take `?`.
- **D7.1** `HANDLER_EXTRA`, `STDLIB`, `NOT_REQUIRED` become
  `runtime_backing` rows; `KNOWN_UNBACKED` stays as the drift waiver;
  `xtask` links `tcl-runtime` and `tcl-vm` to ask `backing_report()`.
  **D7.2** The WASM manifest is a custom section `tcl.manifest` encoded
  by one function pair in `tcl-runtime-api`.
- **D7.3** `BodySource::PackText` carries its text: `PackText { text:
  &'static str }`, where the page's `PackText,` is a unit variant. The
  statement `tcl-body {-pack-text {TEXT}}` has a body to keep, and a unit
  variant would drop it at load — the Studio round trip could not write it
  back, and rung 3 (CC8.1) compares the body against the live procedure. The
  design page's block is corrected to match.
- **D7.4** A `ShippedBuiltin`'s identity is the spec's own name, spelled as
  the spec spells it, `::` kept — the spelling the VM's builtin table
  already keys (`register_spec_builtin` keeps the registry's spelling as the
  stable identity). A command the registry carries under two spellings
  (`tcl::mathfunc::abs`, `::tcl::mathfunc::abs`) is two specs and two
  identities. The qualified `oo::Helpers::*` specs follow their bare twin:
  shipped where it is, and the `ooutil` twins (8.6/8.7, from a package, not
  core) declare none. Mathop's bare operator word (`+`) is grammar
  evaluated inside `expr`, not a command, and stays `None`.
- **D7.5** The rows are the report's rows and are checked against the report.
  335 declarations (324 `ShippedBuiltin`, 11 `TclBody`) and 54 defaulted
  `None` fall out of the 389 rows; `every_core_command_declares_a_backing`
  reads the committed `docs/generated/wasm-command-backing.md` rather than
  keeping a second list in the test, checks it in both directions, and
  requires the non-core specs in the Tcl table to declare nothing. The
  three lists CC7.2 deletes are the report's source until then; this test
  keeps the declarations honest against it, and can be deleted with them.
- **D7.6** One spelling serves six consumers. `BackingSyntax` (in
  `tcl-spectcl`, beside `semantic_operation_spelling`) parses the words after
  `runtime_backing`, spells them back, converts a spec's backing, and leaks
  its strings only where the loader needs `'static` data — the Studio parses
  and re-spells on every edit and must not leak. The Studio holds the
  statement's own spelling as the draft value and gives the kind the `text`
  wire tag: an editor for a closed vocabulary would not fit strings, paths
  and bodies, and no front-end change was needed. A spelling that does not
  read renders a `TODO` comment in `.tclspec`, and no field in `.rs`.
- **D7.7** The `-pack-text` notice is a `PackNotice` raised where merged
  commands are assembled (`pack::load_sources`), like the stamp refusals and
  the dormant-hook notices, because a loader `Notice` is always a warning and
  this is information. The Spec Studio assembles its own set (`store.rs`) and
  `spectcl_check` previews an install; neither raises it, having no consumer
  for an information line about a body they do not run.
- **D7.8** The plan expects `gen_irule_test_data.rs`'s output byte-identical
  because "every iRules command is `None`". The iRules registry also holds the
  shared Tcl core commands (surface `ALL_TCL_AND_IRULES`), which now declare a
  backing, so gating on `None`/`HostNative` drops 46 of the table's entries —
  `append`, `array`, `binary`, `break`, `catch`, `clock`, `concat`, `continue`,
  `encoding`, `error`, `eval`, `expr`, `for`, `foreach`, `format`, `global`,
  `if`, `incr`, `info`, `join`, `lappend`, `lindex`, `linsert`, `list`,
  `llength`, `lrange`, `lreplace`, `lsearch`, `lset`, `lsort`, `regexp`,
  `regsub`, `return`, `scan`, `set`, `split`, `string`, `subst`, `switch`,
  `trace`, `unset`, `uplevel`, `upvar`, `variable`, `while` and `pkg_create`.
  They were dead: the harness looks a command up in `_command_map` only from
  its `unknown` handler, for commands the interpreter has not defined, and
  real Tcl defines these. The file is regenerated (933 stub actions, was 979)
  and the `tcl-irule-test` suite passes on it.
- **D7.9** The ports declare their backing. `every_port_loads_and_matches_its_shipped_spec`
  compares each port's draft with its shipped spec's, and a shipped spec now
  declares one; the DSL has the spelling, so the difference is not a gap to
  document but a line to transcribe — `if`, `foreach`, `switch`, `lsort`,
  `string`, `upvar`, `return`, `oo::class` and `subst` each gain
  `runtime_backing shipped-builtin NAME`.
- **D7.10** The family is the widest reach of a member under any invocation
  form, which gives 14 `Value` and 14 `FamilyB` members (the page's "about
  half"). `string is` (its `-failindex` stores, which is why its spec is not
  `pure`) and `regexp` (its match variables store through the adapter) are
  Family B although a call without those words touches nothing; classifying
  by the common form would let a later fast path skip a trace the rest of the
  forms run. Every Family-B member takes `GuardDomain::VariableTrace`, `puts`
  included: the lattice has no channel domain, and adding one is a lattice
  change (D5.13's reasoning), not this item's. `fires_traces` marks the four
  members that run a variable's traces while only *observing* it (`info
  exists` a read trace, `array exists`, `names` and `size` an array trace),
  measured with `tclsh` 8.4, 8.5, 8.6 and 9.0; a storing member's write
  traces are the store's and are not marked, although `lset`, `dict set` and
  the rest do run read and write traces in C Tcl. `tcl-vm` answers all four;
  `runtime/rust` answers the array queries and not `info exists`, which runs
  no read trace there (reported, and stated in `family-b-routing.md` § 4),
  so the cross-check test lives in the VM and the runtime keeps its existing
  `array_trace_oracle.rs`.
- **D7.11** Neither runtime had a per-call table of guard domains for the
  family to replace: `tcl_codegen_guard_prepare` takes the mask the compiler
  sends, and the VM's `prepare_command_guard` has no production caller. The
  family is read where a domain is decided, which is two places. The compiler
  builds a guarded plan's domains as the dispatch dependencies' plus the
  member's family's (`guard_domains_for_intrinsic`), and both runtimes'
  `prepare_command_guard` refuse a request that omits a domain the identity's
  family requires (`GuardError::DomainsInsufficient`), so a module from a
  compiler that forgot the variable-trace domain declines instead of taking a
  fast path over a variable store that has a trace on it. The plan's
  "registration derives the domain" is read as issuance: a registration
  attests an identity and has no domain to derive. Only `StringLength`, a
  `Value` member, is guarded today, so no emitted request changes.
- **D7.12** No `SEMANTICS_REVISION` row rises. CC5.1's rule raises a member's
  row when what a compiled fast path may assume of it changes, and nothing is
  compiled against a Family-B member's contract: the boxed fast path is
  `StringLength`'s alone and neither runtime executes another intrinsic. The
  requirement is additive and refused at issuance, so a module asking for an
  under-covered guard declines whether or not its identity moves. CC7.4's
  table hash covers the family as well as `guard_semantics_variants()`, so a
  later change to the classification is visible to the manifest without a
  row bump.
- **D7.13** `family-b-routing.md` § 4 had no gap row for the family split to
  close: the page mentioned no intrinsic at all. § 3 gains the *Intrinsic
  families* subsection, stated as the code is, and § 4 gains the one gap this
  item measured, `info exists` in `runtime/rust`.
- **D8.1** `Engine::set_release` is slice 4's. **D8.2** `docker.rs` stays
  registry-free; `tcl docker create` computes native extensions.
- **D9.1** Versioned stamps are `StampWindow<T>` slices mirroring
  `ArityWindow`. **D9.2** `DialectProfile::evaluation_point` is the
  evidence gate; `TclVersion::from_profile` delegates.
- **D10.1** The authored header is `runtime/rust/include/tcl.h` (the ABI
  § 4.1 places it with the runtime); the shim includes it by path.

### Open questions for the owner

The plan does not block on these; each states what it assumes.

- **Q1** Whether `clause_grammar` should also cover `switch`'s inline
  `pattern body …` pairs. Assumed no: the page keeps `case_list` as a
  value grammar, and the two share vocabulary only.
- **Q2** Whether the studio's clause-grammar form is editable in step 2
  or read-only until a pack needs to author one. Assumed read-only
  (CC2.3); the round trip does not depend on the editor.
- **Q3** The exact set of hooks retired in step 2 (CC2.13's table). The
  page says "most"; the table is the plan's reading of each handler.
- **Q4** `content_hash` as `u64` (D4.3) versus the page's `[u8; 32]`.
- **Q5** `tcl-spectcl` depending on `tcl-pkg` (D6.2), or a data-model-only
  crate split out of `tcl-pkg`.
- **Q6** The `load` bridge's engine ownership (CC10.3): a shim `Interp`
  over `&mut E` for one load, or the bridge living in `tcl-vm-cli` as the
  embedder. Assumed the former.
- **Q7** Whether the studio should seed an `alias_of` suggestion from the
  realm's alias facts (the page: "may seed a suggestion"). Assumed not in
  step 4.
- **Q8** Which new `IntrinsicId` members "grow the intrinsic table by
  family" (CC7.3). Assumed none in step 7; the family split and the
  trace-firing flag land, members follow their own changes.
- **Q9** The `-inline` / capture-variable option relation's message
  (CC2.6): taken from `tclsh9.0`'s error text at implementation time and
  pinned by the differential row.
- **Q10** The `xtask` build cost of linking both runtimes (CC7.2).
