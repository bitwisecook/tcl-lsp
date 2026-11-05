# naming.string-trim-original-result-identity

Kind: `native-observation`

## Problem statement

No-cut trim can yield equal bytes through a fresh string or the same subject. C bytearrays and Jim counted strings are different constructors, and the result getter can conceal residency differences.

## Question

When can the original interpreted or compiled trim route return its subject unchanged, and what header/string state exists before a getter?

## Conclusion

The exact trim/trimleft/trimright windows retain original subject/result/cache/refcount identity separately for direct and procedure routes. C85 lacks private trim hooks; C86/C9 no-cut compiled routes preserve the subject while interpreted C trims create a fresh string. Earlier C9 observer compilation failures remain harness limitations. Jim uses its independently captured constructor.

## Scope

Exact native C bytearray and Jim counted-string probe inputs, including raw00/rawFF; headers captured before result getter. This is not Document Unicode ingress.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_string_trim_compilation](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation)
