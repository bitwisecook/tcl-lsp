# naming.editor.original-diagnostic-edit-currency

Kind: `implementation-contract`

## Problem statement

A diagnostic span or suggested replacement cannot authenticate the current document. Equal offsets can belong to changed source, another full lexer configuration or a different Registry generation; rendered diagnostic prose cannot repair those missing inputs.

## Question

Which independently retained source inputs must agree before primary diagnostic fixes, compiler-check fixes and bulk fixes can be offered for the current document?

## Conclusion

DiagnosticEditSource borrows the actual current document and AnalysisResult and reuses WorkspaceDiagnosticSourceContext for whole Document image, full lexer configuration and Registry semantic currency. Primary code actions require this guard before any range action; published analyser fixes also match an independently present current code/span/fix, while messages and titles remain presentation. Compiler and DB diagnostic aggregates retain DiagnosticSourceContext after whole-document span rebasing; check_diagnostic_actions requires each issuer context to match the current guard before lifting fixes or emitting a suppression edit. Every edit extent must be in bounds and lie on actual UTF8 byte boundaries, including end-of-file insertions. Server bulk selection captures the guard on each freshly analysed source iteration and preserves each fix’s independent safety classification and non-overlap rules. The current Server action request uses the Registry retained by its actual analysis. Currency supplies no Native name, handler, cell, value, evaluation, equivalence or edit permission; those remain the fix issuer’s separate obligations.

Compiler diagnostic edit currency retains the issuer complete actual Module ResolvedAnalysisInput, not only equal image/config/command-store labels. Generic fixes and typed context actions compare that input and actual availability generation with the current analysis. Missing Module input cannot issue source context; a same-store older availability request refuses even when original source geometry remains equal.

## Scope

Document source edits and advice under actual retained Analysis source/config/Registry. SourceText and UTF8 edit geometry do not become native input, runtime identity or execution authority. The diagnostic title/message may change without changing the owned edit. Missing whole-source correspondence, missing or foreign compiler issuer context, unowned published fixes, invalid bounds and byte-interior UTF8 positions decline edits. Independently selected host filtering still decides which owned diagnostics are published. Broader structured diagnostic subjects remain separate from this source-currency contract.

The two current source/API controls establish issuer correspondence only. Original fixes and typed subjects still supply their independent editing purposes; source currency grants no Native frame/Normal/effect, safe evaluation, rewrite equivalence or executed assertion result.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `DiagnosticEditSource::for_analysis`: Borrow exact current Document source and actual Analysis full configuration/Registry through the existing shared readonly source context; supply no semantic grant.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `code_actions_in_program`: Require current whole-source correspondence before primary range actions and independent current analyser code/span/fix correspondence for published quick fixes.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `check_diagnostic_actions`: Require independently retained matching compiler issuer source/config/Registry before fix lifting or suppression edits.
- [rust/tcl-compiler/src/compiler_checks.rs](../../../../rust/tcl-compiler/src/compiler_checks.rs), `retain_diagnostic_source_context`: Retain whole original module source/config/Registry at final diagnostic coordinates; foreign lowering stores withdraw this currency.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `compiler_check_diagnostics`: Attach original issuer currency after memoised function diagnostics have their final whole-document spans.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `Backend::bulk_applicable_fixes`: Select only current independently owned analyser fixes with valid source geometry, preserving each actual safety classification and overlap checks.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspaceDiagnosticSourceContext::for_analysis`: Shared readonly complete source/config/Registry invalidation owner; diagnostic edits reuse it without issuing lookup or execution permission.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `code_actions::diagnostic_currency::tests::original_diagnostic_actions_require_current_whole_source_and_owned_fixes` (linked): A genuine E201 comment-boundary analyser warning retains its exact zero-width insertion at byte10 and closing bracket text on current source; changed whole source, changed configuration, absent analysis and foreign fixes withdraw actions. Changed diagnostic presentation preserves the independently owned edit.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `code_actions::diagnostic_currency::tests::original_check_actions_require_the_issuers_source_config_and_registry` (linked): A genuine current S100 warning supplies the exact line-one zero-width noqa edit; independently changed source/config/Registry and missing issuer provenance decline actions, while changed prose preserves the edit.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `code_actions::diagnostic_currency::tests::original_diagnostic_source_extents_require_actual_byte_boundaries_and_channel` (linked): A genuine Document emoji extent and end-of-file insertion are valid; UTF8-interior/out-of-bounds positions decline, and identical Native bytes do not match the Document source owner.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `DiagnosticEditSource::matches_compiler_diagnostic`: Compare the original issuer full actual analysis input and availability generation with current analysis, alongside independent image/config/Registry/span/fix currency.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_context.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_context.rs), `ContextDiagnosticData::matches`: Require the typed original subject and complete issuer input to match current source/config/store/generation before a context action; displayed prose does not select the owner.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_currency.rs), `code_actions::diagnostic_currency::tests::original_check_currency_refuses_same_store_changed_availability_and_missing_input` (linked): Generic compiler fix currency requires the issuer complete actual Module input; same command store and identical image/config with older availability still refuse, and missing Module input cannot issue compiler source context.
- [rust/tcl-lsp-core/src/code_actions/diagnostic_context.rs](../../../../rust/tcl-lsp-core/src/code_actions/diagnostic_context.rs), `code_actions::diagnostic_context::tests::original_context_diagnostic_currency_keeps_the_complete_availability_owner` (linked): Typed diagnostic context actions retain complete issuer input/availability independently of original source/config/store equality; a same-store older availability request supplies no action despite identical original source geometry.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

The exact linked Rust selectors exercise source currency and edit geometry. No Rust execution receipt or native provider observation is attached to this implementation contract.
