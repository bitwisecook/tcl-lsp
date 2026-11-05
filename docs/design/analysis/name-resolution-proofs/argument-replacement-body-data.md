# naming.source.argument-replacement-body-data

Kind: `native-observation`

## Problem statement

If the selected runtime handler treats an advisory body operand as data, changing descendant separators can change a result even when the enclosing word remains valid source syntax.

## Question

Does the proposed body-separator-only source change preserve the custom handler result after the tested argument substitution replaces if?

## Conclusion

The original and proposed body-separator candidate both complete successfully on all six tested providers, but their counted result bytes differ. Each result equals the exact body bytes independently retained by the replacement custom handler. The difference is observed in both explicit C evaluation modes and in Jim source evaluation. The candidate is proposed source, not captured output of the minifier, and this observation does not prove general rewrite equivalence or failure.

## Scope

The two ASCII scripts differ only in separators inside the outer braced body operand. Literal data VALUE retains its internal spaces. Result and custom-body bytes are sampled independently after fresh source evaluation. C Tcl_EvalEx flags0 and TCL_EVAL_DIRECT are separate controls; Jim Direct remains unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### jim

Status: `observed`. Channel: Original ASCII source passed to Jim_Eval; independent ASCII post-evaluation queries. No Jim Direct recipe. Version: jim 0.84-9-g5bac7c9.

Both source cases complete0 under Jim_Eval, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output; Jim Direct is unavailable.

### bigip

Status: `not-tested`.

No appliance capture for this exact source-handler replacement and proposed-body question.

## Consumer bindings

- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceStructure`: Keep measured body-as-data discriminator separate from source geometry and closed descendant traversal.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `source_structure::syntax_compaction_tests::original_syntax_regions_reject_body_roles_when_arguments_replace_the_head` (linked): The matched original source remains readonly advisory body geometry, while the separate closed syntax traversal excludes that outer body and retains the authentic lexical substitution. This binding does not claim native Rust execution.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_minifier_script_role](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim)
