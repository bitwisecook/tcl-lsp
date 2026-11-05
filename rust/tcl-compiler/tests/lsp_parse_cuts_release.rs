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

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

#[test]
fn nested_command_reports_extra_characters_after_closer_2436() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, code, offset) in [
            ("puts [list {a b}c]", DiagCode::E204, 16),
            ("puts [list \"a b\"c]", DiagCode::E205, 16),
            ("puts [list [list {a}b]]", DiagCode::E204, 20),
            ("puts $a([list {a}b])", DiagCode::E204, 5),
            ("puts {a}b", DiagCode::E204, 8),
            ("puts pre\nputs [list {a b}c]", DiagCode::E204, 25),
        ] {
            let result = Analyser::new().analyse(source, dialect);
            let findings: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|d| d.code == code)
                .collect();
            assert_eq!(findings.len(), 1, "{dialect} {source}: {findings:?}");
            assert_eq!(findings[0].span.start(), offset, "{dialect} {source}");
        }
        let source = "puts [list {a}b]\nputs [list \"c\"d]";
        let result = Analyser::new().analyse(source, dialect);
        for code in [DiagCode::E204, DiagCode::E205] {
            assert_eq!(
                result.diagnostics.iter().filter(|d| d.code == code).count(),
                1,
                "{dialect}: {:?}",
                result.diagnostics
            );
        }
        for source in ["puts {[list {a}b]}", "puts [list {a b} c]"] {
            let result = Analyser::new().analyse(source, dialect);
            assert!(
                !result
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.code, DiagCode::E204 | DiagCode::E205)),
                "{dialect} {source}"
            );
        }
    }
}

#[test]
fn jim_accepts_nested_close_quote_suffix() {
    let result = Analyser::new().analyse("puts [list \"a b\"c]", "jim");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::E205),
        "{:?}",
        result.diagnostics
    );
}

fn assert_native_grammar(interpreter: &std::path::Path, jim: bool) {
    for (source, message) in [
        (
            "puts [list {a b}c]",
            Some("extra characters after close-brace"),
        ),
        (
            "puts [list \"a b\"c]",
            (!jim).then_some("extra characters after close-quote"),
        ),
        (
            "puts [list [list {a}b]]",
            Some("extra characters after close-brace"),
        ),
        (
            "puts $a([list {a}b])",
            Some("extra characters after close-brace"),
        ),
        ("puts {[list {a}b]}", None),
        ("puts [list {a b} c]", None),
    ] {
        let word = tcl_compiler::codegen::helpers::tcl_list_element(source);
        let script =
            format!("set completion [catch {word} message]\nputs [list $completion $message]\n");
        let outcome = tcl_test_support::run_script(interpreter, script.as_bytes()).unwrap();
        let output = outcome.strict_text().unwrap();
        let completion = output.lines().last().unwrap();
        if let Some(message) = message {
            assert_eq!(
                completion,
                format!("1 {{{message}}}"),
                "{interpreter:?} {source}"
            );
        } else {
            assert!(
                completion.starts_with("0 "),
                "{} {source}: {output}",
                interpreter.display()
            );
        }
    }
}

#[test]
fn nested_parse_cuts_match_c_tcl_references() {
    let interpreters = tcl_test_support::available_tclshs();
    assert!(
        !interpreters.is_empty(),
        "configure the C Tcl oracle matrix"
    );
    for interpreter in interpreters {
        assert_native_grammar(&interpreter.path, false);
    }
}

#[test]
fn nested_parse_cuts_match_current_jim() {
    let interpreter = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required current Jim oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("validated Jim oracle")
    };
    if let Some(interpreter) = interpreter {
        assert_native_grammar(&interpreter.path, true);
    }
}
