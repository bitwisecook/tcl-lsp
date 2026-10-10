# naming.consumer.original-mcp-refactor-context

Kind: `implementation-contract`

## Problem statement

A refactoring tool can bypass editor purpose guards by forcing iRules for a Tcl request or reconstructing parser configuration and builtin identity from a dialect label. A shadowed builtin spelling then risks selecting the wrong transformation.

## Question

Do MCP refactor requests preserve their actual requested analysis and the shared source rewrite/lexical authoring permissions?

## Conclusion

MCP resolves the request dialect, performs actual source analysis and passes that AnalysisResult to if/switch/expression rewrite helpers. Data-group extraction has an explicit dialect parameter and consumes the actual retained configuration/Registry only when that analysis independently selects lexical declaration advice. The aggregate refactor list uses the same guard. Native and hosted inputs cannot become lexical advice through a forced iRules registry; selected Native rewrites retain the shared disabled obligation reason where equivalence/store permission is absent. Supported explicit lexical source authoring remains a separate branch. No NativeValue, original name input, runtime effect or edit authority is fabricated from the API dialect string. The code-action response also retains the shared disabled reason beside its empty edit list, so tool clients can distinguish a missing rewrite permission from an available edit.

## Scope

Current MCP direct and aggregate refactor request adapters. Fixed controls cover several actual Native/hosted dialect requests and a shadowed expression head. No native transformed-program experiment or passing Rust result is attached. The code-action response assertion selects an authentic Tcl 8.6 if invocation and checks the shared missing-control-flow-equivalence reason.

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

- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `refactor_at`: Pass actual request Analysis and cursor geometry to the shared source rewrite helper instead of independently rebuilding config/Registry.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `extract_datagroup`: Honor the requested dialect and require its independently selected lexical source-advice branch.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `refactor`: Use the same actual analysis and shared permissions when enumerating available refactoring tools.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `code_actions`: Transport the shared action disabled reason with its edits without creating an additional rewrite permission.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `tools::original_refactor_request_tests::original_refactor_requests_do_not_replace_native_or_vendor_policy_with_irules` (linked): Native and hosted requests cannot emit a data-group transformation by substituting an iRules policy.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `tools::original_refactor_request_tests::original_refactor_requests_use_the_selected_analysis_for_shadowed_heads` (linked): A source-defined expr head cannot receive a builtin expression rewrite from its spelling alone.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `tools::original_refactor_request_tests::original_code_action_responses_retain_disabled_rewrite_obligations` (linked): An actual Tcl 8.6 if action keeps its empty edit list and missing-control-flow-equivalence disabled reason in the MCP response.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source-consumer assertions. No Rust execution receipt or native experiment is attached. Whole-image/configuration/Registry currency and independently required purpose permissions remain mandatory.
