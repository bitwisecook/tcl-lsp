# naming.try-source-completion-and-options

Kind: `native-observation`

## Problem statement

Return, finally override, handler continuation and error option handling can settle at different boundaries. A root result is not the raw producer completion or private option identity.

## Question

What completion/result does each of the fifteen original try sources expose after native root evaluation and the explicit inner catcher?

## Conclusion

The exact fifteen-case tables retain native code/result fields captured before observer queries. C8.4/8.5 record try unavailability; executable structured-exception controls apply to C8.6/C9/Jim. Only the stated root and inner-catch observation boundaries are covered.

## Scope

Original public C/Jim EvalObjEx sources in probe.c and exact cases.tsv vectors. Private raw completion/result ownership requires another receipt.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim Jim.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "rows": 15}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_try](../../../../rust/tcl-vm/tests/data/native_try)
