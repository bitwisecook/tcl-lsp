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

//! tcllib import wrappers, command aliases, `switch -regexp` patterns and regex variables the walk records.

use super::*;

// -- tcllib `<NS>::import <ALIAS>` wrapper detection

#[test]
fn analyse_records_tcllib_import_wrapper_as_conjectured() {
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::vt");
    assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
}

#[test]
fn analyse_tcllib_import_wrapper_alias_relative_to_current_namespace() {
    // ``some::ns::import alias`` inside ``namespace eval outer``
    // creates ``::outer::alias``, not ``::alias``.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval outer { term::ansi::send::import vt }\n",
        "tcl",
    );
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::outer::vt");
    assert_eq!(conjectured[0].pattern, "::term::ansi::send::*");
}

#[test]
fn analyse_tcllib_import_wrapper_absolute_alias_keeps_leading_colons() {
    // ``::alias`` argument is taken as an absolute namespace —
    // current-namespace prefixing is skipped.
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import ::abs::vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].ns, "::abs::vt");
}

#[test]
fn analyse_tcllib_import_wrapper_skips_substituted_alias() {
    // ``$var`` / ``[cmd]`` aliases can't be statically resolved —
    // the guard requires the alias to contain no ``$`` or ``[``.
    let mut a = Analyser::new();
    let r1 = a.analyse("term::ansi::send::import $alias\n", "tcl");
    assert!(r1.namespace_imports.iter().all(|i| !i.conjectured));
    let mut a = Analyser::new();
    let r2 = a.analyse("term::ansi::send::import [build]\n", "tcl");
    assert!(r2.namespace_imports.iter().all(|i| !i.conjectured));
}

#[test]
fn analyse_tcllib_import_wrapper_requires_single_argument() {
    // ``X::import alias extras`` is a non-wrapper call — the
    // wrapper idiom takes exactly one alias word.
    let mut a = Analyser::new();
    let r = a.analyse("term::ansi::send::import vt extra\n", "tcl");
    assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
}

#[test]
fn analyse_tcllib_import_wrapper_qualifies_unprefixed_source_ns() {
    // Wrapper command names without a leading ``::`` still
    // resolve to absolute source namespaces — the helper
    // prepends the missing ``::``.
    let mut a = Analyser::new();
    let r = a.analyse("foo::import vt\n", "tcl");
    let conjectured: Vec<_> = r
        .namespace_imports
        .iter()
        .filter(|i| i.conjectured)
        .collect();
    assert_eq!(conjectured.len(), 1);
    assert_eq!(conjectured[0].pattern, "::foo::*");
}

// -- ``command_aliases`` tests
//
// Pin the no-transitive-chain behaviour of command aliases.

#[test]
fn analyse_alias_chain_records_each_step_independently() {
    // ``a -> b`` and ``b -> expr`` are recorded as two
    // independent entries — neither side resolves
    // transitively to ``expr``.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} a {} b\ninterp alias {} b {} expr\n", "tcl");
    let alias_a = r.command_aliases.get("::a").expect("::a recorded");
    assert_eq!(alias_a.target, "b");
    assert_eq!(alias_a.extras, [] as [std::string::String; 0]);
    let alias_b = r.command_aliases.get("::b").expect("::b recorded");
    assert_eq!(alias_b.target, "expr");
    assert_eq!(alias_b.extras, [] as [std::string::String; 0]);
}

#[test]
fn analyse_alias_redefinition_overwrites_target() {
    // The second declaration wins.
    let mut a = Analyser::new();
    let r = a.analyse(
        "interp alias {} myop {} expr\ninterp alias {} myop {} puts\n",
        "tcl",
    );
    let alias = r.command_aliases.get("::myop").expect("::myop recorded");
    assert_eq!(alias.target, "puts");
    assert_eq!(alias.extras, [] as [std::string::String; 0]);
}

#[test]
fn analyse_alias_qualified_name_recorded() {
    // ``interp alias {} ::ns::myop {} expr`` records under the
    // fully-qualified key.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} ::math::= {} expr\n", "tcl");
    let alias = r
        .command_aliases
        .get("::math::=")
        .expect("::math::= recorded");
    assert_eq!(alias.target, "expr");
}

#[test]
fn analyse_alias_dynamic_name_not_recorded() {
    // ``$n`` in the alias name field doesn't resolve statically
    // — ``::=`` must not appear in ``command_aliases``.
    let mut a = Analyser::new();
    let r = a.analyse("set n \"=\"\ninterp alias {} $n {} expr\n", "tcl");
    assert!(!r.command_aliases.contains_key("::="));
}

// -- ``switch -regexp`` literal-pattern recording
//
// The pattern arms whose token is a literal are recorded as
// ``RegexPattern { command = "switch" }``; ``default`` and
// var / cmd-sub patterns are skipped (variable patterns are
// handled by the regex-vars resolution instead).

#[test]
fn analyse_switch_regexp_form1_records_literal_patterns() {
    // Form 1: pattern/body pairs inline.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val \"^foo\" { puts foo } \"^bar\" { puts bar }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 2);
    assert_eq!(switch_pats[0].pattern, "^foo");
    assert_eq!(switch_pats[1].pattern, "^bar");
}

#[test]
fn analyse_switch_regexp_form2_records_literal_patterns() {
    // Form 2: braced body with pattern/body pairs.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { ^foo { puts foo } ^bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 2);
    assert_eq!(switch_pats[0].pattern, "^foo");
    assert_eq!(switch_pats[1].pattern, "^bar");
}

#[test]
fn analyse_braced_expect_clause_regexp_records_literal_and_const_var_patterns() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "set pat {(a+)+$}\nexpect {-regexp {^lit$} { puts lit } -re $pat { puts var }}\n",
        "expect",
    );
    let patterns: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "expect")
        .collect();
    assert!(
        patterns.iter().any(|pattern| pattern.pattern == "^lit$"),
        "literal per-clause regexp missing: {patterns:?}"
    );
    assert!(
        patterns.iter().any(|pattern| pattern.pattern == "(a+)+$"),
        "const variable per-clause regexp missing: {patterns:?}"
    );
}

#[test]
fn analyse_switch_regexp_skips_default_arm() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { ^foo { puts foo } default { puts none } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 1);
    assert_eq!(switch_pats[0].pattern, "^foo");
}

#[test]
fn analyse_switch_without_regexp_records_nothing() {
    // No ``-regexp`` flag — patterns are glob, not regex.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -- $val { foo { puts foo } bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(
        switch_pats,
        [] as [&crate::analyser::types::RegexPattern; 0]
    );
}

#[test]
fn analyse_switch_regexp_skips_unresolved_var_pattern() {
    // ``$pat`` arm with no defining ``set`` — no const value
    // available, so the arm is dropped.
    let mut a = Analyser::new();
    let r = a.analyse(
        "switch -regexp -- $val { $pat { puts hit } ^lit { puts lit } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    assert_eq!(switch_pats.len(), 1);
    assert_eq!(switch_pats[0].pattern, "^lit");
}

// -- ``regex-vars`` const-string propagation
//
// Verify that ``$var`` regex pattern arguments resolve to the
// literal stored by a preceding ``set var "..."`` (regexp /
// regsub and switch -regexp Form 2).

#[test]
fn analyse_regexp_resolves_var_pattern_to_const_string() {
    let mut a = Analyser::new();
    let r = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
    // Two records: the use site (the ``$p`` token) and the
    // defining ``set`` value.
    let regexp_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "regexp")
        .collect();
    assert_eq!(regexp_pats.len(), 2);
    assert!(regexp_pats.iter().all(|p| p.pattern == "^foo"));
}

#[test]
fn analyse_regsub_resolves_var_pattern_to_const_string() {
    let mut a = Analyser::new();
    let r = a.analyse("set p {a+}\nregsub -all $p $line - out\n", "tcl");
    let regsub_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "regsub")
        .collect();
    assert_eq!(regsub_pats.len(), 2);
    assert!(regsub_pats.iter().all(|p| p.pattern == "a+"));
}

/// A braced switch body substitutes nothing: `$p` there is the literal
/// pattern `$p`, never `p`'s value. tclsh 8.4.20, 8.6 and 9.0 send `foo`
/// to `default`.
#[test]
fn analyse_switch_regexp_braced_pattern_is_not_the_variable_value() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "set p {^foo}\nswitch -regexp -- $val { $p { puts foo } ^bar { puts bar } }\n",
        "tcl",
    );
    let switch_pats: Vec<_> = r
        .regex_patterns
        .iter()
        .filter(|p| p.command == "switch")
        .collect();
    let pats: Vec<&str> = switch_pats.iter().map(|p| p.pattern.as_str()).collect();
    assert!(!pats.contains(&"^foo"), "got {pats:?}");
    assert!(pats.contains(&"^bar"), "got {pats:?}");
}

#[test]
fn analyse_regex_var_unresolved_records_nothing() {
    // No defining ``set`` — Var has no const value.  The
    // pattern arg is dropped.
    let mut a = Analyser::new();
    let r = a.analyse("regexp $p $line\n", "tcl");
    assert_eq!(
        r.regex_patterns,
        [] as [crate::analyser::types::RegexPattern; 0]
    );
}

#[test]
fn analyse_regexp_var_added_to_regex_vars_set() {
    // Side effect: the var name is recorded in
    // ``regex_vars`` so downstream consumers (var-as-regex
    // hint, future W*-codes) can find the defining set.
    let mut a = Analyser::new();
    let _ = a.analyse("set p {^foo}\nregexp $p $line\n", "tcl");
    // The const-string scope is the global scope (path = []).
    assert!(a.regex_vars.contains(&(Vec::new(), "p".to_string())));
}

#[test]
fn analyse_alias_with_prepended_args_recorded() {
    // Prepended args after the target are stored on
    // ``extras``.
    let mut a = Analyser::new();
    let r = a.analyse("interp alias {} logerr {} puts stderr\n", "tcl");
    let alias = r
        .command_aliases
        .get("::logerr")
        .expect("::logerr recorded");
    assert_eq!(alias.target, "puts");
    assert_eq!(alias.extras.as_slice(), &["stderr"]);
}

#[test]
fn analyse_tcllib_import_wrapper_does_not_fire_on_namespace_import() {
    // The wrapper detector must not trip on Tcl's own
    // ``namespace import`` — that's handled by
    // ``handle_namespace_import_command`` and is never
    // conjectured.
    let mut a = Analyser::new();
    let r = a.analyse("namespace import ::foo::bar\n", "tcl");
    assert!(r.namespace_imports.iter().all(|i| !i.conjectured));
}
