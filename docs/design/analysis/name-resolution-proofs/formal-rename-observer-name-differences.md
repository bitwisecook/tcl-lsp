# naming.formal.rename-observer-name-differences

Kind: `native-observation`

## Problem statement

Renaming a formal changes name-observing operations even when a whole-word return still yields the same argument object. A transform that considers only returned bytes can miss info locals, info args or trace callback subjects. These explicit observer controls delimit the simple return comparison; they do not authorize a transform in any body with introspection or observers.

## Question

Which tested local/formal introspection and variable read-trace outputs differ after longargument is renamed a?

## Conclusion

All six report longargument versus a from info locals and info args. All five C interpreters still return the same held VALUE object in the read-traced return bodies, but callbacks report longargument versus a. Jim's actual trace command query is empty, so its trace-body question is unsupported. Observed whole-object return identity does not erase these naming differences.

## Scope

ASCII original object-vector declarations and same VALUE argv; independent info args source query. C8.4 trace variable ... r and C8.5+ trace add variable ... read are separate actual source forms. Jim only queries trace availability, with no callback execution. No arbitrary external observer, execution trace or rename implementation is proved.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### jim

Status: `observed`. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries. Version: jim 0.84-9-g5bac7c9.

info locals and info args report longargument versus a. Trace command query is empty; no trace callback executes.

### bigip

Status: `not-tested`.

No appliance capture for this exact counted formal/body/argv question.

## Consumer bindings

- [rust/tcl-vm/src/command/native_formal_alpha_tests.rs](../../../../rust/tcl-vm/src/command/native_formal_alpha_tests.rs), `command::native_formal_alpha_tests::formal_renaming_observer_controls_report_the_distinct_names` (linked): Compares actual local/formal introspection and C trace-name controls, with independent Jim trace absence; this does not claim arbitrary observer equivalence.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_alpha](../../../../rust/tcl-vm/tests/data/native_formal_alpha)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim)
- [rust/tcl-vm/tests/data/native_formal_alpha/v1](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v1)
