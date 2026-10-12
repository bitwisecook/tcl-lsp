# naming.variable.registry-receiver-authoring-parity

Kind: `implementation-contract`

## Problem statement

Whole-array effects do not imply a root-only argument naming form. Derived roles can also use subcommand/form offsets. An opaque Rust-only receiver field would make Registry, SpecTcl and Studio consumers disagree about the same authored command.

## Question

Does variable receiver naming metadata keep scope, explicit withdrawal and authoring parity across Registry, SpecTcl and Studio?

## Conclusion

Variable receiver metadata is explicitly authored Combined or TraceSubject argv naming data, separate from whole-array effects, alias declaration purposes, compiler preparation and successful access. Command/subcommand/form selection preserves exact offsets; absent metadata inherits and an explicit empty list withdraws. SpecTcl validates unique byte-sized positions and Studio load/render round trips the same field, including its selected naming purpose. Alias receivers remain under their independently selected target/local purposes.

## Scope

Rust Registry/SpecTcl/Studio implementation contract. Fixed tests cover selected forms, member offsets, alias exclusion, metadata inheritance/withdrawal and authoring round trips. Fresh empirical array source records independently establish the measured combined receiver law and guest completion differences.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native provider observation or executed test result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `variable_receiver_operand_form`: Selects actual argument naming form independently of access effects and alias transitions.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `variable_receivers_value`: Validates authored Combined receiver positions and explicit-empty withdrawal.
- [rust/tcl-spec-studio/src/draft.rs](../../../../rust/tcl-spec-studio/src/draft.rs), `variable_receiver_positions`: Projects the same authored position data into Studio drafts.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::variable_receiver_fact_projection_preserves_form_precedence_and_member_offsets` (linked): Preserves command/subcommand/form selection, member offsets and explicit metadata withdrawal.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::selected_variable_receiver_form_keeps_array_argv_combined_and_aliases_separate` (linked): Keeps stock array argv Combined and excludes alias-purpose receivers.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::combined_variable_receiver_positions_preserve_scope_and_explicit_withdrawal` (linked): Keeps authored command/member/form positions, rejects invalid positions and preserves declared-empty withdrawal.
- [rust/tcl-spec-studio/src/render_spectcl.rs](../../../../rust/tcl-spec-studio/src/render_spectcl.rs), `render_spectcl::tests::combined_variable_receiver_metadata_round_trips_commands_members_and_forms` (linked): Round trips the same naming metadata through Studio DSL rendering/loading.

A named test is a coverage binding, not a claim that it executed.

## Replay

Run the exact linked Rust selectors through the maintained workspace test setup. This record contains no interpreter observation or executed Rust result.
