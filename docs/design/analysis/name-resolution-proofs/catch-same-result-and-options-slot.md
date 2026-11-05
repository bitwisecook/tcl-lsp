# naming.catch-same-result-and-options-slot

Kind: `native-observation`

## Problem statement

Pairwise result/options stores can overwrite one another and differ by release. A final variable value cannot prove every intermediate store or that the command completed normally.

## Question

What happens when catch stores its result and return options in the same variable in the original four release controls?

## Conclusion

C8.5 returns the captured dictionary code1; C8.6/C9 return the exact missing-value dictionary error for the original same-slot source. This is the native order/result of these stores, not a general no-error write proof.

## Scope

Four C sourcehex/code/resulthex records, C8.5–9.1 only; no C8.4, Jim or BIG-IP result.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.5

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.5 8.5.19.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"code": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"code": 1, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
