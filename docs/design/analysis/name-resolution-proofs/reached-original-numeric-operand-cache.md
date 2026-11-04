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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 e435788258a29e55c12cf41da80718e7cd2250c0d7e367b73a62f7be182b61ca. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 89866a5407df17152ca457d3b0b7cfdaa6518e28cbe04df15d87d686b47fd0c0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 c4dd70932f2819e156a13b636a9547ef483f9f1a32b4f821c6bfae1476130b37. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 f14dcd6c2776305598e2e8a47003a261565dadab3149b86e14492e941ffc9cc9. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 e28062f5342967cfd98c1f8e5b76c1b45f336aeff9077a087fca39811eb26d07. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0, "rows": 42}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/manifest.json](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.4.20.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.5.19.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.6.18.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.0.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.1.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-operand-cache-probe.c](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-operand-cache-probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-original-class-probe.c](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-original-class-probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.4.20.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.5.19.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.6.18.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.0.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.1.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
