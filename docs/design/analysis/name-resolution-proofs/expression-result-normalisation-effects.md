# naming.expression-result-normalisation-effects

Kind: `native-observation`

## Problem statement

An algebraic expression rewrite may produce the same number but run a custom operand twice instead of once. Numeric equivalence does not establish effect equivalence.

## Question

Can rewriting the original repeated operand expression preserve its values yet change a custom operand effect count?

## Conclusion

For the captured ordinary numeric/error sources the compared results agree; the custom original emits6 with count2 while rewritten emits6 with count1 on every retained provider. A normal numeric result alone cannot justify eliminating evaluation.

## Scope

Six retained ASCII normalisation.tcl shell runs with exact native outputs and executable digests; no original object-header or BIG-IP claim.

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
