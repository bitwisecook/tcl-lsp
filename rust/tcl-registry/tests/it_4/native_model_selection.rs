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

//! Registry selections over the native model: compile recipes and command
//! admission read through specs, at the dialects the native model selects.

use tcl_dialect::TclVersion;
use tcl_native_model::InvocationDialect;
use tcl_registry::InvocationWords;
use tcl_registry::native_compilation::{
    NamedCompilerOperandView, NativeCompilationContext, NativeCompilationFrame,
    NativeCompilationMode, NativeCompilationSelection, NativeCompilationWordShape,
    NativeNamedInvocationProtocol,
};

#[test]
fn array_default_uses_the_actual_private_worker_only_at_accepted_outer_arity() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let context = NativeCompilationContext {
        frame: NativeCompilationFrame::ProcedureCode,
        mode: NativeCompilationMode::BytecodeObject,
        ..Default::default()
    };
    let original = NamedCompilerOperandView::OriginalArguments {
        selectors_retained: 1,
    };
    for arguments in [
        vec!["default", "get", "a", "EXTRA"],
        vec!["default", "get", "a"],
    ] {
        let spec = registry
            .resolve_invocation("array", &arguments, registry.own_surface_query())
            .unwrap()
            .facts()
            .native_compilation
            .unwrap();
        let shapes = vec![NativeCompilationWordShape::Literal; arguments.len()];
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = InvocationDialect::for_version(version);
            let words = InvocationWords::literals("array", &arguments);
            assert_eq!(
                spec.select(words, &shapes, Some(dialect), context),
                NativeCompilationSelection::Unknown
            );
            assert!(matches!(
                spec.select_with_named_operand_view(words, &shapes, Some(dialect), context, original),
                NativeCompilationSelection::NamedInvocation {
                    lookup,
                    arguments_from: 1,
                    protocol: NativeNamedInvocationProtocol::Direct,
                } if lookup.slot == "::tcl::array::default"
            ));
        }
        assert_eq!(
            spec.select_with_named_operand_view(
                InvocationWords::literals("array", &arguments),
                &shapes,
                Some(InvocationDialect::for_version(TclVersion::V8_6)),
                context,
                original,
            ),
            NativeCompilationSelection::Generic
        );
        let broad = ["default", "get"];
        assert_eq!(
            spec.select_with_named_operand_view(
                InvocationWords::literals("array", &broad),
                &[NativeCompilationWordShape::Literal; 2],
                Some(InvocationDialect::for_version(TclVersion::V9_0)),
                context,
                original,
            ),
            NativeCompilationSelection::Generic
        );
    }
}

#[test]
fn stock_admission_does_not_exclude_live_custom_jim_commands() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let dialect = InvocationDialect::of_profile(profile);
    let registry = tcl_registry::model::static_context_for_profile(profile).commands();
    assert!(dialect.native_jim_local_protocol().is_some());
    assert_eq!(registry.native_command_admission("p", dialect), None);
    assert_eq!(registry.native_command_admission("::active", dialect), None);
    assert_eq!(
        registry.native_command_admission("set", dialect),
        Some(true)
    );
    assert_eq!(
        registry.native_command_admission("trace", dialect),
        Some(false)
    );
    assert!(
        InvocationDialect::for_version(TclVersion::V9_0)
            .native_jim_local_protocol()
            .is_none()
    );
}
