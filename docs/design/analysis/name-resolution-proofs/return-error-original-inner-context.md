# naming.return-error-original-inner-context

Kind: `native-observation`

## Problem statement

An Error result may share or differ from its registered message and inner context. Reading options or errorInfo after the primary snapshot can change those original owners.

## Question

Which original message/literal/innerContext identity survives the fifteen ReturnImmediate and compiled Error bodies?

## Conclusion

The exact return-context rows preserve original result/literal/innerContext identity for each selected native body. C8.4 absence of modern context rows is retained rather than filled by a later release. These windows do not certify an arbitrary source Return or callback completion.

## Scope

Five C matching-private-header probes of return-context.c, original15 bodies, snapshots before observers.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable SHA-256 5d5ee8e13f9cb7d1aab37a2a7937a529518ec33c2102279566040986aac8260a. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable SHA-256 10eea70cd9f4d0e84265a3e2edef789d9fb9c3ece059d5cee4b6c7cc1bcbc9cf. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable SHA-256 324924e1d75c46eefc63dd5506b9a811619664b1d14d760a4c67ca577ed0eb67. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable SHA-256 2941ef226a2dbdcc89d5cd9e1b43b8c745a9df7e0242ae5c1fb6cb6e93e1aa7d. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable SHA-256 8a866bc47488a322cc5680e11f7d6b038915215df983707ea4ac42625eff70f7. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"context_rows": 9, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-manifest.json](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_control_expression/catch-publication6.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/catch-publication6.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_control_expression/catch4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/catch4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (input): [rust/tcl-registry/tests/data/native_control_expression/compound40.tcl](../../../../rust/tcl-registry/tests/data/native_control_expression/compound40.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_control_expression/compound40.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/compound40.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (input): [rust/tcl-registry/tests/data/native_control_expression/control75.tcl](../../../../rust/tcl-registry/tests/data/native_control_expression/control75.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-registry/tests/data/native_control_expression/control75.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/control75.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (input): [rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.c](../../../../rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (input): [rust/tcl-registry/tests/data/native_control_expression/logical84.c](../../../../rust/tcl-registry/tests/data/native_control_expression/logical84.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_control_expression/logical84.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/logical84.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (observation): [rust/tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (input): [rust/tcl-registry/tests/data/native_control_expression/return-context.c](../../../../rust/tcl-registry/tests/data/native_control_expression/return-context.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (input): [rust/tcl-registry/tests/data/native_control_expression/storage40.c](../../../../rust/tcl-registry/tests/data/native_control_expression/storage40.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (observation): [rust/tcl-registry/tests/data/native_control_expression/storage40.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/storage40.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-19` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-20` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-21` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-22` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-23` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-24` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-context.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-context.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-25` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle-9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-26` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-lifecycle.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-27` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-28` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-29` (observation): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners-9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-30` (input): [rust/tcl-registry/tests/data/native_control_expression/syntax-owners.c](../../../../rust/tcl-registry/tests/data/native_control_expression/syntax-owners.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
