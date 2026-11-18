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

//! Display-tier methods of a registered instance the walk proved: a Tk widget
//! path or a handle variable bound by its constructor. The instance's own
//! factory descriptor supplies the methods under the document's availability
//! floor; nothing here feeds rename, references or edits.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::command_binding::OriginalSourceRegisteredInstanceWords;
use tcl_registry::{CommandRegistry, CommandSpec, SubCommand};

/// The proved registered-instance call whose head starts at `head`.
pub(crate) fn words_at(
    source: &str,
    analysis: &AnalysisResult,
    head: u32,
) -> Option<OriginalSourceRegisteredInstanceWords> {
    tcl_compiler::registry_invocation::source_structure::source_registered_instance_words_at(
        source, analysis, head,
    )
}

/// The factory and the instance methods available in this document. A
/// package-owned factory (Tk's widgets) exists only once its package is
/// loaded: tclsh 8.6.18 without `package require Tk` answers
/// `invalid command name "ttk::treeview"`.
pub(crate) fn methods(
    analysis: &AnalysisResult,
    words: &OriginalSourceRegisteredInstanceWords,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<(&'static CommandSpec, Vec<&'static SubCommand>)> {
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let spec = words.instance().descriptor(&context)?;
    let floor = crate::document_floor::DocumentFloor::new(analysis, profile);
    let available = floor.context()?;
    if spec
        .owning_package()
        .is_some_and(|package| !floor.advises_package(package))
    {
        return None;
    }
    let commands: &CommandRegistry = context.commands();
    let methods = CommandRegistry::instance_method_providers_for_descriptor_at(
        commands,
        spec,
        Some(available.authoring_query()),
        |owner| {
            available
                .resolve_spec(commands, owner.name)
                .filter(|selected| std::ptr::eq(*selected, owner))
                .map(|selected| floor.for_spec(selected))
        },
    );
    Some((
        spec,
        methods.into_iter().map(|(_owner, method)| method).collect(),
    ))
}

/// The method the call's first argument selects: its exact name, else the
/// one available method it abbreviates.
pub(crate) fn selected_method(
    analysis: &AnalysisResult,
    words: &OriginalSourceRegisteredInstanceWords,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<(&'static CommandSpec, &'static SubCommand)> {
    let written = std::str::from_utf8(words.argument_bytes(0)?).ok()?;
    let (spec, methods) = methods(analysis, words, profile)?;
    let method = methods
        .iter()
        .copied()
        .find(|method| method.name == written)
        .or_else(|| {
            let mut prefixed = methods
                .iter()
                .copied()
                .filter(|method| method.name.starts_with(written));
            let only = prefixed.next()?;
            prefixed.next().is_none().then_some(only)
        })?;
    Some((spec, method))
}
