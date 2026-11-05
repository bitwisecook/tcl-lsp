# naming.tcl91-syntax-message-lifecycle

Kind: `native-observation`

## Problem statement

A retained message can be shared by result, private errorInfo, variable errorInfo, return options or literal table at different times. Resetting one owner cannot retire every other live role.

## Question

How do the six Tcl9.1 reset/options/errorInfo windows retain or release the original syntax message?

## Conclusion

The exact six windows retain each original reference and same-message role across reset and publication. This is a C9.1 lifecycle observation for this producer and lease protocol, not a generic source message refcount rule.

## Scope

C9.1-only syntax-lifecycle.c and columns explicitly declared in the manifest; original private roles and table counts.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
