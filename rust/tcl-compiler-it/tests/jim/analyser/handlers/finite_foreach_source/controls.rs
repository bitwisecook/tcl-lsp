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

fn analyser() -> Analyser {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs");
    Analyser::new()
}

#[test]
fn literal_jim_iterations_keep_readonly_list_lineage_and_source_names() {
    let source = "foreach name {a b} {proc $name {} {{n 0}} {return $n}}";
    let result = analyser().analyse(source, "jim");
    for name in ["a", "b"] {
        let procedure = result.all_procs.values().find(|p| p.name == name).unwrap();
        assert_eq!(&source[procedure.body_span.as_range()], "{return $n}");
    }
    let read = u32::try_from(source.find("$name").unwrap()).unwrap();
    assert!(
        result.global_scope.variables["name"]
            .references
            .iter()
            .any(|span| span.start() >= read && span.end() <= read + 5)
    );
}

#[test]
fn iteration_names_refuse_equal_foreign_owner_changed_source_and_namespace() {
    let mut analyser = analyser();
    for (source, expected) in [
        (
            "foreach name {a b} {proc $name {} {return VALUE}}",
            ["::a", "::b"],
        ),
        (
            "namespace eval n {foreach name {c d} {proc $name {} {return VALUE}}}",
            ["::n::c", "::n::d"],
        ),
        (
            "foreach name {a b} {proc $name {} {return VALUE}}",
            ["::a", "::b"],
        ),
    ] {
        let result = analyser.analyse(source, "jim");
        let mut names = result
            .all_procs
            .values()
            .map(|p| p.qualified_name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, expected, "{source}");
        for procedure in result.all_procs.values() {
            assert_eq!(&source[procedure.body_span.as_range()], "{return VALUE}");
        }
    }
}

#[test]
fn iteration_plan_refuses_mutation_multiple_bindings_expansion_and_name_prefixes() {
    for (source, forbidden) in [
        (
            "foreach name {a b} {set name changed; proc $name {} {return VALUE}}",
            vec!["a", "b"],
        ),
        (
            "foreach {name other} {a b} {proc $name {} {return VALUE}}",
            vec!["b"],
        ),
        (
            "foreach name {*}$values {proc $name {} {return VALUE}}",
            vec!["a", "b"],
        ),
        (
            "foreach name {a b} {proc prefix$name {} {return VALUE}}",
            vec!["a", "b"],
        ),
    ] {
        let result = analyser().analyse(source, "jim");
        assert!(
            result
                .all_procs
                .values()
                .all(|p| !forbidden.contains(&p.name.as_str())),
            "{source}"
        );
    }
    let source = "foreach name {a b} {proc $name {} {return VALUE}; proc fixed {} {}}";
    let result = analyser().analyse(source, "jim");
    let fixed = result
        .all_procs
        .values()
        .find(|p| p.name == "fixed")
        .unwrap();
    assert_eq!(
        &source[tcl_lexer::word_span_at(source, fixed.body_span).as_range()],
        "{}"
    );
}
