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

use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::{CheckPayload, Diagnostic, run_all_checks};
use crate::irules_checks::{IrulesCheckPayload, IrulesCheckWarning};
use crate::shimmer::{SharingWarning, ShimmerWarning, ThunkingWarning};
use crate::taint::TaintWarning;
use tcl_core_types::DiagCode;
use tcl_lexer::Span;
use tcl_registry::types::TclType;

/// End-to-end sources. The payload contract is release-invariant: Tcl
/// codes run under Tcl 9.0.4 and iRules codes under the F5 iRules dialect.
const FIXTURES: &[(DiagCode, &str, &str)] = &[
    (
        DiagCode::O100,
        "tcl9.0",
        "if {1} { set x 1 } else { set y 2 }",
    ),
    (DiagCode::O105, "tcl9.0", "llength {a b}\nllength {a b}"),
    (DiagCode::S100, "tcl9.0", "set x \"hello\"\nincr x"),
    (
        DiagCode::S101,
        "tcl9.0",
        "proc f {n} { set x 0\n while {$n} { set x \"s\"\n set x [list 1] }\n return $x }",
    ),
    (
        DiagCode::S102,
        "tcl9.0",
        "proc f {n} { set x 0\n while {$n} { set x \"s\"\n set x [list 1] }\n return $x }",
    ),
    (
        DiagCode::S103,
        "tcl9.0",
        "set a [lrepeat 1000 x]\nset b $a\nlappend b y\nputs [llength $a]",
    ),
    (
        DiagCode::S110,
        "tcl9.0",
        "proc f {} {\n  set b [binary format a* hello]\n  set u [string toupper $b]\n}",
    ),
    (DiagCode::T100, "tcl9.0", "set x [gets stdin]\neval $x"),
    (DiagCode::T101, "tcl9.0", "set x [gets stdin]\nputs $x"),
    (DiagCode::T102, "tcl9.0", "set x [read stdin]\nglob $x"),
    (
        DiagCode::T103,
        "tcl9.0",
        "set pat [read stdin]\nregexp $pat teststring",
    ),
    (
        DiagCode::T104,
        "tcl9.0",
        "set host [read stdin]\nsocket $host 80",
    ),
    (
        DiagCode::T105,
        "tcl9.0",
        "set x [gets stdin]\ninterp eval child $x",
    ),
    (
        DiagCode::W313,
        "tcl9.0",
        "set p [gets stdin]\nfile delete $p",
    ),
    (
        DiagCode::Irule1005,
        "f5-irules",
        "when HTTP_REQUEST_DATA { log local0. \"data\" }",
    ),
    (
        DiagCode::Irule1006,
        "f5-irules",
        "when HTTP_REQUEST { HTTP::payload }",
    ),
    (
        DiagCode::Irule1007,
        "f5-irules",
        "when HTTP_REQUEST { HTTP::collect 100 }",
    ),
    (
        DiagCode::Irule1008,
        "f5-irules",
        "when HTTP_REQUEST { HTTP::release }",
    ),
    (
        DiagCode::Irule1201,
        "f5-irules",
        "when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::uri /x }",
    ),
    (
        DiagCode::Irule1202,
        "f5-irules",
        "when HTTP_REQUEST { HTTP::respond 200 content x; HTTP::respond 404 content y }",
    ),
    (
        DiagCode::Irule3001,
        "f5-irules",
        "set u [HTTP::uri]\nHTTP::respond 200 content $u",
    ),
    (
        DiagCode::Irule3002,
        "f5-irules",
        "set val [HTTP::header Host]\nHTTP::header insert X-Foo $val",
    ),
    (
        DiagCode::Irule3003,
        "f5-irules",
        "set x [HTTP::uri]\nlog local0. $x",
    ),
    (
        DiagCode::Irule3004,
        "f5-irules",
        "set dest [HTTP::query]\nHTTP::redirect $dest",
    ),
    (DiagCode::Irule3101, "f5-irules", "HTTP::uri foo"),
    (
        DiagCode::Irule3103,
        "f5-irules",
        "if { [HTTP::uri] starts_with \"/api\" } { log local0. x }",
    ),
    (
        DiagCode::Irule5002,
        "f5-irules",
        "when CLIENT_ACCEPTED { drop }",
    ),
    (
        DiagCode::Irule5004,
        "f5-irules",
        "when DNS_REQUEST { DNS::return }",
    ),
];

fn diagnostics(dialect: &str, source: &str) -> Vec<Diagnostic> {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect)
        .with_interprocedural(registry, Some(profile));
    run_all_checks(&unit, registry, Some(profile))
}

#[test]
fn every_emitted_diagnostic_carries_its_code_family_payload() {
    for (code, dialect, source) in FIXTURES {
        let found = diagnostics(dialect, source);
        assert!(
            found.iter().any(|diagnostic| diagnostic.code == *code),
            "{code} not reached by {source:?}: {:?}",
            found.iter().map(|d| d.code).collect::<Vec<_>>()
        );
        for diagnostic in &found {
            assert!(
                CheckPayload::CODES.contains(&diagnostic.code)
                    && diagnostic.payload.accepts(diagnostic.code),
                "{} carries {:?}",
                diagnostic.code,
                diagnostic.payload
            );
        }
    }
}

fn taint(code: DiagCode) -> Diagnostic {
    Diagnostic::from_taint(&TaintWarning {
        span: Span::new(0, 1),
        variable: "v".to_owned(),
        sink_command: "sink".to_owned(),
        source_sink: None,
        code,
        message: String::new(),
        replacement: None,
        fixes: Vec::new(),
    })
}

fn irules(code: DiagCode, payload: IrulesCheckPayload) -> Diagnostic {
    Diagnostic::from_irules_check(&IrulesCheckWarning {
        span: Span::new(0, 1),
        code,
        message: String::new(),
        payload,
        replacement: None,
        fixes: Vec::new(),
    })
}

fn taint_conversions() -> Vec<Diagnostic> {
    [
        DiagCode::T100,
        DiagCode::T101,
        DiagCode::T102,
        DiagCode::T103,
        DiagCode::T104,
        DiagCode::T105,
        DiagCode::T106,
        DiagCode::W313,
        DiagCode::Irule3001,
        DiagCode::Irule3002,
        DiagCode::Irule3003,
        DiagCode::Irule3004,
        DiagCode::Irule3101,
        DiagCode::Irule3103,
    ]
    .map(taint)
    .to_vec()
}

fn irules_conversions() -> Vec<Diagnostic> {
    let protocol = || "HTTP".to_owned();
    let side = || "request".to_owned();
    vec![
        irules(
            DiagCode::Irule1005,
            IrulesCheckPayload::EventWithoutCollect {
                event: "HTTP_REQUEST_DATA".to_owned(),
                side: side(),
                collect_commands: vec!["HTTP::collect".to_owned()],
            },
        ),
        irules(
            DiagCode::Irule1006,
            IrulesCheckPayload::PayloadWithoutCollect {
                protocol: protocol(),
                side: side(),
            },
        ),
        irules(
            DiagCode::Irule1007,
            IrulesCheckPayload::CollectWithoutRelease {
                protocol: protocol(),
                side: side(),
            },
        ),
        irules(
            DiagCode::Irule1008,
            IrulesCheckPayload::ReleaseWithoutCollect {
                protocol: protocol(),
                side: side(),
            },
        ),
        irules(
            DiagCode::Irule1201,
            IrulesCheckPayload::ContextAfterResponse {
                command: "HTTP::uri".to_owned(),
                event: "HTTP_REQUEST".to_owned(),
            },
        ),
        irules(
            DiagCode::Irule1202,
            IrulesCheckPayload::RepeatedResponse {
                command: "HTTP::respond".to_owned(),
                event: "HTTP_REQUEST".to_owned(),
            },
        ),
        irules(
            DiagCode::Irule4002,
            IrulesCheckPayload::GenericStaticName {
                source_name: "static::debug".to_owned(),
                pattern_name: "debug".to_owned(),
            },
        ),
        irules(
            DiagCode::Irule4004,
            IrulesCheckPayload::HoistableAssignment {
                variable: "svc".to_owned(),
            },
        ),
        irules(
            DiagCode::Irule5002,
            IrulesCheckPayload::UnguardedDrop {
                command: "drop".to_owned(),
            },
        ),
        irules(DiagCode::Irule5004, IrulesCheckPayload::UnguardedDnsReturn),
    ]
}

/// Every contract code through its producer-to-envelope conversion,
/// including the codes no end-to-end source above reaches.
#[test]
fn every_contract_code_converts_to_its_family_payload() {
    let shimmer = |code| {
        Diagnostic::from_shimmer(&ShimmerWarning {
            span: Span::new(0, 1),
            variable: "v".to_owned(),
            from_type: TclType::String,
            to_type: TclType::Int,
            command: "incr".to_owned(),
            in_loop: code == DiagCode::S101,
            code,
            message: String::new(),
            related: Vec::new(),
        })
    };
    let redundant = |code| {
        Diagnostic::from_redundant(&crate::gvn::RedundantComputation {
            span: Span::new(4, 5),
            first_span: Span::new(0, 1),
            expression_text: "llength $l".to_owned(),
            code,
            message: String::new(),
        })
    };
    let mut converted = vec![
        Diagnostic::from_constant_branch(&crate::sccp::ConstantBranch {
            block: "b".to_owned(),
            condition: "1".to_owned(),
            value: true,
            taken_target: "t".to_owned(),
            not_taken_target: "n".to_owned(),
            span: None,
            kind: crate::sccp::BranchFactKind::Applied,
        }),
        redundant(DiagCode::O105),
        redundant(DiagCode::O106),
        shimmer(DiagCode::S100),
        shimmer(DiagCode::S101),
        Diagnostic::from_thunking(&ThunkingWarning {
            span: Span::new(0, 1),
            variable: "v".to_owned(),
            type_a: TclType::List,
            type_b: TclType::String,
            code: DiagCode::S102,
            message: String::new(),
            related: Vec::new(),
        }),
        Diagnostic::from_sharing(&SharingWarning {
            span: Span::new(0, 1),
            variable: "b".to_owned(),
            shared_with: "a".to_owned(),
            command: "lappend".to_owned(),
            code: DiagCode::S103,
            message: String::new(),
            related: Vec::new(),
        }),
        shimmer(DiagCode::S110),
        Diagnostic::from_path_concat(&crate::path_concat::PathConcatWarning {
            span: Span::new(0, 1),
            variable: "p".to_owned(),
            code: DiagCode::W201,
            message: String::new(),
            replacement: None,
        }),
    ];
    converted.extend(taint_conversions());
    converted.extend(irules_conversions());
    let codes: Vec<_> = converted.iter().map(|d| d.code).collect();
    let mut expected = CheckPayload::CODES.to_vec();
    let mut sorted = codes.clone();
    expected.sort();
    sorted.sort();
    assert_eq!(sorted, expected);
    for diagnostic in converted {
        assert!(
            diagnostic.payload.accepts(diagnostic.code),
            "{} carries {:?}",
            diagnostic.code,
            diagnostic.payload
        );
    }
    assert!(
        !CheckPayload::Irules(IrulesCheckPayload::UnguardedDnsReturn).accepts(DiagCode::Irule5002)
    );
    assert!(!CheckPayload::ConstantBranch { value: true }.accepts(DiagCode::O105));
}

#[test]
fn payload_fields_are_the_producer_subjects() {
    let taint = diagnostics("tcl9.0", "set x [gets stdin]\neval $x");
    let subject = taint
        .iter()
        .find(|d| d.code == DiagCode::T100)
        .and_then(Diagnostic::taint_subject)
        .expect("T100 taint subject");
    assert_eq!(subject.variable(), "x");
    assert_eq!(subject.sink_command(), "eval");

    let shared = diagnostics(
        "tcl9.0",
        "set a [lrepeat 1000 x]\nset b $a\nlappend b y\nputs [llength $a]",
    );
    assert!(
        shared.iter().any(|d| d.payload
            == CheckPayload::SharedCopy {
                variable: "b".to_owned(),
                shared_with: "a".to_owned(),
                command: "lappend".to_owned(),
            }),
        "{shared:?}"
    );

    let drop = diagnostics("f5-irules", "when CLIENT_ACCEPTED { drop }");
    assert!(
        drop.iter().any(|d| d.payload
            == CheckPayload::Irules(IrulesCheckPayload::UnguardedDrop {
                command: "drop".to_owned(),
            })),
        "{drop:?}"
    );
}
