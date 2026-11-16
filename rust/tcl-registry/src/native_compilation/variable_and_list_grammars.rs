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

//! Grammars of the compiled variable and list commands: `unset`, `append`/`lappend`, `array`, `upvar`, `lassign`, `linsert` and `lrange`.

use super::implementation_lookup::NativeCompilerImplementationLookup;
use super::{
    NativeAppendKind, NativeArrayCommand, NativeCompilationContext, NativeCompilationFrame,
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
    NativeNamedInvocationProtocol, literal_shape, static_value_shape,
};
use crate::{InvocationDialect, InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;

pub(super) fn literal_unset_grammar(
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> Result<bool, NativeCompilationSelection> {
    if version < TclVersion::V8_6 {
        Err(NativeCompilationSelection::Generic)
    } else if !shapes.iter().all(|shape| literal_shape(*shape)) {
        // Dynamic operands can interleave evaluation and individual unsets.
        Err(NativeCompilationSelection::Unknown)
    } else {
        Ok(true)
    }
}

pub(super) fn array_compilation_grammar(
    operation: SemanticOperationId,
    command: NativeArrayCommand,
    lookup: &'static NativeCompilerImplementationLookup,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let private = words.head_literal().is_some_and(|head| {
        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
            == tcl_syntax::naming::unroot_rooted_key(lookup.slot).unwrap_or(lookup.slot)
    });
    array_compilation_grammar_for(
        (operation, command, lookup, private),
        words,
        shapes,
        version,
        context,
    )
}

pub(super) fn array_compilation_grammar_for(
    descriptor: (
        SemanticOperationId,
        NativeArrayCommand,
        &'static NativeCompilerImplementationLookup,
        bool,
    ),
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let (operation, command, lookup, private) = descriptor;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let from = usize::from(!private);
    let Some(operands) = shapes.get(from..) else {
        return Selection::Generic;
    };
    let arity = match command {
        NativeArrayCommand::Exists => crate::Arity::exact(1),
        NativeArrayCommand::Set => crate::Arity::exact(2),
        NativeArrayCommand::Unset => crate::Arity::new(1, 2),
    };
    if !u16::try_from(operands.len()).is_ok_and(|count| arity.accepts(count))
        || (!private && !literal_shape(shapes[0]))
    {
        return Selection::Generic;
    }
    let arguments = words.arguments().slice_from(from);
    if command == NativeArrayCommand::Set {
        let values = arguments
            .literal_at(1)
            .and_then(|value| tcl_syntax::list::split_list(value).ok());
        let empty = values.as_ref().is_some_and(Vec::is_empty);
        let odd = values
            .as_ref()
            .is_some_and(|values| !values.len().is_multiple_of(2));
        if odd
            || !literal_shape(operands[0])
            || (context.frame != NativeCompilationFrame::ProcedureCode && !empty)
        {
            return Selection::NamedInvocation {
                lookup,
                arguments_from: from,
                protocol: NativeNamedInvocationProtocol::Direct,
            };
        }
    } else if command == NativeArrayCommand::Unset && operands.len() == 2 {
        return Selection::NamedInvocation {
            lookup,
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::Direct,
        };
    }
    if literal_shape(operands[0])
        && arguments
            .literal_at(0)
            .is_some_and(|name| tcl_syntax::naming::split_array_name(name).1.is_some())
    {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(super) fn variable_append_grammar(
    kind: NativeAppendKind,
    operation: SemanticOperationId,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    variable_append_grammar_bytes(
        kind,
        operation,
        words.arguments().literal_at(0).map(str::as_bytes),
        shapes,
        version,
        context,
    )
}

pub(super) fn variable_append_grammar_bytes(
    kind: NativeAppendKind,
    operation: SemanticOperationId,
    name: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if shapes.contains(&NativeCompilationWordShape::Opaque) {
        return Selection::Unknown;
    }
    if kind == NativeAppendKind::List && version < TclVersion::V8_6 {
        match context.frame {
            NativeCompilationFrame::ScriptCode => return Selection::Generic,
            NativeCompilationFrame::Unknown => return Selection::Unknown,
            NativeCompilationFrame::ProcedureCode => {}
        }
    }
    if shapes.is_empty() {
        return if version == TclVersion::V8_4 {
            Selection::CompileError
        } else {
            Selection::Generic
        };
    }
    let selected = match kind {
        NativeAppendKind::String if shapes.len() <= 2 => true,
        NativeAppendKind::String if version < TclVersion::V8_6 => false,
        NativeAppendKind::String => {
            match context.frame {
                NativeCompilationFrame::ScriptCode => return Selection::Generic,
                NativeCompilationFrame::Unknown => return Selection::Unknown,
                NativeCompilationFrame::ProcedureCode => {}
            }
            if !literal_shape(shapes[0]) {
                return Selection::Generic;
            }
            let Some(name) = name else {
                return Selection::Unknown;
            };
            !tcl_syntax::naming::is_qualified(name)
                && tcl_syntax::naming::split_element_ref_bytes(name).is_none()
        }
        NativeAppendKind::List if version < TclVersion::V8_6 => shapes.len() == 2,
        NativeAppendKind::List if version < TclVersion::V9_1 => shapes.len() >= 2,
        NativeAppendKind::List => shapes[0] != NativeCompilationWordShape::Expanded,
    };
    if !selected {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::for_c_version(version),
    }
}

pub(super) fn list_insertion_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 || shapes.len() < 2 {
        return Selection::Generic;
    }
    if version < TclVersion::V9_0 {
        if !static_value_shape(shapes[1]) {
            return Selection::Generic;
        }
        let Some(index) = words.arguments().literal_at(1) else {
            return Selection::Unknown;
        };
        match tcl_cmd_core::index::compiler_encodable_in(
            index,
            tcl_dialect::IndexSyntax::for_version(version),
        ) {
            Some(true) => {}
            Some(false) => return Selection::Generic,
            None => return Selection::Unknown,
        }
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(super) fn list_range_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 || shapes.len() != 3 {
        return Selection::Generic;
    }
    for index in [1, 2] {
        if !static_value_shape(shapes[index]) {
            return Selection::Generic;
        }
        let Some(value) = words.arguments().literal_at(index) else {
            return Selection::Unknown;
        };
        let before = if index == 1 { 0 } else { -1 };
        let after = if index == 1 && version < TclVersion::V9_0 {
            i32::MAX
        } else if index == 1 {
            -1
        } else {
            -2
        };
        match tcl_cmd_core::index::compiled_list_bound_in(value, version, before, after) {
            Ok(Some(bound)) if index != 1 || bound.encoded() != -1 => {}
            Ok(_) => return Selection::Generic,
            Err(_) => return Selection::Unknown,
        }
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(super) fn list_assignment_grammar(
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_5 || shapes.len() < 2 {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(super) fn upvar_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if version == TclVersion::V8_4 || context.frame == NativeCompilationFrame::ScriptCode {
        return Err(Selection::Generic);
    }
    if context.frame == NativeCompilationFrame::Unknown {
        return Err(Selection::Unknown);
    }
    if shapes.len() < 2 || !literal_shape(shapes[0]) {
        return Err(Selection::Generic);
    }
    let arguments = words
        .arguments()
        .with_dialect(InvocationDialect::for_version(version));
    let first = arguments.literal_at(0).ok_or(Selection::Unknown)?;
    let width = original_upvar_level_width(first, shapes.len(), version)?;
    for index in (width + 1..shapes.len()).step_by(2) {
        if !literal_shape(shapes[index]) {
            return Err(Selection::Generic);
        }
        let Some(name) = arguments.literal_at(index) else {
            return Err(Selection::Unknown);
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
        {
            return Err(Selection::Generic);
        }
    }
    Ok(true)
}

/// Compile-time leading-word probe shared by the original upvar recipe and
/// grammar selector. This does not select a runtime frame or install a cache.
///
/// # Errors
/// Returns native decline or unavailable invocation-stack evidence.
pub fn original_upvar_level_width(
    first: &str,
    count: usize,
    version: TclVersion,
) -> Result<usize, NativeCompilationSelection> {
    use crate::frame_effect::{
        FrameArgLayout, FrameArgumentResolution, FrameLevel, FrameLevelWord,
    };
    use NativeCompilationSelection as Selection;
    let mut operands = vec![crate::InvocationWord::Dynamic; count];
    let Some(leading) = operands.first_mut() else {
        return Err(Selection::Generic);
    };
    *leading = crate::InvocationWord::Literal(first);
    let grammar = crate::FrameEffectSpec {
        level_word: FrameLevelWord::LeadingProbe,
        layout: FrameArgLayout::AliasPairs,
    };
    let arguments = InvocationWords::structured(crate::InvocationWord::Dynamic, &operands)
        .arguments()
        .with_dialect(InvocationDialect::for_version(version));
    match grammar.resolve_arguments(arguments) {
        FrameArgumentResolution::Valid {
            level_word_len,
            level,
        } => {
            if level_word_len != 0
                && !matches!(
                    level,
                    FrameLevel::Relative(0 | 1) | FrameLevel::Absolute(0 | 1)
                )
            {
                return Err(Selection::Unknown);
            }
            Ok(level_word_len)
        }
        FrameArgumentResolution::Invalid => Err(Selection::Generic),
        FrameArgumentResolution::Unknown => Err(Selection::Unknown),
    }
}
