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

Two independent paths each treated a `try` handler body as a script:

1. The IR lowerer (`lower_try`) turned every handler body into a nested
   `Script`. A body of `-` was therefore lowered as a script containing the
   single command `-`, which the registry knows as the arithmetic-minus
   operator. With no operands, the generic arity check (E002) fired.
2. The analyser's `try` walk (`handle_try_command`) re-lexed each braced
   handler body as a script for variable / diagnostic analysis. A braced
   `{-}` body re-lexed to a `-` command and tripped the same E002 — even
   after the lowerer was fixed.

Tcl's `TclNRTryObjCmd` (in `tclCmdMZ.c`) recognises a handler body of `-` by
its string value and treats the clause as a fallthrough: when the clause
matches, the runtime scans forward to the next handler whose body is not `-`
and runs that body with that handler's variable bindings. The last
non-`finally` clause must not be `-` (there is nothing to fall through to). The
`switch` command uses the same mechanism for pattern bodies, and the lowerer /
analyser already handled `switch` correctly — `try` was the gap.

## Decision rules / contracts

1. A `try` handler body that is the literal `-` (single-token word, including
   the braced `{-}` / quoted `"-"` forms, which evaluate to the same string) is
   a **fallthrough**, not a script. The lowerer sets
   [`TryHandler.fallthrough`](../../rust/tcl-compiler/src/ir.rs) to `true` and
   gives the handler an empty `Script` body, so no command — and therefore no
   arity check — is synthesised for it.
2. The analyser's `handle_try_command` skips `analyse_body` for a `-` handler
   body, mirroring `handle_switch_command`'s arm handling. A `-` body is never
   re-lexed as a nested script.
3. `try`'s `arg_role_resolver` gives a `-` handler body no `ArgRole::Body`,
   mirroring `switch`. A `-` body is never a nested script for semantic tokens
   or any other role consumer.
4. A genuine zero-argument `-` *command* in a real body (a true mistake) is
   still flagged E002. The suppression is scoped to the handler-body slot, not
   to the `-` command everywhere.
5. The set of commands with `-` fallthrough is exactly `{switch, try}` —
   confirmed against the Tcl 8.4–9.0 C sources. No other Tcl or Tk command
   uses it.
6. Like `switch`, the LSP does not flag the invalid "last non-`finally` clause
   is `-`" case; that rare runtime error is accepted silently rather than
   mis-reported.
7. A `-` handler may bind a result/options var whose name differs from the
   target handler's. Real Tcl is itself inconsistent here — a byte-compiled
   (proc) `try` binds the *matching* handler's var while the interpreted form
   binds the *target*'s — so the shared body is analysed with the whole
   group's vars treated as defined: the precise over-approximation that avoids
   a read-before-set (W210) false positive under either binding rule.

## File-path anchors

- `rust/tcl-compiler/src/lowering/structured.rs` — `lower_try`
  (fallthrough detection)
- `rust/tcl-compiler/src/ir.rs` — `TryHandler.fallthrough`
- `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` — `lower_try`
  (shared-body var defs)
- `rust/tcl-compiler/src/analyser/handlers.rs` — `handle_try_command`
  (no body re-lex for `-`)
- `rust/tcl-registry/src/commands/tcl/try_.rs` — `try_arg_roles`
  (no Body role for `-`)
- `rust/tcl-registry/src/commands/tcl/switch_.rs` — `switch_arg_roles`
  (the mirrored precedent)

## Failure modes

- A future refactor lowers the handler body before checking for `-`, so the
  arity error returns.
- `handle_try_command` is changed to re-lex the `-` body again, re-tripping
  E002 on the braced form.
- The `arg_role_resolver` is changed to mark the `-` body as `Body` again,
  re-tagging it as an embedded script for semantic tokens.
- A new fallthrough-style command is added without the same handling (the
  investigation found none today, but a future Tcl version could add one).

## Test anchors

- `rust/tcl-compiler/src/lowering/structured.rs` — `try_dash_handler_body_is_fallthrough`
  and siblings (lowering marks the fallthrough flag + empty body).
- `rust/tcl-compiler/src/analyser/diagnostics/tests.rs` — `issue_703_*`
  (no E002 on `-`/`{-}`; genuine zero-arg `-` still flagged; no false W210 on
  the shared body; genuine read-before-set still fires).
- `rust/tcl-registry/src/commands/tcl/try_.rs` — `dash_handler_body_gets_no_body_role`
  and siblings (role resolver withholds `Body` from `-`).

## Related

- [KCS index](README.md)
- [control-flow patterns](../design/compiler/control-flow-patterns.md)
