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

use tcl_bigip::lint::*;
use tcl_bigip::parser::parse_bigip_conf;

fn lint(src: &str) -> Vec<Finding> {
    let cfg = parse_bigip_conf(src, "Common");
    run_lint(&[("u".to_owned(), &cfg)], &[], None, None)
}

#[test]
fn empty_pool_and_pool_without_monitor() {
    crate::install();
    let src = "\
ltm pool /Common/empty { }
ltm pool /Common/nomon {
    members {
        /Common/n1:80 { address 10.0.0.1 }
    }
}
ltm node /Common/n1 { address 10.0.0.1 }
";
    let findings = lint(src);
    assert!(findings.iter().any(|f| f.rule_id == "empty-pool"
        && f.full_path == "/Common/empty"
        && f.severity == "warning"));
    assert!(
        findings
            .iter()
            .any(|f| f.rule_id == "pool-without-monitor" && f.full_path == "/Common/nomon")
    );
    // Empty pool is NOT also flagged pool-without-monitor.
    assert!(
        !findings
            .iter()
            .any(|f| f.rule_id == "pool-without-monitor" && f.full_path == "/Common/empty")
    );
}

#[test]
fn orphan_monitor_checks_every_pool_and_member_reference() {
    crate::install();
    let findings = lint(
        r#"
ltm monitor http /Common/a { }
ltm monitor http /Tenant/a { }
ltm monitor http /Tenant/b { }
ltm monitor http /Tenant/c { }
ltm monitor http /Tenant/d { }
ltm monitor http /Tenant/unreferenced { }
ltm pool /Tenant/web {
    monitor "a and /Tenant/b"
    members {
        /Tenant/node:80 { monitor min 1 of { /Tenant/c "/Tenant/d" } }
    }
}
"#,
    );
    let mut orphans: Vec<_> = findings
        .iter()
        .filter(|finding| finding.rule_id == "orphan-monitor")
        .map(|finding| finding.full_path.as_str())
        .collect();
    orphans.sort_unstable();
    assert_eq!(orphans, ["/Common/a", "/Tenant/unreferenced"]);
}

#[test]
fn orphan_monitor_factory_vs_custom() {
    crate::install();
    let src = "\
ltm monitor http /Common/factory_mon { }
ltm monitor http /Partition2/custom_mon { }
";
    let cfg = parse_bigip_conf(src, "Partition2");
    let findings = run_lint(&[("u".to_owned(), &cfg)], &[], None, None);
    // /Common/ factory monitor downgrades to info.
    assert!(findings.iter().any(|f| f.rule_id == "orphan-monitor"
        && f.full_path == "/Common/factory_mon"
        && f.severity == "info"));
    assert!(findings.iter().any(|f| f.rule_id == "orphan-monitor"
        && f.full_path == "/Partition2/custom_mon"
        && f.severity == "warning"));
}

#[test]
fn virtual_without_pool_skips_when_irule_attached() {
    crate::install();
    let src = "\
ltm virtual /Common/vs_no_pool { }
ltm virtual /Common/vs_with_rule {
    rules {
        /Common/r1
    }
}
ltm rule /Common/r1 { when HTTP_REQUEST { } }
";
    let findings = lint(src);
    assert!(
        findings
            .iter()
            .any(|f| f.rule_id == "virtual-without-pool" && f.full_path == "/Common/vs_no_pool")
    );
    assert!(
        !findings
            .iter()
            .any(|f| f.rule_id == "virtual-without-pool" && f.full_path == "/Common/vs_with_rule")
    );
}

#[test]
fn irule_empty_when_and_deprecated() {
    crate::install();
    let src = "\
ltm rule /Common/r1 {
    when HTTP_REQUEST { }
    log_local0.info \"hi\"
}
";
    let findings = lint(src);
    assert!(findings.iter().any(|f| f.rule_id == "irule-empty-when"));
    assert!(
        findings
            .iter()
            .any(|f| f.rule_id == "irule-deprecated-command")
    );
}

#[test]
fn unknown_event_fires() {
    crate::install();
    let src = "ltm rule /Common/r1 { when NOT_A_REAL_EVENT { log local0. x } }";
    let findings = lint(src);
    assert!(
        findings
            .iter()
            .any(|f| f.rule_id == "irule-unknown-event" && f.message.contains("NOT_A_REAL_EVENT"))
    );
    // A real event does not fire.
    let ok = "ltm rule /Common/r2 { when HTTP_REQUEST { log local0. x } }";
    let cfg = parse_bigip_conf(ok, "Common");
    let f2 = run_lint(&[("u".to_owned(), &cfg)], &[], None, None);
    assert!(!f2.iter().any(|f| f.rule_id == "irule-unknown-event"));
}
