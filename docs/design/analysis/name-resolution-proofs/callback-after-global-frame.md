# naming.callback.after-global-frame

Kind: `native-observation`

## Problem statement

An after idle script is scheduled by a procedure in ::A with competing global and ::A callback names. Reusing the installer namespace would choose a different delayed implementation; timing alone says when it runs, not where its head resolves.

## Question

When the exact ::A::queue schedules after idle delayed and update reaches it, which namespace resolves delayed?

## Conclusion

All six retained providers select the root delayed procedure and report namespace ::. C update returns empty while Jim update returns the callback list; the lookup conclusion does not equate those results. This is reached deferred-global lookup, not proof that any queued callback must run.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source file through selected native CLI. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

after 0 {} {ROOT 1 ::}

### jim

Status: `observed`. Channel: ASCII source file through selected native CLI. Version: jim 0.84-9-g5bac7c9.

after 0 {ROOT 1 ::} {ROOT 1 ::}

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_callback_lookup_scope](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope)
