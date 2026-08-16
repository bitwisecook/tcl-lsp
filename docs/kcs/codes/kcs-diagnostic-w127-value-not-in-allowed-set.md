# KCS: W127 — Why does the analyser flag a value that is not in the command's allowed set?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that the value I passed is not one of the command's accepted values?

## Why

Some command arguments accept only a fixed, closed set of literals. For example, the bareword `HTTP::version` setter accepts only the HTTP/1.x versions `0.9`, `1.0`, and `1.1`. A literal outside that set is almost always a mistake — it will not behave as intended at runtime.

The check only fires for arguments whose value set is declared *exhaustive*. It is skipped for dynamic values (`$var`, `[cmd]`) and for option flags, so forms that intentionally take a raw value (e.g. `HTTP::version -string <raw>`) are never flagged.

## Symptoms

- A yellow squiggle appears under the argument, with the message "Invalid value '…' for '…'; expected one of: …".

## Example that triggers it

```tcl
when HTTP_RESPONSE priority 5 {
    HTTP::version "2.0"
}
```

The analyser reports **`W127`** on `"2.0"`: HTTP/2 is the separate `HTTP2::` command namespace, not a value of `HTTP::version`.

## Fix

```tcl
when HTTP_RESPONSE priority 5 {
    HTTP::version "1.1"
}
```

Use one of the accepted values, or — when you genuinely need to set a non-standard raw version — the `-string` form, which is not constrained:

```tcl
HTTP::version -string "1.2"
```

## How to suppress

Add `# noqa: W127` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W126`, `W001`
