# naming.core.original-math-function-hover

Kind: `implementation-contract`

## Problem statement

A hover card can report a nonexistent command wrapper for a fixed function-table registration, or borrow documentation from an unrelated procedure with the same spelling. Reporting function spans and labels also cannot establish source currency or implementation identity.

## Question

Does math-function hover use the genuine original function selection without claiming nonexistent wrapper dispatch?

## Conclusion

Hover asks the shared current original function selector before compatibility advice. Canonical override cards use the selected original procedure metadata; builtin cards use the retained selected Registry identity. Fixed-table cards state their function-table purpose and omit command wrapper synopsis or dispatch claims. Command-table cards describe only the actual selected identity. Missing or stale ownership yields no Native card.

## Scope

Readonly Core hover formatting and its selection boundary. Numeric conversion, runtime return, native function registration availability and source edits are outside this card contract.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/hover.rs](../../../../rust/tcl-lsp-core/src/hover.rs), `original_math_function_hover`: Render documentation for the independently selected fixed-table, command-table or procedure implementation.
- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs), `select_at_offset`: Provide current source selection and terminal unknowns to hover.
- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `selected_registry_spec`: Supply selected identity metadata without reconstructing a qualified wrapper.
- [rust/tcl-lsp-core/src/hover.rs](../../../../rust/tcl-lsp-core/src/hover.rs), `hover::original_math_hover_tests::original_math_hover_keeps_fixed_registration_without_a_fake_wrapper` (linked): C 8.4 and Jim cards omit a tcl::mathfunc wrapper or command-table claim.
- [rust/tcl-lsp-core/src/hover.rs](../../../../rust/tcl-lsp-core/src/hover.rs), `hover::original_math_hover_tests::original_math_hover_uses_the_actual_current_source_and_override` (linked): The selected override card survives cleared reporting maps and refuses displaced source.

## Evidence

- [rust/tcl-lsp-core/src/hover.rs](../../../../rust/tcl-lsp-core/src/hover.rs)
- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs)
- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs)
