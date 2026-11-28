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

//! Removing decided structure: `if`, `while`, `for` and `switch`, nested through multipass, with what must survive the rewrite.

use super::*;

#[test]
fn structure_elimination_if_while_for() {
    crate::install_surfaces();
    // tclsh: `if {1} {set x 1}` runs the body ⇒ unwrap to `set x 1`.
    let if_true = "if {1} {\n    set x 1\n}";
    let it = optimised(if_true, TCL);
    assert!(!it.contains("if"));
    assert!(it.contains("set x 1"));
    assert!(opt_fires(if_true, TCL, "O112"));

    // `if {0}` deletes its body; trailing `puts always` survives.
    let if_false = "if {0} {\n    set x 1\n}\nputs always";
    let iff = optimised(if_false, TCL);
    assert!(!iff.contains("set x 1"));
    assert!(iff.contains("puts always"));
    assert!(opt_fires(if_false, TCL, "O112"));

    // `if {0} ... else ...` keeps the else body. tclsh: else branch runs.
    let if_else = "if {0} {\n    set x 1\n} else {\n    set y 2\n}";
    let ie = optimised(if_else, TCL);
    assert!(!ie.contains("set x 1"));
    assert!(ie.contains("set y 2"));
    assert!(!ie.contains("if"));
    assert!(opt_fires(if_else, TCL, "O112"));

    // elseif chain finds the first true clause. tclsh: `set b 2` runs.
    let elif = "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}";
    let el = optimised(elif, TCL);
    assert!(!el.contains("set a 1"));
    assert!(el.contains("set b 2"));
    assert!(!el.contains("set c 3"));
    assert!(opt_fires(elif, TCL, "O112"));

    // `while {0}` deletes the loop; `for {init} {0} ...` keeps only init.
    let wf = "while {0} {\n    puts looping\n}\nputs done";
    assert!(!optimised(wf, TCL).contains("puts looping"));
    assert!(optimised(wf, TCL).contains("puts done"));
    assert!(opt_fires(wf, TCL, "O112"));
    // A following statement: a sole `for` is the script's result (`""`),
    // which `set i 0` would change to 0 (tclsh 8.4-9.1: `{} 0`).
    let ff = "for {set i 0} {0} {incr i} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ff, TCL).contains("puts looping"));
    assert!(optimised(ff, TCL).contains("set i 0"));
    assert!(opt_fires(ff, TCL, "O112"));
    let ffe = "for {} {0} {} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ffe, TCL).contains("puts looping"));
    assert!(optimised(ffe, TCL).contains("puts done"));
    assert!(opt_fires(ffe, TCL, "O112"));

    // Runtime condition `if {$x}` is untouched.
    assert!(!opt_fires(
        &in_proc("if {$x} {\n    set y 1\n}"),
        TCL,
        "O112"
    ));
}

#[test]
fn structure_elimination_switch() {
    crate::install_surfaces();
    // tclsh: switch abc {abc {1} def {2}} ⇒ first arm.
    let lit = "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}";
    let l = optimised(lit, TCL);
    assert!(l.contains("set x 1"));
    assert!(!l.contains("set y 2"));
    assert!(!l.contains("switch"));
    assert!(opt_fires(lit, TCL, "O112"));

    // No literal match → default arm. tclsh: switch xyz {...default {2}} ⇒ 2.
    let def = "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}";
    assert!(!optimised(def, TCL).contains("set a 1"));
    assert!(optimised(def, TCL).contains("set b 2"));
    assert!(opt_fires(def, TCL, "O112"));

    // No match, no default → whole switch deleted.
    let nomatch = "switch xyz {\n    abc { set a 1 }\n}\nputs done";
    assert!(!optimised(nomatch, TCL).contains("set a 1"));
    assert!(optimised(nomatch, TCL).contains("puts done"));
    assert!(opt_fires(nomatch, TCL, "O112"));

    // -glob: aaab matches `a*b` (first arm). tclsh-verified ⇒ arm 1.
    let glob = "switch -glob aaab {\n    a*b { set x 1 }\n    b { set y 2 }\n    a* { set z 3 }\n    default { set w 4 }\n}";
    let g = optimised(glob, TCL);
    assert!(g.contains("set x 1"));
    assert!(!g.contains("set y 2") && !g.contains("set z 3") && !g.contains("set w 4"));
    assert!(!g.contains("switch"));
    assert!(opt_fires(glob, TCL, "O112"));

    // -glob no match → default.
    let gdef =
        "switch -glob xyz {\n    a* { set x 1 }\n    b* { set y 2 }\n    default { set z 3 }\n}";
    assert!(!optimised(gdef, TCL).contains("set x 1"));
    assert!(!optimised(gdef, TCL).contains("set y 2"));
    assert!(optimised(gdef, TCL).contains("set z 3"));
    assert!(opt_fires(gdef, TCL, "O112"));

    // -regexp folds through the regexp engine: `^a` matches abc (first arm),
    // `^b` does not (default). tclsh 8.6: arm 1 and arm 2 respectively.
    let re = "switch -regexp abc {\n    ^a { set x 1 }\n    default { set y 2 }\n}";
    assert!(optimised(re, TCL).contains("set x 1"));
    assert!(!optimised(re, TCL).contains("set y 2"));
    assert!(!optimised(re, TCL).contains("switch"));
    assert!(opt_fires(re, TCL, "O112"));
    let rdef = "switch -regexp abc {\n    ^b { set x 1 }\n    default { set y 2 }\n}";
    assert!(!optimised(rdef, TCL).contains("set x 1"));
    assert!(optimised(rdef, TCL).contains("set y 2"));

    // -glob fallthrough (`a* -` then `z* {body}`) selects the next body.
    // tclsh: switch -glob abc {a* - z* {1} default {2}} ⇒ 1.
    let ft = "switch -glob abc {\n    a* -\n    z* { set x 1 }\n    default { set y 2 }\n}";
    assert!(optimised(ft, TCL).contains("set x 1"));
    assert!(!optimised(ft, TCL).contains("set y 2"));
    assert!(opt_fires(ft, TCL, "O112"));

    // Fallthrough chain to default. tclsh: switch abc {abc - def - default {99}} ⇒ 99.
    let ftc = "switch abc {\n    abc -\n    def -\n    default { set x 1 }\n}";
    assert!(optimised(ftc, TCL).contains("set x 1"));
    assert!(!optimised(ftc, TCL).contains("switch"));
    assert!(opt_fires(ftc, TCL, "O112"));

    // -nocase: ABC matches abc. tclsh: switch -nocase ABC {abc {1} default {2}} ⇒ 1.
    let nc = "switch -nocase ABC {\n    abc { set x 1 }\n    default { set y 2 }\n}";
    assert!(optimised(nc, TCL).contains("set x 1"));
    assert!(!optimised(nc, TCL).contains("set y 2"));
    assert!(opt_fires(nc, TCL, "O112"));
}

#[test]
fn structure_elimination_nesting_via_multipass() {
    crate::install_surfaces();
    // First pass unwraps `if {1}` body, leaving inner `if {0}`; a second pass
    // eliminates the inner dead block. Use the fixpoint helper for the 2nd pass.
    let nested = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    let pass1 = optimised(nested, TCL);
    assert!(pass1.contains("set alive 2"));
    assert!(opt_count(nested, TCL, "O112") >= 1);
    let registry = static_context_for(TCL).commands();
    let (fixed, _) = optimise_source_multipass(
        nested,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        10,
    );
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// Constant var-ref propagation — O100 / O105 (string interpolation)

#[test]
fn branch_condition_ending_in_a_nested_empty_pair_rewrites_the_whole_word() {
    crate::install_surfaces();
    // The branch condition span is the lexer's word span, so
    // it stops one byte short of the outer `}`. Deciding the widening from
    // the slice's last byte would read `{$n == 0 && $y eq {}` as already
    // whole — it does end in a `}`, the *inner* empty pair's — so naively
    // the pass would unwrap an opener with no matching closer and emit the
    // unbalanced replacement `{0 == 0 && $y eq {}`.
    let src = "proc p {y} {\n    set n 0\n    if {$n == 0 && $y eq {}} { puts a }\n}\n";
    let rewrites = opt_rewrites(src, TCL);
    let o100: Vec<&str> = rewrites
        .iter()
        .filter(|(code, _)| code == "O100")
        .map(|(_, replacement)| replacement.as_str())
        .collect();
    assert_eq!(o100, vec!["{0 == 0 && $y eq {}}"], "{rewrites:?}");
    assert_eq!(
        optimised(src, TCL),
        "proc p {y} {\n    set n 0\n    if {0 == 0 && $y eq {}} { puts a }\n}\n",
    );
}

// Profile directive / multipass — profile survival + string-build collapse

#[test]
fn profile_directive_survives_structure_elimination() {
    crate::install_surfaces();
    // The `# profiles: HTTP2` comment must survive optimisation (it drives later
    // HTTP2 hints), and the constant `if {1}` inside the event unwraps (O112).
    let src =
        "# profiles: HTTP2\nwhen HTTP_REQUEST {\n    if {1} {\n        HTTP2::active\n    }\n}\n";
    let out = optimised(src, "f5-irules");
    assert!(out.contains("# profiles: HTTP2"));
    assert!(opt_fires(src, "f5-irules", "O112"));
    // The unwrapped body retains the HTTP2 call.
    assert!(out.contains("HTTP2::active"));
}
