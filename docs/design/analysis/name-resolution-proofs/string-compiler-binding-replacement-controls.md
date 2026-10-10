# naming.string-compiler-binding-replacement-controls

Kind: `native-observation`

## Problem statement

A builtin descriptor may remain in a catalogue after the real command has been renamed or replaced. Written argument roles do not prove the original native compiler is still registered.

## Question

How do the exact original string comparison registration/replacement sources affect results and available compiler selection?

## Conclusion

The retained per-release sources and stdout/stderr explicitly distinguish registration/replacement outcomes. These guest operations do not supply a current compiler prerequisite to unrelated calls.

## Scope

Five original C shell controls, with each source string and binary digest retained directly in the JSON rows.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_string_compilation/original-registration-controls.json](../../../../rust/tcl-registry/tests/data/native_string_compilation/original-registration-controls.json). SHA-256 `088a733197d3ea9b848b9d0f001eeffa413cbb48ec119ec491f4ab7fbd3ee1c2`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt). SHA-256 `f224b961ef897044e592dafa135ae8eee44447cb6f5c829f6b3a6d56e04ece9a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt). SHA-256 `66dc2cb6c4b9db695664b7c11b393ea2395543ee916f6f80e08638392c7c590a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt). SHA-256 `44b35ff29b9d71eadb214cc66324d26d3dc75c50beb2e95d8007d0ad0b11d4cd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt). SHA-256 `44b35ff29b9d71eadb214cc66324d26d3dc75c50beb2e95d8007d0ad0b11d4cd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt). SHA-256 `c30347e3e72d656297b636a1017da386b28055e7d137bc356712266f6a214a0b`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_string_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_string_compilation/probe.c). SHA-256 `059a12e50297aae6a683af7a96e7904176e8db3e518b586964d4670727e5c5ff`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-registry/tests/data/native_string_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_string_compilation/windows.tsv). SHA-256 `4057e573d42d71d6af5a335695d14b269b802b779f1567743a87bded38136034`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
