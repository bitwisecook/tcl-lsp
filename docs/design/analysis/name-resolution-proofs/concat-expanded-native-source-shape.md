# naming.concat-expanded-native-source-shape

Kind: `native-observation`

## Problem statement

A literal expansion marker, parsed expansion word and runtime expanded argument list are different producers. Reconstructing argv from source text can change grammar, allocation and result identity.

## Question

How do native parser and compiler routes treat the original concat expansion-shaped sources and their result owners?

## Conclusion

The recovered C/Jim probe outputs retain sourcehex, parser completion and result/cache identity windows separately. C8.4 parser failures and other release-specific expansion results remain exact. Successful source parsing or concat completion does not authenticate a captured argv producer.

## Scope

Original probe.c/jim-probe.c sources and complete six-provider native output; native source shape and value/physical result windows kept distinct.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim0.84. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/manifest.json](../../../../rust/tcl-registry/tests/data/native_concat_expansion/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_concat_expansion/cases.rs](../../../../rust/tcl-registry/tests/data/native_concat_expansion/cases.rs). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_concat_expansion/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_concat_expansion/jim-probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_concat_expansion/jim0.84.tsv](../../../../rust/tcl-registry/tests/data/native_concat_expansion/jim0.84.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_concat_expansion/probe.c](../../../../rust/tcl-registry/tests/data/native_concat_expansion/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
