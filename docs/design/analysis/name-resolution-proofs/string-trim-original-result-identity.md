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

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 2de5d511c0ad6c4b06574059f8f6d59c44cc9a582413bbc8557400aaa7f50897. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 c0f9dd23f8b8fadf6ae194a2196dfee85f83f963aec8fc87ccf6ce2bf154d674. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 9906c19b8f9aa189f21ccd05ac9d75ffbb06caf7ed0d94e03caa75fa339d275d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 89978b19215aad2132392e225c30a585f3c7b7cd413607463a0cda761a7bda6c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 16fd60b49e5ca3073d712f66e008a944a3f6df242e94045909464be81b2c716a. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/manifest.json). SHA-256 `baeb0ced3aecf83036dc89d8398b5f6314aba34b937cb2f010301b76fd5c587c`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.4.20.txt). SHA-256 `2b5944b3f23e64c6a1857753c9dc70efbeb2a3458d8ae283a7000ab096d42c15`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.5.19.txt). SHA-256 `2b5944b3f23e64c6a1857753c9dc70efbeb2a3458d8ae283a7000ab096d42c15`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/8.6.18.txt). SHA-256 `52d4645a4747e4c8670a12e230d55e7ed7c63e9234d010f7f62117003777dabc`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/9.0.4.txt). SHA-256 `52d4645a4747e4c8670a12e230d55e7ed7c63e9234d010f7f62117003777dabc`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/9.1.0.txt). SHA-256 `2654e34a2754a98592b93211d88b114c36550ba7e3c75e7e88433fb2dd9aa147`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/cases.rs). SHA-256 `ad5be875a116845883fe5dab1e76242bc49fc1c012ba055a9f3e31312110e267`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim-probe.c). SHA-256 `8054c8e0775c909d178e1d66b7dd09d7fcd041f32a74dbec08666c040d315dd7`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim.txt](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim.txt). SHA-256 `f00c3620d6e453c983450cf9700ecfa2eaa47c8466167da0cb9d89e1169c15ee`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_string_trim_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/probe.c). SHA-256 `2949b6c638c1e7157b5448d6fd5cacdcad8d8b7c6a8acde34e569a4341c37ac3`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/windows.tsv). SHA-256 `d092c651b268160e3dd602b133fb455810f7731b82551bc56574ba49eee257dd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `receipt-11` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/jim-manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/jim-manifest.json). SHA-256 `93426112d250dce33dafdd23c2ff37d868448ddc810101d5ee10feaa45f43f9c`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `receipt-12` (observation): [rust/tcl-registry/tests/data/native_string_trim_compilation/attempt1-manifest.json](../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/attempt1-manifest.json). SHA-256 `b26266c08bba4c77a2d0139a23b195dbdd743e8ad317394c86845900dde04602`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
