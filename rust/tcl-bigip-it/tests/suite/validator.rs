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

use tcl_bigip::validator::*;
fn codes(source: &str) -> Vec<String> {
    validate_bigip_source(source, "Common")
        .into_iter()
        .map(|d| d.code)
        .collect()
}
fn has(source: &str, code: &str) -> bool {
    codes(source).iter().any(|c| c == code)
}
#[test]
fn address_fixture_has_no_false_reference_or_ip_data_group_warnings() {
    crate::install();
    let source = include_str!(
        "../../../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses/addresses.conf"
    );
    let diagnostics: Vec<_> = validate_bigip_source(source, "Common")
        .into_iter()
        .filter(|diagnostic| matches!(diagnostic.code.as_str(), "BIGIP6011" | "BIGIP6013"))
        .collect();
    let (route_domains, diagnostics): (Vec<_>, Vec<_>) = diagnostics
        .into_iter()
        .partition(|diagnostic| diagnostic.message.contains("uses route domain 2"));
    assert_eq!(route_domains.len(), 7, "{route_domains:#?}");
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.code == "BIGIP6013"
                && ["/Common/internal", "/Common/fe80::1:80"]
                    .iter()
                    .any(|name| diagnostic.message.contains(name))
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn class_match_validates_only_static_items_of_ip_data_groups() {
    crate::install();
    let source = r"
ltm data-group internal /Common/addresses {
    type ip
    records { 192.0.2.0%0/24 { } }
}
ltm data-group internal /Common/words {
    type string
    records { invalid { } }
}
ltm rule /Common/probe {
    when CLIENT_ACCEPTED {
        class match -value 192.0.2.53%2 equals /Common/addresses
        class match 192.0.2.53/24 equals /Common/addresses
        class match invalid equals /Common/addresses
        class match invalid equals /Common/words
        class match $dynamic equals /Common/addresses
        class match invalid equals $dynamic
        # class match comment equals /Common/addresses
    }
}
";
    let diagnostics: Vec<_> = validate_bigip_source(source, "Common")
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "BIGIP6015")
        .collect();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(diagnostics[0].message.contains("'invalid'"));
    assert!(diagnostics[0].message.contains("/Common/addresses"));
}

#[test]
fn measured_21_1_address_rejections_have_diagnostics() {
    crate::install();
    let source = r"#TMSH-VERSION: 21.1.0.1
ltm virtual /Common/invalid_port { destination 192.0.2.53:65536 }
ltm virtual /Common/invalid_service { destination 192.0.2.53:nh234_unknown }
ltm virtual /Common/bad_colon { destination 2001:db8::53%0:80 }
ltm virtual /Common/good_v6 { destination 2001:db8::53%0.http }
ltm monitor tcp /Common/bad_wildcard { destination 2001:db8::53.* }
ltm monitor tcp /Common/good_colon { destination 2001:db8::53%0:80 }
ltm node /Common/bad_fqdn { fqdn { name example.com. } }
ltm node /Common/good_fqdn { fqdn { name xn--caf-dma.example } }
";
    let diagnostics: Vec<_> = validate_bigip_source(source, "Common")
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "BIGIP6015")
        .collect();
    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(!has(&source.replace("21.1.0.1", "17.1.0"), "BIGIP6015"));
}

#[test]
fn route_domain_requirements_are_selected_by_registry_property() {
    crate::install();
    let source = r"
ltm node /Common/missing { address 192.0.2.1%65432 }
ltm node /Common/explicit_zero { address 192.0.2.2%0 }
ltm node /Common/defined { address 192.0.2.3%11 }
ltm monitor tcp /Common/missing { destination 192.0.2.1%65432:80 }
ltm monitor https /Common/defined { destination 2001:db8::53%11:80 }
ltm data-group internal /Common/ip { type ip records { 192.0.2.1%65432 { } } }
net route-domain /Common/example { id 11 }
";
    let diagnostics: Vec<_> = validate_bigip_source(source, "Common")
        .into_iter()
        .filter(|diagnostic| diagnostic.message.contains("uses route domain"))
        .collect();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.code == "BIGIP6013" && diagnostic.message.contains("65432")
    }));
}

#[test]
fn bigip6002_fires_for_missing_pool() {
    crate::install();
    let src =
        "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
    assert!(has(src, "BIGIP6002"));
}

#[test]
fn bigip6002_quiet_when_pool_exists() {
    crate::install();
    let src = "ltm pool /Common/web {\n  members {\n    /Common/n:80 { address 10.0.0.1 }\n  }\n}\nltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/web\n  }\n}\n";
    assert!(!has(src, "BIGIP6002"));
}

#[test]
fn bigip6008_fires_for_empty_pool() {
    crate::install();
    let src = "ltm pool /Common/empty {\n}\n";
    assert!(has(src, "BIGIP6008"));
}

#[test]
fn bigip6001_fires_for_missing_data_group() {
    crate::install();
    let src = "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    if { [class match [HTTP::host] equals /Common/no_dg] } { }\n  }\n}\n";
    assert!(has(src, "BIGIP6001"));
}

#[test]
fn bigip6011_fires_for_invalid_ip_record() {
    crate::install();
    let src = "ltm data-group internal /Common/ips {\n  type ip\n  records {\n    not-an-ip { }\n  }\n}\n";
    assert!(has(src, "BIGIP6011"));
}

#[test]
fn bigip6011_quiet_for_valid_ip_records() {
    crate::install();
    let src = "ltm data-group internal /Common/ips {\n  type ip\n  records {\n    10.0.0.0/8 { }\n    192.168.1.1 { }\n  }\n}\n";
    assert!(!has(src, "BIGIP6011"));
}

#[test]
fn bigip6003_fires_for_virtual_referencing_undefined_irule() {
    crate::install();
    let src = "ltm virtual /Common/vs {\n  rules {\n    /Common/no_such_rule\n  }\n}\n";
    assert!(has(src, "BIGIP6003"));
}

#[test]
fn bigip6003_quiet_when_irule_is_defined() {
    crate::install();
    let src = "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n  }\n}\n";
    assert!(!has(src, "BIGIP6003"));
}

#[test]
fn bigip6013_uses_registry_edges_and_keeps_dynamic_references_unknown() {
    crate::install();
    let missing = "ltm virtual /Tenant/vs {\n  fallback-persistence /Tenant/missing\n}\n";
    let finding = validate_bigip_source(missing, "Tenant")
        .into_iter()
        .find(|diagnostic| diagnostic.code == "BIGIP6013")
        .expect("registry-declared fallback-persistence edge is validated");
    assert_eq!(finding.subject, ConfigDiagnosticSubject::Object);
    assert!(finding.message.contains("fallback-persistence"));

    let dynamic = "ltm virtual /Tenant/vs {\n  fallback-persistence $profile_name\n}\n";
    assert!(!has(dynamic, "BIGIP6013"));
}

#[test]
fn bigip6013_resolves_tenant_and_common_targets_without_a_handwritten_table() {
    crate::install();
    let source = "ltm persistence cookie /Common/common_persist { }\n\
                      ltm persistence cookie /Tenant/local_persist { }\n\
                      ltm virtual /Tenant/vs {\n\
                        fallback-persistence local_persist\n\
                      }\n";
    assert!(!has(source, "BIGIP6013"));
    let common = source.replace("local_persist", "common_persist");
    assert!(!has(&common, "BIGIP6013"));
}

#[test]
fn bigip6013_recognises_registry_owned_factory_profiles_only() {
    crate::install();
    let factory = "ltm virtual /Common/vs {\n  profiles {\n    /Common/http { }\n    /Common/tcp { }\n    /Common/clientssl { context clientside }\n  }\n}\n";
    assert!(!has(factory, "BIGIP6013"));

    let mutated = factory.replace("/Common/clientssl", "/Common/not-a-factory-profile");
    assert!(has(&mutated, "BIGIP6013"));
}

#[test]
fn bigip6014_reports_only_same_kind_and_path_duplicates() {
    crate::install();
    let source = "ltm pool /Tenant/p { }\nltm pool /Tenant/p { }\n\
                      ltm rule /Tenant/p { when RULE_INIT { } }\n";
    let diagnostics: Vec<_> = validate_bigip_source(source, "Tenant")
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "BIGIP6014")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].subject, ConfigDiagnosticSubject::Object);
}

#[test]
fn bigip6038_uses_the_event_profile_graph_for_all_registered_events() {
    crate::install();
    let source = "ltm rule /Common/http_rule {\n\
                        when HTTP_REQUEST { return }\n\
                      }\n\
                      ltm virtual /Common/vs {\n\
                        rules { /Common/http_rule }\n\
                        profiles { /Common/tcp { } }\n\
                      }\n";
    let finding = validate_bigip_source(source, "Common")
        .into_iter()
        .find(|diagnostic| diagnostic.code == "BIGIP6038")
        .expect("the registry requires HTTP for this authored handler");
    assert!(finding.message.contains("HTTP_REQUEST"));
    assert!(finding.message.contains("priority 500"));

    let with_http = format!(
        "ltm profile http /Common/http_profile {{ }}\n{}",
        source.replace(
            "profiles { /Common/tcp { } }",
            "profiles { /Common/tcp { } /Common/http_profile { } }",
        )
    );
    assert!(!has(&with_http, "BIGIP6038"));
}

#[test]
fn bigip6009_fires_for_duplicate_irule_attachment() {
    crate::install();
    let src = "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n    /Common/r\n  }\n}\n";
    assert!(has(src, "BIGIP6009"));
}

#[test]
fn bigip6012_fires_for_implicit_priority_conflict() {
    crate::install();
    let src = "ltm rule /Common/first {\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /Common/second {\n  when HTTP_REQUEST { two }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second }\n}\n";
    let diagnostics = validate_bigip_source(src, "Common");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "BIGIP6012")
        .expect("same implicit priority conflicts");
    assert_eq!(diagnostic.severity, DiagSeverity::Warning);
    assert!(diagnostic.message.contains("HTTP_REQUEST"));
    assert!(diagnostic.message.contains("priority 500"));
    assert!(diagnostic.message.contains("first"));
    assert!(diagnostic.message.contains("second"));
}

#[test]
fn bigip6012_honours_top_level_priority_and_inline_override() {
    crate::install();
    let src = "ltm rule /Common/first {\n  priority 200\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /Common/second {\n  priority 100\n  when CLIENT_ACCEPTED { ignored }\n  priority 200\n  when HTTP_REQUEST { two }\n  when HTTP_RESPONSE priority 300 { three }\n}\n\
                   ltm rule /Common/third {\n  when HTTP_RESPONSE priority 300 { four }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second /Common/third }\n}\n";
    let diagnostics: Vec<_> = validate_bigip_source(src, "Common")
        .into_iter()
        .filter(|diagnostic| diagnostic.code == "BIGIP6012")
        .collect();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].message.contains("HTTP_REQUEST"));
    assert!(diagnostics[0].message.contains("priority 200"));
    assert!(diagnostics[1].message.contains("HTTP_RESPONSE"));
    assert!(diagnostics[1].message.contains("priority 300"));
}

#[test]
fn bigip6012_quiet_for_different_priorities_or_events() {
    crate::install();
    let src = "ltm rule /Common/first {\n  when HTTP_REQUEST priority 100 { one }\n}\n\
                   ltm rule /Common/second {\n  when HTTP_REQUEST priority 200 { two }\n  when HTTP_RESPONSE priority 100 { three }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second }\n}\n";
    assert!(!has(src, "BIGIP6012"));
}

#[test]
fn bigip6012_distinguishes_same_named_rules_in_different_partitions() {
    crate::install();
    let src = "ltm rule /TenantA/shared {\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /TenantB/shared {\n  when HTTP_REQUEST { two }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /TenantA/shared /TenantB/shared }\n}\n";
    let diagnostics = validate_bigip_source(src, "Common");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "BIGIP6012")
        .expect("same-named cross-partition rules remain distinct");
    assert!(diagnostic.message.contains("/TenantA/shared"));
    assert!(diagnostic.message.contains("/TenantB/shared"));
}

#[test]
fn bigip6005_fires_for_virtual_referencing_undefined_pool() {
    crate::install();
    let src = "ltm virtual /Common/vs {\n  destination /Common/1.2.3.4:80\n  pool /Common/no_such_pool\n}\n";
    assert!(has(src, "BIGIP6005"));
}

#[test]
fn bigip6004_fires_for_http_command_without_http_profile() {
    crate::install();
    let src = "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    HTTP::respond 200\n  }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n  }\n}\n";
    assert!(has(src, "BIGIP6004"));
}

#[test]
fn bigip6007_fires_for_missing_snatpool() {
    crate::install();
    let src = "ltm rule /Common/r {\n  when CLIENT_ACCEPTED {\n    snatpool /Common/no_such_snat\n  }\n}\n";
    assert!(has(src, "BIGIP6007"));
}

#[test]
fn bigip6007_skips_variable_snatpool_reference() {
    crate::install();
    // A `$var` / `[cmd]` operand is dynamic — the checker must not flag it.
    let src = "ltm rule /Common/r {\n  when CLIENT_ACCEPTED {\n    snatpool $dynamic_sp\n  }\n}\n";
    assert!(!has(src, "BIGIP6007"));
}

#[test]
fn bigip6006_fires_for_unused_data_group() {
    crate::install();
    let src = "ltm data-group internal /Common/unused {\n  type string\n  records {\n    foo { }\n  }\n}\n";
    assert!(has(src, "BIGIP6006"));
}

#[test]
fn bigip6006_quiet_when_data_group_is_referenced() {
    crate::install();
    let src = "ltm data-group internal /Common/used {\n  type string\n  records {\n    foo { }\n  }\n}\nltm rule /Common/r {\n  when HTTP_REQUEST {\n    if { [class match [HTTP::host] equals /Common/used] } { }\n  }\n}\n";
    assert!(!has(src, "BIGIP6006"));
}

#[test]
fn bigip6013_does_not_treat_monitor_operators_as_references() {
    crate::install();
    let src = r"
ltm monitor http /Common/http { }
ltm monitor tcp /Common/tcp { }
ltm pool /Common/all {
    monitor /Common/http and /Common/tcp
}
ltm pool /Common/quorum {
    monitor min 1 of { /Common/http /Common/tcp }
}
";
    assert!(!has(src, "BIGIP6013"));
}

#[test]
fn signed_scan_data_group_equality_promotes_for_a_configured_unsigned_key() {
    crate::install();
    for (key, expected) in [
        (129, DiagSeverity::Warning),
        (127, DiagSeverity::Hint),
        (300, DiagSeverity::Hint),
    ] {
        let source = format!(
            "ltm data-group internal /Common/int_dg {{ type integer
 records {{ {key} {{ data hit }} }} }}
                 ltm rule /Common/sample {{
 when CLIENT_DATA {{
  binary scan [TCP::payload] c value
  class match -- $value equals int_dg
 }}
}}
"
        );
        let diagnostics = validate_bigip_source(&source, "Common");
        let finding = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "W156")
            .expect("signed data-group item");
        assert_eq!(finding.severity, expected, "{diagnostics:?}");
        assert_eq!(finding.message.contains("Configured key 129"), key == 129);
    }
}
