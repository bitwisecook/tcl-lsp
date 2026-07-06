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

//! End-to-end HTML render check: the Architecture, GTM and Firewall/NAT
//! sections appear in the generated report.

use tcl_bigip_report::{RenderOptions, Source, build_report};

const GTM: &str = r"#TMSH-VERSION: 15.1.0
gtm datacenter /Common/dc-east { location Boston }
gtm server /Common/ltm-edge {
    datacenter /Common/dc-east
    addresses { 10.0.0.5 { } }
    virtual-servers { app_vs { destination 10.1.0.10:443 } }
}
gtm pool a /Common/app_pool {
    members { ltm-edge:app_vs { } }
}
gtm wideip a /Common/app.example.com {
    pools { app_pool { } }
}
";

const LTM: &str = r"#TMSH-VERSION: 15.1.0
ltm virtual /Common/app_vs {
    destination /Common/10.1.0.10:443
    pool /Common/app_pool
}
ltm pool /Common/app_pool {
    members { /Common/10.9.0.9:8080 { address 10.9.0.9 } }
}
ltm virtual-address /Common/10.1.0.10 { address 10.1.0.10 }
security firewall address-list /Common/webservers {
    addresses { 10.0.0.0/24 { } }
}
security firewall rule-list /Common/app-rules {
    rules {
        allow-web {
            action accept
            ip-protocol tcp
            source { address-lists { /Common/webservers } }
        }
    }
}
";

fn render() -> String {
    let sources: Vec<Source> = vec![
        ("gtm.scf".to_string(), GTM.to_string()),
        ("ltm.scf".to_string(), LTM.to_string()),
    ];
    let opts = RenderOptions {
        embed_console: false,
        ..RenderOptions::default()
    };
    build_report(&sources, &opts).expect("report renders")
}

#[test]
fn architecture_section_renders() {
    let html = render();
    assert!(
        html.contains("class=\"architecture\""),
        "architecture section present"
    );
    assert!(html.contains("auto-detected"), "auto-detect note shown");
    // The GTM->LTM link with its evidencing address.
    assert!(html.contains("10.1.0.10"), "link evidence address rendered");
    // The interactive diagram host + its wiring are present.
    assert!(
        html.contains("id=\"archDiagram\""),
        "interactive diagram host present"
    );
    assert!(
        html.contains("initArchitecture"),
        "architecture init wired in topology.js"
    );
    // The elkjs graph (embedded in the model) drives the diagram: one node per
    // device, keyed by its `d<index>` id and the `device` node class.
    assert!(
        html.contains(r#"\"cls\":\"device\""#),
        "architecture graph (elkjs) embedded in the model"
    );
}

#[test]
fn gtm_and_firewall_tabs_render() {
    let html = render();
    assert!(html.contains("data-panel=\"gtm\""), "GTM panel present");
    assert!(html.contains("app.example.com"), "wide-IP shown");
    assert!(
        html.contains("data-panel=\"firewall\""),
        "firewall panel present"
    );
    assert!(html.contains("app-rules"), "rule-list shown");
}

/// Helper (opt-in): write a full report to `$WRITE_REPORT` for browser checks.
#[test]
fn write_report_for_browser() {
    let Ok(path) = std::env::var("WRITE_REPORT") else {
        return;
    };
    std::fs::write(&path, render()).expect("write report html");
}

#[test]
fn manifest_note_when_defined() {
    let sources: Vec<Source> = vec![
        ("gtm.scf".to_string(), GTM.to_string()),
        ("ltm.scf".to_string(), LTM.to_string()),
    ];
    let opts = RenderOptions {
        embed_console: false,
        architecture: Some("device gtm.scf -role gtm -label \"DNS Edge\"".to_string()),
        ..RenderOptions::default()
    };
    let html = build_report(&sources, &opts).expect("renders");
    assert!(html.contains("from manifest"), "manifest note shown");
    assert!(html.contains("DNS Edge"), "manifest label applied");
}
