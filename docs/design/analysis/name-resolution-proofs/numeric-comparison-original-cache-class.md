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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 8fe5819e603914b0ed6fdfa7004622c2b97cf416c439678b81476235ce34c272. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 2322607c245950d9b34cccdc8d306cc8da8543b449f14eab8d88213c6206e6fd. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 4bfc72b7ed32568a55be531d2662a619aeb85ea904b548cf058bd24b1acdfc29. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 6e0c687fc03ffefedd234bb447bb7882867a8e011d7085076e28421aba89caef. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 04ff1e23a384a2c9754dc7a14834fcb4fb99345855c6da55516d701781e19b30. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-manifest.json](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
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
