# KCS: IRULE3103 — Why does the analyser suggest `HTTP::path` instead of splitting `HTTP::uri`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag my `split` or `string match` on the result of
`HTTP::uri`?

## Why

`HTTP::uri` returns the whole request target — path, `?`, and query
string together. Taking it apart by hand means re-implementing URI
parsing in Tcl, and your version will disagree with the one the back end
uses. That difference is exploitable: a request crafted so your rule sees
one path and the server sees another slips straight past a path-based
check.

`HTTP::path` and `HTTP::query` return the components directly. They are
clearer, they are cheaper, and — unlike a hand-rolled split — they accept
`-normalized`, which is what actually closes the evasion.

## Symptoms

- An informational underline on the statement, with a message like
  "Splitting HTTP::uri on '?' to extract path or query; use HTTP::path
  and HTTP::query instead for clearer, more efficient URI decomposition."
- Comparison shapes get their own wording, for example "HTTP::uri used
  with string match on a path-like pattern; use HTTP::path instead for
  clearer intent and to avoid query-string interference."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set uri [HTTP::uri]
  set parts [split $uri "?"]
  if {[lindex $parts 0] eq "/admin"} {
    reject
  }
}
```

The analyser reports **`IRULE3103`** on the `split`.

## Fix

```tcl
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/admin"} {
    reject
  }
}
```

Ask for the component you want, and normalise it.

## What it detects

The check recognises the common hand-decomposition shapes: splitting the
URI on `?` or `&`; `string match` with a path-like or query-like pattern;
`string first` looking for `?` or `&`; and the iRules word operators
`starts_with`, `ends_with`, `contains`, and `matches_glob` applied to the
URI with a path-like or query-like operand. It generalises to any `*::uri`
getter that has `*::path` or `*::query` siblings in the registry, not
just `HTTP::`.

## How to suppress

Add `# noqa: IRULE3103` on the line **above** the offending command.
`IRULE3103` is internal, so it has no per-code entry in the generated
editor settings list; to silence it more widely, use a
`# tcl-lsp: disable=IRULE3103` directive at the top of the file, or
`disabled = IRULE3103` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3101`, `IRULE3102`
