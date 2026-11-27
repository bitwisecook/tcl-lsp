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

fn model(source: &str) -> TkUiModel {
    let dialect = crate::profile_for_dialect("tk");
    analyse_tk_ui(
        source,
        dialect,
        crate::registry_for_dialect_profile(dialect),
    )
}

fn model_in_dialect(source: &str, name: &str) -> TkUiModel {
    let dialect = crate::profile_for_dialect(name);
    analyse_tk_ui(
        source,
        dialect,
        crate::registry_for_dialect_profile(dialect),
    )
}

#[test]
fn original_tk_literals_decode_escapes_and_retain_complete_word_spans() {
    let source = r#"label \x2ew -te\x78t "snow \u2603 \$cash \[literal\]"
pack .w -\x69n \x2e"#;
    let ui = model(source);
    let widget = &ui.root.as_ref().unwrap().children[0];
    assert_eq!(widget.path, ".w");
    let text = &widget.options["-text"];
    assert_eq!(text.value, "snow ☃ $cash [literal]");
    assert_eq!(
        &source[text.source.start as usize..text.source.end as usize],
        r#""snow \u2603 \$cash \[literal\]""#,
    );
    assert_eq!(
        &source[widget.source.path.start as usize..widget.source.path.end as usize],
        r"\x2ew",
    );
    let geometry = widget.geometry.as_ref().unwrap();
    assert_eq!(geometry.container.as_deref(), Some("."));
    assert_eq!(geometry.options["-in"].value, ".");
}

#[test]
fn original_tk_literals_keep_braced_data_and_the_actual_full_configuration() {
    let source = r"label .w -text {literal $name [data] \u2603}";
    let ui = model(source);
    assert_eq!(
        ui.root.as_ref().unwrap().children[0].options["-text"].value,
        r"literal $name [data] \u2603",
    );
    let source = concat!("label .w -text {first\\", "\n", "  second}");
    let profile = crate::profile_for_dialect("tk");
    for (style, expected) in [
        (tcl_dialect::BraceBackslashNewline::Folds, "first second"),
        (
            tcl_dialect::BraceBackslashNewline::Literal,
            "first\\\n  second",
        ),
    ] {
        let config = LexerConfig {
            brace_backslash_newline: style,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::context_for_profile(profile),
            config,
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let original = TkSourceCommand::capture(source, &analysis, &command);
        assert_eq!(literal_word(&original, 3).unwrap().0, expected);
    }
}

#[test]
fn original_tk_literals_refuse_substitution_expansion_and_foreign_source() {
    let profile = crate::profile_for_dialect("tk");
    let registry = crate::registry_for_dialect_profile(profile);
    for source in [
        "label .w -text $name",
        "label .w -text [value]",
        "label .w -text prefix$name",
        "label .w -text {*}{value}",
        r"label .w -text \uD800",
    ] {
        let analysis = tk_source_analysis(source, profile, registry);
        let command = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            analysis.body_lexer_config.unwrap(),
        )
        .remove(0);
        let original = TkSourceCommand::capture(source, &analysis, &command);
        assert!(literal_word(&original, 3).is_none(), "{source}");
    }
    let source = "label .w -text kept";
    let analysis = tk_source_analysis(source, profile, registry);
    let command = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        analysis.body_lexer_config.unwrap(),
    )
    .remove(0);
    let changed = TkSourceCommand::capture("label .w -text lost", &analysis, &command);
    assert!(literal_word(&changed, 3).is_none());
    let mut incomplete = command.clone();
    incomplete.is_partial = true;
    let incomplete = TkSourceCommand::capture(source, &analysis, &incomplete);
    assert!(literal_word(&incomplete, 3).is_none());
    let mut unavailable = analysis;
    unavailable.resolved_input = None;
    let unavailable = TkSourceCommand::capture(source, &unavailable, &command);
    assert!(literal_word(&unavailable, 3).is_none());
}

#[test]
fn supplied_tk_preview_keeps_actual_availability_and_refuses_withheld_source() {
    // naming.core.original-tk-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-tk-source-context.md
    let source = "package require Tk\nttk::toggleswitch .new\nframe .kept";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let newer = crate::registry_for_dialect_profile(crate::profile_for_dialect("tcl9.1"));
    let analyse = |version: &str| {
        let generation =
            tcl_registry::model::context_for_profile(crate::profile_for_dialect(version));
        let context =
            std::sync::Arc::new(generation.with_command_store(newer.snapshot().shared_registry()));
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            LexerConfig {
                braced_var: tcl_dialect::BracedVarStyle::FirstClose,
                ..LexerConfig::for_profile(Some(profile))
            },
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    let current = analyse("tcl9.1");
    let paths = |analysis: &AnalysisResult| {
        analyse_tk_ui_with_analysis(source, analysis)
            .root
            .unwrap()
            .children
            .into_iter()
            .map(|widget| widget.path)
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(&current), vec![".kept", ".new"]);
    let older = analyse("tcl8.6");
    assert_eq!(
        current
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        older.resolved_registry().unwrap().snapshot().semantic_key()
    );
    assert_eq!(paths(&older), vec![".kept"]);

    let mut missing = current.clone();
    missing.resolved_input = None;
    let mut foreign = current.clone();
    foreign.resolved_input = older.resolved_input.clone();
    let mut grammar = current.clone();
    grammar.body_lexer_config = Some(LexerConfig::for_profile(Some(profile)));
    let mut unavailable = current.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl".to_owned(),
        overlay: u64::MAX - 361,
    });
    for refused in [&missing, &foreign, &grammar, &unavailable] {
        let model = analyse_tk_ui_with_analysis(source, refused);
        assert_eq!(model.widget_count, 0);
        assert!(model.root.is_none());
        assert_eq!(
            model.uncertainties[0].kind,
            TkUncertaintyKind::SourceContextUnavailable
        );
    }
    assert_eq!(
        analyse_tk_ui_with_analysis("frame .changed", &current).widget_count,
        0
    );
}

#[test]
fn original_tk_source_context_keeps_explicit_store_version_and_operand_boundaries() {
    // naming.core.original-tk-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-tk-source-context.md
    let supplied = crate::profile_for_dialect("tcl8.6");
    let newer = crate::registry_for_dialect_profile(crate::profile_for_dialect("tcl9.1"));
    let source = "package require Tk\nttk::toggleswitch .unavailable\nframe .kept";
    let retained = tk_source_analysis(source, supplied, newer);
    let input = retained.resolved_input.as_ref().unwrap();
    assert_eq!(input.analyser_profile().cache_key(), supplied.cache_key());
    assert_eq!(
        input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        newer.snapshot().semantic_key()
    );
    let ui = analyse_tk_ui(source, supplied, newer);
    assert!(ui.tk_active);
    assert_eq!(ui.widget_count, 2);
    assert_eq!(ui.root.unwrap().children[0].path, ".kept");
    let source = "package require Tk\ninterp alias {} install {} frame .captured\ninstall .wrong\ninterp alias {} normal {} frame\nnormal .written";
    let ui = model_in_dialect(source, "tcl8.6");
    let paths = ui
        .root
        .unwrap()
        .children
        .into_iter()
        .map(|widget| widget.path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec![".written"]);
    let source = "package require Tk\nproc if args {}\nif 1 {frame .inert}\nframe .live";
    let ui = model_in_dialect(source, "tcl8.6");
    let paths = ui
        .root
        .unwrap()
        .children
        .into_iter()
        .map(|widget| widget.path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec![".live"]);
}

#[test]
fn plain_tcl_requires_a_resolved_literal_tk_package_request() {
    let inactive = model_in_dialect("frame .looks_like_a_widget", "tcl8.6");
    assert!(!inactive.tk_active);
    assert!(inactive.root.is_none());
    assert_eq!(inactive.widget_count, 0);

    let active = model_in_dialect("package require Tk\nframe .real_widget", "tcl8.6");
    assert!(active.tk_active);
    assert_eq!(active.widget_count, 2);

    let exact = model_in_dialect("package require -exact Tk\nframe .exact_widget", "tcl8.6");
    assert!(exact.tk_active);
    assert_eq!(exact.widget_count, 2);

    let dynamic = model_in_dialect("package require $package\nframe .not_proven", "tcl8.6");
    assert!(!dynamic.tk_active);

    let impossible = model_in_dialect("package require Tk 99\nframe .not_proven", "tcl8.6");
    assert!(!impossible.tk_active);

    let compatible = model_in_dialect("package require Tk 8.5\nframe .proven", "tcl8.6");
    assert!(compatible.tk_active);

    let impossible_exact =
        model_in_dialect("package require -exact Tk 8.5\nframe .not_proven", "tcl8.6");
    assert!(!impossible_exact.tk_active);
}

#[test]
fn finds_multiline_semicolon_and_quoted_literal_widgets_with_sources() {
    let source = "# frame .commented\nframe .top -background {navy blue}; button \".top.ok\" -text \"Save\"\npack .top.ok\n";
    let ui = model(source);
    assert_eq!(ui.schema_version, TK_UI_SCHEMA_VERSION);
    assert_eq!(ui.widget_count, 3);
    let root = ui.root.as_ref().expect("Tk dialect is active");
    assert_eq!(root.children[0].path, ".top");
    let button = &root.children[0].children[0];
    assert_eq!(button.path, ".top.ok");
    assert_eq!(button.options["-text"].value, "Save");
    assert_eq!(
        button.geometry.as_ref().map(|g| g.manager.as_str()),
        Some("pack")
    );
    assert!(button.source.command.end > button.source.command.start);
}

#[test]
fn dynamic_paths_and_manager_forms_abstain() {
    // An unset `$path` read ends the script (tclsh: `can't read "path"`).
    let ui = model("set path [gets stdin]\nframe $path\npack $path\npack configure .missing\n");
    assert_eq!(ui.widget_count, 1);
    assert!(
        ui.uncertainties
            .iter()
            .any(|u| u.kind == TkUncertaintyKind::DynamicWidgetPath)
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|u| u.kind == TkUncertaintyKind::DynamicGeometryTarget)
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|u| u.kind == TkUncertaintyKind::UnknownGeometryTarget)
    );
}

#[test]
fn visits_registry_declared_nested_bodies() {
    let ui = model("proc make {} {\n frame .inside\n pack .inside\n}\n");
    assert_eq!(ui.widget_count, 2);
    let root = ui.root.as_ref().expect("Tk dialect is active");
    assert_eq!(root.children[0].path, ".inside");
    assert_eq!(root.children[0].certainty, TkFactCertainty::Potential);
    assert_eq!(
        root.children[0]
            .geometry
            .as_ref()
            .map(|g| g.manager.as_str()),
        Some("pack")
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|uncertainty| uncertainty.kind == TkUncertaintyKind::PotentialExecution)
    );
}

#[test]
fn potential_bodies_do_not_activate_tk_or_create_definite_conflicts() {
    let inactive = model_in_dialect(
        "proc maybe {} { package require Tk; frame .inside }",
        "tcl8.6",
    );
    assert!(!inactive.tk_active);

    let alternatives =
        model("frame .holder\nif {$mode} {pack .a -in .holder} else {grid .b -in .holder}");
    assert_eq!(
        alternatives.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );
}

#[test]
fn follows_imported_and_qualified_heads_but_not_rebound_ones() {
    let ui = model(
        "namespace import ::ttk::button\nbutton .imported\n::frame .qualified\nproc frame args {}\nframe .rebound\n",
    );
    let paths: Vec<_> = ui
        .root
        .as_ref()
        .expect("Tk dialect is active")
        .children
        .iter()
        .map(|widget| widget.path.as_str())
        .collect();
    assert_eq!(paths, vec![".imported", ".qualified"]);
}

#[test]
fn all_registry_geometry_managers_are_consumed_without_a_command_list() {
    let dialect = crate::profile_for_dialect("tk");
    let registry = crate::registry_for_dialect_profile(dialect);
    let managers = registry.commands_with_trait(Traits::TK_GEOMETRY_MANAGER);
    assert_ne!(managers, [] as [&str; 0]);
    for manager in managers {
        let source = format!("frame .w; {manager} .w");
        let ui = analyse_tk_ui(&source, dialect, registry);
        let widget = &ui.root.as_ref().expect("Tk dialect is active").children[0];
        assert_eq!(
            widget.geometry.as_ref().map(|item| item.manager.as_str()),
            Some(manager),
            "registry-declared manager {manager} was not consumed"
        );
    }
}

#[test]
fn one_direct_manager_call_places_every_literal_leading_target() {
    let ui = model("frame .a\nframe .b\npack .a .b -side left\n");
    let root = ui.root.as_ref().expect("Tk dialect is active");
    assert_eq!(root.children.len(), 2);
    for widget in &root.children {
        let placement = widget.geometry.as_ref().expect("direct pack placement");
        assert_eq!(placement.manager, "pack");
        assert_eq!(placement.options["-side"].value, "left");
    }
}

#[test]
fn registry_option_abbreviations_keep_literal_options() {
    let ui = model("button .save -und 0");
    assert_eq!(
        ui.root.as_ref().expect("Tk dialect is active").children[0].options["-underline"].value,
        "0"
    );
}

#[test]
fn preview_filters_constructors_by_the_resolved_tk_floor() {
    let old = model_in_dialect(
        "package require Tk\nttk::toggleswitch .switch\npack .switch",
        "tcl8.6",
    );
    assert!(old.tk_active);
    assert_eq!(old.widget_count, 1);
    assert_eq!(
        old.root.as_ref().expect("Tk is active").children,
        [] as [crate::tk_preview::TkWidget; 0]
    );
    assert_eq!(
        old.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );

    let current = model_in_dialect("package require Tk\nttk::toggleswitch .switch", "tcl9.1");
    assert_eq!(current.widget_count, 2);
    assert_eq!(
        current.root.as_ref().expect("Tk is active").children[0].path,
        ".switch"
    );
}

#[test]
fn preview_filters_options_before_resolving_abbreviations() {
    let old = model_in_dialect(
        "package require Tk\nlabel .label -text old -texta 45",
        "tcl8.6",
    );
    let old_options = &old.root.as_ref().expect("Tk is active").children[0].options;
    assert_eq!(old_options["-text"].value, "old");
    assert!(!old_options.contains_key("-textangle"));

    let current = model_in_dialect(
        "package require Tk\nlabel .label -text old -texta 45",
        "tcl9.1",
    );
    let current_options = &current.root.as_ref().expect("Tk is active").children[0].options;
    assert_eq!(current_options["-text"].value, "old");
    assert_eq!(current_options["-textangle"].value, "45");
}

#[test]
fn projects_source_parent_for_root_regular_and_orphan_widgets() {
    let ui = model(
        "frame .\nframe .main\nbutton .main.child\npack .main.child\nframe .lost.child\nframe .lost.child.grandchild\n",
    );
    let root = ui.root.as_ref().expect("Tk is active");
    assert_eq!(
        root.path, ".",
        "the implicit root remains the only root node"
    );
    assert_eq!(
        root.children[0].path, ".main",
        "a constructor cannot replace the root"
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| { entry.kind == TkUncertaintyKind::DynamicWidgetPath })
    );
    assert_eq!(
        root.source_parent_path, None,
        "the root has no pathname parent"
    );
    assert_eq!(root.children[0].source_parent_path.as_deref(), Some("."));
    assert_eq!(
        root.children[0].children[0].source_parent_path.as_deref(),
        Some(".main")
    );
    assert_eq!(
        root.children[0].children[0]
            .geometry
            .as_ref()
            .and_then(|placement| placement.container.as_deref()),
        Some(".main"),
        "the default geometry container comes from the same owner projection",
    );
    assert_eq!(
        ui.orphan_widgets[0].source_parent_path.as_deref(),
        Some(".lost")
    );
    assert_eq!(
        ui.orphan_widgets[0].children[0]
            .source_parent_path
            .as_deref(),
        Some(".lost.child"),
    );
    let payload = serde_json::to_value(&ui).expect("model serializes");
    assert!(payload["root"]["source_parent_path"].is_null());
    assert_eq!(payload["root"]["children"][0]["source_parent_path"], ".");
    assert_eq!(payload["orphan_widgets"][0]["source_parent_path"], ".lost");
}

#[test]
fn tk_widget_deserializes_legacy_payload_without_source_parent_path() {
    let ui = model("frame .main\n");
    let mut legacy =
        serde_json::to_value(&ui.root.as_ref().unwrap().children[0]).expect("model serializes");
    legacy
        .as_object_mut()
        .expect("widget serializes as an object")
        .remove("source_parent_path");
    let restored: TkWidget = serde_json::from_value(legacy).expect("legacy widget deserializes");
    assert_eq!(restored.source_parent_path, None);
}

#[test]
fn reports_mixed_geometry_managers_and_unknown_parent() {
    let ui = model(
        "frame .a\nframe .a.one\nframe .a.two\npack .a.one\ngrid .a.two\nframe .lost.child\n",
    );
    assert_eq!(ui.geometry_conflicts.len(), 1);
    assert_eq!(ui.geometry_conflicts[0].container, ".a");
    assert!(
        ui.uncertainties
            .iter()
            .any(|u| u.kind == TkUncertaintyKind::UnknownWidgetParent)
    );
    assert_eq!(ui.orphan_widgets[0].path, ".lost.child");
}

#[test]
fn nested_missing_parents_are_rendered_once_as_one_orphan_subtree() {
    let ui = model("frame .lost.child\nframe .lost.child.grandchild\n");
    assert_eq!(ui.orphan_widgets.len(), 1);
    assert_eq!(ui.orphan_widgets[0].path, ".lost.child");
    assert_eq!(
        ui.orphan_widgets[0].children[0].path,
        ".lost.child.grandchild"
    );
}

#[test]
fn one_widget_can_switch_managers_without_a_conflict() {
    let ui = model("frame .a\npack .a\ngrid .a\n");
    let root = ui.root.as_ref().expect("Tk dialect is active");
    assert_eq!(
        root.children[0]
            .geometry
            .as_ref()
            .map(|placement| placement.manager.as_str()),
        Some("grid")
    );
    assert_eq!(
        ui.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );
}

#[test]
fn geometry_release_and_query_forms_do_not_place_widgets() {
    let released = model(
        "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\npack forget .holder.a\ngrid .holder.b\n",
    );
    assert!(released.geometry_conflicts.is_empty(), "{released:#?}");

    let query = model(
        "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\ngrid info .holder.b\n",
    );
    assert!(query.geometry_conflicts.is_empty(), "{query:#?}");
    assert!(
        !query
            .uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::UnsupportedGeometryForm)
    );

    let conflict = model(
        "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a .holder.b\ngrid .holder.a\n",
    );
    assert_eq!(conflict.geometry_conflicts.len(), 1, "{conflict:#?}");
}

#[test]
fn geometry_conflicts_use_registry_policy_and_effective_in_container() {
    let place = model("frame .a\nframe .b\nplace .a\ngrid .b\n");
    assert_eq!(
        place.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );

    let separate = model(
        "frame .left\nframe .right\nframe .a\nframe .b\npack .a -in .left\ngrid .b -in .right\n",
    );
    assert_eq!(
        separate.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );
    let a = separate
        .root
        .as_ref()
        .unwrap()
        .children
        .iter()
        .find(|widget| widget.path == ".a")
        .unwrap();
    assert_eq!(
        a.geometry.as_ref().unwrap().container.as_deref(),
        Some(".left")
    );

    let same =
        model("frame .holder\nframe .a\nframe .b\npack .a -in .holder\ngrid .b -in .holder\n");
    assert_eq!(same.geometry_conflicts.len(), 1);
    assert_eq!(same.geometry_conflicts[0].container, ".holder");

    let missing = model("frame .a\npack .a -in .not-created\n");
    assert!(
        missing
            .uncertainties
            .iter()
            .any(|entry| { entry.kind == TkUncertaintyKind::UnknownGeometryContainer })
    );

    let later_destroy = model(
        "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\ngrid .holder.b\ndestroy .holder\n",
    );
    assert_eq!(
        later_destroy.geometry_conflicts.len(),
        1,
        "later lifecycle changes must not erase an already-triggered Tk error: {later_destroy:#?}"
    );
}

#[test]
fn accepts_real_tk_path_components_and_marks_lifecycle_mutations_uncertain() {
    let ui = model("ttk::frame .main-pane\n.main-pane configure -padding 4\ndestroy .main-pane\n");
    let widget = &ui.root.as_ref().unwrap().children[0];
    assert_eq!(widget.path, ".main-pane");
    assert_eq!(widget.certainty, TkFactCertainty::Potential);
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation })
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| { entry.kind == TkUncertaintyKind::WidgetLifecycleMutation })
    );
}

#[test]
fn deferred_mutation_before_constructor_downgrades_the_later_widget() {
    let ui = model("proc later {} {.w configure -text changed}\ncheckbutton .w -text initial\n");
    let widget = &ui.root.as_ref().unwrap().children[0];
    assert_eq!(widget.path, ".w");
    assert_eq!(widget.certainty, TkFactCertainty::Potential);
    assert!(ui.uncertainties.iter().any(|entry| {
        entry.kind == TkUncertaintyKind::PostConstructorMutation
            && entry.message.contains("configure")
    }));

    let destroyed = model("proc later {} {destroy .w}\ncheckbutton .w\n");
    let widget = &destroyed.root.as_ref().unwrap().children[0];
    assert_eq!(widget.certainty, TkFactCertainty::Potential);
    assert!(
        destroyed
            .uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
    );
}

#[test]
fn command_table_lifecycle_uses_only_rename_old_name() {
    for source in ["frame .w\nproc .w {} {}\n", "frame .w\nrename other .w\n"] {
        let ui = model(source);
        assert_eq!(
            ui.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Certain,
            "an unrelated command-table operand must not mutate the widget: {source}"
        );
        assert!(
            !ui.uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation),
            "an unrelated command-table operand must not add lifecycle uncertainty: {ui:#?}"
        );
    }

    let renamed = model("frame .w\nrename .w other\n");
    assert_eq!(
        renamed.root.as_ref().unwrap().children[0].certainty,
        TkFactCertainty::Potential
    );
    assert!(
        renamed
            .uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
    );
}

#[test]
fn argument_sensitive_widget_queries_do_not_look_like_mutations() {
    let cases = [
        ("button", "configure", "configure -text Save"),
        ("entry", "configure -width", "configure -width 20"),
        ("ttk::button", "state", "state disabled"),
        ("ttk::combobox", "current", "current 2"),
        ("ttk::notebook", "select", "select .page"),
        ("ttk::treeview", "focus", "focus item"),
        ("ttk::treeview", "selection", "selection set item"),
        ("ttk::toggleswitch", "switchstate", "switchstate 1"),
        ("entry", "selection present", "selection clear"),
        ("spinbox", "selection element", "selection element buttonup"),
        ("listbox", "selection includes 0", "selection set 0"),
        ("canvas", "select item", "select clear"),
        ("panedwindow", "proxy coord", "proxy forget"),
        ("text", "edit canundo", "edit undo"),
        ("text", "tag ranges hot", "tag raise hot"),
        ("text", "peer names", "peer create .peer"),
        ("ttk::treeview", "tag has hot item", "tag add hot item"),
    ];
    for (constructor, query, mutation) in cases {
        let query_ui = model(&format!("{constructor} .w\n.w {query}\n"));
        let query_widget = &query_ui.root.as_ref().unwrap().children[0];
        assert_eq!(
            query_widget.certainty,
            TkFactCertainty::Certain,
            "query must retain constructor certainty: {constructor} {query}"
        );
        assert!(
            !query_ui
                .uncertainties
                .iter()
                .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation }),
            "query must not report mutation: {constructor} {query}"
        );

        let mutation_ui = model(&format!("{constructor} .w\n.w {mutation}\n"));
        let mutation_widget = &mutation_ui.root.as_ref().unwrap().children[0];
        assert_eq!(
            mutation_widget.certainty,
            TkFactCertainty::Potential,
            "setter must downgrade constructor certainty: {constructor} {mutation}"
        );
        assert!(
            mutation_ui
                .uncertainties
                .iter()
                .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation }),
            "setter must report mutation: {constructor} {mutation}"
        );
    }

    let deferred_query = model("proc later {} {.w configure}\nframe .w\n");
    assert_eq!(
        deferred_query.root.as_ref().unwrap().children[0].certainty,
        TkFactCertainty::Certain,
        "a deferred getter before construction must stay a getter"
    );
    assert!(
        !deferred_query
            .uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::PostConstructorMutation)
    );

    let deferred_operation_query = model("proc later {} {.w selection present}\nentry .w\n");
    assert_eq!(
        deferred_operation_query.root.as_ref().unwrap().children[0].certainty,
        TkFactCertainty::Certain,
        "deferred calls must retain literal operation facts"
    );
}

#[test]
fn computed_unknown_and_ambiguous_widget_operations_keep_parent_mutation_fallback() {
    for source in [
        "entry .w\nset op present\n.w selection $op\n",
        "entry .w\n.w selection unknown\n",
        "text .w\n.w tag ra hot\n",
        "ttk::treeview .w\n.w tag c hot\n",
    ] {
        let ui = model(source);
        assert_eq!(
            ui.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Potential,
            "indeterminate operation must not be guessed as a query: {source}"
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::PostConstructorMutation),
            "parent fallback must remain conservative: {source}"
        );
    }

    let abbreviated = model("entry .w\n.w selection pres\n");
    assert_eq!(
        abbreviated.root.as_ref().unwrap().children[0].certainty,
        TkFactCertainty::Certain,
        "a unique operation prefix resolves to the query form"
    );
}

#[test]
fn recreated_widget_cannot_inherit_an_earlier_instances_geometry() {
    let ui = model("frame .item\npack .item\ndestroy .item\nframe .item\n");
    let widget = &ui.root.as_ref().expect("Tk dialect is active").children[0];
    assert_eq!(widget.path, ".item");
    assert_eq!(widget.certainty, TkFactCertainty::Potential);
    assert!(
        widget.geometry.is_none(),
        "old pack evidence must not attach to the recreated instance: {ui:#?}"
    );
    assert_eq!(
        ui.geometry_conflicts,
        [] as [crate::tk_preview::TkGeometryConflict; 0]
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
    );
}

#[test]
fn destroying_the_root_releases_descendant_geometry_and_lifetimes() {
    let ui = model(
        "frame .holder\nframe .holder.a\npack .holder.a\ndestroy .\nframe .holder\nframe .holder.b\ngrid .holder.b\n",
    );
    assert!(ui.geometry_conflicts.is_empty(), "{ui:#?}");
    let holder = ui
        .root
        .as_ref()
        .unwrap()
        .children
        .iter()
        .find(|widget| widget.path == ".holder")
        .unwrap();
    assert_eq!(holder.certainty, TkFactCertainty::Potential);
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
    );
}

#[test]
fn absent_or_lifecycle_ambiguous_containers_cannot_form_definite_conflicts() {
    let absent = model("frame .a\nframe .b\npack .a -in .missing\ngrid .b -in .missing\n");
    assert!(absent.geometry_conflicts.is_empty(), "{absent:#?}");
    assert!(
        absent
            .uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::UnknownGeometryContainer)
    );

    let recreated = model(
        "frame .holder\nframe .a\nframe .b\ndestroy .holder\npack .a -in .holder\ngrid .b -in .holder\n",
    );
    assert!(recreated.geometry_conflicts.is_empty(), "{recreated:#?}");
}

#[test]
fn serializes_versioned_host_metadata_only_when_stamped() {
    let mut ui = model("frame .x");
    let json = serde_json::to_value(&ui).expect("model is serializable");
    assert!(json.get("document_uri").is_none());
    ui.document_uri = Some("file:///tmp/example.tcl".to_owned());
    ui.document_version = Some(7);
    let stamped = serde_json::to_value(ui).expect("model is serializable");
    assert_eq!(stamped["document_version"], 7);
}

#[test]
fn caps_repetitive_uncertainty_detail_and_reports_the_omitted_count() {
    let source = std::iter::once("set dynamic [gets stdin]".to_owned())
        .chain((0..(MAX_TK_UI_UNCERTAINTIES + 17)).map(|index| format!("frame $dynamic.f{index}")))
        .collect::<Vec<_>>()
        .join("\n");
    let ui = model(&source);
    assert_eq!(ui.uncertainties.len(), MAX_TK_UI_UNCERTAINTIES);
    assert_eq!(ui.uncertainties_truncated, 17);

    let json = serde_json::to_value(ui).expect("model is serializable");
    assert_eq!(json["uncertainties_truncated"], 17);
}

#[test]
fn cap_includes_unknown_parent_uncertainties_added_during_hierarchy_building() {
    let source = (0..(MAX_TK_UI_UNCERTAINTIES + 9))
        .map(|index| format!("frame .missing{index}.child"))
        .collect::<Vec<_>>()
        .join("\n");
    let ui = model(&source);
    assert_eq!(ui.uncertainties.len(), MAX_TK_UI_UNCERTAINTIES);
    assert_eq!(ui.uncertainties_truncated, 9);
    assert!(
        ui.uncertainties
            .iter()
            .all(|entry| entry.kind == TkUncertaintyKind::UnknownWidgetParent)
    );
}

#[test]
fn caps_retained_widgets_and_reports_the_complete_static_count() {
    let extra = 19;
    let source = (0..(MAX_TK_UI_WIDGETS + extra))
        .map(|index| format!("frame .widget{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let ui = model(&source);
    assert_eq!(ui.widget_count, MAX_TK_UI_WIDGETS + extra + 1);
    assert_eq!(ui.widgets_truncated, extra);
    assert_eq!(
        ui.root
            .as_ref()
            .expect("Tk dialect is active")
            .children
            .len(),
        MAX_TK_UI_WIDGETS
    );
    assert!(
        ui.uncertainties
            .iter()
            .any(|entry| entry.kind == TkUncertaintyKind::WidgetLimitReached)
    );
}
