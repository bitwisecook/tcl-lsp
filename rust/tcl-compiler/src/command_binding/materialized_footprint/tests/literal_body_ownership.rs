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

//! Name ownership in a literal body operand: purposes, aliases and source currency.

use super::*;

#[test]
fn original_literal_body_ownership_keeps_named_purposes_and_literal_roots() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional lexical names only, never installed aliases or current reads.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let source = "catch {global ::N::x; set {$literal} VALUE; set {café(open} VALUE; puts $missing; set byname}";
    let unit = unit(source, &context);
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
    for (purpose, expected_names, expected_reads) in [
        (
            crate::script_binds::Ownership::Bindings,
            vec!["x", "$literal", "café(open"],
            Vec::<&str>::new(),
        ),
        (
            crate::script_binds::Ownership::BindingsOrNameReads,
            vec!["x", "$literal", "café(open"],
            vec!["byname"],
        ),
        (
            crate::script_binds::Ownership::ScopeAliases,
            vec!["x"],
            Vec::<&str>::new(),
        ),
    ] {
        let names = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            0,
            purpose,
        )
        .unwrap();
        assert!(!names.opaque, "{names:?}");
        assert_eq!(
            names
                .names
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            expected_names.into_iter().collect()
        );
        assert_eq!(
            names
                .read_names
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            expected_reads.into_iter().collect()
        );
        assert!(
            !names
                .names
                .iter()
                .chain(&names.read_names)
                .any(|name| name == "missing")
        );
    }
    assert!(
        crate::script_binds::original_literal_body_ownership(
            context.commands(),
            None,
            tokens,
            0,
            crate::script_binds::Ownership::Bindings,
        )
        .is_none()
    );
    assert!(
        crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            1,
            crate::script_binds::Ownership::Bindings,
        )
        .is_none()
    );
}

#[test]
fn original_literal_body_aliases_keep_local_names_and_opaque_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Held alias names/captures and a conditional residual, no frame or execution.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    for (source, expected, opaque) in [
        (
            "interp alias {} expose {} global ::N::x; catch {expose}",
            Some("x"),
            false,
        ),
        (
            "rename global retired; interp alias {} expose {} retired ::N::x; catch {expose}",
            Some("x"),
            false,
        ),
        (
            "interp alias {} expose {} global ::N::x; rename global {}; catch {expose}",
            None,
            true,
        ),
        ("catch {set known VALUE; eval $script}", None, true),
    ] {
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let names = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            InvocationMetadataContext::for_module(context.commands(), &unit.ir_module),
            tokens,
            0,
            crate::script_binds::Ownership::ScopeAliases,
        )
        .unwrap();
        assert_eq!(
            names.names.first().map(String::as_str),
            expected,
            "{source}: {names:?}"
        );
        assert_eq!(names.opaque, opaque, "{source}: {names:?}");
    }
}

#[test]
fn original_literal_body_ownership_keeps_availability_and_source_currency() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source names only; availability is not actual execution.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    for (context, expected_opaque) in [(&current, false), (&older, true)] {
        let unit = unit("catch {throw {ERR} message}", context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        let ownership = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            0,
            crate::script_binds::Ownership::Bindings,
        )
        .unwrap();
        assert_eq!(ownership.opaque, expected_opaque, "{ownership:?}");
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(
            crate::script_binds::original_literal_body_ownership(
                foreign.commands(),
                metadata,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
        let mut changed = tokens.clone();
        let tcl_source::words::WordExpr::BracedLiteral { text, .. } = &mut changed.word_exprs[1]
        else {
            panic!("original body word");
        };
        *text = "set fabricated VALUE".to_owned();
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                &changed,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
    }
}
