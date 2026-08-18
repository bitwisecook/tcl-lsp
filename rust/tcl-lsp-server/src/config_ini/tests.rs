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

//! Unit tests for INI config-file parsing + layer merging.

use super::*;
use serde_json::json;

#[test]
fn global_section_top_level_keys() {
    let ini = "[global]\n\
               dialect = tcl9.0\n\
               extraCommands = mylib::send, mylib::recv\n\
               libraryPaths =\n\
               \x20   /opt/tcl/lib\n\
               \x20   /home/me/stubs\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["dialect"], json!("tcl9.0"));
    assert_eq!(s["extraCommands"], json!(["mylib::send", "mylib::recv"]));
    assert_eq!(s["libraryPaths"], json!(["/opt/tcl/lib", "/home/me/stubs"]));
}

#[test]
fn project_entry_points_list() {
    // Newline-continuation list of entry files under [project].
    let ini = "[project]\n\
               entryPoints =\n\
               \x20   main.tcl\n\
               \x20   src/app.tcl\n";
    let s = settings_from_ini(ini, Layer::Project);
    assert_eq!(s["entryPoints"], json!(["main.tcl", "src/app.tcl"]));
    // A comma-separated one-liner is equivalent.
    let comma = settings_from_ini(
        "[project]\nentryPoints = main.tcl, src/app.tcl\n",
        Layer::Project,
    );
    assert_eq!(comma["entryPoints"], json!(["main.tcl", "src/app.tcl"]));
    // Absent key ⇒ no entryPoints emitted (auto-detection stays on).
    let none = settings_from_ini("[project]\ndialect = tcl9.0\n", Layer::Project);
    assert!(none.get("entryPoints").is_none());
}

#[test]
fn project_layer_reads_project_section_only() {
    let ini = "[global]\ndialect = tcl8.6\n[project]\ndialect = tcl9.0\n";
    // As a project file, the [project] dialect is honoured and [global] ignored.
    assert_eq!(
        settings_from_ini(ini, Layer::Project)["dialect"],
        json!("tcl9.0")
    );
    // As a global file, the [global] dialect is honoured and [project] ignored.
    assert_eq!(
        settings_from_ini(ini, Layer::Global)["dialect"],
        json!("tcl8.6")
    );
}

#[test]
fn diagnostics_disabled_and_patterns() {
    let ini = "[diagnostics]\n\
               disabled = W111, T100\n\
               generic_variable_patterns =\n\
               \x20   ^dbg$\n\
               \x20   ^log_(level|server)$\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["diagnostics"]["W111"], json!(false));
    assert_eq!(s["diagnostics"]["T100"], json!(false));
    assert_eq!(
        s["diagnostics"]["genericVariablePatterns"],
        json!(["^dbg$", "^log_(level|server)$"])
    );
}

#[test]
fn diagnostics_exclude_glob_list() {
    // `exclude` is a one-pattern-per-line list (#1556); it is never
    // comma-split, so a brace alternation keeps its comma.
    let ini = "[diagnostics]\n\
               exclude =\n\
               \x20   docs/**\n\
               \x20   generated/[a-c]*.tcl\n\
               \x20   {vendor,third_party}/**\n\
               \x20   *.ruff\n";
    let s = settings_from_ini(ini, Layer::Project);
    assert_eq!(
        s["diagnostics"]["exclude"],
        json!([
            "docs/**",
            "generated/[a-c]*.tcl",
            "{vendor,third_party}/**",
            "*.ruff"
        ])
    );
    // A one-line value is a single pattern, commas included.
    let one = settings_from_ini("[diagnostics]\nexclude = {a,b}/*.tcl\n", Layer::Project);
    assert_eq!(one["diagnostics"]["exclude"], json!(["{a,b}/*.tcl"]));
    // Absent key ⇒ no `exclude` entry at all.
    let none = settings_from_ini("[diagnostics]\ndisabled = W111\n", Layer::Project);
    assert!(none["diagnostics"].get("exclude").is_none());
}

#[test]
fn multiline_disabled_codes_list() {
    // A `configparser`-style continuation list joins into the same code set a
    // comma list would.
    let ini = "[diagnostics]\n\
               disabled =\n\
               \x20   W111\n\
               \x20   T100\n\
               \x20   W120\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["diagnostics"]["W111"], json!(false));
    assert_eq!(s["diagnostics"]["T100"], json!(false));
    assert_eq!(s["diagnostics"]["W120"], json!(false));
}

#[test]
fn diagnostic_severity_section() {
    // `[diagnosticSeverity]` entries pass through verbatim as the nested
    // `diagnosticSeverity` object `settings_severity_overrides` parses;
    // validation (and skip-unknown) happens there, not here.
    let ini = "[diagnosticSeverity]\n\
               W211 = warning\n\
               W220 = Error\n\
               W210 = nonsense\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["diagnosticSeverity"]["W211"], json!("warning"));
    assert_eq!(s["diagnosticSeverity"]["W220"], json!("Error"));
    assert_eq!(s["diagnosticSeverity"]["W210"], json!("nonsense"));
    // Absent section -> no key at all (leave the current overrides untouched).
    let none = settings_from_ini("[diagnostics]\ndisabled = W111\n", Layer::Global);
    assert!(none.get("diagnosticSeverity").is_none());
}

#[test]
fn extra_commands_multiline() {
    // Continuation form of `extraCommands`, equivalent to the comma form.
    // `::`-qualified
    // names now join correctly even in the continuation form (the parser treats
    // any indented line as a continuation, configparser-style).
    let ini = "[global]\n\
               extraCommands =\n\
               \x20   mylib::send\n\
               \x20   mylib::recv\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["extraCommands"], json!(["mylib::send", "mylib::recv"]));
    // The comma form also handles namespace-qualified names.
    let comma = settings_from_ini("[global]\nextraCommands = a::b, c::d\n", Layer::Global);
    assert_eq!(comma["extraCommands"], json!(["a::b", "c::d"]));
}

#[test]
fn indented_comment_inside_multiline_value_is_not_absorbed() {
    // An indented `#`/`;` line inside a continuation value is a full-line
    // comment (configparser semantics), NOT part of the value — otherwise a
    // commented-out `# recv` entry would become a live extra command
    // (issue 176).
    let ini = "[global]\n\
               extraCommands =\n\
               \x20   mylib::send\n\
               \x20   # mylib::recv\n\
               \x20   ; mylib::log\n\
               \x20   mylib::flush\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(
        s["extraCommands"],
        json!(["mylib::send", "mylib::flush"]),
        "commented continuation lines must be dropped"
    );
}

#[test]
fn continuation_lines_with_colons_join_correctly() {
    // A regex pattern with a `:` and a `::`-qualified name in continuation lists
    // must join as continuations, not be mis-read as new `key: value` lines.
    let ini = "[diagnostics]\n\
               generic_variable_patterns =\n\
               \x20   ^a:b$\n\
               \x20   ^c$\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(
        s["diagnostics"]["genericVariablePatterns"],
        json!(["^a:b$", "^c$"])
    );
}

#[test]
fn invalid_values_are_ignored_not_crashing() {
    // Non-bool / non-integer values are dropped, leaving the defaults intact.
    let ini = "[shimmer]\nenabled = banana\n\
               [optimiser]\nenabled = maybe\n\
               [formatting]\nmax_line_length = abc\n\
               [style]\nline_length = wide\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert!(
        s.get("shimmer").is_none(),
        "invalid shimmer bool dropped: {s}"
    );
    assert!(
        s.get("optimiser").is_none(),
        "invalid optimiser bool dropped: {s}",
    );
    assert!(
        s.get("formatting").is_none(),
        "non-integer formatting line length dropped: {s}",
    );
    assert!(
        s.get("style").is_none(),
        "non-integer style line length dropped: {s}",
    );
}

#[test]
fn empty_dialect_is_ignored() {
    // An empty `dialect =` keeps the default.
    let s = settings_from_ini("[global]\ndialect =\n", Layer::Global);
    assert!(s.get("dialect").is_none(), "empty dialect ignored: {s}");
}

#[test]
fn top_level_keys_coexist_with_nested_sections() {
    // A `[global]` top-level key and a `[diagnostics]` section in one file are
    // both honoured.
    let ini = "[global]\ndialect = tcl9.0\n[diagnostics]\ndisabled = W111\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["dialect"], json!("tcl9.0"));
    assert_eq!(s["diagnostics"]["W111"], json!(false));
}

#[test]
fn optimiser_section() {
    let ini = "[optimiser]\nenabled = true\nprofile = readability\ndisabled = O109, O126\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["optimiser"]["enabled"], json!(true));
    assert_eq!(s["optimiser"]["profile"], json!("readability"));
    assert_eq!(s["optimiser"]["O109"], json!(false));
    assert_eq!(s["optimiser"]["O126"], json!(false));
}

#[test]
fn features_shimmer_xc_and_line_length() {
    let ini = "[features]\nhover = true\ninlayHints = false\n\
               [shimmer]\nenabled = true\n\
               [xcDiagnostics]\nenabled = false\n\
               [formatting]\nmax_line_length = 90\n\
               [style]\nline_length = 100\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["features"]["hover"], json!(true));
    assert_eq!(s["features"]["inlayHints"], json!(false));
    assert_eq!(s["shimmer"]["enabled"], json!(true));
    assert_eq!(s["xcDiagnostics"]["enabled"], json!(false));
    // `[formatting] max_line_length` → formatter width; `[style] line_length`
    // → the W111 threshold. These are distinct settings.
    assert_eq!(s["formatting"]["lineLength"], json!(90));
    assert_eq!(s["style"]["lineLength"], json!(100));
}

#[test]
fn formatting_section_maps_all_keys() {
    // Every `[formatting]` key maps to its camelCase editor key with the right
    // value type.
    let ini = "[formatting]\n\
               max_line_length = 100\n\
               goal_line_length = 90\n\
               indent_size = 2\n\
               indent_style = tabs\n\
               brace_style = k_and_r\n\
               line_ending = crlf\n\
               trim_trailing_whitespace = false\n\
               expand_single_line_bodies = true\n";
    let s = settings_from_ini(ini, Layer::Global);
    let f = &s["formatting"];
    assert_eq!(f["maxLineLength"], json!(100));
    // The legacy `lineLength` alias is also emitted for the server's resolved
    // willSaveWaitUntil width.
    assert_eq!(f["lineLength"], json!(100));
    assert_eq!(f["goalLineLength"], json!(90));
    assert_eq!(f["indentSize"], json!(2));
    assert_eq!(f["indentStyle"], json!("tabs"));
    assert_eq!(f["braceStyle"], json!("k_and_r"));
    assert_eq!(f["lineEnding"], json!("crlf"));
    assert_eq!(f["trimTrailingWhitespace"], json!(false));
    assert_eq!(f["expandSingleLineBodies"], json!(true));
}

#[test]
fn comments_and_blank_lines_ignored() {
    let ini = "# a comment\n[global]\n; another\ndialect = tcl9.0\n\n";
    assert_eq!(
        settings_from_ini(ini, Layer::Global)["dialect"],
        json!("tcl9.0")
    );
}

#[test]
fn merge_is_deep_with_high_layer_winning() {
    let low = json!({
        "optimiser": {"profile": "readability", "enabled": true},
        "dialect": "tcl8.6",
        "features": {"hover": true},
    });
    let high = json!({
        "optimiser": {"enabled": false, "O109": false},
        "dialect": "tcl9.0",
    });
    let merged = merge_settings(&low, &high);
    // Section merged key-by-key: profile inherited, enabled overridden, code added.
    assert_eq!(merged["optimiser"]["profile"], json!("readability"));
    assert_eq!(merged["optimiser"]["enabled"], json!(false));
    assert_eq!(merged["optimiser"]["O109"], json!(false));
    // Scalar overridden.
    assert_eq!(merged["dialect"], json!("tcl9.0"));
    // Untouched section preserved.
    assert_eq!(merged["features"]["hover"], json!(true));
}

#[test]
fn merge_precedence_global_then_editor_then_project() {
    // global config.ini < editor < project .tcl-lsp.ini.
    let global = json!({"dialect": "tcl8.5", "optimiser": {"enabled": true}});
    let editor = json!({"dialect": "tcl8.6"});
    let project = json!({"dialect": "tcl9.0", "optimiser": {"profile": "full"}});
    let merged = merge_settings(&merge_settings(&global, &editor), &project);
    assert_eq!(merged["dialect"], json!("tcl9.0"), "project wins");
    assert_eq!(
        merged["optimiser"]["enabled"],
        json!(true),
        "global preserved"
    );
    assert_eq!(
        merged["optimiser"]["profile"],
        json!("full"),
        "project adds"
    );
}

#[test]
fn empty_or_absent_keys_emit_nothing() {
    assert_eq!(settings_from_ini("", Layer::Global), json!({}));
    assert_eq!(settings_from_ini("[global]\n", Layer::Global), json!({}));
}
