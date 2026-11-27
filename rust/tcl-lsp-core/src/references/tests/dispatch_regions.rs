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

//! Nested regions a dispatch scan reaches: frame shifts, `expect` clauses, case lists and their aliases.

use super::*;

#[test]
fn shifted_dispatch_regions_follow_original_moves_and_known_source_barriers() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    for (source, visible) in [
        (
            "rename namespace shifted; shifted eval ::N {puts nested}",
            true,
        ),
        (
            "rename namespace shifted; proc shifted {args} {}; shifted eval ::N {puts hidden}",
            false,
        ),
        (
            "rename namespace shifted; rename shifted {}; shifted eval ::N {puts hidden}",
            false,
        ),
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let command = tcl_source::segment::segment_commands(source).pop().unwrap();
        let regions = frame_shifted_dispatch_regions(source, &analysis, profile, &command);
        assert_eq!(
            regions
                .iter()
                .any(|&(start, end)| &source[start..end] == "puts nested"),
            visible
        );
        assert_eq!(
            nested_dispatch_regions(source, &analysis, profile, &command),
            [] as [(usize, usize); 0]
        );
        assert_eq!(
            frame_shifted_dispatch_regions(
                &source.replace("puts", "gets"),
                &analysis,
                profile,
                &command
            ),
            [] as [(usize, usize); 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            frame_shifted_dispatch_regions(source, &analysis, profile, &command),
            [] as [(usize, usize); 0]
        );
    }
}

#[test]
fn expect_clause_flags_reach_each_clause_body() {
    let source = "expect {-re {^ready$} {puts ready} -timeout 5 timeout {puts slow}}";
    let command = tcl_source::segment::segment_commands(source)
        .into_iter()
        .next()
        .expect("expect command");
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
        tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
        &command,
    );
    assert_eq!(
        regions.len(),
        2,
        "Expect clause flags must not shift bodies"
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts ready"))
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts slow"))
    );
}

#[test]
fn inline_expect_clause_flags_reach_each_clause_body() {
    let source = "expect -re {^ready$} {puts ready} -timeout 5 timeout {puts slow}";
    let command = tcl_source::segment::segment_commands(source)
        .into_iter()
        .next()
        .expect("expect command");
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
        tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
        &command,
    );
    assert_eq!(
        regions.len(),
        2,
        "inline Expect flags must not shift bodies"
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts ready"))
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts slow"))
    );
}

#[test]
fn malformed_case_lists_do_not_expose_nested_dispatch_regions() {
    for source in [
        "switch subject {a {puts hidden} orphan}",
        "switch subject {}",
    ] {
        let command = tcl_source::segment::segment_commands(source)
            .into_iter()
            .next()
            .expect("case-list command");
        assert!(
            nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &command
            )
            .is_empty(),
            "semantic references must abstain for malformed case list: {source:?}"
        );
    }
}

#[test]
fn case_list_aliases_use_positioned_resolved_registry_identity() {
    let source = concat!(
        "pick subject {default {puts before_alias}}\n",
        "interp alias {} pick {} switch\n",
        "pick subject {default {puts through_alias}}\n",
    );
    let commands = tcl_source::segment::segment_commands(source);
    assert!(
        nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &commands[0]
        )
        .is_empty(),
        "a later alias must not apply before its declaration"
    );
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &commands[2],
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("through_alias")),
        "a live static switch alias must expose its clause body: {regions:?}"
    );
}

#[test]
fn deleted_or_rebound_case_list_aliases_do_not_expose_clause_bodies() {
    for source in [
        concat!(
            "interp alias {} pick {} switch\n",
            "interp alias {} pick {}\n",
            "pick subject {default {puts deleted_alias}}\n",
        ),
        concat!(
            "interp alias {} pick {} switch\n",
            "interp alias {} pick {} puts\n",
            "pick subject {default {puts rebound_alias}}\n",
        ),
    ] {
        let command = tcl_source::segment::segment_commands(source)
            .into_iter()
            .last()
            .expect("alias call");
        assert!(
            nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &command
            )
            .is_empty(),
            "a deleted or rebound alias must not use switch grammar: {source:?}"
        );
    }
}

#[test]
fn rooted_and_namespaced_case_list_aliases_expose_clause_bodies() {
    let source = concat!(
        "namespace eval ::case_alias {}\n",
        "interp alias {} ::root_pick {} ::switch\n",
        "interp alias {} ::case_alias::pick {} switch\n",
        "::root_pick subject {default {puts rooted_alias}}\n",
        "::case_alias::pick subject {default {puts namespaced_alias}}\n",
    );
    let commands = tcl_source::segment::segment_commands(source);
    for (command, marker) in [
        (&commands[3], "rooted_alias"),
        (&commands[4], "namespaced_alias"),
    ] {
        let regions = nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            command,
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains(marker)),
            "rooted/namespaced alias must use its resolved switch grammar: {marker}; {regions:?}"
        );
    }
}

#[test]
fn references_reach_my_dispatch_inside_a_static_switch_alias() {
    let source = concat!(
        "interp alias {} pick {} switch\n",
        "oo::class create C {\n",
        "    method target {} {}\n",
        "    method caller {} {\n",
        "        pick subject {default {my target}}\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(source);
    // `target` begins after the four-space indent plus `method `.
    let refs = references(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|range| range.start_line).collect();
    assert!(lines.contains(&2), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "`my target` inside the resolved alias case body is missing: {refs:?}"
    );
}

#[test]
fn switch_braced_body_references_reach_nested_command() {
    // tclsh 8.4.20-9.1.0: an unset `$state` raises before `switch` runs;
    // with `set state ready` the `ready` arm calls the procedure.
    let source =
        "proc ready {} {}\nset state ready; switch $state { ready {ready} default {set x 1}}\n";
    let analysis = analyse(source);
    let refs = references(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        5,
        &analysis,
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 0) && refs.iter().any(|r| r.start_line == 1),
        "switch braced body reference missing: {refs:?}"
    );
}
