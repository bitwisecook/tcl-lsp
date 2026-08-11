# KCS: W146 — Why is a literal argument member invalid?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser

## Profiles

default, dialect:tcl

## Question

Why does the analyser say that an operation in my `trace` command is invalid?

## Why

The operations accepted by `trace add` and `trace remove` depend on the trace
type. Variable traces accept `array`, `read`, `unset`, and `write`; command
traces accept `delete` and `rename`; execution traces accept `enter`, `leave`,
`enterstep`, and `leavestep`. Tcl requires every operation-list member to use
an exact spelling.

Tcl 8.4–8.6 also support the deprecated `trace variable` and `trace vdelete`
forms. Their operation argument is a concatenated string of the letters `r`,
`w`, `u`, and `a`, rather than a Tcl list. Tcl 9 removes those legacy forms.

## Symptoms

- A yellow squiggle appears under the operation-list word.
- The message names the invalid member and the operations legal for the
  selected trace type.

## Example that triggers it

```tcl
trace add variable ::config(port) {read rename write} logChange
```

The analyser reports **`W146`** on `{read rename write}` because `rename` is a
command-trace operation, not a variable-trace operation.

## Fix

```tcl
trace add variable ::config(port) {read write} logChange
```

When a complete literal list contains both valid and invalid members, the
quick fix removes only the invalid members and keeps the valid operations. It
requires review because changing a trace registration changes runtime
behaviour. No fix is offered when removal would leave an empty operation list.

The analyser abstains on substituted, expanded, malformed, or incomplete
lists, and when the trace type is invalid or ambiguous.

Execution callbacks have two appended arguments for `enter` and `enterstep`,
but four for `leave` and `leavestep`. For a complete literal operation list,
the callback-arity check verifies that the callback accepts every applicable
argument count. A substituted or malformed operation list is left unchecked.

## How to suppress

Add `# noqa: W146` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W127` (value outside a closed set), `W145` (ambiguous
  keyword abbreviation), `E002` (too few arguments).
