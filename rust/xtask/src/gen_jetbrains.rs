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

//! Generate the `JetBrains` `DiagnosticCatalog.kt` from the `DiagCode` catalogue
//! — the native successor to the Kotlin-catalog half of
//! `scripts/codegen/editor_settings.py`.
//!
//! The file is a full-file projection of the user-configurable diagnostics +
//! optimisations, each carrying a short checkbox `label`. `--check` verifies
//! the committed file matches, exiting non-zero on drift.
//!
//! It also gates the plugin's hand-written semantic-token colour map against
//! the legend the server advertises, in both modes: a token type the server
//! emits that the map has no colour for is painted in the default foreground,
//! which in an editor is indistinguishable from no semantic highlighting.

use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_core_types::{DiagCode, DocRow};

use tcl_compiler::optimiser::profiles::{DEFAULT_EDITOR_PROFILE, OptimisationProfile};

use crate::util::{repo_root, write_if_changed};

const CATALOG_PATH: &str = "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/generated/DiagnosticCatalog.kt";

/// The hand-written map this module verifies (rather than generates): the
/// colour for a token type is an editor judgement, but its *key set* is the
/// server's to decide.
const SEMANTIC_TOKENS_PATH: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/TclSemanticTokens.kt";

/// The section grouping `(key, title)` in table order (the three `irules*` keys
/// share the iRules title, collapsing in `sectionTitles`/`sectionOrder`).
const SECTIONS: &[(&str, &str)] = &[
    ("error", "Diagnostics — Errors"),
    ("warning", "Diagnostics — Style & Best Practice"),
    ("variable", "Diagnostics — Variables"),
    ("security", "Diagnostics — Security"),
    ("hint", "Diagnostics — Hints"),
    ("shimmer", "Diagnostics — Shimmer"),
    ("taint", "Diagnostics — Taint"),
    ("irules", "Diagnostics — iRules"),
    ("irules_security", "Diagnostics — iRules"),
    ("irules_variable", "Diagnostics — iRules"),
    ("bigip", "Diagnostics — BIG-IP Configuration"),
    ("sslictcl", "Diagnostics — SslicTcl"),
    ("tclpkg", "Diagnostics — Package Manager"),
];

/// The sort rank of a section key (its index in [`SECTIONS`]).
fn section_rank(key: &str) -> usize {
    SECTIONS.iter().position(|(k, _)| *k == key).unwrap_or(999)
}

/// A short checkbox label `"{code}: {desc}"` — `desc` is the description up to
/// the first em/en dash, backtick-stripped, `.`-trimmed, and truncated to
/// 52 chars + `...` past 55 — then Kotlin-string-escaped (`\ " $`).
fn short_label(code: &str, description: &str) -> String {
    let head = description.split('—').next().unwrap_or(description);
    let head = head.split('–').next().unwrap_or(head);
    let head = head.trim().trim_end_matches('.');
    let stripped: String = head.chars().filter(|&c| c != '`').collect();
    let desc = if stripped.chars().count() > 55 {
        let t: String = stripped.chars().take(52).collect();
        format!("{t}...")
    } else {
        stripped
    };
    let label = format!("{code}: {desc}");
    let mut out = String::new();
    for c in label.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            c => out.push(c),
        }
    }
    out
}

/// A user-configurable diagnostic row: `(code, section, default_on, description)`.
type DiagRow = (&'static str, &'static str, bool, &'static str);

/// User-configurable diagnostics `(code, section, default_on, description)`,
/// grouped by section then code.
fn sorted_diags() -> Vec<DiagRow> {
    let mut diags: Vec<_> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Diagnostic {
                section,
                default_on,
                internal: false,
                reserved: false,
                description,
                tag: _,
            } => Some((c.as_str(), section.as_str(), default_on, description)),
            DocRow::Diagnostic { .. } | DocRow::Optimisation { .. } => None,
        })
        .collect();
    diags.sort_by_key(|(code, section, _, _)| (section_rank(section), *code));
    diags
}

/// Optimisation codes `(code, description)`, by code (always default-on).
fn sorted_opts() -> Vec<(&'static str, &'static str)> {
    let mut opts: Vec<_> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation { description, .. } => Some((c.as_str(), description)),
            DocRow::Diagnostic { .. } => None,
        })
        .collect();
    opts.sort_by_key(|(code, _)| *code);
    opts
}

/// Diagnostics grouped by their deduplicated section title, in title order
/// (only titles that have diagnostics).
fn diag_section_groups() -> Vec<(&'static str, Vec<DiagRow>)> {
    let diags = sorted_diags();
    let mut ordered_titles: Vec<&str> = Vec::new();
    for (_, title) in SECTIONS {
        if !ordered_titles.contains(title) {
            ordered_titles.push(title);
        }
    }
    ordered_titles
        .into_iter()
        .filter_map(|title| {
            let group: Vec<_> = SECTIONS
                .iter()
                .filter(|(_, t)| t == &title)
                .flat_map(|(k, _)| diags.iter().filter(move |(_, s, _, _)| s == k).copied())
                .collect();
            (!group.is_empty()).then_some((title, group))
        })
        .collect()
}

/// Replace the text between `// @generated:{marker}:begin` and `:end`,
/// preserving the begin-line indentation.
fn replace_block(content: &str, marker: &str, replacement: &str) -> String {
    let begin_tag = format!("// @generated:{marker}:begin");
    let end_tag = format!("// @generated:{marker}:end");
    let (Some(begin_idx), Some(end_idx)) = (content.find(&begin_tag), content.find(&end_tag))
    else {
        return content.to_owned();
    };
    let indent_start = content[..begin_idx].rfind('\n').map_or(0, |i| i + 1);
    let indent = &content[indent_start..begin_idx];
    let before = &content[..begin_idx + begin_tag.len()];
    let after = &content[end_idx..];
    format!("{before}\n{replacement}{indent}{after}")
}

/// Render the full `DiagnosticCatalog.kt`.
fn catalog() -> String {
    let diags = sorted_diags();
    let opts = sorted_opts();

    let mut out = crate::util::license_banner("//");
    out.push('\n');
    out.push_str(
        "// GENERATED by `cargo xtask gen-jetbrains-catalog` — do not edit.\n\
         // Source of truth: `tcl-core-types` `DiagCode` catalogue.\n\
         package com.tcllsp.jetbrains.settings.generated\n\
         \n\
         data class DiagnosticDef(\n    val code: String,\n    val section: String,\n    \
         val label: String,\n    val defaultEnabled: Boolean,\n)\n\
         \n\
         data class OptimisationDef(\n    val code: String,\n    val label: String,\n    \
         val defaultEnabled: Boolean,\n)\n\
         \n\
         object DiagnosticCatalog {\n    val diagnostics: List<DiagnosticDef> = listOf(\n",
    );
    for (code, section, default_on, description) in &diags {
        let _ = writeln!(
            out,
            "        DiagnosticDef(\"{code}\", \"{section}\", \"{}\", {default_on}),",
            short_label(code, description),
        );
    }
    out.push_str("    )\n\n    val optimisations: List<OptimisationDef> = listOf(\n");
    for (code, description) in &opts {
        let _ = writeln!(
            out,
            "        OptimisationDef(\"{code}\", \"{}\", true),",
            short_label(code, description),
        );
    }
    out.push_str("    )\n\n    val sectionTitles: Map<String, String> = mapOf(\n");
    // Unique titles in declaration order.
    let mut seen: Vec<&str> = Vec::new();
    let mut ordered_keys: Vec<&str> = Vec::new();
    for (key, title) in SECTIONS {
        if !seen.contains(title) {
            seen.push(title);
            ordered_keys.push(key);
            let _ = writeln!(out, "        \"{key}\" to \"{title}\",");
        }
    }
    out.push_str("    )\n\n    val sectionOrder: List<String> = listOf(\n");
    for key in &ordered_keys {
        let _ = writeln!(out, "        \"{key}\",");
    }
    out.push_str("    )\n}\n");
    out
}

const SETTINGS_PATH: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt";
const PANEL_PATH: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettingsPanel.kt";

/// Regenerate the `@generated` blocks of `TclLspSettings.kt` (persisted var
/// declarations + the code→var maps).
fn settings(current: &str) -> String {
    let diags = sorted_diags();
    let opts = sorted_opts();
    let mut out = current.to_owned();

    let mut vars = String::new();
    for (code, _, default_on, _) in &diags {
        let _ = writeln!(vars, "    var diagnostic{code}: Boolean = {default_on}");
    }
    out = replace_block(&out, "diagnostic-vars", &vars);

    let mut map = String::new();
    for (code, _, _, _) in &diags {
        let _ = writeln!(map, "                \"{code}\" to diagnostic{code},");
    }
    out = replace_block(&out, "diagnostic-map", &map);

    let mut opt_vars = String::from("    var optimiserEnabled: Boolean = true\n");
    let _ = writeln!(
        opt_vars,
        "    var optimiserProfile: String = \"{}\"",
        DEFAULT_EDITOR_PROFILE.name()
    );
    // Each per-code override is tri-state, exactly as the VS Code setting is
    // (`["boolean", "null"]`, default `null`). A non-null value here *beats*
    // the profile server-side — `true` lifts the code out of the profile's
    // disabled set — so a hard default would silently switch on every
    // optimisation the chosen profile deliberately leaves off.
    for (code, _) in &opts {
        let _ = writeln!(opt_vars, "    var optimiser{code}: Boolean? = null");
    }
    out = replace_block(&out, "optimiser-vars", &opt_vars);

    let mut opt_map = String::from("                \"enabled\" to optimiserEnabled,\n");
    opt_map.push_str("                \"profile\" to optimiserProfile,\n");
    for (code, _) in &opts {
        let _ = writeln!(opt_map, "                \"{code}\" to optimiser{code},");
    }
    replace_block(&out, "optimiser-map", &opt_map)
}

/// The note under the optimiser grid. A tri-state checkbox is unusual enough
/// in a settings page that the third state needs saying out loud.
const PROFILE_LEGEND: [&str; 6] = [
    r#"        builder.addWrappedComment("#,
    r#"            "The profile chooses which optimisation families run. A per-code box left " +"#,
    r#"                "in its mixed state inherits from the profile; tick or untick one to " +"#,
    r#"                "force that code on or off regardless of the profile. Reset to profile " +"#,
    r#"                "clears every override and hands the choice back to the profile.","#,
    r#"        )"#,
];

/// Join checkbox field refs six-per-line at `indent` spaces.
fn six_per_line_at(refs: &[String], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out = String::new();
    for chunk in refs.chunks(6) {
        let _ = writeln!(out, "{pad}{},", chunk.join(", "));
    }
    out
}

/// Regenerate the `@generated` blocks of `TclLspSettingsPanel.kt` (checkbox
/// fields + the Swing UI + dirty/apply/reset handlers).
fn panel(current: &str) -> String {
    let diags = sorted_diags();
    let opts = sorted_opts();
    let groups = diag_section_groups();
    let mut out = current.to_owned();

    // diag-checkboxes: grouped with `// title` headers, blank line between.
    let mut cb = String::new();
    for (title, group) in &groups {
        let _ = writeln!(cb, "    // {title}");
        for (code, _, _, description) in group {
            let _ = writeln!(
                cb,
                "    private val diag{code} = JBCheckBox(\"{}\")",
                short_label(code, description)
            );
        }
        cb.push('\n');
    }
    let cb = cb.strip_suffix('\n').unwrap_or(&cb);
    out = replace_block(&out, "diag-checkboxes", cb);

    // diag-ui: titled separator + grid panel + six-per-line refs.
    let mut ui = String::new();
    for (title, group) in &groups {
        let _ = writeln!(
            ui,
            "        builder.addComponent(TitledSeparator(\"{title}\"))"
        );
        ui.push_str(
            "        builder.addComponent(\n            ReflowingGrid(\n                listOf(\n",
        );
        let refs: Vec<String> = group
            .iter()
            .map(|(code, _, _, _)| format!("diag{code}"))
            .collect();
        ui.push_str(&six_per_line_at(&refs, 20));
        ui.push_str("                ),\n            ),\n        )\n");
        ui.push('\n');
    }
    let ui = ui.strip_suffix('\n').unwrap_or(&ui);
    out = replace_block(&out, "diag-ui", ui);

    // diag dirty/apply/reset.
    let mut dirty = String::new();
    let mut apply = String::new();
    let mut reset = String::new();
    for (code, _, _, _) in &diags {
        let _ = writeln!(
            dirty,
            "            diag{code}.isSelected != s.diagnostic{code} ||"
        );
        let _ = writeln!(apply, "        s.diagnostic{code} = diag{code}.isSelected");
        let _ = writeln!(reset, "        diag{code}.isSelected = s.diagnostic{code}");
    }
    out = replace_block(&out, "diag-dirty", &dirty);
    out = replace_block(&out, "diag-apply", &apply);
    out = replace_block(&out, "diag-reset", &reset);

    // opt-checkboxes.
    let mut opt_cb =
        String::from("    private val optEnabled = JBCheckBox(\"Enable optimiser suggestions\")\n");
    let profile_items = OptimisationProfile::ALL
        .iter()
        .map(|p| format!("\"{}\"", p.name()))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        opt_cb,
        "    private val optProfile = JComboBox(arrayOf({profile_items}))"
    );
    // Tri-state, so a code can say "inherit from the profile" — the state the
    // VS Code setting expresses as `null` and its default.
    for (code, description) in &opts {
        let _ = writeln!(
            opt_cb,
            "    private val opt{code} = ThreeStateCheckBox(\"{}\", ThreeStateCheckBox.State.DONT_CARE)",
            short_label(code, description)
        );
    }
    // One named list, so the grid and the "reset to profile" link cannot
    // disagree about which boxes are per-code overrides.
    opt_cb.push_str("    private val optCodeBoxes: List<ThreeStateCheckBox> = listOf(\n");
    let box_refs: Vec<String> = opts.iter().map(|(code, _)| format!("opt{code}")).collect();
    opt_cb.push_str(&six_per_line_at(&box_refs, 8));
    opt_cb.push_str("    )\n");
    out = replace_block(&out, "opt-checkboxes", &opt_cb);

    // opt-ui.
    let mut opt_ui = String::from(
        "        builder.addComponent(TitledSeparator(\"Optimiser\"))\n\
         \x20       builder.addComponent(optEnabled)\n\
         \x20       builder.addLabeledComponent(JBLabel(\"Profile:\"), profileRow())\n\
         \x20       builder.addComponent(ReflowingGrid(optCodeBoxes))\n",
    );
    for line in PROFILE_LEGEND {
        let _ = writeln!(opt_ui, "{line}");
    }
    out = replace_block(&out, "opt-ui", &opt_ui);

    // opt dirty/apply/reset.
    let mut opt_dirty =
        String::from("            optEnabled.isSelected != s.optimiserEnabled ||\n");
    opt_dirty.push_str("            optProfile.selectedItem != s.optimiserProfile ||\n");
    let mut opt_apply = String::from("        s.optimiserEnabled = optEnabled.isSelected\n");
    opt_apply.push_str(
        "        s.optimiserProfile = optProfile.selectedItem as? String ?: s.optimiserProfile\n",
    );
    let mut opt_reset = String::from("        optEnabled.isSelected = s.optimiserEnabled\n");
    opt_reset.push_str("        optProfile.selectedItem = s.optimiserProfile\n");
    for (code, _) in &opts {
        let _ = writeln!(
            opt_dirty,
            "            triState(opt{code}) != s.optimiser{code} ||"
        );
        let _ = writeln!(opt_apply, "        s.optimiser{code} = triState(opt{code})");
        let _ = writeln!(
            opt_reset,
            "        opt{code}.state = threeState(s.optimiser{code})"
        );
    }
    out = replace_block(&out, "opt-dirty", &opt_dirty);
    out = replace_block(&out, "opt-apply", &opt_apply);
    replace_block(&out, "opt-reset", &opt_reset)
}

/// One generated `JetBrains` file: `(relative-path, regenerated-content)`.
fn artifacts() -> Result<Vec<(&'static str, String)>> {
    let read = |rel: &str| -> Result<String> {
        let p = repo_root().join(rel);
        std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))
    };
    Ok(vec![
        (CATALOG_PATH, catalog()),
        (SETTINGS_PATH, settings(&read(SETTINGS_PATH)?)),
        (PANEL_PATH, panel(&read(PANEL_PATH)?)),
    ])
}

/// Write (or, with `check`, verify) the three generated `JetBrains` files.
/// Every token type and modifier the plugin must have a colour rule for is
/// one the server actually advertises, and vice versa.
fn verify_semantic_token_colors(root: &std::path::Path) -> Result<()> {
    let path = root.join(SEMANTIC_TOKENS_PATH);
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let mapped = quoted_keys(&source, "internal val SEMANTIC_TOKEN_COLORS", "\n)");
    let legend: Vec<String> = tcl_lsp_core::semantic_tokens::legend_token_types()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let missing: Vec<&String> = legend.iter().filter(|t| !mapped.contains(*t)).collect();
    let unknown: Vec<&String> = mapped.iter().filter(|t| !legend.contains(t)).collect();
    if !missing.is_empty() || !unknown.is_empty() {
        anyhow::bail!(
            "{SEMANTIC_TOKENS_PATH} SEMANTIC_TOKEN_COLORS disagrees with the server \
             legend — uncoloured token types: {missing:?}; types the server never \
             emits: {unknown:?}"
        );
    }

    // The modifier half: a rule keyed on a modifier the server cannot set is
    // dead, and reads as a colour the user will never see.
    let modifiers = tcl_lsp_core::semantic_tokens::legend_token_modifiers();
    for key in quoted_keys(&source, "SEMANTIC_TOKEN_MODIFIER_COLORS", "\n)") {
        if !legend.contains(&key) && !modifiers.contains(&key.as_str()) {
            anyhow::bail!(
                "{SEMANTIC_TOKENS_PATH} SEMANTIC_TOKEN_MODIFIER_COLORS names {key:?}, \
                 which is in neither the token-type nor the token-modifier legend"
            );
        }
    }
    Ok(())
}

/// Every double-quoted string between `start` and the next `end` after it.
fn quoted_keys(source: &str, start: &str, end: &str) -> Vec<String> {
    let Some(from) = source.find(start) else {
        return Vec::new();
    };
    let rest = &source[from..];
    let body = rest.find(end).map_or(rest, |n| &rest[..n]);
    let mut out = Vec::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        if c != '"' {
            continue;
        }
        if let Some(len) = body[i + 1..].find('"') {
            out.push(body[i + 1..i + 1 + len].to_owned());
            for _ in 0..body[i + 1..=i + len + 1].chars().count() {
                chars.next();
            }
        }
    }
    out
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    verify_semantic_token_colors(&root)?;
    let mut drift = Vec::new();
    for (rel, content) in artifacts()? {
        let path = root.join(rel);
        if check {
            let current = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            if current != content {
                drift.push(rel);
            }
        } else if write_if_changed(&path, &content)? {
            eprintln!("wrote {rel}");
        }
    }
    if check && !drift.is_empty() {
        eprintln!(
            "{} JetBrains file(s) are stale — run `cargo xtask gen-jetbrains-catalog`:",
            drift.len()
        );
        for rel in &drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: JetBrains generated files are in sync with the DiagCode catalogue.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_label_truncates_and_escapes() {
        assert_eq!(
            short_label("W001", "Unknown subcommand."),
            "W001: Unknown subcommand"
        );
        // Split on em-dash, strip backticks.
        assert_eq!(
            short_label(
                "W110",
                "Use `eq`/`ne` instead of `==`/`!=` for string comparison — foo"
            ),
            "W110: Use eq/ne instead of ==/!= for string comparison"
        );
        // `$` is Kotlin-escaped.
        assert_eq!(short_label("X", "a $b"), "X: a \\$b");
    }

    #[test]
    fn the_semantic_token_colour_map_covers_the_server_legend() {
        verify_semantic_token_colors(&repo_root())
            .expect("every advertised semantic-token type needs a colour in the plugin");
    }

    #[test]
    fn quoted_keys_reads_only_the_block_it_is_given() {
        let source = "internal val A = mapOf(\n  \"x\" to 1,\n  \"y\" to 2,\n)\nval B = \"z\"\n";
        assert_eq!(quoted_keys(source, "internal val A", "\n)"), ["x", "y"]);
        assert!(quoted_keys(source, "no such marker", "\n)").is_empty());
    }

    #[test]
    fn every_diagnostic_section_gets_its_own_titled_reflowing_grid() {
        let panel = artifacts()
            .expect("render JetBrains files")
            .into_iter()
            .find(|(rel, _)| rel.ends_with("TclLspSettingsPanel.kt"))
            .expect("the settings panel is generated")
            .1;

        let groups = diag_section_groups();
        for (title, _) in &groups {
            assert_eq!(
                panel
                    .matches(&format!("TitledSeparator(\"{title}\")"))
                    .count(),
                1,
                "diagnostics section {title:?} must be emitted exactly once"
            );
        }
        let diag_ui = panel
            .split_once("// @generated:diag-ui:begin")
            .and_then(|(_, rest)| rest.split_once("// @generated:diag-ui:end"))
            .expect("the diagnostics UI block")
            .0;
        assert_eq!(
            diag_ui.matches("ReflowingGrid(").count(),
            groups.len(),
            "one reflowing grid per diagnostics section"
        );
        // A fixed column count is what made the page wider than any settings
        // pane and pushed its right-hand columns out of reach.
        assert!(
            !panel.contains("GridLayout("),
            "the settings page must not lay checkboxes out in a fixed number of columns"
        );
    }

    #[test]
    fn committed_jetbrains_files_match_generated() {
        for (rel, content) in artifacts().expect("read JetBrains files") {
            let current = std::fs::read_to_string(repo_root().join(rel))
                .unwrap_or_else(|e| panic!("reading {rel}: {e}"));
            assert_eq!(
                current, content,
                "{rel} is stale — run `cargo xtask gen-jetbrains-catalog`"
            );
        }
    }
}
