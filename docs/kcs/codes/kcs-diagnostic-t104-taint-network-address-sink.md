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

Use the untrusted value to *select* an address rather than to *be* one,
so the word that reaches the sink is a literal from your own source:

```tcl
set choice [gets stdin]
switch -- $choice {
  primary { set host api.internal.example }
  replica { set host db.internal.example }
  default { error "host not permitted" }
}
set ch [socket $host 80]
```

Guarding the value in place — `if {$host ni {…}} {error …}` — does *not*
clear the finding. The analyser does not model expression-level
validation, so `$host` is still the attacker-controlled value where it
reaches `socket`. If you have validated the address in a way the
analyser cannot see, suppress the code at that line instead of
restructuring around it.

## When it does not fire

- **Only the address slots count.** A tainted value in some other
  argument of the same command is ordinary data and does not trip the
  sink. `socket -myaddr 1.2.3.4 $h 80` still fires on `$h`; the
  `-myaddr` value is not the sink slot.
- **A literal address never fires.** `socket localhost 80` and
  `http::geturl "http://example.com"` carry no taint to begin with.
- **An address-typed source clears it.** In the F5 iRules dialect a
  value from `IP::client_addr`, `TCP::client_port`/`TCP::remote_port`,
  or `SSL::sni` is born carrying the `IP_ADDRESS`, `PORT`, or `FQDN`
  colour, and that colour suppresses the sink. These colours come from
  the registry's `taint_source`; no general-purpose Tcl command confers
  them, which is why an ordinary validation guard cannot.

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
