# KCS: How do I declare an option effect in a `.tclspec` pack?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, MCP

## Question

One of my pack's options changes what the command does with its other
arguments — it turns a kind of substitution on or off, picks a pattern
language, or swallows an argument that would otherwise be read as data.
How do I say that in a `.tclspec` pack, instead of the editor treating the
option as a plain flag?

## Before you start

- You have a `.tclspec` pack already, with a `command` block for the
  command whose option you are describing. If not, start with [how do I
  write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- Know which closed axis your option moves: a substitution kind
  (backslashes, commands, variables), a pattern language, case
  sensitivity, or a selection mode. If your option does something no axis
  covers, it is an ordinary option with no effect — most are.

## Answer

An option's `-effect` names one of five things it does to the call:
`disables` or `selects` a value on an axis, `suppresses-role` (it removes
an argument role the command's layout would otherwise assign), `reserves`
(it changes how many trailing words the option scan holds back from the
rest of the call), or `ends-options`. Options that move the same axis
share a family, declared once on the command with `option_effect_family`:
the family's `base` says where the axis starts before any of its options
is seen, and `combine` says whether several of its options accumulate or
the last one wins.

`subst`'s three negating options are one family; here is the whole
declaration, including the Tcl 9.1 positive family that selects the same
three kinds the other way around:

```tcl
command subst {
    arity 1..
    reserved_trailing_words 1
    option_effect_family negated { base all-on  combine accumulate }
    option_effect_family positive { base all-off combine accumulate \
                                    -introduced 9.1 }
    option -nobackslashes -effect {disables substitution backslashes} \
                          -family negated
    option -nocommands    -effect {disables substitution commands} \
                          -family negated
    option -novariables   -effect {disables substitution variables} \
                          -family negated
    option -backslashes   -effect {selects substitution backslashes} \
                          -family positive -introduced 9.1
    option -commands      -effect {selects substitution commands} \
                          -family positive -introduced 9.1
    option -variables     -effect {selects substitution variables} \
                          -family positive -introduced 9.1
    option_conflict {-nobackslashes -nocommands -novariables} \
                    {-backslashes -commands -variables}
}
```

Read as prose: with none of `negated`'s options present, every kind of
substitution runs (`base all-on`); each `-no…` option that is present
turns its own kind off, and several combine (`combine accumulate`), so
`subst -nocommands -novariables` leaves only backslash substitution
running. The 9.1 `positive` family starts from the opposite base and its
options turn kinds on instead; `option_conflict` is what makes mixing a
negated and a positive option in one call a reported error rather than a
silently confusing combination.

A `suppresses-role` or `reserves` effect reads the same way for a
different kind of command: `regexp -inline`'s effect is
`{suppresses-role VarWrite}`, because it removes the trailing
match-variable arguments the command would otherwise expect by position;
`regexp -about`'s is `{reserves 1}`, because with `-about` present the
call only takes the expression, not the expression and a string.

## How to tell it worked

Run the pack through `mcp__tcl-lsp__spectcl_check` as usual — it reports
your option rows with the fields it parsed, so a dropped `-effect` or
`-family` word shows up there rather than staying silent. Then check the
call sites the effect actually feeds: a diagnostic that reads
substitution kinds (for example the one that flags an unsafe braced
template) stops firing on a call your `-novariables`-equivalent option
makes safe, and hover or a quick fix that depends on which argument is
which position agrees with what your `-effect` says the option suppresses
or reserves.

## Related

- [How do I write a SpecTcl pack?](kcs-howto-write-a-tclspec-pack.md)
- [How do I create a command spec without knowing Rust?](kcs-howto-create-command-specs-without-rust.md)
- [Glossary — Option effect](../GLOSSARY.md#option-effect)
- [Registry consumer contracts § Options with semantic effects](../design/compiler/registry-consumer-contracts.md#options-with-semantic-effects) — the full contract and the derived `option_effects` query
- [KCS index](README.md)
