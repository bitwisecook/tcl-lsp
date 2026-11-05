# naming.compiled-expression-constant-storage

Kind: `native-observation`

## Problem statement

A constant-folded value can share a registered literal, standalone result or nested context object. Equivalent value text is insufficient to choose the original storage lineage.

## Question

What original literal/result storage survives the40 constant expression compiler sources?

## Conclusion

The /sources/2 native windows preserve the original registered constant/result storage for the exact40 sources. They do not establish a fresh-literal-world or source-name byte producer outside those registered windows.

## Scope

Exact storage40.c/storage40.tsv and /sources/2 metadata; five C original storage probes.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
