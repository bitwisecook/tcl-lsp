# KCS: T101 — Why does the analyser warn about tainted data in an output sink?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, irule

## Question

Why does the analyser flag user-controlled data flowing into `puts`, `log`, or similar output commands?

## Why

Unsanitised user input in log or output commands can inject misleading log entries or enable log-based attacks.

## Symptoms

- A yellow squiggle appears under the output command, with the message "tainted data flows into output sink".

## Example that triggers it

```tcl
set host [HTTP::host]
log local0. $host
```

The analyser reports **`T101`** because `host` carries tainted data into `log`.

## Fix

```tcl
set host [HTTP::host]
set safe_host [string map {"\n" "" "\r" ""} $host]
log local0. $safe_host
```

Sanitise the value before logging, or use a structured log format.

The injection T101 warns about is a CR or LF that forges a record boundary,
so a value proven free of both clears the diagnostic. The analyser reads that
proof off the mapping you wrote: a `string map` whose keys cover **both**
`"\n"` and `"\r"` and whose replacement values contain neither cannot leave
one behind, whatever it is given. Any other mapping proves nothing, so
`string map {a b}` leaves the warning standing.

Wrapping in place works the same way — `puts [string map {"\n" "" "\r" ""}
$host]` is as good as the two-line form above. A quick fix ("Sanitise $var
(strip CR/LF) before output") writes exactly that wrap.

Commands that guarantee a CR/LF-free result unconditionally clear it too:
`URI::encode`, `html_encode` and their aliases, and any value already known to
be an IP address, a port, or an FQDN.

## How to suppress

Add `# noqa: T101` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T102`
