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

fn identity(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> OriginalDeclarationIdentity {
    let class = analysis.original_class_declarations().next().unwrap();
    let method = class
        .metadata()
        .original_members
        .declarations()
        .next()
        .unwrap();
    OriginalDeclarationIdentity::for_method("file:///owner.tcl", source, analysis, class, method)
        .unwrap()
}

fn document<'a>(
    source: &'a str,
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
) -> OriginalDeclarationDocument<'a> {
    OriginalDeclarationDocument {
        uri: "file:///owner.tcl",
        source,
        analysis,
    }
}

// Implementation contract: naming.editor.original-instance-member-rename-plans
// docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
#[test]
fn original_member_rename_keeps_actual_selectors_and_opaque_declarations_after_ui_clear() {
    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for name in ["pick", r"m\uD800"] {
            let source = format!(
                "oo::class create C {{method {name} {{}} {{return body}}}}\nC create object\nobject {name}\n"
            );
            let mut analysis = Analyser::new().analyse(&source, dialect);
            analysis.all_classes.clear();
            analysis.global_scope.classes.clear();
            analysis.instance_classes.clear();
            analysis.created_instance_commands.clear();
            let identity = identity(&source, &analysis);
            let doc = document(&source, &analysis);
            let edits = original_member_rename_edits(&[doc], &identity, "changed")
                .unwrap_or_else(|refusal| panic!("{dialect}: {name}: {refusal:?}"));
            assert_eq!(edits.len(), 2, "{dialect}: {name}");
            let mut changed = source.clone();
            for edit in edits.iter().rev() {
                changed.replace_range(edit.span.as_range(), &edit.new_text);
            }
            assert_eq!(
                changed,
                "oo::class create C {method changed {} {return body}}\nC create object\nobject changed\n"
            );
            let selector = u32::try_from(source.rfind(name).unwrap()).unwrap();
            let span = original_member_prepare_span(doc, &identity, selector).unwrap();
            assert_eq!(source.get(span.as_range()), Some(name));
            assert!(analysis.all_classes.is_empty());
        }
    }
}

/// tclsh 8.6 and 9.0: `oo::class create A {method Foo {} {}; export Foo}`
/// lists `Foo` among the exported methods; the export follows the rename.
/// An `export Bar` already present would export the renamed member.
#[test]
fn original_member_rename_rewrites_the_visibility_words_naming_the_member() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let source = "oo::class create A {\n    method Foo {} { return foo }\n    export Foo\n}\nset a [A new]\nputs [$a Foo]\n";
        let analysis = Analyser::new().analyse(source, dialect);
        let target = identity(source, &analysis);
        let edits = original_member_rename_edits(&[document(source, &analysis)], &target, "Bar")
            .unwrap_or_else(|refusal| panic!("{dialect}: {refusal:?}"));
        let mut changed = source.to_owned();
        for edit in edits.iter().rev() {
            changed.replace_range(edit.span.as_range(), &edit.new_text);
        }
        assert_eq!(
            changed,
            "oo::class create A {\n    method Bar {} { return foo }\n    export Bar\n}\nset a [A new]\nputs [$a Bar]\n",
            "{dialect}"
        );
        let occupied = "oo::class create A {\n    method Foo {} {}\n    export Bar\n}\n";
        let analysis = Analyser::new().analyse(occupied, dialect);
        let target = identity(occupied, &analysis);
        assert_eq!(
            original_member_rename_edits(&[document(occupied, &analysis)], &target, "Bar"),
            Err(OriginalMemberRenameRefusal::ChangesVisibility),
            "{dialect}"
        );
    }
}

#[test]
fn original_member_body_coverage_uses_whole_words_and_formed_content() {
    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (body, expected) in [
            ("{return body}", Some("return body")),
            ("\"return body\"", Some("return body")),
            ("\"return\\ body\"", None),
            ("{return {λ 😀}}", Some("return {λ 😀}")),
            ("\"return {λ 😀}\"", Some("return {λ 😀}")),
            ("{return {\r\nλ 😀\r\n}}", Some("return {\r\nλ 😀\r\n}")),
        ] {
            let source = format!("oo::class create C {{method pick {{}} {body}}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let class = analysis.original_class_declarations().next().unwrap();
            let method = class
                .metadata()
                .original_members
                .declarations()
                .next()
                .unwrap();
            let word = method.body_word().unwrap();
            let result = original_method_body_region(method, std::slice::from_ref(word));
            if let Some(expected) = expected {
                let content = result.unwrap();
                assert_eq!(
                    source.get(content.as_range()),
                    Some(expected),
                    "{dialect}/{body}"
                );
                assert!(
                    content.start() > word.span().start(),
                    "wrappers stay outside the script"
                );
                assert_eq!(
                    original_method_body_region(method, &[]),
                    Err(OriginalMemberRenameRefusal::IncompleteCoverage),
                    "a declaration body cannot substitute for the current worker's word"
                );
            } else {
                assert_eq!(
                    result,
                    Err(OriginalMemberRenameRefusal::IncompleteCoverage),
                    "transformed value has no affine original script region"
                );
            }
        }
    }
}

// Implementation contract: naming.editor.original-instance-member-rename-plans
// docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
#[test]
fn original_member_rename_checks_builtin_roster_visibility_and_source_family_closure() {
    // Implementation contract: naming.editor.original-instance-member-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-instance-member-rename-plans.md
    let source =
        "oo::class create C {method pick {} {return body}}\nC create object\nobject pick\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let target = identity(source, &analysis);
    let doc = document(source, &analysis);
    assert_eq!(
        original_member_rename_edits(&[doc], &target, "destroy"),
        Err(OriginalMemberRenameRefusal::Collision)
    );
    assert_eq!(
        original_member_rename_edits(&[doc], &target, "Upper"),
        Err(OriginalMemberRenameRefusal::ChangesVisibility)
    );
    assert_eq!(
        original_member_rename_edits(&[doc, doc], &target, "changed"),
        Err(OriginalMemberRenameRefusal::AmbiguousOwner)
    );
    let displaced = format!("# displaced\n{source}");
    assert_eq!(
        original_member_rename_edits(&[document(&displaced, &analysis)], &target, "changed"),
        Err(OriginalMemberRenameRefusal::StaleSource)
    );
    for source in [
        "oo::class create C {method pick {} {}; method pick {} {return body}}",
        "oo::class create C {method pick {} {return body}; foreach name {other} {method $name {} {}}}",
        "oo::class create C {method pick {} {return body}}; C create object; after idle {object pick}",
        // `B`'s own instances reach no `pick`, so its `my pick` selects
        // the family only on `D` receivers.
        "oo::class create C {method pick {} {return body}}; oo::class create B {method run {} {my pick}}; oo::class create D {superclass B C}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let target = identity(source, &analysis);
        assert!(
            original_member_rename_edits(&[document(source, &analysis)], &target, "changed")
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn retained_member_rename_checks_completed_selector_values_in_callbacks() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for name in ["pick", "m;n", "m n", "m(n)"] {
            let base = format!(
                "oo::class create C {{method {{{name}}} {{}} {{return body}}}}\nC create object\nobject {{{name}}}\n"
            );
            for (callback, safe) in [
                (String::new(), true),
                ("set callback [list object other]".to_owned(), true),
                (format!("set callback [list object {{{name}}}]"), false),
                (format!("set callback {{object {{{name}}}}}"), false),
                ("set callback [list object changed]".to_owned(), false),
            ] {
                let source = format!("{base}{callback}\n");
                let analysis = Analyser::new().analyse(&source, dialect);
                let profile = analysis.resolved_profile().unwrap();
                let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
                let edits = crate::rename::rename_with_diagnosis(
                    &source, profile, 0, cursor, "changed", &analysis, None,
                );
                if safe {
                    assert_eq!(edits.unwrap().len(), 2, "{dialect} {name}: {callback}");
                } else {
                    assert!(edits.is_err(), "{dialect} {name}: {callback}");
                }
            }
        }
    }
}
