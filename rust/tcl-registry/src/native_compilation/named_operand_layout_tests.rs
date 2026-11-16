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

use super::*;
fn context() -> NativeCompilationContext {
    NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        loop_depth: 0,
        catch_depth: Some(0),
    }
}
#[test]
fn original_named_metadata_layout_keeps_public_nested_and_private_offsets() {
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        let public_args = ["encode", "hex", "DATA"];
        let public_words = InvocationWords::literals("binary", &public_args).with_dialect(dialect);
        let public =
            registry.resolve_structured_invocation(public_words, dialect.authoring_query());
        let public = public.resolved().unwrap().facts();
        let spec = public.native_compilation.unwrap();
        assert!(spec.is_named_invocation_compiler());
        for (head, args, selectors, role_offset) in [
            ("binary", vec!["encode", "hex", "DATA"], 2, Some(1)),
            ("::tcl::binary::encode", vec!["hex", "DATA"], 1, Some(0)),
            ("::tcl::binary::encode::hex", vec!["DATA"], 0, None),
        ] {
            let words = InvocationWords::literals(head, &args).with_dialect(dialect);
            let facts = registry
                .native_registration_invocation_facts(words)
                .or_else(|| {
                    registry
                        .resolve_structured_invocation(words, dialect.authoring_query())
                        .resolved()
                        .map(|resolved| resolved.facts())
                });
            assert_eq!(
                facts.as_ref().map(|facts| facts.argument_offset),
                role_offset,
                "{version:?}: {head}: role coordinates are not compiler selectors"
            );
            let shapes = vec![NativeCompilationWordShape::Literal; args.len()];
            if let Some(facts) = facts {
                assert_eq!(
                    spec.select_for_facts(words, &shapes, &facts, Some(dialect), context()),
                    NativeCompilationSelection::Unknown
                );
            }
            assert!(
                matches!(spec.select_with_named_operand_view(words, &shapes, Some(dialect), context(), NamedCompilerOperandView::OriginalArguments { selectors_retained: selectors }), NativeCompilationSelection::NamedInvocation { arguments_from, .. } if arguments_from == selectors)
            );
            assert_eq!(
                spec.select(words, &shapes, Some(dialect), context()),
                NativeCompilationSelection::Unknown
            );
            let missing = &args[..args.len() - 1];
            assert_eq!(
                spec.select_with_named_operand_view(
                    InvocationWords::literals(head, missing).with_dialect(dialect),
                    &shapes[..shapes.len() - 1],
                    Some(dialect),
                    context(),
                    NamedCompilerOperandView::OriginalArguments {
                        selectors_retained: selectors
                    }
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}
