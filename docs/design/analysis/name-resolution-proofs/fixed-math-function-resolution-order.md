# naming.fixed-math-function-resolution-order

Kind: `native-observation`

## Problem statement

Tcl8.4 fixed function tables and modern mutable math commands differ in admission and argument order. The same-spelled abs procedure or custom function table cannot donate an entered handler to another call.

## Question

Are fixed math function names/arity selected before substituted arguments, and how do command or fixed-table replacement controls affect later calls?

## Conclusion

The source controls retain C8.4 fixed-table validation before arguments and its entered builtin across argument-time replacement; later selection sees the replacement. C8.5+ uses ordinary math-command dispatch after argument evaluation. Tcl_CreateMathFunc controls apply only where the real API exists; C9 lacks that API. Jim retains its own measured fixed surface.

## Scope

Separate original probe.tcl shell outputs and C probe.c custom function controls for C8.4–8.6; no C9 API fabrication or BIG-IP claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20, c-8.4.20; tcl8.5 8.5.19, c-8.5.19; tcl8.6 8.6.18, c-8.6.18.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_fixed_math_table](../../../../rust/tcl-vm/tests/data/native_fixed_math_table)
