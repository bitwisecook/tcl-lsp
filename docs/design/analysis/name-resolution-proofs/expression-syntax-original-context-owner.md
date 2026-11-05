# naming.expression-syntax-original-context-owner

Kind: `native-observation`

## Problem statement

A compiler can register a syntax diagnostic without executing the rejected child. A message with equal text does not establish the registered literal or innerContext identity.

## Question

Which original syntax-message and innerContext objects are retained at the40 source-error storage windows?

## Conclusion

The40-source native rows retain primary storage and actual innerContext child identity with additive opcode observations. They are exact original syntax storage windows, not successful expression execution or arbitrary message ownership.

## Scope

Five C syntax-context.c probes plus per-release tables; original storage windows and checked actual private headers.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
