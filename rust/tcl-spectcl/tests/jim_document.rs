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

//! What a `# tcl-dialect: jim` document sees, end to end: Jim's own commands
//! from the compiled-in pack, the analyser and the signature scan reading the
//! same registry, and the same text under another dialect keeping that
//! dialect's answers.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::signature_scan::extract_signatures;
use tcl_core_types::DiagCode;
use tcl_registry::model::resolve_environment;

/// One diagnostic: its code and the source text it covers.
type Finding = (DiagCode, String);

fn analyse_with(source: &str, per_item: bool) -> Vec<Finding> {
    tcl_spectcl::core_surfaces::ensure();
    let dialect = tcl_registry::dialects::detect_dialect(source, None, "tcl");
    let mut analyser = Analyser::new();
    let result = if per_item {
        analyser.analyse_per_item(source, dialect)
    } else {
        analyser.analyse(source, dialect)
    };
    result
        .diagnostics
        .iter()
        .map(|d| {
            let end = (d.span.end() as usize).min(source.len());
            (d.code, source[d.span.start() as usize..end].to_owned())
        })
        .collect()
}

/// The findings that are not a hint about an unused variable.
fn substantive(findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .filter(|(code, _)| !matches!(code, DiagCode::W211 | DiagCode::W220))
        .collect()
}

fn jim(body: &str) -> String {
    format!("# tcl-dialect: jim\n{body}")
}

fn tcl86(body: &str) -> String {
    format!("# tcl-dialect: tcl8.6\n{body}")
}

/// Both analyser tiers: the whole-file walk and the per-item shell walk that
/// defers proc bodies.
fn both_tiers(source: &str) -> [Vec<Finding>; 2] {
    [analyse_with(source, false), analyse_with(source, true)]
}

/// A static is declared in the body scope and is not an argument.
#[test]
fn a_static_is_a_local_of_the_body_and_no_argument() {
    let source = jim("proc counter {} {{n 0}} { return [incr n] }\nputs [counter]\n");
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// Each spelling of a static declares its name: `name`, `{name value}` and,
/// from 0.83, `&name`.
#[test]
fn every_static_spelling_declares_its_name() {
    let source = jim("set seed 5\nset shared 1\n\
         proc mix {a} {seed {n 0} &shared} { return [expr {$a + $seed + $n + $shared}] }\n\
         puts [mix 1]\n");
    for findings in both_tiers(&source) {
        assert_eq!(substantive(findings), vec![], "{source}");
    }
}

/// The body of a proc with statics is still walked: an unknown command in it
/// is reported.
#[test]
fn the_body_of_a_proc_with_statics_is_walked() {
    let source = jim("proc counter {} {{n 0}} { not_a_command_anywhere $n }\ncounter\n");
    for findings in both_tiers(&source) {
        assert_eq!(
            findings,
            vec![(DiagCode::W123, "not_a_command_anywhere".to_owned())],
            "{source}"
        );
    }
}

/// A static never adds to the arity: the call supplies the parameters alone.
#[test]
fn a_static_adds_nothing_to_the_arity() {
    let source = jim("proc pair {a b} {{n 0}} { return $a$b$n }\npair 1 2\npair 1\npair 1 2 3\n");
    for findings in both_tiers(&source) {
        let codes: Vec<(DiagCode, &str)> = findings
            .iter()
            .map(|(code, text)| (*code, text.as_str()))
            .collect();
        assert_eq!(
            codes,
            vec![(DiagCode::E002, "pair 1"), (DiagCode::E003, "3")],
            "{source}"
        );
    }
}

/// Three words is the plain form under Jim as under Tcl.
#[test]
fn a_three_word_proc_is_the_plain_form() {
    let source = jim("proc plain {a b} { return $a$b }\nputs [plain 1 2]\n");
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// Tcl's `proc` has no static list: the same four words are an arity error,
/// and the error names the three-word usage.
#[test]
fn a_tcl_document_still_rejects_the_four_word_proc() {
    let source = tcl86("proc counter {} {{n 0}} { incr n }\n");
    for findings in both_tiers(&source) {
        assert!(
            findings.iter().any(|(code, _)| *code == DiagCode::E003),
            "{findings:?}"
        );
    }
}

/// The signature scan places the name, parameter list and body from the same
/// role data, so a cross-file consumer sees the four-word definition as the
/// analyser does.
#[test]
fn the_signature_scan_reads_the_four_word_definition() {
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("jim").default_context_registry();
    let source = "proc pair {a b} {{n 0}} { return $a$b$n }\nproc plain {x} { return $x }\n";
    let scan = extract_signatures(source, generation.commands());

    let pair = &scan.procs["::pair"];
    let params: Vec<&str> = pair.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(params, ["a", "b"]);
    let body = &source[pair.body_range.start() as usize..pair.body_range.end() as usize];
    assert!(body.contains("return $a$b$n"), "{body}");

    let plain = &scan.procs["::plain"];
    let body = &source[plain.body_range.start() as usize..plain.body_range.end() as usize];
    assert!(body.contains("return $x"), "{body}");
}

/// A Tcl registry places the three-word definition exactly as before.
#[test]
fn the_signature_scan_reads_the_three_word_definition_under_tcl() {
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("tcl8.6").default_context_registry();
    let source = "proc pair {a b} { return $a$b }\n";
    let scan = extract_signatures(source, generation.commands());
    let pair = &scan.procs["::pair"];
    assert_eq!(pair.params.len(), 2);
    let body = &source[pair.body_range.start() as usize..pair.body_range.end() as usize];
    assert!(body.contains("return $a$b"), "{body}");
}
