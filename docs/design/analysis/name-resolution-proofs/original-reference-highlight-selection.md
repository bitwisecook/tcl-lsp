# naming.editor.original-reference-highlight-selection

Kind: `implementation-contract`

## Problem statement

Reference and highlight providers can drop opaque declarations after display-map changes or reconstruct a different target from a reported qualified name. Reusing an Analysis result after the whole source changes can also project stale locations into a new document.

## Question

Do references and document highlights share actual original source selection and complete currency while keeping String method helpers restricted to independent lexical advice?

## Conclusion

References and document highlights use the shared original namespace, variable, method and declaration selectors before any explicitly selected lexical compatibility. Genuine byte/policy declaration identity and actual call edges survive erased reporting names, and complete source/configuration changes withdraw the original query. Method helpers accepting qualified String names refuse Native analysis; a genuine typed member remains independently available through the canonical method candidate. Highlight projection deduplicates only the selected source locations and supplies readonly Text/Read/Write presentation. It creates no original input, live cell, entered receiver, native dispatch or edit permission.

## Scope

Current Core references/highlight entry points, shared original selectors and public String method-reference compatibility boundaries. Fixed assertions distinguish two opaque source declarations, erased UI maps/names, an actual selected call, stale whole source and a genuine typed method from nominal String helper queries. All seven native providers are not-tested; no executed Rust receipt is attached.

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

- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `references_in_program`: Use current original source selectors and canonical declaration/call identity before explicitly selected lexical reference compatibility.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `document_highlights_in_program`: Share actual original selection and whole-source currency before projecting readonly highlight locations.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `original_text_highlights`: Project the selected canonical source references as text highlights without reparsing report names.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `select_at_offset`: Select genuine current namespace facts independently of optional reporting labels.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `select`: Select genuine current original variable roots and source symbols independently of runtime cell authority.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `local_candidate`: Retain genuine queried selector/receiver purpose and independently selected canonical source member.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `invocation_targets_declaration`: Match the authentic original lookup and target declaration allocation under current source/configuration.
- [rust/tcl-compiler/src/analyser/class_hierarchy.rs](../../../../rust/tcl-compiler/src/analyser/class_hierarchy.rs), `ClassHierarchy::declared_member`: The explicit lexical compatibility branch shares the actual declared class/member/side selector and compares retained declaration metadata and span; Native queries remain terminal.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `references::original_highlight_selection_tests::original_highlights_preserve_opaque_declarations_and_whole_source_currency` (linked): Two distinct opaque declarations retain only the selected declaration/call reference and highlight set after reporting maps/names are cleared; stale full source withdraws both providers.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `references::original_highlight_selection_tests::original_method_reference_helpers_do_not_select_from_reporting_names` (linked): A genuine typed member retains its readonly highlight while QName method, next-dispatch and object-call helpers decline Native report-name queries. The tuple compatibility helper preserves Native refusal and the explicit lexical branch selects the same retained class/member metadata and side through the common hierarchy owner.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source-consumer assertions. No Rust execution receipt or native experiment is attached. Whole-image/configuration/Registry currency and independently required purpose permissions remain mandatory.
