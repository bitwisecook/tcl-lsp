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
use std::fmt::Write as _;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

pub(super) fn whole_document_range(source: &str) -> LspRange {
    let line_count = source.lines().count().max(1);
    LspRange {
        start_line: 0,
        start_character: 0,
        end_line: u32::try_from(line_count - 1).unwrap_or(0),
        end_character: u32::MAX,
    }
}

#[test]
fn empty_hints_when_analysis_is_none() {
    let hints = inlay_hints(
        "set x 1\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range("set x 1\n"),
        None,
        None,
        false,
        true,
    );
    assert_eq!(hints, [] as [crate::inlay_hints::InlayHint; 0]);
}

#[test]
fn hints_emitted_for_user_proc_call() {
    let src = "proc greet {name greeting} {}\ngreet alice hello\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(
        labels.contains(&"name:"),
        "expected `name:` hint; got {labels:?}",
    );
    assert!(
        labels.contains(&"greeting:"),
        "expected `greeting:` hint; got {labels:?}",
    );
}

#[test]
fn hints_anchored_at_argument_start() {
    let src = "proc greet {name} {}\ngreet alice\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    // Line 0 carries the `proc` command's own built-in hints.
    let hints: Vec<_> = hints.into_iter().filter(|h| h.position_line == 1).collect();
    assert_eq!(hints.len(), 1);
    let h = &hints[0];
    assert_eq!(h.position_line, 1);
    // `greet ` is 6 chars; `alice` starts at column 6.
    assert_eq!(h.position_character, 6);
    assert_eq!(h.label, "name:");
}

#[test]
fn no_hints_for_unknown_command() {
    let src = "unknown_cmd a b c\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    assert!(hints.is_empty(), "{hints:?}");
}

#[test]
fn no_hints_for_args_tail_parameter() {
    // `args` is the variadic-tail collector — we skip it
    // because there's no individual name to surface.
    let src = "proc many {first args} {}\nmany 1 2 3 4\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    // Only the `first` arg gets a hint; line 0 is the `proc` command's own.
    let hints: Vec<_> = hints.into_iter().filter(|h| h.position_line == 1).collect();
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].label, "first:");
}

#[test]
fn qualified_call_hints_from_namespaced_proc() {
    // A qualified `A::greet` call at global scope resolves to the proc in
    // `::A`, so its parameter name (`alpha`) drives the hint — the
    // qualified name is honoured, not tail-matched to a same-named proc.
    let src = "namespace eval A {\n    proc greet {alpha} {}\n}\nA::greet x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&registry),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(labels.contains(&"alpha:"), "{labels:?}");
}

#[test]
fn namespaced_proc_does_not_hijack_builtin_hints_from_global() {
    // A `proc incr` buried in `::foo` must not supply argument hints for an
    // `incr` call in the global namespace — C Tcl resolves the builtin
    // there, so the builtin's `varName` hint surfaces, never the proc's
    // `custom` parameter.
    let src = "namespace eval foo {\n    proc incr {custom} {}\n}\nincr y\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&registry),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(
        !labels.contains(&"custom:"),
        "proc hijacked builtin: {labels:?}"
    );
}

#[test]
fn extra_args_past_param_count_not_hinted() {
    // `proc one {a} {}` then `one 1 2 3` — the call binds no formal at
    // all: every release raises `wrong # args: should be "one a"`
    // (tclsh 8.4-9.1, jimsh), so no argument carries a parameter name.
    let src = "proc one {a} {}\none 1 2 3\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    assert!(hints.iter().all(|h| h.position_line != 1), "{hints:?}");
}

#[test]
fn hints_filtered_by_range() {
    // Three lines, each with a proc call.  Range covers
    // only line 2.
    let src = "proc greet {name} {}\ngreet alice\ngreet bob\ngreet charlie\n";
    let analysis = analyse(src);
    let range = LspRange {
        start_line: 2,
        start_character: 0,
        end_line: 2,
        end_character: u32::MAX,
    };
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        range,
        Some(&analysis),
        None,
        false,
        true,
    );
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert_eq!(hints[0].position_line, 2);
}

/// A command whose span falls entirely outside the
/// requested range must never reach original procedure selection or the registry
/// synopsis lookup. There is no call counter to assert against through
/// the public API, so this is parity: the narrow-range result must equal
/// the in-range subset of the full-range result, for a document whose
/// out-of-range lines exercise *both* resolution branches the skip has to
/// short-circuit — real builtin calls that would otherwise emit their own
/// hints (lines 2 and 3, `puts`, two positionals) and an unresolvable
/// call (line 5). The unresolvable call comes last: run earlier, it would
/// stop the script before line 4 in every release.
#[test]
fn out_of_range_segments_are_skipped_without_changing_in_range_results() {
    let src =
        "proc greet {name} {}\ngreet alice\nputs stdout a\nputs stdout b\ngreet zed\nnosuch\n";
    let analysis = analyse(src);
    let reg = registry();
    let full = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 10,
        end_character: 0,
    };
    let narrow = LspRange {
        start_line: 4,
        start_character: 0,
        end_line: 4,
        end_character: u32::MAX,
    };
    let full_hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        full,
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let narrow_hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        narrow,
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let expected: Vec<_> = full_hints
        .into_iter()
        .filter(|h| h.position_line == 4)
        .collect();
    assert_eq!(narrow_hints, expected);
    assert_eq!(narrow_hints.len(), 1, "{narrow_hints:?}");
}

// built-in command synopsis hints

fn registry() -> tcl_registry::CommandRegistry {
    tcl_registry::CommandRegistry::build_default()
}

/// `walk_scope_type_hints` recurses once per nested namespace/proc
/// scope, capped by `MAX_SCOPE_WALK_DEPTH` (`crate::lib`). 80 nested
/// `namespace eval` levels is past the point (confirmed empirically:
/// 100+) where unguarded namespace-scope recursion overflows `cargo
/// test`'s bare ~2 MiB per-test default. The assertion is that
/// `inlay_hints` returns at all, not what it returns.
#[test]
fn deeply_nested_namespaces_survive_type_hints() {
    const DEPTH: usize = 80;
    let mut src = String::new();
    for i in 0..DEPTH {
        let _ = writeln!(src, "namespace eval ns{i} {{");
    }
    src.push_str("proc leaf {} { set x 1 }\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    let analysed = src.clone();
    let analysis = crate::on_worker_stack(move || analyse(&analysed));
    let reg = registry();
    let _ = inlay_hints(
        &src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(&src),
        Some(&analysis),
        Some(&reg),
        true,
        false,
    );
}

#[test]
fn synopsis_groups_rejoins_optional_flag_value() {
    let g = synopsis_groups("string compare ?-nocase? ?-length length? a b");
    assert_eq!(
        g,
        vec![
            "string",
            "compare",
            "?-nocase?",
            "?-length length?",
            "a",
            "b"
        ],
    );
}

#[test]
fn param_names_skips_flags_and_keeps_positionals() {
    // After `string compare`, the flags drop out leaving the
    // two required positionals.
    let names = param_names_from_synopsis(
        "string compare ?-nocase? ?-length length? string1 string2",
        2,
    );
    assert_eq!(
        names,
        vec![
            ("string1".to_string(), false),
            ("string2".to_string(), false)
        ],
    );
}

#[test]
fn param_names_stops_at_varargs() {
    let names = param_names_from_synopsis("string cat ?string1? ?string2 ...?", 2);
    // `?string1?` is an optional positional; `?string2 ...?`
    // is varargs → stop.
    assert_eq!(names, vec![("string1".to_string(), true)]);
}

#[test]
fn param_names_tags_optional_and_required() {
    // `puts ?-nonewline? ?channelId? string` — the flag drops,
    // `channelId` is optional, `string` is required.
    let names = param_names_from_synopsis("puts ?-nonewline? ?channelId? string", 1);
    assert_eq!(
        names,
        vec![
            ("channelId".to_string(), true),
            ("string".to_string(), false),
        ],
    );
}

#[test]
fn param_names_drops_flag_group_placeholders() {
    // `?options?` / `?switches?` are flag-group docs, not params.
    let names = param_names_from_synopsis("cmd ?options? path", 1);
    assert_eq!(names, vec![("path".to_string(), false)]);
    let names = param_names_from_synopsis("cmd ?switches? path", 1);
    assert_eq!(names, vec![("path".to_string(), false)]);
}

#[test]
fn param_names_skips_optional_varargs_flag_group_then_labels_positionals() {
    // `lsearch ?option ...? list pattern` (the actual registry hover
    // synopsis) — the `?option ...?` flag-group placeholder must be
    // skipped, NOT treated as a hard varargs stop that drops the real
    // `list` / `pattern` positionals after it.
    let names = param_names_from_synopsis("lsearch ?option ...? list pattern", 1);
    assert_eq!(
        names,
        vec![("list".to_string(), false), ("pattern".to_string(), false)],
    );
    // A *non-optional* `...` repeat marker still stops the parse, after
    // the preceding plain positional is labelled.
    let names = param_names_from_synopsis("cmd first ...", 1);
    assert_eq!(names, vec![("first".to_string(), false)]);
}

#[test]
fn select_prefers_required_trailing_positional() {
    // One supplied arg against `?channelId? string` binds to the
    // required `string`, not the leading optional `channelId`.
    let params = vec![
        ("channelId".to_string(), true),
        ("string".to_string(), false),
    ];
    assert_eq!(select_param_names(&params, 1), vec!["string"]);
    // Two args fill the optional then the required.
    assert_eq!(select_param_names(&params, 2), vec!["channelId", "string"]);
}

#[test]
fn builtin_hint_one_arg_labels_required_trailing_positional() {
    // `puts hello` — one positional arg against
    // `puts ?-nonewline? ?channelId? string` labels it `string:`,
    // not the leading optional `channelId:`.
    let src = "puts hello\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert_eq!(labels, vec!["string:"], "{hints:?}");
}

#[test]
fn builtin_hint_two_args_fill_optional_then_required() {
    // `puts stderr hello` — two positionals fill `channelId` then
    // `string`.
    let src = "puts stderr hello\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert_eq!(labels, vec!["channelId:", "string:"], "{hints:?}");
}

#[test]
fn builtin_hint_for_subcommand_positional() {
    // `string index $s 3` — subcommand `index`, synopsis
    // `string index string charIndex`.
    let src = "string index $s 3\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(labels.contains(&"string:"), "{hints:?}");
    assert!(labels.contains(&"charIndex:"), "{hints:?}");
}

#[test]
fn builtin_hint_skips_call_site_flags() {
    // `string compare -nocase $a $b` — `-nocase` is a flag,
    // so $a→string1, $b→string2.
    let src = "string compare -nocase $a $b\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    // The flag token shouldn't be labelled.
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(labels.contains(&"string1:"), "{hints:?}");
    assert!(labels.contains(&"string2:"), "{hints:?}");
    // No hint anchored on the `-nocase` flag.
    for h in &hints {
        assert_ne!(h.position_character, 15, "flag should not be hinted: {h:?}");
    }
}

#[test]
fn builtin_hint_treats_negative_number_as_positional() {
    // `string index $s -1` — the `index` subcommand declares no
    // `-1` option, so the registry-driven walk keeps `-1` as the
    // `charIndex` positional rather than skipping it as a flag.
    let src = "string index $s -1\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(labels.contains(&"string:"), "{hints:?}");
    assert!(labels.contains(&"charIndex:"), "{hints:?}");
}

#[test]
fn builtin_hint_consumes_value_taking_option() {
    // `string compare -length 3 $a $b` — `-length` takes a value,
    // so both `-length` and `3` are skipped; $a→string1, $b→string2.
    let src = "string compare -length 3 $a $b\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    // Only the two positionals are labelled — `-length` and its
    // value `3` are both consumed.
    assert_eq!(labels, vec!["string1:", "string2:"], "{hints:?}");
}

#[test]
fn builtin_hints_use_the_analysis_registry_without_a_caller_registry() {
    // Same source, no caller registry: the analysis carries its own
    // command store, so the built-in hints still come from it.
    let src = "string index $s 3\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert_eq!(labels, vec!["string:", "charIndex:"], "{hints:?}");
}

#[test]
fn user_proc_takes_precedence_over_builtin() {
    // A user proc named `string` (contrived) wins over the
    // built-in.  Here we just confirm a user proc still
    // gets its param hints when a registry is also present.
    let src = "proc greet {name} {}\ngreet alice\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    let labels: Vec<&str> = hints.iter().map(|h| h.label.as_str()).collect();
    assert!(labels.contains(&"name:"), "{hints:?}");
}

// inferred type hints (InlayHintKind::Type)

#[test]
fn same_named_vars_in_different_procs_do_not_bleed_types() {
    // `x` is Int in `a` and a String in `b`. The type hint
    // for each `x` must reflect its own proc, not last-writer-wins across a
    // flat name→type map.
    let src = "proc a {} { set x 42 }\nproc b {} { set x \"hi\" }\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        true,
        false,
    );
    let labels_on_line = |line: u32| -> Vec<String> {
        hints
            .iter()
            .filter(|h| h.kind == InlayHintKind::Type && h.position_line == line)
            .map(|h| h.label.clone())
            .collect()
    };
    // Line 0: proc a's `x` is int. Line 1: proc b's `x` is str.
    assert!(
        labels_on_line(0).iter().any(|l| l == ": int"),
        "proc a's x should be int: {:?}",
        labels_on_line(0),
    );
    assert!(
        labels_on_line(0).iter().all(|l| l != ": str"),
        "proc b's str type must not bleed onto proc a: {:?}",
        labels_on_line(0),
    );
    assert!(
        labels_on_line(1).iter().all(|l| l != ": int"),
        "proc a's int type must not bleed onto proc b: {:?}",
        labels_on_line(1),
    );
}

#[test]
fn type_hint_for_integer_set() {
    // `set x 42` → a `: int` Type-kind hint immediately after `x`
    // (character 5).
    let src = "set x 42\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        true,
        false,
    );
    let type_hints: Vec<&InlayHint> = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Type)
        .collect();
    assert!(
        type_hints
            .iter()
            .any(|h| h.label == ": int" && h.position_character == 5),
        "{type_hints:?}",
    );
    // No parameter hints leak in when only type hints are requested.
    assert!(
        hints.iter().all(|h| h.kind == InlayHintKind::Type),
        "{hints:?}"
    );
}

#[test]
fn type_hints_use_the_analysis_registry_without_a_caller_registry() {
    // No caller registry: the analysis's own command store builds the
    // type map.
    let src = "set x 42\n";
    let analysis = analyse(src);
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        None,
        true,
        false,
    );
    assert!(
        hints
            .iter()
            .any(|h| h.kind == InlayHintKind::Type && h.label == ": int"),
        "{hints:?}",
    );
}

#[test]
fn type_and_parameter_families_gate_independently() {
    // `set x 42` carries a type hint; `add 1 2` carries parameter hints.
    let src = "proc add {a b} {}\nset x 42\nadd 1 2\n";
    let analysis = analyse(src);
    let reg = registry();
    let type_only = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        true,
        false,
    );
    assert!(!type_only.is_empty(), "expected type hints");
    assert!(
        type_only.iter().all(|h| h.kind == InlayHintKind::Type),
        "{type_only:?}",
    );
    let param_only = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        true,
    );
    assert!(!param_only.is_empty(), "expected parameter hints");
    assert!(
        param_only
            .iter()
            .all(|h| h.kind == InlayHintKind::Parameter),
        "{param_only:?}",
    );
}

#[test]
fn both_families_disabled_yields_nothing() {
    let src = "set x 42\nadd 1 2\n";
    let analysis = analyse(src);
    let reg = registry();
    let hints = inlay_hints(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        false,
        false,
    );
    assert!(hints.is_empty(), "{hints:?}");
}

// format-string specifier hints (InlayHintKind::Type)

fn type_labels(src: &str) -> Vec<(u32, String)> {
    type_labels_for_dialect(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
    )
}

fn type_labels_for_dialect(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<(u32, String)> {
    let analysis = analyse(src);
    let reg = registry();
    inlay_hints(
        src,
        dialect,
        whole_document_range(src),
        Some(&analysis),
        Some(&reg),
        true,
        false,
    )
    .into_iter()
    .filter(|h| h.kind == InlayHintKind::Type)
    .map(|h| (h.position_character, h.label))
    .collect()
}

#[test]
fn format_sprintf_specifier_hints() {
    let labels = type_labels("format \"%d-%s\" 1 two\n");
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"int"), "{labels:?}");
    assert!(names.contains(&"str"), "{labels:?}");
}

#[test]
fn format_literal_percent_not_hinted() {
    // `%%` is a literal percent — no specifier hint.
    let labels = type_labels("format \"100%%\"\n");
    assert!(labels.is_empty(), "{labels:?}");
}

#[test]
fn clock_format_specifier_hints() {
    let labels = type_labels("clock format $t -format \"%Y-%m-%d\"\n");
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"year"), "{labels:?}");
    assert!(names.contains(&"month"), "{labels:?}");
    assert!(names.contains(&"day"), "{labels:?}");
}

#[test]
fn binary_format_specifier_hints() {
    let labels = type_labels("binary format \"a3 i\" foo 1\n");
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"strN"), "{labels:?}");
    assert!(names.contains(&"i32le"), "{labels:?}");
}

#[test]
fn inlay_hints_label_a_computed_format_string() {
    // A literal `format "%d" 1` already labels its specifier;
    // the lattice proves the same text when it reaches the word
    // through a variable, so the computed spelling must not go blind
    // (there is no in-place span of its own, so the label anchors at
    // the `$fmt` token's own end instead of a content offset).
    let labels = type_labels("set fmt \"%d\"\nformat $fmt 1\n");
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"int"), "{labels:?}");

    // Negative: an unproven `$fmt` (a bare parameter, never assigned a
    // literal) proves nothing, so no hint is fabricated.
    let labels = type_labels("proc p {fmt} {\n    format $fmt 1\n}\n");
    assert!(labels.is_empty(), "{labels:?}");
}

#[test]
fn binary_q_and_q_hints_are_owned_by_shared_spec_table() {
    let labels = type_labels_for_dialect(
        "binary format \"q Q\" 1 2\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"f64le"), "{labels:?}");
    assert!(names.contains(&"f64be"), "{labels:?}");
}

// `ss` is two short-integer fields on every release (tclsh 8.4.20 through
// 9.0.4 all pack four bytes), so it earns two hints — the shared owner
// used to swallow the second `s` as a signedness modifier and emit one.
#[test]
fn short_specifier_pair_gets_a_hint_each() {
    let labels = type_labels_for_dialect(
        "binary format ss 1 2\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(
        names.iter().filter(|n| **n == "i16le").count(),
        2,
        "{labels:?}"
    );
}

#[test]
fn regsub_subspec_backreference_hints() {
    let labels = type_labels("regsub {(a)(b)} $s {\\1-\\2} out\n");
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(names.contains(&"grp1"), "{labels:?}");
    assert!(names.contains(&"grp2"), "{labels:?}");
}

#[test]
fn dynamic_leading_option_does_not_claim_regsub_subspec_hints() {
    let dynamic = type_labels("regsub $mode {a} $value {\\1}\n");
    assert!(
        !dynamic.iter().any(|(_, label)| label == "grp1"),
        "a dynamic leading word can shift regsub's replacement: {dynamic:?}"
    );

    for src in [
        "regsub -c {a} $value {\\1}\n",
        "regsub -command {a} $value callback\n",
    ] {
        let labels = type_labels(src);
        assert!(
            !labels.iter().any(|(_, label)| label == "grp1"),
            "{src:?}: only a valid replacement template has capture hints: {labels:?}"
        );
    }

    let fixed = type_labels("regsub -start $start {a} $value {\\1}\n");
    assert!(
        fixed.iter().any(|(_, label)| label == "grp1"),
        "a declared -start value keeps the replacement layout fixed: {fixed:?}"
    );
}

/// Format hints follow the head's *effective command
/// identity*, so a proven `interp alias` / `rename` of a format-family
/// command hints like the original — and a spelling whose binding was
/// taken over hints not at all.
///
/// tclsh-proof (9.0.4 / 8.6.16): `interp alias {} myfmt {} format;
/// myfmt "%d-%s" 1 two` -> `1-two`; after `proc format {args} {return
/// USER}`, `format "%d"` -> `USER`.
#[test]
fn format_hints_follow_the_effective_command_identity() {
    let names_of =
        |src: &str| -> Vec<String> { type_labels(src).into_iter().map(|(_, l)| l).collect() };
    // TP — an aliased and a renamed head both hint like the original.
    for src in [
        "interp alias {} myfmt {} format\nmyfmt \"%d-%s\" 1 two\n",
        "rename format myfmt\nmyfmt \"%d-%s\" 1 two\n",
    ] {
        let names = names_of(src);
        assert!(
            names.iter().any(|n| n == "int") && names.iter().any(|n| n == "str"),
            "`{}` must hint like a direct format call, got {names:?}",
            src.lines().next().unwrap_or_default()
        );
    }
    // FP — a top-level `proc` that shadows the built-in takes the name
    // over, so its `%d` is an ordinary string.
    assert!(
        names_of("proc format {args} { return USER }\nformat \"%d-%s\" 1 two\n").is_empty(),
        "a shadowed `format` must not be hinted"
    );
    // TN — an unprovable binding states nothing, so the name stays an
    // ordinary unknown command.
    assert!(
        names_of("interp alias {} myfmt {} $target\nmyfmt \"%d-%s\" 1 two\n").is_empty(),
        "a dynamic alias target must not confer format hints"
    );
}
