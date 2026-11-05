# naming.nested-expression-syntax-failure-order

Kind: `native-observation`

## Problem statement

An expression function containing an incomplete Tcl command can be rejected by the script delimiter parser, the outer expression parser or function validation. Reporting a live substitution or normal evaluation from recovery spans could invent an operand effect before the real rejection.

## Question

Which delimiter failure is reported before an absent command bracket in each original malformed nested expression, and can the inner set run?

## Conclusion

All retained cases return an error and leave x equal to NONE. C reports the inner brace, quote or variable failure where applicable; C8.5+ adds expression context. Jim has its distinct unmatched-bracket/function-tree diagnostics. The exact rows retain these differences, not a uniform diagnostic or successful operand execution.

## Scope

Seven exact ASCII expressions evaluated on five C releases and the recorded Jim provider, with full diagnostic bytes and unchanged x. No BIG-IP observation.

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

- [rust/tcl-syntax/tests/data/native_nested_expression_syntax](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax)
