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

use super::tests::decode_semantic;
use super::*;
use std::sync::Arc;

/// Representative iRules sources: the shipped samples, the screenshot
/// fixtures, and an inline rule with procedures, nested event bodies and
/// every object-reference command family.
fn fixtures() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "cookbook_http_collect",
            include_str!("../../../../samples/irules/cookbook_http_collect.irul"),
        ),
        (
            "cookbook_tcp_collect",
            include_str!("../../../../samples/irules/cookbook_tcp_collect.irul"),
        ),
        (
            "diagnostics_event_flow",
            include_str!("../../../../samples/irules/diagnostics_event_flow.irul"),
        ),
        (
            "diagnostics_security",
            include_str!("../../../../samples/irules/diagnostics_security.irul"),
        ),
        (
            "diagnostics_style_perf",
            include_str!("../../../../samples/irules/diagnostics_style_perf.irul"),
        ),
        (
            "security_taint_before",
            include_str!("../../../../samples/for_screenshots/05-security-taint-before.irul"),
        ),
        (
            "template_fallback",
            include_str!("../../../../samples/for_screenshots/24-template-fallback.irul"),
        ),
        (
            "extract_datagroup_before",
            include_str!("../../../../samples/for_screenshots/30-extract-datagroup-before.irul"),
        ),
        (
            "inline",
            concat!(
                "proc helper {} { pool helper_pool }\n",
                "when CLIENT_ACCEPTED {\n",
                "  virtual vs1\n  node n1\n  snatpool sp1\n",
                "}\n",
                "when HTTP_REQUEST priority 100 {\n",
                "  call helper\n",
                "  if {[class match [HTTP::host] equals hosts_dg]} {\n",
                "    pool web_pool\n",
                "  } else {\n",
                "    pool $dynamic\n",
                "  }\n",
                "  set v [class lookup [HTTP::uri] uri_dg]\n",
                "}\n",
            ),
        ),
    ]
}

/// The analysis and unit the server holds for an iRules document.
fn irules_document(
    source: &str,
) -> (
    tcl_compiler::analyser::AnalysisResult,
    CompilationUnit,
    &'static tcl_dialect::DialectProfile,
) {
    let profile = tcl_dialect::DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_context_registry(
        source,
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(&context),
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    (analysis, unit, profile)
}

/// The object overlay of a document read under its own analysis — with
/// the caller's unit, or the unit the analysis rebuilds — paints exactly
/// the objects the standalone source API paints for the same iRules
/// source: moving to the document's input loses no highlighting.
#[test]
fn object_overlay_under_the_document_analysis_loses_nothing() {
    let objects = |tokens: &SemanticTokens| {
        decode_semantic(tokens)
            .into_iter()
            .filter(|token| token.3 == TokenKind::Object as u32)
            .collect::<Vec<_>>()
    };
    let mut painted = 0;
    for (name, source) in fixtures() {
        let (analysis, unit, profile) = irules_document(source);
        let registry = analysis.resolved_registry().expect("registry");
        let standalone = objects(&full(source, profile, registry));
        let with_unit = objects(&full_with_cu_and_analysis(
            source,
            profile,
            registry,
            Some(&unit),
            Some(&analysis),
        ));
        let without_unit = objects(&full_with_cu_and_analysis(
            source,
            profile,
            registry,
            None,
            Some(&analysis),
        ));
        assert_eq!(
            with_unit, standalone,
            "{name}: document analysis with its unit"
        );
        assert_eq!(without_unit, standalone, "{name}: document analysis alone");
        painted += standalone.len();
    }
    assert!(painted > 0, "the fixtures exercise the object overlay");
}

/// An event declaration role comes from the document's own registry: an
/// iRules document paints `when EVENT`'s event; a plain Tcl buffer, whose
/// registry has no event-handler command, borrows none from a
/// process-wide iRules store.
#[test]
fn event_declarations_read_the_documents_own_registry() {
    let source = "when HTTP_REQUEST {\n    set x 1\n}\n";
    let events = |tokens: &SemanticTokens| {
        decode_semantic(tokens)
            .into_iter()
            .filter(|token| token.3 == TokenKind::Event as u32)
            .map(|token| (token.0, token.1, token.2))
            .collect::<Vec<_>>()
    };
    let (analysis, unit, profile) = irules_document(source);
    let registry = analysis.resolved_registry().expect("registry");
    assert_eq!(
        events(&full_with_cu_and_analysis(
            source,
            profile,
            registry,
            Some(&unit),
            Some(&analysis),
        )),
        vec![(0, 5, 12)],
        "iRules document"
    );
    let tcl = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let plain = CommandRegistry::build_default();
    assert_eq!(
        plain.commands_with_trait(tcl_registry::Traits::IS_EVENT_HANDLER),
        [] as [&str; 0]
    );
    assert!(events(&full(source, tcl, &plain)).is_empty(), "plain Tcl");
}
