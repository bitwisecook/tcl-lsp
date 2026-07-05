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

//! AFM security-firewall + NAT projection into the report model.

use serde_json::Value as J;
use tcl_bigip_report::{Source, collect_model};

const FW: &str = r"#TMSH-VERSION: 15.1.0
security firewall address-list /Common/webservers {
    addresses {
        10.0.0.0/24 { }
        192.168.1.5 { }
    }
}
security firewall port-list /Common/web-ports {
    ports {
        80 { }
        443 { }
    }
}
security firewall rule-list /Common/app-rules {
    rules {
        allow-web {
            action accept
            ip-protocol tcp
            log yes
            source {
                address-lists {
                    /Common/webservers
                }
            }
            destination {
                port-lists {
                    /Common/web-ports
                }
            }
        }
        deny-all {
            action drop
            ip-protocol any
        }
    }
}
security firewall policy /Common/edge-policy {
    rules {
        app {
            rule-list /Common/app-rules
        }
    }
}
security nat source-translation /Common/snat-xlate {
    type dynamic-pat
    addresses {
        203.0.113.10 { }
    }
    ports {
        1024-65535 { }
    }
}
security nat destination-translation /Common/dnat-xlate {
    type static-nat
    addresses {
        10.2.0.20 { }
    }
}
security nat policy /Common/nat-pol {
    rules {
        r1 {
            source-translation /Common/snat-xlate
        }
    }
}
";

fn device() -> J {
    let sources: Vec<Source> = vec![("fw.scf".to_string(), FW.to_string())];
    let m = collect_model(&sources, "Firewall Estate");
    m["devices"].as_array().unwrap()[0].clone()
}

#[test]
fn firewall_objects_are_projected() {
    let d = device();

    let addr = d["firewallAddressLists"].as_array().unwrap();
    assert_eq!(addr.len(), 1);
    assert_eq!(addr[0]["name"], "webservers");
    let addresses = addr[0]["addresses"].as_array().unwrap();
    assert!(addresses.iter().any(|a| a == "10.0.0.0/24"));

    let ports = d["firewallPortLists"].as_array().unwrap();
    assert_eq!(ports.len(), 1);
    assert!(ports[0]["ports"].as_array().unwrap().iter().any(|p| p == "443"));

    let policies = d["firewallPolicies"].as_array().unwrap();
    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0]["name"], "edge-policy");
    assert_eq!(
        policies[0]["ruleLists"].as_array().unwrap()[0],
        "/Common/app-rules"
    );
}

#[test]
fn firewall_rule_list_rules_are_structured() {
    let d = device();
    let rls = d["firewallRuleLists"].as_array().unwrap();
    assert_eq!(rls.len(), 1);
    let rl = &rls[0];
    assert_eq!(rl["ruleCount"], 2);
    let rules = rl["rules"].as_array().unwrap();

    let allow = &rules[0];
    assert_eq!(allow["name"], "allow-web");
    assert_eq!(allow["action"], "accept");
    assert_eq!(allow["ipProtocol"], "tcp");
    assert_eq!(allow["log"], J::Bool(true));
    assert_eq!(
        allow["source"]["addressLists"].as_array().unwrap()[0],
        "/Common/webservers"
    );
    assert_eq!(
        allow["destination"]["portLists"].as_array().unwrap()[0],
        "/Common/web-ports"
    );

    let deny = &rules[1];
    assert_eq!(deny["action"], "drop");
}

#[test]
fn nat_objects_are_projected() {
    let d = device();

    let src = d["natSourceTranslations"].as_array().unwrap();
    assert_eq!(src.len(), 1);
    assert_eq!(src[0]["type"], "dynamic-pat");
    assert!(src[0]["addresses"].as_array().unwrap().iter().any(|a| a == "203.0.113.10"));

    let dst = d["natDestinationTranslations"].as_array().unwrap();
    assert_eq!(dst.len(), 1);
    assert_eq!(dst[0]["type"], "static-nat");

    let pol = d["natPolicies"].as_array().unwrap();
    assert_eq!(pol.len(), 1);
    assert_eq!(pol[0]["name"], "nat-pol");
}
