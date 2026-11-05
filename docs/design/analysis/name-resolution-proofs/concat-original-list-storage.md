# naming.concat-original-list-storage

Kind: `native-observation`

## Problem statement

Equal concatenated text does not establish original list identity, child ownership or backing reuse. Direct calls and compiled procedure calls can produce different original storage under the same source name.

## Question

How do original direct and procedure concat calls preserve or replace list headers, backing storage, children and result bytes for the retained inputs?

## Conclusion

The retained C and Jim tables report the original before/result/after/value windows separately, preserving storage and child identity flags and exact result values. These measurements apply to the probe constructor and call route; they do not justify copy-on-write or opcode reference counts in another route.

## Scope

Exact C/Jim native constructors and direct/procedure concat observer windows in the recovered probes. The retained probe and observer order define this question. Semantic values, cache classes, physical identity and completion applicability remain separate; these observations grant no authority to another producer or callback context.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim jim0.84.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_concat](../../../../rust/tcl-cmd-core/tests/data/native_concat)
