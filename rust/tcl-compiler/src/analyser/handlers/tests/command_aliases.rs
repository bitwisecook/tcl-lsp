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

//! `interp alias`, `rename` and the matching of computed command names against a moved command.

use super::*;

fn command_name_pattern_with_closer(
    word: &str,
    candidate: &str,
    closer: tcl_lexer::BracedVarStyle,
) -> bool {
    let mut input = Analyser::new().prepare_analysis_input("tcl").unwrap();
    input.config.braced_var = closer;
    let config = input.lexer_config();
    let source = format!("marker {word}");
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(&source, 0, config).remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(&source),
        config,
        &segment,
    );
    let original = crate::registry_invocation::original_native_compiler_words(
        &tcl_lexer::SourceImage::document(&source),
        tokens.words(),
        0,
        config,
    )
    .unwrap();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
        input.borrowed_context_registry().commands(),
        &input,
        config,
        Some(input.unit_profile()),
    );
    crate::dynamic_names::original_command_name_overlap(&original[1], candidate, metadata)
        .unwrap()
        .may_equal
}

fn dynamic_command_name_may_equal(word: &str, candidate: &str) -> bool {
    command_name_pattern_with_closer(word, candidate, tcl_lexer::BracedVarStyle::FirstClose)
}

#[test]
fn command_mutation_overlap_retains_the_original_configured_closer() {
    assert!(dynamic_command_name_may_equal("a:::$x:b", "::a::b"));
    assert!(!dynamic_command_name_may_equal("a:::$x:b", "::a::OTHER"));
    assert!(command_name_pattern_with_closer(
        "${a{b}}",
        "::string",
        tcl_lexer::BracedVarStyle::Tcl9Nesting
    ));
    assert!(!command_name_pattern_with_closer(
        "${a{b}}",
        "::string",
        tcl_lexer::BracedVarStyle::FirstClose
    ));
    for (closer, trusted) in [
        (tcl_lexer::BracedVarStyle::FirstClose, true),
        (tcl_lexer::BracedVarStyle::Tcl9Nesting, false),
    ] {
        let mut input = Analyser::new().prepare_analysis_input("tcl").unwrap();
        input.config.braced_var = closer;
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.analyse_and_retain_result_for_test("proc ${a{b}} {} {}", "tcl");
        assert_eq!(
            analyser.static_provenance_command_is_trusted("::string"),
            trusted
        );
    }
}

#[test]
fn dynamic_command_name_patterns_keep_only_proven_fixed_fragments() {
    assert!(!dynamic_command_name_may_equal(
        "${ns}::define::$method",
        "::string"
    ));
    assert!(dynamic_command_name_may_equal("$name", "::string"));
    assert!(dynamic_command_name_may_equal(
        "${ns}::::helper",
        "::T::helper"
    ));
    assert!(dynamic_command_name_may_equal(
        "$array(index)::define",
        "::anything"
    ));
}

#[test]
fn command_mutation_matching_preserves_constructed_colon_keys() {
    assert!(dynamic_command_name_may_equal("p", ":::::p"));
    assert!(dynamic_command_name_may_equal(":p", ":::p"));
    assert!(!dynamic_command_name_may_equal(":p", ":::::p"));
    // Written separator runs canonicalise once; an already constructed
    // namespace key must not be interpreted as that written spelling.
    assert!(dynamic_command_name_may_equal(":::::p", "::p"));
    assert!(!dynamic_command_name_may_equal(":::::p", ":::::p"));
    assert!(dynamic_command_name_may_equal("x:::y", "::x::y"));
    assert!(dynamic_command_name_may_equal("x:::", "::x::"));
    assert!(!dynamic_command_name_may_equal("x:::y", "::x:::y"));
}

/// A command-table mutation inside a child interpreter's eval body
/// belongs to *that* interpreter, not to a flat file-wide map.
///
/// tclsh 9.0.4, verified: after `interp create c; c eval { rename puts
/// myputs }` the parent's `puts` still works, the parent has no `myputs`,
/// the child has `myputs` and no `puts`; and `c eval { rename set {} }`
/// leaves the parent's `set` alone.
#[test]
fn rename_inside_a_child_body_does_not_touch_the_parent_command_table() {
    let src = "interp create c\nc eval { rename puts myputs }\nputs hi\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert_eq!(
        r.renamed_commands.len(),
        0,
        "a child's rename must not enter the parent's rename map: {:?}",
        r.renamed_commands
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.message.contains("'puts'")),
        "the parent's own `puts` is still a live builtin: {:?}",
        r.diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect::<Vec<_>>()
    );
}

/// The incremental per-item path must agree with full analysis here: a
/// deferred proc body is walked by a fresh `Analyser` that does not
/// inherit the interpreter stack, so a `rename` buried in a proc defined
/// inside a child body is the shape most likely to diverge.
#[test]
fn per_item_agrees_on_a_rename_nested_in_a_child_body() {
    let src = "interp create c\n\
                   c eval { proc setup {} { rename puts myputs } }\n\
                   puts hi\n";
    let codes = |r: &super::super::super::types::AnalysisResult| {
        let mut v: Vec<(String, u32)> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.span.start()))
            .collect();
        v.sort();
        v
    };
    let full = Analyser::new().analyse(src, "tcl8.6");
    let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
    assert_eq!(codes(&full), codes(&per_item));
    assert!(
        !full
            .diagnostics
            .iter()
            .any(|d| d.message.contains("'puts'")),
        "{:?}",
        codes(&full)
    );
}

/// The same rename written in the parent still records normally — the
/// guard narrows the fact to its interpreter, it does not disable it.
#[test]
fn rename_in_the_parent_still_records_the_command_move() {
    let src = "interp create c\nrename puts myputs\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert_eq!(
        r.renamed_commands.get("::myputs").map(String::as_str),
        Some("::puts"),
        "{:?}",
        r.renamed_commands
    );
}

/// `interp alias {} A {} B` inside a child's eval body creates the alias
/// in the *child* (an empty srcPath is "the interpreter I am running
/// in"), so it must home under that child's `@interp@` domain rather than
/// at the parent's global root.
#[test]
fn empty_path_interp_alias_inside_a_child_body_homes_in_that_child() {
    let src = "interp create c\nc eval { interp alias {} shout {} puts }\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        !r.command_aliases.contains_key("::shout"),
        "the parent gained no alias: {:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
    assert!(
        r.command_aliases.contains_key("::@interp@c::shout"),
        "the child's alias homes under its own domain: {:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
}

/// The same alias written at the top level is unchanged.
#[test]
fn empty_path_interp_alias_at_top_level_still_homes_globally() {
    let src = "interp alias {} shout {} puts\n";
    let r = Analyser::new().analyse(src, "tcl8.6");
    assert!(
        r.command_aliases.contains_key("::shout"),
        "{:?}",
        r.command_aliases.keys().collect::<Vec<_>>()
    );
}

// handle_interp_alias

#[test]
fn handle_interp_alias_records_canonical_form() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(
        &[
            "alias".to_string(),
            String::new(),
            "myset".to_string(),
            String::new(),
            "set".to_string(),
        ],
        &[],
        42,
    );
    assert!(a.command_aliases.contains_key("::myset"));
    assert!(a.result.command_aliases.contains_key("::myset"));
    // The resolution map keys the target as the global command it invokes,
    // while the published record keeps the written spelling. tclsh 8.4.20,
    // 8.6.18, 9.0.4 and 9.1.0: after `namespace eval n {proc set args
    // {return n-set}}; namespace eval n {interp alias {} ::ms {} set}`,
    // `ms x 1` returns 1 (the global `set`) and `interp alias {} ::ms`
    // returns `set`.
    let (target, prepended) = &a.command_aliases["::myset"];
    assert_eq!(target, "::set");
    assert_eq!(a.result.command_aliases["::myset"].target, "set");
    assert_eq!(prepended.as_slice(), [] as [String; 0]);
    assert_eq!(a.alias_offsets.get("::myset"), Some(&42));
    // The offset is also promoted onto the finalised `AnalysisResult`
    // (not just the in-progress `Analyser`), so a cross-document
    // consumer such as `tcl_lsp_core::WorkspaceIndex` can tell an
    // unconditional alias apart from one nested in a proc/class body.
    assert_eq!(a.result.alias_offsets.get("::myset"), Some(&42));
}

#[test]
fn handle_interp_alias_with_prepended_args() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(
        &[
            "alias".to_string(),
            String::new(),
            "logerr".to_string(),
            String::new(),
            "puts".to_string(),
            "stderr".to_string(),
        ],
        &[],
        0,
    );
    let (target, prepended) = &a.command_aliases["::logerr"];
    assert_eq!(target, "::puts");
    assert_eq!(a.result.command_aliases["::logerr"].target, "puts");
    assert_eq!(prepended, &vec!["stderr".to_string()]);
}

#[test]
fn handle_interp_alias_wrong_shape_no_op() {
    let mut a = Analyser::new();
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.handle_interp_alias(&["alias".to_string()], &[], 0);
    assert_eq!(a.command_aliases.len(), 0);
    assert_eq!(a.alias_offsets.len(), 0);
}

// handle_rename

#[test]
fn handle_rename_records_static_move() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(
        &["target".to_string(), "target_orig".to_string()],
        &[],
        &[],
        &[],
        42,
    );
    assert!(!dynamic, "a fully static rename is not dynamic");
    assert_eq!(
        a.renamed_commands.get("::target_orig").map(String::as_str),
        Some("::target")
    );
    assert_eq!(
        a.result
            .renamed_commands
            .get("::target_orig")
            .map(String::as_str),
        Some("::target")
    );
    assert_eq!(a.rename_offsets.get("::target_orig"), Some(&42));
    // The old name is recorded as deleted from this offset —
    // confirmed against tclsh 9.0.4: `target` becomes an unknown
    // command, not still callable under its original arity.
    assert_eq!(a.deleted_commands.get("::target"), Some(&42));
}

#[test]
fn handle_rename_deletion_records_old_name_as_deleted() {
    // `rename OLD {}` deletes OLD — no new binding to record (it is
    // not a "dynamic" rename either, nothing to widen for), but OLD
    // itself must be recorded as gone (confirmed against tclsh
    // 9.0.4: also "invalid command name" afterwards).
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["target".to_string(), String::new()], &[], &[], &[], 7);
    assert!(!dynamic);
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.get("::target"), Some(&7));
}

#[test]
fn handle_rename_dynamic_old_name_reports_dynamic() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["$x".to_string(), "y".to_string()], &[], &[], &[], 0);
    assert!(dynamic, "rename $x y cannot be resolved statically");
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

#[test]
fn handle_rename_dynamic_new_name_reports_dynamic() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["x".to_string(), "y[z]".to_string()], &[], &[], &[], 0);
    assert!(dynamic, "rename x y[z] cannot be resolved statically");
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

#[test]
fn handle_rename_wrong_shape_no_op() {
    let mut a = Analyser::new();
    let dynamic = a.handle_rename(&["onlyone".to_string()], &[], &[], &[], 0);
    assert!(!dynamic);
    assert_eq!(a.renamed_commands.len(), 0);
    assert_eq!(a.deleted_commands.len(), 0);
}

/// A `try` handler body is `Selected` — conditional and control flow — so
/// a `rename` there may never run and is not a straight-line deletion,
/// as one inside a loop body is not. A
/// `finally` body always runs, so its `rename` still is one.
#[test]
fn a_rename_in_a_try_handler_is_not_a_straight_line_deletion() {
    let destroys_foo = |src: &str| {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .destroyed_commands
            .contains_key("::foo")
    };
    assert!(!destroys_foo(
        "proc foo {} {}\ntry { set x 1 } on error {} { rename foo {} }\n"
    ));
    assert!(!destroys_foo(
        "proc foo {} {}\ntry { set x 1 } trap {POSIX ENOENT} {} { rename foo {} }\n"
    ));
    assert!(destroys_foo(
        "proc foo {} {}\ntry { set x 1 } finally { rename foo {} }\n"
    ));
    assert!(destroys_foo("proc foo {} {}\nrename foo {}\n"));
}
