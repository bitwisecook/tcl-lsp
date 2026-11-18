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

//! The value a compiled site leaves where the bytecode compiler pushes a
//! constant in place of the command it selected.

use crate::ir::CommandTokens;
use tcl_registry::native_compilation::NativeCompilationSelection;
use tcl_registry::value_transfer::{ExactValueOrUnavailable, LiftedAnswer, RepresentationEvidence};

/// The representation of the constant the compiler pushes for `tokens`,
/// whose site it compiled inline: the registry's
/// [`tcl_registry::native_scalar_compilation::NativeScalarOperation::compiled_constant`]
/// over the operand words' compiler shapes. `None` where the site is not
/// compiled inline or the instruction runs.
pub(super) fn compiled_constant(
    tokens: &CommandTokens,
    resolved: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<RepresentationEvidence> {
    let binding = tokens.source_binding.as_ref()?;
    if !matches!(
        binding.native_compilation_admission,
        Some(NativeCompilationSelection::Inline { .. })
    ) {
        return None;
    }
    let (operation, _) = resolved
        .semantics
        .native_compilation?
        .scalar_compilation()?;
    let dialect = binding.variable_context.invocation_dialect?;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
        return None;
    }
    let version = dialect.tcl_version?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let original = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &original,
        dialect.native_string_protocol()?,
    )
    .ok()?;
    let shapes = words.shapes();
    let operands = shapes.get(shapes.len().checked_sub(operation.arity())?..)?;
    operation.compiled_constant(operands, version)
}

/// `answer` with each evaluated result carrying `representation`: the
/// value the compiled program holds, whatever the command would build.
pub(super) fn with_representation(
    answer: LiftedAnswer,
    representation: RepresentationEvidence,
) -> LiftedAnswer {
    let LiftedAnswer::Evaluated(mut outcomes) = answer else {
        return answer;
    };
    for outcome in &mut outcomes {
        if let ExactValueOrUnavailable::Exact(value) = &mut outcome.result {
            value.representation = representation;
        }
    }
    LiftedAnswer::Evaluated(outcomes)
}
