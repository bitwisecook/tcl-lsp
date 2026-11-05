# naming.quoted-tmm-operator-value

Kind: `native-observation`

## Problem statement

A source adapter could mistake a final escaped backslash or command-substitution component for the end of an evaluated quoted expression. The original TMM-style join source checks the actual value independently of parser token geometry.

## Question

What value does the original TMM-style quoted operator expression produce on the six retained interpreters?

## Conclusion

Every retained C and Jim run emits the same value bytes for this exact ASCII source. This checks this quoted source and its Tcl substitution result only; it does not establish BIG-IP appliance state or original object identity.

## Scope

Six script evaluations of probe.tcl; ASCII source through the recorded shell providers. Exact C patchlevels are identified by adjacent parse runs; script rows themselves retain engine and executable digests. Jim patchlevel is not recorded here.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-lexer/tests/data/native_quoted_word_geometry](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry)
