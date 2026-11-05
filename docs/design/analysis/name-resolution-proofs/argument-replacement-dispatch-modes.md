# naming.source.argument-replacement-dispatch-modes

Kind: `native-observation`

## Problem statement

A source head selected before argument evaluation can be replaced by an argument substitution. Compiled source and direct evaluation must be measured separately before treating a Registry body role as stable.

## Question

Which handler receives the following body operand after the tested substitution replaces if, in explicit C ScriptCode flags0 and Direct evaluation and in the available Jim source driver?

## Conclusion

All five tested C releases invoke the replacement custom if handler in both Tcl_EvalEx flags0 and TCL_EVAL_DIRECT for both retained inputs. The independent markers report custom handler1 and builtin body0, and the command-table query reports if and saved. The current Jim source driver has the same observed selection; a Jim TCL_EVAL_DIRECT recipe is unavailable. These finite results do not describe all compiler specialisation or dispatch behaviour.

## Scope

Exact ASCII source strings evaluated in a fresh interpreter per mode and case, followed by separate handler-marker, captured-body and command-table queries. C mode flags are actual public API arguments. Jim uses Jim_Eval only and records Direct unavailable. No source execution mode is inferred from an Unknown compilation label.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### jim

Status: `observed`. Channel: Original ASCII source passed to Jim_Eval; independent ASCII post-evaluation queries. No Jim Direct recipe. Version: jim 0.84-9-g5bac7c9.

Jim_Eval selects the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved. A Jim TCL_EVAL_DIRECT recipe is unavailable and was not tested.

### bigip

Status: `not-tested`.

No appliance capture for this exact source-handler replacement and proposed-body question.

## Consumer bindings

- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceStructure`: Keep measured body-as-data discriminator separate from source geometry and closed descendant traversal.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_minifier_script_role](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0)
- [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim)
