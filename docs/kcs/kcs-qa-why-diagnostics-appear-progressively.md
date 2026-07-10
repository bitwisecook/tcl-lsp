# KCS: Why do diagnostics on a large file appear in two waves?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

On a large Tcl file, why do some warnings show up almost immediately and
others follow a moment later?

## Answer

On a big or freshly opened file the server delivers diagnostics
**progressively**: it shows the ones it can work out quickly first, then
fills in the rest once the deeper analysis finishes. You get useful
feedback early instead of a blank editor until every check has run.

The first wave — the **fast tier** — is everything the server can decide from
the one file on its own: syntax and structural errors, style lints (line
length, trailing whitespace), and variable-usage warnings. The second wave —
the **deep tier** — adds the checks that take longer or need the wider
picture: the optimiser and compiler warnings, the shimmer and taint findings,
and the workspace-aware checks that look at your other files and installed
packages. The deep tier replaces the first wave with the complete set, so the
final result is exactly what you would have seen before, just delivered in two
steps rather than one.

Two kinds of warning are deliberately held back from the first wave so they
never flash up and then vanish: **W120** ("command used without a corresponding
`package require`") and **W123** ("unresolved command"). These can only be
judged correctly once the server has scanned your workspace and package
database, so showing them early would risk a false positive that the deep tier
then retracts. Small files skip the two-wave behaviour entirely and publish
once — the split only appears when the deep analysis would otherwise make you
wait.

You do not need to do anything: the split is automatic, and both waves are
tagged with the same document version, so an edit part-way through never leaves
a stale warning behind.

## Related

- [KCS index](README.md)
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md)
- [Glossary](../GLOSSARY.md)
