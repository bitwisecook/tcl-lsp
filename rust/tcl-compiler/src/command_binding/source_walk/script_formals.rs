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

//! Script parameters: a called body that runs a formal as a script runs the
//! braced literal its caller wrote.
//!
//! When a walked call passes a braced literal to a value formal, the entered
//! activation remembers which written word supplied it. A later script body
//! (`uplevel 1 $script`, `eval $script`, `catch $script`) reading that whole
//! formal, still holding the same text, in the same body source, is walked
//! from the caller's word, so the caller's source carries the commands the
//! script runs and their positions.

use super::super::command_identity::CommandAllocationSite;
use super::super::executed_script_source::ExecutedScriptSource;
use super::super::source_origin::SourceOriginId;
use super::super::{DeferredSourceBody, SourceCommandBindings};
use std::sync::Arc;

/// One formal bound from a braced literal at the call that entered its body.
#[derive(Debug, Clone)]
pub(in super::super) struct ScriptFormalSource {
    /// Source of the entered body, where reads of the formal are written.
    body: Arc<SourceOriginId>,
    formal: String,
    value: String,
    /// The calling invocation and the written argument that supplied it.
    call: CommandAllocationSite,
    argument: usize,
    word: tcl_source::words::WordExpr,
    /// The calling activation, which `uplevel 1` in the entered body selects.
    caller_frame: crate::var_resolve::VariableExecutionFrame,
}

/// The formals of `body` bound from braced literals in `written` (the call's
/// words, head first). A call with an alias prefix, an expansion or an
/// unbindable argument count records none.
pub(super) fn script_formal_sources(
    call: &CommandAllocationSite,
    written: &[tcl_source::words::WordExpr],
    body: &DeferredSourceBody,
    prepended: usize,
    grammar: tcl_dialect::ParameterGrammar,
    caller_frame: &crate::var_resolve::VariableExecutionFrame,
) -> Vec<ScriptFormalSource> {
    use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
    let Some(origin) = body.source_origin.as_ref() else {
        return Vec::new();
    };
    let Some(arguments) = written.get(1..) else {
        return Vec::new();
    };
    if prepended != 0
        || arguments
            .iter()
            .any(|word| matches!(word, tcl_source::words::WordExpr::Expand { .. }))
    {
        return Vec::new();
    }
    let Ok(bindings) = tcl_syntax::formal_params::bind_formal_arguments(
        &body.parameters,
        arguments.len(),
        grammar,
    ) else {
        return Vec::new();
    };
    bindings
        .into_iter()
        .filter_map(|binding| {
            let Binding::Value {
                parameter,
                argument,
            } = binding
            else {
                return None;
            };
            let word = arguments.get(argument)?;
            let tcl_source::words::WordExpr::BracedLiteral { text, .. } = word else {
                return None;
            };
            Some(ScriptFormalSource {
                body: Arc::clone(origin),
                formal: body.parameters.get(parameter)?.name.clone(),
                value: text.clone(),
                call: call.clone(),
                argument,
                word: word.clone(),
                caller_frame: caller_frame.clone(),
            })
        })
        .collect()
}

impl SourceCommandBindings {
    /// The caller's written script when `word`, read in `origin`, is a whole
    /// read of a formal still holding the braced literal that bound it,
    /// with the calling activation that supplied it.
    pub(super) fn script_formal_operand(
        &mut self,
        origin: Option<&Arc<SourceOriginId>>,
        word: Option<&tcl_source::words::WordExpr>,
        value: Option<&str>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(
        ExecutedScriptSource,
        crate::var_resolve::VariableExecutionFrame,
    )> {
        let origin = origin?;
        let (spelling, _) = word?.sole_variable_substitution()?;
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)?;
        let value = value?;
        let source = self.script_formal_sources.iter().rev().find(|source| {
            source.formal == name && source.body == *origin && source.value == value
        })?;
        let script = ExecutedScriptSource::from_word(
            source.call.clone(),
            source.argument,
            &source.word,
            value,
            config,
        );
        let caller_frame = source.caller_frame.clone();
        let (call_source, span) = (Arc::clone(&source.call.source), source.word.source().span);
        self.record_executed_word(&call_source, span, None, script.clone());
        Some((script, caller_frame))
    }
}
