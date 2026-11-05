# naming.quoted-complete-word-extents

Kind: `native-observation`

## Problem statement

A quoted word ending in an escape or bracketed fragment may have a parser component ending before its closing quote. Borrowing that component as the written word extent can truncate compiler/source correspondence and editor selection.

## Question

Does Tcl_ParseCommand retain the final complete word extent for quoted escape tails, command-fragment tails and empty quotes in the five original sources?

## Conclusion

All five C releases report a final top-level word end equal to the complete source length in every original case. This is word geometry, not evaluated value or compiler admission evidence. No Jim ParseCommand or BIG-IP parser result is attached.

## Scope

C Tcl releases explicitly identified in the retained provider rows. Jim and BIG-IP are not tested unless a separate row identifies them. Public Tcl_ParseCommand over the exact five ASCII C strings; original token sizes and word counts, not Unicode/native value conversion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-lexer/tests/data/native_quoted_word_geometry](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry)
