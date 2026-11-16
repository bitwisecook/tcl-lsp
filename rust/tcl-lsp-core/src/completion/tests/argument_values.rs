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

//! Argument-position candidates: subcommands, switches, event names and enumerated values.

use super::*;

#[test]
fn irules_subcommand_completion_uses_configured_bigip_version() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let profile = tcl_dialect::DialectProfile::irules();
    let c3d = registry.get("SSL::c3d").expect("SSL::c3d spec");

    let mut analysis = AnalysisResult::default();
    analysis.library_versions.bigip_version = Some("21.0.0".to_owned());
    let labels_21_0: Vec<String> = subcommand_completions(c3d, &analysis, profile, "")
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert!(!labels_21_0.iter().any(|label| label == "cert_lifespan"));
    assert!(!labels_21_0.iter().any(|label| label == "cert_start_date"));

    analysis.library_versions.bigip_version = Some("21.1.0".to_owned());
    let labels_21_1: Vec<String> = subcommand_completions(c3d, &analysis, profile, "")
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert!(labels_21_1.iter().any(|label| label == "cert_lifespan"));
    assert!(labels_21_1.iter().any(|label| label == "cert_start_date"));
}

// subcommand completion
//
// When the cursor sits at word-index 1 of a known command
// whose spec declares non-empty `subcommands`, the
// completion surface lists subcommand names (not user procs
// or unrelated built-ins).

#[test]
fn subcommand_completion_surfaces_subcommands_at_word_index_1() {
    // `string l` — partial `l`, cursor at word-index 1 of
    // `string`.  The registry declares many `string`
    // subcommands; expect `length` and similar `l*` ones.
    let src = "string l\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"length"),
        "expected `length` subcommand; got {labels:?}",
    );
    // No proc / unrelated commands should leak through.
    assert!(
        !labels.contains(&"puts"),
        "subcommand context should not include `puts`; got {labels:?}",
    );
}

#[test]
fn sub_subcommand_completion_surfaces_at_word_index_2() {
    // After `info object ` (word-index 2), offer the
    // OBJECT INTROSPECTION operations with their descriptions.
    let src = "info object \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"class"),
        "expected `class`; got {labels:?}"
    );
    assert!(
        labels.contains(&"methods"),
        "expected `methods`; got {labels:?}"
    );
    // Descriptions are surfaced as detail.
    assert!(
        items
            .iter()
            .find(|i| i.label == "class")
            .and_then(|i| i.detail.as_deref())
            .is_some_and(|d| !d.is_empty()),
        "expected a detail on the `class` op",
    );
    // Partial filters: `info class super` → `superclasses`.
    let src = "info class super\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        16,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels,
        vec!["superclasses"],
        "prefix should filter; got {labels:?}"
    );
}

#[test]
fn subcommand_completion_lists_all_subcommands_with_empty_partial() {
    // `string ` (cursor just past the space) — empty partial,
    // word-index 1 of `string`.  Should list every `string`
    // subcommand, alphabetically.
    let src = "string \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // `string` has at minimum `length`, `match`, `range`,
    // `tolower`, `toupper` across all dialects.
    assert!(labels.contains(&"length"), "{labels:?}");
    assert!(labels.contains(&"match"), "{labels:?}");
    // Should be sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted, "expected sorted subcommands");
}

#[test]
fn subcommand_completion_falls_through_when_word_index_not_1() {
    // `string length f` — cursor at word-index 2, the string operand.
    // Subcommand completion must not fire, and a data operand offers no
    // command names either.
    let src = "string length f\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.iter().any(|l| l.starts_with('f')),
        "a data operand offers no command names; got {labels:?}",
    );
    // String's `length` subcommand should NOT be in the
    // result — we're past word-index 1.
    assert!(
        !labels.contains(&"length"),
        "should not surface `length` subcommand at word-index 2; got {labels:?}",
    );
}

#[test]
fn subcommand_completion_skipped_when_command_has_no_subcommands() {
    // `puts hel` — `puts` declares no subcommands, and `hel` is the
    // string it writes: data, where no procedure name is offered.
    let src = "proc helper {} {}\nputs hel\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"helper"),
        "a data operand offers no procedure names; got {labels:?}",
    );
}

// switch completion
//
// When the partial starts with `-` and the surrounding
// command's spec declares matching options, the completion
// surface lists option names (not subcommands or built-ins).

#[test]
fn switch_completion_surfaces_options_for_dash_partial() {
    // `lsearch -n` — partial `-n`.  The `lsearch` spec
    // declares many `-...` options; expect at least one
    // starting with `-n` (`-nocase`).
    let src = "lsearch -n\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.iter().any(|l| l.starts_with("-n")),
        "expected at least one `-n*` switch; got {labels:?}",
    );
    // No unrelated commands / procs.
    assert!(
        !labels.iter().any(|l| !l.starts_with('-')),
        "all switch-completion items must start with '-'; got {labels:?}",
    );
}

#[test]
fn switch_completion_lists_all_options_for_bare_dash() {
    // `lsearch -` — partial `-`, every option should
    // surface.
    let src = "lsearch -\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Every result must start with `-`.
    for l in &labels {
        assert!(l.starts_with('-'), "non-switch in result: {l}");
    }
    // Sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
}

#[test]
fn switch_completion_gates_options_by_package_version() {
    // `entry -placeholder` was introduced in Tk 8.7.  Completion must gate
    // it on the version resolved from `package require Tk <req>`.
    let registry = CommandRegistry::build_default();
    let older = "package require Tk 8.6\nentry .e -p\n";
    let a1 = analyse(older);
    let l1: Vec<String> = completions(
        older,
        1,
        11,
        &a1,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !l1.iter().any(|l| l == "-placeholder"),
        "Tk 8.6 must not offer -placeholder: {l1:?}",
    );

    let newer = "package require Tk 8.7\nentry .e -p\n";
    let a2 = analyse(newer);
    let l2: Vec<String> = completions(
        newer,
        1,
        11,
        &a2,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l2.iter().any(|l| l == "-placeholder"),
        "Tk 8.7 must offer -placeholder: {l2:?}",
    );
}

#[test]
fn switch_completion_gates_options_by_dialect() {
    // `lsearch -stride` is Tcl 9.0+ (see the W004 diagnostic for the same
    // gate).  Completion must not suggest it under an older dialect, and
    // must once the document's dialect supports it.
    let registry = CommandRegistry::build_default();
    let src = "lsearch -s {a b} x\n";
    let old_analysis = analyse_for(src, "tcl8.6");
    let old: Vec<String> = completions(
        src,
        0,
        10,
        &old_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !old.iter().any(|l| l == "-stride"),
        "tcl8.6 must not offer -stride: {old:?}",
    );
    let new_analysis = analyse_for(src, "tcl9.0");
    let new: Vec<String> = completions(
        src,
        0,
        10,
        &new_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        new.iter().any(|l| l == "-stride"),
        "tcl9.0 must offer -stride: {new:?}",
    );
}

#[test]
fn switch_completion_resolves_subcommand_scoped_options() {
    // `chan configure -inputmode` is a subcommand-scoped, Tcl 9.0+
    // option that lives only on the `configure` SubCommand's own table —
    // absent from `chan`'s top-level option table entirely.  Completion
    // must resolve the typed subcommand to find it, and still gate it by
    // dialect exactly like W004 does.
    let registry = CommandRegistry::build_default();
    let src = "chan configure $chan -i\n";
    let old_analysis = analyse_for(src, "tcl8.6");
    let old: Vec<String> = completions(
        src,
        0,
        23,
        &old_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !old.iter().any(|l| l == "-inputmode"),
        "tcl8.6 must not offer -inputmode: {old:?}",
    );
    let new_analysis = analyse_for(src, "tcl9.0");
    let new: Vec<String> = completions(
        src,
        0,
        23,
        &new_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        new.iter().any(|l| l == "-inputmode"),
        "tcl9.0 must offer -inputmode: {new:?}",
    );
}

#[test]
fn event_completion_surfaces_when_handler_first_arg() {
    // `when HT` inside iRules — should list every event
    // starting with `HT` (HTTP_REQUEST, HTTP_RESPONSE, …).
    let src = "when HT\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-irules").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"HTTP_REQUEST"),
        "expected `HTTP_REQUEST`; got {labels:?}",
    );
    assert!(
        labels.iter().all(|l| l.starts_with("HT")),
        "all results should start with `HT`; got {labels:?}",
    );
}

#[test]
fn event_completion_marks_detail_as_irules_event() {
    let src = "when CL\n";
    let analysis = analyse(src);
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        items
            .iter()
            .all(|i| i.detail.as_deref() == Some("F5 iRules event")),
        "every event completion should be labelled as an iRules event; got {items:?}",
    );
}

#[test]
fn event_completion_does_not_fire_in_plain_tcl_dialect() {
    // The default registry doesn't carry the `when` spec —
    // completion should fall through to plain command + proc
    // completion (which won't surface `HTTP_REQUEST`).
    let src = "when HT\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"HTTP_REQUEST"),
        "event completion should not fire outside iRules; got {labels:?}",
    );
}

#[test]
fn event_completion_skipped_at_word_index_other_than_1() {
    // `when HTTP_REQUEST f` — word-index 2 (after the event
    // name).  Event completion must not fire.
    let src = "when HTTP_REQUEST f\n";
    let analysis = analyse(src);
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        19,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.iter().any(|l| l.contains("HTTP_REQUEST")),
        "event completion should not fire at word-index 2; got {labels:?}",
    );
}

// -- subcommand argument-value completion ----

#[test]
fn string_is_completes_character_classes() {
    // `string is a` — cursor at the class arg (word 2);
    // expect the `a*` classes (alnum, alpha, ascii).
    let src = "string is a\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        11,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alnum"), "{labels:?}");
    assert!(labels.contains(&"alpha"), "{labels:?}");
    assert!(labels.contains(&"ascii"), "{labels:?}");
    // Every result is an enum value, not a proc / command.
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
    // Non-`a` classes filtered out.
    assert!(!labels.contains(&"digit"), "{labels:?}");
}

#[test]
fn string_is_lists_all_classes_with_empty_partial() {
    // `string is ` — cursor just past the space, empty
    // partial → all 22 character classes.
    let src = "string is \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"boolean"), "{labels:?}");
    assert!(labels.contains(&"wordchar"), "{labels:?}");
    assert!(
        labels.len() >= 20,
        "expected the full class set; got {labels:?}"
    );
    // Sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
}

#[test]
fn string_is_class_detail_is_surfaced() {
    let src = "string is alnum\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let alnum = items.iter().find(|i| i.label == "alnum").expect("alnum");
    assert!(
        alnum
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("alphabet or digit"),
        "{alnum:?}",
    );
}

#[test]
fn subcommand_without_arg_values_falls_through() {
    // `string length x` — `length` has no arg_values, so
    // word-index 2 falls through to plain completion (no
    // enum-value items).
    let src = "string length x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        items.iter().all(|i| i.kind != CompletionKind::EnumValue),
        "no enum-value completions expected; got {items:?}",
    );
}

#[test]
fn option_enum_value_completion_offers_members() {
    // `button .b -relief ra` — cursor on the value word after a closed-set
    // option offers the relief members, filtered by the partial.
    let src = "button .b -relief ra";
    let cur = analyse(src);
    let registry = CommandRegistry::build_default();
    let col = u32::try_from(src.len()).unwrap();
    let items = completions(
        src,
        0,
        col,
        &cur,
        Some(&registry),
        None,
        crate::profile_for_dialect("tk"),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"raised"),
        "expected `raised` among relief completions; got {labels:?}"
    );
}

#[test]
fn prefix_subcommand_list_pinned() {
    let src = "string to\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["tolower", "totitle", "toupper"]);
    assert!(items.iter().all(|i| i.filter_text.is_none()));
}

#[test]
fn fuzzy_subcommand_offers_length_for_lenght() {
    let src = "string lenght\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        13,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["length"], "typo'd subcommand fragment");
    assert_eq!(items[0].filter_text.as_deref(), Some("lenght"));
}

#[test]
fn fuzzy_switch_offers_nocase_for_ncoase() {
    let src = "lsort -ncoase\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        13,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let nocase = items
        .iter()
        .find(|i| i.label == "-nocase")
        .unwrap_or_else(|| panic!("expected fuzzy -nocase, got {items:?}"));
    // The filter text carries the dash, matching the replaced range.
    assert_eq!(nocase.filter_text.as_deref(), Some("-ncoase"));
    let edit = nocase.text_edit.as_ref().expect("switch replace edit");
    assert_eq!(edit.new_text, "-nocase");
}

// Dialect-profile availability (dialect-profile-model.md): the §5.1
// available_subcommands gap and the §5.2 option semantics in
// completion.

#[test]
fn subcommand_completion_is_version_gated_by_the_profile() {
    let src = "dict \n";
    let analysis = analyse_for(src, "tcl8.6");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    // 8.6 buffer: 9.0-only subcommands are not offered.
    let items = completions(
        src,
        0,
        5,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"get"), "{labels:?}");
    assert!(
        !labels.contains(&"getwithdefault"),
        "dict getwithdefault is 9.0+ and must not be offered under tcl8.6: {labels:?}"
    );
    // 9.0 buffer: it is offered.
    let registry90 = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let analysis90 = analyse_for(src, "tcl9.0");
    let items90 = completions(
        src,
        0,
        5,
        &analysis90,
        Some(registry90),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels90: Vec<&str> = items90.iter().map(|i| i.label.as_str()).collect();
    assert!(labels90.contains(&"getwithdefault"), "{labels90:?}");
}

#[test]
fn switch_option_completion_respects_profile_gating() {
    // Option completion under f5-iapps follows the measured fork:
    // F5 reclassification (measurements §4/§4a,
    // `docs/design/f5/bigip-irule-parser-measurements.md`) — the iApps
    // host is the 8.4.6 `f5-tcl` fork, not the falsified 8.5.13
    // hypothesis, so the 8.5+ `switch -nocase` is not offered
    // there, and the 9.0-only `regsub
    // -command` stays out. The 8.4-real `-exact` is offered, so
    // the composed vendor mask keeps working.
    let src = "switch -\n";
    let analysis = analyse_for(src, "f5-iapps");
    let registry = tcl_registry::model::ingress::static_context_for("f5-iapps").commands();
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"-exact"),
        "switch -exact is 8.4 core, offered under f5-iapps: {labels:?}"
    );
    assert!(
        !labels.contains(&"-nocase"),
        "switch -nocase is 8.5+ and the iApps host is a measured 8.4.6 fork: {labels:?}"
    );

    let src9 = "regsub -\n";
    let analysis9 = analyse_for(src9, "f5-iapps");
    let items9 = completions(
        src9,
        0,
        8,
        &analysis9,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    let labels9: Vec<&str> = items9.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels9.contains(&"-command"),
        "regsub -command is 9.0-only, hidden under f5-iapps: {labels9:?}"
    );
}
