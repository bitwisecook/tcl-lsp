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

//! Method hover on a registered instance the walk proved (a Tk widget path or
//! a handle variable bound by its constructor). The instance's own factory
//! descriptor supplies the method, under the document's availability floor.

use super::Hover;
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

pub(super) fn method_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Hover> {
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span.start() <= cursor)
        })
        .max_by_key(|command| command.argv[0].span.start())?;
    let head = command.argv.first()?;
    let words =
        tcl_compiler::registry_invocation::source_structure::source_registered_instance_words_at(
            source,
            analysis,
            head.span.start(),
        )?;
    let selector = words.argument_word(0)?;
    if !(selector.word_span().start() <= cursor && cursor <= selector.word_span().end()) {
        return None;
    }
    let written = std::str::from_utf8(words.argument_bytes(0)?).ok()?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let spec = words.instance().descriptor(&context)?;
    let floor = crate::document_floor::DocumentFloor::new(analysis, profile);
    let available = floor.context()?;
    // A package-owned factory (Tk's widgets) exists only once its package is
    // loaded: tclsh 8.6.18 without `package require Tk` answers
    // `invalid command name "ttk::treeview"`.
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
    let method = methods
        .iter()
        .map(|(_owner, method)| method)
        .find(|method| method.name == written)
        .or_else(|| {
            let mut prefixed = methods
                .iter()
                .map(|(_owner, method)| method)
                .filter(|method| method.name.starts_with(written));
            let only = prefixed.next()?;
            prefixed.next().is_none().then_some(only)
        })?;
    let mut text = format!("**{} {}**", spec.name, method.name);
    if !method.detail.is_empty() {
        text.push_str("\n\n");
        text.push_str(method.detail);
    }
    Some(Hover::markdown(text))
}
