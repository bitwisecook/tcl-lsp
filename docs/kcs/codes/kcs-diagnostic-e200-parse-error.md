# KCS: E200 — Why does the analyser report a general parse error?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying the parser cannot recover from a syntax error?

## Why

An unclosed delimiter prevents the parser from determining where one command ends and the next begins. Everything after the unclosed token is misinterpreted, so no further analysis is reliable until the delimiter is matched.

## Symptoms

- A red squiggle appears on the unclosed delimiter, with the message "missing
  close-brace", "missing close-bracket", or `missing "` — whichever delimiter
  the parser was still inside.

## Example that triggers it

```tcl
proc report {x} {
    if {$x > 0} {
        puts yes
    }
proc other {} {
    return 1
}
```

The analyser reports **`E200`** on the `{` that opens `report`'s body: the
missing `}` swallowed more than one following statement, so the analyser
cannot say which brace was meant to close it.

## Fix

```tcl
proc report {x} {
    if {$x > 0} {
        puts yes
    }
}
proc other {} {
    return 1
}
```

Close the brace, bracket, or quote so the parser can process the rest of the file correctly.

When the missing delimiter is unambiguous the analyser reports the more
specific code instead — `E201` for a bracket, `E202` for a quote, `E203` for
a brace, and `E103` when exactly one nested body consumed the closer. `E200`
is the fallback for the cases none of those can pin down.

## How to suppress

Add `# noqa: E200` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E201`, `E202`, `E203`
