# KCS: T101 — Why does the analyser warn about tainted data in an output sink?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data flowing into `puts` or
`tclLog`?

## Why

Unsanitised user input in output commands can inject misleading lines into a
log or a terminal, or forge entries that hide what really happened.

## Symptoms

- A yellow squiggle under the output argument, with the message "Tainted
  variable $name flows into puts; output may contain injected content".
- A **Sanitise $name (strip CR/LF) before output** quick fix on the
  diagnostic.

## Example that triggers it

```tcl
set name [gets stdin]
puts $name
```

The analyser reports **`T101`** on `$name`: the value came from an untrusted
source and reaches `puts`'s content argument.

## Fix

Strip the line-ending characters an attacker would use to forge a record —
this is the edit the quick fix applies:

```tcl
set name [gets stdin]
puts [string map {"\n" "" "\r" ""} $name]
```

The analyser still reports the sink afterwards: it clears the finding only
when every appearance of the value is consumed by a call whose result is a
fixed numeric or boolean type, as in `puts "input length: [string length
$name]"`. Rewriting the text — `string map`, `regsub` — leaves attacker-derived
text, so if you have sanitised the value in a way the analyser cannot see,
suppress the code at that line.

## How to suppress

Add `# noqa: T101` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = T101` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.T101` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T102`, `IRULE3003`
