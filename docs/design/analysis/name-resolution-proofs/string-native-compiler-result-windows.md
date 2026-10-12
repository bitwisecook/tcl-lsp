# naming.string-native-compiler-result-windows

Kind: `native-observation`

## Problem statement

A string operation result may reuse an operand or a pooled execution constant and thereby alter later residency. The original compiler result window is not the same as a fresh getter or semantic value alone.

## Question

Which native instruction/result-header windows do the sixty original string cases retain under the separately pinned observer?

## Conclusion

The successful separately pinned captures preserve the60 exact case windows and selected streq/strmatch instructions. Cases run in order in one interpreter, so earlier result observation can affect later pooled residency. Failed initial builds and a separately retained crashing C84 observer are retained as limitations, not successful native evidence.

## Scope

Five C builds with matching private compiler headers. Current probe/cases/windows and full manifest, including initial observer failures; physical counts apply only at these named windows.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 1e0b0c7f8fcde9b500a9aeaeca1f6f42f4a457669ae713a1b87b9e3953ccbeaa. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 194d25232ff6d331c516577d6d6ed5d66ced82cbe893c2990dc6a0bfa880882a. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

3 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 a7fe22a3ec0dfb92dc6f3ff98f5d57026bc65842aac87f830ecafdb4350ddc32. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

3 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 c9fd065c2934947ef63ff8fc6be4bcf77cd4a974f3e0e979b47a670eb61f152c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

3 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 ecbdd2367819b662fc6df2b69ed62c7df46e01e24dc987ec54d101cb3dd366d5. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

3 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_string_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_string_compilation/manifest.json). SHA-256 `56a0d3efebbf7bfe714f220a1260b44bae0ec98f8e29acbffe87d5a18756c8f0`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.4.20.txt). SHA-256 `f224b961ef897044e592dafa135ae8eee44447cb6f5c829f6b3a6d56e04ece9a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.5.19.txt). SHA-256 `66dc2cb6c4b9db695664b7c11b393ea2395543ee916f6f80e08638392c7c590a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/8.6.18.txt). SHA-256 `44b35ff29b9d71eadb214cc66324d26d3dc75c50beb2e95d8007d0ad0b11d4cd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.0.4.txt). SHA-256 `44b35ff29b9d71eadb214cc66324d26d3dc75c50beb2e95d8007d0ad0b11d4cd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_string_compilation/9.1.0.txt). SHA-256 `c30347e3e72d656297b636a1017da386b28055e7d137bc356712266f6a214a0b`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/native_string_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_string_compilation/probe.c). SHA-256 `059a12e50297aae6a683af7a96e7904176e8db3e518b586964d4670727e5c5ff`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-registry/tests/data/native_string_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_string_compilation/windows.tsv). SHA-256 `4057e573d42d71d6af5a335695d14b269b802b779f1567743a87bded38136034`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `receipt-8` (observation): [rust/tcl-registry/tests/data/native_string_compilation/cases.json](../../../../rust/tcl-registry/tests/data/native_string_compilation/cases.json). SHA-256 `41a3bdd510da707d7a6558e9089229917259ebe1b0477b586fbffc6ef4917769`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
