# naming.concat-original-list-storage

Kind: `native-observation`

## Problem statement

Equal concatenated text does not establish original list identity, child ownership or backing reuse. Direct calls and compiled procedure calls can produce different original storage under the same source name.

## Question

How do original direct and procedure concat calls preserve or replace list headers, backing storage, children and result bytes for the retained inputs?

## Conclusion

The retained C and Jim tables report the original before/result/after/value windows separately, preserving storage and child identity flags and exact result values. These measurements apply to the probe constructor and call route; they do not justify copy-on-write or opcode reference counts in another route.

## Scope

Exact C/Jim native constructors and direct/procedure concat observer windows in the recovered probes. The retained probe and observer order define this question. Semantic values, cache classes, physical identity and completion applicability remain separate; these observations grant no authority to another producer or callback context.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 e00c9165ff58879881c10f4c1be4d7b5a4f095b78028543f0329b5e359ec7335. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 5a638bc58a70492b0417ad6e6c94402137ea4c82a90b5061ce1f81285ae006d7. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 8f899f7280bcfff4c69ed09722502c80eb711fb7462c960b21dfb97f646e7a0f. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 1737848d2af824280455ea548b0b9e1feb1617d6c718c5f06e4ce3bb9e2cac83. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 2cc4bb74066f14036befd5cda7112a4b311764df634ba5e033fb7d3460011805. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim0.84. Build: Recorded executable SHA-256 5ead9860ae65428fb9838edb22bed17d0ceb085409517b1015981ae438edf9ea. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-cmd-core/tests/data/native_concat/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_concat/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.4.20.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.5.19.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.6.18.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-cmd-core/tests/data/native_concat/9.0.4.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-cmd-core/tests/data/native_concat/9.1.0.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.4.20.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.5.19.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.6.18.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.0.4.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.1.0.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (input): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-cmd-core/tests/data/native_concat/arithseries.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-cmd-core/tests/data/native_concat/cases.rs](../../../../rust/tcl-cmd-core/tests/data/native_concat/cases.rs). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (input): [rust/tcl-cmd-core/tests/data/native_concat/jim-probe.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/jim-probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (observation): [rust/tcl-cmd-core/tests/data/native_concat/jim.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/jim.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (input): [rust/tcl-cmd-core/tests/data/native_concat/malformed.tcl](../../../../rust/tcl-cmd-core/tests/data/native_concat/malformed.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (input): [rust/tcl-cmd-core/tests/data/native_concat/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
