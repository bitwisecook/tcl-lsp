# KCS: W152 — Option relation unmet

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser say that an option I supplied needs another option, or
that the call has to supply one of a set?

## Why

Real command tables do not only say "these two cannot go together" — they also
say "this one needs that one". `bibtex::parse -command` is the completion
callback for a *channel* parse, so it is meaningless without `-channel`;
`http::geturl -queryprogress` reports progress posting a query body, so it does
nothing unless `-query` or `-querychannel` is also present.

Those relations are registry data, declared on the command's spec, and the
analyser checks them generically: nothing is written for the specific command.
`W152` is the "this one needs that one" half; the "these cannot go together"
half is [`W147`](kcs-diagnostic-w147-mutually-exclusive-options.md).

## Symptoms

- A warning over the option (or argument) that started the requirement.
- The message names what the call supplied and what it is missing — often in
  the library's own words, when the spec quotes them.

## Example that triggers it

```tcl
package require bibtex
::bibtex::parse -command handle -recordcommand rec
```

`-command` requires `-channel`, and the call supplied neither a channel nor an
inline text argument. The library raises *"Option -command and text exclude
each other"* at run time.

```tcl
package require http
::http::geturl http://example.invalid/ -queryprogress cb
```

`-queryprogress` needs a query body: `-query` or `-querychannel`.

## Fix

Supply the companion the relation names:

```tcl
::bibtex::parse -channel $chan -command handle
::http::geturl http://example.invalid/ -query a=1 -queryprogress cb
```

No automatic fix is offered: which companion the caller meant, and what value
it should carry, is not something the analyser can infer.

## Where it does not fire

Proving an option *present* is easy; proving one **absent** is not, so `W152`
is deliberately silent whenever the call could be supplying the missing option
somewhere the analyser cannot read. It abstains when any word is
`{*}`-expanded, when an option name or a relevant value is a `$var` or a
`[command]` substitution, and whenever the invocation could not be read to its
end. It also honours the active dialect and the resolved package version: a
relation a later release of the package introduced is not enforced against a
document pinned to an older one.

## How to suppress

Add `# noqa: W152` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W147 — mutually exclusive options](kcs-diagnostic-w147-mutually-exclusive-options.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W004` (option unavailable in the active dialect),
  `W136` (option requires a newer package version),
  `W141` (option value fails a shape check).
