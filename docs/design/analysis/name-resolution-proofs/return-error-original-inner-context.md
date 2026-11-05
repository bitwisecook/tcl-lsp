# naming.return-error-original-inner-context

Kind: `native-observation`

## Problem statement

An Error result may share or differ from its registered message and inner context. Reading options or errorInfo after the primary snapshot can change those original owners.

## Question

Which original message/literal/innerContext identity survives the fifteen ReturnImmediate and compiled Error bodies?

## Conclusion

The exact return-context rows preserve original result/literal/innerContext identity for each selected native body. C8.4 absence of modern context rows is retained rather than filled by a later release. These windows do not certify an arbitrary source Return or callback completion.

## Scope

Five C matching-private-header probes of return-context.c, original15 bodies, snapshots before observers.

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
