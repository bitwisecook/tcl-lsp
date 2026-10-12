# naming.switch-original-compiler-source-visits

Kind: `native-observation`

## Problem statement

Exact-mode switch can omit a masked duplicate arm while glob mode or a prior continuation retains it. A source body list or all syntax spans cannot stand in for the actual selected compiler visit set.

## Question

Which exact arm bodies and local names does the native switch compiler visit in the six original procedure sources?

## Conclusion

C8.5+ exact compilation omits the masked duplicate local; glob and preceding-continuation controls retain that body. The subject child is prepared before reached arm bodies. C8.4 lacks this compiler and rejects the expansion case. These are selected compiler source visits, not runtime arm execution.

## Scope

Five C releases, six exact bodies in probe.c, result bytes and counted local layouts after original procedure calls.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 d3cbdef99d2541a1141608d5dd0dd7aa7602cd9caf32560849459cd46d88635d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 fda03e8724f2a3f0566c8d9c74d6f7ec100f66322941ced10ac48e3f366718bb. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 68eda92f609918b888c1fafb84ce186fb847c134df87635e01d5937073511096. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 7a19a5dc6e0bd94edbd386943800f7b8d5fe84c996f3a894ba853cc2e6600e07. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 4b2d1cdbd797092eed4977dcbcc3e42ec741d260d102b73552f0dfe04fde1029. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 6}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/manifest.json](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/manifest.json). SHA-256 `6f3ed264f48e67b2b37cdb5b711dac3cea43b6257c41246a6aa845e01c6ef3e7`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.4.20.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.4.20.tsv). SHA-256 `5616544a40565374b101d7b1ba86930b8619f27c6c27a45d0e19c39ce1937981`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.5.19.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.5.19.tsv). SHA-256 `ea5cfcfadc46ce83cc06932ed1d230b0f94d8d09405ab3fb2e4fd07d58680599`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.6.18.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/8.6.18.tsv). SHA-256 `ea5cfcfadc46ce83cc06932ed1d230b0f94d8d09405ab3fb2e4fd07d58680599`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.0.4.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.0.4.tsv). SHA-256 `ea5cfcfadc46ce83cc06932ed1d230b0f94d8d09405ab3fb2e4fd07d58680599`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.1.0.tsv](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/9.1.0.tsv). SHA-256 `ea5cfcfadc46ce83cc06932ed1d230b0f94d8d09405ab3fb2e4fd07d58680599`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-compiler/tests/data/native_switch_compiler_sources/probe.c](../../../../rust/tcl-compiler/tests/data/native_switch_compiler_sources/probe.c). SHA-256 `7c3f0b60f7e97c43342d353c920065e234e1f0c72d5b06892d43aacaf2a38108`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
