# KCS: Why does a regexp call with constant arguments sometimes not fold?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

`regexp {(a+)+b} $s` has a literal pattern and, in the case I'm looking
at, a subject the lattice already proves — so why does `tcl explore
--show sccp` still answer `overdefined` for it, instead of folding the
match?

## Answer

Because the engine, not just the registry declaration, has to prove the
answer — and a regular expression's matching cost is exponential in the
pattern's structure on every Tcl release, including the one `tclsh`
links against. `(a+)+b` against a subject with no trailing `b` is the
textbook catastrophic-backtracking shape: a real `tclsh` can spend a very
long time on it too.

The engine (`tcl-regex`) answers one of three ways, typed so a consumer
can never confuse them: `Matched`, `NoMatch`, or `Stopped`. `Stopped`
carries why — fuel exhausted, the dissector's depth cap reached, or the
request cancelled — and it is never treated as a no-match. Before slice 5
an exhausted search answered `0` (no match), which is unsound: `regexp`,
`regsub`, `switch -regexp`, and `lsearch -regexp` all raise a real error
when the engine gives up (`error while matching regular expression: …`),
so folding a `Stopped` result to "no match" would make the optimised
program produce a value where the original program raises. The
value-transfer route follows the engine's answer exactly:
`RegexpSemantics` declines `Approximate` for a `Stopped` result, and the
call's definition stays `Overdefined` with that reason recorded
(`tcl explore --show sccp` prints `declined: approximate`).

Two budgets bound the work before the engine even starts: `AnalysisMatch::charge`
charges a pattern's compile proportional to its declared length squared,
and the shared pattern cache is capped at 4 MiB of retained bytes,
evicting the coldest entry first rather than growing without bound.
Either one can turn "the engine would eventually answer" into "the
budget says stop now" — same typed decline, same non-fold.

None of this is a registry gap to fill in: raising the caps only moves
where the cliff edge sits, and every release the profile spans has to
agree on the answer, so a capture the cap lets through on one release and
not another would be a soundness bug wearing a precision costume.
[precision-limitations.md](../../design/compiler/precision-limitations.md)
records the trade-off so it is not re-litigated from scratch.

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
- [value-transfers.md](../../design/compiler/value-transfers.md) — the interface contract; the regexp owner's typed precision result
- [precision-limitations.md](../../design/compiler/precision-limitations.md) — the accepted trade-off, and why the cap is not simply raised
- [kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md](kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md)
