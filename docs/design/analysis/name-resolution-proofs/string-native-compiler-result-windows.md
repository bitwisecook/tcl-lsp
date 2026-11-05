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

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

3 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_string_compilation](../../../../rust/tcl-registry/tests/data/native_string_compilation)
