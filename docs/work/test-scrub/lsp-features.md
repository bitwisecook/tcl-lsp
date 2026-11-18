# Test scrub: LSP features

Ledger: [`ledger/lsp-features.tsv`](ledger/lsp-features.tsv), 677 failing tests (672 triaged here plus five `$`-completion witnesses routed from server-e2e-db): the `tcl-lsp-core` library (473 failing on `work` after today's fixes, plus the 19 minifier rows the owner retired), the 177 server navigation tests routed from server-e2e-db that still fail (127 end to end, 50 library; 4 of the 181 now pass), and the three `preview_tickets_e2e` tests. Measured on `work` on 17 November 2026; every Tcl claim was run on tclsh 8.4.20 to 9.1.0 (Jim 0.84 where it applies). Tk and Itcl cannot load here; rows that depend on them say so.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| `KEEP-ORACLE` | 220 |
| `KEEP-CONTRACT` | 155 |
| `FIX-FIXTURE` | 149 |
| `FIX-EXPECTATION` | 84 |
| `REWRITE` | 35 |
| `RETIRE` | 13 |
| `UNDECIDED` | 7 |
| `KEEP-F5` | 5 |
| `MERGE` | 4 |
| total | 672 |

## Causes ranked by tests blocked

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `uncalled-procedure-facts` | 30 | FIX-FIXTURE 21, KEEP-CONTRACT 8, MERGE 1 |
| `cross-file-resolution-pinned-release` | 21 | FIX-FIXTURE 21 |
| `pinned-release-cross-document-class-withdrawn` | 21 | FIX-FIXTURE 20, MERGE 1 |
| `minifier-lexical-naming-passes-retired` | 19 | RETIRE 10, FIX-EXPECTATION 9 |
| `import-link-installed-where-tclsh-refuses` | 15 | KEEP-ORACLE 15 |
| `descriptor-in-unentered-body` | 14 | KEEP-CONTRACT 12, KEEP-ORACLE 2 |
| `tk-not-required` | 13 | FIX-FIXTURE 13 |
| `refusal-reason-wording` | 12 | FIX-EXPECTATION 12 |
| `renamemethod-link-proof-refuses` | 11 | KEEP-ORACLE 9, KEEP-CONTRACT 2 |
| `forced-import-shadow-resolver-disagree` | 10 | KEEP-ORACLE 10 |
| `if-to-switch-trace-gate-incomplete-visit` | 10 | KEEP-CONTRACT 9, FIX-EXPECTATION 1 |
| `original-member-rename-refuses` | 9 | KEEP-ORACLE 9 |
| `package-census-absent-head-incomplete` | 9 | KEEP-CONTRACT 9 |
| `profileless-registry-fixture` | 9 | FIX-FIXTURE 9 |
| `switch-to-dict-edits-expected` | 9 | FIX-EXPECTATION 9 |
| `user-metaclass-made-class-publishes-no-factory` | 9 | KEEP-ORACLE 9 |
| `bare-undefined-command-raises-fixture` | 8 | FIX-FIXTURE 8 |
| `original-command-rename-refuses` | 8 | KEEP-ORACLE 8 |
| `lexical-member-target-pinned-release` | 7 | REWRITE 7 |
| `method-lens-partial-census-lower-bound` | 7 | FIX-EXPECTATION 7 |
| `classmethod-on-tcl86-fixture` | 6 | FIX-FIXTURE 6 |
| `fixture-raises-before-subject` | 6 | FIX-FIXTURE 6 |
| `minifier-default-tier-switch-arm` | 6 | KEEP-CONTRACT 6 |
| `lenient-tcl-strict-rename-refuses` | 5 | FIX-EXPECTATION 5 |
| `literal-foreach-installed-methods-missing` | 5 | KEEP-ORACLE 3, KEEP-CONTRACT 2 |
| `logical-workspace-hierarchy-cross-document-none` | 5 | KEEP-CONTRACT 5 |
| `refactor-refusal-reason-generic` | 5 | KEEP-CONTRACT 5 |
| `release-extraction-refused` | 5 | KEEP-ORACLE 5 |
| `brace-expr-expectation-changes-evaluation` | 4 | FIX-EXPECTATION 4 |
| `class-publication-surrogate-name-missing` | 4 | KEEP-CONTRACT 4 |
| `collection-receiver-method-token-missing` | 4 | KEEP-ORACLE 3, REWRITE 1 |
| `constant-command-value-dispatch-references` | 4 | KEEP-CONTRACT 3, KEEP-ORACLE 1 |
| `escaped-class-name-declaration-record-missing` | 4 | KEEP-ORACLE 2, FIX-EXPECTATION 1, REWRITE 1 |
| `hosted-proc-gets-native-declaration` | 4 | KEEP-CONTRACT 4 |
| `member-rename-refuses-unknown-roster` | 4 | KEEP-CONTRACT 3, KEEP-ORACLE 1 |
| `minifier-keyword-selection-receipt-missing` | 4 | KEEP-ORACLE 4 |
| `unknown-head-world-ends` | 4 | FIX-FIXTURE 3, FIX-EXPECTATION 1 |
| `variable-rename-edit-extent-shape` | 4 | REWRITE 4 |
| `workspace-lexical-rename-pinned-release` | 4 | FIX-FIXTURE 4 |
| `class-variable-unresolved` | 3 | KEEP-ORACLE 3 |
| `classmethod-analysed-under-tcl8.6` | 3 | FIX-FIXTURE 2, FIX-EXPECTATION 1 |
| `cross-document-wildcard-import-references-missing` | 3 | KEEP-ORACLE 3 |
| `embedded-operand-hover-missing` | 3 | KEEP-ORACLE 3 |
| `fixture-reads-unset-variable` | 3 | FIX-FIXTURE 3 |
| `foreach-rename-reinstall-not-followed` | 3 | KEEP-ORACLE 3 |
| `inline-expectation-reads-free-variable` | 3 | FIX-EXPECTATION 3 |
| `lenient-logical-scoped-environment-missing` | 3 | KEEP-CONTRACT 2, FIX-EXPECTATION 1 |
| `linked-editing-logical-self-call-withheld` | 3 | KEEP-CONTRACT 3 |
| `logical-implementation-descendants-missing` | 3 | KEEP-ORACLE 3 |
| `member-rename-refusal-reason-generic` | 3 | KEEP-CONTRACT 3 |
| `namespace-rename-whole-word-edit-extent` | 3 | KEEP-CONTRACT 3 |
| `oo-define-if-wrapper-body` | 3 | KEEP-CONTRACT 2, KEEP-ORACLE 1 |
| `opaque-unicode-class-name-declaration` | 3 | KEEP-CONTRACT 2, KEEP-ORACLE 1 |
| `package-not-required` | 3 | FIX-FIXTURE 3 |
| `t100-silent-under-logical-unit` | 3 | KEEP-CONTRACT 3 |
| `type-definition-object-variable-untyped` | 3 | KEEP-ORACLE 2, MERGE 1 |
| `uncalled-body-generic-entry` | 3 | KEEP-CONTRACT 3 |
| `variable-held-command-literal-rename-refused` | 3 | KEEP-CONTRACT 3 |

223 further causes hold one or two tests each; the ledger names them.

## UNDECIDED, for the coordinator

- `call_hierarchy::tests::classmethod_incoming_names_proc_callers_qualified` (lib): two faults: (1) the instance-method caller ::Factory::viaInstance is missing (tclsh 9.0.4/9.1.0: [$o viaInstance] -> 1; descriptor-in-unentered-body) and (2) the proc caller is labelled `helper` where the test wants `::util::helper`; call_hierarchy.rs item_for_proc says "the short name is presentati
- `definition::tests::command_heads::fn_unresolved_indirect_head_does_not_block_the_bareword_fallback` (lib): fixture: `set cmd $env(WHATEVER)` raises can't read "env(WHATEVER)" on tclsh 8.4.20-9.1.0 when unset (use [gets stdin]); policy question: after an unbounded `$cmd` head, may Go to Definition on a later bare `greet` answer the only written `proc greet` (navigation tier), or does it abstain like a pro
- `definition::tests::variables::jump_to_catch_resultvar_bareword_resolves_to_the_reused_variable` (lib): tclsh: `catch {foo} name` writes the formal `name` (resolveSwitch a b -> invalid command name "foo"); product answers two ranges, the formal (line 0) and the cursor's own catch word (line 1); not a wrong variable. Question: does Go to Definition at a write site list that site, or only the declaratio
- `definition::the_server_hands_code_actions_the_shadow_answer` (e2e): inline is proof tier (it changes code); SHADOW case gives no action (null), INERT case expects the LOCAL body offered. Under lenient tcl the sibling's export (SRC case) and the absence of any export elsewhere (LOCAL case) are not proved loads; whether inline may be offered from an unsourced sibling'
- `navigation::an_unbound_dollar_read_never_resolves_to_a_same_named_method` (e2e): tclsh 8.6.18/9.0.4: [widget new] -> can't read "thing": no such variable (variables and commands are disjoint). Hover and definition abstain as asserted; references from the read are non-empty but the failure message does not print them: if they are only the read itself (line 1) the expectation is t
- `rename::rename_mathfunc_override_updates_call_site_and_skips_unrelated_proc` (e2e): rename from proc ::nsa::tcl::mathfunc::pf returns no edits (no refusal reason). original_command_rename.rs has ReadonlyMathReferences ('expression identifier references have no separately authorised edit owner'): if that is the contract the expected outcome is a stated refusal, not a silent empty ed
- `tests::navigation::native_pattern_hover::native_pattern_hover_preserves_source_shadows_and_retired_snapshots` (lib): first failing check is internal (provider `rename regexp {}; oo::class create regexp {}` not in index.original_class_source_candidates on the regexp conditional path); docs default dialect tcl8.6 (Native by design: the test asserts !allows_lexical_declaration_advice). tclsh 8.6.18/9.0.4: that provid

## Wrong answers (fix first)

Accepted edits that break code:

- **Namespace rename leaves callback and list-built names behind.** Renaming `::old` is accepted while `after 0 {::old::tick}` and `set handlers [list ::old::tick]` keep the old name; tclsh 8.4.20 to 9.1.0 print `TICK` before and `invalid command name "::old::tick"` after (`namespace_rename` `tp_rewrites_a_name_inside_a_walked_callback_body`, `tp_refuses_an_unattributed_embedded_name`, and the e2e twin). Slug `wrong-namespace-rename-leaves-callback-word`.
- **The namespace-variable hazard clears a branch-dependent constant.** `set n other; if {$c} {set n v}; variable v; set $n 2` reports no hazard for renaming `v`; tclsh gives `::ns::v`=2 before, `::ns::total`=1 after (`rename_safety` `tp_a_branch_dependent_constant_still_refuses`; slug `wrong-namespace-variable-hazard-clears-branch-constant`).

Navigation answers tclsh contradicts:

- **Imports the interpreter refuses are installed** (`import-link-installed-where-tclsh-refuses`, 15 index tests): glob then exact and exact then glob, two importers of one name, an import in a procedure body ordered by text rather than by when the body runs, `namespace forget` not revoking, an unforced import onto an existing command, a call before the import, an export written after the import, `export -clear`. Feeds go-to-definition and the linked references view.
- **A `-force` import is followed to the deleted local** (`forced-import-shadow-resolver-disagree`, 10 tests): definition, hover, signature help, inlay hints, call hierarchy and references land on the local proc that `namespace import -force` deleted; tclsh runs the imported body.
- **Import ordering inside bodies** (3 definition tests): `namespace forget` in the same body, an export in the importing body itself.
- **An inner namespaced class does not shadow the global one**: inside `::a`, `C cm` goes to `::C` (`tcloo_navigation::tn_an_inner_namespaced_class_shadows_the_global_one`).
- **Property code lens** counts `my size` as two references; on 9.0.4 and 9.1.0 `my size` raises `unknown method "size"` (the accessors are `<ReadProp-size>`), and `property` needs `oo::configurable`.

Unconfirmed, need one run: the member hazard inventory may miss `export m` inside `if {1} {...}` in a definition script (a proof consumer); `argument_mutation_withdraws_all_external_member_identity` may still answer the original `method ping` where tclsh runs the redefinition.

## Tests that demand a wrong answer

These pass a wrong edit if made green as written; each row is `FIX-EXPECTATION` with the counter-example: `brace_expr` on `expr $a + $b` (with `a={1 + 1}` the braced form raises); three `inline_proc` expectations that read the caller's variable where the procedure read its own unset local; eight `switch_to_dict` edits that raise `key not known in dictionary` on an unmatched subject; direct procedure calls inside an iRule (IRULE5005: `call`); renaming `method make` beside a 9.x `classmethod make` that replaced it (three tests); a colon variable `$:` that is literal text; a wildcard-imported call left behind; `property` read through `my`; two dynamic renames (`issue1281`, `issue945`) that leave a list-built or `gets`-fed name stale; a shadowed-duplicate rename that changes ONE/TWO; a Tk `bind` script treated as namespace-local (it runs global).

## Distinct failing behaviours

Proof tier (rename, refactors, code actions):

1. Command rename under pinned releases returns nothing or refuses safe renames (unknown completed command world, changed publication, incomplete coverage), single- and cross-document.
2. Member rename refuses safe renames: unknown roster, `$class create` receivers, renames beside `renamemethod`, a fully tracked member ("unreferenced edit"), unclosed cross-file class-side families.
3. Refusal reasons are generic where the tests and the rename feature page want the specific cause (computed name, untracked receiver, interpreter error, call frame); some refusals arrive as no answer.
4. The namespace-variable gate counts every dynamic name and alias obligation in a document (owner ruling A amends this: a computed name is a barrier only when it could spell the old or new name).
5. Namespace rename edits the whole word instead of the written component; refuses a literal `namespace path` entry tclsh shows safe; returns nothing from the segment of a qualified call.
6. The coverage gate refuses any document with a computed command head, even one that cannot spell either name.
7. Linked editing gives no ranges for a self-recursive procedure (`tcl`, pinned with a resolved input, Jim).
8. `if_to_switch` reports an incomplete visit where no trace is possible (ruled: the proof doc's Conclusion governs).
9. Package actions withdraw on an unresolved namespaced head; the census answers None where the contract says Some(false).
10. Extraction under 9.0 is refused where tclsh shows the extracted procedure equivalent; extract-variable refuses `café` and command-substitution words under `tcl`.
11. Refactors do not enter `when`, `apply` lambda or `eval`-alias bodies.

Navigation and references:

12. Calls written in bodies the analysis did not enter are missing from call hierarchy and member references (waiting on the entry-independent binding query).
13. A class made by a user metaclass publishes no factory, so `$w go` does not navigate.
14. Wildcard imports: re-exported chains, an `oo::class` reached through one, cross-document callers, export and import ordered by `source`.
15. References miss: unbraced `uplevel` bodies, `if {1} $cb` and `eval $cmd` with constant values, list-built trace and `namespace code` callbacks, `interp alias` target words, a command literal held in a variable or array (and renaming it refuses), `my` in a `switch` arm or a separate `oo::define`, `[self] m`, class `variable` names, `self method` and `classmethod` call sites.
16. Literal `foreach` loops: installed methods and procedures, the rename-and-reinstall idiom, the second varlist of a multi-list `foreach`.
17. Surrogate-escaped class names (`C\uD800`) get no original class publication.
18. Under `tcl`: the cross-document hierarchy answers None, implementation misses descendants, cross-document instance methods and command renames have no world (class-facts is fixing the walk; the record goes in the Logical domain).
19. After a resolved `source` under 8.6 the server's source-view routes answer nothing (class hover card, references).
20. Per-object `export`/`unexport` and class-side `self unexport` are mishandled by completion and navigation.

Display tier:

21. Hosted iRules and iApps procedures get native declarations (completion inserts `::helper`, outline labels).
22. Collection receivers (`[dict get ...]`, `dict for`, `foreach`) get no method token; cross-document named instances neither.
23. Hover misses: `link` aliases, embedded languages (glob in a method body, a quoted switch action through an alias, `format` inside `regexp`), a format string through a constant, the **Requires** line, the Tk ensemble splice, a superclass argument.
24. Under `tcl`, inside uncalled procedures, inferred-type hover and inlay hints are missing (facts that hold for every entry).
25. Minifier: switch case lists and arm bodies, nested `expr` operands, unbracing, quote-dropping, De Morgan, keyword abbreviation all wait on a receipt (tclsh shows each equivalent under its condition; bytes in `docs/work/name-handling/outstanding/minifier-name-shortening.md`).
26. Two iRules rows go to the BIG-IP checks: variables set in a `when` handler missing from the outline, and `$tcl_platform` hover.

## Fixture faults, by kind

`pinned-release-cross-document-class-withdrawn` and `cross-file-resolution-pinned-release` (42): the fixture names a release and expects facts from a document nothing sources; the remedy is the lenient `tcl` environment or a proved `source`, assertions unchanged. `uncalled-procedure-facts` (30, mostly rename through a `[self]`/`my` callback in a method never called): call the method. Then missing `package require` (Tk 13, opt, snit, msgcat), `classmethod` analysed under 8.6, a bare `mystery` command that raises before the subject (the #2427 ruling bounds it), profile-less registries, unset variables, a second `create` of an existing object, `link` under 8.6.

## The three `preview_tickets_e2e` tests

Red since the naming restructuring: their bodies are unchanged since `origin/rust` `3475cff4`, where CI runs every server target; the 16 and 17 November lists covered only the `lib` and `e2e` targets, which is why they were missing. `class_member_bareword_call_requires_link_e2e` is `FIX-FIXTURE` (`link` exists from 9.0; its documents open under 8.6). The two pack tests are `KEEP-CONTRACT` (`docs/design/compiler/registry-consumer-contracts.md` names this file as their holder).

## Copies and overlap

- Exact copies: `type_definition::tests::variable_jumps_to_inferred_class` and `declaration::tests::upvar_local_alias_is_a_declaration` fold into their `it_5` `lsp_providers` twins; `cross_file_method_definition_jumps_to_inherited_declaration` into `goto_definition_follows_inherited_method_cross_file`; one receipt test into `composed_callback_prefix_references_keep_public_and_private_original_selectors`.
- About fifteen core tests hold the same observable on the same source as an end-to-end test in the routed set (`foreach` reinstall, multi-list `foreach`, the Tk splice, unbraced `uplevel`, `my` in a `switch` arm, the moved-member renames, the forced-import shadow). The end-to-end test holds the protocol observable and is the survivor; the merges wait until both pass, per the method.
- Near copies: the two lens tests on `issue_864`, the three uses of the `if {1} $body` coverage check, the `rename_inherited_method_*` pair.
- Vacuous passes for the second pass: negative `member_targets` tests (the helper answers None for every pinned input), `a_receiver_that_unexports_the_inherited_name_abstains_both_ways`, `an_export_sourced_after_the_import_does_not_resolve_end_to_end`, `check_actions_shimmer_noqa_suppress_honours_disabled_codes`.
- Dead code: `fold_static_substrings` (test-only); `documents_in_namespace` has no production caller.
- Stale documentation: `docs/kcs/features/kcs-feature-rename.md` says namespace rename is unsupported; the "Source-action obligation boundary" section of `refactor-original-source-rewrite-permissions.md` (ruled stale; amendment pending).

## Targets the failing lists never covered

The 16 and 17 November lists were taken from the library and `e2e` targets only, so the scoreboard undercounts. Never listed: the `tcl-lsp-core` integration targets `it_2`, `it_3`, `it_4` and `it_5` (a run on 17 November found 43 failing in the rename, references and command-table groups alone, all with empty answers; they are to be triaged as a further part of this ledger), and the `tcl-lsp-server` targets `preview_tickets_e2e` (three failing, triaged above), `smoke` and `stdio_deadlock` (not yet measured).

## Rulings on the UNDECIDED rows (coordinator, 17 November)

Applied in the ledger: call-hierarchy procedure items are labelled by qualified name; Go to Definition follows the display policy after an unbounded `$cmd` head; Definition lists only the sites that create the variable; inline needs a proved target, so an unsourced sibling's shadow refuses both ways; a mathfunc rename that returns no edits and no reason is a wrong answer of its own (`wrong-rename-returns-no-edits-no-reason`); an unsourced workspace `rename regexp {}` is no known shadow under a pinned release. The unbound `$` read (`navigation::an_unbound_dollar_read_never_resolves_to_a_same_named_method`) was run: its references are only the read itself, never the method, so the row is `FIX-EXPECTATION`.
