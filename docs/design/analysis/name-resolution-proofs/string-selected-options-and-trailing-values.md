# naming.string.selected-options-and-trailing-values

Kind: `native-observation`

## Problem statement

Option-looking trailing values are data when the selected string equal form reserves its two positional operands. Shortening every matching word could alter those values while a selected option prefix remains legal.

## Question

For six fixed ASCII string equal argv forms, can the selected subcommand/option be shortened while the two reserved trailing values remain exact data in C Tcl 8.4–9.1 and current Jim?

## Conclusion

All six captured providers return 1 for string equal -nocase -nocase -nocase and string e -n -nocase -nocase. Replacing either reserved data operand -nocase with -n produces 0 in the corresponding controls. The option-free two-value form and the selected -length 3 form each return 1. These observations distinguish selected option prefixes from reserved trailing values for the six exact forms; they do not establish arbitrary option grammars or minifier equivalence.

## Scope

Six fixed ASCII source-file argv forms per provider, constructed by Tcl list and invoked through uplevel #0 in the original native process. Exact queried versions, executable/build/source/stream SHA joins and caught results are retained. No raw/counting operand, physical object, compiler admission, cache, storage, observer, Native completion, opaque Unicode or BIG-IP result is established.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Selected executable. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: Jim Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation is attached to this exact question.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_string_option_trailing_values/probe.tcl](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/probe.tcl). Exact ASCII source program constructs all six complete argv lists and invokes them through uplevel #0.
- `capture` (input): [rust/tcl-registry/tests/data/native_string_option_trailing_values/capture.py](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/capture.py). Retained original capture runner; replay requires independently verified provider executables and libraries.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stderr). Exact empty stderr stream.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stderr). Exact empty stderr stream.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stderr). Exact empty stderr stream.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stderr). Exact empty stderr stream.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stderr). Exact empty stderr stream.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/receipt.json). Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stdout). Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stderr). Exact empty stderr stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Replay the byte-identical retained probe.tcl in fresh independently verified native shell processes. capture.py retains the original capture-machine executable paths; required provider/header/library/build/source and stream hashes remain in each receipt. This is a source-file CLI observation, not an object-vector API or Rust execution receipt.
