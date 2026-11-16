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

//! The bundled tier: the EDA vendor loadables and the UPF hooks reach the editor.

use super::*;

const EDA_COMMAND_CASES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "xilinx-eda-tcl",
        "synth_design -top top",
        "synth_design",
        "vsim -c top",
        "vsim",
    ),
    (
        "synopsys-eda-tcl",
        "compile_ultra",
        "compile_ultra",
        "vsim -c top",
        "vsim",
    ),
    (
        "cadence-eda-tcl",
        "ccopt_design",
        "ccopt_design",
        "synth_design -top top",
        "synth_design",
    ),
    (
        "intel-quartus-eda-tcl",
        "execute_flow -compile",
        "execute_flow",
        "synth_design -top top",
        "synth_design",
    ),
    (
        "mentor-eda-tcl",
        "vsim -c top",
        "vsim",
        "synth_design -top top",
        "synth_design",
    ),
    (
        "microchip-libero-eda-tcl",
        "run_designer",
        "run_designer",
        "synth_design -top top",
        "synth_design",
    ),
];

/// The **bundled** tier, end to end: the EDA vendor libraries are `.tclspec`
/// loadables now, not compiled-in Rust (`docs/design/registry/spec-packs.md`), so this
/// is the proof that a shipped pack reaches the analyser in the real server
/// process. Each vendor command must be discovered, parsed, merged and installed
/// before the corresponding document can resolve it.
///
/// The negative half matters as much: every vendor's pack is discovered for
/// every dialect, so a rival's command must still be unknown here.
#[test]
fn the_bundled_eda_loadables_make_their_vendor_commands_known() {
    let root = workspace("bundled-eda");
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    await_pack_named(&mut lsp, "eda_xilinx");
    let loaded = loaded_packs(&mut lsp);
    for name in [
        "eda_xilinx",
        "eda_synopsys",
        "eda_cadence",
        "eda_quartus",
        "eda_mentor",
        "eda_microchip",
        "sdc_base",
        "upf",
    ] {
        let pack = loaded
            .iter()
            .find(|pack| pack["name"] == name)
            .unwrap_or_else(|| panic!("{name}: {loaded:?}"));
        assert!(
            pack["commands"].as_u64().is_some_and(|count| count > 0),
            "{name} must have commands before the first vendor request: {pack:?}"
        );
    }

    let unknown_of = |diags: &[Value]| -> Vec<String> {
        diags
            .iter()
            .filter(|d| matches!(d.get("code").and_then(Value::as_str), Some("W123" | "W002")))
            .filter_map(|d| d.get("message").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect()
    };
    for &(dialect, own_call, own, rival_call, rival) in EDA_COMMAND_CASES {
        let source = format!(
            "# tcl-dialect: {dialect}\n{own_call}\nget_cells *\ncreate_power_domain PD\n{rival_call}\n"
        );
        let doc = root.join(format!("{dialect}.tcl"));
        write(&doc, &source);
        let uri = file_uri(&doc);
        lsp.open_ready(&uri, &source);
        let first_hover = hover_text(&lsp.hover(&uri, 1, 1));
        assert!(
            first_hover.contains(own),
            "{dialect}: the first vendor hover must know `{own}`: {first_hover}"
        );
        let diagnostics =
            lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(30), |diags| {
                !unknown_of(diags)
                    .iter()
                    .any(|message| message.contains(own))
            });
        let unknown = unknown_of(&diagnostics);

        for command in [own, "get_cells", "create_power_domain"] {
            assert!(
                !unknown.iter().any(|message| message.contains(command)),
                "{dialect}: bundled command `{command}` must resolve: {diagnostics:#?}"
            );
        }
        assert!(
            unknown.iter().any(|message| message.contains(rival)),
            "{dialect}: rival `{rival}` must remain unknown: {diagnostics:#?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

fn assert_upf_optional_option_tokens(
    tokens: &[crate::common::helpers::SemToken],
    legend: &[String],
    expected_options: Vec<(i64, i64, &str)>,
    expected_values: Vec<(i64, i64, &str, &str)>,
) {
    for (line, column, option) in expected_options {
        assert!(
            tokens.iter().any(|token| token.line == line
                && token.char == column
                && token.length == i64::try_from(option.len()).unwrap()
                && legend[usize::try_from(token.ttype).unwrap()] == "decorator"),
            "{line}:{column}: {option} must remain a declared option after its predecessor: {tokens:?}"
        );
    }
    for (line, column, value, kind) in expected_values {
        assert!(
            tokens.iter().any(|token| token.line == line
                && token.char == column
                && token.length == i64::try_from(value.len()).unwrap()
                && legend[usize::try_from(token.ttype).unwrap()] == kind),
            "{line}:{column}: {value} must be consumed as the declared option value ({kind}): {tokens:?}"
        );
    }
}

/// Every shipped UPF optional-value slot reaches the ordinary server's option
/// scanner. Direct `upf_hooks` tests independently prove all 17 body answers;
/// here a following option must remain an option when the value is omitted.
#[test]
fn every_bundled_upf_optional_value_hook_reaches_editor_option_scanning() {
    const BOOLEAN_SLOTS: &[(&str, &str)] = &[
        ("connect_supply_set", "-transitive"),
        ("find_objects", "-transitive"),
        ("set_design_attributes", "-is_soft_macro"),
        ("set_design_attributes", "-is_hard_macro"),
        ("set_design_attributes", "-is_refinable_macro"),
        ("set_design_attributes", "-is_power_aware_model"),
        ("set_equivalent", "-interchangeable"),
        ("set_isolation", "-diff_supply_only"),
        ("set_isolation", "-use_functional_equivalence"),
        ("set_level_shifter", "-use_functional_equivalence"),
        ("set_repeater", "-use_functional_equivalence"),
        ("set_retention_elements", "-transitive"),
        ("sim_assertion_control", "-transitive"),
        ("sim_corruption_control", "-transitive"),
        ("sim_replay_control", "-transitive"),
    ];
    assert_eq!(BOOLEAN_SLOTS.len(), 15);
    let root = workspace("bundled-upf-option-hooks");
    let mut source = String::from("# tcl-dialect: xilinx-eda-tcl\n");
    let mut expected_options = Vec::new();
    let mut expected_values = Vec::new();
    for &(command, option) in BOOLEAN_SLOTS {
        // The first option takes zero words; the second takes TRUE. A fixed
        // one-word fallback instead eats the second option as a closed value.
        let line = format!("{command} {option} {option} TRUE UPF_TARGET\n");
        let line_number = i64::try_from(source.lines().count()).expect("line number");
        for (column, _) in line.match_indices(option) {
            expected_options.push((line_number, i64::try_from(column).unwrap(), option));
        }
        expected_values.push((
            line_number,
            i64::try_from(line.find(" TRUE ").unwrap() + 1).unwrap(),
            "TRUE",
            "enumMember",
        ));
        source.push_str(&line);
    }
    for line in [
        "set_port_attributes -feedthrough -unconnected\n",
        "set_port_attributes -feedthrough FT -unconnected\n",
    ] {
        let line_number = i64::try_from(source.lines().count()).expect("line number");
        for option in ["-feedthrough", "-unconnected"] {
            expected_options.push((
                line_number,
                i64::try_from(line.find(option).unwrap()).unwrap(),
                option,
            ));
        }
        if let Some(column) = line.find(" FT ") {
            expected_values.push((
                line_number,
                i64::try_from(column + 1).unwrap(),
                "FT",
                "property",
            ));
        }
        source.push_str(line);
    }
    let doc = root.join("upf-options.tcl");
    write(&doc, &source);
    let uri = file_uri(&doc);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    await_pack_named(&mut lsp, "upf");
    lsp.open_ready(&uri, &source);
    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
    assert_upf_optional_option_tokens(&tokens, &legend, expected_options, expected_values);
    let diagnostics =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(30), |diags| {
            !diags.iter().any(|diag| {
                matches!(
                    diag.get("code").and_then(Value::as_str),
                    Some("E002" | "E003" | "W002" | "W123" | "W127")
                )
            })
        });
    assert!(
        !diagnostics.iter().any(|diag| matches!(
            diag.get("code").and_then(Value::as_str),
            Some("E002" | "E003" | "W002" | "W123" | "W127")
        )),
        "optional words/closed booleans must be consumed under the shipped hook answers: {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The moving trailing body role must reach both diagnostics and memoised
/// semantic tokens, while an ordinary braced UPF data list remains data.
#[test]
fn the_bundled_upf_body_resolver_reaches_editor_diagnostics_and_tokens() {
    let root = workspace("bundled-upf-body-hook");
    let source = "# tcl-dialect: xilinx-eda-tcl\ndefine_power_model PM {\n    upf_body_plain_missing\n    set from_plain 1\n}\ndefine_power_model PM2 -for {MODEL} {\n    upf_body_option_missing\n    set from_option 1\n}\ncreate_power_domain PD -elements {\n    upf_body_data_missing\n    set not_script 1\n}\n";
    let doc = root.join("upf-bodies.tcl");
    write(&doc, source);
    let uri = file_uri(&doc);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    await_pack_named(&mut lsp, "upf");
    lsp.open_ready(&uri, source);
    let missing = |diags: &[Value], marker: &str| {
        diags.iter().any(|diag| {
            diag.get("code").and_then(Value::as_str) == Some("W123")
                && diag
                    .get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|message| message.contains(marker))
        })
    };
    let diagnostics =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(30), |diags| {
            missing(diags, "upf_body_plain_missing") && missing(diags, "upf_body_option_missing")
        });
    for (marker, line) in [
        ("upf_body_plain_missing", 2),
        ("upf_body_option_missing", 6),
    ] {
        assert!(
            diagnostics.iter().any(
                |diag| diag.get("code").and_then(Value::as_str) == Some("W123")
                    && diag["range"]["start"]["line"].as_i64() == Some(line)
                    && diag
                        .get("message")
                        .and_then(Value::as_str)
                        .is_some_and(|message| message.contains(marker))
            ),
            "the resolver's actual trailing body must be analysed: {diagnostics:#?}"
        );
    }
    assert!(
        !missing(&diagnostics, "upf_body_data_missing"),
        "ordinary UPF list data is not a script: {diagnostics:#?}"
    );
    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
    for line in [3, 7] {
        assert!(
            tokens.iter().any(|token| token.line == line
                && token.char == 4
                && token.length == 3
                && legend[usize::try_from(token.ttype).unwrap()] == "function"),
            "set inside each resolved body must be a command: {tokens:?}"
        );
    }
    assert!(
        !tokens.iter().any(|token| token.line == 11
            && token.char == 4
            && legend[usize::try_from(token.ttype).unwrap()] == "function"),
        "set inside a data list must not be a command: {tokens:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
