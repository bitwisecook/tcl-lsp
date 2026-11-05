# naming.return-instruction-original-result-owners

Kind: `native-observation`

## Problem statement

A semantic Return or Error result can be confused with the exact retained message, option dictionary, stack alias or interpreter owner. Observing after a getter or reset may change the very cache and reference counts being tested.

## Question

Which original result, return-options and error-line objects survive the immediate, syntax, capture and reset instruction windows?

## Conclusion

The C8.5–9.1 probe records exact original identity/reference/cache flags at each named instruction window, with an explicit retained message lease across reset. These private owner observations do not certify unrelated Return operands or normal source completion; C8.4 and Jim are not observed here.

## Scope

Four C builds, matching private headers and the exact probe.c observer order. The observations.tsv columns are defined by the producer; no borrowed Native owner.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_return_instructions/manifest.json](../../../../runtime/rust/tests/data/native_return_instructions/manifest.json)
- [runtime/rust/tests/data/native_return_instructions/observations.tsv](../../../../runtime/rust/tests/data/native_return_instructions/observations.tsv)
- [runtime/rust/tests/data/native_return_instructions/probe.c](../../../../runtime/rust/tests/data/native_return_instructions/probe.c)
