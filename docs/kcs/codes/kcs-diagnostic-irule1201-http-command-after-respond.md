# KCS: IRULE1201 — Why does the analyser flag HTTP commands after respond or redirect?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an HTTP command appears after `HTTP::respond` or `HTTP::redirect`?

## Why

HTTP state is committed once `respond` or `redirect` is called. Any further header or URI changes are silently ignored, which hides bugs.

## Symptoms

- A yellow squiggle appears on the HTTP command that follows the response, with
  the message "'HTTP::header' used after response is committed. HTTP context is
  invalid after HTTP::respond/HTTP::redirect."

## Example that triggers it

```tcl
when HTTP_REQUEST { HTTP::respond 200; HTTP::header insert X-Custom val }
```

The analyser reports **`IRULE1201`** on the `HTTP::header` call.

## Fix

Move all header work before the respond or redirect:

```tcl
when HTTP_REQUEST { HTTP::header insert X-Custom val; HTTP::respond 200 }
```

## Limits

`HTTP::has_responded` remains valid after a response is committed because its
purpose is to query that state. The analyser reads this exception, and the set
of commands that still need a live HTTP context, from the command registry.

A `respond` or `redirect` in a branch the analyser proves never runs commits
nothing, so a command after it is not reported: `if {0} { HTTP::respond 200 }`,
or the same behind `set flag 0; if {$flag} { … }`. One in a branch that might
run still counts.

## How to suppress

Add `# noqa: IRULE1201` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1202`
