# naming.procedure.original-relocation-namespace-and-identity

Kind: `native-observation`

## Problem statement

Moving a procedure command can preserve its command identity while changing the namespace used by future calls. An already active frame can retain a different namespace, and Jim namespace-object reuse makes pointer comparison unsafe unless the original object is pinned. This probe compares those owners separately within one interpreter.

## Question

Which original command/declaration and active/future namespace owners survive the six recorded procedure moves?

## Conclusion

All C captures retain both original command and Proc identity; Jim retains command identity. C root moves change future namespace to root while the already active frame keeps its original namespace. Jim root moves in the two root cases retain the original procedure namespace object and future calls report ::A; other destination qualification controls select the recorded new namespaces. The Jim observer pins the original namespace object to prevent freed-address reuse. No display name or pointer from another process can mint this identity.

## Scope

Six exact move controls per engine; C8.4.20–9.1.0 and the hashed Jim library are observed. Jim reported patchlevel/revision and executable hashes are not retained. BIG-IP is not tested. Namespace pointer comparison stays within the retained interpreter and lifetime.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Six moves retain original command and Proc identity; active frames and future namespace move are recorded separately; root moves future namespace ::.

### jim

Status: `observed`. Channel: ASCII source via Tcl_Eval/Jim_Eval plus retained original command/private procedure and namespace objects, within one native interpreter. Version: jim Jim; patchlevel/revision not retained.

Six moves retain command identity; root rename controls preserve original namespace holder and future ::A; active/future and qualification rows remain separately recorded.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_procedure_relocation_tests.rs](../../../../runtime/rust/src/interp/native_procedure_relocation_tests.rs), `relocation_retains_native_binding_and_declaration_with_separate_active_namespace` (linked): Compare each actual command/declaration identity and original active/future namespace independently.

## Evidence

- [runtime/rust/tests/data/native_procedure_relocation/manifest.json](../../../../runtime/rust/tests/data/native_procedure_relocation/manifest.json)
- [runtime/rust/tests/data/native_procedure_relocation/probe.c](../../../../runtime/rust/tests/data/native_procedure_relocation/probe.c)
- [runtime/rust/tests/data/native_procedure_relocation/observations.tsv](../../../../runtime/rust/tests/data/native_procedure_relocation/observations.tsv)
