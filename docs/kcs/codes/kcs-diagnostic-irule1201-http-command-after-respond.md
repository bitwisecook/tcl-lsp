# KCS: IRULE1201 — Why does the analyser flag HTTP commands after respond or redirect?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an HTTP command appears after `HTTP::respond` or `HTTP::redirect`?

## Why

HTTP state is committed once `respond` or `redirect` is called. Any further header or URI changes are silently ignored, which hides bugs.

## Symptoms

- A squiggle appears on the HTTP command that follows the respond or redirect call, with the message "HTTP command after respond/redirect".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  HTTP::respond 200
  HTTP::header insert X-Custom val
}
```

The analyser reports **`IRULE1201`** on the `HTTP::header` call.

## Fix

Move all header work before the respond or redirect call:

```tcl
when HTTP_REQUEST {
  HTTP::header insert X-Custom val
  HTTP::respond 200
}
```

## How to suppress

Add `# noqa: IRULE1201` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1202`
