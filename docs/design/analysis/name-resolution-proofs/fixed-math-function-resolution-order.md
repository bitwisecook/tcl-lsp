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

Status: `observed`. Version: 8.4.20, c-8.4.20. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19, c-8.5.19. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18, c-8.6.18. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/manifest.json](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/c-8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/c-8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-vm/tests/data/native_fixed_math_table/jim.tsv](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/jim.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (input): [rust/tcl-vm/tests/data/native_fixed_math_table/probe.c](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (input): [rust/tcl-vm/tests/data/native_fixed_math_table/probe.tcl](../../../../rust/tcl-vm/tests/data/native_fixed_math_table/probe.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
