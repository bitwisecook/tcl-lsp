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

## Exact evidence

- `receipt-0` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-manifest.json](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-manifest.json). SHA-256 `d02d00f359923e7b92c237e60e8b04f1fd0d60679f05d7104e589860008c1ea2`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.4.20.txt). SHA-256 `886324bbd54b9ef0adba38c619eae5f3e6c18763b64cf07d53c36a2d2231026d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.5.19.txt). SHA-256 `55d95eaf1d1a803743c7cbb087bfb912d281bbb14c6ad0b21d218bf2c66df0f7`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/8.6.18.txt). SHA-256 `db4feff25349b337956ff14fe3e7b0bf93526b33a1d6f6c9fd9239bc6cb3f027`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.0.4.txt). SHA-256 `db52b8f404c024ca70b2511f3422b54f5fb471f17303e684cfa384f0bce508a4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/9.1.0.txt). SHA-256 `db52b8f404c024ca70b2511f3422b54f5fb471f17303e684cfa384f0bce508a4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-operand-cache-probe.c](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-operand-cache-probe.c). SHA-256 `ccab657a7d348899848e26017b9347a4e36edc1c0147239d684b6c07000fb760`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-original-class-probe.c](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/c-original-class-probe.c). SHA-256 `a20fd32df772c0b4a4e2bce31656a8d5541a43f7b6e4f9a866e807a5c82f3907`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv). SHA-256 `29a2649982b1c70722638b462e5543208ebc918ca0b64113d697c3588745444a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.4.20.txt). SHA-256 `7e9f03c23d6a26d31921d6913f880bf837c919e96b5472d6effe1a2a85ac66f9`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.5.19.txt). SHA-256 `2e29f1469c0a73c2ec09181a6c7d5474e7e3a24cbb58898e09d3a0c58691d27d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-8.6.18.txt). SHA-256 `2e29f1469c0a73c2ec09181a6c7d5474e7e3a24cbb58898e09d3a0c58691d27d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.0.4.txt). SHA-256 `2e29f1469c0a73c2ec09181a6c7d5474e7e3a24cbb58898e09d3a0c58691d27d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/original-class-9.1.0.txt). SHA-256 `2e29f1469c0a73c2ec09181a6c7d5474e7e3a24cbb58898e09d3a0c58691d27d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
