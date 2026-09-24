# KCS: What does a member effect say?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

When the analyser walks a `TclOO`, snit, or itcl definition body and meets
one of its member words — `method`, `superclass`, `variable`, `forward`,
`export`, `self`, and so on — where does it learn what that word does, and
what do I add when a `.tclspec` pack declares a definer family of its own?

## Answer

From the member's own `MemberEffect`, on `MemberSpec` beside its `kind`.
`MemberKind` stays the *layout* fact — flat, wrapper, or flag-keyed — and
`MemberEffect` is what the member actually declares: a callable (a method,
constructor, destructor, or procedure, with its receiver side and which
slot holds its name, parameters, and body), a dispatch redirect, a state
declaration, a contribution to the ancestry or interposition graph, a
visibility change, a retraction, or an init script run at definition or
construction time. The vocabulary is closed and family-neutral — no
variant names `TclOO`, snit, or itcl, and `DefinerFamily` stays the only
place a family is ever named.

The analyser does not match member keywords to find this out. It reads
`DefinitionBodyGrammar::member_row`, which segments one member statement
and answers a `MemberRow` carrying the effect, the receiver side after
every wrapper shift is applied, the declared name when the word is
literal, its arity, and its visibility. Three things fold over that row
generically instead of being hand-rolled per family: the class hierarchy
comes from every `Relation` row folded through its slot's own append/set
rule; method arity comes from a `Callable` row's parameter slot; and
whether a row is reachable through `my` comes from comparing its receiver
and visibility against the dispatch spelling, not a name list. A wrapper
such as `self` or itcl's access modifiers never needs its own arm either
— its shift is `MemberKind::Wrapper`, and the row's `receiver` is already
resolved past it.

To give a pack's own definer family a new member, declare its effect on
the `member` row in the `.tclspec` DSL — a `callable` row names its
receiver and role, a `relation` row names the slot it feeds, and so on —
and the analyser, the navigation providers, and the lowering all reach it
with no consumer-side edit. Adding a keyword arm to `analyser/oo.rs` or an
editor provider is the one thing not to do.

## Related

- [KCS index](../README.md)
- [Glossary — Member effect](../../GLOSSARY.md#member-effect)
- [Registry consumer contracts § The member-effect descriptor](../../design/compiler/registry-consumer-contracts.md#the-member-effect-descriptor) — the full contract, the `MemberEffect` shapes, and the sites it retired
- [ObjectClassSpec](../../GLOSSARY.md#objectclassspec)
- [kcs-qa-where-does-a-clause-shape-come-from.md](kcs-qa-where-does-a-clause-shape-come-from.md)
