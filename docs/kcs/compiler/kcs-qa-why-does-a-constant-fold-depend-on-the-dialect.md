# KCS: Why does a constant fold answer differently, or not at all, under another dialect?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

`tcl opt --dialect tcl8.6` folds `set x 010; incr x; puts $x` to `puts 9`,
`--dialect tcl9.0` folds it to `puts 11`, `--dialect f5-irules` folds it to
`puts 9`, and `--dialect tk` leaves it alone. `string range abcdefghijkl
010 end` does the same. Which one is right, and why does `tk` get
nothing?

## Answer

All four are right, because the fold is the target's answer or no answer.
A registry-owned evaluator on the direct route runs the same shared core
the runtimes run (`tcl-cmd-core`), through one compile-time value model
that carries the target's release semantics. Where a release changes the
answer, the model reads the operand under that release: a leading-zero
numeral is octal up to Tcl 8.6 and decimal from 9.0, so `incr` of `010`
is 9 on one and 11 on the other; an index numeral follows the same
grammar, so `string range … 010 end` is `ijkl` up to 8.6 and `kl` from
9.0; an increment past the wide boundary widens to a bignum from 8.5 and
is an error on 8.4.

`expr` reads the same release axis, because its route runs the shared
expression engine over the same value model rather than a private
parser: `expr {"010"}` folds to `8` up to Tcl 8.6 and `10` from 9.0, for
the identical reason `incr` does. The expression route takes its numeral
grammar from the profile's runtime, and iRules' runtime is 8.4's, so
`expr {"010"}` folds there too (`8`); a profile with no runtime at all —
an F5 BIG-IP config context, say — declines, because nothing pins which
grammar applies.

A dialect that declares a base release evaluates under it. iRules, iApps
and tmsh embed a Tcl 8.4, so `incr` of `010` is 9 under `f5-irules`, as
`tclsh8.4` prints; `expect` evaluates as 8.6 and each EDA shell as the
release its vendor ships. Where a vendor dialect declares an answer of its
own on one axis that its base release does not give — the F5 dialects'
character model, which no TMOS measurement settles yet — that axis falls
back to the rule below, so the declaration blocks the base release's
answer rather than borrowing it.

A profile that names no release — `tk`, the version-less `tcl` profile, a
BIG-IP config context — gets the answer every modelled release gives, and
a decline where they differ. `incr x 5` still folds there because every
release adds 5; `incr x` over `010` does not, because 8.x and 9.x
disagree, and folding either answer would bake the wrong constant into a
program built for the other. The same rule keeps a
non-ASCII operand off `string range` and `string length` under 8.x: the
source was decoded as UTF-8, which is the 9.x reader's answer, and an 8.x
reader following `encoding system` may hold a different string — `string
length` of one astral character read from a UTF-8 file is 1 under 9.0 and
4 under 8.6 in a Latin-1 locale, so under 8.x neither command folds it.

So the direct and the expression routes agree under iRules: with `set z
010`, `incr z` and `expr {$z + 1}` both fold to 9, as tclsh 8.4 prints.
Before the declared base reached the direct routes, `incr z` declined
there while `expr` folded — imprecise, never wrong.

The decline is a recorded reason, not silence. `tcl explore --show sccp`
lists every statement's route and answer — `direct cell-increment
(registry)` with `evaluated`, or `declined: release-ambiguous:
numeral-grammar` — so a fold that did not happen says which axis stopped
it. The Explorer's `sccp` view shows the same rows.

Adding a release rule to the compiler is the one thing not to do: the
rule belongs to the axis's owner (`NumberSyntax`, `StringCharacterModel`,
`index::resolve_opt_with`, `ValueOps::int_add`), and the value model asks
it once for every route.

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
- [kcs-qa-what-does-a-value-transfer-declaration-say.md](kcs-qa-what-does-a-value-transfer-declaration-say.md)
- [value-evaluation.md](../../design/compiler/value-evaluation.md) § *The direct route* — the value model, the admissibility set, and the core table
- [value-transfers-migration.md](../../design/compiler/value-transfers-migration.md) — the slices and the ledger
