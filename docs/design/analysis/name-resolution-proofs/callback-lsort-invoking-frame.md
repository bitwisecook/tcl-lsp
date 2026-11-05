# naming.callback.lsort-invoking-frame

Kind: `native-observation`

## Problem statement

A comparator named cmp exists in both the root namespace and ::A. Resolving the receiving lsort head or copying its prefix does not establish where the comparator itself will resolve; choosing the root table would navigate or analyze the wrong implementation.

## Question

In the exact selected lsort -command call inside ::A::sort, which namespace resolves an unqualified cmp callback?

## Conclusion

All six retained providers select ::A::cmp, whose callback reports its invoking caller at level1 in ::A. This establishes invoking-frame namespace lookup for this exact comparator call, independently of sort result identity, object caches and callback reach for other inputs.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII source file through selected native CLI. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

lsort 0 {b a} {A 1 ::A}

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_callback_lookup_scope](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope)
