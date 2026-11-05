# naming.numeric-comparison-original-cache-class

Kind: `native-observation`

## Problem statement

A normal relational result can come from string comparison, so semantic numeric text or completion alone does not prove the original operand cache was converted.

## Question

Does relational comparison convert every retained original cache class containing the spelling2 to Int?

## Conclusion

String, one-element List and ASCII ByteArray2 become Int; original Integer remains integer and Double remains Double. Dict2 3 can complete through string comparison without changing Dict; C8.4 uses a two-element List because that Dict constructor is absent. These30 same-object windows do not supply arbitrary current cache provenance.

## Scope

Five C releases, six original native cache constructors per release, exact c-original-class-probe.c/output digests; no Jim or BIG-IP result.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_numeric_operand_conversions](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions)
