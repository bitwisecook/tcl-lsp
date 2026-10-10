# naming.cli.original-source-highlight-geometry

Kind: `implementation-contract`

## Problem statement

Source highlighting needs actual selected Body versus data geometry, including aliases, captured prefixes, shadows and custom syntax-only schemas. A newline/semicolon brace heuristic cannot supply that distinction or preserve the actual source grammar and availability.

## Question

How do ANSI and HTML highlighting retain actual full input/context, original lexical spans and genuine selected Syntax script regions without converting data or ReferenceOnly syntax into execution authority?

## Conclusion

The CLI retains one actual standalone input/context, project command store and complete grammar, then captures the readonly SourceSyntaxStructure from the authentic AnalysisResult. Original Syntax commands and actual effective schema operand ordinals feed one shared HighlightPlan. SourceSyntaxStructure::lexical_regions returns SourceSyntaxRegion::span and tokens: each independently selected original script interior owns selected-grammar token spans in the complete-document address space. Lexer source_region_tokens_in shares checked region slicing and rebasing with the original word-region owner; inner leading BOM remains source content. ANSI and HTML consume those retained tokens and the same intervals, preserving original source bytes or their explicit HTML escaping without child input recapture or consumer offset arithmetic. Shared IfWalk compares exact known non-NUL byte controls with ASCII then/elseif/else; opaque and C modified-UTF8 payloads remain genuine source words, while unknown, expanded or unresolved counted-NUL controls decline clause selection. Genuine single-line bodies, data, aliases/captured prefixes, known shadows and custom ReferenceOnly roles keep their own original selection. Missing/stale image, changed lexer configuration or missing actual input refuses capture. The capture, commands, script_regions and lexical_regions APIs supply syntax only, with no execution, installed Native binding, entered frame, physical header or editable naming authority.

## Scope

Eight marked fixed source/API controls cover body/data distinction, HTML/ANSI geometry and escaping, selected alias-prefix/shadow roles, custom ReferenceOnly schemas and exact refusal, C/Jim/iRules lexical axes, whole-document nested token offsets with Unicode source, opaque/counting source bytes and checked invalid-region refusal, plus non-NUL original IfWalk controls. The linked CLI project spec-pack integration selector is distinct from these source controls. No executed Rust, rendering or Native provider result is supplied by this record; all seven providers remain not tested.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-cli-original-source-highlight-geometry-highlight.rs` (implementation): [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs). SHA-256 `8cab93e4cf53333545e71dfbeae9cb6a3543825f0964b2f2a4e6f2a2e66e6156`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-cli-original-source-highlight-geometry-source_structure.rs` (implementation): [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs). SHA-256 `77aee56bb58e3db03487a680167c745efa20c5ef17021a0108e864f0f03670a7`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceSyntaxStructure`: Capture authentic current AnalysisResult Syntax geometry and expose readonly command/script spans without execution authority.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `HighlightPlan::capture`: Combine actual selected source schema/effective operand ordinals and original lexer styling into the shared ANSI/HTML interval plan.
- [rust/tcl-lexer/src/native_script_words.rs](../../../../rust/tcl-lexer/src/native_script_words.rs), `source_region_tokens_in`: Slice one genuine original region using the selected lexer grammar and share checked whole-image token rebasing; no consumer-local offset reconstruction or execution authority.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceSyntaxRegion`: Retain original independently selected script interior and readonly lexical tokens in complete-document coordinates.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceSyntaxStructure::lexical_regions`: Expose already joined selected-region lexical geometry to highlighting without child recapture or offset arithmetic.
- [rust/tcl-registry/src/commands/tcl/if_.rs](../../../../rust/tcl-registry/src/commands/tcl/if_.rs), `walk_if_arguments`: Delegate original InvocationArguments clause layout and exact keyword selection to the shared ClauseGrammarSpec; missing or unresolved operands supply no Native branch execution grant.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `highlight::tests::original_highlight_keeps_data_inert_and_single_line_bodies_visible` (linked): Genuine data remains unstyled as commands while selected single-line script bodies retain command spans; ANSI plain source bytes remain exact.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `highlight::tests::original_highlight_html_shares_script_regions_and_source_escaping` (linked): HTML and ANSI share the selected script-region plan; data stays data and Unicode/angle/ampersand source bytes receive explicit HTML escaping.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `highlight::tests::original_highlight_keeps_selected_alias_prefix_and_shadow_roles` (linked): Actual selected direct/moved/alias/bound-prefix source roles retain body geometry while known shadow/replacement refuses the borrowed body schema.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `highlight::tests::original_highlight_keeps_actual_custom_and_reference_only_source_schemas` (linked): Authentic custom and ReferenceOnly Syntax schemas are retained without execution; stale full source, changed config and missing input withhold the capture.
- [rust/tcl-cli-support/src/highlight.rs](../../../../rust/tcl-cli-support/src/highlight.rs), `highlight::tests::original_highlight_uses_actual_c_jim_and_irules_lexical_axes` (linked): Actual full C8.4 through C9.1, Jim and iRules input selects its own lexical axes and original Unicode/source spans, without a default-profile fallback.
- [rust/tcl-lexer/src/native_script_words.rs](../../../../rust/tcl-lexer/src/native_script_words.rs), `native_script_words::tests::original_region_tokens_keep_global_offsets_and_opaque_source_bytes` (linked): Genuine native-byte source region retains global offsets, exact opaque/counting payload and comment spans even with nonzero input base; invalid region refuses. This is lexical geometry without Native command/read/frame admission.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `source_structure::tests::original_syntax_tokens_join_absolute_region_and_command_geometry` (linked): Authentic complete C/Jim analyses join selected body/nested bracket tokens to whole-document command spans, preserve Unicode boundaries, exclude genuine data and refuse stale source; no executed provider or binding is inferred.
- [rust/tcl-registry/src/commands/tcl/if_.rs](../../../../rust/tcl-registry/src/commands/tcl/if_.rs), `commands::tcl::if_::original_clause_repair_tests::original_if_roles_keep_opaque_payloads_and_decline_unknown_controls` (linked): Shared IfWalk retains exact Expr/Body slots for known opaque and C modified-UTF8 non-NUL payloads and exact trailing-word repair; dynamic, expanded and counted-NUL grammar controls refuse. Supplied source values establish no Native argv or entered handler.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
