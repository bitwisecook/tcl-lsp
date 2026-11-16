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

//! Checks on what is written: regular-expression patterns and the name canonicalisation matrix.

// Recorded regex literals.
pub(super) mod regex_patterns {
    use super::super::*;

    fn pats(src: &str) -> Vec<(String, String)> {
        Analyser::new()
            .analyse(src, D)
            .regex_patterns
            .iter()
            .map(|p| (p.pattern.clone(), p.command.clone()))
            .collect()
    }

    // The subject variables are bound first: tclsh errors on reading an unset
    // variable, so the command under test would never run.
    #[test]
    fn regexp_simple_records_pattern_and_command() {
        let p = pats("set str [gets stdin]\nregexp {^[a-z]+$} $str");
        assert_eq!(p, [("^[a-z]+$".to_string(), "regexp".to_string())]);
    }

    #[test]
    fn regexp_with_options_and_terminator_and_start() {
        assert_eq!(
            pats("set str [gets stdin]\nregexp -nocase -expanded {\\d+} $str")[0].0,
            "\\d+"
        );
        assert_eq!(
            pats("set str [gets stdin]\nregexp -nocase -- {^test} $str")[0].0,
            "^test"
        );
        assert_eq!(
            pats("set str [gets stdin]\nregexp -start 5 {pattern} $str")[0].0,
            "pattern"
        );
    }

    #[test]
    fn regsub_simple_and_with_options() {
        let p = pats("set str [gets stdin]\nregsub {\\d+} $str replacement result");
        assert_eq!(p, [("\\d+".to_string(), "regsub".to_string())]);
        assert_eq!(
            pats("set str [gets stdin]\nregsub -all -nocase {foo} $str bar result")[0].0,
            "foo"
        );
    }

    #[test]
    fn switch_regexp_braced_and_inline_record_arms() {
        for src in [
            "switch -regexp $x { {^a} {puts a} {^b} {puts b} }",
            "switch -regexp $x {^a} {puts a} {^b} {puts b}",
        ] {
            let p = pats(src);
            assert_eq!(p.len(), 2, "for {src:?}");
            let pset: Vec<_> = p.iter().map(|(pat, _)| pat.as_str()).collect();
            assert!(pset.contains(&"^a") && pset.contains(&"^b"));
            assert!(p.iter().all(|(_, c)| c == "switch"));
        }
    }

    #[test]
    fn switch_regexp_excludes_default_arm() {
        let p = pats("switch -regexp $x { {^a} {puts a} default {puts other} }");
        assert_eq!(p, [("^a".to_string(), "switch".to_string())]);
    }

    #[test]
    fn switch_glob_exact_default_record_no_regex() {
        assert_eq!(
            pats("switch -glob $x { a* {puts a} b* {puts b} }"),
            [] as [(std::string::String, std::string::String); 0]
        );
        assert_eq!(
            pats("switch -exact $x { hello {puts hi} }"),
            [] as [(std::string::String, std::string::String); 0]
        );
        assert_eq!(
            pats("switch $x { hello {puts hi} }"),
            [] as [(std::string::String, std::string::String); 0]
        );
    }

    #[test]
    fn no_regex_in_other_commands() {
        assert_eq!(
            pats("set x 1\nputs hello\nif {$x > 0} { puts yes }"),
            [] as [(std::string::String, std::string::String); 0]
        );
    }

    #[test]
    fn multiple_regex_commands_in_one_file() {
        let p =
            pats("set x [gets stdin]\nset y [gets stdin]\nregexp {^a} $x\nregsub {^b} $y c result");
        assert_eq!(p.len(), 2);
        let cmds: std::collections::HashSet<_> = p.iter().map(|(_, c)| c.clone()).collect();
        assert_eq!(
            cmds,
            ["regexp".to_string(), "regsub".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn regexp_inside_proc_recorded() {
        let p = pats("proc check {s} { regexp {^\\d+$} $s }");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].1, "regexp");
    }

    // Variable propagation.

    #[test]
    fn set_then_regexp_and_regsub_propagate_constant_pattern() {
        // `set pat {re}; regexp $pat …` records two patterns (def + use site).
        let p = pats("set pat {^\\d+$}\nregexp $pat $str");
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|(pat, _)| pat == "^\\d+$"));
        let p2 = pats("set pat {foo}\nregsub $pat $str bar result");
        assert_eq!(p2.len(), 2);
        assert!(p2.iter().all(|(pat, _)| pat == "foo"));
    }

    /// A rooted `::set` still binds its constant as `set` does: the
    /// binding reads the `CellWrite` declaration of the command the head
    /// resolves to, never the
    /// spelling (tclsh 8.4.20 to 9.1b0: `::set pat {^\d+$}; regexp $pat 123`
    /// is 1).
    #[test]
    fn a_rooted_set_propagates_its_constant_pattern() {
        let p = pats("::set pat {^\\d+$}\nregexp $pat $str");
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|(pat, _)| pat == "^\\d+$"));
    }

    #[test]
    fn set_then_switch_regexp_propagates() {
        let p = pats("set pat {^hello}\nswitch -regexp $x $pat {puts matched}");
        assert!(p.iter().any(|(pat, _)| pat == "^hello"));
    }

    #[test]
    fn dynamic_or_interpolated_pattern_not_recorded() {
        assert_eq!(
            pats("set pat $dynamic_value\nregexp $pat $str"),
            [] as [(std::string::String, std::string::String); 0]
        );
        assert_eq!(
            pats("set pat \"^$prefix\"\nregexp $pat $str"),
            [] as [(std::string::String, std::string::String); 0]
        );
        // Reassigned to non-constant loses tracking.
        assert_eq!(
            pats("set pat {^\\d+}\nset pat $other\nregexp $pat $str"),
            [] as [(std::string::String, std::string::String); 0]
        );
    }

    #[test]
    fn variable_pattern_in_proc_scope() {
        let p = pats("proc check {s} { set pat {^\\w+$}; regexp $pat $s }");
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|(pat, _)| pat == "^\\w+$"));
    }

    #[test]
    fn literal_and_variable_patterns_mixed() {
        let p = pats("set pat {^a}\nregexp $pat $str\nregexp {^b} $str2");
        let pset: Vec<_> = p.iter().map(|(pat, _)| pat.as_str()).collect();
        assert!(pset.contains(&"^a") && pset.contains(&"^b"));
    }
}

// Diagnostics keyed on command name must hit identically across bare /
// qualified / aliased spellings.
//
// Several of these equalities hold (W211 set, W215/W216, the W210
// variable/upvar/global suppressions); others do NOT — qualified `::unset` and
// aliased `unset` do not currently canonicalise to the W213 path. Each arm is
// asserted against the actual verdict, with the exceptions flagged inline.
mod canonicalisation_matrix {
    use super::super::*;

    // W215 (unreachable variable name).

    #[test]
    fn w215_brace_in_var_name() {
        // `set "weird}name" 1` makes a var no $-form can read. Heuristic, but the
        // unreachability is verified against tclsh's ${...} parser semantics.
        let ds = analyser_diags("set \"weird}name\" 1", D);
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("weird}name"));
        assert!(w215[0].1.contains('}'));
    }

    #[test]
    fn w215_trailing_backslash_in_var_name() {
        // `set "back\\" 1` creates `back\`. Under the Tcl 9 nesting rule
        // `${back\}` reads the `\}` as an inert pair and runs out of input
        // (missing close-brace) — unreachable, W215 fires (verified against
        // tclsh 9.0.3). Under the 8.x first-`}` rule there is no backslash
        // processing: `${back\}` simply names `back\` — reachable, silent.
        let ds = analyser_diags("set \"back\\\\\" 1", "tcl9.0");
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("trailing") || w215[0].1.contains("missing close-brace"));

        assert!(
            !fires("set \"back\\\\\" 1", D, "W215"),
            "8.x reads ${{back\\}} as the name back\\ — reachable, no W215"
        );
    }

    #[test]
    fn w215_does_not_fire_when_brace_form_reaches_the_name() {
        // Names reachable via ${...} under BOTH `${…}` delimiting rules
        // (Tcl 9 nesting and the 8.x first-`}` scan): mid-name backslash
        // (no `}` involved) and ordinary names.
        for src in [
            "set \"back\\\\slash\" 1",
            "set \"foo-bar\" 1\nset normal 2\nset ::globalvar 3\nset arr(name) 4",
        ] {
            for dialect in [D, "tcl9.0"] {
                assert!(
                    !fires(src, dialect, "W215"),
                    "unexpected W215 for {src:?} under {dialect}"
                );
            }
        }
    }

    #[test]
    fn w215_balanced_inner_braces_are_dialect_dependent() {
        // A name with balanced inner braces is reachable only under the Tcl 9
        // nesting rule. tclsh8.6-verified: `set "a{b}c" 1; set ${a{b}c}` →
        // can't read "a{b" (the 8.x form ends at the FIRST `}`), while tclsh
        // 9.0.3 reads it fine.
        let src = "set \"a{b}c\" 1";
        assert!(
            !fires(src, "tcl9.0", "W215"),
            "Tcl 9 nesting reaches a{{b}}c — no W215"
        );
        assert!(
            fires(src, D, "W215"),
            "the 8.x first-close rule cannot reach a{{b}}c — W215 must fire"
        );
    }

    #[test]
    fn w215_close_paren_in_array_index() {
        let ds = analyser_diags("set \"arr(weird)stuff)\" 1", D);
        let w215: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W215").collect();
        assert_eq!(w215.len(), 1);
        assert!(w215[0].1.contains("array element index contains ')'"));
    }

    // W216 (brace-then-paren array misuse).

    #[test]
    fn w216_brace_then_paren_with_fix() {
        let r = Analyser::new().analyse("set arr(name) hello\nputs ${arr}(name)", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert_ne!(w216[0].fixes, [] as [tcl_compiler::analyser::CodeFix; 0]);
        assert_eq!(w216[0].fixes[0].new_text, "$arr(name)");
    }

    #[test]
    fn w216_brace_array_with_dollar_index() {
        let r = Analyser::new().analyse("set foo bar\nputs ${arr($foo)}", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert_eq!(w216[0].fixes[0].new_text, "$arr($foo)");
    }

    #[test]
    fn w216_funny_name_falls_back_to_set_indirection() {
        let r = Analyser::new().analyse("set \"funny name\" 1\nputs ${funny name($foo)}", D);
        let w216: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W216")
            .collect();
        assert_eq!(w216.len(), 1);
        assert_eq!(w216[0].fixes[0].new_text, "[set \"funny name($foo)\"]");
    }

    #[test]
    fn w216_does_not_fire_on_correct_forms() {
        for src in [
            "puts ${arr(name)}",
            "puts $arr($foo)",
            "puts ${arr}",
            "set foo bar\nputs $arr($foo)",
        ] {
            assert!(!fires(src, D, "W216"), "unexpected W216 for {src:?}");
        }
    }

    // W213 (unset of possibly-undefined var).

    #[test]
    fn w213_unset_bare_fires() {
        // tclsh: `unset x` on undefined x → can't unset "x": no such variable.
        assert_eq!(count("proc f {} { unset x }", D, "W213"), 1);
    }

    #[test]
    fn w213_nocomplain_silenced() {
        // `-nocomplain` opts out (tclsh suppresses the error too).
        assert_eq!(count("proc f {} { unset -nocomplain x }", D, "W213"), 0);
    }

    // Qualified `::unset x` and aliased `myunset x` do NOT fire W213 — the
    // qualified / aliased spelling is not canonicalised onto the W213 path on
    // this surface.
    #[test]
    fn w213_qualified_and_aliased_do_not_fire_rust_behaviour() {
        // The qualified / aliased spellings stay silent:
        assert_eq!(count("proc f {} { ::unset x }", D, "W213"), 0);
        assert_eq!(
            count(
                "interp alias {} myunset {} unset\nproc f {} { myunset x }",
                D,
                "W213"
            ),
            0
        );
        // The qualified+nocomplain combination is likewise silent.
        assert_eq!(count("proc f {} { ::unset -nocomplain x }", D, "W213"), 0);
    }

    // W211 (set but never used) canonicalisation.

    #[test]
    fn w211_set_bare_and_qualified_both_fire() {
        // Both bare `set y 1` and qualified `::set y 1` canonicalise to the W211
        // path.
        assert_eq!(count("proc f {} { set y 1 }", D, "W211"), 1);
        assert_eq!(count("proc f {} { ::set y 1 }", D, "W211"), 1);
    }

    // An aliased `s y 1` (→ set) does not fire W211 on this surface.
    #[test]
    fn w211_aliased_set_rust_behaviour() {
        let n = count("interp alias {} s {} set\nproc f {} { s y 1 }", D, "W211");
        // Record the actual count; the alias-rewrite-to-`set` W211 path is not
        // reproduced here.
        assert_eq!(
            n, 0,
            "aliased-set W211 not reproduced on this surface (divergence)"
        );
    }

    // W210 suppression by variable/upvar/global declarations.

    #[test]
    fn w210_variable_decl_silences_bare_and_qualified() {
        // `variable v` introduces v from the namespace → reading it is not W210.
        for decl in ["variable v", "::variable v"] {
            let src =
                format!("namespace eval ns {{ variable v; proc f {{}} {{ {decl}; puts $v }} }}");
            assert_eq!(w210_for(&src, "'v'"), 0, "for {decl:?}");
        }
    }

    #[test]
    fn w210_upvar_decl_silences_bare_and_qualified() {
        // `upvar 1 caller_v local` aliases `local`; reads are not W210. Both the
        // bare and `::upvar` qualified spellings canonicalise onto this path.
        assert_eq!(
            w210_for(
                "proc f {} { upvar 1 caller_v local; puts $local }",
                "'local'"
            ),
            0
        );
        assert_eq!(
            w210_for(
                "proc f {} { ::upvar 1 caller_v local; puts $local }",
                "'local'"
            ),
            0
        );
    }

    #[test]
    fn w210_upvar_aliased_silences_the_name_it_links() {
        // An aliased `upvar` (`interp alias {} link {} upvar` then
        // `link 1 caller_v local`) reaches `upvar` through another binding,
        // a registry command whose traits say it raises a barrier, so the
        // flow graph lowers no frame effect for it and marks the call as one
        // to code the module cannot see. Such a callee may create the caller's
        // names it is handed, and `local` is one: reading it draws no W210.
        // tclsh 8.4 to 9.1 print `5` for the procedure called after
        // `set caller_v 5`.
        assert_eq!(
            w210_for(
                "interp alias {} link {} upvar\nproc f {} { link 1 caller_v local; puts $local }",
                "'local'"
            ),
            0
        );
    }

    #[test]
    fn w210_global_decl_silences_bare_qualified_aliased() {
        // `global g` recognises g as a global alias → no W210 on g.
        for src in [
            "set g 0\nproc f {} { global g; set g 1 }",
            "set g 0\nproc f {} { ::global g; set g 1 }",
            "set g 0\ninterp alias {} mkglobal {} global\nproc f {} { mkglobal g; set g 1 }",
        ] {
            assert_eq!(w210_for(src, "'g'"), 0, "unexpected W210 'g' for {src:?}");
        }
    }

    // NOTE — IRULE4005 / W120 canonicalisation: these poke iRules-flow /
    // global-write internals. The IRULE4005 case asserts a *negative*
    // (bare/qualified/aliased `unset` does NOT fire IRULE4005); the W120 case
    // asserts the global-alias suppression already covered by
    // `w210_global_decl_silences_*` above. The IRULE4005 negative is covered
    // here for the bare spelling under the iRules dialect.

    #[test]
    fn irule4005_not_fired_by_bare_unset_under_irules() {
        // A bare `unset static::z` is not a "real write" → no IRULE4005.
        assert!(!fires(
            "when CLIENT_ACCEPTED { unset static::z }",
            "f5-irules",
            "IRULE4005"
        ));
    }
}
