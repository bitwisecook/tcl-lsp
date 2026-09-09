# KCS: IRULE1202 — Why does the analyser flag multiple respond or redirect calls?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that more than one `HTTP::respond` or `HTTP::redirect` can run on the same request?

## Why

Only the first response takes effect. The later call is silently discarded, so a request you meant to block can be redirected instead — the usual cause is a branch that responds and then falls through.

## Symptoms

- A yellow squiggle appears on the later call, with the message "Multiple
  'HTTP::redirect' calls possible in HTTP_REQUEST. Only the first response takes
  effect."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/blocked"} {
    HTTP::respond 403
  }
  HTTP::redirect "https://example.com/"
}
```

The analyser reports **`IRULE1202`** on the `HTTP::redirect`: a blocked request
responds, then carries on into the redirect.

## Fix

`return` out of the event once the response is committed, so each path issues
at most one:

```tcl
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/blocked"} {
    HTTP::respond 403
    return
  }
  HTTP::redirect "https://example.com/"
}
```

## How to suppress

Add `# noqa: IRULE1202` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1201`
