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

//! GTM projection + GTM->LTM architecture linking, from inline SCF sources.

use bigip_report_gen_rust::{Source, collect_model};
use serde_json::Value as J;

/// A GTM (DNS) device: a wide-IP -> pool -> member chain, plus a `gtm server`
/// whose virtual-server destination (10.1.0.10:443) is the LTM tier's virtual.
const GTM: &str = r"#TMSH-VERSION: 15.1.0
gtm datacenter /Common/dc-east {
    location Boston
}
gtm server /Common/ltm-edge {
    datacenter /Common/dc-east
    product bigip
    addresses {
        10.0.0.5 { }
    }
    virtual-servers {
        app_vs {
            destination 10.1.0.10:443
        }
    }
}
gtm pool a /Common/app_pool {
    load-balancing-mode global-availability
    members {
        ltm-edge:app_vs {
            member-order 0
        }
    }
}
gtm wideip a /Common/app.example.com {
    pool-lb-mode topology
    aliases { www.example.com }
    pools {
        app_pool {
            order 0
        }
    }
}
";

/// The LTM tier the GTM fronts: a virtual on 10.1.0.10 (the GTM server's
/// virtual-server destination).
const LTM: &str = r"#TMSH-VERSION: 15.1.0
ltm virtual /Common/app_vs {
    destination /Common/10.1.0.10:443
    pool /Common/app_pool
}
ltm pool /Common/app_pool {
    members {
        /Common/10.9.0.9:8080 {
            address 10.9.0.9
        }
    }
}
ltm virtual-address /Common/10.1.0.10 {
    address 10.1.0.10
}
";

fn model() -> J {
    let sources: Vec<Source> = vec![
        ("gtm.scf".to_string(), GTM.to_string()),
        ("ltm.scf".to_string(), LTM.to_string()),
    ];
    collect_model(&sources, "GSLB Estate")
}

#[test]
fn gtm_objects_are_projected() {
    let m = model();
    let gtm = &m["devices"].as_array().unwrap()[0];

    let wideips = gtm["gtmWideips"].as_array().unwrap();
    assert_eq!(wideips.len(), 1);
    assert_eq!(wideips[0]["name"], "app.example.com");
    // Wide-IP pool references use the leaf name, as written in the config.
    assert_eq!(wideips[0]["pools"].as_array().unwrap()[0], "app_pool");
    assert_eq!(
        wideips[0]["aliases"].as_array().unwrap()[0],
        "www.example.com"
    );

    let pools = gtm["gtmPools"].as_array().unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0]["lbMode"], "global-availability");
    assert_eq!(
        pools[0]["members"].as_array().unwrap()[0]["name"],
        "ltm-edge:app_vs"
    );

    let servers = gtm["gtmServers"].as_array().unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0]["datacenter"], "/Common/dc-east");
    let vss = servers[0]["virtualServers"].as_array().unwrap();
    assert_eq!(vss[0], "10.1.0.10:443");

    let dcs = gtm["gtmDatacenters"].as_array().unwrap();
    assert_eq!(dcs.len(), 1);
    assert_eq!(dcs[0]["location"], "Boston");
}

#[test]
fn gtm_device_role_is_detected() {
    let m = model();
    let devs = m["architecture"]["devices"].as_array().unwrap();
    assert_eq!(devs[0]["role"], "gtm", "the GTM device is inferred as gtm");
    assert_eq!(devs[1]["role"], "ltm");
}

#[test]
fn gtm_fronts_ltm_link_is_detected() {
    let m = model();
    let links = m["architecture"]["links"].as_array().unwrap();
    assert_eq!(links.len(), 1, "GTM -> LTM");
    let l = &links[0];
    assert_eq!(l["from"], 0);
    assert_eq!(l["to"], 1);
    let via = &l["vias"].as_array().unwrap()[0];
    assert_eq!(via["address"], "10.1.0.10");
    assert_eq!(via["fromObj"], "/Common/ltm-edge");
    assert_eq!(via["toObj"], "/Common/app_vs");
    // Tiers: GTM tier 0, LTM tier 1.
    let devs = m["architecture"]["devices"].as_array().unwrap();
    assert_eq!(devs[0]["tier"], 0);
    assert_eq!(devs[1]["tier"], 1);
}
