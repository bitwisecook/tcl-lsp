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

//! W218 misplaced `args` and W314 colon-named definitions.

use super::*;

#[test]
fn w218_args_in_non_final_position_fires() {
    // TP: the classic pitfall — `args` before another parameter is an
    // ordinary parameter (C Tcl sets VAR_IS_ARGS only on the last formal).
    let mut a = crate::analyser::Analyser::new();
    let src = "proc p {args extra} { puts $extra }
";
    let r = a.analyse(src, "tcl8.6");
    let w218: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W218)
        .collect();
    assert_eq!(w218.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.find("args").unwrap();
    assert_eq!(
        w218[0].span.start() as usize,
        expected,
        "anchor at the args parameter name"
    );
}

#[test]
fn w218_silent_for_final_args_and_plain_params() {
    // TN: `args` in final position is the variadic idiom; no `args` at all
    // is trivially fine; a lone `args` IS final.
    for src in [
        "proc p {a args} { puts $a }
",
        "proc p {a b} { puts $a$b }
",
        "proc p {args} { llength $args }
",
    ] {
        assert!(
            !codes_for_dialect(src, "tcl8.6").contains(&"W218".to_string()),
            "must be silent for {src:?}"
        );
    }
}

#[test]
fn w218_fires_for_method_and_apply_params() {
    // TP: the same pitfall inside a TclOO method parameter list and an
    // apply lambda.
    let method_src = "oo::class create C {
    method m {args other} { puts $other }
}
";
    assert!(
        codes_for_dialect(method_src, "tcl8.6").contains(&"W218".to_string()),
        "method params must be checked"
    );
    let apply_src = "apply {{args extra} { puts $extra }} 1 2
";
    assert!(
        codes_for_dialect(apply_src, "tcl8.6").contains(&"W218".to_string()),
        "apply lambda params must be checked"
    );
}

// Colon-named definitions.

/// The count of W314 diagnostics `src` draws under the plain `tcl` dialect.
fn w314_count(src: &str) -> usize {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W314)
        .count()
}

#[test]
fn w314_fires_for_colon_named_proc_class_and_namespace() {
    // TP: an all-colon simple name has no absolute written form.
    assert_eq!(w314_count("proc : args { return hello }\n"), 1);
    assert_eq!(w314_count("oo::class create :\n"), 1);
    assert_eq!(w314_count("namespace eval : { }\n"), 1);
    // ... inside a namespace, still only reachable relatively.
    assert_eq!(w314_count("namespace eval x { proc : {} {} }\n"), 1);
}

#[test]
fn w314_does_not_fire_for_addressable_colon_shapes() {
    // A lone interior colon is an ordinary name character.
    assert_eq!(w314_count("proc a:b {} {}\n"), 0);
    assert_eq!(w314_count("proc :2 {} {}\n"), 0);
    // The empty-string name IS absolutely addressable (`::` / `::x::`).
    assert_eq!(w314_count("proc {} args {}\n"), 0);
    assert_eq!(w314_count("namespace eval x {}\nproc x:: {} {}\n"), 0);
    // A written all-colon word names the global `{}` proc — addressable.
    assert_eq!(w314_count("proc ::: args {}\n"), 0);
    // Ordinary namespaces never fire.
    assert_eq!(w314_count("namespace eval a::b { proc p {} {} }\n"), 0);
}

#[test]
fn w314_namespace_flags_once_without_per_definition_cascade() {
    // The namespace-eval site carries the warning; an addressable simple
    // name inside it does not re-fire per definition.
    assert_eq!(w314_count("namespace eval : { proc helper {} {} }\n"), 1);
    // A colon-named proc *inside* a colon namespace is its own defect too.
    assert_eq!(w314_count("namespace eval : { proc : {} {} }\n"), 2);
}

#[test]
fn colon_named_proc_keeps_its_simple_name_and_key() {
    // `proc :` must keep its simple name: collapsing the key into `::` and
    // re-deriving the tail with a naive `rsplit` leaves the simple name (the
    // documentSymbol name) empty.
    let mut a = Analyser::new();
    let r = a.analyse("proc : args { return hello }\n", "tcl");
    let (key, def) = r
        .all_procs
        .iter()
        .find(|(_, p)| p.name_span.start() == 5)
        .expect("proc : recorded");
    assert_eq!(key, ":::", "constructed key is \"::\" + \":\"");
    assert_eq!(def.name, ":", "simple name preserved");
    assert_eq!(def.qualified_name, ":::");
}

#[test]
fn colon_named_namespace_keeps_nested_definitions_distinct() {
    // `namespace eval : { proc inner {} {} }` must home `inner` under the
    // `:`-named namespace — never collapse into the global namespace.
    let mut a = Analyser::new();
    let r = a.analyse(
        "namespace eval : { proc inner {} {} }\nproc inner {} {}\n",
        "tcl",
    );
    assert!(
        r.all_procs.contains_key(":::::inner"),
        "nested key = ns key \":::\" + \"::\" + \"inner\": {:?}",
        r.all_procs.keys().collect::<Vec<_>>(),
    );
    assert!(
        r.all_procs.contains_key("::inner"),
        "the global `inner` is a distinct definition"
    );
}

#[test]
fn colon_named_proc_resolves_from_bare_calls_not_written_runs() {
    // Construction symmetry: a bare `:` call settles onto the `:` proc; a
    // written `:::` names the global `{}` command instead (tclsh-pinned).
    let mut a = Analyser::new();
    let r = a.analyse("proc : args {}\nproc {} args {}\n: x\n::: y\n", "tcl");
    let resolved: Vec<(&str, Option<&str>)> = r
        .command_invocations
        .iter()
        .filter(|i| i.name == ":" || i.name == ":::")
        .map(|i| (i.name.as_str(), i.resolved_qualified_name.as_deref()))
        .collect();
    assert!(
        resolved.contains(&(":", Some(":::"))),
        "bare `:` settles onto the colon proc: {resolved:?}"
    );
    assert!(
        resolved.contains(&(":::", Some("::"))),
        "written `:::` settles onto the `{{}}` proc: {resolved:?}"
    );
}
