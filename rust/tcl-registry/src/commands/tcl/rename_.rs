// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `rename` — rename or delete a command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "rename oldName newName",
    ..FormSpec::DEFAULT
}];

/// Command spec for `rename`.
///
/// Tcl `rename(n)` specifies exactly `rename oldName newName`: an empty
/// `newName` deletes the command, while a non-empty target moves it.  The Tcl
/// 9.0 manpage and Tcl 8.5/9.0 oracle runs agree that a qualified target
/// creates its missing namespace lineage, rather than rejecting the target.
/// Missing sources and existing targets error; Tcl 9.0 reports the structured
/// `TCL LOOKUP COMMAND` and `TCL OPERATION RENAME TARGET_EXISTS` codes, while
/// Tcl 8.5 leaves `errorCode` as `NONE`.  The detailed evidence and the
/// unavailable source-tree record live in the command-oracle audit.
///
/// `surface: ALL_TCL` (no `IRULES` bit) here is deliberate, not an
/// oversight: F5 iRules is the one modelled dialect that drops `rename` —
/// it is one of the K36322151 commands F5 bans from direct command-table
/// surgery in the TMM event sandbox alongside its `namespace`/`interp`
/// siblings — and that exclusion is enforced simply by this group's
/// omission of the `IRULES` bit: an `ALL_TCL` group never intersects the
/// bare `IRULES` availability mask, so `rename` falls out by plain
/// intersection with no separate disable list, the same treatment
/// `pwd`/`cd`/`open`/`glob`/`exec` get in their own spec files. Every
/// other modelled dialect (Expect, Tk, the EDA vendor consoles, F5 iApps,
/// F5 tmsh, incr Tcl) hosts a real Tcl core whose availability mask
/// intersects this `ALL_TCL` group and adds no dedicated `rename` override
/// of its own, so the command resolves there identically to plain Tcl.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename",
        surface: Some(SpecSurface::ALL_TCL),
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_RenameObjCmd` → `TclRenameCommand`
        // (tclCmdMZ.c / tclBasic.c) deletes `oldName` (an empty `newName`
        // deletes the command outright) and errors when `oldName` doesn't
        // exist — the property the W302 fire-and-forget suppression
        // (`catch {rename foo ""}`) keys off.
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::INSTALLS_NAMED_DEFINITION
            | Traits::FIRE_AND_FORGET_TEARDOWN
            // Both arguments are command identities held as data; a program
            // that renames commands observes their spelled names.
            | Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Rename or delete a command",
            synopsis: &["rename oldName newName"],
            snippet: "If newName is an empty string, oldName is deleted instead of renamed. Both oldName and newName may include namespace qualifiers; renaming a command into a different namespace relocates it there, creating any missing target namespaces, and future invocations run in that namespace's context. Raises an error if oldName does not name an existing command, or if newName is non-empty and already names one — rename never silently overwrites an existing command. Any `trace add command` handler registered on the command fires as part of the operation: a rename trace when moved, a delete trace when removed. A common idiom wraps a built-in with custom logic: move the original out of the way, then define a same-named replacement that delegates to the saved name.",
            source: "Tcl rename(n)",
            examples: "rename ::source ::theRealSource\nset sourceCount 0\nproc ::source args {\n    global sourceCount\n    puts \"called source for the [incr sourceCount]'th time\"\n    uplevel 1 ::theRealSource $args\n}",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Rename),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        // The command-table mutation is declared **once**, by naming the
        // stock descriptor (ledger C8). The former `command_table_effect`
        // stamp beside it was the same fact in a second vocabulary.
        state_transitions: Some(crate::state_transition::command_binding::RENAMES_COMMANDS),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_literal_target_ensures_its_namespace_lineage_before_moving() {
        let transitions = crate::command_table::CommandTableEffect::RenamesCommands
            .transitions()
            .resolve(InvocationArguments::literals(&[
                "::old",
                "::fresh::nested::new",
            ]));

        assert!(matches!(
            transitions.facts(),
            [ensure, moved]
                if matches!(
                    &ensure.transition,
                    StateTransition::Namespace(NamespaceTransition::Ensure {
                        namespace: NamespaceTransitionTarget::Named(
                            TransitionSubject::Literal(namespace),
                        ),
                    }) if namespace == "::fresh::nested"
                ) && matches!(
                    &moved.transition,
                    StateTransition::CommandBinding(CommandBindingTransition::Move {
                        from: TransitionSubject::Literal(from),
                        to: TransitionSubject::Literal(to),
                    }) if from == "::old" && to == "::fresh::nested::new"
                )
        ));
    }

    #[test]
    fn deletion_and_dynamic_target_do_not_claim_a_specific_namespace() {
        let delete = crate::command_table::CommandTableEffect::RenamesCommands
            .transitions()
            .resolve(InvocationArguments::literals(&["old", ""]));
        assert!(matches!(
            delete.facts(),
            [fact]
                if matches!(
                    &fact.transition,
                    StateTransition::CommandBinding(CommandBindingTransition::Delete {
                        interpreter: None,
                        name: TransitionSubject::Literal(name),
                    }) if name == "old"
                )
        ));

        // A dynamic target states the unknown mutation *and* the descriptor's
        // declared widening for the dynamic operand — the widening is what
        // keeps a consumer conservative about the identities it can no
        // longer name.
        let dynamic = crate::command_table::CommandTableEffect::RenamesCommands
            .transitions()
            .resolve(InvocationArguments::structured(&[
                crate::InvocationWord::Literal("old"),
                crate::InvocationWord::Dynamic,
            ]));
        assert!(matches!(
            dynamic.facts(),
            [unknown, widen]
                if matches!(
                    &unknown.transition,
                    StateTransition::CommandBinding(CommandBindingTransition::Unknown { operands })
                        if operands.len() == 2
                ) && matches!(
                    &widen.transition,
                    StateTransition::Widen(widening)
                        if widening.domains.contains(&StateTransitionDomain::CommandBindings)
                )
        ));
        assert!(dynamic.touches_command_bindings());
    }
}
