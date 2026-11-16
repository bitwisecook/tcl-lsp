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

#[test]
fn report_model_preserves_and_routes_validator_output() {
    let source = "ltm pool /Common/empty {\n}\n\
                      ltm data-group internal /Common/bad {\n  type ip\n  records {\n    nope { }\n  }\n}\n\
                      ltm rule /Common/a {\n  when HTTP_REQUEST { one }\n}\n\
                      ltm rule /Common/b {\n  when HTTP_REQUEST { two }\n}\n\
                      ltm virtual /Common/vs {\n  rules { /Common/a /Common/b }\n}\n";
    let mut device = Map::new();
    let sources = vec![("memory://config.bigip.conf".to_owned(), source.to_owned())];
    add_config_diagnostics(&mut device, "memory://config.bigip.conf", source, &sources);

    let codes = |key: &str| {
        device[key]
            .as_array()
            .expect("diagnostic array")
            .iter()
            .filter_map(|item| item["code"].as_str())
            .collect::<Vec<_>>()
    };
    assert!(codes("poolDiagnostics").contains(&"BIGIP6008"));
    assert!(codes("dataGroupDiagnostics").contains(&"BIGIP6011"));
    assert!(codes("virtualDiagnostics").contains(&"BIGIP6012"));
    assert_eq!(
        device["configDiagnostics"].as_array().map(Vec::len),
        Some(
            device["virtualDiagnostics"].as_array().unwrap().len()
                + device["ruleDiagnostics"].as_array().unwrap().len()
                + device["poolDiagnostics"].as_array().unwrap().len()
                + device["dataGroupDiagnostics"].as_array().unwrap().len()
                + device["appDiagnostics"].as_array().unwrap().len()
                + device["objectDiagnostics"].as_array().unwrap().len()
        )
    );
}

#[test]
fn registry_diagnostics_route_to_the_object_index() {
    let source = "ltm virtual /Common/vs { fallback-persistence /Common/missing }\n";
    let mut device = Map::new();
    let sources = vec![("memory://config.bigip.conf".to_owned(), source.to_owned())];
    add_config_diagnostics(&mut device, "memory://config.bigip.conf", source, &sources);
    assert!(
        device["objectDiagnostics"]
            .as_array()
            .expect("object diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "BIGIP6013")
    );
}

#[test]
fn iapp_source_obligations_withdraw_unused_field_claims() {
    let sources = vec![
        (
            "memory://iapp/presentation.apl".to_owned(),
            "section basic { string addr; string port }".to_owned(),
        ),
        (
            "memory://iapp/implementation".to_owned(),
            "eval $implementation\nset a $::basic__addr".to_owned(),
        ),
    ];
    let evidence = collect_iapp_diagnostic_evidence(&sources[0].0, &sources);
    assert_eq!(evidence["state"], "partial_source");
    let diagnostics = collect_report_diagnostics(&sources[0].0, &sources[0].1, &sources);
    assert!(
        !diagnostics
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "IAPP7002")
    );
}

#[test]
fn iapp_diagnostics_are_global_routed_to_apps_and_mark_partial_evidence() {
    let presentation =
        "#include \"missing.inc\"\nsection basic {\n  string addr\n  string port\n}\n";
    let implementation = "set ok $::basic__addr\nset missing $::basic__missing\n";
    let sources = vec![
        (
            "memory://iapp/presentation.apl".to_owned(),
            presentation.to_owned(),
        ),
        (
            "memory://iapp/implementation".to_owned(),
            implementation.to_owned(),
        ),
    ];

    let model = collect_model(&sources, "iApp diagnostics");
    let devices = model["devices"].as_array().expect("devices");
    let presentation_device = devices
        .iter()
        .find(|device| device["uri"] == "memory://iapp/presentation.apl")
        .expect("presentation device");
    let implementation_device = devices
        .iter()
        .find(|device| device["uri"] == "memory://iapp/implementation")
        .expect("implementation device");
    let codes = |device: &J| {
        device["configDiagnostics"]
            .as_array()
            .expect("global diagnostics")
            .iter()
            .filter_map(|item| item["code"].as_str().map(str::to_owned))
            .collect::<Vec<_>>()
    };
    assert!(codes(presentation_device).contains(&"IAPP7002".to_owned()));
    assert!(codes(presentation_device).contains(&"IAPP7003".to_owned()));
    assert!(codes(implementation_device).contains(&"IAPP7001".to_owned()));
    assert!(
        presentation_device["appDiagnostics"]
            .as_array()
            .expect("apps diagnostics")
            .iter()
            .all(|item| item["tab"] == "apps")
    );
    assert_eq!(
        presentation_device["iappDiagnosticEvidence"]["state"],
        "complete"
    );

    let partial_sources = vec![(
        "memory://iapp/presentation.apl".to_owned(),
        presentation.to_owned(),
    )];
    let partial = collect_model(&partial_sources, "partial iApp diagnostics");
    assert_eq!(
        partial["devices"][0]["iappDiagnosticEvidence"]["state"],
        "presentation_only"
    );
}
