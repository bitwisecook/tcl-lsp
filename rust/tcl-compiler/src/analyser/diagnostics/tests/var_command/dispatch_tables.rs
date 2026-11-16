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

//! References recorded for the procedure names in dispatch-table literals and dictionaries.

use super::*;

#[test]
fn dispatch_table_value_becomes_a_reference_when_consumed_m7() {
    let mut a = Analyser::new();
    let src = "proc do_add {a b} {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let r = a.analyse(src, "tcl8.6");
    let lit = u32::try_from(src.find("add do_add").unwrap() + 4).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.name == "do_add"
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::do_add")
            && !i.indirect),
        "table value literal must reference ::do_add: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_table_value_abstains_when_the_table_is_not_consumed_m7() {
    // The same table with no dispatch site is configuration data — its
    // values must not gain phantom references.
    let mut a = Analyser::new();
    let src = "proc do_add {a b} {}\narray set ops {add do_add}\n";
    let r = a.analyse(src, "tcl");
    assert!(
        !r.command_invocations.iter().any(|i| i.name == "do_add"),
        "{:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

// `emit_dispatch_table_command_references`'s `known` closure must not
// synthesise a "live reference" for a dispatch-table literal naming a
// proc/class that is renamed or deleted away with no later
// re-establishment: the table entry's own position is the call site it
// passes to `fact_live_for_call`. Confirmed against tclsh 8.6.14.

#[test]
fn dispatch_table_tp_issue_1010_deleted_proc_draws_no_reference() {
    let src = "proc do_add {a b} {}\nrename do_add {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    // `rename do_add {}` itself intentionally draws its own self-reference
    // to the OLD argument's token (go-to-definition on
    // that exact written word must still resolve, and real Tcl requires
    // `do_add` to exist at that point). That reference is not what this
    // test guards against; only a *dispatch-table*-synthesised reference
    // (via `array set ops {add do_add}` / `$ops($k)`) to the deleted,
    // never-re-established proc must be absent.
    let rename_self_ref_offset =
        u32::try_from(src.find("rename do_add").unwrap() + "rename ".len()).unwrap();
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.name == "do_add" && i.range.start() != rename_self_ref_offset),
        "a deleted proc with no re-establishment must draw no reference beyond rename's own self-reference: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_table_fp_issue_1010_reestablished_proc_still_references() {
    let src = "proc do_add {a b} {}\nrename do_add {}\nproc do_add {a b} {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.command_invocations
            .iter()
            .any(|i| i.name == "do_add" && i.resolved_qualified_name.as_deref() == Some("::do_add")),
        "a proc re-established after deletion must still be referenced: {:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dict_set_table_value_becomes_a_reference_when_consumed_m7() {
    let mut a = Analyser::new();
    let src =
        "proc handle_get {} {}\ndict set handlers GET handle_get\n{*}[dict get $handlers GET]\n";
    let r = a.analyse(src, "tcl8.6");
    let lit = u32::try_from(src.find(" handle_get\n").unwrap() + 1).unwrap();
    assert!(
        r.command_invocations.iter().any(|i| i.name == "handle_get"
            && i.range.start() == lit
            && i.resolved_qualified_name.as_deref() == Some("::handle_get")),
        "{:?}",
        r.command_invocations
            .iter()
            .map(|i| (&i.name, i.range.start()))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn dispatch_table_navigation_tracks_only_the_reaching_element_literal() {
    let src = "proc old {} {}\nproc keep {} {}\nproc new {} {}\narray set ops {x old y keep}\nset ops(x) new\n$ops(x)\n$ops(y)\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    for (literal, target, expected) in [
        ("x old", "old", false),
        ("y keep", "keep", true),
        ("ops(x) new", "new", true),
    ] {
        let start =
            u32::try_from(src.find(literal).unwrap() + literal.len() - target.len()).unwrap();
        assert_eq!(
            result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start
                    && reference.resolved_qualified_name.as_deref()
                        == Some(format!("::{target}").as_str())),
            expected,
            "{target}: {:?}",
            result.command_invocations,
        );
    }
}

#[test]
fn dispatch_table_navigation_uses_the_consuming_namespace() {
    let src = "namespace eval ::left {proc handler {} {}; array set ::ops {key handler}}\nnamespace eval ::right {proc handler {} {}; $::ops(key)}\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    let start = u32::try_from(src.find("key handler").unwrap() + 4).unwrap();
    assert!(
        result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == start
                && reference.resolved_qualified_name.as_deref() == Some("::right::handler")),
        "{:?}",
        result.command_invocations
    );
}

#[test]
fn dispatch_table_navigation_resolves_declarations_after_the_store() {
    let src = "array set ops {key later}\nproc later {} {}\n$ops(key)\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    let start = u32::try_from(src.find("key later").unwrap() + 4).unwrap();
    assert!(
        result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == start
                && reference.resolved_qualified_name.as_deref() == Some("::later")),
        "{:?}",
        result.command_invocations
    );
}

#[test]
fn dispatch_table_navigation_does_not_borrow_another_frames_store() {
    let src = "proc old {} {}\nproc new {} {}\nproc producer {} {array set ops {key old}}\nproc consumer {} {array set ops {key new}; $ops(key)}\nconsumer\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    let old = u32::try_from(src.find("key old").unwrap() + 4).unwrap();
    let new = u32::try_from(src.find("key new").unwrap() + 4).unwrap();
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == old)
    );
    assert!(
        result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == new
                && reference.resolved_qualified_name.as_deref() == Some("::new")),
        "{:?}",
        result.command_invocations
    );
}

#[test]
fn dispatch_table_navigation_declines_opaque_stores_and_observed_reads() {
    for suffix in [
        "set ops(key) [read stdin]\n$ops(key)\n",
        "proc watch args {uplevel 1 {set ops(key) replacement}}\ntrace add variable ops(key) read watch\n$ops(key)\n",
        "rename target {}\n$ops(key)\n",
    ] {
        let src = format!("proc target {{}} {{}}\narray set ops {{key target}}\n{suffix}");
        let result = Analyser::new().analyse(&src, "tcl8.6");
        let start = u32::try_from(src.find("key target").unwrap() + 4).unwrap();
        assert!(
            !result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start),
            "{suffix}: {:?}",
            result.command_invocations
        );
    }
}

#[test]
fn dispatch_table_dictionary_updates_preserve_other_entries_and_original_spans() {
    let src = "proc old {} {}\nproc keep {} {}\nproc new {} {}\nset ops [dict create x old nested {y keep}]\ndict set ops x new\n[dict get $ops x]\n[dict get $ops nested y]\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    for (literal, target, expected) in [
        ("x old", "old", false),
        ("y keep", "keep", true),
        ("ops x new", "new", true),
    ] {
        let start =
            u32::try_from(src.find(literal).unwrap() + literal.len() - target.len()).unwrap();
        assert_eq!(
            result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start
                    && reference.resolved_qualified_name.as_deref()
                        == Some(format!("::{target}").as_str())),
            expected,
            "{target}: {:?}",
            result.command_invocations
        );
    }
}

#[test]
fn dispatch_table_dictionary_literal_maps_each_repeated_value_separately() {
    let src =
        "proc handler {} {}\nset ops {unused handler chosen {handler}}\n[dict get $ops chosen]\n";
    let result = Analyser::new().analyse(src, "tcl8.6");
    let chosen = u32::try_from(src.find("chosen {handler}").unwrap() + 8).unwrap();
    let unused = u32::try_from(src.find("unused handler").unwrap() + 7).unwrap();
    assert!(
        result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == chosen
                && reference.range.end() == chosen + 7
                && reference.resolved_qualified_name.as_deref() == Some("::handler")),
        "{:?}",
        result.command_invocations
    );
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == unused)
    );
}

#[test]
fn dispatch_table_navigation_uses_the_selected_native_list_grammar() {
    // Pinned Jim 0.84 splits the adjacent braced key and bare value. C Tcl
    // rejects that dictionary before the outer invocation can execute.
    let src = "proc handler {} {}\nset ops {{key}handler}\n[dict get $ops key]\n";
    let start = u32::try_from(src.find("{key}handler").unwrap() + 5).unwrap();
    for (dialect, expected) in [("jim", true), ("tcl8.6", false)] {
        let result = Analyser::new().analyse(src, dialect);
        assert_eq!(
            result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start
                    && reference.resolved_qualified_name.as_deref() == Some("::handler")),
            expected,
            "{dialect}: {:?}",
            result.command_invocations
        );
    }
}

#[test]
fn dispatch_table_dictionary_keeps_known_entries_beside_unknown_values() {
    for update in [
        "set ops [dict create chosen handler other $other]",
        "set ops {chosen handler}; dict set ops other $other",
    ] {
        let src = format!(
            "proc handler {{}} {{}}\nproc p {{other}} {{{update}; [dict get $ops chosen]}}\n"
        );
        let start = u32::try_from(src.find("chosen handler").unwrap() + 7).unwrap();
        let result = Analyser::new().analyse(&src, "tcl8.6");
        assert!(
            result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start
                    && reference.resolved_qualified_name.as_deref() == Some("::handler")),
            "{update}: {:?}",
            result.command_invocations
        );
    }
    let src = "proc handler {} {}\nproc p {key value} {set ops {chosen handler}; dict set ops $key $value; [dict get $ops chosen]}\n";
    let start = u32::try_from(src.find("chosen handler").unwrap() + 7).unwrap();
    let result = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == start)
    );
}

#[test]
fn dispatch_table_navigation_never_confuses_equal_offsets_in_derived_source() {
    let src = "set ops {key handler}\nproc handler {} {}\neval [list set ops {key handler}]\n[dict get $ops key]\n";
    let stale = u32::try_from(src.find("key handler").unwrap() + 4).unwrap();
    let result = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == stale),
        "a derived store at offset zero cannot donate the overwritten authored literal: {:?}",
        result.command_invocations
    );
}

#[test]
fn dispatch_table_literal_cannot_select_one_of_two_redefined_implementations() {
    let src =
        "proc handler {} {}\nset ops(key) handler\n$ops(key)\nproc handler {} {}\n$ops(key)\n";
    let stored = u32::try_from(src.find("ops(key) handler").unwrap() + 9).unwrap();
    let result = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !result
            .command_invocations
            .iter()
            .any(|reference| reference.range.start() == stored && reference.rename_safe),
        "one shared literal denotes two definition allocations: {:?}",
        result.command_invocations
    );
    let calls = src
        .match_indices("$ops(key)")
        .map(|(offset, _)| u32::try_from(offset).unwrap())
        .collect::<Vec<_>>();
    let definitions = result
        .command_invocations
        .iter()
        .filter(|reference| calls.contains(&reference.range.start()))
        .filter_map(|reference| reference.resolved_definition.as_ref())
        .collect::<Vec<_>>();
    assert!(
        definitions.len() >= 2,
        "actual dispatch sites retain their definitions: {:?}",
        result.command_invocations
    );
    assert_ne!(definitions[0], definitions[1]);
}

#[test]
fn dispatch_table_expanded_prefix_maps_only_its_original_command_element() {
    for setup in [
        "array set ops {key {handler ARG}}\n{*}$ops(key)\n",
        "set ops {key {handler ARG}}\n{*}[dict get $ops key]\n",
    ] {
        let src = format!("proc handler {{argument}} {{}}\n{setup}");
        let start = u32::try_from(src.find("handler ARG").unwrap()).unwrap();
        let result = Analyser::new().analyse(&src, "tcl8.6");
        assert!(
            result
                .command_invocations
                .iter()
                .any(|reference| reference.range.start() == start
                    && reference.range.end() == start + 7
                    && reference.resolved_qualified_name.as_deref() == Some("::handler")),
            "{setup}: {:?}",
            result.command_invocations
        );
    }
}

#[test]
fn dispatch_table_constructed_list_preserves_each_original_head_operand() {
    for (name, setup) in [
        ("handler", "set ops(key) [list handler]\n$ops(key) ARG\n"),
        ("handler", "set ops(key) [list handler ARG]\n{*}$ops(key)\n"),
        (
            "my handler",
            "set ops [dict create key [list {my handler} ARG]]\n{*}[dict get $ops key]\n",
        ),
        (
            "handler",
            "set ops [list key [list handler ARG]]\n{*}[dict get $ops key]\n",
        ),
    ] {
        let src = format!("proc {{{name}}} {{argument}} {{}}\n{setup}");
        let start = u32::try_from(src.rfind(name).unwrap()).unwrap();
        let result = Analyser::new().analyse(&src, "tcl8.6");
        assert!(
            result.command_invocations.iter().any(|reference| {
                reference.range.start() == start
                    && reference.range.end() == start + u32::try_from(name.len()).unwrap()
                    && reference.resolved_qualified_name.as_deref()
                        == Some(format!("::{name}").as_str())
            }),
            "{setup}: {:?}",
            result.command_invocations
        );
    }
}

#[test]
fn dispatch_table_constructed_list_cannot_borrow_a_later_argument_as_its_head() {
    for (name, src) in [
        (
            "handler",
            "proc handler {argument} {}\nproc dispatch {name} {set ops(key) [list $name handler]; {*}$ops(key)}\n",
        ),
        (
            "my handler",
            "proc {my handler} {} {}\nset ops(key) [list {my handler}]\n$ops(key)\n",
        ),
    ] {
        let stored = u32::try_from(src.rfind(name).unwrap()).unwrap();
        let result = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !result
                .command_invocations
                .iter()
                .any(|reference| { reference.range.start() == stored && reference.rename_safe }),
            "only an actual head with unchanged native encoding can donate its literal: {:?}",
            result.command_invocations
        );
    }
}
