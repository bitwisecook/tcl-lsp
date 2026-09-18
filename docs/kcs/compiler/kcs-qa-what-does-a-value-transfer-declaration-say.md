# KCS: What does a command's value-transfer declaration tell the analyser?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

When the constant-propagation pass meets `incr n`, `set x [llength $l]`, or
a `foreach` header, where does it learn what the command computes, and what
may I add to the compiler when a new command needs the same treatment?

## Answer

From the registry, never from the command's name. Every `CommandSpec`,
`SubCommand`, and `CommandForm` carries a `semantics` field with three
states. `Inherited` says nothing at that scope, so an inner declaration
applies or, at the outermost scope, a specialisation is *derived* from a
descriptor that already states the same operation — a
`NativeLowering::CellReadModifyWrite` gives `incr`, `append`, and `lappend`
a cell update, and the `DESTROYS_VARIABLE` trait gives `unset` an unbind.
`Declared` names a registry-owned specialisation, a `CommandSemantics`
value with a structural plan, per-domain transfers, and an evaluator route.
`Declined` abstains explicitly and stops both inheritance and derivation,
which is what lets a form say "no evaluator for this shape" under a parent
that has one.

The compiler resolves an invocation once through the ordinary invocation
resolver and reads `InvocationSemantics::value`. Its driver
(`rust/tcl-compiler/src/value_transfer.rs`) proves operand values from the
SCCP lattice, resolves places, and hands the specialisation a read-only
`AnalysisInputs`; the answer — pending, declined for a recorded reason, or
evaluated with ordered storage outcomes — is applied to the definition.
`sccp.rs` therefore contains no `match command { "foo" => … }`; the gate
`cargo xtask value-transfers` fails if one returns.

Two facts are separate columns in `docs/generated/value-transfers.md`:
whether a descriptor exists and whether its route is *enabled*. `incr`,
`append`, and `lappend` carry a derived cell update with an enabled direct
route — the runtime's own value computation over the compile-time value
model (`ConstOps`), under the target's release semantics — and `string
range` declares one. A few shipped folds (`list`, `format`, `llength`,
`string length`, `expr`) declare a route the compiler still implements;
`NativeEvalId::owner` says so, and the migration plan's ledger names each
with the slice that retires it.

To give a new command a value: declare it on the spec (or add the
descriptor the derivation reads), add its route to the pinned set in
`rust/tcl-registry/tests/value_transfers.rs`, and regenerate the inventory.
Adding an arm to the compiler is the one thing not to do.

## Related

- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
- [value-transfers.md](../../design/compiler/value-transfers.md) — the interface contract
- [value-transfers-migration.md](../../design/compiler/value-transfers-migration.md) — the slices, the ledger, and the gate
- [kcs-issue-the-value-transfers-gate-reports-a-command-name.md](../kcs-issue-the-value-transfers-gate-reports-a-command-name.md)
