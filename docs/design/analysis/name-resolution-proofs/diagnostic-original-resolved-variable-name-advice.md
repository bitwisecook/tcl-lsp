# naming.diagnostics.original-resolved-variable-name-advice

Kind: `implementation-contract`

## Problem statement

Resolved SSA variables, authored aliases and dictionary operand values can contain literal dollars or unmatched parentheses. Applying substitution-aware normalisation to those values can accidentally donate startup-variable advice or shorten an unrelated scalar name.

## Question

How does source diagnostic advice preserve literal resolved variable names and select only genuine combined array roots or exact global aliases, without borrowing Native cell, read or startup-value authority?

## Conclusion

Resolved-name consumers delegate to split_array_name_braced(name, true), preserving literal dollar and brace units rather than interpreting them as substitution syntax. A balanced combined array name selects its actual base; unmatched scalar parentheses remain complete. startup_var_name removes only the leading global marker after that literal boundary, while named namespaces retain qualification. has_global_startup_binding compares exact authored alias roots after the same selected boundary; an alias argv cannot donate global advice to literal $argv, whereas an authentic alias ::$argv retains that literal root. collect_defined_vars, dictionary suppression/constants and dataflow declared/suppression/output-name consumers use the shared resolved-name purpose. Native Namespace cell handling remains its independent actual namespace-identity path. The marked authored-cell helper control fixes literal dollars/braces, unmatched parentheses, Unicode element roots and exact aliases. The separate genuine retained Logical analysis control checks emitted W210 span ${$argv}, distinguishes a defined literal $counter from counter, and retains ordinary startup argv advice. Those readonly Logical diagnostic assertions do not establish a Native variable cell, startup value, read/store success, callback/frame, completed body or native completion. Literal unmatched scalar parentheses do not borrow a same-root array parameter: the already resolved scalar name remains exact through source read-before-set advice, with only genuine closed combined elements sharing an array root.

## Scope

Two fixed source/API controls: an explicitly chosen authored-cell model verifies literal name/root/global-alias selection, and complete plain Logical analyses verify the bounded emitted diagnostic spans and ordinary argv positive. They do not run C Tcl, Jim or BIG-IP or turn their symbolic names into original Native storage facts. All seven native providers are not tested; actual software test results are retained separately.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-diagnostics-original-resolved-variable-name-advice-helpers.rs` (implementation): [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs). SHA-256 `4c49e4e1f3ebaf80c1309f655536d08baf6171687ea0125e46f3aa5002ce98ab`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-diagnostics-original-resolved-variable-name-advice-var_command.rs` (implementation): [rust/tcl-compiler/src/analyser/diagnostics/var_command.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs). SHA-256 `182a6ad9b6ba58959f65b7ffea8349e95f51297423b380ee29c8c019766e707e`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-diagnostics-original-resolved-variable-name-advice-naming.rs` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `77c05067c4a55d218010c4225b664fbb07d16624293457299a52d96e3912c6fc`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `split_array_name_braced`: Own literal resolved scalar/combined-element names with braced_literal=true independently of substitution-sigil parsing.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `startup_var_name`: Preserve literal resolved units and remove only the global marker for authored startup advice.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `has_global_startup_binding`: Require exact literal authored root/alias correspondence; named namespaces and different literal dollars do not donate root startup advice.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `collect_defined_vars`: Retain complete literal resolved definition names instead of sigil-aware source normalisation.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `harvest_dict_with_suppression`: Share literal resolved dictionary output-name geometry within its separate suppression purpose.
- [rust/tcl-compiler/src/analyser/diagnostics/var_command.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/var_command.rs), `harvest_dict_with_constants`: Retain literal resolved dictionary operand/output-name boundaries without Native cell or read admission.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `analyser::diagnostics::helpers::original_resolved_variable_advice_tests::authored_startup_advice_preserves_literal_dollars_and_exact_global_aliases` (linked): Chosen authored-cell fixtures preserve literal dollars/braces, unmatched scalar parentheses and Unicode element roots; argv alias cannot donate advice to $argv, while exact ::$argv alias can retain its literal root. No Native cell or startup value is asserted.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `analyser::diagnostics::helpers::original_resolved_variable_advice_tests::literal_variable_diagnostics_keep_defined_names_and_startup_advice_distinct` (linked): Genuine complete retained Logical analyses emit only exact ${$argv} or ${counter} W210 spans while preserving defined literal $counter and ordinary argv startup advice. These diagnostic outputs grant no original Native read/store/body/completion purpose.
- [rust/tcl-compiler/src/analyser/diagnostics/helpers.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/helpers.rs), `analyser::diagnostics::helpers::original_resolved_variable_advice_tests::literal_scalar_parentheses_do_not_borrow_array_root_parameters` (linked): Genuine retained Logical diagnostics distinguish an unmatched/trailing-parenthesis scalar from the declared array-root parameter; literal dollar/source spans and real closed element suppression keep separate purposes.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
