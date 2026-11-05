# naming.string-compiler-binding-replacement-controls

Kind: `native-observation`

## Problem statement

A builtin descriptor may remain in a catalogue after the real command has been renamed or replaced. Written argument roles do not prove the original native compiler is still registered.

## Question

How do the exact original string comparison registration/replacement sources affect results and available compiler selection?

## Conclusion

The retained per-release sources and stdout/stderr explicitly distinguish registration/replacement outcomes. These guest operations do not supply a current compiler prerequisite to unrelated calls.

## Scope

Five original C shell controls, with each source string and binary digest retained directly in the JSON rows.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_string_compilation/original-registration-controls.json](../../../../rust/tcl-registry/tests/data/native_string_compilation/original-registration-controls.json)
- [rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt)
- [rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt)
- [rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt)
- [rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt)
- [rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt)
- [rust/tcl-registry/tests/data/native_string_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_string_compilation/probe.c)
- [rust/tcl-registry/tests/data/native_string_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_string_compilation/windows.tsv)
