# Luna-3 source review of Sol fix-1..fix-5 (call hierarchy / code lens)

Scope: `/tmp/.../agents/sol-callhier/fix-{1..5}.patch` against repo `work`. The checkout HEAD is
`4c3eb7953`, not `2a3928bc6`. `git diff --stat 2a3928bc6 HEAD` touches only four `docs/work/...` files,
so the Rust source is identical to `2a3928bc6`. Evidence:
`cp09/partial-lsp35r2/original-tcl_lsp_core/result.json` (664 selected, 525 pass / 139 fail) and the
`original-tcl_lsp_server` result. Nothing was built or run, so every flip below is a source-level
prediction.

## Application and formatting

The four affected files were copied to `luna-3/mini` (baseline) and `luna-3/seq2` (patched in the
order 1, 3, 2, 4, 5).
- `git apply --check` succeeds for each patch alone. `patch` applies 1→3→2→4→5 cleanly with no fuzz.
  fix-3 alone also applies, at a −7 line offset.
- `rustfmt --edition 2024` (stdin, scratch copies only): every baseline file is clean.
  After patching, `source_structure.rs`, `original_declaration.rs` and `original_call_hierarchy.rs` are
  clean. **`handlers.rs` (fix-2) is not**: rustfmt re-wraps the new `or_else` chain at
  handlers.rs:6360-6365, so `make rust-check` would fail until it is formatted. The diff is in
  `luna-3/fmt/all-handlers.rs.diff`.

## API and compile check (all verified at HEAD)

| API used | Location | Result |
|---|---|---|
| `OriginalSourceDefinitionMemberRegion::original_command_words_at(u32) -> Option<&[NativeWord]>` | tcl-compiler/src/registry_invocation/source_scoped_body.rs:1034 | pub, ok |
| `script_body_disposition(&ContextRegistry, &[NativeWord]) -> Disposition` | source_scoped_body.rs:1056 | pub, ok |
| `OriginalSourceDefinitionMemberDisposition::{CompleteNonMember, Unavailable, CompleteMember(Vec<..>)}` | source_scoped_body.rs:992-999 | exact, exhaustive match ok |
| `OriginalSourceDefinitionMemberScriptBody::content_span()` / `definition_parent()` | source_scoped_body.rs:868 / 878 | const pub, ok. The queue element type `(Span,u32,Option<OriginalSourceScriptBody>)` matches |
| `SourceScriptContext.definition_members` / `.definition_parent` / `.depth` | tcl-lsp-core/src/source_structure.rs:1036-1043 | fields exist |
| `TokenType::Sep` / `Eol`, `Token.kind` | tcl-lexer/src/tokens.rs:45-57, 271 | ok |
| `ResolvedInvocation::authored_source_argument_roles() -> (Vec<(u8,ArgRole)>, bool)` | tcl-registry/src/resolved_invocation.rs:3582 | pub; `.1` is the completeness bool |
| `CommandSpec::definition_body: Option<&'static DefinitionBodyGrammar>` | tcl-registry/src/spec.rs:2481 | Copy, so `?` inside the closure is ok |
| `DefinitionBodyGrammar::is_member(&str)` | tcl-registry/src/definer.rs:3835 | `&String` deref-coerces |
| `ResolvedInvocation::arg_roles() -> Option<Vec<(usize,ArgRole)>>` | resolved_invocation.rs:3120 | ok, **but returns None for unknown subcommands (see fix-3)** |
| `ArgRole::CommandPrefix`, `ArgRole::OpaqueScript` | tcl-registry/src/arg_role.rs:80,147 | exist |
| `original_source_self_method_references`, `..._property_references` | tcl-compiler/.../original_metadata/source_method_reference.rs:308, :141 | pub |
| `OriginalSourceSelfMethodReference::{body_class, body_method, declaring_class, method, selector_span, command_span}` | source_method_reference.rs:220-292 | pub; types match `for_method` |
| `OriginalDeclarationIdentity::for_method(&str,&str,&AnalysisResult,&SourceDeclarationMetadata<ClassDef>,&OriginalSourceMethodMetadata)` | tcl-lsp-core/src/original_declaration.rs:230 | ok |
| `OriginalSourceProcedureCall::uncertain_operations()` / `has_lookup_operand_effects()` | tcl-compiler/.../source_procedure.rs:133 / 95 | pub, ok |
| `if a && let ...` chains | workspace `edition = "2024"` (Cargo.toml:162) | ok |

`use … as D;` sits first in its block, so clippy's `items_after_statements` does not fire. fix-1's new
`.package_census.as_mut().unwrap()` follows the established invariant in `census_command_bodies`: the
CheckedSyntax purpose always sets `package_census`. No new `#[allow]`, no hardcoded command names in
code, and no test, selector or fixture is touched.

## fix-1: HOLD (diagnosis confirmed; as a standalone patch it opens a fail-open census path)

Both diagnosed defects are real:
- **Global head lookup in definition bodies.** `census_command_bodies` (source_structure.rs:1582-1607)
  calls the global `source_registry_words` for every command, including class-body members. Member
  bodies are never queued, and the head is recorded `unavailable`/`syntax_unavailable`. As a result,
  `SourceSyntaxCoverage::applicability` (:389-407) never returns Complete for a class document. The
  readonly purpose already routes members through `script_body_disposition`
  (`readonly_command_bodies` :1328-1371), and fix-1 mirrors it. The executable walker enters member
  bodies at the same `(content_span, depth+1)` (executable_regions.rs:470-487), so region and command
  coverage line up.
- **Separators in `member_matches`.** `SegmentedCommand::all_tokens` keeps inter-word `Sep` tokens
  (parsing/syntax/segment.rs:121-138). `NativeWord::tokens()` holds content tokens only
  (tcl-lexer/src/native_word.rs:148-152). The equality at original_declaration.rs:1025 can therefore
  never hold, so every member command became `uncertain`. Filtering `Sep|Eol` is correct. An `{*}`
  expanded member still mismatches because the `Expand` token stays in `all_tokens`, which is
  fail-closed.

Findings:
- **F1-a (blocking, rules: no absence-as-completeness).** fix-1 makes `complete_property_source_occurrences`
  (original_declaration.rs:977-1059) reachable for the first time; at HEAD it can never complete. It
  then issues `CompleteSourceOccurrenceCensus`, with exact titles including "0 references", while known
  unrepresented dispatch shapes remain:
  - registry `CommandPrefix`/`OpaqueScript` callbacks, e.g. `trace add variable v write [list my size]`
    or `namespace code {my size}`. The walker sees only `list …`, which classifies as `Some(false)`.
  - definition members that dispatch later, e.g. `forward f my size`. Members take the
    `member_matches → Some(false)` arm without inspecting member reference roles.

  Sol's REPORT itself says this hole is "guarded in fix-3", and fix-3's guard is broken (below). fix-1
  must not land without a correct guard.
- **F1-b (should fix).** The new block runs only when
  `definition_members.and_then(original_command_words_at)` is `Some`. When `definition_parent.is_some()`
  but the member region or the argv join is missing, it falls through to the global head lookup. Its
  own comment says "Unavailable member geometry stays incomplete", and the readonly twin returns
  early (source_structure.rs:1348-1356). Push `unavailable` and `return` in that case.
- Minor: member bodies are not added to `scoped_environment_regions`. This is harmless, because that
  field holds `OriginalSourceScriptBody` only, and the behaviour is unchanged from HEAD.

Expected fail→pass:
- `code_lens::tests::lens_reports_zero_for_unused_property` (060)
- `code_lens::tests::lens_counts_property_my_dispatch_within_class_body` (059). Medium confidence: it
  needs the `property -get/-set` bodies to pass `checked_script_bodies`.
- Not captured: `readonly_property_reports_promote_only_a_complete_typed_source_census` up to :1757,
  with the separate :1823 cause.
- Possible but unverified: `call_hierarchy::logical_source::tests::genuine_own_member_frame_…` (500),
  because method-body commands now appear in `SourceSyntaxCoverage::commands`. Also the `rename_safety`
  `fn_guard_allows_*` / `tn_allows_*` class tests, which are not in the capture.

Pass→fail risk: none found among the captured passing tests. Method lens coverage is still hard-coded
Partial, so test-096 and the server `readonly_member_wire_retains_owner_and_partial_coverage` are
unaffected. The following uncaptured consumers can change verdicts on class documents, from refusal
to allow, and need a run:
- `rename_safety`
- `namespace_rename` hazard
- the minify/lexical census
- extract/inline variable
- `receiver_identity`
- `references.rs:4321`

## fix-2: ADMIT (source level), after a mechanical rustfmt

Diagnosis confirmed. `authored_source_definition_body_grammar` (resolved_invocation.rs:5468-5497)
returns `None` for a Configure-target descriptor unless argv[target+1] carries the `Body` role. The
inline form `oo::objdefine obj method p {} {}` has no such role, so `handle_oo_objdefine_at`
(handlers.rs:6352-6363) returns `true` before `retain_original_class_configuration_metadata`, and no
object configuration is recorded. The pre-`ab88e9f0d` handler had no such gate. The fallback is pure
registry data (`definition_body`, `is_member`) under complete selected roles, and it reuses the
handler's existing `args.get(1)` member test (:6366). Dynamic or non-member keywords still decline.
- Must fix: the rustfmt drift at handlers.rs:6360-6365.
- Optional: a registry-owned accessor for the inline-member grammar would keep the selection in
  `tcl-registry` rather than the analyser.

Expected fail→pass:
- `original_declaration::own_object_identity_tests::original_own_object_identity_keeps_uri_currency_and_wire_reselection` (413)
- `code_lens::native_lens_presentation_tests::native_own_object_method_lens_keeps_identity_when_class_display_key_is_unavailable` (503)

Both currently panic at `original_object_configurations().next().unwrap()`.
`original_own_object_calls_and_references_keep_the_canonical_object_worker` (459) may still fail at
prepare :926 (REPORT G9). Pass→fail: low, because this restores pre-regression behaviour. The compiler
crate's `oo::objdefine` inline tests were not in the capture.

## fix-3: HOLD (blocking defect plus an unresolvable test conflict)

- **F3-a (blocking).** `schema.arg_roles().is_none_or(…)` marks `my <method>` itself as deferred. The
  `my` spec has `SUBCOMMANDS = [variable]` and `allow_unknown_subcommands: true`
  (oo_my.rs:58, :145). `resolve_semantic_subcommand` returns `Unknown` for `ping`/`size`
  (registry.rs:9382), and `ResolvedInvocation::arg_roles` returns `None` for `Unknown`
  (resolved_invocation.rs:3123-3130). Every census containing `my X` therefore sets
  `uncertain = true`:
  - the property census never completes again, which reverts fix-1's gain on 059 and the admission26
    control at :1757;
  - the method census cannot complete for `lens_counts_method_calls_within_class_body` (056).

  Self-dispatch commands must be exempt from the `None` arm; their dispatch is already reconciled by
  `walked == recorded`.
- **F3-b (blocking, test conflict that needs escalation).** Promoting method coverage at all conflicts
  with currently passing tests on the same source shape:
  - `original_declaration::readonly_source_reference_tests::readonly_member_reports_keep_source_spans_out_of_edit_inventories`
    (test-096, PASS) asserts `Partial` (original_declaration.rs:1691) for
    `oo::class create C {method ping {} {}; method body {} {my ping; my ping}}`;
  - server `readonly_reference_reports::tests::readonly_member_wire_retains_owner_and_partial_coverage`
    (PASS) asserts `"partial"` and "at least 2 references" (readonly_reference_reports.rs:2017, 2098,
    2136) on the identical source.

  `lens_counts_method_calls_within_class_body` wants "2 references" on an equivalent class. The
  contract doc `original-method-candidates.md:18` also states that the method self-selector facet is
  partial-only. Any working fix-3 flips one of these sets, and no source change satisfies both.
  As written, F3-a hides the conflict by accident.
- **F3-c.** Even with F3-a corrected, members that reference methods (`filter greet`,
  `forward f my greet`) take the `Some(false)` arm. A method `CompleteSourceOccurrenceCensus` would
  then claim absence while missing them.

Predicted effect as written, on top of fix-1:
- fail→pass: `lens_reports_zero_for_uncalled_method` (058) only.
- 059 and the property control revert to fail compared with fix-1 alone.
- 056 and 057 stay failing.
- test-096 and the server partial-wire test keep passing only because of F3-a.

The refactor of `complete_property_source_occurrences` into a bool census otherwise preserves every
`None` path.

## fix-4: ADMIT (source level), with a non-blocking policy note

Diagnosis confirmed. Native `edges()` (original_call_hierarchy.rs:487-583) has no producer for direct
self selectors. The method-symbol queries produce nothing for `my greet`: incoming is `[]` in the 546
and 561 logs. fix-4 joins the compiler's catalogue rows (`body_class`/`body_method` → caller,
`declaring_class`/`method` → target) to source declaration identities. This is the same conditional
source-relation precedent as `source_procedure_edges`, with no Native identity grant and no hardcoded
name. Only instance `TclOoMethod` bodies produce rows (source_method_reference.rs:416-427,
`original_body_owner`), so constructor and destructor frames are untouched.
- Note: this is inconsistent with fix-5. fix-5 withholds procedure edges that carry earlier unknown
  mutations, while fix-4 ignores `reference.source_schema().uncertain_operations()`
  (source_transition_advice.rs:538), so `method m {} {mystery; my greet}` still issues an edge. Decide
  explicitly before adding a check: a preceding `my greet` may itself count as an uncertain operation
  and would then drop the second range that 546/561 require.
- Note: the obligations on inheritance, lookup and receiver are dropped silently, because the native
  API has no debt channel. This is the same as the existing method-symbol edges.

Expected fail→pass:
- `call_hierarchy::tests::incoming_calls_for_method_grouped_by_caller_method` (546)
- `outgoing_calls_from_method_to_sibling_method` (561)
- likely `incoming_calls_for_method_nested_in_control_flow` (547) and
  `outgoing_calls_from_method_nested_in_control_flow` (560), if the member catalogue reaches the nested
  `if`/`switch` bodies (not verified)

Not fixed: 539/540 (callbacks), 562 (method→proc), and the classmethod tests (G6). Pass→fail: none
found. `incoming_calls_excludes_bare_head_that_is_not_valid_tcl` and
`unresolved_method_outgoing_calls_excludes_my_keyword` are unaffected.

## fix-5: ADMIT

Diagnosis confirmed for
`original_call_fallback_does_not_restore_deleted_or_unknown_occupied_headers` (451, failing at :1037
on `proc caller {} {mystery; ::target value}`):
- `procedure_target` already declines (the :1023 assertion passes);
- `references()` uses `procedure_target` only;
- the remaining incoming edge comes from `source_procedure_edges`;
- that call carries `graph.uncertain_operations` (source_procedure.rs:449-452).

Skipping calls with retained debts is fail-closed and withholds positives only. Side effect:
`has_lookup_operand_effects` counts any non-text operand part, including `$var`
(body_effects.rs:147-169). So same-document, source-only edges (`resolved_command_reference == None`)
with variable or substitution operands are also withheld. No captured passing test relies on this:
the passing `call_hierarchy::tests` sources resolve through `procedure_target`.

Expected fail→pass: 451. Pass→fail: none found.

## Bottom line

- ADMIT: fix-2 (after rustfmt), fix-4, fix-5.
- HOLD fix-1 until it ships with a corrected deferred-callback guard and fails closed on missing
  member geometry.
- HOLD fix-3:
  - the `my` arg-roles bug inverts its intent;
  - method census promotion contradicts passing test-096 and the passing server partial-wire test;
  - escalate the G3a conflict, since it cannot be resolved without a test or contract change.
