# naming.reached-original-numeric-operand-cache

Kind: `native-observation`

## Problem statement

Successful comparison or index lookup may use fallback/string comparison, convert an extracted list child, or keep a Double/offset cache. It cannot be treated as a uniform Wide getter on the original object.

## Question

Which cache does the retained original operand acquire when reaching numeric comparison, lrange or lindex under each original input class?

## Conclusion

The210 retained original-object windows distinguish numeric comparison, runtime range/index and primed-list index routes. C8.4 huge comparison may retain String while later C publishes Bignum; grouped list conversion can leave the parent List and end-1 uses an offset cache. Normal result alone is not an integer conversion or increment recipe.

## Scope

Five C releases, c-operand-cache-probe.c exact constructors/routes; original operand retained before the operation, no Jim/BIG-IP measurement.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_numeric_operand_conversions](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions)
