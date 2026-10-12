# naming.variable.original-body-cursor-frame

Kind: `implementation-contract`

## Problem statement

A variable-completion insertion cursor before an authentic body closer is at the exclusive end of the body occurrence range. Treating that cursor as a source occurrence loses its genuine procedure frame; extending ordinary membership would admit the closer as variable source.

## Question

Can readonly variable-completion cursor selection retain the actual body frame at its unchanged source end while preserving half-open occurrence membership?

## Conclusion

The cursor-only frame query reuses the exact original declaration entry, full consumer source/configuration and unanimous frame/context ownership. Interior cursors retain existing occurrence membership. End affinity is issued only for an unchanged contiguous body whose retained bytes equal the owning source slice; it stops before the following source unit. Variable-name occurrences and physical read/write/frame lifetime queries retain their half-open boundaries. The receipt supplies readonly lexical frame visibility, without an entered activation, current cell, contents, Normal or edit authority.

## Scope

Rust original source/body cursor correspondence. The fixed test selects two independent authentic procedure frames immediately before their original closers despite erased reporting maps, distinguishes each interior from the cursor after its closer, verifies ordinary entry membership remains half-open, and rejects changed full source/configuration. Materialised body values cannot obtain contiguous end affinity. No native provider result is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `owns_source_cursor`: Authenticate cursor-only contiguous body-end affinity independently of unchanged half-open occurrence membership.
- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `original_variable_frame_at_offset`: Select one actual original local naming frame with independent full configuration and frame/context unanimity; conflicting owners still withdraw.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `original_variable_frame_in_source`: Check the entire current consumer source and full configuration before exposing readonly cursor-frame correspondence.
- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `command_binding::declaration_layout::tests::original_variable_cursor_at_body_end_keeps_occurrence_membership_half_open` (linked): Two genuine original procedure frames remain distinct at their body-end insertion cursors and match each interior frame; after-closer cursors abstain, ordinary occurrence membership remains half-open, and changed complete source/configuration withdraws.
- [rust/tcl-lsp-core/src/completion/original_variables.rs](../../../../rust/tcl-lsp-core/src/completion/original_variables.rs), `completion::original_variables::tests::original_variable_completion_selects_actual_formal_frame_and_alias_spelling` (linked): The genuine cursor before the first procedure body closer selects its actual formal/local/alias frame despite cleared reporting maps, excludes the second procedure frame and rejects changed complete source.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace setup. Naming a selector supplies no Rust execution result; all native providers remain not-tested for this source-cursor contract.
