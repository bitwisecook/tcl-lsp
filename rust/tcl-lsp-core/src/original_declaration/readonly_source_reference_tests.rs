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

#[test]
fn readonly_member_reports_keep_source_spans_out_of_edit_inventories() {
    // naming.editor.original-method-candidates
    // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
    let source = "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let class = analysis.original_class_declarations().next().unwrap();
    let method = class
        .metadata()
        .original_members
        .declarations()
        .find(|method| method.original_name_input().bytes() == b"ping")
        .unwrap();
    let identity =
        OriginalDeclarationIdentity::for_method("", source, &analysis, class, method).unwrap();
    let old = reference_spans(&identity, source, &analysis, true);
    let report = readonly_reference_report(&identity, source, &analysis, true).unwrap();
    assert_eq!(
        report.coverage(),
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    );
    assert_eq!(report.spans().len(), 2);
    assert!(
        report
            .spans()
            .iter()
            .all(|span| &source[span.as_range()] == "ping")
    );
    assert!(
        old.is_empty(),
        "the pre-existing reference kernel does not acquire the source facet"
    );
    let (_, strong) = crate::references::method_references_for_declaration(
        source,
        analysis.resolved_profile().unwrap(),
        &analysis,
        class.metadata(),
        method.metadata(),
        false,
    );
    assert!(
        strong.is_empty(),
        "source-only selectors cannot enter a transformation inventory"
    );
    assert!(
        crate::references::method_references_for_class(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            "::C",
            "ping",
            false,
        )
        .is_none()
    );
    assert_eq!(
        crate::rename::method_spans_in_document(
            source,
            analysis.resolved_profile().unwrap(),
            &analysis,
            "::C",
            "ping",
            false,
        ),
        [] as [tcl_lexer::Span; 0]
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(readonly_reference_report(&identity, source, &missing, true).is_none());
}

fn member_report(source: &str, dialect: &str, name: &[u8]) -> ReadonlyReferenceReport {
    let analysis = Analyser::new().analyse(source, dialect);
    let class = analysis
        .original_class_declarations()
        .find(|class| {
            class
                .metadata()
                .original_members
                .declarations()
                .any(|method| method.original_name_input().bytes() == name)
                || class
                    .metadata()
                    .original_properties
                    .declarations()
                    .any(|property| property.original_name_input().bytes() == name)
        })
        .unwrap();
    let identity = class
        .metadata()
        .original_members
        .declarations()
        .find(|method| method.original_name_input().bytes() == name)
        .map_or_else(
            || {
                let property = class
                    .metadata()
                    .original_properties
                    .declarations()
                    .find(|property| property.original_name_input().bytes() == name)
                    .unwrap();
                OriginalDeclarationIdentity::for_property("", source, &analysis, class, property)
            },
            |method| OriginalDeclarationIdentity::for_method("", source, &analysis, class, method),
        )
        .unwrap();
    readonly_reference_report(&identity, source, &analysis, true).unwrap()
}

#[test]
fn readonly_method_census_completes_only_proven_self_dispatch_shapes() {
    // naming.editor.original-method-candidates
    // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
    for (source, spans) in [
        ("oo::class create C {method ping {} {}}", 0),
        (
            "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}",
            2,
        ),
        (
            "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method body {} {my ping}}",
            1,
        ),
        // A literal object command call on a document instance reaches
        // the class's method (tclsh 8.6, 9.0, 9.1), and is counted.
        (
            "oo::class create C {method ping {} {}; method body {script cmd} {my ping; return}}; C create c; c ping",
            2,
        ),
    ] {
        let report = member_report(source, "tcl8.6", b"ping");
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus,
            "{source}"
        );
        assert_eq!(report.spans().len(), spans, "{source}");
    }
}

#[test]
fn readonly_member_census_fails_closed_for_deferred_and_unproven_dispatch() {
    // naming.editor.original-method-candidates
    // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
    let body = |tail: &str| {
        format!(
            "oo::class create C {{method ping {{}} {{}}; method body {{script cmd}} {{my ping; {tail}}}}}"
        )
    };
    let member = |member: &str| {
        format!(
            "oo::class create M {{}}; oo::class create C {{method ping {{}} {{}}; {member}; method body {{}} {{my ping}}}}"
        )
    };
    let mut cases = vec![
            // Retained callbacks: a stored script, a command prefix or a
            // wrapped prefix may dispatch later without a self-dispatch site.
            ("tcl8.6", body("after 0 [list my ping]")),
            ("tcl8.6", body("after idle {my ping}")),
            (
                "tcl8.6",
                body("trace add variable ::v write [list my ping]"),
            ),
            ("tcl8.6", body("set handler [namespace code {my ping}]")),
            ("tcl8.6", body("lsort -command {my ping} {b a}")),
            // A captured selector that leaves the document.
            ("tcl8.6", body("return [list my ping]")),
            ("tcl8.6", body("set captured {my ping}")),
            // Dynamic selectors, heads and scripts.
            ("tcl8.6", body("my $script")),
            ("tcl8.6", body("{*}$cmd")),
            ("tcl8.6", body("[self] ping")),
            ("tcl8.6", body("eval $script")),
            ("tcl8.6", body("uplevel 1 $script")),
            ("tcl8.6", body("mystery")),
            // Substitution templates evaluate escaped or computed selectors.
            ("tcl8.6", body(r"subst {[my p\x69ng]}")),
            ("tcl9.0", body(r"subst {[my p\x69ng]}")),
            ("tcl8.6", body("set s [string cat pi ng]; subst {[my $s]}")),
            ("tcl9.0", body("set s [string cat pi ng]; subst {[my $s]}")),
            // Method-chain and alias helpers, bare and namespaced.
            ("tcl8.6", body("next")),
            (
                "tcl8.6",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::next}}".to_owned(),
            ),
            (
                "tcl9.0",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::next}}".to_owned(),
            ),
            (
                "tcl8.6",
                "oo::class create Base {method ping {} {}}; oo::class create Child {superclass Base; method ping {} {::oo::Helpers::nextto Base}}".to_owned(),
            ),
            (
                "tcl9.0",
                "oo::class create C {method ping {} {}; constructor {} {::oo::Helpers::link [list set [string cat pi ng]]}; method body {} {set a 1}}".to_owned(),
            ),
            // Definition members that dispatch, name or reorder methods.
            ("tcl8.6", member("forward fw my ping")),
            ("tcl8.6", member("filter ping")),
            ("tcl8.6", member("export ping")),
            ("tcl8.6", member("mixin M")),
            // Members installed outside a selected definition body.
            (
                "tcl8.6",
                format!("{}; oo::define C forward fw my ping", body("return")),
            ),
        ];
    for tail in [
        "callback ping",
        "mymethod ping",
        "oo::Helpers::callback ping",
        "link ping",
    ] {
        cases.push(("tcl9.0", body(tail)));
    }
    for (dialect, source) in &cases {
        let report = member_report(source, dialect, b"ping");
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::Partial,
            "{dialect}: {source}"
        );
    }
    for source in [
        "oo::class create C {property size; method body {} {my size; my configure -size 3}}",
        "oo::class create C {property size; method body {} {my size}}; C create c; c configure -si 4",
    ] {
        let report = member_report(source, "tcl9.0", b"size");
        assert_eq!(
            report.coverage(),
            ReadonlyReferenceCoverage::Partial,
            "{source}"
        );
    }
}

#[test]
fn readonly_member_census_stays_partial_for_other_class_families() {
    // naming.editor.original-method-candidates
    // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
    // A snit method dispatches through `$self`, which has no typed
    // self-dispatch tape. itcl is not covered: nothing establishes that
    // `itcl::class` publishes original method declarations, and a
    // control that checks no identity proves nothing.
    let source = "snit::type C {method ping {} {}; method body {} {$self ping}}";
    for dialect in ["tcl8.5", "tcl8.6"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let mut checked = 0;
        for class in analysis.original_class_declarations() {
            for method in class.metadata().original_members.declarations() {
                let Some(identity) =
                    OriginalDeclarationIdentity::for_method("", source, &analysis, class, method)
                else {
                    continue;
                };
                checked += 1;
                assert_ne!(
                    readonly_reference_report(&identity, source, &analysis, true)
                        .map(|report| report.coverage()),
                    Some(ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus),
                    "{dialect}: {}",
                    identity.label()
                );
            }
        }
        assert!(
            checked > 0,
            "{dialect}: no snit method identity was checked"
        );
    }
}

#[test]
fn readonly_property_reports_promote_only_a_complete_typed_source_census() {
    // tclsh 9.0.4/9.1.0: property needs oo::configurable, and its read is
    // `my configure -size` (a bare `my size` is an unknown method).
    let source = "oo::configurable create C {property size; method body {} {my configure -size}}";
    let analysis = Analyser::new().analyse(source, "tcl9.0");
    let class = analysis.original_class_declarations().next().unwrap();
    let property = class
        .metadata()
        .original_properties
        .declarations()
        .find(|property| property.original_name_input().bytes() == b"size")
        .unwrap();
    let identity =
        OriginalDeclarationIdentity::for_property("", source, &analysis, class, property).unwrap();
    let report = readonly_reference_report(&identity, source, &analysis, true).unwrap();
    assert_eq!(
        report.coverage(),
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    );
    assert_eq!(report.spans().len(), 1);
    assert_eq!(&source[report.spans()[0].as_range()], "size");
    let unused_source = "oo::configurable create C {property size; method body {} {return ok}}";
    let unused_analysis = Analyser::new().analyse(unused_source, "tcl9.0");
    let unused_class = unused_analysis
        .original_class_declarations()
        .next()
        .unwrap();
    let unused_property = unused_class
        .metadata()
        .original_properties
        .declarations()
        .find(|property| property.original_name_input().bytes() == b"size")
        .unwrap();
    let unused_identity = OriginalDeclarationIdentity::for_property(
        "",
        unused_source,
        &unused_analysis,
        unused_class,
        unused_property,
    )
    .unwrap();
    let unused_report =
        readonly_reference_report(&unused_identity, unused_source, &unused_analysis, true).unwrap();
    assert_eq!(
        unused_report.coverage(),
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    );
    assert_eq!(unused_report.spans(), []);
    let dynamic_source = "oo::class create C {property size; method body {} {my $selector}}";
    let dynamic_analysis = Analyser::new().analyse(dynamic_source, "tcl9.0");
    let dynamic_class = dynamic_analysis
        .original_class_declarations()
        .next()
        .unwrap();
    let dynamic_property = dynamic_class
        .metadata()
        .original_properties
        .declarations()
        .find(|property| property.original_name_input().bytes() == b"size")
        .unwrap();
    let dynamic_identity = OriginalDeclarationIdentity::for_property(
        "",
        dynamic_source,
        &dynamic_analysis,
        dynamic_class,
        dynamic_property,
    )
    .unwrap();
    let dynamic_report =
        readonly_reference_report(&dynamic_identity, dynamic_source, &dynamic_analysis, true)
            .unwrap();
    assert_eq!(
        dynamic_report.coverage(),
        ReadonlyReferenceCoverage::Partial
    );
    assert_eq!(dynamic_report.spans(), []);
    let mut foreign = analysis.clone();
    foreign.resolved_input = Analyser::new()
        .analyse("puts foreign", "tcl9.0")
        .resolved_input;
    assert!(readonly_reference_report(&identity, source, &foreign, true).is_none());
}
