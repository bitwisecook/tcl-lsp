# KCS: T104 — Why does the analyser warn about tainted data in a network address?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data used as the address a
connection is opened to?

## Why

If an attacker chooses the host or port your script connects to, they can
point it at somewhere it should never reach — an internal admin service,
a cloud metadata endpoint, a machine behind the firewall. That is
server-side request forgery (SSRF): the request comes from your trusted
server, so it passes controls a request from the attacker would not.

## Symptoms

- A yellow squiggle under the address argument, with the message
  "Tainted variable $host in network address argument of socket; risk of
  SSRF (server-side request forgery)".

## Example that triggers it

```tcl
set host [gets stdin]
set ch [socket $host 80]
```

The analyser reports **`T104`** on `$host`: the value came from an
untrusted source and reaches `socket`'s address slot.

## Fix

Validate the address against an allow-list before connecting:

```tcl
set host [gets stdin]
if {$host ni {api.internal.example db.internal.example}} {
  error "host not permitted"
}
set ch [socket $host 80]
```

## When it does not fire

- **Only the address slots count.** A tainted value in some other
  argument of the same command is ordinary data and does not trip the
  sink.
- **A validated address clears it.** Once the analyser can see that the
  value has been through an IP-address, port, or hostname check, the
  finding is withdrawn.

## How to suppress

Add `# noqa: T104` on the line **above** the offending command. You can
also turn the code off for a project with `disabled = T104` under
`[diagnostics]` in `.tcl-lsp.ini`, or in your editor with
`tclLsp.diagnostics.T104` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T101`, `T105`
