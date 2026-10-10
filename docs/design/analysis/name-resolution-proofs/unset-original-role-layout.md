# naming.variable.unset-original-role-layout

Kind: `implementation-contract`

## Problem statement

An unset operand can retain counted bytes without a host String value. Losing its selected VarWrite role would widen the invocation before the independently selected destruction or retained-wrapper operation, while role geometry alone cannot establish the target cell or completion.

## Question

Can selected unset option grammar retain exact variable operand ordinals for counted bytes without requiring later variable values or granting a successful destruction?

## Conclusion

Unset argument roles use exact argv cardinality and the independently selected shared unset option grammar. Only reached option candidates require known values; a known first non-option preserves every following variable operand ordinal, including unknown name values. Counted byte names retain their VarWrite roles, while an unknown option prefix or expansion withdraws completeness. The bare authored descriptor keeps its counted source grammar separately. Role metadata grants neither a variable cell, binding detachment, observer closure, successful unset nor Normal completion.

## Scope

Rust Registry structured role projection for independently selected C8.4–9.1 and current Jim grammars. Runtime option extent delegates to UnsetOptionProtocol rather than a duplicated scanner. Logical authored source grammar remains separate. These selectors do not capture native outcomes or prove retained variable wrapper destruction.

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

- [rust/tcl-registry/src/commands/tcl/unset_.rs](../../../../rust/tcl-registry/src/commands/tcl/unset_.rs), `unset_layout_roles`: Project exact variable operand ordinals from selected shared option grammar and cardinality, independently of executable variable receiver authority.
- [rust/tcl-registry/src/native_unset_options.rs](../../../../rust/tcl-registry/src/native_unset_options.rs), `UnsetOptionProtocol`: Keep native runtime and counted known-source option selection in their existing sole purpose owner.
- [rust/tcl-registry/src/commands/tcl/unset_.rs](../../../../rust/tcl-registry/src/commands/tcl/unset_.rs), `commands::tcl::unset_::tests::unset_roles_keep_counted_names_and_selected_option_prefixes` (linked): Retains counted first name and later unknown name VarWrite ordinals, uses the selected raw-zero option prefix extent, and withdraws for an unknown first option candidate.

A named test is a coverage binding, not a claim that it executed.

## Replay

The fixed Rust selector is linked without an attached execution receipt. Native unset, variable storage and retained-wrapper observations remain independent questions.
