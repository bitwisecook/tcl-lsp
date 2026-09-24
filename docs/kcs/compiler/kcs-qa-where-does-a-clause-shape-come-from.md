# KCS: Where does a clause shape come from?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

When a pass meets a clause-chain command — `if` / `elseif` / `else`, `try`
/ `on` / `trap` / `finally`, `for`, `while`, a `foreach`-family command, or
`catch` — where does it learn which word introduces which clause, what
each clause's slots are, and when its body runs; and what do I do when a
command's clause shape does not fit what is already declared?

## Answer

From the registry's clause-grammar descriptor, never from a hand-matched
keyword. Every clause-chain `CommandSpec` (and, for `dict for` and its
siblings, `SubCommand`) carries a `ClauseGrammarSpec`: the leading clause,
the further clauses in declaration order with their introducing keyword
and repetition, an optional trailing clause, the fall-through body word
if the command has one, and how many clauses one call selects. It states
locations and grammar and nothing executable — first-match dispatch, list
iteration, and completion stay the [value transfer](../../GLOSSARY.md#value-transfer)
interface's structural plan, declared beside it and never inferred from
the slots.

A pass never walks that descriptor by hand either. It resolves the call
once and reads the one derived `clause_plan` query, which returns each
clause the call actually supplied, in source order, with the argument
role each of its operands filled and the timing its body runs under —
selected, always, per iteration, a loop fixture, or protected. `lower_if`
and `lower_try`, the generic body-argument walk that sets a body's
conditional and control-flow depth, the CFG's `on ok` edge, the stray-keyword
report, and `signature_scan`'s walker all read this one plan; none of them
compares a word against `"elseif"` or `"trap"` itself.

`CommandSpec::clause_shape_check` is the escape hatch, not the mechanism.
It exists only for a clause chain the grammar genuinely cannot spell, and
no shipped command needs it today — `if`'s own former hand-written checker
retired once the grammar and the walk it derives from could state the same
rule (including the two rows that make `if else {a}` a well-formed call
whose condition is the bareword `else`, and the rule that nothing may
follow a bare trailing body). Reach for it only after checking whether a
new `ClauseRow` or `ClauseSlot` shape already says what you need; adding a
grammar is a data change reviewed like any other registry entry, while a
new hand-written checker is exactly the kind of command-specific code the
consumer-contracts migration exists to retire.

## Related

- [KCS index](../README.md)
- [Glossary — Clause grammar](../../GLOSSARY.md#clause-grammar)
- [Registry consumer contracts § The clause-grammar descriptor](../../design/compiler/registry-consumer-contracts.md#the-clause-grammar-descriptor) — the full contract, the `ClauseGrammarSpec` / `ClausePlan` shapes, and the consumer list
- [kcs-issue-the-registry-axes-gate-reports-a-keyword.md](../kcs-issue-the-registry-axes-gate-reports-a-keyword.md)
- [kcs-qa-what-does-a-member-effect-say.md](kcs-qa-what-does-a-member-effect-say.md)
