# naming.variable.namespace-upvar-target-and-empty-pair-grammar

Kind: `native-observation`

## Problem statement

Namespace upvar can validate pair count before selecting a missing namespace, while Jim helper forwarding may pad an odd pair and select flat variable storage. Treating a common displayed namespace or local alias as the same grammar/cell route changes whether a target exists and which error is returned.

## Question

What exact codes/results follow the six retained namespace upvar no-pair, odd-pair, odd-tail, missing-read, missing-write and current-relative scripts on each selected provider?

## Conclusion

The thirty-six exact results preserve provider-specific pair grammar and target selection: C84 lacks the subcommand, C85 rejects no pairs, modern C allows no pairs, and Jim forwards odd pairs and permits the measured missing-write/current-relative paths. These observations do not establish a missing physical namespace token, header, arbitrary alias-chain closure, or normal completion for other operands.

## Scope

Six ASCII scripts per C84-91 and manifested Jim build, public counted evaluation in fresh interpreters. Separate from original argv eighteen-row grammar and original compiler frontier probes; these inputs query execution through the namespace helper and actual target-cell read/write paths.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.4 8.4.20.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-odd-pair | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-odd-tail | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-missing-read | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-missing-write | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-current-relative | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |

### tcl8.5

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.5 8.5.19.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 0 | "" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### jim

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "wrong # args: should be \"upvar ?level? otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-pair | 0 | "VALUE" |
| upvar-odd-tail | 0 | "VALUE SECOND" |
| upvar-missing-read | 1 | "can't read \"y\": no such variable" |
| upvar-missing-write | 0 | "VALUE" |
| upvar-current-relative | 0 | "VALUE" |

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `namespace_upvar_grammar_and_target_cells_match_36_native_results`: Independent current Rust comparison at the stated semantic boundary.
- [runtime/rust/src/cmd_namespace/native_upvar_tests.rs](../../../../runtime/rust/src/cmd_namespace/native_upvar_tests.rs), `namespace_upvar_grammar_and_target_cells_match_36_native_results`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `cmd_namespace::native_upvar_fixture_tests::namespace_upvar_grammar_and_target_cells_match_36_native_results` (linked): Compares all thirty-six original script completions and target-cell results, including unsupported C84.
- [runtime/rust/src/cmd_namespace/native_upvar_tests.rs](../../../../runtime/rust/src/cmd_namespace/native_upvar_tests.rs), `cmd_namespace::native_upvar_tests::namespace_upvar_grammar_and_target_cells_match_36_native_results` (linked): Independently compares the same thirty-six scripts in the Runtime original native core.

## Evidence

- [rust/tcl-vm/tests/data/native_namespace_upvar](../../../../rust/tcl-vm/tests/data/native_namespace_upvar)
