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

//! Grammars of the compiled `info` and `namespace` introspection commands and the implementations they name.

use super::implementation_lookup::NativeCompilerImplementationLookup;
use super::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
    NativeNamedInvocationProtocol, literal_shape, static_value_shape,
};
use crate::{InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;

pub(super) const INFO_EXISTS_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "exists",
        slot: "::tcl::info::exists",
        command: "info",
        prepended: &["exists"],
    };

pub(super) const INFO_COROUTINE_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "coroutine",
        slot: "::tcl::info::coroutine",
        command: "info",
        prepended: &["coroutine"],
    };

pub(super) const INFO_LEVEL_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "level",
        slot: "::tcl::info::level",
        command: "info",
        prepended: &["level"],
    };

pub(super) const NAMESPACE_WHICH_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "which",
        slot: "::tcl::namespace::which",
        command: "namespace",
        prepended: &["which"],
    };

pub(super) const NAMESPACE_CURRENT_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "current",
        slot: "::tcl::namespace::current",
        command: "namespace",
        prepended: &["current"],
    };

pub(super) const NAMESPACE_ORIGIN_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "origin",
        slot: "::tcl::namespace::origin",
        command: "namespace",
        prepended: &["origin"],
    };

pub(super) const NAMESPACE_CODE_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "code",
        slot: "::tcl::namespace::code",
        command: "namespace",
        prepended: &["code"],
    };

pub(super) fn namespace_code_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    // TclCompileNamespaceCodeCmd accepts one SIMPLE_WORD. The namespace is
    // captured at execution, and an existing scoped prefix uses the handler.
    let value = words.arguments().literal_at(0);
    let already_scoped =
        value.is_some_and(|value| value.len() > 20 && value.starts_with("::namespace inscope "));
    if shapes.len() == 1 && literal_shape(shapes[0]) && value.is_some() && !already_scoped {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
            == tcl_syntax::naming::unroot_rooted_key(NAMESPACE_CODE_IMPLEMENTATION.slot).unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_CODE_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

pub(super) fn namespace_which_selection(
    shapes: &[NativeCompilationWordShape],
    option: Option<&[u8]>,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use crate::native_introspection_compilation::{
        NativeNamespaceWhichShape, namespace_which_shape,
    };
    match namespace_which_shape(shapes, option, version) {
        NativeNamespaceWhichShape::Command { .. } => NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        },
        NativeNamespaceWhichShape::Declined => NativeCompilationSelection::Generic,
        NativeNamespaceWhichShape::Unavailable => NativeCompilationSelection::Unknown,
    }
}

pub(super) fn original_namespace_which_grammar(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use crate::native_introspection_compilation::{
        NativeIntrospectionKind, compile_native_introspection,
    };
    if words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version) {
        return NativeCompilationSelection::Unknown;
    }
    let Some(shapes) = words.shapes().get(operand_from..) else {
        return NativeCompilationSelection::Unknown;
    };
    let selected =
        namespace_which_selection(shapes, words.literal(operand_from), version, operation);
    if matches!(selected, NativeCompilationSelection::Inline { .. })
        && compile_native_introspection(
            words,
            operand_from,
            NativeIntrospectionKind::NamespaceWhich,
            version,
        )
        .is_none()
    {
        NativeCompilationSelection::Unknown
    } else {
        selected
    }
}

pub(super) fn namespace_origin_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.len() == 1 && !matches!(shapes[0], NativeCompilationWordShape::Expanded) {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
            == tcl_syntax::naming::unroot_rooted_key(NAMESPACE_ORIGIN_IMPLEMENTATION.slot).unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_ORIGIN_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

pub(super) fn namespace_current_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.is_empty() {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
            == tcl_syntax::naming::unroot_rooted_key(NAMESPACE_CURRENT_IMPLEMENTATION.slot).unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_CURRENT_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

pub(super) fn info_level_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.len() <= 1 {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head) == "tcl::info::level"
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &INFO_LEVEL_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

pub(super) fn info_commands_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 || !matches!(shapes.len(), 1 | 2) || !literal_shape(shapes[0]) {
        return NativeCompilationSelection::Generic;
    }
    let absolute = shapes.len() == 2
        && static_value_shape(shapes[1])
        && words.arguments().literal_at(1).is_some_and(|pattern| {
            crate::native_introspection_compilation::native_info_commands_literal_is_trivial(
                pattern.as_bytes(),
            )
        });
    if absolute {
        NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NativeCompilerImplementationLookup {
                ensemble: "::info",
                member: "commands",
                slot: "::tcl::info::commands",
                command: "info",
                prepended: &["commands"],
            },
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::Direct,
        }
    }
}
