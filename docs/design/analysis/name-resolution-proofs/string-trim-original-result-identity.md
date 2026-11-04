# naming.string-trim-original-result-identity

Kind: `native-observation`

## Problem statement

No-cut trim can yield equal bytes through a fresh string or the same subject. C bytearrays and Jim counted strings are different constructors, and the result getter can conceal residency differences.

## Question

When can the original interpreted or compiled trim route return its subject unchanged, and what header/string state exists before a getter?

## Conclusion

The exact trim/trimleft/trimright windows retain original subject/result/cache/refcount identity separately for direct and procedure routes. C85 lacks private trim hooks; C86/C9 no-cut compiled routes preserve the subject while interpreted C trims create a fresh string. Earlier C9 observer compilation failures remain harness limitations. Jim uses its independently captured constructor.

## Scope

Exact native C bytearray and Jim counted-string probe inputs, including raw00/rawFF; headers captured before result getter. This is not Document Unicode ingress.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.4.20.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.5.19.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.6.18.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/9.0.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/9.1.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/cases.rs). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim-probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/windows.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `receipt-11` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim-manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim-manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `receipt-12` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/attempt1-manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/attempt1-manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
