# KCS: W310 — Why does the analyser warn about a hardcoded credential?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag a literal password, token, or API key in my
script?

## Why

A secret written into source is a secret that leaks. It travels into
version control, into build logs, into every copy of the file, and it
cannot be rotated without an edit and a redeploy. Reading the value from
an environment variable or a vault at run time keeps it out of the code
entirely.

## Symptoms

- A yellow squiggle under the value, with the message "Hardcoded
  credential in -password argument. Store secrets in environment
  variables or a vault, not in source code."
- For a sensitive header, the message names the header instead:
  "Hardcoded credential in authorization header value."
- At most one W310 per command, even when several credential arguments
  are present.

## Example that triggers it

```tcl
package require http

http::geturl $url -headers {Authorization "Bearer sk-live-9f3a2b"}
```

```tcl
when HTTP_REQUEST {
  HTTP::header insert authorization "Bearer sk-live-9f3a2b"
}
```

The analyser reports **`W310`** on the literal value in each case: the
first because `-headers` is a credential-bearing option on
`http::geturl`, the second because `authorization` is a sensitive header
on `HTTP::header insert`.

## Fix

Read the secret at run time and pass the variable:

```tcl
package require http

set token $::env(API_TOKEN)
http::geturl $url -headers [list Authorization "Bearer $token"]
```

The check only fires on a *literal* value. A `$variable` or a
`[command]` substitution is never flagged, because the analyser cannot
see — and does not assume — what it resolves to.

The credential option names recognised for every command are `-password`,
`-pass`, `-secret`, `-token`, and `-apikey`, matched case-insensitively.
Individual commands add more through the registry.

## How to suppress

`W310` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=W310`
directive at the top of the file, or for a whole project with
`disabled = W310` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

Prefer moving the secret out of the file over silencing the finding.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W311`, `W312`, `T101`
