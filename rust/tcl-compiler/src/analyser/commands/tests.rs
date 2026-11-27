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

use super::*;
use tcl_lexer::{Span, TokenType};

#[test]
fn document_image_reuse_preserves_full_bytes_channel_and_line_positions() {
    for source in ["", "set x 1\r\nputs $x\r", "café\nλ", "a\0b"] {
        let expected = tcl_lexer::SourceImage::document(source);
        let native = tcl_lexer::SourceImage::native(source.as_bytes());
        let changed = tcl_lexer::SourceImage::document(&format!("{source}different"));
        let reused = original_document_source_image(source, Some(&expected));
        assert!(Arc::ptr_eq(
            &reused.shared_bytes(),
            &expected.shared_bytes()
        ));
        for retained in [None, Some(&expected), Some(&native), Some(&changed)] {
            let actual = original_document_source_image(source, retained);
            assert_eq!(actual, expected);
            assert_eq!(actual.channel(), tcl_lexer::SourceChannel::Document);
            let actual_map = actual.source_map();
            let expected_map = expected.source_map();
            for offset in 0..=source.len() {
                let offset = u32::try_from(offset).unwrap();
                assert_eq!(
                    actual_map.position_at(offset),
                    expected_map.position_at(offset)
                );
            }
        }
    }
}

fn uncached_original_source_segment(
    analyser: &Analyser,
    offset: u32,
    argument_tokens: &[Token],
) -> Option<(SegmentedCommand, tcl_lexer::NativeWord)> {
    let tokens = analyser.retained_invocation_tokens(offset, argument_tokens)?;
    let image = tcl_lexer::SourceImage::document(&analyser.source);
    let config = analyser.file_lexer_config();
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let first = native.first()?;
    let last = native.last()?;
    let source = analyser
        .source
        .get(first.span().start() as usize..last.span().end() as usize)?;
    let mut segments = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        first.span().start(),
        config.nested(),
    );
    if segments.len() != 1 {
        return None;
    }
    Some((segments.pop()?, first.clone()))
}

#[test]
fn diagnostic_source_geometry_matches_uncached_capture_and_missing_origin() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
        for source in [
            "set auto_index(example) [list source [file join $dir example.tcl]]",
            "list {} \"\" {a\\}}",
            "list {*}{one two}",
        ] {
            let mut analyser = Analyser::new();
            let _analysis = analyser.analyse(source, dialect);
            let config = analyser.file_lexer_config();
            let segments =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
            for segment in &segments {
                let offset = segment.argv.first().unwrap().span.start();
                let args = &segment.argv[1..];
                let expected = uncached_original_source_segment(&analyser, offset, args);
                for _ in 0..2 {
                    assert_eq!(
                        analyser.original_source_segment(offset, args, false),
                        expected,
                        "{dialect}: {source}",
                    );
                }
                assert!(
                    analyser
                        .original_source_segment(offset, args, true)
                        .is_none()
                );
            }
            analyser.head_identities = crate::realm::CommandBindingRealm::default();
            for segment in &segments {
                let offset = segment.argv.first().unwrap().span.start();
                let args = &segment.argv[1..];
                assert_eq!(
                    analyser.original_source_segment(offset, args, false),
                    uncached_original_source_segment(&analyser, offset, args),
                    "missing origin: {dialect}: {source}",
                );
            }
        }
    }
}

#[test]
fn original_body_heads_keep_parent_file_config_and_nested_bom_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Source syntax only; a command-head observation grants no execution.
    let body = "if 1 {\u{feff}set hidden value; set visible value}\nset result [\u{feff}set inner value; set tail value]";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let default_source = format!("\u{feff}{body}");
    let ordinary = Analyser::new().analyse(&default_source, "tcl9.1");
    let selected = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(&default_source, "display-only");
    for result in [&ordinary, &selected] {
        let names = result
            .command_invocations
            .iter()
            .map(|call| call.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"if"), "{names:?}");
        assert_eq!(
            names.iter().filter(|name| **name == "\u{feff}set").count(),
            2,
            "{names:?}"
        );
        assert_eq!(result.resolved_input.as_ref(), Some(&input));
        assert!(result.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&default_source),
            input.lexer_config()
        ));
    }
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        supplied.config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let result = Analyser::new()
            .with_resolved_input(supplied.clone())
            .analyse(body, "display-only");
        let names = result
            .command_invocations
            .iter()
            .map(|call| call.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names.iter().filter(|name| **name == "\u{feff}set").count(),
            2,
            "{bom:?}: {names:?}"
        );
        assert_eq!(result.resolved_input.as_ref(), Some(&supplied));
        assert_eq!(result.body_lexer_config, Some(supplied.lexer_config()));
        let mut foreign = supplied.lexer_config();
        foreign.strict_quoting = !foreign.strict_quoting;
        assert!(
            !result.matches_original_source_image(&tcl_lexer::SourceImage::document(body), foreign)
        );
        assert!(!result.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&(body.to_owned() + "\n")),
            supplied.lexer_config()
        ));
    }
}

/// `if {1} { if {1} { ... } }`, `depth` levels deep, wrapped in a `proc`
/// body.
fn nested_if_source(depth: u32) -> String {
    let mut source = String::from("proc deepnest {} {\n");
    for _ in 0..depth {
        source.push_str("if {1} {\n");
    }
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source.push_str("}\n");
    source
}

/// Run `analyse` on a dedicated thread with a generous stack.
///
/// `cargo test` runs each `#[test]` on its own thread with the platform
/// default stack size (~2 MiB on Linux) — the same undersized budget
/// that a deep walk overflows (Tokio's default
/// worker-thread stack is the same size). A test that walks source
/// nested past [`MAX_BODY_DEPTH`] needs the same generous, explicit
/// stack production code now gets via `tokio::runtime::Builder::
/// thread_stack_size` (`tcl-lsp-server`/`tcl-mcp`) and `std::thread::
/// Builder::stack_size` (the `tcl` CLI) — otherwise the test harness
/// itself hits the bug this suite exists to catch.
fn analyse_on_big_stack(source: String, dialect: &str) -> super::super::types::AnalysisResult {
    let dialect = dialect.to_owned();
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || Analyser::new().analyse(&source, &dialect))
        .expect("spawn big-stack test thread")
        .join()
        .expect("analyse on big-stack thread panicked")
}

#[test]
fn depth_exactly_at_cap_emits_no_e207() {
    // The wrapping `proc` body is itself one level of `body_depth`, so
    // `MAX_BODY_DEPTH - 1` nested `if`s is what brings body_depth to
    // exactly `MAX_BODY_DEPTH`.
    let source = nested_if_source(MAX_BODY_DEPTH.0 - 1);
    let res = analyse_on_big_stack(source, "tcl9.0");
    assert!(
        !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
        "depth == MAX_BODY_DEPTH must not trip the cap: {:?}",
        res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );
}

#[test]
fn depth_one_past_cap_emits_e207_exactly_once() {
    let source = nested_if_source(MAX_BODY_DEPTH.0);
    let res = analyse_on_big_stack(source, "tcl9.0");
    let e207_count = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E207)
        .count();
    assert_eq!(
        e207_count, 1,
        "depth == MAX_BODY_DEPTH + 1 must trip the cap exactly once, got {e207_count}"
    );
}

#[test]
fn depth_far_past_cap_still_emits_e207_exactly_once() {
    // Not flooded: the cap trips at the same nesting level every time
    // (once body_depth cannot go higher), so it must still be exactly
    // one diagnostic at 2000 levels, not one per level past the cap.
    let source = nested_if_source(2000);
    let res = analyse_on_big_stack(source, "tcl9.0");
    let e207_count = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E207)
        .count();
    assert_eq!(
        e207_count, 1,
        "2000 levels must still trip the cap exactly once, got {e207_count}"
    );
}

#[test]
fn shallow_nesting_never_emits_e207() {
    // False-positive guard: ordinary, hand-written nesting must never
    // draw E207.
    let source = nested_if_source(10);
    let mut a = Analyser::new();
    let res = a.analyse(&source, "tcl9.0");
    assert!(
        !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
        "shallow nesting must never emit E207: {:?}",
        res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );
}

#[test]
fn resolve_analyser_hook_uses_registry_rooted_resolution() {
    use tcl_registry::hooks::AnalyserHookId as H;
    let a = Analyser::new();
    let args = |words: &[&str]| words.iter().map(ToString::to_string).collect::<Vec<_>>();

    // Unstamped head: no handler family.
    assert_eq!(a.resolve_analyser_hook("puts", &args(&["hi"])), None);
    // Command-level stamp.
    assert_eq!(
        a.resolve_analyser_hook("proc", &args(&["p", "a", "b"])),
        Some(H::Proc)
    );
    // Subcommand-level stamps pick the family per subcommand word…
    assert_eq!(
        a.resolve_analyser_hook("namespace", &args(&["eval", "ns", "{}"])),
        Some(H::NamespaceEval)
    );
    assert_eq!(
        a.resolve_analyser_hook("namespace", &args(&["import", "::t::*"])),
        Some(H::NamespaceImport)
    );
    // …and an unstamped subcommand of a stamped command resolves
    // nothing (`namespace which`, `dict set`).
    assert_eq!(
        a.resolve_analyser_hook("namespace", &args(&["which", "x"])),
        None
    );
    assert_eq!(
        a.resolve_analyser_hook("dict", &args(&["set", "d", "k", "v"])),
        None
    );
    assert_eq!(
        a.resolve_analyser_hook("dict", &args(&["with", "$d", "{}"])),
        Some(H::DictWith)
    );
    // A namespaced registry spelling and its rooted form resolve
    // identically through the registry owner.
    assert_eq!(
        a.resolve_analyser_hook("tcl::OptProc", &args(&["p", "{a}", "{}"])),
        Some(H::OptProc)
    );
    assert_eq!(
        a.resolve_analyser_hook("::tcl::OptProc", &args(&["p", "{a}", "{}"])),
        Some(H::OptProc)
    );
    // Rootable global commands have the same exact binding and hook with
    // or without their explicit global qualifier.
    assert_eq!(
        a.resolve_analyser_hook("::proc", &args(&["p", "a", "b"])),
        Some(H::Proc)
    );
    assert_eq!(
        a.resolve_analyser_hook("set", &args(&["name", "value"])),
        a.resolve_analyser_hook("::set", &args(&["name", "value"]))
    );
}

/// A Tk widget constructor is syntactically identical to a tcllib
/// naming factory (`struct::graph g`) — a bareword `ttk::treeview .t`
/// retains a conditional `.t` class label through the shared original
/// source factory receipt, without publishing successful creation.
/// Tk widgets exist only once Tk is loaded (tclsh 8.4.20-9.1.0 without it:
/// `invalid command name "ttk::treeview"`).
const BAREWORD_WIDGET: &str = "package require Tk\nttk::treeview .t\n.t instate {selected} {}\n";

#[test]
fn bareword_widget_constructor_binds_instance_class() {
    let mut a = super::super::state::Analyser::new();
    let res = a.analyse(BAREWORD_WIDGET, "tcl8.6");
    assert_eq!(
        res.instance_classes.get(".t").map(String::as_str),
        Some("ttk::treeview"),
        "instance_classes: {:?}",
        res.instance_classes
    );
    assert!(res.created_instance_commands.is_empty());
    assert!(
        crate::registry_invocation::source_structure::source_command_publication_at(
            BAREWORD_WIDGET,
            &res,
            u32::try_from(BAREWORD_WIDGET.find("ttk::treeview").unwrap()).unwrap()
        )
        .is_some()
    );
}

#[test]
fn a_widget_constructor_without_its_package_publishes_no_instance() {
    // tclsh 8.4.20-9.1.0 without `package require Tk`: invalid command name
    // "ttk::treeview", so `.t` is never created.
    let source =
        "ttk::treeview .t\n.t instate {selected} {}\nset lb [listbox .l]\n$lb curselection\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse(source, dialect);
        assert!(
            !res.instance_classes.contains_key(".t") && !res.instance_classes.contains_key("lb"),
            "{dialect}: instance_classes: {:?}",
            res.instance_classes
        );
        for call in [".t instate", "$lb curselection"] {
            assert!(
                crate::registry_invocation::source_structure::source_registered_instance_words_at(
                    source,
                    &res,
                    u32::try_from(source.find(call).unwrap()).unwrap()
                )
                .is_none(),
                "{dialect}: {call}"
            );
        }
    }
}

#[test]
fn a_widget_constructor_after_an_unproved_require_publishes_no_instance() {
    // tclsh 8.6.18, 9.0.4 and 9.1.0 without Tk installed: a caught or
    // conditional require leaves `info commands ttk::treeview` empty.
    for prefix in [
        "catch {package require Tk}\n",
        "if {[info exists nothing]} {package require Tk}\n",
        "package provide Tk 8.6; package require Tk\n",
        "package require Tk; rename ttk::treeview {}; proc ttk::treeview args {}\n",
    ] {
        let source = format!("{prefix}ttk::treeview .t\n");
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse(&source, "tcl8.6");
        assert!(
            !res.instance_classes.contains_key(".t"),
            "{prefix}: instance_classes: {:?}",
            res.instance_classes
        );
    }
}

#[test]
fn a_widget_constructor_publishes_where_tk_is_present() {
    // A stock loader reached from the selected authoring entry supplies the
    // factory. A successful namespace body preserves its load.
    for (source, dialect) in [
        ("package require Tk\nttk::treeview .t\n", "tcl8.6"),
        (
            "namespace eval ui {package require Tk}\nttk::treeview .t\n",
            "tcl8.6",
        ),
    ] {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse(source, dialect);
        assert_eq!(
            res.instance_classes.get(".t").map(String::as_str),
            Some("ttk::treeview"),
            "{dialect}: {source:?}"
        );
    }
}

#[test]
fn a_widget_constructor_in_a_sourced_document_uses_its_actual_entry() {
    use crate::registry_invocation::source_structure::original_project_source_load_at;
    let child = "ttk::treeview .t";
    let mut previous_site = None;
    for (prefix, expected) in [
        ("package require Tk", true),
        ("set before 1; package require Tk", true),
        ("package provide Tk 8.6; package require Tk", false),
        (
            "package require Tk; rename ttk::treeview {}; proc ttk::treeview args {}",
            false,
        ),
    ] {
        let parent = format!("{prefix}; source child.tcl");
        let analysis = Analyser::new().analyse(&parent, "tcl8.6");
        let offset = u32::try_from(parent.find("source child.tcl").unwrap()).unwrap();
        let load =
            original_project_source_load_at(&parent, &analysis, offset).expect("source handler");
        let entry = load.source_point_entry(&analysis).expect("parent entry");
        let child = Analyser::new()
            .analyse_with_source_point_entry(child, "tcl8.6", &entry)
            .expect("inherited child");
        assert_eq!(
            child.instance_classes.contains_key(".t"),
            expected,
            "{prefix}"
        );
        if expected {
            let context = child
                .resolved_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry();
            let descriptor = context
                .context()
                .resolve_spec(context.commands(), "ttk::treeview")
                .unwrap();
            let realm = child.retained_command_realm().unwrap();
            let binding = realm.invocation_at_source("", 0);
            let site = binding.invocation_site().expect("reached child factory");
            let bindings = realm.source_bindings_ref();
            assert!(bindings.package_factory_is_proved_at(Some(site), descriptor));
            if let Some(other) = &previous_site {
                assert_ne!(site, other);
                assert!(!bindings.package_factory_is_proved_at(Some(other), descriptor));
            }
            previous_site = Some(site.clone());
        }
    }
}

/// The `set w [ctor .path]` return-value-capture shape resolves through
/// the shared source handle-class receipt and retains a class label.
#[test]
fn var_captured_widget_constructor_binds_instance_class() {
    let mut a = super::super::state::Analyser::new();
    let res = a.analyse(
        "package require Tk\nset lb [listbox .l]\n$lb curselection\n",
        "tcl8.6",
    );
    assert_eq!(
        res.instance_classes.get("lb").map(String::as_str),
        Some("listbox"),
        "instance_classes: {:?}",
        res.instance_classes
    );
}

/// A plain widget with no modelled subcommands (`button`) still binds
/// an instance class (useful for definition/references/W123
/// suppression) even though it has no `object_class` to dispatch
/// methods against.
#[test]
fn simple_widget_without_subcommands_still_binds_instance_class() {
    let mut a = super::super::state::Analyser::new();
    let res = a.analyse("package require Tk\nbutton .b -text hi\n", "tcl8.6");
    assert_eq!(
        res.instance_classes.get(".b").map(String::as_str),
        Some("button"),
        "instance_classes: {:?}",
        res.instance_classes
    );
}

#[test]
fn interp_create_original_name_positions_keep_option_width_and_terminators() {
    // naming.source.original-command-name-publications
    // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
    for (source, name) in [
        ("interp create -safe sandbox", "sandbox"),
        ("interp create -- sandbox", "sandbox"),
        ("interp create -- -safe", "-safe"),
        ("interp create -safe -- sandbox", "sandbox"),
        ("interp create sandbox", "sandbox"),
    ] {
        let result = Analyser::new().analyse(source, "tcl9.0");
        let publication =
            crate::registry_invocation::source_structure::source_command_publication_at(
                source, &result, 0,
            )
            .unwrap();
        assert_eq!(publication.name_bytes(), name.as_bytes());
        assert_eq!(
            publication.kind(),
            tcl_registry::AuthoredSourceCommandPublicationKind::Command
        );
        assert!(
            result.created_instance_commands.is_empty(),
            "a source naming layout is not successful creation"
        );
    }
}

#[test]
fn interp_create_safe_with_literal_name_suppresses_w123_on_later_call() {
    // TP — the user-visible symptom: with the name unrecorded, every
    // literal call to the interpreter's own object command raises a
    // false-positive W123 "unknown command".
    let src = "interp create -safe sandbox\nsandbox eval {set x 1}\n";
    assert!(
        !has_code(src, "tcl9.0", "W123"),
        "a literal call to a `interp create -safe`-created name must \
             not raise W123: {:?}",
        diag_codes(src, "tcl9.0")
    );
}

#[test]
fn interp_create_with_no_name_records_nothing() {
    // naming.source.original-command-name-publications
    // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
    let source = "interp create -safe";
    let result = Analyser::new().analyse(source, "tcl9.0");
    assert!(
        crate::registry_invocation::source_structure::source_command_publication_at(
            source, &result, 0
        )
        .is_none()
    );
    assert!(result.created_instance_commands.is_empty());
}

#[test]
fn scan_nested_command_heads_simple() {
    let out = scan_nested_command_heads("[helper $x]");
    assert_eq!(out, vec![("helper".to_string(), 1)]);
}

#[test]
fn scan_nested_command_heads_nested() {
    // [outer [inner $x]]
    let out = scan_nested_command_heads("[outer [inner $x]]");
    assert_eq!(
        out,
        vec![("outer".to_string(), 1), ("inner".to_string(), 8)]
    );
}

/// Depth coverage: `scan_nested_command_heads` self-recurses once per
/// nested `[…]` substitution inside a single word's raw text, and
/// `collect_segment_recursive` / `collect_substitution_segments` mutually
/// recurse the same way. Both axes are unbounded without their own caps,
/// independent of the statement-tree `MAX_BODY_DEPTH` cap, and overflow the
/// native stack (SIGABRT) in the low thousands of levels on a 2 MiB
/// thread. 3000 is past that crash range and past `MAX_BRACKET_TEXT_DEPTH`
/// (256); the assertion is that each returns.
#[test]
fn deeply_nested_command_head_scans_survive() {
    // `[a [a [a … [a x] … ]]]` nested bracket substitutions drive the
    // direct Tier 1B text scanner.
    let mut brackets = "x".to_owned();
    for _ in 0..3000 {
        brackets = format!("[a {brackets}]");
    }
    let _ = scan_nested_command_heads(&brackets);

    // `a [a [a … [x] … ]]` — each level is a literal-headed command whose
    // argument holds the next `[…]` substitution — drives the
    // `collect_segment_recursive` / `collect_substitution_segments` mutual
    // recursion directly (isolated from the full analyser pipeline).
    let mut nested = "x".to_owned();
    for _ in 0..3000 {
        nested = format!("a [{nested}]");
    }
    let config = LexerConfig::for_dialect("tcl8.6");
    let sm = SourceMap::new(&nested);
    let analysis = crate::analyser::types::AnalysisResult::default();
    let mut out = Vec::new();
    for seg in tcl_source::segment::segment_commands_with_offset_and_config(&nested, 0, config) {
        collect_segment_recursive(&sm, &analysis, seg, config, &mut out, 0);
    }
}

#[test]
fn scan_nested_command_heads_inside_quoted_string() {
    // "got [count $items]" — quotes don't interfere with [
    // ``count`` starts at byte 6 (after ``"got [``).
    let out = scan_nested_command_heads("\"got [count $items]\"");
    assert_eq!(out, vec![("count".to_string(), 6)]);
}

#[test]
fn scan_nested_command_heads_skips_braced_regions() {
    // Braced regions are opaque — `[` inside `{...}` doesn't count.
    // ``real`` starts at byte 15 (after ``{[not_a_cmd]} [``).
    let out = scan_nested_command_heads("{[not_a_cmd]} [real]");
    assert_eq!(out, vec![("real".to_string(), 15)]);
}

#[test]
fn scan_nested_command_heads_skips_backslash_escape() {
    // \[foo\] is a literal pair, not a substitution.
    // ``bar`` starts at byte 9 (after ``\\[foo\\] [``).
    let out = scan_nested_command_heads("\\[foo\\] [bar]");
    assert_eq!(out, vec![("bar".to_string(), 9)]);
}

#[test]
fn scan_nested_command_heads_no_match_for_pure_var_head() {
    // [$cmd args] — head is a variable substitution, not a name
    let out = scan_nested_command_heads("[$cmd args]");
    assert_eq!(out, [] as [(std::string::String, u32); 0]);
}

#[test]
fn scan_nested_command_heads_no_match_for_unclosed() {
    // Unclosed [ — returns nothing (recovery is segmenter's job)
    let out = scan_nested_command_heads("[lindex $x 0");
    assert_eq!(out, [] as [(std::string::String, u32); 0]);
}

#[test]
fn scan_nested_command_heads_two_independent_substs() {
    // [a $x] [b $y]
    let out = scan_nested_command_heads("[a $x] [b $y]");
    assert_eq!(out, vec![("a".to_string(), 1), ("b".to_string(), 8)]);
}

fn esc_tok(span: Span) -> Token {
    Token::new(TokenType::Esc, span)
}

fn str_tok(span: Span) -> Token {
    Token {
        kind: TokenType::Str,
        span,
        content_offset: 1,
        in_quote: false,
    }
}

fn span(start: u32, end: u32) -> Span {
    Span::new(start, end)
}

// Decision 2026-10-22T13:17Z models the original loaded source. The
// normal analysis route installs its actual input and command owners.
#[test]
fn process_set_defines_variable() {
    let mut a = Analyser::new();
    a.result = a.analyse("set x 1\n", "tcl8.6");
    assert!(a.result.global_scope.variables.contains_key("x"));
}

#[test]
fn process_proc_records_at_global() {
    let mut a = Analyser::new();
    a.process_command(
        &[
            "proc".to_string(),
            "foo".to_string(),
            "a b".to_string(),
            "set x $a".to_string(),
        ],
        &[
            esc_tok(span(0, 4)),
            esc_tok(span(5, 8)),
            esc_tok(span(9, 14)),
            str_tok(span(15, 25)),
        ],
        &[true, true, true, true],
        &[],
        &[],
    );
    assert!(a.result.all_procs.contains_key("::foo"));
}

#[test]
fn process_namespace_eval_opens_scope() {
    let mut a = Analyser::new();
    a.process_command(
        &[
            "namespace".to_string(),
            "eval".to_string(),
            "ns1".to_string(),
            String::new(),
        ],
        &[
            esc_tok(span(0, 9)),
            esc_tok(span(10, 14)),
            esc_tok(span(15, 18)),
            str_tok(span(19, 21)),
        ],
        &[true, true, true, true],
        &[],
        &[],
    );
    assert_eq!(a.result.global_scope.children.len(), 1);
    assert_eq!(a.result.global_scope.children[0].name, "ns1");
}

#[test]
fn process_foreach_defines_loop_var() {
    let mut a = Analyser::new();
    a.result = a.analyse("foreach i {1 2 3} {puts $i}\n", "tcl8.6");
    assert!(a.result.global_scope.variables.contains_key("i"));
}

#[test]
fn process_global_defines_each_name() {
    let mut a = Analyser::new();
    a.result = a.analyse("global x y\n", "tcl8.6");
    assert!(a.result.global_scope.variables.contains_key("x"));
    assert!(a.result.global_scope.variables.contains_key("y"));
}

#[test]
fn process_unknown_command_silently_no_op() {
    let mut a = Analyser::new();
    a.process_command(
        &["my_unknown_command".to_string(), "arg".to_string()],
        &[esc_tok(span(0, 18)), esc_tok(span(19, 22))],
        &[true, true],
        &[],
        &[],
    );
    // No handler matched; no procs, vars, classes, or aliases
    // recorded.
    assert_eq!(a.result.all_procs.len(), 0);
    assert_eq!(a.result.global_scope.variables.len(), 0);
}

#[test]
fn process_empty_argv_is_no_op() {
    let mut a = Analyser::new();
    a.process_command(&[], &[], &[], &[], &[]);
    // No panic, no state mutation.
}

#[test]
fn process_interp_alias_records_target() {
    let mut a = Analyser::new();
    // The handler reads the registry's one command-table transition
    // vocabulary, so a bare harness has to carry a registry the way a
    // real walk does.
    a.registry = Some(std::sync::Arc::clone(
        tcl_registry::model::ingress::static_context_for("tcl").commands(),
    ));
    a.process_command(
        &[
            "interp".to_string(),
            "alias".to_string(),
            String::new(),
            "myset".to_string(),
            String::new(),
            "set".to_string(),
        ],
        &[
            esc_tok(span(0, 6)),
            esc_tok(span(7, 12)),
            str_tok(span(13, 15)),
            esc_tok(span(16, 21)),
            str_tok(span(22, 24)),
            esc_tok(span(25, 28)),
        ],
        &[true, true, true, true, true, true],
        &[],
        &[],
    );
    assert!(a.command_aliases.contains_key("::myset"));
}

#[test]
fn logical_namespace_references_keep_original_words_and_independent_frames() {
    use crate::signature_scan::scope::SignatureNamespaceScope;
    for (source, home) in [
        ("namespace exists Target", None),
        ("namespace eval N {namespace exists Target}", Some("N")),
        (
            "namespace eval N {proc p {} {namespace exists Target}}",
            Some("N"),
        ),
    ] {
        let result = Analyser::new().analyse(source, "tcl");
        let start = u32::try_from(source.find("Target").unwrap()).unwrap();
        let span = Span::new(start, start + 6);
        let references = result
            .namespace_refs
            .iter()
            .filter(|reference| reference.span == span)
            .collect::<Vec<_>>();
        assert_eq!(references.len(), 1, "one actual written operand: {source}");
        let reference = references[0];
        let input = reference
            .original_name_input
            .as_ref()
            .expect("Logical original word");
        assert_eq!(input.bytes(), b"Target");
        let word = input.original_word_key().unwrap().original_word();
        assert_eq!(word.image(), &tcl_lexer::SourceImage::document(source));
        assert_eq!(word.config(), result.body_lexer_config.unwrap());
        assert!(
            crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
                input.policy(),
            )
            .is_none(),
            "Logical word must not acquire Native purpose"
        );
        let SignatureNamespaceScope::C(context) = reference.source_context.as_ref().unwrap() else {
            panic!("Logical C namespace context");
        };
        assert_eq!(
            context.as_segments(),
            home.into_iter()
                .map(tcl_core_types::NameBytes::from)
                .collect::<Vec<_>>()
        );
        let SignatureNamespaceScope::C(target) = reference.source_namespace.as_ref().unwrap()
        else {
            panic!("Logical C namespace target");
        };
        assert_eq!(
            target.as_segments(),
            home.into_iter()
                .chain(["Target"])
                .map(tcl_core_types::NameBytes::from)
                .collect::<Vec<_>>()
        );
        assert!(!result.namespace_name_unknowns.contains(&span), "{source}");
    }
    let result = Analyser::new().analyse("namespace exists {}", "tcl");
    assert_eq!(result.namespace_refs.len(), 1);
    let reference = &result.namespace_refs[0];
    assert_eq!(reference.original_name_input.as_ref().unwrap().bytes(), b"");
    let SignatureNamespaceScope::C(target) = reference.source_namespace.as_ref().unwrap() else {
        panic!("empty original operand selects source root");
    };
    assert_eq!(target.as_segments(), [] as [tcl_core_types::NameBytes; 0]);
    assert_eq!(result.namespace_name_unknowns, [] as [tcl_lexer::Span; 0]);
}

#[test]
fn logical_namespace_references_preserve_alias_written_indices_and_unknowns() {
    let source = "interp alias {} nsExists {} namespace exists; namespace eval N {nsExists Target}";
    let result = Analyser::new().analyse(source, "tcl");
    let start = u32::try_from(source.find("Target").unwrap()).unwrap();
    let references = result
        .namespace_refs
        .iter()
        .filter(|reference| reference.span == Span::new(start, start + 6))
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 1, "alias's actual written NamespaceName");
    let reference = references[0];
    assert_eq!(
        reference.original_name_input.as_ref().unwrap().bytes(),
        b"Target"
    );
    assert_eq!(
        reference
            .source_namespace
            .as_ref()
            .unwrap()
            .display()
            .as_deref(),
        Some("::N::Target")
    );
    assert!(!result.namespace_name_unknowns.contains(&reference.span));

    let source = "interp alias {} nsFixed {} namespace exists Captured; nsFixed";
    let result = Analyser::new().analyse(source, "tcl");
    let call = u32::try_from(source.rfind("nsFixed").unwrap()).unwrap();
    assert!(
        result
            .namespace_refs
            .iter()
            .all(|reference| reference.span.start() < call),
        "captured prefix name has no written operand at the alias call"
    );
    assert!(
        result
            .namespace_name_unknowns
            .iter()
            .all(|span| span.start() < call)
    );

    let source = "proc p {ns} {namespace exists $ns; namespace exists ::other::$ns}";
    for dialect in ["tcl", "tcl8.6"] {
        let result = Analyser::new().analyse(source, dialect);
        assert_eq!(
            result
                .namespace_name_unknowns
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["$ns", "::other::$ns"],
            "{dialect}"
        );
        assert!(
            result.namespace_refs.is_empty(),
            "dynamic source names remain unresolved: {dialect}"
        );
    }
}

#[test]
fn logical_namespace_reference_inputs_require_current_original_owners_and_sites() {
    let source = "namespace eval N {namespace exists Target}";
    let mut analyser = Analyser::new();
    analyser.result = analyser.analyse(source, "tcl");
    let reference = analyser
        .result
        .namespace_refs
        .iter()
        .find(|reference| reference.original_name == "Target")
        .unwrap();
    let span = reference.span;
    let word = reference
        .original_name_input
        .as_ref()
        .unwrap()
        .original_word_key()
        .unwrap()
        .original_word()
        .clone();
    let offset = u32::try_from(source.find("namespace exists").unwrap()).unwrap();
    let (input, context) =
        analyser.original_namespace_reference_input(Some(&word), span, offset, &[]);
    assert_eq!(input.as_ref().unwrap().bytes(), b"Target");
    assert_eq!(
        context.as_ref().unwrap().display().as_deref(),
        Some("::N"),
        "the caller's reporting scope path cannot donate a namespace"
    );
    let (input, context) = analyser.original_namespace_reference_input(Some(&word), span, 0, &[]);
    assert_eq!(input.as_ref().unwrap().bytes(), b"Target");
    assert!(
        context.is_none(),
        "a foreign invocation cannot donate namespace geometry"
    );

    let owner = analyser.result.resolved_input.take().unwrap();
    assert_eq!(
        analyser.original_namespace_reference_input(Some(&word), span, offset, &[]),
        (None, None)
    );
    analyser.result.resolved_input = Some(owner.clone());
    let mut changed = owner.lexer_config();
    changed.strict_quoting = !changed.strict_quoting;
    analyser.result.body_lexer_config = Some(changed);
    assert_eq!(
        analyser.original_namespace_reference_input(Some(&word), span, offset, &[]),
        (None, None)
    );
    analyser.result.body_lexer_config = Some(owner.lexer_config());
    analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        owner.analyser_profile(),
        owner.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        owner.lexer_config(),
    ));
    assert_eq!(
        analyser.original_namespace_reference_input(Some(&word), span, offset, &[]),
        (None, None)
    );
    analyser.result.resolved_input = Some(owner);
    analyser.source = format!("{}{}", analyser.source, ' ').into();
    assert_eq!(
        analyser.original_namespace_reference_input(Some(&word), span, offset, &[]),
        (None, None)
    );
}

fn diag_codes(source: &str, dialect: &str) -> Vec<(String, String)> {
    let mut a = Analyser::new();
    let res = a.analyse(source, dialect);
    res.diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

fn has_code(source: &str, dialect: &str, code: &str) -> bool {
    diag_codes(source, dialect).iter().any(|(c, _)| c == code)
}

/// A stray clause word is reported against the command whose clause
/// grammar owns it — the registry's keyword table, not a list here.
#[test]
fn orphaned_keywords_name_the_grammar_that_owns_them() {
    use tcl_registry::clause_grammar::owner_of_keyword;
    assert_eq!(owner_of_keyword("else"), Some("if"));
    assert_eq!(owner_of_keyword("elseif"), Some("if"));
    assert_eq!(owner_of_keyword("then"), Some("if"));
    assert_eq!(owner_of_keyword("on"), Some("try"));
    assert_eq!(owner_of_keyword("trap"), Some("try"));
    assert_eq!(owner_of_keyword("finally"), Some("try"));
    assert_eq!(owner_of_keyword("set"), None);
}

#[test]
fn w125_fires_for_orphaned_else() {
    // A misplaced newline split the `if` — `else` lands as a standalone
    // command with no parent. `x` is set first: reading an unset `x`
    // fails the `if` and `else` is never reached (tclsh 8.6, 9.0).
    assert!(has_code(
        "set x 1\nif {$x} {\n  puts hi\n}\nelse {\n  puts bye\n}",
        "tcl8.6",
        "W125"
    ));
}

#[test]
fn w125_quiet_for_attached_else() {
    // Properly attached `} else {` never segments `else` as a command.
    assert!(!has_code(
        "if {$x} {\n  puts hi\n} else {\n  puts bye\n}",
        "tcl",
        "W125"
    ));
}

#[test]
fn w125_suppressed_when_user_proc_shadows_keyword() {
    // A user proc named `finally` shadows the keyword — no warning.
    assert!(!has_code(
        "proc finally {} { return }\nfinally",
        "tcl",
        "W125"
    ));
}

#[test]
fn w125_requires_positive_original_slot_absence() {
    for dialect in ["tcl", "tcl8.6", "tcl9.1"] {
        for source in ["else {}", "set ignored [else {}]"] {
            assert!(has_code(source, dialect, "W125"), "{dialect}/{source}");
        }
    }
}

#[test]
fn w125_preserves_actual_occupants_and_unknown_lookup_barriers() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    for source in [
        "proc finally {} {}; finally",
        "rename puts finally; finally VALUE",
        "interp alias {} finally {} puts; finally VALUE",
        "namespace eval N {proc finally {} {}; proc p {} {finally}}",
        "namespace eval P {proc finally {} {}}; namespace eval N {namespace path ::P; finally}",
        "set auto_index(unknown_command) {source lib.tcl}; unknown_command; finally",
        "proc unknown {args} {}; finally",
        "if {$condition} {proc finally {} {}}; finally",
        "namespace eval N {namespace path $paths; finally}",
        "set ::auto_index(unknown_command) {source lib.tcl}; finally [unknown_command]",
    ] {
        for dialect in ["tcl", "tcl8.6", "tcl9.1"] {
            assert!(!has_code(source, dialect, "W125"), "{dialect}/{source}");
        }
    }
}

#[test]
fn w125_original_lookup_ignores_report_maps_and_requires_current_inputs() {
    for (source, absent) in [
        ("finally", true),
        ("interp alias {} finally {} puts; finally VALUE", false),
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, "tcl");
        let input = analyser.result.resolved_input.clone().unwrap();
        let token = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .last()
        .unwrap()
        .argv[0];
        assert_eq!(
            analyser.original_keyword_slot_is_absent("finally", token),
            absent
        );
        analyser.result.all_procs.clear();
        analyser.result.all_classes.clear();
        analyser.result.command_aliases.clear();
        let foreign = Analyser::new().analyse("proc finally {} {}", "tcl");
        analyser
            .result
            .all_procs
            .insert("::finally".into(), foreign.all_procs["::finally"].clone());
        assert_eq!(
            analyser.original_keyword_slot_is_absent("finally", token),
            absent
        );
        analyser.result.diagnostics.clear();
        analyser.emit_proc_resolution_diagnostics("finally", &[], token, &[]);
        assert_eq!(
            analyser
                .result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == DiagCode::W125 }),
            absent
        );
        assert!(!analyser.original_keyword_slot_is_absent("else", token));
        analyser.result.resolved_input = None;
        assert!(!analyser.original_keyword_slot_is_absent("finally", token));
        analyser.result.resolved_input = Some(input.clone());
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        analyser.result.body_lexer_config = Some(config);
        assert!(!analyser.original_keyword_slot_is_absent("finally", token));
        analyser.result.body_lexer_config = Some(input.lexer_config());
        analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            input.lexer_config(),
        ));
        assert!(!analyser.original_keyword_slot_is_absent("finally", token));
        analyser.result.resolved_input = Some(input);
        analyser.source = format!("{}{}", analyser.source, ' ').into();
        assert!(!analyser.original_keyword_slot_is_absent("finally", token));
    }
}

#[test]
fn irule5005_fires_for_direct_proc_call_in_event() {
    let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
    assert!(has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn irule5005_fires_for_direct_proc_call_in_command_substitution() {
    // A direct proc call nested inside a `[…]` substitution still bypasses
    // `call`, so IRULE5005 must reach it via the nested-segment walk.
    let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set x [helper] }";
    assert!(has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn irule5005_quiet_with_call_prefix() {
    let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { call helper }";
    assert!(!has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn malformed_irules_proc_is_not_registered_or_walked() {
    let mut analyser = Analyser::new();
    let result = analyser.analyse(
        "proc malformed {} { pool /Common/inert } extra\n\
             when HTTP_REQUEST { call malformed }",
        "f5-irules",
    );
    assert!(!result.all_procs.contains_key("::malformed"));
    assert!(result.command_invocations.iter().all(|call| {
        call.name != "pool" && call.resolved_qualified_name.as_deref() != Some("::pool")
    }));
}

#[test]
fn irule5005_fires_for_direct_proc_call_from_proc() {
    let src =
        "proc helper {} { return 1 }\nproc caller {} { helper }\nwhen RULE_INIT { call caller }";
    assert!(has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn irule5005_fires_for_direct_proc_call_in_proc_command_substitution() {
    let src = "proc helper {} { return 1 }\nproc caller {} { set x [helper] }\nwhen RULE_INIT { call caller }";
    assert!(has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn irule5005_quiet_for_call_prefix_from_proc() {
    let src = "proc helper {} { return 1 }\nproc caller {} { call helper }\nwhen RULE_INIT { call caller }";
    assert!(!has_code(src, "f5-irules", "IRULE5005"));
}

#[test]
fn irule5007_rejects_direct_or_call_prefixed_proc_at_top_level() {
    for invocation in ["helper", "call helper"] {
        let src = format!("proc helper {{}} {{ return 1 }}\n{invocation}");
        assert!(has_code(&src, "f5-irules", "IRULE5007"));
        assert!(!has_code(&src, "f5-irules", "IRULE5005"));
    }
}

#[test]
fn irule5005_quiet_in_plain_tcl_dialect() {
    let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
    assert!(!has_code(src, "tcl", "IRULE5005"));
}

#[test]
fn irule5005_carries_call_rewrite_fix() {
    let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper x y }";
    let mut a = Analyser::new();
    let res = a.analyse(src, "f5-irules");
    let d = res
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule5005)
        .expect("IRULE5005 expected");
    assert!(d.message.contains("call helper x y"));
    assert_eq!(d.fixes.len(), 1);
    assert_eq!(d.fixes[0].new_text, "call helper");
}

/// A math-function call in an expression resolves to the
/// `::tcl::mathfunc::<name>` command it dispatches to: the written head is
/// the bare tail, the resolved / settled name is the mathfunc command, and
/// the span covers just the function token so a rename rewrites the tail.
#[test]
fn expr_function_call_records_a_mathfunc_invocation() {
    let src = "proc ::tcl::mathfunc::myfunc {x} { return $x }\nexpr {myfunc($a) + 1}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| {
            i.name == "myfunc"
                && i.resolved_qualified_name.as_deref() == Some("::tcl::mathfunc::myfunc")
        })
        .expect("mathfunc invocation recorded");
    assert_eq!(
        &src[inv.range.start() as usize..inv.range.end() as usize],
        "myfunc",
        "span should cover the function-name token only",
    );
    assert_eq!(inv.argc, Some(1), "one argument expression");
    assert!(
        inv.resolution_candidates
            .iter()
            .any(|c| c == "::tcl::mathfunc::myfunc"),
        "settled candidates should include the mathfunc command: {:?}",
        inv.resolution_candidates,
    );
}

/// A math function used before its introducing release is W002 (the
/// `::tcl::mathfunc::<name>` command does not exist in the older core):
/// `min` is 8.5+, the `is*` classification family is 9.0+.
#[test]
fn expr_function_before_its_release_is_disabled_in_dialect() {
    let flags = |src: &str, dialect: &str| {
        let mut a = Analyser::new();
        a.analyse(src, dialect)
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W002)
    };
    assert!(flags("expr {min(1, 2)}\n", "tcl8.4"), "min() is 8.5+");
    assert!(
        !flags("expr {min(1, 2)}\n", "tcl8.6"),
        "min() exists in 8.6"
    );
    assert!(flags("expr {isinf(1.0)}\n", "tcl8.6"), "isinf() is 9.0+");
    assert!(
        !flags("expr {isinf(1.0)}\n", "tcl9.0"),
        "isinf() exists in 9.0"
    );
    // An 8.4-era function is fine everywhere.
    assert!(!flags("expr {abs(-1)}\n", "tcl8.4"), "abs() is 8.4");
}

/// A nested function call (`sqrt(abs($x))`) records both the outer and the
/// inner function as mathfunc invocations.
#[test]
fn expr_nested_function_calls_are_both_recorded() {
    let src = "expr {sqrt(abs($x))}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    for f in ["sqrt", "abs"] {
        assert!(
            r.command_invocations.iter().any(|i| {
                i.name == f
                    && i.resolved_qualified_name.as_deref()
                        == Some(&format!("::tcl::mathfunc::{f}"))
            }),
            "{f} should be recorded: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>(),
        );
    }
}

/// An unrelated ordinary `proc sin` sharing a math function's bare tail
/// name must never hijack `expr {sin(...)}` — the two live in entirely
/// separate command tables in real Tcl (confirmed by the VM's own
/// `tcl::mathfunc::*` dispatch, which never consults an ordinary
/// top-level command). Before the two-candidate resettlement rule
/// below, `finalise_invocation_resolutions`'s generic one-hop
/// `{ns}::{name}` suffix-strip misparsed the mathfunc-qualified
/// `::tcl::mathfunc::sin` as if `::tcl::mathfunc` were the *calling*
/// namespace, and — once `sin` was `known` only as the unrelated
/// global proc — silently rewrote the resolved name to `::sin`.
#[test]
fn expr_function_call_ignores_unrelated_same_named_global_proc() {
    let src = "proc sin {x} { return bogus }\nset y [expr {sin(1.0)}]\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "sin" && i.argc == Some(1))
        .expect("mathfunc invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::tcl::mathfunc::sin"),
        "the unrelated proc sin must not hijack expr's sin(...): {inv:?}",
    );
}

/// TIP 232: a namespace-local `proc ::ns::tcl::mathfunc::f` shadows the
/// global `::tcl::mathfunc::f` for a call made from inside `::ns` —
/// confirmed real Tcl behaviour by the VM's
/// `namespace_local_mathfunc_shadows_global_in_expr`
/// (`tcl-vm-it/tests/it_3/tricky_resolution_e2e.rs`). The analyser must settle
/// the call to the local override, not the fixed global form.
#[test]
fn expr_function_call_resolves_namespace_local_mathfunc_override() {
    let src = "namespace eval ::nsa::tcl::mathfunc {}\n\
                    proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "pf")
        .expect("mathfunc invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::nsa::tcl::mathfunc::pf"),
        "pf(1) inside ::nsa must resolve to the local override: {inv:?}",
    );
    assert_eq!(
        inv.resolution_candidates,
        vec!["::nsa::tcl::mathfunc::pf", "::tcl::mathfunc::pf"],
        "local-first, then global, matching the VM's own search order",
    );
}

/// The companion of the override case above: no local override exists at
/// `::nsa::tcl::mathfunc::pf`, only a *global* user-defined one — the
/// call still finds it via the two-candidate rule's fallback step.
#[test]
fn expr_function_call_falls_back_to_global_user_override_from_a_namespace() {
    let src = "proc ::tcl::mathfunc::pf {x} { return 10 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "pf")
        .expect("mathfunc invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::tcl::mathfunc::pf"),
        "no local override at ::nsa -- must fall back to the global proc: {inv:?}",
    );
}

/// The everyday case on the namespace-aware resolution path: a built-in
/// (`sin`), called from inside a namespace with no override anywhere,
/// must still settle to the global built-in slot — the
/// collision/shadowing rules above must not disturb the common
/// no-namespace, no-override call the diagnostic layer covers.
#[test]
fn expr_function_call_resolves_builtin_from_inside_a_namespace() {
    let src = "namespace eval ::nsa {\n    proc caller {} { return [expr {sin(1.0)}] }\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "sin")
        .expect("mathfunc invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::tcl::mathfunc::sin"),
        "a built-in with no override anywhere must settle globally: {inv:?}",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "must still draw no W123: {:?}",
        r.diagnostics,
    );
}

/// TIP 232 math functions are ordinary commands, so `namespace path`
/// applies to their resolution exactly as it does to any other command —
/// confirmed against the VM's own `resolve_command_fqn`, which routes
/// every lookup (mathfunc calls included) through the same `ns_paths`-
/// aware resolver. Neither the caller's own namespace nor the global
/// slot defines `triple`; only the `namespace path` entry does.
#[test]
fn expr_function_call_honours_namespace_path() {
    let src = "namespace eval ::libns::tcl::mathfunc {}\n\
                    proc ::libns::tcl::mathfunc::triple {x} { return [expr {$x * 3}] }\n\
                    namespace eval ::consumer {\n    \
                        namespace path ::libns\n    \
                        proc caller {} { return [expr {triple(2)}] }\n\
                    }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let inv = r
        .command_invocations
        .iter()
        .find(|i| i.name == "triple")
        .expect("mathfunc invocation recorded");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::libns::tcl::mathfunc::triple"),
        "must settle via the namespace path entry: {inv:?}",
    );
    assert_eq!(
        inv.resolution_candidates,
        vec![
            "::consumer::tcl::mathfunc::triple",
            "::libns::tcl::mathfunc::triple",
            "::tcl::mathfunc::triple",
        ],
        "current namespace, then the path entry, then global",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "must draw no W123: {:?}",
        r.diagnostics,
    );
}

/// [`Analyser::analyse_list_quoted_body`]'s *variable-reference*
/// recording, isolated at the analyser tier.
///
/// A gate that treated any non-`Str` body token as an opaque barrier
/// would leave a script argument **built** with `list` — Tk's own
/// `library/tk.tcl` writes `namespace eval :: [list source [file join
/// $::tk_library $file.tcl]]` — without `record_arg_var_reads` running
/// inside it, so the enclosing proc's parameter would look unread from its
/// own scope.  This pins the analyser primitive that find-references and
/// semantic tokens are built on.
///
/// Oracle (tclsh 8.6.16 and 9.0.4): the parameter deterministically
/// drives which file is sourced, so the read is real.
#[test]
fn a_list_built_body_records_its_parameter_reads_923_idx102() {
    // The read lands on the *scope tree*'s copy of the parameter — the
    // binding the LSP's variable providers resolve through.
    fn find<'a>(
        scope: &'a super::super::types::Scope,
        name: &str,
    ) -> Option<&'a super::super::types::VarDef> {
        scope
            .variables
            .get(name)
            .or_else(|| scope.children.iter().find_map(|c| find(c, name)))
    }
    let src = "proc ::app::SourceLibFile {file} {\n    \
                   namespace eval :: [list source [file join $::app::lib_dir $file.tcl]]\n}\n";
    let r = Analyser::new().analyse(src, "tcl8.6").clone();
    let param = find(&r.global_scope, "file")
        .expect("the `file` parameter must be recorded in the proc scope");
    // The recorded span covers the whole `$file` substitution, `$` included.
    let read_start = u32::try_from(src.find("$file.tcl").expect("the read is in the source"))
        .expect("offset fits u32");
    assert!(
        param.references.iter().any(|s| s.start() == read_start),
        "the `$file` read inside the list-built body must be recorded on the \
             parameter (expected a reference at {read_start}): {:?}",
        param.references,
    );
}

/// TN control for the above — a body that is genuinely dynamic
/// (`[gen]`, not a statically known `list`-built command) stays the
/// opaque barrier it always was, so nothing inside it is attributed.
#[test]
fn a_dynamic_body_records_no_parameter_reads_923_idx102() {
    let src = "proc ::app::Run {file} {\n    namespace eval :: [gen $file]\n}\n";
    let r = Analyser::new().analyse(src, "tcl8.6").clone();
    // The `$file` inside `[gen $file]` is a real argument read of the
    // enclosing command substitution and is recorded once; what must not
    // happen is the `[gen …]` body being *walked* as a script.
    assert!(
        !r.command_invocations
            .iter()
            .any(|i| i.name == "source" || i.name == "namespace eval"),
        "a dynamic body must not be walked as a script: {:?}",
        r.command_invocations
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>(),
    );
}
