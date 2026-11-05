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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### bigip

Status: `not-tested`.

No appliance observation is attached to this exact question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_string_option_trailing_values](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0)
- [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim)
