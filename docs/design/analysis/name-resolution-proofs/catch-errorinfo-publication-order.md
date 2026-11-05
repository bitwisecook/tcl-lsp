# naming.catch-errorinfo-publication-order

Kind: `native-observation`

## Problem statement

Catch settles a child error but can publish errorInfo and store result/options in an observable order. Ignoring these callbacks can preserve stale cell or name state.

## Question

Which errorInfo, result and options write callbacks run in the original catch error-publication source?

## Conclusion

The five C source controls retain exact traced publication/store callback order. Jim ordinary Catch completion and separate trace unavailability remain independent observations; no C trace callback is fabricated for Jim. Successful outer Catch does not erase child or store effects.

## Scope

Original per-provider ASCII source retained directly in JSON and catch-publication6.tsv; variable callback order, not physical object headers.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"code": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_control_expression](../../../../rust/tcl-registry/tests/data/native_control_expression)
