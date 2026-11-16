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

//! The receiver and method word at the cursor, and `next`/`nextto` targets.

use super::*;

#[test]
fn jump_to_next_super_method() {
    // `next` inside B::greet jumps to the overridden A::greet.
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `next` (line 5). `    method greet {} { next }`
    let locs = definition(src, 5, 23, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // A::greet's name is on line 1 at column 11.
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn jump_to_nextto_named_class_method() {
    // `nextto A` inside C::greet jumps to A::greet (skipping B).
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` (line 9). `    method greet {} { nextto A }`
    let locs = definition(src, 9, 22, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1, "should land on A::greet");
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn jump_to_nextto_namespaced_class_method() {
    // `nextto A` names a namespaced sibling bare from within `::Ns::C`.
    // A method body resolves `A` through the object namespace, its
    // `::oo::Helpers` path, then `::` — never `::Ns` — so tclsh 8.6/9.0
    // raise "A does not refer to an object" and there is no target.
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn jump_to_nextto_relative_qualified_class_method() {
    // `nextto Ns::A` inside `::Ns::C` falls back to `::` and reaches
    // `::Ns::A` (tclsh 8.6/9.0 dispatch `::Ns::A::greet`).
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto Ns::A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // ::Ns::A::greet's name token is on line 2.
    assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
}

#[test]
fn jump_to_nextto_fully_qualified_class_method() {
    // `nextto ::Ns::A` inside `::Ns::C` (tclsh 8.6/9.0 dispatch
    // `::Ns::A::greet`).
    let src = "namespace eval Ns {\n    oo::class create A {\n        method greet {} { return hi }\n    }\n    oo::class create B {\n        superclass A\n        method greet {} { next }\n    }\n    oo::class create C {\n        superclass B\n        method greet {} { nextto ::Ns::A }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 28, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // ::Ns::A::greet's name token is on line 2.
    assert_eq!(locs[0].start_line, 2, "should land on ::Ns::A::greet");
}

#[test]
fn nextto_refuses_a_class_name_reached_through_the_helpers_path() {
    // Bare `A` reaches `::oo::Helpers::A` through the method body's
    // namespace path before `::A`: tclsh 8.6/9.0 dispatch
    // `::oo::Helpers::A::greet` (line 2), never `::A::greet` (line 6).
    // The run-time object namespace is not modelled, so the answer is
    // refused rather than guessed.
    let src = "namespace eval ::oo::Helpers {\n    oo::class create A {\n        method greet {} { return helpers }\n    }\n}\noo::class create A {\n    method greet {} { return root }\n}\noo::class create C {\n    superclass ::A ::oo::Helpers::A\n    method greet {} { nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor on `nextto` inside C::greet (line 10).
    let locs = definition(src, 10, 22, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn nextto_picks_occurrence_at_cursor_not_first_on_line() {
    // The line has a decoy `nextto` earlier (inside a string) before the
    // real `nextto A`.  With the cursor on the real one, resolution must
    // read the class after *that* occurrence (A), not the first match.
    let src = "oo::class create A {\n    method greet {} { return hi }\n}\noo::class create B {\n    superclass A\n    method greet {} { next }\n}\noo::class create C {\n    superclass B\n    method greet {} { set x \"nextto Z\" ; nextto A }\n}\n";
    let analysis = analyse(src);
    // Cursor inside the real `nextto` (line 9, col 44).
    let locs = definition(src, 9, 44, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "should land on A::greet, not the string decoy"
    );
    assert_eq!(locs[0].start_character, 11);
}

#[test]
fn instance_method_at_cursor_detects_dollar_head() {
    let src = "$d bark\n";
    let got = instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default());
    assert_eq!(got, Some(("d".to_string(), "bark".to_string(), true)));
}

#[test]
fn instance_method_var_decoration_vectors_keep_unclosed_braces() {
    assert_eq!(
        instance_method_at_cursor("${x} bark\n", 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("x".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$x bark\n", 0, 3, tcl_lexer::LexerConfig::default()),
        Some(("x".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$arr(idx) bark\n", 0, 10, tcl_lexer::LexerConfig::default()),
        Some(("arr(idx)".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$::a:::b bark\n", 0, 9, tcl_lexer::LexerConfig::default()),
        Some(("::a:::b".to_string(), "bark".to_string(), true))
    );
    assert_eq!(
        instance_method_at_cursor("$foo::: bark\n", 0, 9, tcl_lexer::LexerConfig::default()),
        Some(("foo:::".to_string(), "bark".to_string(), true))
    );
    // The malformed source is tokenised before this feature sees it;
    // assert the shared codec's contract directly, while the following
    // feature-level test proves it cannot resolve the real `x` object.
    assert_eq!(tcl_syntax::naming::var_reference("${x"), "{x");
}

#[test]
fn definition_does_not_resolve_an_unclosed_braced_instance_head() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 4, &analysis);
    assert!(
        locs.is_empty(),
        "unclosed `${{x` must not dispatch through real `x`: {locs:?}"
    );
}

#[test]
fn instance_method_at_cursor_reports_bare_head_as_non_dollar() {
    // `foo bark` — a bare-word receiver (an object command); reported
    // with `is_dollar == false`.  Whether it actually resolves to a
    // class is decided later by `logical_receiver_candidate_at`.
    let src = "foo bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("foo".to_string(), "bark".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_extracts_a_substituted_head_verbatim() {
    // `[x] bark` — a command-substitution head is not a bare object
    // command, but this function only extracts the receiver text;
    // whether `[x]` names anything a caller can resolve is a semantic
    // question answered downstream by the original selected descriptor, not here. Rejecting every bracketed head
    // outright would also reject `[self]` / `[self object]`, TclOO's own
    // same-object dispatch spelling.
    let src = "[x] bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 5, tcl_lexer::LexerConfig::default()),
        Some(("[x]".to_string(), "bark".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_extracts_a_self_receiver_head_with_brackets_intact() {
    // `[self] m`. The brackets must survive
    // extraction: `parse_command_substitution` (and so
    // `original_internal_dispatch`) requires them.
    let src = "[self] mrun\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("[self]".to_string(), "mrun".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_still_rejects_a_malformed_bracket_head() {
    // A stray, unmatched `]` with no opening `[` in scope is not a
    // well-formed command substitution and not a bare identifier
    // either, so it is rejected.
    let src = "x] bark\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default()),
        None
    );
}

// Method names with non-identifier characters.
// Oracle (tclsh 8.6.14 + 9.0.4): `method with-dash`, `method a.b`, and
// TIP 558's `<ReadProp-x>` / `<WriteProp-x>` all dispatch for real.

#[test]
fn instance_method_at_cursor_keeps_a_hyphenated_method_name_whole() {
    // TP — `-` is part of the method word, not a boundary.
    let src = "$d with-dash\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("d".to_string(), "with-dash".to_string(), true))
    );
}

#[test]
fn instance_method_at_cursor_keeps_an_angle_bracketed_method_name_whole() {
    // TP — the TIP 558 property-accessor shape, cursor inside the name.
    let src = "my <ReadProp-colour>\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        Some(("my".to_string(), "<ReadProp-colour>".to_string(), false))
    );
}

#[test]
fn instance_method_at_cursor_keeps_a_dotted_method_name_whole() {
    // TP — `method a.b` dispatches in real Tcl; `.` is not a boundary.
    let src = "$d a.b\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 4, tcl_lexer::LexerConfig::default()),
        Some(("d".to_string(), "a.b".to_string(), true))
    );
}

#[test]
fn instance_method_at_cursor_rejects_subtraction_in_an_expression() {
    // TN — `$x-1` is arithmetic. The whole `x-1` run is one word whose
    // head slice is a bare `$`, so nothing dispatches.
    let src = "expr {$x-1}\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 8, tcl_lexer::LexerConfig::default()),
        None
    );
    assert_eq!(
        instance_method_at_cursor(src, 0, 9, tcl_lexer::LexerConfig::default()),
        None
    );
}

#[test]
fn instance_method_at_cursor_rejects_a_bare_comparison_operator() {
    // TN — `expr {$a < $b}` must not read as `$a` dispatching `<`:
    // the word carries no alphanumeric content.
    let src = "expr {$a < $b}\n";
    assert_eq!(
        instance_method_at_cursor(src, 0, 9, tcl_lexer::LexerConfig::default()),
        None
    );
}

#[test]
fn hyphenated_and_angle_bracketed_methods_resolve_from_a_call_site() {
    // TP — end-to-end. `with-dash` starts with an ASCII lowercase
    // letter so it is exported and dispatches externally; the TIP 558
    // accessor `<ReadProp-x>` starts with `<` so it is *unexported* and
    // only reachable from inside the class via `my` (oracle: tclsh
    // 9.0.4 answers `unknown method "<foo>"` for an external call but
    // runs `my <ReadProp-x>` fine).
    let src = concat!(
        "oo::class create C {\n",
        "    method with-dash {} { return 1 }\n",
        "    method <ReadProp-x> {} { return 2 }\n",
        "    method probe {} { my <ReadProp-x> }\n",
        "}\n",
        "C create rex\n",
        "rex with-dash\n",
    );
    let analysis = analyse(src);
    let dash = definition(src, 6, 6, &analysis);
    assert_eq!(dash.len(), 1, "{dash:?}");
    assert_eq!(dash[0].start_line, 1);
    assert_eq!(dash[0].start_character, 11);
    let prop = definition(src, 3, 28, &analysis);
    assert_eq!(prop.len(), 1, "{prop:?}");
    assert_eq!(prop[0].start_line, 2);
    assert_eq!(prop[0].start_character, 11);
}

#[test]
fn unexported_angle_bracketed_method_is_not_externally_dispatchable() {
    // TN — the export rule still applies to the widened word: an
    // external `rex <ReadProp-x>` is `unknown method` in real Tcl
    // (verified, tclsh 8.6.14 + 9.0.4), so it must resolve to nothing
    // rather than to the declaration.
    let src = concat!(
        "oo::class create C {\n",
        "    method <ReadProp-x> {} { return 2 }\n",
        "}\n",
        "C create rex\n",
        "rex <ReadProp-x>\n",
    );
    let analysis = analyse(src);
    assert_eq!(
        definition(src, 4, 8, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
pub(super) fn logical_receiver_candidates_keep_instance_and_class_buckets_together() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let source = "oo::class create Bar {method get {} {}}; Bar create rex; set b [Bar new]";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_lexical_declaration_advice());
    assert!(matches!(
        logical_receiver_candidate_at(&analysis, "rex", false, 0),
        Some((_, MethodBucket::Instance))
    ));
    assert!(logical_receiver_candidate_at(&analysis, "b", false, 0).is_none());
    assert!(matches!(
        logical_receiver_candidate_at(&analysis, "Bar", false, 0),
        Some((_, MethodBucket::Class))
    ));
    let native = analyse(source);
    for receiver in ["rex", "b", "Bar"] {
        assert!(logical_receiver_candidate_at(&native, receiver, false, 0).is_none());
    }
}
