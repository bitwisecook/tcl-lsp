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

//! Redundancy and motion: legacy GVN/CSE detection, O125 code sinking and O127 load forwarding.

use super::*;

/// Full-redundancy codes from the explicitly legacy CFG/SSA API.
///
/// This helper exercises the value-numbering algorithm's keys and kill sets.
/// It deliberately does not stand in for production legality: the production
/// `FunctionUnit` path fails closed unless an occurrence has exact
/// common-semantic registry facts, which has separate boundary tests in `gvn`.
fn legacy_gvn_codes(src: &str) -> Vec<String> {
    let registry = static_context_for(TCL).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for function in cu.analysable_functions() {
        for redundancy in find_redundancies(
            registry,
            &function.cfg,
            &function.ssa,
            Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
        ) {
            v.push(redundancy.code.as_str().to_owned());
        }
    }
    v.sort();
    v
}

// GVN / CSE — explicit legacy algorithm coverage (O105)

#[test]
fn legacy_gvn_cse_detection() {
    // A repeated pure call ([string length $x]) is detected (O105). (Structural:
    // the second computation is provably redundant.)
    let dup = "set x hello\nset a [string length $x]\nset b [string length $x]";
    assert!(legacy_gvn_codes(dup).iter().any(|c| c == "O105"));
    assert_ne!(legacy_gvn_codes(dup), [] as [std::string::String; 0]);

    // Different arguments ⇒ not flagged.
    assert_eq!(
        legacy_gvn_codes("set a [string length hello]\nset b [string length world]"),
        Vec::<String>::new()
    );

    // Three identical ⇒ two redundancy warnings. `$x` is bound first: tclsh
    // errors on reading an unset variable.
    let three = "set x [gets stdin]\nset a [string length $x]\nset b [string length $x]\n\
                 set c [string length $x]";
    assert!(legacy_gvn_codes(three).len() >= 2);

    // A mutation between the calls invalidates the redundancy.
    assert_eq!(
        legacy_gvn_codes(
            "set x [gets stdin]\nset a [string length $x]\nset x different\n\
             set b [string length $x]"
        ),
        Vec::<String>::new()
    );

    // Precision note (NOT a miscompile): this explicit legacy finder can flag
    // repeated `[clock seconds]` from its command-string classifier even though
    // the call is impure. Production GVN cannot inherit that fact: it requires
    // exact common-semantic legality and fails closed. The applied optimiser
    // likewise leaves both calls intact, which is the sound end-to-end
    // invariant asserted here.
    let clk = "set a [clock seconds]\nset b [clock seconds]";
    assert!(opt_absent(clk, TCL, "O105"));
    assert!(optimised(clk, TCL).contains("set a [clock seconds]"));
    assert!(optimised(clk, TCL).contains("set b [clock seconds]"));
}

// O125 — code sinking

#[test]
fn o125_code_sinking() {
    // Basic sink into an `if` whose condition does not read the var. Either O125
    // or O100 is accepted; here O125 fires. tclsh: `set b foo; if {$a} {puts $b}`
    // and the sunk form both print foo iff $a is true.
    let basic = "set a [llength $argv]\nset b foo\nif {$a} {\n    puts $b\n}";
    assert!(opt_any_of(basic, TCL, &["O125", "O100"]));

    // O125 must NOT fire when the var is read in the condition or its RHS is a
    // command substitution.
    assert!(opt_absent(
        "set b $x\nif {$b} {\n    puts hello\n}",
        TCL,
        "O125"
    )); // var in condition
    assert!(opt_absent(
        "set b [clock seconds]\nif {$a} {\n    puts $b\n}",
        TCL,
        "O125"
    )); // cmd-sub RHS

    // Coexists with O120 (string-compare rewrite in the condition).
    let with120 = "set kind [lindex $argv 0]\nset b foo\nif {$kind == \"x\"} {\n    puts $b\n}";
    assert!(opt_fires(with120, TCL, "O120"));
    assert!(optimised(with120, TCL).contains("$kind eq \"x\""));
}

// O127 — single-use store-to-load forwarding (proc-local)

#[test]
fn o127_load_forwarding() {
    // Single-use proc-local with a command substitution is inlined. tclsh: the
    // inlined `[set x [clock seconds]]` assigns x and yields its value, identical
    // to `set x [clock seconds]; puts $x`.
    let cmdsub = "proc f {} { set x [clock seconds]\n puts $x }";
    assert!(opt_fires(cmdsub, TCL, "O127"));
    assert!(optimised(cmdsub, TCL).contains("puts [set x [clock seconds]]"));

    // A constant is handled by propagation/DSE, NOT O127. tclsh: x=42 ⇒ puts 42.
    let constv = "proc f {} { set x 42\n puts $x }";
    assert!(opt_absent(constv, TCL, "O127"));
    assert!(opt_fires(constv, TCL, "O102"));
    assert!(optimised(constv, TCL).contains("puts 42"));

    // More-than-once use ⇒ not inlined.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n puts $x\n puts $x }",
        TCL,
        "O127"
    ));
    // Top-level variables ⇒ not inlined.
    assert!(opt_absent("set x [clock seconds]\nputs $x", TCL, "O127"));
    // A barrier (info exists between) blocks forwarding.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n info exists x\n puts $x }",
        TCL,
        "O127"
    ));
    // upvar/global aliasing blocks forwarding.
    assert!(opt_absent(
        "proc f {} { upvar 1 ext x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    assert!(opt_absent(
        "proc f {} { global x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    // A phi merge (two defining branches) blocks forwarding.
    assert!(opt_absent(
        "proc f {c} { if {$c} { set x [clock seconds] } else { set x [clock clicks] }\n puts $x }",
        TCL,
        "O127"
    ));

    // NOTE on gaps (sound): the var-copy form `set x $arg; puts $x` is left
    // unchanged rather than inlined (O127); an intervening empty `eval {}` barrier
    // is not honoured.
}
