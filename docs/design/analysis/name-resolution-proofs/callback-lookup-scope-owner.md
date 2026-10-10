# naming.callback.lookup-scope-owner

Kind: `implementation-contract`

## Problem statement

Executable positions, evaluation timing and lookup frame can drift between Registry, DSL and Studio. A frame annotation on a receiving command must not turn a data operand, removal prefix, invalid argv or unknown expansion into a reached callback or lend installer bindings to future code.

## Question

How does the shared owner project callback lookup purpose only for selected executable positions, and retain that purpose across Rust, DSL and Studio?

## Conclusion

Selected metadata combines complete valid argv, actual role/prefix/option positions and script timing with an explicit optional ScriptLookupScope. InvokingFrame, GlobalFrame and TriggerFrame select coordinates only; reference-only forms, data operands, incomplete argv and missing scope remain unavailable. Command and subcommand fields round-trip through Studio Rust/DSL renderers and loader, while an invalid scope spelling withdraws the field. A caller still needs the actual operand producer and an independent command table at reached entry. Callback producers retain the exact selected effective operand ordinal and readonly head lineage. List receivers use native list children; trace script receivers additionally require actual counted/CString script and appended-suffix argv equivalence. InvokingFrame lookup uses the authentic post-argument caller point or a quiet conditional declaration point; GlobalFrame retains root coordinates without a future table, and unreached relative TriggerFrame names retain no lookup. A C-owned absolute name can retain its genuine target slot independently of an unknown trigger namespace; it keeps TriggerFrame purpose and no caller naming scope. This does not select future command occupancy or callback entry. Background scans can borrow an independently retained exact-image and full-config source inventory through the same prefix issuer. Header-only scans never run binding analysis; they retain static readonly lineage after the selected evaluator check and leave lookup absent. Background callback rows are emitted only from authentic OriginalCallbackPrefix and its reported_source_head projection. A matching original inventory, including refusal for a replaced/deleted installer or changed vector, is authoritative and cannot fall back to nominal header metadata. Without a site issuer, independently selected authoring ingress and selected prefix evaluator can retain readonly header lineage with absent lookup. Both foreground and background rows retain argc absent and rename_safe false.

Two additional marked Database source definitions keep exact execution-trace suffix-count alternatives and TriggerFrame purpose even when the future lookup is unavailable. Mixed enter/leave counts remain distinct across handler declaration shapes and cannot borrow installer coordinates for arity advice.

## Scope

Current Registry facts and descriptor/authoring parity contract. Native observations for exact lsort, after and trace namespace lookup remain separate records. These Rust selectors are coverage bindings and are not claimed executed here.

Registration syntax supplies count/scope metadata only. These source controls neither observe a future command table/trigger namespace nor establish callback execution, Native frame/argv, actual completion or a passing assertion; original callback provider records remain unchanged.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/script_lookup_scope.rs](../../../../rust/tcl-registry/src/script_lookup_scope.rs), `ScriptLookupScope`: Explicit executable-entry coordinate purpose, independent of timing, reach and future bindings.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `InvocationFacts::script_lookup_scope`: Select one effective post-head argv position through actual roles, timing and option grammar.
- [rust/tcl-registry/src/selected_script_timing.rs](../../../../rust/tcl-registry/src/selected_script_timing.rs), `SelectedScriptMetadata::lookup_arguments`: Reuse the same selected position/timing projection rather than a command-name consumer heuristic.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `apply_command_stmt`: Parse explicit scope field and withdraw malformed enum spelling.
- [rust/tcl-spec-studio/src/render_spectcl.rs](../../../../rust/tcl-spec-studio/src/render_spectcl.rs), `render_pack`: Preserve explicit scope across authoring formats without implying future table state.
- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `SourceInvocationBinding::original_callback_prefix`: Authenticate selected effective callback ordinal, retained operand, actual prefix evaluator and readonly head without installer head donation.
- [rust/tcl-compiler/src/command_binding/source_name_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/source_name_lookup.rs), `SourceInvocationBinding::original_callback_lookup_from_rows`: Keep invoking/global/trigger scope independent of future occupancy; absolute C qualification can retain a target slot without borrowing a trigger namespace, while relative unreached trigger names refuse.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `InvocationFacts::command_prefix_arity`: Project appended arity only for selected executable prefix operands, retaining effective post-head ordinals and reference-only refusal.
- [rust/tcl-compiler/src/signature_scan/mod.rs](../../../../rust/tcl-compiler/src/signature_scan/mod.rs), `extract_signatures_with_original_bindings`: Borrow an existing exact-source/full-config binding inventory for callback projections without running source analysis in a header query.
- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `OriginalCallbackPrefix::reported_source_head`: Project the sealed readonly name child into a checked UTF-8 label and exact original extent without editable-word or target selection.
- [rust/tcl-compiler/src/signature_scan/walker.rs](../../../../rust/tcl-compiler/src/signature_scan/walker.rs), `record_command_prefix_invocations`: Emit background callback metadata only from an authentic prefix issuer rather than nominal command-label scanning.
- [rust/tcl-registry/src/selected_script_timing.rs](../../../../rust/tcl-registry/src/selected_script_timing.rs), `selected_script_timing::tests::selected_lookup_scopes_require_executable_positions_and_reference_timing` (linked): Known callback positions select scope, unknown option values retain shape-only positions, reference/data/invalid/expanded cases decline.
- [rust/tcl-spec-studio/src/render_spectcl.rs](../../../../rust/tcl-spec-studio/src/render_spectcl.rs), `render_spectcl::tests::executable_lookup_scope_survives_rust_and_dsl_round_trip` (linked): Distinct command and member scopes survive draft, Rust text, DSL load and draft; malformed scope is diagnosed and withdrawn.
- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `command_binding::original_callback_lookup::tests::original_callback_heads_keep_selected_operand_evaluator_and_frame_scope` (linked): Selected genuine callback operands retain readonly head/count/scope. Unreached relative trigger heads borrow no installer lookup; C-owned absolute trigger targets retain only their original slot and TriggerFrame purpose without caller naming scope.
- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `command_binding::original_callback_lookup::tests::original_callback_scope_refuses_foreign_vectors_and_installer_head_lookup` (linked): Missing consuming binding, changed complete word vector and installer-head lookup cannot donate callback producer correspondence.
- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `command_binding::original_callback_lookup::tests::original_callback_invocation_rows_retain_readonly_scope_lookup` (linked): Foreground callback rows transport the exact readonly prefix head, baked argument count and independent invoking-frame lookup without edit permission.
- [rust/tcl-compiler/src/signature_scan/mod.rs](../../../../rust/tcl-compiler/src/signature_scan/mod.rs), `signature_scan::original_callback_scan_tests::original_callback_scan_borrows_only_matching_inventory_scope` (linked): A matching source inventory projects the identical sealed callback input and lookup; changed document bytes or full lexer configuration cannot lend scope.
- [rust/tcl-compiler/src/signature_scan/mod.rs](../../../../rust/tcl-compiler/src/signature_scan/mod.rs), `signature_scan::original_callback_scan_tests::original_callback_header_scan_keeps_readonly_lineage_without_scope` (linked): Header-only scans retain readonly native list or trace SCRIPT-equivalent head lineage without any lookup; substituted trace head is unavailable.
- [rust/tcl-compiler/src/signature_scan/mod.rs](../../../../rust/tcl-compiler/src/signature_scan/mod.rs), `signature_scan::original_callback_scan_tests::original_callback_scan_respects_selected_installer_moves_and_terminal_barriers` (linked): Moved and aliased installers retain authentic prefixes; known replacement/deletion supplies no callback row, with no argc or rename permission.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/command_binding/original_callback_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/original_callback_lookup.rs), `SourceInvocationBinding::original_callback_prefix`: Retain the authentic callback operand and selected appended-count/scope purpose without converting registration-time metadata into a future trigger lookup.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::execution_trace_source_counts_keep_unavailable_future_frame` (linked): Execution-trace source registrations retain exact selected enter/leave appended-count alternatives and TriggerFrame purpose while future lookup is absent; unavailable future coordinates cannot supply arity diagnostics.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::mixed_execution_trace_source_counts_do_not_issue_a_future_lookup` (linked): Mixed execution-trace enter/leave source alternatives retain both appended counts across exact/default/variadic handler shapes without issuing future lookup or installer-frame arity claims.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-registry",
  "selected_lookup_scopes_require_executable_positions_and_reference_timing"
]
```

Also run the named Studio round-trip selector. Scope annotations do not execute a callback; no Native entry or Normal grant follows.
