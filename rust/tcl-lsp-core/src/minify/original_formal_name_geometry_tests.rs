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

fn logical_input() -> tcl_compiler::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        LexerConfig::for_profile(Some(profile)),
    )
}

#[test]
fn compact_original_formal_fields_preserve_wrappers_and_unicode_defaults() {
    let input = logical_input();
    let source = r#"proc p {{{longName} {default α}} {"otherName" longName} args} {return [list $longName $otherName $args]}"#;
    let analysis = original::analysis_with_input(source, &input);
    let procedure = analysis.all_procs.values().next().unwrap();
    let fields = lexical_metadata::parameter_name_sources(source, &analysis, procedure)
        .expect("actual Logical header, ordered formal fields and body card");
    assert_eq!(
        fields
            .iter()
            .map(tcl_compiler::signature_scan::formal_parameters::OriginalSourceFormalName::name)
            .collect::<Vec<_>>(),
        [
            b"longName".as_slice(),
            b"otherName".as_slice(),
            b"args".as_slice()
        ]
    );
    let mut edits = Vec::new();
    rename_params_in_list(
        &fields,
        &BTreeMap::from([
            ("longName".to_owned(), "a".to_owned()),
            ("otherName".to_owned(), "b".to_owned()),
        ]),
        &mut edits,
    );
    assert_eq!(
        apply_edits(source, edits),
        r#"proc p {{{a} {default α}} {"b" longName} args} {return [list $longName $otherName $args]}"#
    );
}

#[test]
fn compact_original_formal_fields_reach_the_existing_editing_pass() {
    let input = logical_input();
    let source = "proc p {{longName default} {otherName longName} args} {return VALUE}";
    let (output, mapping) = compact_names(source, &input, false);
    assert_eq!(
        output,
        "proc p {{a default} {b longName} args} {return VALUE}"
    );
    assert_eq!(mapping.variables["::p"]["longName"], "a");
    assert_eq!(mapping.variables["::p"]["otherName"], "b");
    assert!(!mapping.variables["::p"].contains_key("args"));
}

#[test]
fn compact_original_formal_fields_refuse_cooked_dynamic_and_foreign_geometry() {
    let input = logical_input();
    for source in [
        r#"proc p "first\x20second" {return $first}"#,
        "proc p $parameters {return VALUE}",
    ] {
        let analysis = original::analysis_with_input(source, &input);
        for procedure in analysis.all_procs.values() {
            assert!(
                lexical_metadata::parameter_name_sources(source, &analysis, procedure).is_none(),
                "{source}"
            );
        }
        let (output, _) = compact_names(source, &input, false);
        assert_eq!(output, source);
    }
    let source = "proc p {longName} {return $longName}";
    let mut analysis = original::analysis_with_input(source, &input);
    let actual = analysis.all_procs.values().next().unwrap();
    assert!(lexical_metadata::parameter_name_sources(source, &analysis, actual).is_some());
    let procedure = actual.clone();
    let mut wrong_procedure = procedure.clone();
    wrong_procedure.params[0].name = "otherName".to_owned();
    assert!(
        lexical_metadata::parameter_name_sources(source, &analysis, &wrong_procedure).is_none()
    );
    assert!(
        lexical_metadata::parameter_name_sources(
            "proc p {otherName} {return $longName}",
            &analysis,
            analysis.all_procs.values().next().unwrap()
        )
        .is_none()
    );
    analysis.body_lexer_config = Some(LexerConfig {
        leading_bom: match input.lexer_config().leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        },
        ..input.lexer_config()
    });
    assert!(
        lexical_metadata::parameter_name_sources(
            source,
            &analysis,
            analysis.all_procs.values().next().unwrap()
        )
        .is_none()
    );
    let native = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let native_analysis =
        tcl_compiler::analyser::Analyser::new().analyse(source, native.analyser_profile().name);
    for procedure in native_analysis.all_procs.values() {
        assert!(
            lexical_metadata::parameter_name_sources(source, &native_analysis, procedure).is_none()
        );
    }
}
