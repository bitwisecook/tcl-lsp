# naming.folded-boolean-original-literal-lineage

Kind: `native-observation`

## Problem statement

The value true can belong to different pooled literals or result objects. Treating all equal boolean bytes as one native literal registration can donate residency/reference roles.

## Question

Does a folded original boolean result share the first registered literal at the eight retained identity windows?

## Conclusion

The exact same_result/same_first_literal windows retain per-release literal/result identity; differing C8.5 and later original windows are preserved. Boolean value equality alone does not establish original literal lineage.

## Scope

C8.5–9.1 folded-boolean8.c and8 native windows; matching private headers, exact original identity flags.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
