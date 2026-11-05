# naming.procedure.local-colon-creation-result

Kind: `native-observation`

## Problem statement

A single-colon :p procedure name in non-global ::ns is neither an ordinary p nor an explicitly rooted name. Older C releases reject its creation while later C and Jim accept it, and successful native return text differs from a pure validation result.

## Question

Does proc :p succeed inside namespace eval ::ns, and what exact caught completion/result does each provider return?

## Conclusion

C8.4 and C8.5 catch code1 with the exact non-global colon-leading procedure-creation error. C8.6, C9.0 and C9.1 catch code0 with an empty result; Jim catches code0 with result :p. This measures declaration creation acceptance and result only; the procedure body is not invoked.

## Scope

One exact ASCII source creates ::ns and performs local-colon, root-colon and ordinary-local procedure declarations under catch. It prints case/completion/result-character hex in that order; no body invocation occurs. Six original shell digests, native source/header digests, selected Jim dynamic-library digests and process/status/stream hashes are retained. Full launched patchlevels, Jim revision/configure flags and original shell stdin-versus-file argument vector were not queried. Success-return bytes are separate from textual validation and do not grant physical native/CPP/compiler storage.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Version: tcl8.4 8.4 (capture association; tcl8.5 8.5 (capture association.

Exact original case/completion/result-character hex:

```text
local-colon	1	63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Version: tcl8.6 8.6 (capture association; tcl9.0 9.0 (capture association; tcl9.1 9.1 (capture association.

Exact original case/completion/result-character hex:

```text
local-colon	0	
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### jim

Status: `observed`. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Version: jim jim (capture association.

Exact original case/completion/result-character hex:

```text
local-colon	0	3a70
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### bigip

Status: `not-tested`.

No appliance declaration capture for this exact ASCII source is attached; C/Jim holder validation and return text do not establish BIG-IP load or event outcomes.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `procedure_name_creation_error_for_policy`: Selected authored/native textual procedure-name validation; acceptance supplies no physical procedure/header/CPP allocation or native success return value.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::tests::procedure_creation_name_policy_matches_18_native_slots_without_header_grants` (linked): Compares selected naming-policy acceptance/error bytes for all18 provider/holder/name annotations, including this case; native success-result bytes remain separate observed data and are not manufactured by the validation query.

## Evidence

- [rust/tcl-registry/tests/data/authored_procedure_names](../../../../rust/tcl-registry/tests/data/authored_procedure_names)
