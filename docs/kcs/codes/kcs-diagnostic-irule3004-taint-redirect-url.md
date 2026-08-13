# KCS: IRULE3004 — Why does the analyser warn about tainted data in a redirect?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag user-controlled data used as the target of
`HTTP::redirect`?

## Why

If the client controls where the redirect points, they control where your
site sends its own visitors. An attacker sends someone a link to *your*
host, your iRule bounces them to a look-alike site, and the victim sees a
trusted domain in the link they clicked. That is an open redirect, and it
is the standard opening move for credential phishing.

## Symptoms

- A yellow squiggle under the redirect target, with the message "Tainted
  variable $target in redirect URL (HTTP::redirect); risk of open
  redirect".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set target [HTTP::header Location]
  HTTP::redirect $target
}
```

The analyser reports **`IRULE3004`** on `$target`: the value comes
straight from a request header and lands in the redirect URL.

## Fix

Redirect to a path on your own host rather than to whatever the client
supplied:

```tcl
when HTTP_REQUEST {
  HTTP::redirect "/login"
}
```

Where the destination really does have to vary, check it against an
allow-list first:

```tcl
when HTTP_REQUEST {
  set target [HTTP::header Location]
  if {[class match $target equals allowed_redirects]} {
    HTTP::redirect $target
  } else {
    HTTP::redirect "/login"
  }
}
```

## When it does not fire

- **A same-origin target clears it.** A value the analyser can see starts
  with `/`, or that has been through `file normalize`, routes back to the
  current host and cannot be an open redirect.

## How to suppress

Add `# noqa: IRULE3004` on the line **above** the offending command. You
can also turn the code off for a project with `disabled = IRULE3004`
under `[diagnostics]` in `.tcl-lsp.ini`, or in your editor with
`tclLsp.diagnostics.IRULE3004` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3001`, `IRULE3002`, `IRULE3003`, `IRULE1202`
