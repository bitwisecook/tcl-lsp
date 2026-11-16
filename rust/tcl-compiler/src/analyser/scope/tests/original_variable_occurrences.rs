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

//! The original source symbols kept for variable inputs, receivers, formals, aliases and stores.

use super::*;

#[test]
// Implementation contract: naming.variable.byte-cell-correspondence
// docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
fn qualified_variable_inputs_keep_document_units_and_producer_kind() {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "namespace eval N {}; set {::N::v\0tail} 1; puts ${::N::v\0tail}";
        let result = Analyser::new().analyse(source, dialect);
        let reference = result
            .qualified_var_refs
            .iter()
            .find(|reference| {
                matches!(
                    reference.original_name_input,
                    Some(SignatureSourceNameInput::OriginalVariableRoot(_))
                )
            })
            .expect("original substitution root is retained");
        let Some(SignatureSourceNameInput::OriginalVariableRoot(root)) =
            &reference.original_name_input
        else {
            unreachable!()
        };
        let expected = if dialect == "jimtcl" {
            b"::N::v\0tail".as_slice()
        } else {
            b"::N::v\xc0\x80tail".as_slice()
        };
        assert_eq!(
            root.bytes(),
            expected,
            "{dialect}: original Document channel"
        );
        assert_eq!(
            root.source_image().channel(),
            tcl_lexer::SourceChannel::Document
        );
        assert!(root.original_word().is_none());
        assert!(reference.original_namespace.is_some());
        assert!(
            result.qualified_var_refs.iter().any(|reference| {
                reference.original_name_input.as_ref().is_some_and(|input| {
                    !matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_))
                        && input.bytes() == expected
                })
            }),
            "{dialect}: name operand keeps its independently retained value producer"
        );
    }
}

#[test]
fn original_static_variable_words_survive_unavailable_execution_without_new_authority() {
    let source = r"unknown; set {v\uD800} 1; set v\uD801 2";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let mut analyser = Analyser::new();
        let _ = analyser.analyse(source, profile);
        let first = u32::try_from(source.find(r"{v\uD800}").unwrap()).unwrap();
        let second = u32::try_from(source.find(r"v\uD801").unwrap()).unwrap();
        let first = analyser
            .original_static_source_name_at_span(Span::new(first, first + 9))
            .unwrap();
        assert_eq!(first.name_input().bytes(), br"v\uD800");
        let second = analyser
            .original_static_source_name_at_span(Span::new(second, second + 7))
            .unwrap();
        assert_eq!(second.name_input().bytes(), b"v\xed\xa0\x81");
        assert_eq!(
            second.name_input().source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
        let previous = second.clone();
        let _ = analyser.analyse("set $computed 1", profile);
        assert!(
            analyser
                .original_static_source_name_at_span(previous.name_input().original_word().span())
                .is_none()
        );
    }
}

#[test]
fn original_variable_cursor_roots_keep_lexical_membership_and_full_source_currency() {
    let source =
        "set ns ::N; namespace exists $ns; namespace exists $missing; puts {${inert}}; # $comment";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        for (needle, expected) in [("$ns", b"ns".as_slice()), ("$missing", b"missing")] {
            let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
            let root = analysis
                .original_variable_root_in_source(&image, config, offset)
                .unwrap_or_else(|| panic!("{profile}: actual lexical {needle}"));
            assert_eq!(root.bytes(), expected);
            assert_eq!(root.source_image(), &image);
            assert!(
                analysis
                    .original_variable_root_in_source(
                        &tcl_lexer::SourceImage::document(&source.replace("::N", "::M")),
                        config,
                        offset,
                    )
                    .is_none()
            );
        }
        for needle in ["${inert}", "$comment"] {
            let offset = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
            assert!(
                analysis
                    .original_variable_root_in_source(&image, config, offset)
                    .is_none()
            );
        }
    }
}

#[test]
fn original_conditional_variable_receivers_keep_absolute_source_geometry_and_unknown_coverage() {
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    let source = r"namespace eval n\uD800 {}
namespace exists n\uD800
namespace eval N {}
set ::N::v\uD800 1
info exists ::N::v\uD800";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let declaration_offset = u32::try_from(source.find("set ::N::v").unwrap() + 4).unwrap();
        let read_offset = u32::try_from(source.rfind("::N::v").unwrap()).unwrap();
        let declaration = analysis
            .original_variable_symbol_in_source(&image, config, declaration_offset)
            .unwrap_or_else(|| panic!("{profile}: conditional absolute source declaration"));
        let reference = analysis
            .original_variable_symbol_in_source(&image, config, read_offset)
            .unwrap_or_else(|| panic!("{profile}: conditional absolute source reference"));
        assert!(declaration.is_declaration(), "{profile}: source write role");
        assert!(!reference.is_declaration(), "{profile}: source read role");
        assert_eq!(
            declaration.symbol(),
            reference.symbol(),
            "{profile}: exact source identity"
        );
        assert!(declaration.symbol().is_namespace());
        let advice = analysis
            .original_variable_write_advice
            .iter()
            .find(|advice| advice.span() == declaration.span())
            .unwrap_or_else(|| panic!("{profile}: original source write card"));
        let metadata = advice
            .conditional_metadata()
            .unwrap_or_else(|| panic!("{profile}: retained conditional applicability"));
        assert!(metadata.matches_source(&image, config));
        assert_ne!(metadata.obligations(), []);
        assert_eq!(
            declaration.original_name_input().bytes(),
            b"::N::v\xed\xa0\x80"
        );
        assert!(
            !analysis.original_variable_rename_is_complete(declaration.symbol()),
            "{profile}: unresolved command alternatives remain a coverage obligation"
        );
        assert!(
            analysis
                .original_variable_symbol_in_source(
                    &tcl_lexer::SourceImage::document(
                        &source.replace("namespace eval N", "namespace eval M")
                    ),
                    config,
                    declaration_offset,
                )
                .is_none(),
            "{profile}: complete source currency"
        );
    }
}

#[test]
fn original_conditional_variable_receivers_do_not_invent_bare_namespace_targets() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    let source = "set auto_index(unknown_command) {source lib.tcl}; unknown_command; namespace eval N {set bare 1; info exists bare}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        for offset in source
            .match_indices("bare")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
        {
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, config, offset)
                    .is_none(),
                "{profile}: a conditional namespace card cannot select ordinary variable storage"
            );
        }
    }
}

#[test]
fn original_write_name_advice_keeps_source_cards_and_authentic_local_frames() {
    // Implementation contract: naming.variable.conditional-write-name-advice
    // docs/design/analysis/name-resolution-proofs/conditional-write-name-advice.md
    let source = r"set v\uD800 1; set v\uD801 2; proc p {} {set local 3; set local; unset local; global shared}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let inputs: Vec<_> = analysis
            .original_variable_write_advice
            .iter()
            .map(|advice| advice.original_name_input().bytes())
            .collect();
        assert!(
            inputs.contains(&b"v\xed\xa0\x80".as_slice()),
            "{profile}: first opaque source card"
        );
        assert!(
            inputs.contains(&b"v\xed\xa0\x81".as_slice()),
            "{profile}: distinct opaque source card"
        );
        let local = analysis
            .original_variable_write_advice
            .iter()
            .find(|advice| advice.original_name_input().bytes() == b"local")
            .unwrap_or_else(|| panic!("{profile}: own local write card"));
        assert!(
            matches!(
                local.original_frame().map(|frame| frame.frame().layout()),
                Some(crate::var_resolve::VariableExecutionFrame::Procedure { .. })
            ),
            "{profile}: original procedure owner"
        );
        assert_eq!(
            inputs.iter().filter(|&&input| input == b"local").count(),
            1,
            "reads and unsets are not source write declarations"
        );
        assert!(
            !inputs.contains(&b"shared".as_slice()),
            "alias source names remain separate"
        );
        assert_eq!(
            local.receiver_form(),
            tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined
        );
    }
}

#[test]
fn caught_caller_rebinding_cannot_retain_the_original_formal_symbol() {
    // Native proof: naming.variable.catch-result-alias-continuity
    // docs/design/analysis/name-resolution-proofs/catch-result-alias-continuity.md
    // Native ASCII controls independently show catch's result reaching the
    // rebound root slot. These assertions test source identity withdrawal.
    let source = "proc mutate {} {uplevel 1 {unset name; upvar #0 ::destination name}; error BOOM}\nproc probe {name} {catch {mutate} name; return $name}";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let formal = u32::try_from(source.find("{name}").unwrap() + 1).unwrap();
        let original = analysis
            .original_variable_symbol_at_offset(formal)
            .unwrap_or_else(|| panic!("{profile}: original formal declaration"));
        let read = u32::try_from(source.rfind("$name").unwrap() + 1).unwrap();
        assert!(
            analysis
                .original_variable_symbol_at_offset(read)
                .is_none_or(|current| current.symbol() != original.symbol()),
            "a caught caller mutation cannot donate the original formal cell"
        );
        assert!(!analysis.original_variable_rename_is_complete(original.symbol()));
    }
}

#[test]
fn logical_lambda_formals_join_body_source_symbols_without_native_or_edit_authority() {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    for source in [
        "apply {{first {second DEFAULT} args} {set first NEXT; return $first}} ONE",
        "namespace eval N {apply {{first {second DEFAULT} args} {return $first}} ONE}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        for name in ["first", "second", "args"] {
            let offset = u32::try_from(source.find(name).unwrap()).unwrap();
            let declaration = analysis
                .original_variable_symbol_in_source(&image, config, offset)
                .unwrap_or_else(|| panic!("actual source formal {name}: {source}"));
            assert!(declaration.is_declaration());
            assert_eq!(
                declaration.receiver(),
                OriginalVariableSymbolReceiver::FormalDeclaration
            );
            assert!(matches!(
                declaration.original_name_input(),
                SignatureSourceNameInput::OriginalLogicalListField(_)
            ));
            assert!(matches!(
                declaration.symbol().slot(),
                SignatureSourceVariableSlot::Local { .. }
            ));
            assert!(declaration.rename_span().is_none());
            assert!(declaration.renamed_input(b"renamed").is_none());
            assert!(!analysis.original_variable_rename_is_complete(declaration.symbol()));
            let mut wrong = config;
            wrong.strict_quoting = !wrong.strict_quoting;
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, wrong, offset)
                    .is_none()
            );
            assert!(
                analysis
                    .original_variable_symbol_in_source(
                        &tcl_lexer::SourceImage::document(&source.replace("ONE", "TWO")),
                        config,
                        offset,
                    )
                    .is_none()
            );
            if name == "first" {
                let read = analysis
                    .original_variable_symbol_in_source(
                        &image,
                        config,
                        u32::try_from(source.rfind("$first").unwrap() + 1).unwrap(),
                    )
                    .expect("actual body source read");
                assert_eq!(declaration.symbol(), read.symbol());
            }
        }
    }
}

#[test]
fn logical_lambda_formal_source_symbols_refuse_invalid_layouts_and_keep_activations_distinct() {
    for source in [
        "apply {{first} {return $first}}",
        "apply {{first first} {return $first}} ONE TWO",
        r"apply {{{fi\u0072st}} {return}} ONE",
        "proc apply args {}; apply {{first} {return $first}} ONE",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(
            source
                .find("first")
                .unwrap_or_else(|| source.find("fi\\u0072st").unwrap()),
        )
        .unwrap();
        assert!(
            analysis
                .original_variable_symbol_at_offset(offset)
                .is_none(),
            "{source}"
        );
    }
    let source = "apply {{item} {return $item}} ONE; apply {{item} {return $item}} TWO";
    let analysis = Analyser::new().analyse(source, "tcl");
    let offsets = source
        .match_indices("item")
        .map(|(offset, _)| u32::try_from(offset).unwrap())
        .collect::<Vec<_>>();
    let first = analysis
        .original_variable_symbol_at_offset(offsets[0])
        .unwrap();
    let first_read = analysis
        .original_variable_symbol_at_offset(offsets[1])
        .unwrap();
    let second = analysis
        .original_variable_symbol_at_offset(offsets[2])
        .unwrap();
    let second_read = analysis
        .original_variable_symbol_at_offset(offsets[3])
        .unwrap();
    assert_eq!(first.symbol(), first_read.symbol());
    assert_eq!(second.symbol(), second_read.symbol());
    assert_ne!(first.symbol(), second.symbol());
    assert!(!analysis.original_variable_rename_is_complete(first.symbol()));
    assert!(!analysis.original_variable_rename_is_complete(second.symbol()));
}

#[test]
fn original_formal_variable_symbols_keep_list_children_and_binding_topology() {
    use crate::signature_scan::variable_symbol::{
        OriginalVariableSymbolReceiver, SignatureSourceVariableSlot,
    };
    let source =
        r"proc p {v\uD800 {d\uD800 DEFAULT} args} {list [set v\uD800] [set d\uD800] [set args]}";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        for needle in [r"v\uD800", r"d\uD800", "args"] {
            let offsets: Vec<_> = source
                .match_indices(needle)
                .map(|(offset, _)| u32::try_from(offset).unwrap())
                .collect();
            let declaration = analysis
                .original_variable_symbol_at_offset(offsets[0])
                .unwrap_or_else(|| panic!("{profile}: formal {needle}"));
            let read = analysis
                .original_variable_symbol_at_offset(offsets[1])
                .unwrap_or_else(|| panic!("{profile}: read {needle}"));
            assert_eq!(declaration.symbol(), read.symbol(), "{profile}: {needle}");
            assert_eq!(
                declaration.receiver(),
                OriginalVariableSymbolReceiver::FormalDeclaration
            );
            assert!(declaration.is_declaration());
            assert!(matches!(
                declaration.symbol().slot(),
                SignatureSourceVariableSlot::Local { .. }
            ));
            let container = declaration
                .original_name_input()
                .original_static_list_container()
                .expect("authentic ParamList ancestry");
            assert_ne!(container.ordinals(), [] as [usize; 0]);
            assert!(
                analysis.original_variable_rename_is_complete(declaration.symbol()),
                "{profile}: complete original naming occurrences"
            );
            if needle == "args" {
                assert!(
                    declaration.renamed_input(b"new").is_none(),
                    "variadic binding cannot silently become a required formal"
                );
            } else {
                assert_eq!(declaration.renamed_input(b"new"), Some(b"new".to_vec()));
                assert!(
                    declaration.renamed_input(b"args").is_none(),
                    "required/default binding topology cannot change"
                );
            }
        }
    }
    let source = "proc p {a a {b DEFAULT}} {set a; set b}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let first = analysis
        .original_variable_symbol_at_offset(8)
        .expect("first duplicate formal");
    let second = analysis
        .original_variable_symbol_at_offset(10)
        .expect("second duplicate formal");
    assert_eq!(first.symbol(), second.symbol());
    assert_ne!(
        first
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals(),
        second
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals()
    );
    assert_eq!(first.renamed_input(b"new"), Some(b"new".to_vec()));
    assert_eq!(second.renamed_input(b"new"), Some(b"new".to_vec()));
    let source = "proc p {{args rest}} {set rest}";
    let analysis = Analyser::new().analyse(source, "jimtcl");
    let declaration = analysis
        .original_variable_symbol_at_offset(14)
        .expect("Jim renamed variadic local field");
    let read = analysis
        .original_variable_symbol_at_offset(26)
        .expect("same genuine variadic local");
    assert_eq!(declaration.symbol(), read.symbol());
    assert_eq!(
        declaration
            .original_name_input()
            .original_static_list_container()
            .unwrap()
            .ordinals(),
        &[0, 1]
    );
    assert_eq!(declaration.renamed_input(b"new"), Some(b"new".to_vec()));
}

#[test]
fn original_variable_alias_reads_keep_derived_and_independent_local_spellings() {
    let source = r"set ::g\uD800 VALUE; proc p {} {global g\uD800; set g\uD800; set $name 1}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let offsets: Vec<_> = source
            .match_indices(r"g\uD800")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
            .collect();
        let declaration = analysis
            .original_variable_symbol_at_offset(offsets[1])
            .expect("authentic selected global declaration");
        let read = analysis
            .original_variable_symbol_at_offset(offsets[2])
            .unwrap_or_else(|| panic!("{profile}: original alias read"));
        assert_eq!(declaration.symbol(), read.symbol());
        assert!(read.rename_changes_spelling());
        assert_eq!(read.renamed_input(b"new"), Some(b"new".to_vec()));
        assert!(
            !analysis.original_variable_rename_is_complete(declaration.symbol()),
            "computed variable names are rename coverage obligations"
        );
        let complete_source = source.replace("; set $name 1", "");
        let complete = Analyser::new().analyse(&complete_source, profile);
        let declaration = complete
            .original_variable_symbol_at_offset(offsets[1])
            .unwrap();
        assert!(
            complete.original_variable_rename_is_complete(declaration.symbol()),
            "{profile}: known alias producer and every original read"
        );
    }
    let source = "set ::x VALUE; proc p {} {upvar #0 x local; set local; return $local}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let target = analysis
            .original_variable_symbol_at_offset(35)
            .expect("genuine #0 target");
        for offset in [48, 63] {
            let read = analysis
                .original_variable_symbol_at_offset(offset)
                .unwrap_or_else(|| panic!("{profile}: original independent alias read"));
            assert_eq!(read.symbol(), target.symbol());
            assert!(!read.rename_changes_spelling());
            assert_eq!(read.renamed_input(b"new"), Some(b"local".to_vec()));
        }
        assert!(
            analysis.original_variable_rename_is_complete(target.symbol()),
            "{profile}: original #0 local does not need spelling edits"
        );
    }
    let source = "set ::x VALUE; proc p {} {upvar 1 other local; set local}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let target = analysis.original_variable_symbol_at_offset(6).unwrap();
    assert!(
        !analysis.original_variable_rename_is_complete(target.symbol()),
        "missing authentic foreign frame target blocks coverage"
    );
}

#[test]
fn original_local_variable_symbols_keep_opaque_names_and_genuine_body_frames() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let source = r"proc p {} {set v\uD800 1; set v\uD801 2; set v\uD800}; proc q {} {set v\uD800 3; set v\uD800}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let offsets: Vec<_> = source
            .match_indices(r"v\uD800")
            .map(|(offset, _)| u32::try_from(offset).unwrap())
            .collect();
        let sites: Vec<_> = offsets
            .iter()
            .map(|&offset| {
                analysis
                    .original_variable_symbol_at_offset(offset)
                    .unwrap_or_else(|| panic!("{profile}: original local at {offset}"))
            })
            .collect();
        assert_eq!(
            sites[0].symbol(),
            sites[1].symbol(),
            "{profile}: one original body"
        );
        assert_eq!(
            sites[2].symbol(),
            sites[3].symbol(),
            "{profile}: second original body"
        );
        assert_ne!(
            sites[0].symbol(),
            sites[2].symbol(),
            "{profile}: distinct frame recipes"
        );
        let SignatureSourceVariableSlot::Local { simple, frame } = sites[0].symbol().slot() else {
            panic!("ordinary procedure local became namespace storage");
        };
        assert_eq!(simple.as_bytes(), b"v\xed\xa0\x80");
        assert!(!sites[0].symbol().is_namespace());
        assert_eq!(frame.lexer_config(), analysis.body_lexer_config.unwrap());
        assert!(sites[0].is_declaration());
        assert!(!sites[1].is_declaration());
        let other = analysis
            .original_variable_symbol_at_offset(
                u32::try_from(source.find(r"v\uD801").unwrap()).unwrap(),
            )
            .unwrap();
        assert_ne!(sites[0].symbol(), other.symbol());
        let proposed = sites[0].symbol().renamed(b"new").unwrap();
        let SignatureSourceVariableSlot::Local {
            frame: proposed_frame,
            simple,
        } = proposed.slot()
        else {
            unreachable!()
        };
        assert_eq!(proposed_frame, frame);
        assert_eq!(simple.as_bytes(), b"new");
        assert_eq!(sites[0].renamed_input(b"new"), Some(b"new".to_vec()));
        assert!(sites[0].renamed_input(b"other::new").is_none());
    }
}

// Native proof: naming.variable.namespace-store-frame-and-fallback
// docs/design/analysis/name-resolution-proofs/namespace-store-frame-and-fallback.md
#[test]
fn original_namespace_store_symbols_use_the_selected_activation_and_fallback() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "set ::x ROOT; namespace eval N {set x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.rfind("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: actual namespace set owner absent"));
        match occurrence.symbol().slot() {
            SignatureSourceVariableSlot::C { namespace, simple } => {
                assert_eq!(simple.as_bytes(), b"x");
                let expected = if matches!(profile, "tcl9.0" | "tcl9.1") {
                    tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
                } else {
                    tcl_core_types::ByteNamespacePath::root()
                };
                assert_eq!(*namespace, expected, "{profile}: existing root selection");
            }
            SignatureSourceVariableSlot::Local { frame, simple } => {
                assert_eq!(profile, "jimtcl");
                assert_eq!(simple.as_bytes(), b"x");
                assert!(matches!(
                    frame.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                ));
            }
            SignatureSourceVariableSlot::Jim(_) => {
                panic!("bare Jim namespace set became global storage")
            }
        }
        let source = "namespace eval Fresh {set x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.find("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: fresh namespace set owner absent"));
        if profile == "jimtcl" {
            assert!(!occurrence.symbol().is_namespace());
        } else {
            let SignatureSourceVariableSlot::C { namespace, .. } = occurrence.symbol().slot()
            else {
                panic!("C fresh namespace store lost its namespace owner");
            };
            assert_eq!(
                *namespace,
                tcl_core_types::ByteNamespacePath::from_segments([b"Fresh".as_slice()])
            );
        }
        let source = "set ::x ROOT; namespace eval Explicit {variable x VALUE}";
        let analysis = Analyser::new().analyse(source, profile);
        let offset = u32::try_from(source.find("x VALUE").unwrap()).unwrap();
        let occurrence = analysis
            .original_variable_symbol_at_offset(offset)
            .unwrap_or_else(|| panic!("{profile}: explicit namespace variable absent"));
        match occurrence.symbol().slot() {
            SignatureSourceVariableSlot::C { namespace, simple } => {
                assert_eq!(
                    *namespace,
                    tcl_core_types::ByteNamespacePath::from_segments([b"Explicit".as_slice()])
                );
                assert_eq!(simple.as_bytes(), b"x");
            }
            SignatureSourceVariableSlot::Jim(key) => assert_eq!(key.as_bytes(), b"Explicit::x"),
            SignatureSourceVariableSlot::Local { .. } => {
                panic!("explicit namespace variable became local")
            }
        }
    }
}

#[test]
fn original_variable_symbols_select_global_namespace_and_remote_frame_purposes() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let source = r"namespace eval T {}; namespace eval N {proc p {} {global g\uD800; variable v\uD800; namespace upvar ::T t\uD800 local; upvar #0 u\uD800 rootAlias; upvar 1 remote callerAlias}}";
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        let analysis = Analyser::new().analyse(source, profile);
        let at = |needle: &str| {
            analysis.original_variable_symbol_at_offset(
                u32::try_from(source.find(needle).unwrap()).unwrap(),
            )
        };
        for (needle, namespace, simple) in [
            (r"g\uD800", b"".as_slice(), b"g\xed\xa0\x80".as_slice()),
            (r"v\uD800", b"N".as_slice(), b"v\xed\xa0\x80".as_slice()),
            (r"t\uD800", b"T".as_slice(), b"t\xed\xa0\x80".as_slice()),
            (r"u\uD800", b"".as_slice(), b"u\xed\xa0\x80".as_slice()),
        ] {
            let site = at(needle).unwrap_or_else(|| {
                panic!(
                    "{profile}: {needle} {:#?}",
                    analysis.original_variable_symbols
                )
            });
            assert!(site.is_declaration(), "{profile}: {needle}");
            match site.symbol().slot() {
                SignatureSourceVariableSlot::C {
                    namespace: path,
                    simple: key,
                } => {
                    let expected = if namespace.is_empty() {
                        tcl_core_types::ByteNamespacePath::root()
                    } else {
                        tcl_core_types::ByteNamespacePath::from_segments([namespace])
                    };
                    assert_eq!(*path, expected, "{profile}: {needle}");
                    assert_eq!(key.as_bytes(), simple);
                }
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("namespace receiver selected a local frame")
                }
                SignatureSourceVariableSlot::Jim(key) => {
                    let mut expected = namespace.to_vec();
                    if !expected.is_empty() {
                        expected.extend_from_slice(b"::");
                    }
                    expected.extend_from_slice(simple);
                    assert_eq!(key.as_bytes(), expected, "{profile}: {needle}");
                }
            }
        }
        assert!(
            at("remote").is_none(),
            "a foreign caller frame is not the caller namespace"
        );
        assert!(
            at("callerAlias").is_none(),
            "local alias spelling is a separate symbol"
        );
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis
            .body_lexer_config
            .expect("actual retained body grammar");
        assert!(analysis.matches_original_source_image(&image, config));
        assert!(!analysis.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&format!("{source}; list changed")),
            config
        ));
        assert!(!analysis.matches_original_source_image(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            config
        ));
    }
}

// Native proof: naming.array-source.combined-set-receiver
// docs/design/analysis/name-resolution-proofs/array-source-combined-set-receiver.md
// Native proof: naming.array-source.scalar-element-storage
// docs/design/analysis/name-resolution-proofs/array-source-scalar-element-storage.md
// Native proof: naming.array-source.combined-read-enumeration
// docs/design/analysis/name-resolution-proofs/array-source-combined-read-enumeration.md
// Native proof: naming.array-source.combined-unset
// docs/design/analysis/name-resolution-proofs/array-source-combined-unset.md
#[test]
fn original_array_receiver_symbols_keep_combined_names_and_guest_error_independent() {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    // Each receiver has its own original program point. In C Tcl an array
    // setter on a combined element name errors before later script words.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (command, declaration) in [
            ("array set ::N::a(b) {k V}", false),
            ("set ::N::a(b) VALUE", true),
            ("array get ::N::a(b)", false),
            ("array names ::N::a(b)", false),
            ("array unset ::N::a(b)", false),
        ] {
            let source = format!("namespace eval N {{}}; {command}");
            let analysis = Analyser::new().analyse(&source, profile);
            let offset = u32::try_from(source.find("::N::a(b)").unwrap()).unwrap();
            let site = analysis
                .original_variable_symbol_at_offset(offset)
                .unwrap_or_else(|| panic!("{profile}: original receiver for {command}"));
            assert_eq!(site.is_declaration(), declaration, "{profile}: {command}");
            match site.symbol().slot() {
                SignatureSourceVariableSlot::C { namespace, simple } => {
                    assert_eq!(
                        *namespace,
                        tcl_core_types::ByteNamespacePath::from_segments([b"N".as_slice()])
                    );
                    assert_eq!(simple.as_bytes(), b"a");
                }
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("absolute receiver selected a local frame")
                }
                SignatureSourceVariableSlot::Jim(key) => assert_eq!(key.as_bytes(), b"N::a"),
            }
        }
    }
}
