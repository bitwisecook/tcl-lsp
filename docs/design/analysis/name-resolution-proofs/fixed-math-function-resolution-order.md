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

### tcl8.4

Status: `observed`. Version: 8.4.20, c-8.4.20. Build: Recorded executable SHA-256 8c7e1e3a9a6af2f1f41688b4831da0302c1559fa537638d74b51e895cbdfb565. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19, c-8.5.19. Build: Recorded executable SHA-256 afdddcabb19565374ff6efab6e3e503e212a4561c2be25ffc9ed615e3c89c0be. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18, c-8.6.18. Build: Recorded executable SHA-256 9e9f7ceb16abb1a4e24f5cae0090a439d6eefe6566b648494afc588db03f9e43. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim. Build: Recorded executable SHA-256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/manifest.json](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/manifest.json). SHA-256 `bdb34b08da496047ff71084691e88232091e6c412d04a40203550b89134e2491`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.4.20.tsv). SHA-256 `6aa07cbd4fb5205b5df01ad1800417a7f669aeeb4ee053b1618a4d142583eddf`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.5.19.tsv). SHA-256 `809a556d1aa40033ed95c567a22153f4dbe0ed3a03b69c7f59a417cbc2640e3f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.6.18.tsv). SHA-256 `809a556d1aa40033ed95c567a22153f4dbe0ed3a03b69c7f59a417cbc2640e3f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/9.0.4.tsv). SHA-256 `809a556d1aa40033ed95c567a22153f4dbe0ed3a03b69c7f59a417cbc2640e3f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/9.1.0.tsv). SHA-256 `809a556d1aa40033ed95c567a22153f4dbe0ed3a03b69c7f59a417cbc2640e3f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.4.20.tsv). SHA-256 `e29cef1430103cc382b7c07b2e83e9459ffedbc1aff2be18e16127d2b0e6f9d1`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.5.19.tsv). SHA-256 `0d261fd08020c64f34c795a3df8a30e9e9d211c7bbcf671b43891f551c4a2b23`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.6.18.tsv). SHA-256 `72251939fde923b2f7df7d5d1369f6543b2c74da4ef8f6ac4f31df1825bf350c`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/jim.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/jim.tsv). SHA-256 `a462c40dcd7b6faa6cd14abef543ffd6d03e967f732f861e69922b07f63bdcfe`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (input): [rust/tcl-vm/tests/data/native_fixed_math_table/probe.c](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/probe.c). SHA-256 `f16415ed74aa2650280c7dd62070069102e575b48f62d443f8c7b1c7eec2df51`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (input): [rust/tcl-vm/tests/data/native_fixed_math_table/probe.tcl](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/probe.tcl). SHA-256 `e1741495d9ada3ff6df17f1da5f4f0cde6926e5d20d708c0ae3b0ca27d620232`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
