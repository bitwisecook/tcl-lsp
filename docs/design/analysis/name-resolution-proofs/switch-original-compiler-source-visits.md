# naming.switch-original-compiler-source-visits

Kind: `native-observation`

## Problem statement

Exact-mode switch can omit a masked duplicate arm while glob mode or a prior continuation retains it. A source body list or all syntax spans cannot stand in for the actual selected compiler visit set.

## Question

Which exact arm bodies and local names does the native switch compiler visit in the six original procedure sources?

## Conclusion

C8.5+ exact compilation omits the masked duplicate local; glob and preceding-continuation controls retain that body. The subject child is prepared before reached arm bodies. C8.4 lacks this compiler and rejects the expansion case. These are selected compiler source visits, not runtime arm execution.

## Scope

Five C releases, six exact bodies in probe.c, result bytes and counted local layouts after original procedure calls.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/manifest.json](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/manifest.json)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.4.20.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.4.20.tsv)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.5.19.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.5.19.tsv)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.6.18.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.6.18.tsv)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.0.4.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.0.4.tsv)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.1.0.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.1.0.tsv)
- [rust/tcl-compiler/tests/data/native_switch_compiler_sources/probe.c](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/probe.c)
