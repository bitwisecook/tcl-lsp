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

//! Constant and passthrough returns, return-word classification and whether a call may return.

use super::*;

#[test]
fn constant_return_inferred_for_literal_proc() {
    let ia = build("proc ::f {} { return 1 }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::int(1)));
}

#[test]
fn conditional_return_is_not_constant_when_body_falls_through() {
    // O103: `f` returns 42 only when `$x > 0`; otherwise it
    // falls off the end (the `if` returns "" on the no-match path),
    // so it is NOT constant-returning and must not be folded.
    let ia = build("proc ::f {x} { if {$x > 0} { return 42 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(
        !s.returns_constant,
        "fall-through proc must not be constant-returning, got {:?}",
        s.constant_return,
    );
    assert_eq!(s.constant_return, None);
}

#[test]
fn conditional_return_is_constant_when_all_paths_return() {
    // Fully covered if/else, both arms return the same literal →
    // genuinely constant, still folds.
    let ia = build("proc ::f {x} { if {$x > 0} { return 7 } else { return 7 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::int(7)));
}

#[test]
fn return_word_classification_keeps_literal_names_and_selected_grammar() {
    let params = HashSet::from([
        "x".to_owned(),
        "{x}".to_owned(),
        "a:b".to_owned(),
        "arr".to_owned(),
        "cash$name".to_owned(),
        String::new(),
    ]);
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        for (raw, expected) in [
            ("$x", Some("x")),
            ("${cash$name}", Some("cash$name")),
            ("${}", Some("")),
            ("$a:b", None),
            ("${arr(k)}", None),
            ("$arr(k)", None),
            ("${{x}}", profile.starts_with("tcl9").then_some("{x}")),
        ] {
            let source = format!("return {raw}");
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config);
            let tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(&source),
                config,
                &commands[0],
            );
            let word = tokens.words().get(1);
            let actual = classify_return(Some(raw), false, word, None, &params, Some(config));
            match (actual, expected) {
                (ReturnKind::Passthrough(actual), Some(expected)) => {
                    assert_eq!(actual, expected);
                }
                (ReturnKind::Other, None) => {}
                _ => panic!("{profile}: {raw}"),
            }
            assert!(matches!(
                classify_return(Some(raw), false, word, None, &params, None),
                ReturnKind::Other,
            ));
        }
        assert!(matches!(
            classify_return(Some("$x"), true, None, None, &params, Some(config)),
            ReturnKind::Literal(value) if value.bytes == b"$x",
        ));
        assert!(matches!(
            classify_return(Some("$x"), true, None, None, &params, None),
            ReturnKind::Other,
        ));
    }
}

#[test]
fn passthrough_param_detected() {
    let ia = build("proc ::id {x} { return $x }");
    let s = fixture_proc(&ia, "::id").unwrap();
    assert_eq!(s.return_passthrough_param.as_deref(), Some("x"));
    assert_eq!(s.return_depends_on_params, vec!["x".to_string()]);
}

/// Whether a statement of `::p`'s flow graph may run a `return` of its
/// own, as the return reading asks of each executable block.
fn p_may_return(source: &str) -> bool {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = CompilationUnit::build_for_profile(source, &registry, false, profile);
    unit.procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .any(|stmt| statement_may_return(stmt, &registry, 0))
}

/// A call holding a word the registry gives the Body role may run a
/// `return` there: the flow graph keeps a loop over a qualified variable
/// as a call holding its body, and a barrier's code is unseen. A `catch`
/// absorbs its body's `return`, and the header of a loop the flow graph
/// lowers holds only the list words.
#[test]
fn a_call_running_a_body_word_may_return() {
    assert!(p_may_return(
        "proc p {} {foreach ::x {1} {return 1}; return 2}"
    ));
    assert!(p_may_return(
        "proc p {} {lmap ::x {1} {return 1}; return 2}"
    ));
    assert!(p_may_return("proc p {} {time {return 1}; return 2}"));
    assert!(!p_may_return("proc p {} {catch {return 1}; return 2}"));
    assert!(!p_may_return(
        "proc p {} {set s 0; foreach x {1 2} y {3 4} z {5 6} {incr s}; return $s}"
    ));
}
