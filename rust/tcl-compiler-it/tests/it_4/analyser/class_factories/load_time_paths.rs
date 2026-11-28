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

//! Which creations the load-time path reaches: selected `if`, `switch` and `try` arms and prefixes that must fall through.

use super::*;

#[test]
fn computed_creation_materialises_only_the_selected_if_arm() {
    crate::install_surfaces();
    let source = |enabled: &str| {
        format!(
            "namespace eval ::T {{}}\n\
                 oo::class create ::T::Mother {{ superclass oo::class }}\n\
                 proc ::T::mk {{name enabled}} {{\n\
                     if {{$enabled}} {{ ::T::Mother create ${{name}}::class {{}} }}\n\
                 }}\n\
                 ::T::mk ::T::D {enabled}\n"
        )
    };
    assert!(
        analysis(&source("1"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
    assert!(
        !analysis(&source("0"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
    assert!(
        !analysis(&source("$runtime"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
}

#[test]
fn computed_creation_requires_the_load_time_call_path_to_execute() {
    crate::install_surfaces();
    let source = |condition: &str| {
        format!(
            "namespace eval ::T {{}}\n\
                 oo::class create ::T::Mother {{ superclass oo::class }}\n\
                 proc ::T::mk {{name}} {{\n\
                     ::T::Mother create ${{name}}::class {{}}\n\
                 }}\n\
                 if {{{condition}}} {{ ::T::mk ::T::Ghost }}\n"
        )
    };
    assert!(
        !analysis(&source("0"), "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
    assert!(
        analysis(&source("1"), "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
    assert!(
        !analysis(&source("$runtime"), "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn load_time_call_requires_every_dominating_prefix_to_fall_through() {
    crate::install_surfaces();
    let base = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} { ::T::Mother create ${name}::class {} }\n",
    );
    let top = format!("{base}error stopped\n::T::mk ::T::TopGhost\n");
    assert!(
        !analysis(&top, "tcl9.0")
            .all_classes
            .contains_key("::T::TopGhost::class")
    );

    let arm = format!("{base}if {{1}} {{\n  error stopped\n  ::T::mk ::T::ArmGhost\n}}\n");
    assert!(
        !analysis(&arm, "tcl9.0")
            .all_classes
            .contains_key("::T::ArmGhost::class")
    );

    let live = format!("{base}if {{1}} {{ set x ok; ::T::mk ::T::Live }}\n");
    assert!(
        analysis(&live, "tcl9.0")
            .all_classes
            .contains_key("::T::Live::class")
    );

    let proc_error = format!(
        "{base}proc ::T::stop {{}} {{ error stopped }}\n::T::stop\n::T::mk ::T::ProcGhost\n"
    );
    assert!(
        !analysis(&proc_error, "tcl9.0")
            .all_classes
            .contains_key("::T::ProcGhost::class"),
        "a prior user procedure with non-normal completion blocks the later call"
    );
}

#[test]
fn load_time_call_requires_nested_control_to_reach_its_end() {
    crate::install_surfaces();
    let base = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} { ::T::Mother create ${name}::class {} }\n",
    );
    for prefix in [
        "if {1} { error stopped }",
        "switch yes { yes { error stopped } }",
        "try {} finally { error stopped }",
        "while {0} {}",
        "notARegisteredCommand",
    ] {
        let src = format!("{base}{prefix}\n::T::mk ::T::Ghost\n");
        assert!(
            !analysis(&src, "tcl9.0")
                .all_classes
                .contains_key("::T::Ghost::class"),
            "prefix must abstain: {prefix}"
        );
    }

    let live = format!("{base}if {{0}} {{ error stopped }}\n::T::mk ::T::Live\n");
    assert!(
        analysis(&live, "tcl9.0")
            .all_classes
            .contains_key("::T::Live::class")
    );
}

#[test]
fn malformed_and_handler_dependent_prefixes_do_not_publish_factories() {
    crate::install_surfaces();
    let base = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} { ::T::Mother create ${name}::class {} }\n",
    );
    for prefix in [
        "set",
        "if {1}",
        "if {1} then",
        "if {0} {} else \"error stopped\"",
        "switch yes { yes \"error stopped\" }",
        "try {} finally \"error stopped\"",
        "try {} on ok {} { error stopped }",
    ] {
        let src = format!("{base}{prefix}\n::T::mk ::T::Ghost\n");
        assert!(
            !analysis(&src, "tcl9.0")
                .all_classes
                .contains_key("::T::Ghost::class"),
            "prefix must abstain: {prefix}"
        );
    }
}

#[test]
fn load_time_switch_and_try_paths_select_only_executed_factory_calls() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} { ::T::Mother create ${name}::class {} }\n",
        "switch no {\n",
        "    yes { ::T::mk ::T::SwitchGhost }\n",
        "    default { ::T::mk ::T::SwitchReal }\n",
        "}\n",
        "try {} on error {message options} {\n",
        "    ::T::mk ::T::HandlerGhost\n",
        "} finally {\n",
        "    ::T::mk ::T::FinallyReal\n",
        "}\n",
    );
    let result = analysis(src, "tcl9.0");
    assert!(result.all_classes.contains_key("::T::SwitchReal::class"));
    assert!(result.all_classes.contains_key("::T::FinallyReal::class"));
    assert!(!result.all_classes.contains_key("::T::SwitchGhost::class"));
    assert!(!result.all_classes.contains_key("::T::HandlerGhost::class"));
}

#[test]
fn computed_creation_after_a_non_normal_completion_is_unreachable() {
    crate::install_surfaces();
    for terminator in [
        "error stopped",
        "throw {TEST STOP} stopped",
        "break",
        "continue",
    ] {
        let src = format!(
            "namespace eval ::T {{}}\n\
                 oo::class create ::T::Mother {{ superclass oo::class }}\n\
                 proc ::T::mk {{name}} {{\n\
                     {terminator}\n\
                     ::T::Mother create ${{name}}::class {{}}\n\
                 }}\n\
                 ::T::mk ::T::Ghost\n"
        );
        assert!(
            !analysis(&src, "tcl9.0")
                .all_classes
                .contains_key("::T::Ghost::class"),
            "{terminator} must stop provenance"
        );
    }
}

#[test]
fn computed_creation_materialises_only_the_selected_switch_arm() {
    crate::install_surfaces();
    let source = |selector: &str| {
        format!(
            "namespace eval ::T {{}}\n\
                 oo::class create ::T::Mother {{ superclass oo::class }}\n\
                 proc ::T::mk {{name selector}} {{\n\
                     switch $selector {{\n\
                         yes {{ ::T::Mother create ${{name}}::class {{}} }}\n\
                         default {{ set unrelated no }}\n\
                     }}\n\
                 }}\n\
                 ::T::mk ::T::D {selector}\n"
        )
    };
    assert!(
        analysis(&source("yes"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
    assert!(
        !analysis(&source("no"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
    assert!(
        !analysis(&source("$runtime"), "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
}

#[test]
fn computed_creation_switch_supports_registry_modes_and_fallthrough() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name selector} {\n",
        "    switch -glob -- $selector {\n",
        "        D* -\n",
        "        fallback { ::T::Mother create ${name}::class {} }\n",
        "        default {}\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::D Dialect\n",
    );
    assert!(
        analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );

    let exact = src.replace("switch -glob", "switch -exact");
    assert!(
        !analysis(&exact, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );

    let nocase = src
        .replace("switch -glob", "switch -glob -nocase")
        .replace("::T::mk ::T::D Dialect", "::T::mk ::T::D dialect");
    assert!(
        analysis(&nocase, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );

    // `D*` as a regular expression matches `Dialect` (zero or more `D`
    // from the start), so the regexp selection falls through into the
    // creating body; tclsh 9.0 makes the class.
    let regexp = src.replace("switch -glob", "switch -regexp");
    assert!(
        analysis(&regexp, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "the regexp selection reaches the creating body"
    );
    // A pattern the regexp engine rejects is a selection nothing makes.
    let malformed = src
        .replace("switch -glob", "switch -regexp")
        .replace("D* -", "{(} -");
    assert!(
        !analysis(&malformed, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class"),
        "a malformed pattern must abstain"
    );
}

/// A quoted `-` body of the separate-words form reads two ways on 9.1b0
/// (its byte-compiled `switch` runs it as a command), so the selection
/// declines there and the class is not made; 9.0 reads it by value and
/// falls through into the creating body, and the bare spelling reads
/// alike everywhere.
#[test]
fn computed_creation_declines_a_delimited_fallthrough_body_on_91() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name selector} {\n",
        "    switch -glob -- $selector D* \"-\" fallback {\n",
        "        ::T::Mother create ${name}::class {}\n",
        "    } default {}\n",
        "}\n",
        "::T::mk ::T::D Dialect\n",
    );
    let made = |source: &str, dialect: &str| {
        analysis(source, dialect)
            .all_classes
            .contains_key("::T::D::class")
    };
    assert!(made(src, "tcl9.0"));
    assert!(!made(src, "tcl9.1"));
    let bare = src.replace("\"-\"", "-");
    assert!(made(&bare, "tcl9.0"));
    assert!(made(&bare, "tcl9.1"));
}

#[test]
fn switch_default_is_special_only_in_the_final_clause() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name selector} {\n",
        "    switch $selector {\n",
        "        default { ::T::Mother create ${name}::class {} }\n",
        "        yes {}\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::Ghost no\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn switch_with_an_orphan_braced_clause_never_materialises_a_class() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name selector} {\n",
        "    switch $selector {\n",
        "        yes { ::T::Mother create ${name}::class {} }\n",
        "        orphan\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::Ghost yes\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn switch_with_a_final_fallthrough_body_never_materialises_a_class() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name selector} {\n",
        "    switch $selector {\n",
        "        yes { ::T::Mother create ${name}::class {} }\n",
        "        default -\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::Ghost yes\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn computed_creation_reuses_nested_prefix_reachability() {
    crate::install_surfaces();
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} {\n",
        "    if {1} { error stopped }\n",
        "    ::T::Mother create ${name}::class {}\n",
        "}\n",
        "::T::mk ::T::Ghost\n",
    );
    assert!(
        !analysis(src, "tcl9.0")
            .all_classes
            .contains_key("::T::Ghost::class")
    );
}

#[test]
fn computed_creation_in_try_finally_is_selected_but_handler_abstains() {
    crate::install_surfaces();
    let finally_src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} {\n",
        "    try {} finally { ::T::Mother create ${name}::class {} }\n",
        "}\n",
        "::T::mk ::T::D\n",
    );
    assert!(
        analysis(finally_src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );

    let handler_src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::Mother { superclass oo::class }\n",
        "proc ::T::mk {name} {\n",
        "    try {} on error {message options} {\n",
        "        ::T::Mother create ${name}::class {}\n",
        "    }\n",
        "}\n",
        "::T::mk ::T::D\n",
    );
    assert!(
        !analysis(handler_src, "tcl9.0")
            .all_classes
            .contains_key("::T::D::class")
    );
}
