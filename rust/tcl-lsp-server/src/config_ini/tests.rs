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
fn multiline_disabled_codes_list() {
    // A `configparser`-style continuation list joins into the same code set a
    // comma list would (Python test_user_config::test_multiline_disabled_list).
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
fn extra_commands_multiline() {
    // Continuation form of `extraCommands`, equivalent to the comma form
    // (Python test_user_config::test_extra_commands_multiline). NOTE: bare
    // command names join correctly; `::`-qualified names must use the comma
    // form because the continuation heuristic treats an indented `a::b` as a
    // `key: value` line (a known limitation — see the parity catalog).
    let ini = "[global]\n\
               extraCommands =\n\
               \x20   myhelper\n\
               \x20   myother\n";
    let s = settings_from_ini(ini, Layer::Global);
    assert_eq!(s["extraCommands"], json!(["myhelper", "myother"]));
    // The comma form handles namespace-qualified names.
    let comma = settings_from_ini("[global]\nextraCommands = a::b, c::d\n", Layer::Global);
    assert_eq!(comma["extraCommands"], json!(["a::b", "c::d"]));
}

#[test]
fn invalid_values_are_ignored_not_crashing() {
    // Non-bool / non-integer values are dropped, leaving the defaults intact
    // (Python test_user_config::test_invalid_bool_ignored /
    // test_non_integer_line_length_ignored / test_invalid_optimiser_bool_ignored).
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
    // An empty `dialect =` keeps the default (Python
    // test_user_config::test_empty_dialect_is_ignored).
    let s = settings_from_ini("[global]\ndialect =\n", Layer::Global);
    assert!(s.get("dialect").is_none(), "empty dialect ignored: {s}");
}

#[test]
fn top_level_keys_coexist_with_nested_sections() {
    // A `[global]` top-level key and a `[diagnostics]` section in one file are
    // both honoured (Python test_user_config::test_combined_with_other_sections).
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
    // → the W111 threshold. These are distinct settings (mirrors Python).
    assert_eq!(s["formatting"]["lineLength"], json!(90));
    assert_eq!(s["style"]["lineLength"], json!(100));
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
