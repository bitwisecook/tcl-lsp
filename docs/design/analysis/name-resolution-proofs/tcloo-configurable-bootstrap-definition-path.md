# naming.tcloo.configurable-bootstrap-definition-path

Kind: `native-observation`

## Problem statement

The configurable definition namespaces use their own ordered paths. A worker implementation namespace alone cannot stand in for the selected script context.

## Question

What namespace path is reported in each preexisting configurable definition namespace?

## Conclusion

C9.0.4 and C9.1.0 report ::oo::define as the configurableclass namespace path and ::oo::objdefine as the configurableobject namespace path. This probe does not observe the namespace of an entered definition body.

The linked outline control keeps ordinary class source headers separate from unavailable Tcl9 member vocabulary and configurable factories under actual Tcl86 source metadata. A reporting label cannot recover an unavailable declaration or factory header.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

This source control is independent of original Native configurable bootstrap publication and imported member behaviour. It supplies no successful factory invocation, allocation, entry/frame or provider observation; existing versioned captures remain unchanged and no assertion outcome is attached.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `unsupported`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

scope.class.path: code 0, ::oo::define; scope.object.path: code 0, ::oo::objdefine.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-lsp-core/src/document_symbols.rs](../../../../rust/tcl-lsp-core/src/document_symbols.rs), `document_symbols_from_analysis`: Read genuine selected source class/member headers and withdraw unavailable member/factory roles before generating document outline labels.
- [rust/tcl-lsp-core/src/document_symbols.rs](../../../../rust/tcl-lsp-core/src/document_symbols.rs), `document_symbols::tests::original_outline_does_not_borrow_tcl9_member_roles_under_tcl86` (linked): Actual Tcl86 source metadata may retain an ordinary class header while unavailable Tcl9 classmethod/property roles supply no member declaration; an unavailable configurable factory supplies no header or outline. This is selected source/header advice, not bootstrap publication.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1)
- [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim)
