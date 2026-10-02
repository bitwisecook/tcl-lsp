# KCS: T106 — Why does the analyser say my value is already encoded?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser tell me a value passed to `URI::encode` (or another encoder) has already been encoded?

## Why

Encoders are not idempotent. Running one twice does not make a value safer,
it makes it wrong: the escape character introduced by the first pass is
itself escaped by the second, so `%20` becomes `%2520` and `&amp;` becomes
`&amp;amp;`. Whatever reads the value back decodes it once and gets the
half-decoded middle state, not the original.

The usual cause is a value that was encoded where it was built and encoded
again where it was used, because neither place could see the other.

## Symptoms

- A blue information marker on the second encoding call, with the message
  "Variable `$var` is already URL-encoded; passing through `URI::encode`
  double-encodes the value".
- The wording names whichever encoding applies — URL-encoded, HTML-escaped
  or regex-escaped.
- The value that reaches its destination carries a doubled escape: a literal
  `%2520` in a URL, or a visible `&amp;amp;` in a page.

## Example that triggers it

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    set once [URI::encode $uri]
    set twice [URI::encode $once]
    log local0. $twice
}
```

The analyser reports **`T106`** on the `set twice` line, naming `$once` as
the value that is already URL-encoded.

## Fix

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    set once [URI::encode $uri]
    log local0. $once
}
```

Encode once, at the point where the value crosses into the context that
needs the encoding. A quick fix ("Remove redundant encoder") drops the second
call for you.

If the two encodings are genuinely wanted — an inner URL carried as a
parameter of an outer one — then the second encode is **not** redundant and
neither the quick fix nor a decode between them is the right answer. The
inner URL's own `%20` has to survive the outer decode, which means it must go
out as `%2520`: encoding `http://example.com/a%20b` for a parameter slot
yields `http%3A%2F%2Fexample.com%2Fa%2520b`, and dropping either layer changes
the destination the outer decode returns. Suppress T106 on that line instead.

The way to avoid needing the suppression is to encode each component at the
boundary it crosses, building the parameter value from raw text, rather than
re-encoding a whole URL that is already encoded.

## How to suppress

Add `# noqa: T106` on the line **above** the offending command.

To turn it off more widely than one line, use the file-level directive
`# tcl-lsp: disable=T106` in the leading comment block, or `disabled = T106`
under `[diagnostics]` in `.tcl-lsp.ini` at the workspace root.

T106 is marked internal in the diagnostic registry, which means it gets no
generated entry in your editor's settings list — there is no tick-box for it.
Setting the key by hand still works: `"tclLsp.diagnostics.T106": false`.

See [how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T101`, `T103`, `IRULE3001`
