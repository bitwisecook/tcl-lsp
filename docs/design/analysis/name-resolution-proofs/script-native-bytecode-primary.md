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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "completed_windows": 10, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_body_bytecode/manifest.json](../../../../rust/tcl-vm/tests/data/native_body_bytecode/manifest.json)
- [rust/tcl-vm/tests/data/native_body_bytecode/8.4.20.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.4.20.txt)
- [rust/tcl-vm/tests/data/native_body_bytecode/8.5.19.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.5.19.txt)
- [rust/tcl-vm/tests/data/native_body_bytecode/8.6.18.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/8.6.18.txt)
- [rust/tcl-vm/tests/data/native_body_bytecode/9.0.4.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/9.0.4.txt)
- [rust/tcl-vm/tests/data/native_body_bytecode/9.1.0.txt](../../../../rust/tcl-vm/tests/data/native_body_bytecode/9.1.0.txt)
- [rust/tcl-vm/tests/data/native_body_bytecode/probe.c](../../../../rust/tcl-vm/tests/data/native_body_bytecode/probe.c)
