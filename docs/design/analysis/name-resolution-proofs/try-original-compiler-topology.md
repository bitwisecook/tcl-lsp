# naming.try-original-compiler-topology

Kind: `native-observation`

## Problem statement

Computed handler selectors, continuation bodies, trap prefixes and finally overrides can change both compiler admission and runtime completion. A syntactic handler role must not supply a compiled-local or completion receipt.

## Question

How do the original try sources 19 through 39 select handler locals, bytecode ranges and completion results?

## Conclusion

The retained C tables distinguish each original try/finally/on/trap source, local layout, instruction/range selection and completion result. Pre-try releases retain guest command failures rather than a fabricated exception compiler. No resulting source role proves that a constructor or callback completes.

## Scope

C Tcl releases explicitly identified in the retained provider rows. Jim and BIG-IP are not tested unless a separate row identifies them. Cases19–39 of the unchanged40-source procedure probe, genuine compiled layouts after the call and exact result hex.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 a1ddd2764f42b4995ff0bcfe4c0696ff902d138b2abac5c0ffc07d462330898c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 bdbe07c4fcd31ff063c0ce538355d0bdfe9f325013388c4fb03ff45efcb45744. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 6dd6dd0b9f284741eb3a35baacf6d7360b4c76613fb95424779a5a6712efaabf. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 4468ea72e86fbadc1fcd1fda4407e3ce953f6850a0bd74463579fd55d5d2897d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 feced532ab4437f6f95d1d35e14741901e0dd0f722c30f7ba2f637ab3ddf4be8. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [runtime/rust/tests/data/native_compiled_try/manifest.json](../../../../runtime/rust/tests/data/native_compiled_try/manifest.json). SHA-256 `8e40366c41a58765d7bd979a1c92d02d4a0895317a60e248c17c40b197c15706`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [runtime/rust/tests/data/native_compiled_try/8.4.20.tsv](../../../../runtime/rust/tests/data/native_compiled_try/8.4.20.tsv). SHA-256 `f36ba33c70fb4b121e481a0009ee28fc2d78037e367b9730a5fbaaa723475b87`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [runtime/rust/tests/data/native_compiled_try/8.5.19.tsv](../../../../runtime/rust/tests/data/native_compiled_try/8.5.19.tsv). SHA-256 `a7bbc99f12ddb619fae32195cc6989aa1e781d9c653c6f15c2750cad25d0ce3e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [runtime/rust/tests/data/native_compiled_try/8.6.18.tsv](../../../../runtime/rust/tests/data/native_compiled_try/8.6.18.tsv). SHA-256 `7986af9bc523b824a08d4e02555e7a2e9e9fbf3a201a96c520528b4a95e5f336`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [runtime/rust/tests/data/native_compiled_try/9.0.4.tsv](../../../../runtime/rust/tests/data/native_compiled_try/9.0.4.tsv). SHA-256 `7986af9bc523b824a08d4e02555e7a2e9e9fbf3a201a96c520528b4a95e5f336`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [runtime/rust/tests/data/native_compiled_try/9.1.0.tsv](../../../../runtime/rust/tests/data/native_compiled_try/9.1.0.tsv). SHA-256 `44de70afa64cece3a9779342667b1317de08c3936ddffb7fcf4270525528afc4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [runtime/rust/tests/data/native_compiled_try/cases.tsv](../../../../runtime/rust/tests/data/native_compiled_try/cases.tsv). SHA-256 `cc04483564258bb877e4c02f4f3a0af0ea1dc476c5d63af915a874c147df3fc5`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [runtime/rust/tests/data/native_compiled_try/probe.c](../../../../runtime/rust/tests/data/native_compiled_try/probe.c). SHA-256 `d94eb349ae2638672110fd183d15ca719c7b98d4d3039cd939a5158209902edf`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `receipt-8` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/manifest.json). SHA-256 `8e40366c41a58765d7bd979a1c92d02d4a0895317a60e248c17c40b197c15706`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-9` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv). SHA-256 `f36ba33c70fb4b121e481a0009ee28fc2d78037e367b9730a5fbaaa723475b87`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv). SHA-256 `a7bbc99f12ddb619fae32195cc6989aa1e781d9c653c6f15c2750cad25d0ce3e`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv). SHA-256 `7986af9bc523b824a08d4e02555e7a2e9e9fbf3a201a96c520528b4a95e5f336`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv). SHA-256 `7986af9bc523b824a08d4e02555e7a2e9e9fbf3a201a96c520528b4a95e5f336`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv). SHA-256 `44de70afa64cece3a9779342667b1317de08c3936ddffb7fcf4270525528afc4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-registry/tests/data/native_each_try_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/cases.rs). SHA-256 `3b2d7606ee9fe6d08828e0470cea35c248aa9fde9b428d924cb6a6b000e52c97`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (input): [rust/tcl-registry/tests/data/native_each_try_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_each_try_compilation/probe.c). SHA-256 `d94eb349ae2638672110fd183d15ca719c7b98d4d3039cd939a5158209902edf`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
