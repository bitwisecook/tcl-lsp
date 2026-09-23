# KCS: Why does a command stop folding once my file defines or renames it?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

Once my file defines `proc incr {v args} {…}`, or renames or aliases
`incr`, why does `proc p {} {set n 1; incr n; return $n}` no longer show
`n` as 2, in the editor or in `tcl opt`?

## Answer

Because the call no longer means the builtin. The shared constant lattice
evaluates a command only while the module's own bindings prove that its
name still denotes the registry's command: no `proc` of that name, no
`rename` or `interp alias` onto it, no import into a namespace the scan
cannot enumerate. With `proc incr` at the top level, `incr n` calls that
procedure, so evaluating it with the builtin's semantics would give a value
the program never computes (#2164). The route declines instead, `n` is
unknown after the call, and `tcl explore --show sccp` records the statement
as `declined: rebinding-suspected`.

Every consumer answers alike. The editor's memoised lattice and a
standalone build take the same stance under the same analysis context, and
that context carries the module's command bindings, so adding a `proc
incr` or a `rename incr` anywhere in the file re-keys every procedure's
cached lattice instead of leaving a stale constant behind. A rewrite is
stricter still: before `tcl opt` edits the source it proves the value again
under the whole module's trust, which also refuses while any command head
in the module is dynamic, where the shared lattice keeps folding past a
head it cannot name.

The fold comes back once the name means the builtin again — give the
procedure a name of its own. Teaching the compiler that a particular
`proc incr` is harmless is the one thing not to do: a binding fact is the
module's, and the lattice reads it rather than a list of exceptions.

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
- [kcs-qa-what-does-a-value-transfer-declaration-say.md](kcs-qa-what-does-a-value-transfer-declaration-say.md)
- [kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md](kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md)
- [value-transfers.md](../../design/compiler/value-transfers.md) § *One invocation, one context* — the analysis context every answer is keyed under
