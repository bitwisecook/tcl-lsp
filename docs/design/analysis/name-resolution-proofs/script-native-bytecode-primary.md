# naming.script-native-bytecode-primary

Kind: `native-observation`

## Problem statement

A script can contain a parse error yet receive Bytecode that reports it only when reached. Equal text or a successful preceding Return does not establish the same compiled script storage.

## Question

What primary/cache/refcount remains on the original script object immediately before and after Tcl_EvalObjEx for the five body sources?

## Conclusion

C8.4 parse failures remain without Bytecode; C8.5–9.1 can retain Bytecode that reports the error when reached. A preceding Return can bypass the error. Empty and unknown-command source objects retain Bytecode in all measured C releases. These original script windows provide no Jim Script or unrelated body-entry grant.

## Scope

Five source strings and ten pre/post windows per C release, before result/option observers; exact public EvalObjEx plus private original object fields.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/manifest.json](../../../../rust/tcl-vm/tests/data/native_body_bytecode/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/8.4.20.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.4.20.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/8.5.19.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.5.19.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/8.6.18.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.6.18.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/9.0.4.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/9.0.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-vm/tests/data/native_body_bytecode/9.1.0.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/9.1.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-vm/tests/data/native_body_bytecode/probe.c](../../../../rust/tcl-vm/tests/data/native_body_bytecode/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
