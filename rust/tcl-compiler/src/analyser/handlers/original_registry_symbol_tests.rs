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

#[test]
fn original_registry_symbol_names_keep_opaque_inputs_without_command_identity_from_labels() {
    // Implementation contract: naming.compiler.original-symbol-declaration-advice
    // docs/design/analysis/name-resolution-proofs/original-symbol-declaration-advice.md
    let source = r"package require tcltest; tcltest::test case\uD800 {first} -body {} -result {}; tcltest::test case\uD801 {second} -body {} -result {}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(result.all_defined_symbols.len(), 2);
    let original = result
        .original_symbol_declarations()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(original.len(), 2);
    assert!(original.iter().all(|record| {
        record
            .original_namespace()
            .is_some_and(tcl_names::namespace::SignatureNamespaceScope::is_root)
    }));
    assert_ne!(
        original[0].name_input().bytes(),
        original[1].name_input().bytes()
    );
    let mut cleared = result.clone();
    cleared.all_defined_symbols.clear();
    cleared.global_scope.defined_symbols.clear();
    assert_eq!(cleared.original_symbol_declarations().count(), 2);
    assert!(original.iter().all(|record| record.matches_source(
        &tcl_lexer::SourceImage::document(source),
        result.body_lexer_config.unwrap()
    )));
    assert!(original.iter().all(|record| !record.matches_source(
        &tcl_lexer::SourceImage::document(&format!("# displaced\n{source}")),
        result.body_lexer_config.unwrap()
    )));
    let inputs = result
        .all_defined_symbols
        .iter()
        .map(|symbol| symbol.original_name_input.as_ref().unwrap().bytes())
        .collect::<Vec<_>>();
    assert_eq!(
        inputs,
        [
            b"case\xed\xa0\x80".as_slice(),
            b"case\xed\xa0\x81".as_slice()
        ]
    );
    assert_ne!(
        result.all_defined_symbols[0].name,
        result.all_defined_symbols[1].name
    );
    let shadow = Analyser::new().analyse(
        "proc ::tcltest::test args {}; tcltest::test ordinary {description} -body {}",
        "tcl8.6",
    );
    assert_eq!(
        shadow.all_defined_symbols,
        [] as [crate::analyser::types::DefinedSymbol; 0]
    );
}

#[test]
fn original_conditional_registry_symbol_cards_retain_unknown_applicability() {
    // Implementation contract: naming.compiler.conditional-registry-source-metadata
    // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
    let source = r"package require tcltest; ::tcltest::test case\uD800 {first} -body {} -result {}; ::tcltest::test case\uD801 {second} -body {} -result {}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let cards = result.original_symbol_declarations().collect::<Vec<_>>();
    assert_eq!(cards.len(), 2);
    let registry = result.resolved_registry().unwrap();
    for card in cards {
        let metadata = card
            .conditional_metadata()
            .expect("unknown package execution stays explicit");
        assert_ne!(metadata.obligations(), []);
        assert!(metadata.matches_source(
            &tcl_lexer::SourceImage::document(source),
            result.body_lexer_config.unwrap()
        ));
        assert!(metadata.matches_registry(registry));
        assert_eq!(
            metadata.symbol_definition().as_ref(),
            Some(card.descriptor())
        );
        assert!(!metadata.matches_source(
            &tcl_lexer::SourceImage::document(&format!("# changed\n{source}")),
            result.body_lexer_config.unwrap()
        ));
        assert!(!metadata.matches_registry(
            tcl_registry::model::ingress::static_context_for("jimtcl").commands()
        ));
    }
    assert!(
        result.original_completed_command_world().is_none(),
        "source cards do not close the execution world"
    );
}

#[test]
fn original_registry_symbol_cards_retain_the_actual_namespace_geometry() {
    // Implementation contract: naming.compiler.original-symbol-declaration-advice
    // docs/design/analysis/name-resolution-proofs/original-symbol-declaration-advice.md
    let source = "namespace eval suite {::tcltest::test scoped {description} -body {} -result {}}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let cards = result.original_symbol_declarations().collect::<Vec<_>>();
    assert_eq!(cards.len(), 1);
    let scope = result
        .global_scope
        .children
        .iter()
        .find(|scope| scope.kind == super::super::types::ScopeKind::Namespace)
        .unwrap();
    assert!(cards[0].original_namespace().is_some());
    assert_eq!(cards[0].original_namespace(), scope.naming_scope.as_ref());
    assert!(!cards[0].original_namespace().unwrap().is_root());
    assert!(cards[0].matches_source(
        &tcl_lexer::SourceImage::document(source),
        result.body_lexer_config.unwrap()
    ));
    assert!(!cards[0].matches_source(
        &tcl_lexer::SourceImage::document(&format!("# changed\n{source}")),
        result.body_lexer_config.unwrap()
    ));
}

#[test]
fn original_registry_symbol_advice_preserves_hosted_source_event_declarations() {
    // Implementation contract: naming.compiler.original-symbol-declaration-advice
    // docs/design/analysis/name-resolution-proofs/original-symbol-declaration-advice.md
    let result = Analyser::new().analyse("when HTTP_REQUEST {}", "f5-irules");
    assert!(!result.allows_lexical_declaration_advice());
    assert!(result.has_original_vendor_source_names());
    let original = result.original_vendor_symbol_declarations().next().unwrap();
    assert_eq!(
        original.name_input().literal_units(original.purpose()),
        Some(b"HTTP_REQUEST".as_slice())
    );
    assert!(original.metadata().original_name_input.is_none());
    assert_eq!(result.all_defined_symbols.len(), 1);
    assert_eq!(result.all_defined_symbols[0].name, "HTTP_REQUEST");
    assert_eq!(
        result.all_defined_symbols[0].kind,
        tcl_registry::DefinedSymbolKind::Event
    );
    assert!(result.all_defined_symbols[0].original_name_input.is_none());
    let invalid = Analyser::new().analyse("when HTTP_REQUEST set", "f5-irules");
    assert_eq!(
        invalid.all_defined_symbols,
        [] as [crate::analyser::types::DefinedSymbol; 0]
    );
    let shadowed =
        Analyser::new().analyse("proc when {args} {}; when HTTP_REQUEST {}", "f5-irules");
    assert_eq!(shadowed.original_vendor_symbol_declarations().count(), 0);
    assert_eq!(
        shadowed.all_defined_symbols,
        [] as [crate::analyser::types::DefinedSymbol; 0]
    );
    let uncertain = Analyser::new().analyse("unknown_mutator; when HTTP_REQUEST {}", "f5-irules");
    assert_eq!(
        uncertain.original_vendor_symbol_declarations().count(),
        1,
        "unknown applicability retains a source card without a runtime claim"
    );
}
#[test]
fn original_formal_counts_survive_procedure_and_signature_records() {
    // naming.procedure.original-formal-count-shape
    // docs/design/analysis/name-resolution-proofs/procedure-original-formal-count-shape.md
    use crate::signature_scan::formal_count::SourceFormalCountOrigin;
    let cases: &[(&str, &[usize], &[usize])] = &[
        ("", &[0], &[0]),
        ("a {b 2} c", &[3], &[2, 3]),
        ("a args b", &[3], &[2, 3, 4, 5]),
        ("a {args tail} b", &[3], &[2, 3, 4, 5]),
        ("a args", &[1, 2, 3, 4, 5], &[1, 2, 3, 4, 5]),
        ("a {b 2} args {c 3}", &[3, 4], &[1, 2, 3, 4, 5]),
        ("{a 1} {b 2}", &[0, 1, 2], &[0, 1, 2]),
    ];
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for &(formals, c_counts, jim_counts) in cases {
            let source = format!("proc target {{{formals}}} {{return ok}}");
            let analysis = Analyser::new().analyse(&source, profile);
            let proc = analysis.all_procs.get("::target").expect(profile);
            let scan = crate::signature_scan::extract_signatures(
                &source,
                analysis.resolved_registry().unwrap(),
            );
            let signature = scan.procs.get("::target").expect(profile);
            let expected = if profile == "jim" {
                jim_counts
            } else {
                c_counts
            };
            assert_eq!(
                proc.formal_count_projection().origin(),
                SourceFormalCountOrigin::OriginalSource
            );
            assert_eq!(
                signature.formal_count_projection(),
                proc.formal_count_projection()
            );
            for count in 0..=5 {
                let accepts = |arity: tcl_registry::Arity| {
                    usize::from(arity.min) <= count
                        && (arity.is_unlimited() || count <= usize::from(arity.max))
                };
                assert_eq!(
                    accepts(proc.arity()),
                    expected.contains(&count),
                    "{profile} {formals:?} argc={count}"
                );
                assert_eq!(
                    accepts(signature.arity()),
                    expected.contains(&count),
                    "scanned {profile} {formals:?} argc={count}"
                );
            }
        }
    }
    // An extra grouping level is a legal single formal named "a b c";
    // it cannot stand in for a malformed three-field parameter.
    let legal = Analyser::new().analyse("proc target {{{a b c}}} {return ok}", "jim");
    assert_eq!(
        legal.all_procs["::target"]
            .formal_count_projection()
            .origin(),
        SourceFormalCountOrigin::OriginalSource
    );
    assert_eq!(
        legal.all_procs["::target"].arity(),
        tcl_registry::Arity::exact(1)
    );
    for source in [
        "proc target $formals {return ok}",
        // Three fields in one formal, matching the native malformed row.
        "proc target {{a b c}} {return ok}",
    ] {
        let analysis = Analyser::new().analyse(source, "jim");
        let proc = analysis.all_procs.get("::target").unwrap();
        assert_eq!(
            proc.formal_count_projection().origin(),
            SourceFormalCountOrigin::Unknown
        );
        assert_eq!(proc.arity(), tcl_registry::Arity::any());
    }
}
