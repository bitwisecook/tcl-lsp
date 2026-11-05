# naming.expression-abs-handler-replacement

Kind: `native-observation`

## Problem statement

The same expression function spelling can select a fixed builtin table or a mutable ordinary command. A generic PURE declaration or unchanged syntax cannot decide which handler survives replacement.

## Question

Does replacing ::tcl::mathfunc::abs affect the original expr abs(-3) call?

## Conclusion

C8.4 and Jim retain result3 after command replacement; C8.5–9.1 return the replacement99. The captured source establishes this script-visible dispatch distinction, not physical handler lifetime or a current registry binding receipt.

## Scope

Exact math_binding.tcl source on six retained shell providers with stock/replaced outputs; no BIG-IP observation.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_conditional_expression_results](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results)
