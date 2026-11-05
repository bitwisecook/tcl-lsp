# naming.procedure.ordinary-local-creation-result

Kind: `native-observation`

## Problem statement

The colon-leading rejection could be mistaken for a general refusal to create a procedure in ::ns. An ordinary p control distinguishes holder existence from the name-specific rule and preserves native creation return text.

## Question

What caught completion and result follow ordinary proc p inside namespace eval ::ns?

## Conclusion

All six captured providers catch code0 for ordinary local p. C creation results are empty; Jim returns p. This control establishes neither the created command's physical header nor its later procedure activation or body execution.

## Scope

One exact ASCII source creates ::ns and performs local-colon, root-colon and ordinary-local procedure declarations under catch. It prints case/completion/result-character hex in that order; no body invocation occurs. Six original shell digests, native source/header digests, selected Jim dynamic-library digests and process/status/stream hashes are retained. Full launched patchlevels, Jim revision/configure flags and original shell stdin-versus-file argument vector were not queried. Success-return bytes are separate from textual validation and do not grant physical native/CPP/compiler storage.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Version: tcl8.4 8.4 (capture association; tcl8.5 8.5 (capture association; tcl8.6 8.6 (capture association; tcl9.0 9.0 (capture association; tcl9.1 9.1 (capture association.

Exact original case/completion/result-character hex:

```text
local-plain	0	
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### jim

Status: `observed`. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Version: jim jim (capture association.

Exact original case/completion/result-character hex:

```text
local-plain	0	70
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
