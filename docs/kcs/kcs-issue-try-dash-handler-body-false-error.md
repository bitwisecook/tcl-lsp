# KCS: `try` handler with a `-` body reports a spurious "too few arguments" error

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors, tcl-lsp-cli, lowering

## Symptom

Reported in issue #703. A `try` command whose `on`/`trap` handler body is a
bare `-` (a fallthrough marker that shares the *next* handler's body, exactly
as `switch` shares pattern bodies) was flagged with a false-positive error:

```
E002  Too few arguments for '-': expected at least 1, got 0
```

The error landed on the solo `-`, for example the `-` in
`on ok result - trap NONE result { … }`. The reported position was correct;
the diagnostic was not. This is valid, working Tcl — it appears in Tcl's own
`tools/findBadExternals.tcl`:

```tcl
try {
    switch $::tcl_platform(platform) {
        unix - macosx { exec nm --extern-only --defined-only $libtcl }
        windows       { exec dumpbin /exports $libtcl }
    }
} on ok result - trap NONE result {
    ...
    return 0
} on error msg {
    ...
    return 1
}
```

## Operational context

The lowerer turned every `try` handler body into a nested script. A handler
body of `-` was therefore lowered as a script containing the single command
`-`, which the registry knows as the arithmetic-minus operator. With no
operands, the generic arity check (E002) fired.

Tcl's `TclNRTryObjCmd` (in `tclCmdMZ.c`) recognises a handler body of `-` by
its string value and treats the clause as a fallthrough: when the clause
matches, the runtime scans forward to the next handler whose body is not `-`
and runs that body with that handler's variable bindings. The last
non-`finally` clause must not be `-` (there is nothing to fall through to). The
`switch` command uses the same mechanism for pattern bodies, and the lowerer
already handled `switch` correctly — `try` was the gap.

## Decision rules / contracts

1. A `try` handler body that is the literal `-` (single-token word, including
   the braced `{-}` form, which evaluates to the same string) is a
   **fallthrough**, not a script. The lowerer sets
   [`IRTryHandler.fallthrough`](../../compiler/ir.py) to `True` and gives the
   handler an empty `IRScript` body, so no command — and therefore no arity
   check — is synthesised for it.
2. `dialects/tcl/try_.py`'s `arg_role_resolver` gives a `-` handler body no
   `ArgRole.BODY`, mirroring `dialects/tcl/switch_.py`. A `-` body is never a
   nested script for semantic tokens or any other role consumer.
3. A genuine zero-argument `-` *command* in a real body (a true mistake) is
   still flagged E002. The suppression is scoped to the handler-body slot, not
   to the `-` command everywhere.
4. The set of commands with `-` fallthrough is exactly `{switch, try}` —
   confirmed against the Tcl 8.4–9.0 C sources. No other Tcl or Tk command
   uses it.
5. Like `switch`, the LSP does not flag the invalid "last non-`finally` clause
   is `-`" case; that rare runtime error is accepted silently rather than
   mis-reported.

## File-path anchors

- `compiler/lowering.py` — `IRLowerer._lower_try` (fallthrough detection)
- `compiler/ir.py` — `IRTryHandler.fallthrough`
- `dialects/tcl/try_.py` — `_try_arg_roles` (no BODY role for `-`)
- `dialects/tcl/switch_.py` — `_switch_arg_roles` (the mirrored precedent)

## Failure modes

- A future refactor lowers the handler body before checking for `-`, so the
  arity error returns.
- The `arg_role_resolver` is changed to mark the `-` body as `BODY` again,
  re-tagging it as an embedded script for semantic tokens.
- A new fallthrough-style command is added without the same handling (the
  investigation found none today, but a future Tcl version could add one).

## Test anchors

- `tests/test_issue_703_try_fallthrough.py` — both sides: the fallthrough `-`
  (and `{-}`) raises no E002 and lowers to a fallthrough handler with an empty
  body; a genuine zero-arg `-` command is still flagged.

## Related

- [KCS index](README.md)
- [control-flow patterns](../design/compiler/control-flow-patterns.md)
- [`else`/`on`/`trap` keyword highlighting (#637)](kcs-issue-shadowed-builtin-breaks-highlighting.md)
- [command registry](../design/compiler/command-registry.md)
