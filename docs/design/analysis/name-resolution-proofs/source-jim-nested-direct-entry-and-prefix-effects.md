# naming.source.jim-nested-direct-entry-and-prefix-effects

Kind: `native-observation`

## Problem statement

Recursive validation of nested script syntax can accidentally suppress an outer command prefix that the selected provider really executes. Conversely, admitting a complete-looking prefix can execute commands when the provider rejects the whole script at entry. Command substitutions, quoted substitutions and variable-index substitutions require the actual selected script-entry purpose rather than a single universal parse cut.

## Question

For these seven counted nested-script sources, which outer and child prefix effects occur under public Direct evaluation on C Tcl 8.4–9.1 and the measured Jim build, and how do they relate to the separate info complete query?

## Conclusion

All six measured providers execute before=1 for the four terminal nested word-weld cases, return extra characters after close-brace, leave sink/after absent and report complete=1. Even the case containing set inner 1 leaves inner absent. In the two missing-brace cases the five C providers execute before=1; Jim leaves every observed effect absent and returns unmatched "[". For bare x{ inside a command substitution, the C providers report complete=1 and succeed through after=2, with sink string bytes 785c7b; Jim reports complete=0 and rejects entry with every effect absent. These are distinct selected script-entry outcomes for these original bytes, not a general prefix-execution rule. BIG-IP is not tested.

## Scope

Seven exact counted ASCII source values reconstructed with binary format H* inside the unchanged public probe. Each uses info complete followed by uplevel #0 and independent reads of before, inner, sink and after. Six original subprocesses exit0 with empty stderr and yield 42 case rows. Physical source LF, binary source bytes, completeness results, script catch results and reached effects are independent facts. No bytecode, compiler binding, original Native object/header, lifetime, callback, frame, Host error metadata, F5/TMM or arbitrary nested grammar equivalence is measured.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases report complete=0 and before=1, then error with sink/after absent. Bare x{ reports complete=1 and succeeds with before=1, after=2 and sink string bytes 785c7b. This answer covers only the seven original case values.

### jim

Status: `observed`. Channel: CLI probe.tcl reconstructs exact hexadecimal script bytes; public info complete and uplevel #0 are followed by separate effect reads. Version: jim 0.84-9-g5bac7c9.

Version is printed by this original process. The four terminal nested cases report complete=1 and before=1, but inner/sink/after remain absent and evaluation errors. The two missing-brace cases and the bare x{ case report complete=0, return unmatched "[" and leave all four effects absent. This answer covers only the seven original case values.

### bigip

Status: `not-tested`. Channel: No appliance capture for this question.

No actual BIG-IP response to these exact inputs is retained. Local C Tcl/Jim outcomes establish no BIG-IP or TMM behaviour.

## Consumer bindings

- [rust/tcl-lexer/src/parse_cut.rs](../../../../rust/tcl-lexer/src/parse_cut.rs), `parse_cut::original_parse_descent_tests::original_enclosing_script_cut_delays_child_syntax_without_hiding_root_delimiters` (linked): The configured software parse-cut owner delays child-command syntax in bracket, quoted and indexed roots under EnclosingScript, while retaining root braced-word, missing bracket and unterminated quote cuts; the independently selected child gets its own cut. This syntax-only definition grants no Direct prefix effects, entered source or execution receipt.
- [rust/tcl-lexer/src/jim_script_tokens.rs](../../../../rust/tcl-lexer/src/jim_script_tokens.rs), `jim_script_tokens::original_script_syntax_tests::original_jim_syntax_roster_delays_child_syntax_but_keeps_enclosing_delimiters` (linked): The selected Jim syntax roster delays terminal child-command syntax in bracket, quoted and indexed forms, preserves actual enclosing unmatched-bracket records for missing-brace and bare x{ sources, and exposes a child-root terminal cut independently. This software syntax projection does not claim the separately measured Direct effects or bytecode/frame authority.
- [rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs](../../../../rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs), `command_binding::original_script_parse_entry_tests::original_jim_nested_script_parse_stops_body_prefix_and_later_outer_commands` (linked): Held Jim source modeling retains the reached outer eval entry but excludes invalid body-prefix and later outer procedure declarations; the selected nested grammar independently locates the parse cut. No actual script effect, callback or Native frame is observed.
- [rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs](../../../../rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs), `command_binding::original_script_parse_entry_tests::original_jim_command_child_parse_keeps_root_prefix_but_refuses_child_prefix` (linked): The held Jim command-child source model keeps the prior root declaration while excluding invalid child-prefix and later-root declarations. Native Direct probe outcomes and public completeness evidence remain independently scoped.
- [rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs](../../../../rust/tcl-compiler/src/command_binding/original_script_parse_entry_tests.rs), `command_binding::original_script_parse_entry_tests::original_jim_direct_delimiter_scan_does_not_borrow_c_bare_brace_syntax` (linked): The held Jim delimiter owner refuses malformed child shapes before any root publication; corresponding selected C source examples retain their prior/later declarations. This finite source-model comparison does not reuse C grammar as a Jim execution owner.

## Evidence

- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.4.20)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.5.19)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/8.6.18)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.0.4)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/9.1.0)
- [runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim](../../../../runtime/rust/tests/data/native_jim_nested_direct_entry_prefix_effects_original/627/Jim)
- [rust/tcl-compiler/src/command_binding](../../../../rust/tcl-compiler/src/command_binding)
