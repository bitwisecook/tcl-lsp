# naming.command.original-publication-row-sharing

Kind: `implementation-contract`

## Problem statement

Flow-sensitive source snapshots need complete original publication producers in their original order. A changed publication must preserve independent prior name, definition and allocation metadata even when unchanged rows share storage.

## Question

How can immutable original publications share their actual producers across authentic ledger snapshots while moved, replaced, imported or updated publications retain exact independent values and history?

## Conclusion

OriginalSourceCommandWorld retains immutable Arc-backed OriginalCommandPublication rows. Snapshot clones share complete unchanged producers; actual move/import edits construct a changed row and definition attachment uses row-level copy-on-write. Readonly queries expose complete original values. Structural equality and hash include every complete row and order; mutable branches cannot rewrite retained earlier snapshots.

## Scope

Current Rust source owner and two genuine software control definitions. Their actual test bodies and module registrations are inspected; no matching executable inventory or assertion result is attached. The five C-profile loops construct source-model ledgers and are not five original interpreter experiments. Sharing full immutable rows supplies no external C/Jim/BIG-IP result, Native currency, handler/body admission, physical object lifetime, source edit or current lookup. Complete source/channel/policy/allocation inputs remain independently required. Allocation-path source inspection establishes neither measured memory/time payoff nor a cause for an interrupted SIGKILL process.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Current Rust source/API definitions.

Storage sharing is a Rust implementation contract; no external interpreter ran in this packet.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalSourceCommandWorld`: Share exact original publication producers across world snapshots without changing complete row semantics.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `CapturedOriginalCommandTransfer::finish`: Actual moves detach a changed publication while previous snapshots retain their original slot/input/definition.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `ModuleCommandBindings::retain_original_class_publication_definition`: Definition updates use row-level copy-on-write under the existing allocation/world guards.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::publication_snapshots_share_original_rows_and_detach_actual_moves_and_replacements` (linked): Actual source-ledger snapshots under five C software profiles share unchanged producer rows. Move/replacement and private definition mutation detach the affected row while original name input, declaration site, definition and earlier ordered snapshots remain unchanged. No external provider or completed performance comparison is inferred.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::publication_row_sharing_preserves_complete_semantic_equality_hash_and_order` (linked): Independently constructed five-profile source snapshots expose complete equal publication values and hashes with unchanged ordered readonly iteration. Storage sharing supplies no current lookup, Native currency, entered body or edit authority.

## Evidence

- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs)
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs)
- [rust/tcl-compiler/src/lib.rs](../../../../rust/tcl-compiler/src/lib.rs)
