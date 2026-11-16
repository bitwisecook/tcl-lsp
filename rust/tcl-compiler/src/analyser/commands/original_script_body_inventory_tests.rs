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

use crate::analyser::Analyser;
use tcl_lexer::SourceImage;

#[test]
fn original_script_inventory_uses_whole_authored_body_and_case_regions() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    for source in [
        "proc f {} { ILX::call $h m }",
        "eval { ILX::call $h m }",
        "when X { if {1} { ILX::call $h m } }",
        "when X { foreach i {1 2} { ILX::call $h m } }",
        "when X { catch { ILX::call $h m } }",
        "when X { while {0} { ILX::call $h m } }",
        "when X { switch [HTTP::uri] { /api { ILX::call $h m } } }",
        "when X { switch [HTTP::uri] /api { ILX::call $h m } }",
        "proc f {} { set h [ILX::init p e]; ILX::call $h m }",
    ] {
        let analysis = Analyser::new().analyse(source, "f5-irules");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let offset = u32::try_from(source.find("ILX::call").unwrap()).unwrap();
        let occurrence = analysis
            .original_vendor_source_names()
            .find(|occurrence| {
                occurrence.site().offset == offset
                    && occurrence.original_words().first()
                        == Some(occurrence.name_input().original_word())
            })
            .unwrap_or_else(|| panic!("missing genuine script-body call: {source}"));
        assert!(occurrence.body_origin().is_some(), "{source}");
        assert_eq!(occurrence.original_words().len(), 3, "{source}");
        assert!(
            occurrence
                .original_words()
                .iter()
                .all(|word| word.image() == &image && word.config() == config)
        );
        let (metadata, _) =
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source, &analysis, offset,
            )
            .unwrap_or_else(|| panic!("missing readonly source schema: {source}"));
        assert_eq!(metadata.shape().command(), "ILX::call");
        assert!(metadata.matches_source(&image, config));
        assert!(
            analysis.original_completed_command_world().is_none(),
            "stored body syntax cannot donate a completed world: {source}"
        );
        assert_eq!(analysis.original_procedure_declarations().count(), 0);
        assert!(
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                &format!("{source}\n# changed"),
                &analysis,
                offset,
            )
            .is_none()
        );
    }
}

#[test]
fn original_script_inventory_keeps_inert_cooked_and_blocked_bodies_opaque() {
    // Implementation contract: naming.vendor.original-source-context-input
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
    // The moved-handler control has an independently checked positive
    // parent and child schema in the very same hosted authoring context.
    let baseline = "if 1 { puts baseline }";
    let baseline_analysis = Analyser::new().analyse(baseline, "f5-tmsh");
    let baseline_offset = u32::try_from(baseline.find("puts").unwrap()).unwrap();
    let baseline_child = baseline_analysis
        .original_vendor_source_names()
        .find(|occurrence| {
            occurrence.site().offset == baseline_offset
                && occurrence.original_words().first()
                    == Some(occurrence.name_input().original_word())
        })
        .expect("tmsh if body must retain its genuine common child");
    assert!(baseline_child.body_origin().is_some());
    let (baseline_metadata, baseline_segment) =
        crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
            baseline,
            &baseline_analysis,
            baseline_offset,
        )
        .expect("tmsh puts baseline must select its readonly source schema");
    assert_eq!(baseline_metadata.shape().command(), "puts");
    assert_eq!(baseline_segment.argv[0].span.start(), baseline_offset);
    for source in [
        "set text { ILX::call $h m }",
        "unknown { ILX::call $h m }",
        r#"if 1 "ILX::call\ \$h\ m""#,
        "proc if {args} {}; if 1 { ILX::call $h m }",
        "rename if saved_if; if 1 { puts blocked }",
        "eval { ILX::call } { $h m }",
    ] {
        // Direct rename is not in the TMM authoring surface. A known
        // moved-handler barrier must select its actual hosted context.
        let dialect = if source.starts_with("rename if") {
            "f5-tmsh"
        } else {
            "f5-irules"
        };
        let analysis = Analyser::new().analyse(source, dialect);
        if source.starts_with("rename if") {
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert!(
                context
                    .context()
                    .resolve_spec(context.commands(), "rename")
                    .is_some()
            );
        }
        let child = if source.starts_with("rename if") {
            "puts"
        } else {
            "ILX::call"
        };
        let offset = u32::try_from(source.find(child).unwrap()).unwrap();
        let occurrences = analysis
            .original_vendor_source_names()
            .filter(|occurrence| occurrence.site().offset == offset)
            .collect::<Vec<_>>();
        assert!(
            occurrences
                .iter()
                .all(|occurrence| occurrence.body_origin().is_none()),
            "inert, cooked, blocked or concatenated source cannot donate a body-role receipt: {source}"
        );
        assert!(
            crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
                source, &analysis, offset,
            )
            .is_none_or(|(_, actual)| actual
                .argv
                .first()
                .is_none_or(|head| head.span.start() != offset)),
            "an inert cursor or lexical vector cannot supply child source roles: {source}"
        );
        if !source.starts_with("proc if") && !source.starts_with("rename if") {
            assert!(
                occurrences.is_empty(),
                "inert, cooked or concatenated source is not a script child: {source}"
            );
        }
    }
}
#[test]
fn original_expression_inventory_keeps_missing_input_and_dynamic_body_opaque() {
    let source = "when HTTP_REQUEST { expr {[class match [HTTP::host] equals /Common/guard_dg]} }";
    let analysis = Analyser::new().analyse(source, "f5-irules");
    let config = analysis.body_lexer_config.unwrap();
    let offset = u32::try_from(source.find("expr ").unwrap()).unwrap();
    let (_, command) =
        crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
            source, &analysis, offset,
        )
        .unwrap();
    let words = crate::registry_invocation::source_structure::source_registry_words(
        source, &analysis, &command,
    )
    .unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    assert_eq!(
        words.source_expression_script_bodies(input).unwrap().len(),
        1
    );
    let child = u32::try_from(source.find("class match").unwrap()).unwrap();
    assert!(analysis.original_vendor_source_names().any(|occurrence| {
        occurrence.site().offset == child
            && occurrence.original_words().first() == Some(occurrence.name_input().original_word())
            && occurrence.body_origin().is_some()
    }));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        crate::registry_invocation::source_structure::source_registry_words(
            source, &missing, &command,
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::source_structure::source_registry_words(
            &source.replace("guard_dg", "other_dg"),
            &analysis,
            &command,
        )
        .is_none()
    );
    let foreign_profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        foreign_profile,
        foreign_profile,
        tcl_registry::model::context_for_profile(foreign_profile),
        config,
    );
    assert!(words.source_expression_script_bodies(&foreign).is_none());
    let dynamic = "when HTTP_REQUEST { expr $body }";
    let dynamic_analysis = Analyser::new().analyse(dynamic, "f5-irules");
    let offset = u32::try_from(dynamic.find("expr ").unwrap()).unwrap();
    let (_, command) =
        crate::registry_invocation::source_structure::selected_vendor_registry_words_at(
            dynamic,
            &dynamic_analysis,
            offset,
        )
        .unwrap();
    let words = crate::registry_invocation::source_structure::source_registry_words(
        dynamic,
        &dynamic_analysis,
        &command,
    )
    .unwrap();
    assert!(
        words
            .source_expression_script_bodies(dynamic_analysis.resolved_input.as_ref().unwrap(),)
            .is_none()
    );
}
