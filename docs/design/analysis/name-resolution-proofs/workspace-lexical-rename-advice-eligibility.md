# naming.workspace.lexical-rename-advice-eligibility

Kind: `implementation-contract`

## Problem statement

Cross-document compatibility rename helpers have no analysis argument and can otherwise reuse reporting-name maps from Native or hosted vendor documents. A changed display dialect, an empty hosted source or a missing captured context cannot select lexical compatibility or authorise an edit.

## Question

Does workspace compatibility rename advice require every active captured document to independently select lexical source advice, including empty hosted vendor documents?

## Conclusion

The workspace getter requires a nonempty active document inventory, matching retained URI ownership and a captured source context for every provider. Each context must independently select lexical advice. Original-name contexts, hosted vendor contexts and missing context refuse. Hosted classification comes from the actual resolved vendor policy or execution environment, including empty source; a changed reporting dialect cannot reopen compatibility. Clone and document removal preserve the active inventory decision. This is compatibility eligibility only; current source validation and an independently issued rename plan remain separate requirements.

## Scope

Rust readonly workspace advice-mode classification. Genuine selected logical Jim 0.79 source configuration is a compatibility control, while selected Native C source, hosted F5 source and unknown context are separate refusals. No interpreter execution, measured Jim 0.79 behaviour, native naming recipe, lookup, runtime cell or edit permission is supplied.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust captured workspace advice-mode question. No native provider observation or executed Rust result is attached; logical source configuration does not attest an interpreter build or edit permission.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspaceIndex::allows_lexical_rename_advice`: Gate reporting-name compatibility advice by every actual active captured provider, without selecting lookup or issuing a rename plan.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspaceDiagnosticSourceContext::for_analysis`: Capture complete original source/configuration/Registry currency and actual Native/vendor versus lexical advice mode, including empty hosted source.
- [rust/tcl-dialect/src/model/bigip_execution_context.rs](../../../../rust/tcl-dialect/src/model/bigip_execution_context.rs), `BigIpExecutionContext::for_environment`: Classify the independently retained hosted environment without borrowing C/Jim naming or appliance execution authority.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `workspace_index::tests::lexical_rename_advice_requires_every_captured_provider_and_refuses_vendor_contexts` (linked): An actual explicitly logical source context permits advice only for a nonempty all-logical inventory. Native opaque and hosted empty/nonempty source contexts refuse despite counterfactual reporting dialect labels; clone/removal and missing context retain independent active ownership.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selector exercises the captured advice-mode and active ownership premises. No Rust execution receipt or native provider observation is attached.
