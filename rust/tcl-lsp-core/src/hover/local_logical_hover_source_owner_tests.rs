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
use tcl_compiler::analyser::Analyser;

fn logical_analysis(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn local_logical_hover_uses_current_procedure_class_and_member_owners() {
    let source = "proc p {arg} {return $arg}; p 1";
    let analysis = logical_analysis(source);
    assert!(analysis.allows_retained_logical_declaration_advice());
    let call_cursor = u32::try_from(source.rfind("p 1").unwrap()).unwrap();
    let call = proc_hover_at(
        &analysis,
        source,
        call_cursor,
        "p",
        crate::definition::CallResolution::document_only(),
    )
    .expect("current original Logical call candidate");
    assert!(call.value.contains("Possible procedure"));
    assert!(call.value.contains("proc ::p {arg} {...}"));
    assert!(
        proc_hover_at(
            &analysis,
            &format!("{source} "),
            call_cursor,
            "p",
            crate::definition::CallResolution::document_only(),
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        proc_hover_at(
            &missing,
            source,
            call_cursor,
            "p",
            crate::definition::CallResolution::document_only(),
        )
        .is_none()
    );

    let class_source = "oo::class create C {method m {} {}}";
    let class_analysis = logical_analysis(class_source);
    let class_cursor = u32::try_from(class_source.find("C {").unwrap()).unwrap();
    let class = class_analysis
        .logical_source_class_at(
            &tcl_lexer::SourceImage::document(class_source),
            class_analysis.body_lexer_config.unwrap(),
            class_cursor,
        )
        .expect("current original Logical class candidate");
    assert!(
        class_analysis
            .logical_source_class_declaration(class)
            .is_some()
    );
    let class_hover = hover_impl(
        class_source,
        0,
        class_cursor,
        &class_analysis,
        crate::definition::CallResolution::document_only(),
        crate::profile_for_analysis(&class_analysis),
        false,
    )
    .expect("source-owned class hover");
    assert!(class_hover.value.contains("oo::class create ::C"));
    assert!(!class_hover.value.contains("provider and dispatch"));

    let member_cursor = u32::try_from(class_source.find("m {} {").unwrap()).unwrap();
    let member = class_member_hover_text(class_source, &class_analysis, "m", member_cursor)
        .expect("current original Logical member candidate");
    assert!(member.contains("**method** `::C::m`"), "{member}");
    assert!(!member.contains("provider and dispatch"), "{member}");
    assert!(
        class_member_hover_text(
            &format!("{class_source} "),
            &class_analysis,
            "m",
            member_cursor,
        )
        .is_none()
    );
}
