# naming.concat-expanded-native-source-shape

Kind: `native-observation`

## Problem statement

A literal expansion marker, parsed expansion word and runtime expanded argument list are different producers. Reconstructing argv from source text can change grammar, allocation and result identity.

## Question

How do native parser and compiler routes treat the original concat expansion-shaped sources and their result owners?

## Conclusion

The recovered C/Jim probe outputs retain sourcehex, parser completion and result/cache identity windows separately. C8.4 parser failures and other release-specific expansion results remain exact. Successful source parsing or concat completion does not authenticate a captured argv producer.

## Scope

Original probe.c/jim-probe.c sources and complete six-provider native output; native source shape and value/physical result windows kept distinct.

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

- [rust/tcl-registry/tests/data/native_concat_expansion](../../../../rust/tcl-registry/tests/data/native_concat_expansion)
