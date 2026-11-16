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

//! IRULE3103: URI splitting, expression operators, `string match` and the edge cases.

// Splitting HTTP::uri on `?` / `&` instead of using
// HTTP::path / HTTP::query. f5-dialect.
mod irule3103_uri_split {
    use super::super::*;

    #[test]
    fn split_uri_on_question_mark() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_uri_on_ampersand() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"&\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_uri_on_question_and_ampersand() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"?&\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn inline_command_substitution() {
        let ws = of_code("set parts [split [HTTP::uri] \"?\"]", IR, "IRULE3103");
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn split_non_uri_clean() {
        let ws = of_code(
            "set x \"foo?bar\"\nset parts [split $x \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn split_http_path_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nset parts [split $p \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn split_uri_on_slash_clean() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset parts [split $uri \"/\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn copy_propagation() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset copy $uri\nset parts [split $copy \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn tcl_dialect_clean() {
        let ws = of_code("set x \"foo\"\nset parts [split $x \"?\"]", D, "IRULE3103");
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn uri_setter_not_flagged() {
        // The HTTP::uri setter form (with arg) is not a getter origin.
        let ws = of_code(
            "HTTP::uri \"/new\"\nset parts [split [HTTP::uri /path] \"?\"]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// Expression operators on HTTP::uri suggesting
// HTTP::path / HTTP::query. f5-dialect.
mod irule3103_expr_operators {
    use super::super::*;

    #[test]
    fn starts_with_path_pattern() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("starts_with"));
    }

    #[test]
    fn starts_with_via_variable() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { $uri starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn ends_with_extension() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] ends_with \".html\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn contains_query_param() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] contains \"&key=\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn contains_equals_query() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] contains \"user=test\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn matches_glob_path() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_glob \"/api/*\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn matches_glob_query() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_glob \"*&key=*\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn non_uri_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nif { $p starts_with \"/api\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn ambiguous_operand_clean() {
        let ws = of_code(
            "if { [HTTP::uri] contains \"something\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }
}

// string match / string first on HTTP::uri.
// f5-dialect.
mod irule3103_string_match {
    use super::super::*;

    #[test]
    fn string_match_path_pattern() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"/api/*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_match_query_pattern() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"*&key=*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn string_first_question_mark() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset pos [string first \"?\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path") || ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn string_match_non_uri_clean() {
        let ws = of_code(
            "set p [HTTP::path]\nset m [string match \"/api/*\" $p]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn string_match_ambiguous_pattern_clean() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"*something*\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn string_match_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string match \"/static/*\" $uri] } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_first_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string first \"?\" $uri] >= 0 } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
    }
}

// glob/regex classifier edge cases and SSA correctness.
// f5-dialect.
//
// tclsh-semantic premises (glob/regex metacharacter meanings): glob `?` is a
// single-char wildcard (not a query delimiter); regex `?` is a quantifier; an
// escaped `\?` / `\&` is a literal char (query signal). These are about how a
// pattern's characters classify, not a runtime side-effect to observe.
mod irule3103_edge_cases {
    use super::super::*;

    #[test]
    fn glob_question_mark_is_wildcard_not_query() {
        // glob `??` is a wildcard; `/api/??` is path-like (starts with `/`).
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"/api/??\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn glob_bare_question_not_query() {
        // Bare `??` wildcard with no path prefix → ambiguous → no fire.
        let ws = of_code(
            "set uri [HTTP::uri]\nset m [string match \"??\" $uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn glob_question_path_with_wildcard() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_glob \"/api/?\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_question_is_quantifier_not_query() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // regex `?` is a quantifier; `/api/v[0-9]+/?` is path-like. Braced,
        // because in quotes the `[0-9]` is a command substitution (#2227).
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_regex {^/api/v[0-9]+/?$} } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_escaped_question_is_query() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // regex `\?` is a literal `?` → query signal.
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_regex \"\\\\?key=\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn later_reassignment_no_false_positive() {
        // $uri is "foo" at the expr check; reassigned to HTTP::uri afterwards.
        let ws = of_code(
            "set uri \"foo\"\nset ok [expr { $uri starts_with \"/api\" }]\nset uri [HTTP::uri]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws, [] as [tcl_compiler::taint::TaintWarning; 0]);
    }

    #[test]
    fn string_match_nocase_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string match -nocase \"/STATIC/*\" $uri] } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn split_separator_from_constant_variable() {
        // SCCP-resolved separator variable still triggers the split warning.
        let ws = of_code(
            "set uri [HTTP::uri]\nset sep \"?\"\nset parts [split $uri $sep]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn phi_mixed_uri_and_non_uri_no_warning() {
        // Mixed-origin phi (URI + non-URI) → no warning.
        let source = "set cond 0\nif { $cond } {\n    set candidate [HTTP::uri]\n} else {\n    set candidate \"/not-uri\"\n}\nset out [string match \"/api/*\" $candidate]";
        assert_eq!(
            of_code(source, IR, "IRULE3103"),
            [] as [tcl_compiler::taint::TaintWarning; 0]
        );
    }

    #[test]
    fn string_first_with_ampersand_in_if_condition() {
        let ws = of_code(
            "set uri [HTTP::uri]\nif { [string first \"&\" $uri] >= 0 } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_separator_from_copied_constant_variable() {
        let ws = of_code(
            "set uri [HTTP::uri]\nset sep \"?\"\nset sep_copy $sep\nset parts [split $uri $sep_copy]",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn nested_boolean_expression_emits_one_hit_per_uri_use() {
        // One warning per concrete URI-pattern use.
        let source = "set uri [HTTP::uri]\nif { ([string match \"/api/*\" $uri] && ([HTTP::uri] contains \"&id=\")) } {\n    log local0. ok\n}";
        let ws = of_code(source, IR, "IRULE3103");
        assert_eq!(ws.len(), 2);
        let mut sinks: Vec<String> = ws.iter().map(|w| w.sink_command.clone()).collect();
        sinks.sort();
        assert_eq!(
            sinks,
            vec!["contains".to_string(), "string match".to_string()]
        );
    }

    #[test]
    fn phi_both_branches_uri_still_warns() {
        let source = "set cond 1\nif { $cond } {\n    set candidate [HTTP::uri]\n} else {\n    set tmp [HTTP::uri]\n    set candidate $tmp\n}\nset out [string match \"/api/*\" $candidate]";
        let ws = of_code(source, IR, "IRULE3103");
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn regex_escaped_ampersand_is_query() {
        // Dialect build: a standalone unit without an invocation dialect stays opaque.
        // regex `\&` is a literal `&` → query signal.
        let ws = dialect_of_code(
            "if { [HTTP::uri] matches_regex \"key\\\\&id\" } { log local0. x }",
            IR,
            "IRULE3103",
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }
}
