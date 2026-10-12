# naming.try-source-completion-and-options

Kind: `native-observation`

## Problem statement

Return, finally override, handler continuation and error option handling can settle at different boundaries. A root result is not the raw producer completion or private option identity.

## Question

What completion/result does each of the fifteen original try sources expose after native root evaluation and the explicit inner catcher?

## Conclusion

The exact fifteen-case tables retain native code/result fields captured before observer queries. C8.4/8.5 record try unavailability; executable structured-exception controls apply to C8.6/C9/Jim. Only the stated root and inner-catch observation boundaries are covered.

## Scope

Original public C/Jim EvalObjEx sources in probe.c and exact cases.tsv vectors. Private raw completion/result ownership requires another receipt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 4dbb31a11b72ea9b37db8ff74bb5ea616143faec52d3864e32c6ec8a55816548. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 a964ed255598f0eec3768bc1da085897c8d355b3819a1e8557e8f6216703865c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 e8347065e05c6d4523508af99c917912ba7156a06eeb981ae1babde21411f1aa. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 9fa6427b4948e1b15823ac370e0bf1a9425f33634f9ebbe9b9d2b465ef11bdfc. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 c58849f1451bb5c79b40a9b1abb7ba96cab5cd15beaa9f8921bcba01601d2790. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: Jim. Build: Recorded executable SHA-256 59123fb78f467423b9e5c06ba2b25f70e0b37f029ba46a79741709a97bc4516a. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-vm/tests/data/native_try/manifest.json](../../../../rust/tcl-vm/tests/data/native_try/manifest.json). SHA-256 `6ba5f2297cacbe6b195f54fe9e78c8b26c2930142e3f3060ca67122c1eca69bb`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-vm/tests/data/native_try/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_try/8.4.20.tsv). SHA-256 `1205791ecbb4c03a6dc3786e1d6baf31c55c52ad2353166cf90e23c41ebfd31a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-vm/tests/data/native_try/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_try/8.5.19.tsv). SHA-256 `1205791ecbb4c03a6dc3786e1d6baf31c55c52ad2353166cf90e23c41ebfd31a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-vm/tests/data/native_try/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_try/8.6.18.tsv). SHA-256 `f477449ed4040da4f98a419c6f66b2fbf92872d6d47b201fe9364674d1a91735`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-vm/tests/data/native_try/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_try/9.0.4.tsv). SHA-256 `f477449ed4040da4f98a419c6f66b2fbf92872d6d47b201fe9364674d1a91735`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-vm/tests/data/native_try/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_try/9.1.0.tsv). SHA-256 `f477449ed4040da4f98a419c6f66b2fbf92872d6d47b201fe9364674d1a91735`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-vm/tests/data/native_try/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_try/Jim.tsv). SHA-256 `8f6fc54a446b04c66e928162a83fae7511d67a8e36debbd691c063967f501e68`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-vm/tests/data/native_try/cases.tsv](../../../../rust/tcl-vm/tests/data/native_try/cases.tsv). SHA-256 `f65f4c5226648a7a736369c9b8d777a21eadcb35ebec6e517c6ad39d2854fbc1`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (input): [rust/tcl-vm/tests/data/native_try/probe.c](../../../../rust/tcl-vm/tests/data/native_try/probe.c). SHA-256 `070e7aad084bd310d07f6aad1dd826f660b0a0bf6a1328cab3f1dadd060f7791`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
