// @generated — do not edit.
//! Generated strict header parser + typed dispatch tables.

// Generated file: the wide dispatch match (identical/long arms), schema-derived
// doc text, and glob re-exports are inherent to codegen and not hand-fixable
// without editing the generator.
#![allow(clippy::match_same_arms, clippy::too_many_lines)]
#![allow(clippy::doc_markdown, clippy::wildcard_imports)]

use super::ModelObject;
use super::parsers::*;
use crate::model::BigipMinimalObject;
use crate::parser::scalar::{description, name_leaf, props_map};
use crate::range::Range;

const TWO_WORD_TYPES: &[&str] = &[
    "aaa active-directory",
    "aaa active-directory-trusted-domains",
    "aaa crldp",
    "aaa endpoint-management-system",
    "aaa f5-mfa-configuration",
    "aaa f5-service-connector",
    "aaa http",
    "aaa http-connector-request",
    "aaa http-connector-transport",
    "aaa kerberos",
    "aaa kerberos-keytab-file",
    "aaa ldap",
    "aaa localdb",
    "aaa oam",
    "aaa oauth-provider",
    "aaa oauth-request",
    "aaa oauth-server",
    "aaa ocsp",
    "aaa okta-connector",
    "aaa radius",
    "aaa saml",
    "aaa saml-idp-automation",
    "aaa saml-idp-connector",
    "aaa securid",
    "aaa tacacsplus",
    "alert lcd",
    "alias private",
    "alias shared",
    "analytics settings",
    "anti-fraud profile",
    "anti-fraud signatures-update",
    "appiq config",
    "application apl-script",
    "application custom-stat",
    "application service",
    "application template",
    "auth crldp-server",
    "auth kerberos-delegation",
    "auth ldap",
    "auth ocsp-responder",
    "auth profile",
    "auth radius",
    "auth radius-server",
    "auth ssl-cc-ldap",
    "auth ssl-crldp",
    "auth ssl-ocsp",
    "auth tacacs",
    "blacklist-publisher category",
    "blacklist-publisher profile",
    "bot-defense profile",
    "bot-defense signature",
    "bot-defense signature-category",
    "bwc policy",
    "bwc priority-group",
    "bwc traffic-group",
    "cipher group",
    "cipher rule",
    "classification application",
    "classification category",
    "classification ce",
    "classification signature-update-schedule",
    "classification url-cat-policy",
    "classification url-category",
    "classification urldb-feed-list",
    "classification urldb-file",
    "client image",
    "clientssl ocsp-stapling-responses",
    "clientssl-proxy cached-certs",
    "cloud-services connector",
    "configuration captcha",
    "cos global-settings",
    "cos map-8021p",
    "cos map-dscp",
    "cos traffic-priority",
    "crypto acceleration-strategy",
    "crypto ca-bundle-manager",
    "crypto cert",
    "crypto cert-order-manager",
    "crypto client",
    "crypto crl",
    "crypto csr",
    "crypto key",
    "crypto master-key",
    "crypto server",
    "daemon-log-settings clusterd",
    "daemon-log-settings csyncd",
    "daemon-log-settings icr-eventd",
    "daemon-log-settings icrd",
    "daemon-log-settings lind",
    "daemon-log-settings mcpd",
    "daemon-log-settings tmm",
    "data-group external",
    "data-group internal",
    "datasync background-tasks",
    "datasync global-profile",
    "datasync local-profile",
    "debug drop-redirect-stats",
    "debug matcher",
    "debug register",
    "device device-context",
    "device-id attribute",
    "diags ihealth",
    "dns nameserver",
    "dns tsig-key",
    "dns zone",
    "dos autodos-file-object",
    "dos behavioral-signature",
    "dos bot-signature",
    "dos bot-signature-category",
    "dos device-config",
    "dos dns-nxdomain-stat",
    "dos dos-signature",
    "dos dynamic-signatures",
    "dos ip-uncommon-protolist",
    "dos ipv6-ext-hdr",
    "dos l4bdos-file-object",
    "dos network-whitelist",
    "dos profile",
    "dos profile-signatures",
    "dos stress-stats",
    "dos udp-portlist",
    "dos virtual",
    "dynad settings",
    "ecm cloud-provider",
    "ephemeral-auth ssh-security-config",
    "epsec epsec-package",
    "fdb tunnel",
    "fdb vlan",
    "file apache-ssl-cert",
    "file browser-capabilities-db",
    "file data-group",
    "file device-capabilities-db",
    "file external-monitor",
    "file ifile",
    "file lwtunneltbl",
    "file rewrite-rule",
    "file ssl-cert",
    "file ssl-crl",
    "file ssl-key",
    "firewall address-list",
    "firewall config-change-log",
    "firewall config-entity-id",
    "firewall global-fqdn-policy",
    "firewall global-rules",
    "firewall management-ip-rules",
    "firewall on-demand-compilation",
    "firewall on-demand-rule-deploy",
    "firewall policy",
    "firewall port-list",
    "firewall port-misuse-policy",
    "firewall rule-list",
    "firewall schedule",
    "firewall user-domain",
    "firewall user-list",
    "firewall uuid-default-autogenerate",
    "flowspec-route-injector profile",
    "fpga firmware-config",
    "global-settings analytics",
    "global-settings connection",
    "global-settings general",
    "global-settings gx",
    "global-settings hsl-flow",
    "global-settings hsl-report",
    "global-settings insert-content",
    "global-settings load-balancing",
    "global-settings metrics",
    "global-settings metrics-exclusions",
    "global-settings policy",
    "global-settings quota-mgmt",
    "global-settings rule",
    "global-settings session-mgmt-attributes",
    "global-settings subscriber-activity-log",
    "global-settings traffic-control",
    "html-rule comment-raise-event",
    "html-rule comment-remove",
    "html-rule tag-append-html",
    "html-rule tag-prepend-html",
    "html-rule tag-raise-event",
    "html-rule tag-remove",
    "html-rule tag-remove-attribute",
    "http profile",
    "icall istats-trigger",
    "icall script",
    "ip-intelligence blacklist-category",
    "ip-intelligence feed-list",
    "ip-intelligence global-policy",
    "ip-intelligence policy",
    "ipfix destination",
    "ipfix element",
    "ipfix irules",
    "ipsec ike-daemon",
    "ipsec ike-peer",
    "ipsec ipsec-policy",
    "ipsec manual-security-association",
    "ipsec traffic-selector",
    "log profile",
    "log-config filter",
    "log-config publisher",
    "monitor bigip",
    "monitor bigip-link",
    "monitor diameter",
    "monitor dns",
    "monitor external",
    "monitor firepass",
    "monitor ftp",
    "monitor gateway-icmp",
    "monitor gtp",
    "monitor http",
    "monitor https",
    "monitor icmp",
    "monitor imap",
    "monitor inband",
    "monitor ldap",
    "monitor module-score",
    "monitor mqtt",
    "monitor mssql",
    "monitor mysql",
    "monitor nntp",
    "monitor oracle",
    "monitor pop3",
    "monitor postgresql",
    "monitor radius",
    "monitor radius-accounting",
    "monitor real-server",
    "monitor rpc",
    "monitor sasp",
    "monitor scripted",
    "monitor sip",
    "monitor smb",
    "monitor smtp",
    "monitor snmp",
    "monitor snmp-dca",
    "monitor snmp-dca-base",
    "monitor snmp-link",
    "monitor soap",
    "monitor tcp",
    "monitor tcp-echo",
    "monitor tcp-half-open",
    "monitor udp",
    "monitor virtual-location",
    "monitor wap",
    "monitor wmi",
    "nat destination-translation",
    "nat policy",
    "nat source-translation",
    "ntlm machine-account",
    "ntlm ntlm-auth",
    "oauth db-instance",
    "oauth jwk-config",
    "oauth jwt-config",
    "oauth jwt-provider-list",
    "oauth oauth-claim",
    "oauth oauth-client-app",
    "oauth oauth-resource-server",
    "oauth oauth-scope",
    "packet-filter default-rules",
    "packet-filter policy",
    "persistence cookie",
    "persistence dest-addr",
    "persistence hash",
    "persistence mcp",
    "persistence msrdp",
    "persistence sip",
    "persistence source-addr",
    "persistence ssl",
    "persistence universal",
    "policy access-policy",
    "policy customization-group",
    "policy customization-languages",
    "policy customization-source",
    "policy image-file",
    "policy policy-item",
    "policy windows-group-policy-file",
    "pool a",
    "pool aaaa",
    "pool cname",
    "pool mx",
    "pool naptr",
    "pool srv",
    "profile access",
    "profile aimcp",
    "profile analytics",
    "profile apiprotection",
    "profile classification",
    "profile client-ssl",
    "profile connectivity",
    "profile diameter",
    "profile diameter-endpoint",
    "profile dns",
    "profile exchange",
    "profile fasthttp",
    "profile fastl4",
    "profile fix",
    "profile ftp",
    "profile html",
    "profile http",
    "profile http-compression",
    "profile http-proxy-connect",
    "profile http2",
    "profile ipother",
    "profile json",
    "profile mqtt",
    "profile oauth",
    "profile one-connect",
    "profile radius",
    "profile radius-aaa",
    "profile request-log",
    "profile rewrite",
    "profile server-ssl",
    "profile sip",
    "profile sse",
    "profile spm",
    "profile stream",
    "profile subscriber-mgmt",
    "profile tcp",
    "profile tcp-analytics",
    "profile udp",
    "profile vdi",
    "profile web-acceleration",
    "profile websocket",
    "protected zone",
    "protocol diameter-avp",
    "protocol radius-avp",
    "protocol-inspection common-config",
    "protocol-inspection compliance-map",
    "protocol-inspection compliance-objects",
    "protocol-inspection learning-stats",
    "protocol-inspection profile",
    "protocol-inspection signature",
    "quota-mgmt rating-group",
    "rate-shaping class",
    "rate-shaping color-policer",
    "rate-shaping drop-policy",
    "rate-shaping queue",
    "rate-shaping shaping-policy",
    "report custom-report-field",
    "report default-report",
    "reporting format-script",
    "resource address-space",
    "resource app-tunnel",
    "resource client-rate-class",
    "resource client-traffic-classifier",
    "resource ipv6-leasepool",
    "resource leasepool",
    "resource network-access",
    "resource portal-access",
    "resource sandbox",
    "resource webtop",
    "resource webtop-link",
    "routing access-list",
    "routing as-path",
    "routing bfd",
    "routing bgp",
    "routing community-list",
    "routing debug",
    "routing extcommunity-list",
    "routing prefix-list",
    "routing route-map",
    "saml artifact-resolution-service",
    "saml attribute-consuming-service",
    "saml auth-context-class-list",
    "scrubber profile",
    "sfc chain",
    "sfc sf",
    "sflow receiver",
    "shared-objects address-list",
    "shared-objects port-list",
    "software hotfix",
    "software image",
    "software signature",
    "software update",
    "software volume",
    "ssh ciphers",
    "ssh profile",
    "sso basic",
    "sso form-based",
    "sso form-basedv2",
    "sso kerberos",
    "sso ntlmv1",
    "sso ntlmv2",
    "sso oauth-bearer",
    "sso saml",
    "sso saml-resource",
    "sso saml-sp-automation",
    "sso saml-sp-connector",
    "tacdb customdb",
    "tacdb customdb-file",
    "tacdb licenseddb",
    "tunnels endpoint",
    "tunnels etherip",
    "tunnels fec",
    "tunnels geneve",
    "tunnels gre",
    "tunnels ipip",
    "tunnels ipsec",
    "tunnels lw4o6",
    "tunnels map",
    "tunnels ppp",
    "tunnels tcp-forward",
    "tunnels tunnel",
    "tunnels v6rd",
    "tunnels vxlan",
    "tunnels wccp",
    "turboflex profile-config",
    "url-db download-schedule",
    "url-db url-category",
    "wideip a",
    "wideip aaaa",
    "wideip cname",
    "wideip mx",
    "wideip naptr",
    "wideip srv",
];
const THREE_WORD_TYPES: &[&str] = &[
    "classification auto-update settings",
    "crypto cert-validation-response ocsp",
    "crypto cert-validator crl",
    "crypto cert-validator ocsp",
    "crypto fips external-hsm",
    "crypto fips key",
    "dns analytics global-settings",
    "dns cache global-settings",
    "dns cache resolver",
    "dns cache transparent",
    "dns cache validating-resolver",
    "dns dnssec key",
    "dns dnssec zone",
    "dns hpke key",
    "dns hpke profile",
    "icall handler periodic",
    "icall handler perpetual",
    "icall handler triggered",
    "log-config destination alertd",
    "log-config destination arcsight",
    "log-config destination ipfix",
    "log-config destination local-database",
    "log-config destination local-syslog",
    "log-config destination management-port",
    "log-config destination remote-high-speed-log",
    "log-config destination remote-syslog",
    "log-config destination splunk",
    "message-routing diameter peer",
    "message-routing diameter route",
    "message-routing diameter transport-config",
    "message-routing generic peer",
    "message-routing generic protocol",
    "message-routing generic route",
    "message-routing generic router",
    "message-routing generic transport-config",
    "message-routing mqtt peer",
    "message-routing mqtt route",
    "message-routing mqtt transport-config",
    "message-routing sip peer",
    "message-routing sip route",
    "message-routing sip transport-config",
    "policy agent aaa-active-directory",
    "policy agent aaa-client-cert",
    "policy agent aaa-crldp",
    "policy agent aaa-http",
    "policy agent aaa-ldap",
    "policy agent aaa-oauth",
    "policy agent aaa-ocsp",
    "policy agent aaa-radius",
    "policy agent aaa-saml",
    "policy agent aaa-securid",
    "policy agent acct-radius",
    "policy agent acct-tacacsplus",
    "policy agent api-authentication",
    "policy agent api-server-selection",
    "policy agent decision-box",
    "policy agent dynamic-acl",
    "policy agent ending-allow",
    "policy agent ending-deny",
    "policy agent ending-redirect",
    "policy agent endpoint-check-machine-cert",
    "policy agent endpoint-check-software",
    "policy agent endpoint-linux-check-file",
    "policy agent endpoint-linux-check-process",
    "policy agent endpoint-mac-check-file",
    "policy agent endpoint-mac-check-process",
    "policy agent endpoint-machine-info",
    "policy agent endpoint-windows-browser-cache-cleaner",
    "policy agent endpoint-windows-check-file",
    "policy agent endpoint-windows-check-process",
    "policy agent endpoint-windows-check-registry",
    "policy agent endpoint-windows-group-policy",
    "policy agent endpoint-windows-info-os",
    "policy agent endpoint-windows-protected-workspace",
    "policy agent external-logon-page",
    "policy agent http-header-modify",
    "policy agent ip-geolocation-lookup",
    "policy agent ip-reputation-lookup",
    "policy agent irule-event",
    "policy agent kerberos",
    "policy agent l7-protocol-lookup",
    "policy agent logging",
    "policy agent logon-page",
    "policy agent message-box",
    "policy agent oam",
    "policy agent oauth-authz",
    "policy agent request-classification",
    "policy agent resource-assign",
    "policy agent response-selection",
    "policy agent route-domain-selection",
    "policy agent server-cert-response-control",
    "policy agent server-cert-status",
    "policy agent session-check",
    "policy agent ssl-check",
    "policy agent tacacsplus",
    "policy agent variable-assign",
    "protocol profile gx",
    "protocol profile radius",
    "resource remote-desktop citrix",
    "resource remote-desktop citrix-client-bundle",
    "resource remote-desktop citrix-client-package-file",
    "resource remote-desktop quest",
    "resource remote-desktop rdp",
    "resource remote-desktop vmware-view",
    "routing profile bgp",
    "sflow global-settings http",
    "sflow global-settings interface",
    "sflow global-settings system",
    "sflow global-settings vlan",
];
const FOUR_WORD_TYPES: &[&str] = &[
    "dns cache records all",
    "dns cache records key",
    "dns cache records msg",
    "dns cache records nameserver",
    "dns cache records rrset",
    "message-routing diameter profile router",
    "message-routing diameter profile session",
    "message-routing mqtt profile router",
    "message-routing mqtt profile session",
    "message-routing sip profile router",
    "message-routing sip profile session",
];

/// Strict header parse -> `(module, object_type, full_path)`.
#[must_use]
pub fn parse_header_strict(header: &str) -> Option<(String, String, String)> {
    // Quote-aware tokenisation (like the generic path): a quoted identifier
    // with spaces — `security bot-defense signature "/Common/Microsoft Access"`
    // — is one token, not truncated at the inner space (issue 188).
    let parts = crate::parser::helpers::tokenise_header(header);
    if parts.len() < 3 {
        return None;
    }
    let module = parts[0].clone();
    if parts.len() >= 6 {
        let four = parts[1..5].join(" ");
        if FOUR_WORD_TYPES.contains(&four.as_str()) {
            return Some((module, four, parts[5].clone()));
        }
    }
    if parts.len() >= 5 {
        let three = parts[1..4].join(" ");
        if THREE_WORD_TYPES.contains(&three.as_str()) {
            return Some((module, three, parts[4].clone()));
        }
    }
    if parts.len() == 4 {
        let three = parts[1..4].join(" ");
        if THREE_WORD_TYPES.contains(&three.as_str()) {
            return Some((module, three, String::new()));
        }
    }
    if parts.len() == 3 {
        let two = format!("{} {}", parts[1], parts[2]);
        if TWO_WORD_TYPES.contains(&two.as_str()) {
            return Some((module, two, String::new()));
        }
    }
    if parts.len() >= 4 {
        let two = format!("{} {}", parts[1], parts[2]);
        if TWO_WORD_TYPES.contains(&two.as_str()) {
            return Some((module, two, parts[3].clone()));
        }
    }
    Some((module, parts[1].clone(), parts[2].clone()))
}

/// Build a minimal object with its TMSH kind label.
#[must_use]
pub fn make_minimal(full_path: &str, body: &str, kind: &str, range: Range) -> BigipMinimalObject {
    let props = props_map(body);
    BigipMinimalObject {
        name: name_leaf(full_path),
        full_path: full_path.to_owned(),
        kind: kind.to_owned(),
        description: description(&props),
        range: Some(range),
    }
}

/// Generated dispatch for dispatch_named.
#[must_use]
pub fn dispatch_named(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<(&'static str, ModelObject)> {
    Some(match (module, object_type) {
        ("apm", "ephemeral-auth ssh-security-config") => (
            "apm_ephemeral_auth_ssh_security_configs",
            ModelObject::ApmEphemeralAuthSshSecurityConfig(
                parse_bigip_apm_ephemeral_auth_ssh_security_config(full_path, body, range),
            ),
        ),
        ("apm", "oauth db-instance") => (
            "apm_oauth_db_instances",
            ModelObject::ApmOauthDbInstance(parse_bigip_apm_oauth_db_instance(
                full_path, body, range,
            )),
        ),
        ("apm", "policy access-policy") => (
            "apm_policy_access_policies",
            ModelObject::ApmPolicyAccessPolicy(parse_bigip_apm_policy_access_policy(
                full_path, body, range,
            )),
        ),
        ("apm", "policy customization-source") => (
            "apm_policy_customization_sources",
            ModelObject::ApmPolicyCustomizationSource(parse_bigip_apm_policy_customization_source(
                full_path, body, range,
            )),
        ),
        ("apm", "policy policy-item") => (
            "apm_policy_items",
            ModelObject::ApmPolicyItem(parse_bigip_apm_policy_item(full_path, body, range)),
        ),
        ("auth", "apm-auth") => (
            "auth_apm_auths",
            ModelObject::AuthApmAuth(parse_bigip_auth_apm_auth(full_path, body, range)),
        ),
        ("auth", "cert-ldap") => (
            "auth_cert_ldaps",
            ModelObject::AuthCertLdap(parse_bigip_auth_cert_ldap(full_path, body, range)),
        ),
        ("auth", "ldap") => (
            "auth_ldaps",
            ModelObject::AuthLdap(parse_bigip_auth_ldap(full_path, body, range)),
        ),
        ("auth", "partition") => (
            "auth_partitions",
            ModelObject::AuthPartition(parse_bigip_auth_partition(full_path, body, range)),
        ),
        ("auth", "radius") => (
            "auth_radius",
            ModelObject::AuthRadius(parse_bigip_auth_radius(full_path, body, range)),
        ),
        ("auth", "radius-server") => (
            "auth_radius_servers",
            ModelObject::AuthRadiusServer(parse_bigip_auth_radius_server(full_path, body, range)),
        ),
        ("auth", "tacacs") => (
            "auth_tacacs",
            ModelObject::AuthTacacs(parse_bigip_auth_tacacs(full_path, body, range)),
        ),
        ("auth", "user") => (
            "auth_users",
            ModelObject::AuthUser(parse_bigip_auth_user(full_path, body, range)),
        ),
        ("cm", "cert") => (
            "cm_certs",
            ModelObject::CmCert(parse_bigip_cm_cert(full_path, body, range)),
        ),
        ("cm", "device") => (
            "cm_devices",
            ModelObject::CmDevice(parse_bigip_cm_device(full_path, body, range)),
        ),
        ("cm", "device-group") => (
            "cm_device_groups",
            ModelObject::CmDeviceGroup(parse_bigip_cm_device_group(full_path, body, range)),
        ),
        ("cm", "ha-group") => (
            "cm_ha_groups",
            ModelObject::CmHaGroup(parse_bigip_cm_ha_group(full_path, body, range)),
        ),
        ("cm", "key") => (
            "cm_keys",
            ModelObject::CmKey(parse_bigip_cm_key(full_path, body, range)),
        ),
        ("cm", "traffic-group") => (
            "cm_traffic_groups",
            ModelObject::CmTrafficGroup(parse_bigip_cm_traffic_group(full_path, body, range)),
        ),
        ("cm", "trust-domain") => (
            "cm_trust_domains",
            ModelObject::CmTrustDomain(parse_bigip_cm_trust_domain(full_path, body, range)),
        ),
        ("gtm", "datacenter") => (
            "gtm_datacenters",
            ModelObject::GtmDatacenter(parse_bigip_gtm_datacenter(full_path, body, range)),
        ),
        ("gtm", "distributed-app") => (
            "gtm_distributed_apps",
            ModelObject::GtmDistributedApp(parse_bigip_gtm_distributed_app(full_path, body, range)),
        ),
        ("gtm", "link") => (
            "gtm_links",
            ModelObject::GtmLink(parse_bigip_gtm_link(full_path, body, range)),
        ),
        ("gtm", "listener") => (
            "gtm_listeners",
            ModelObject::GtmListener(parse_bigip_gtm_listener(full_path, body, range)),
        ),
        ("gtm", "listener-doh-proxy") => (
            "gtm_listener_doh_proxies",
            ModelObject::GtmListenerDohProxy(parse_bigip_gtm_listener_doh_proxy(
                full_path, body, range,
            )),
        ),
        ("gtm", "listener-doh-server") => (
            "gtm_listener_doh_servers",
            ModelObject::GtmListenerDohServer(parse_bigip_gtm_listener_doh_server(
                full_path, body, range,
            )),
        ),
        ("gtm", "prober-pool") => (
            "gtm_prober_pools",
            ModelObject::GtmProberPool(parse_bigip_gtm_prober_pool(full_path, body, range)),
        ),
        ("gtm", "region") => (
            "gtm_regions",
            ModelObject::GtmRegion(parse_bigip_gtm_region(full_path, body, range)),
        ),
        ("gtm", "rule") => (
            "gtm_rules",
            ModelObject::GtmRule(parse_bigip_gtm_rule(full_path, body, range)),
        ),
        ("gtm", "server") => (
            "gtm_servers",
            ModelObject::GtmServer(parse_bigip_gtm_server(full_path, body, range)),
        ),
        ("ltm", "cipher group") => (
            "ltm_cipher_groups",
            ModelObject::LtmCipherGroup(parse_bigip_ltm_cipher_group(full_path, body, range)),
        ),
        ("ltm", "cipher rule") => (
            "ltm_cipher_rules",
            ModelObject::LtmCipherRule(parse_bigip_ltm_cipher_rule(full_path, body, range)),
        ),
        ("ltm", "dns cache resolver") => (
            "ltm_dns_cache_resolvers",
            ModelObject::LtmDnsCacheResolver(parse_bigip_ltm_dns_cache_resolver(
                full_path, body, range,
            )),
        ),
        ("ltm", "dns cache transparent") => (
            "ltm_dns_cache_transparent",
            ModelObject::LtmDnsCacheTransparent(parse_bigip_ltm_dns_cache_transparent(
                full_path, body, range,
            )),
        ),
        ("ltm", "dns cache validating-resolver") => (
            "ltm_dns_cache_validating_resolvers",
            ModelObject::LtmDnsCacheValidatingResolver(
                parse_bigip_ltm_dns_cache_validating_resolver(full_path, body, range),
            ),
        ),
        ("ltm", "dns dnssec key") => (
            "ltm_dns_dnssec_keys",
            ModelObject::LtmDnsDnssecKey(parse_bigip_ltm_dns_dnssec_key(full_path, body, range)),
        ),
        ("ltm", "dns dnssec zone") => (
            "ltm_dns_dnssec_zones",
            ModelObject::LtmDnsDnssecZone(parse_bigip_ltm_dns_dnssec_zone(full_path, body, range)),
        ),
        ("ltm", "dns hpke key") => (
            "ltm_dns_hpke_keys",
            ModelObject::LtmDnsHpkeKey(parse_bigip_ltm_dns_hpke_key(full_path, body, range)),
        ),
        ("ltm", "dns hpke profile") => (
            "ltm_dns_hpke_profiles",
            ModelObject::LtmDnsHpkeProfile(parse_bigip_ltm_dns_hpke_profile(
                full_path, body, range,
            )),
        ),
        ("ltm", "dns nameserver") => (
            "ltm_dns_nameservers",
            ModelObject::LtmDnsNameserver(parse_bigip_ltm_dns_nameserver(full_path, body, range)),
        ),
        ("ltm", "dns tsig-key") => (
            "ltm_dns_tsig_keys",
            ModelObject::LtmDnsTsigKey(parse_bigip_ltm_dns_tsig_key(full_path, body, range)),
        ),
        ("ltm", "dns zone") => (
            "ltm_dns_zones",
            ModelObject::LtmDnsZone(parse_bigip_ltm_dns_zone(full_path, body, range)),
        ),
        ("ltm", "eviction-policy") => (
            "ltm_eviction_policies",
            ModelObject::LtmEvictionPolicy(parse_bigip_ltm_eviction_policy(full_path, body, range)),
        ),
        ("ltm", "ifile") => (
            "ltm_ifiles",
            ModelObject::LtmIfile(parse_bigip_ltm_ifile(full_path, body, range)),
        ),
        ("ltm", "nat") => (
            "ltm_nats",
            ModelObject::LtmNat(parse_bigip_ltm_nat(full_path, body, range)),
        ),
        ("ltm", "policy-strategy") => (
            "ltm_policy_strategies",
            ModelObject::LtmPolicyStrategy(parse_bigip_ltm_policy_strategy(full_path, body, range)),
        ),
        ("ltm", "rate-class") => (
            "ltm_rate_classes",
            ModelObject::LtmRateClass(parse_bigip_ltm_rate_class(full_path, body, range)),
        ),
        ("ltm", "snat") => (
            "ltm_snats",
            ModelObject::LtmSnat(parse_bigip_ltm_snat(full_path, body, range)),
        ),
        ("ltm", "snat-translation") => (
            "ltm_snat_translations",
            ModelObject::LtmSnatTranslation(parse_bigip_ltm_snat_translation(
                full_path, body, range,
            )),
        ),
        ("ltm", "traffic-class") => (
            "ltm_traffic_classes",
            ModelObject::LtmTrafficClass(parse_bigip_ltm_traffic_class(full_path, body, range)),
        ),
        ("ltm", "traffic-matching-criteria") => (
            "ltm_traffic_matching_criteria",
            ModelObject::LtmTrafficMatchingCriteria(parse_bigip_ltm_traffic_matching_criteria(
                full_path, body, range,
            )),
        ),
        ("net", "dns-resolver") => (
            "net_dns_resolvers",
            ModelObject::NetDnsResolver(parse_bigip_net_dns_resolver(full_path, body, range)),
        ),
        ("net", "interface") => (
            "net_interfaces",
            ModelObject::NetInterface(parse_bigip_net_interface(full_path, body, range)),
        ),
        ("net", "port-list") => (
            "net_port_lists",
            ModelObject::NetPortList(parse_bigip_net_port_list(full_path, body, range)),
        ),
        ("net", "route") => (
            "net_routes",
            ModelObject::NetRoute(parse_bigip_net_route(full_path, body, range)),
        ),
        ("net", "route-domain") => (
            "net_route_domains",
            ModelObject::NetRouteDomain(parse_bigip_net_route_domain(full_path, body, range)),
        ),
        ("net", "self") => (
            "net_selves",
            ModelObject::NetSelf(parse_bigip_net_self(full_path, body, range)),
        ),
        ("net", "stp") => (
            "net_stps",
            ModelObject::NetStp(parse_bigip_net_stp(full_path, body, range)),
        ),
        ("net", "tunnels tunnel") => (
            "net_tunnels",
            ModelObject::NetTunnel(parse_bigip_net_tunnel(full_path, body, range)),
        ),
        ("net", "vlan") => (
            "net_vlans",
            ModelObject::NetVlan(parse_bigip_net_vlan(full_path, body, range)),
        ),
        ("pem", "forwarding-endpoint") => (
            "pem_forwarding_endpoints",
            ModelObject::PemForwardingEndpoint(parse_bigip_pem_forwarding_endpoint(
                full_path, body, range,
            )),
        ),
        ("pem", "interception-endpoint") => (
            "pem_interception_endpoints",
            ModelObject::PemInterceptionEndpoint(parse_bigip_pem_interception_endpoint(
                full_path, body, range,
            )),
        ),
        ("pem", "irule") => (
            "pem_rules",
            ModelObject::PemRule(parse_bigip_pem_rule(full_path, body, range)),
        ),
        ("pem", "listener") => (
            "pem_listeners",
            ModelObject::PemListener(parse_bigip_pem_listener(full_path, body, range)),
        ),
        ("pem", "policy") => (
            "pem_policies",
            ModelObject::PemPolicy(parse_bigip_pem_policy(full_path, body, range)),
        ),
        ("pem", "quota-mgmt rating-group") => (
            "pem_rating_groups",
            ModelObject::PemRatingGroup(parse_bigip_pem_rating_group(full_path, body, range)),
        ),
        ("pem", "service-chain-endpoint") => (
            "pem_service_chain_endpoints",
            ModelObject::PemServiceChainEndpoint(parse_bigip_pem_service_chain_endpoint(
                full_path, body, range,
            )),
        ),
        ("security", "bot-defense profile") => (
            "security_bot_defense_profiles",
            ModelObject::SecurityBotDefenseProfile(parse_bigip_security_bot_defense_profile(
                full_path, body, range,
            )),
        ),
        ("security", "device-id attribute") => (
            "security_device_id_attributes",
            ModelObject::SecurityDeviceIdAttribute(parse_bigip_security_device_id_attribute(
                full_path, body, range,
            )),
        ),
        ("security", "dos profile") => (
            "security_dos_profiles",
            ModelObject::SecurityDosProfile(parse_bigip_security_dos_profile(
                full_path, body, range,
            )),
        ),
        ("security", "firewall address-list") => (
            "security_firewall_address_lists",
            ModelObject::SecurityFirewallAddressList(parse_bigip_security_firewall_address_list(
                full_path, body, range,
            )),
        ),
        ("security", "firewall config-entity-id") => (
            "security_firewall_config_entity_ids",
            ModelObject::SecurityFirewallConfigEntityId(
                parse_bigip_security_firewall_config_entity_id(full_path, body, range),
            ),
        ),
        ("security", "firewall policy") => (
            "security_firewall_policies",
            ModelObject::SecurityFirewallPolicy(parse_bigip_security_firewall_policy(
                full_path, body, range,
            )),
        ),
        ("security", "firewall port-list") => (
            "security_firewall_port_lists",
            ModelObject::SecurityFirewallPortList(parse_bigip_security_firewall_port_list(
                full_path, body, range,
            )),
        ),
        ("security", "firewall port-misuse-policy") => (
            "security_firewall_port_misuse_policies",
            ModelObject::SecurityFirewallPortMisusePolicy(
                parse_bigip_security_firewall_port_misuse_policy(full_path, body, range),
            ),
        ),
        ("security", "firewall rule-list") => (
            "security_firewall_rule_lists",
            ModelObject::SecurityFirewallRuleList(parse_bigip_security_firewall_rule_list(
                full_path, body, range,
            )),
        ),
        ("security", "firewall schedule") => (
            "security_firewall_schedules",
            ModelObject::SecurityFirewallSchedule(parse_bigip_security_firewall_schedule(
                full_path, body, range,
            )),
        ),
        ("security", "firewall user-domain") => (
            "security_firewall_user_domains",
            ModelObject::SecurityFirewallUserDomain(parse_bigip_security_firewall_user_domain(
                full_path, body, range,
            )),
        ),
        ("security", "firewall user-list") => (
            "security_firewall_user_lists",
            ModelObject::SecurityFirewallUserList(parse_bigip_security_firewall_user_list(
                full_path, body, range,
            )),
        ),
        ("security", "http profile") => (
            "security_http_profiles",
            ModelObject::SecurityHttpProfile(parse_bigip_security_http_profile(
                full_path, body, range,
            )),
        ),
        ("security", "ip-intelligence feed-list") => (
            "security_ip_intelligence_feed_lists",
            ModelObject::SecurityIpIntelligenceFeedList(
                parse_bigip_security_ip_intelligence_feed_list(full_path, body, range),
            ),
        ),
        ("security", "ip-intelligence policy") => (
            "security_ip_intelligence_policies",
            ModelObject::SecurityIpIntelligencePolicy(parse_bigip_security_ip_intelligence_policy(
                full_path, body, range,
            )),
        ),
        ("security", "log profile") => (
            "security_log_profiles",
            ModelObject::SecurityLogProfile(parse_bigip_security_log_profile(
                full_path, body, range,
            )),
        ),
        ("security", "nat destination-translation") => (
            "security_nat_destination_translations",
            ModelObject::SecurityNatDestinationTranslation(
                parse_bigip_security_nat_destination_translation(full_path, body, range),
            ),
        ),
        ("security", "nat policy") => (
            "security_nat_policies",
            ModelObject::SecurityNatPolicy(parse_bigip_security_nat_policy(full_path, body, range)),
        ),
        ("security", "nat source-translation") => (
            "security_nat_source_translations",
            ModelObject::SecurityNatSourceTranslation(parse_bigip_security_nat_source_translation(
                full_path, body, range,
            )),
        ),
        ("security", "packet-filter policy") => (
            "security_packet_filter_policies",
            ModelObject::SecurityPacketFilterPolicy(parse_bigip_security_packet_filter_policy(
                full_path, body, range,
            )),
        ),
        ("security", "protected zone") => (
            "security_protected_zones",
            ModelObject::SecurityProtectedZone(parse_bigip_security_protected_zone(
                full_path, body, range,
            )),
        ),
        ("security", "protocol-inspection compliance-map") => (
            "security_pi_compliance_maps",
            ModelObject::SecurityProtocolInspectionComplianceMap(
                parse_bigip_security_protocol_inspection_compliance_map(full_path, body, range),
            ),
        ),
        ("security", "protocol-inspection compliance-objects") => (
            "security_pi_compliance_objects",
            ModelObject::SecurityProtocolInspectionComplianceObject(
                parse_bigip_security_protocol_inspection_compliance_object(full_path, body, range),
            ),
        ),
        ("security", "ssh profile") => (
            "security_ssh_profiles",
            ModelObject::SecuritySshProfile(parse_bigip_security_ssh_profile(
                full_path, body, range,
            )),
        ),
        ("security", "zone") => (
            "security_zones",
            ModelObject::SecurityZone(parse_bigip_security_zone(full_path, body, range)),
        ),
        ("sys", "file ssl-cert") => (
            "sys_file_ssl_certs",
            ModelObject::SysFileSslCert(parse_bigip_sys_file_ssl_cert(full_path, body, range)),
        ),
        ("sys", "file ssl-key") => (
            "sys_file_ssl_keys",
            ModelObject::SysFileSslKey(parse_bigip_sys_file_ssl_key(full_path, body, range)),
        ),
        ("sys", "folder") => (
            "sys_folders",
            ModelObject::SysFolder(parse_bigip_sys_folder(full_path, body, range)),
        ),
        ("sys", "management-route") => (
            "sys_management_routes",
            ModelObject::SysManagementRoute(parse_bigip_sys_management_route(
                full_path, body, range,
            )),
        ),
        ("sys", "provision") => (
            "sys_provisions",
            ModelObject::SysProvision(parse_bigip_sys_provision(full_path, body, range)),
        ),
        _ => return None,
    })
}

/// Generated dispatch for dispatch_singleton.
#[must_use]
pub fn dispatch_singleton(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<(&'static str, ModelObject)> {
    Some(match (module, object_type) {
        ("apm", "report default-report") => (
            "apm_report_default_report",
            ModelObject::ApmReportDefaultReport(parse_bigip_apm_report_default_report(
                full_path, body, range,
            )),
        ),
        ("auth", "login-failures") => (
            "auth_login_failures",
            ModelObject::AuthLoginFailures(parse_bigip_auth_login_failures(full_path, body, range)),
        ),
        ("auth", "password") => (
            "auth_password",
            ModelObject::AuthPassword(parse_bigip_auth_password(full_path, body, range)),
        ),
        ("auth", "password-policy") => (
            "auth_password_policy",
            ModelObject::AuthPasswordPolicy(parse_bigip_auth_password_policy(
                full_path, body, range,
            )),
        ),
        ("auth", "remote-role") => (
            "auth_remote_role",
            ModelObject::AuthRemoteRole(parse_bigip_auth_remote_role(full_path, body, range)),
        ),
        ("auth", "remote-user") => (
            "auth_remote_user",
            ModelObject::AuthRemoteUser(parse_bigip_auth_remote_user(full_path, body, range)),
        ),
        ("auth", "source") => (
            "auth_source",
            ModelObject::AuthSource(parse_bigip_auth_source(full_path, body, range)),
        ),
        ("gtm", "global-settings general") => (
            "gtm_global_settings_general",
            ModelObject::GtmGlobalSettingsGeneral(parse_bigip_gtm_global_settings_general(
                full_path, body, range,
            )),
        ),
        ("gtm", "global-settings load-balancing") => (
            "gtm_global_settings_load_balancing",
            ModelObject::GtmGlobalSettingsLoadBalancing(
                parse_bigip_gtm_global_settings_load_balancing(full_path, body, range),
            ),
        ),
        ("gtm", "global-settings metrics") => (
            "gtm_global_settings_metrics",
            ModelObject::GtmGlobalSettingsMetrics(parse_bigip_gtm_global_settings_metrics(
                full_path, body, range,
            )),
        ),
        ("gtm", "global-settings metrics-exclusions") => (
            "gtm_global_settings_metrics_exclusions",
            ModelObject::GtmGlobalSettingsMetricsExclusions(
                parse_bigip_gtm_global_settings_metrics_exclusions(full_path, body, range),
            ),
        ),
        ("ltm", "dns analytics global-settings") => (
            "ltm_dns_analytics_global_settings",
            ModelObject::LtmDnsAnalyticsGlobalSettings(
                parse_bigip_ltm_dns_analytics_global_settings(full_path, body, range),
            ),
        ),
        ("ltm", "dns cache global-settings") => (
            "ltm_dns_cache_global_settings",
            ModelObject::LtmDnsCacheGlobalSettings(parse_bigip_ltm_dns_cache_global_settings(
                full_path, body, range,
            )),
        ),
        ("security", "firewall config-change-log") => (
            "security_firewall_config_change_log",
            ModelObject::SecurityFirewallConfigChangeLog(
                parse_bigip_security_firewall_config_change_log(full_path, body, range),
            ),
        ),
        ("security", "firewall global-fqdn-policy") => (
            "security_firewall_global_fqdn_policy",
            ModelObject::SecurityFirewallGlobalFqdnPolicy(
                parse_bigip_security_firewall_global_fqdn_policy(full_path, body, range),
            ),
        ),
        ("security", "firewall global-rules") => (
            "security_firewall_global_rules",
            ModelObject::SecurityFirewallGlobalRules(parse_bigip_security_firewall_global_rules(
                full_path, body, range,
            )),
        ),
        ("security", "firewall management-ip-rules") => (
            "security_firewall_management_ip_rules",
            ModelObject::SecurityFirewallManagementIpRules(
                parse_bigip_security_firewall_management_ip_rules(full_path, body, range),
            ),
        ),
        ("security", "firewall on-demand-compilation") => (
            "security_firewall_on_demand_compilation",
            ModelObject::SecurityFirewallOnDemandCompilation(
                parse_bigip_security_firewall_on_demand_compilation(full_path, body, range),
            ),
        ),
        ("security", "firewall on-demand-rule-deploy") => (
            "security_firewall_on_demand_rule_deploy",
            ModelObject::SecurityFirewallOnDemandRuleDeploy(
                parse_bigip_security_firewall_on_demand_rule_deploy(full_path, body, range),
            ),
        ),
        ("security", "firewall uuid-default-autogenerate") => (
            "security_firewall_uuid_default_autogenerate",
            ModelObject::SecurityFirewallUuidDefaultAutogenerate(
                parse_bigip_security_firewall_uuid_default_autogenerate(full_path, body, range),
            ),
        ),
        ("security", "ip-intelligence global-policy") => (
            "security_ip_intelligence_global_policy",
            ModelObject::SecurityIpIntelligenceGlobalPolicy(
                parse_bigip_security_ip_intelligence_global_policy(full_path, body, range),
            ),
        ),
        ("security", "packet-filter default-rules") => (
            "security_packet_filter_default_rules",
            ModelObject::SecurityPacketFilterDefaultRules(
                parse_bigip_security_packet_filter_default_rules(full_path, body, range),
            ),
        ),
        ("sys", "dns") => (
            "sys_dns",
            ModelObject::SysDns(parse_bigip_sys_dns(full_path, body, range)),
        ),
        ("sys", "global-settings") => (
            "sys_global_settings",
            ModelObject::SysGlobalSettings(parse_bigip_sys_global_settings(full_path, body, range)),
        ),
        ("sys", "ntp") => (
            "sys_ntp",
            ModelObject::SysNtp(parse_bigip_sys_ntp(full_path, body, range)),
        ),
        ("sys", "snmp") => (
            "sys_snmp",
            ModelObject::SysSnmp(parse_bigip_sys_snmp(full_path, body, range)),
        ),
        _ => return None,
    })
}

/// Generated minimal-kind dispatch (module + obj_type -> attr).
#[must_use]
pub fn dispatch_minimal(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<(&'static str, ModelObject)> {
    let attr = match (module, object_type) {
        ("analytics", "global-settings") => "analytics_global_settings",
        ("api-protection", "profile apiprotection") => "api_protection_profile_apiprotection",
        ("api-protection", "response") => "api_protection_response",
        ("api-protection", "server") => "api_protection_server",
        ("apm", "aaa active-directory") => "apm_aaa_active_directory",
        ("apm", "aaa active-directory-trusted-domains") => {
            "apm_aaa_active_directory_trusted_domains"
        }
        ("apm", "aaa crldp") => "apm_aaa_crldp",
        ("apm", "aaa endpoint-management-system") => "apm_aaa_endpoint_management_system",
        ("apm", "aaa f5-mfa-configuration") => "apm_aaa_f5_mfa_configuration",
        ("apm", "aaa f5-service-connector") => "apm_aaa_f5_service_connector",
        ("apm", "aaa http") => "apm_aaa_http",
        ("apm", "aaa http-connector-request") => "apm_aaa_http_connector_request",
        ("apm", "aaa http-connector-transport") => "apm_aaa_http_connector_transport",
        ("apm", "aaa kerberos") => "apm_aaa_kerberos",
        ("apm", "aaa kerberos-keytab-file") => "apm_aaa_kerberos_keytab_file",
        ("apm", "aaa ldap") => "apm_aaa_ldap",
        ("apm", "aaa localdb") => "apm_aaa_localdb",
        ("apm", "aaa oam") => "apm_aaa_oam",
        ("apm", "aaa oauth-provider") => "apm_aaa_oauth_provider",
        ("apm", "aaa oauth-request") => "apm_aaa_oauth_request",
        ("apm", "aaa oauth-server") => "apm_aaa_oauth_server",
        ("apm", "aaa ocsp") => "apm_aaa_ocsp",
        ("apm", "aaa okta-connector") => "apm_aaa_okta_connector",
        ("apm", "aaa radius") => "apm_aaa_radius",
        ("apm", "aaa saml") => "apm_aaa_saml",
        ("apm", "aaa saml-idp-automation") => "apm_aaa_saml_idp_automation",
        ("apm", "aaa saml-idp-connector") => "apm_aaa_saml_idp_connector",
        ("apm", "aaa securid") => "apm_aaa_securid",
        ("apm", "aaa tacacsplus") => "apm_aaa_tacacsplus",
        ("apm", "acl") => "apm_acl",
        ("apm", "apm-avr-config") => "apm_apm_avr_config",
        ("apm", "client image") => "apm_client_image",
        ("apm", "client-packaging") => "apm_client_packaging",
        ("apm", "configuration captcha") => "apm_configuration_captcha",
        ("apm", "epsec epsec-package") => "apm_epsec_epsec_package",
        ("apm", "log-setting") => "apm_log_setting",
        ("apm", "ntlm machine-account") => "apm_ntlm_machine_account",
        ("apm", "ntlm ntlm-auth") => "apm_ntlm_ntlm_auth",
        ("apm", "oauth jwk-config") => "apm_oauth_jwk_config",
        ("apm", "oauth jwt-config") => "apm_oauth_jwt_config",
        ("apm", "oauth jwt-provider-list") => "apm_oauth_jwt_provider_list",
        ("apm", "oauth oauth-claim") => "apm_oauth_oauth_claim",
        ("apm", "oauth oauth-client-app") => "apm_oauth_oauth_client_app",
        ("apm", "oauth oauth-resource-server") => "apm_oauth_oauth_resource_server",
        ("apm", "oauth oauth-scope") => "apm_oauth_oauth_scope",
        ("apm", "policy customization-group") => "apm_policy_customization_group",
        ("apm", "policy customization-languages") => "apm_policy_customization_languages",
        ("apm", "policy image-file") => "apm_policy_image_file",
        ("apm", "policy windows-group-policy-file") => "apm_policy_windows_group_policy_file",
        ("apm", "profile access") => "apm_profile_access",
        ("apm", "profile connectivity") => "apm_profile_connectivity",
        ("apm", "profile exchange") => "apm_profile_exchange",
        ("apm", "profile oauth") => "apm_profile_oauth",
        ("apm", "profile vdi") => "apm_profile_vdi",
        ("apm", "report custom-report-field") => "apm_report_custom_report_field",
        ("apm", "resource address-space") => "apm_resource_address_space",
        ("apm", "resource app-tunnel") => "apm_resource_app_tunnel",
        ("apm", "resource client-rate-class") => "apm_resource_client_rate_class",
        ("apm", "resource client-traffic-classifier") => "apm_resource_client_traffic_classifier",
        ("apm", "resource ipv6-leasepool") => "apm_resource_ipv6_leasepool",
        ("apm", "resource leasepool") => "apm_resource_leasepool",
        ("apm", "resource network-access") => "apm_resource_network_access",
        ("apm", "resource portal-access") => "apm_resource_portal_access",
        ("apm", "resource remote-desktop citrix") => "apm_resource_remote_desktop_citrix",
        ("apm", "resource remote-desktop citrix-client-bundle") => {
            "apm_resource_remote_desktop_citrix_client_bundle"
        }
        ("apm", "resource remote-desktop citrix-client-package-file") => {
            "apm_resource_remote_desktop_citrix_client_package_file"
        }
        ("apm", "resource remote-desktop quest") => "apm_resource_remote_desktop_quest",
        ("apm", "resource remote-desktop rdp") => "apm_resource_remote_desktop_rdp",
        ("apm", "resource remote-desktop vmware-view") => "apm_resource_remote_desktop_vmware_view",
        ("apm", "resource sandbox") => "apm_resource_sandbox",
        ("apm", "resource webtop") => "apm_resource_webtop",
        ("apm", "resource webtop-link") => "apm_resource_webtop_link",
        ("apm", "saml artifact-resolution-service") => "apm_saml_artifact_resolution_service",
        ("apm", "saml attribute-consuming-service") => "apm_saml_attribute_consuming_service",
        ("apm", "saml auth-context-class-list") => "apm_saml_auth_context_class_list",
        ("apm", "sso basic") => "apm_sso_basic",
        ("apm", "sso form-based") => "apm_sso_form_based",
        ("apm", "sso form-basedv2") => "apm_sso_form_basedv2",
        ("apm", "sso kerberos") => "apm_sso_kerberos",
        ("apm", "sso ntlmv1") => "apm_sso_ntlmv1",
        ("apm", "sso ntlmv2") => "apm_sso_ntlmv2",
        ("apm", "sso oauth-bearer") => "apm_sso_oauth_bearer",
        ("apm", "sso saml") => "apm_sso_saml",
        ("apm", "sso saml-resource") => "apm_sso_saml_resource",
        ("apm", "sso saml-sp-automation") => "apm_sso_saml_sp_automation",
        ("apm", "sso saml-sp-connector") => "apm_sso_saml_sp_connector",
        ("apm", "swg-scheme") => "apm_swg_scheme",
        ("apm", "url-filter") => "apm_url_filter",
        ("asm", "policy") => "asm_policies",
        ("cli", "admin-partitions") => "cli_admin_partitions",
        ("cli", "alias private") => "cli_alias_private",
        ("cli", "alias shared") => "cli_alias_shared",
        ("cli", "global-settings") => "cli_global_settings",
        ("cli", "preference") => "cli_preference",
        ("cli", "script") => "cli_script",
        ("cli", "transaction") => "cli_transaction",
        ("cli", "version") => "cli_version",
        ("cm", "config-sync") => "cm_config_sync",
        ("ilx", "global-settings") => "ilx_global_settings",
        ("ltm", "alg-log-profile") => "ltm_alg_log_profiles",
        ("ltm", "classification application") => "ltm_classification_application",
        ("ltm", "classification auto-update settings") => "ltm_classification_auto_update_settings",
        ("ltm", "classification category") => "ltm_classification_category",
        ("ltm", "classification ce") => "ltm_classification_ce",
        ("ltm", "classification signature-update-schedule") => {
            "ltm_classification_signature_update_schedule"
        }
        ("ltm", "classification url-cat-policy") => "ltm_classification_url_cat_policy",
        ("ltm", "classification url-category") => "ltm_classification_url_category",
        ("ltm", "classification urldb-feed-list") => "ltm_classification_urldb_feed_list",
        ("ltm", "classification urldb-file") => "ltm_classification_urldb_file",
        ("ltm", "clientssl ocsp-stapling-responses") => "ltm_clientssl_ocsp_stapling_responses",
        ("ltm", "clientssl-proxy cached-certs") => "ltm_clientssl_proxy_cached_certs",
        ("ltm", "default-node-monitor") => "ltm_default_node_monitor",
        ("ltm", "global-settings connection") => "ltm_global_settings_connection",
        ("ltm", "global-settings general") => "ltm_global_settings_general",
        ("ltm", "global-settings rule") => "ltm_global_settings_rule",
        ("ltm", "global-settings traffic-control") => "ltm_global_settings_traffic_control",
        ("ltm", "html-rule comment-raise-event") => "ltm_html_rule_comment_raise_event",
        ("ltm", "html-rule comment-remove") => "ltm_html_rule_comment_remove",
        ("ltm", "html-rule tag-append-html") => "ltm_html_rule_tag_append_html",
        ("ltm", "html-rule tag-prepend-html") => "ltm_html_rule_tag_prepend_html",
        ("ltm", "html-rule tag-raise-event") => "ltm_html_rule_tag_raise_event",
        ("ltm", "html-rule tag-remove") => "ltm_html_rule_tag_remove",
        ("ltm", "html-rule tag-remove-attribute") => "ltm_html_rule_tag_remove_attribute",
        ("ltm", "lsn-log-profile") => "ltm_lsn_log_profiles",
        ("ltm", "lsn-pool") => "ltm_lsn_pools",
        ("ltm", "rule-profiler") => "ltm_rule_profiler",
        ("ltm", "tacdb customdb") => "ltm_tacdb_customdb",
        ("ltm", "tacdb customdb-file") => "ltm_tacdb_customdb_file",
        ("ltm", "tacdb licenseddb") => "ltm_tacdb_licenseddb",
        ("net", "address-list") => "net_address_lists",
        ("net", "arp") => "net_arp",
        ("net", "bwc policy") => "net_bwc_policies",
        ("net", "bwc priority-group") => "net_bwc_priority_groups",
        ("net", "bwc traffic-group") => "net_bwc_traffic_groups",
        ("net", "cos global-settings") => "net_cos_global_settings",
        ("net", "cos map-8021p") => "net_cos_map_8021p",
        ("net", "cos map-dscp") => "net_cos_map_dscp",
        ("net", "cos traffic-priority") => "net_cos_traffic_priority",
        ("net", "dag-globals") => "net_dag_globals",
        ("net", "fdb tunnel") => "net_fdb_tunnel",
        ("net", "fdb vlan") => "net_fdb_vlan",
        ("net", "interface-cos") => "net_interface_cos",
        ("net", "ipsec ike-daemon") => "net_ipsec_ike_daemon",
        ("net", "ipsec ike-peer") => "net_ipsec_ike_peers",
        ("net", "ipsec ipsec-policy") => "net_ipsec_ipsec_policies",
        ("net", "ipsec manual-security-association") => "net_ipsec_manual_security_associations",
        ("net", "ipsec traffic-selector") => "net_ipsec_traffic_selectors",
        ("net", "ipv6-subscriber-prefix-length") => "net_ipv6_subscriber_prefix_length",
        ("net", "lacp-globals") => "net_lacp_globals",
        ("net", "lldp-globals") => "net_lldp_globals",
        ("net", "multicast-globals") => "net_multicast_globals",
        ("net", "ndp") => "net_ndp",
        ("net", "packet-filter") => "net_packet_filter",
        ("net", "packet-filter-trusted") => "net_packet_filter_trusted",
        ("net", "port-mirror") => "net_port_mirror",
        ("net", "rate-shaping class") => "net_rate_shaping_class",
        ("net", "rate-shaping color-policer") => "net_rate_shaping_color_policer",
        ("net", "rate-shaping drop-policy") => "net_rate_shaping_drop_policy",
        ("net", "rate-shaping queue") => "net_rate_shaping_queue",
        ("net", "rate-shaping shaping-policy") => "net_rate_shaping_shaping_policy",
        ("net", "router-advertisement") => "net_router_advertisements",
        ("net", "routing access-list") => "net_routing_access_lists",
        ("net", "routing as-path") => "net_routing_as_paths",
        ("net", "routing bfd") => "net_routing_bfd",
        ("net", "routing bgp") => "net_routing_bgp",
        ("net", "routing community-list") => "net_routing_community_lists",
        ("net", "routing debug") => "net_routing_debug",
        ("net", "routing extcommunity-list") => "net_routing_extcommunity_lists",
        ("net", "routing prefix-list") => "net_routing_prefix_lists",
        ("net", "routing profile bgp") => "net_routing_profile_bgp",
        ("net", "routing route-map") => "net_routing_route_maps",
        ("net", "rst-cause") => "net_rst_cause",
        ("net", "self-allow") => "net_self_allow",
        ("net", "service-policy") => "net_service_policy",
        ("net", "sfc chain") => "net_sfc_chain",
        ("net", "sfc sf") => "net_sfc_sf",
        ("net", "stp-globals") => "net_stp_globals",
        ("net", "timer-policy") => "net_timer_policy",
        ("net", "trunk") => "net_trunk",
        ("net", "tunnels endpoint") => "net_tunnels_endpoints",
        ("net", "tunnels etherip") => "net_tunnels_etherip",
        ("net", "tunnels fec") => "net_tunnels_fec",
        ("net", "tunnels geneve") => "net_tunnels_geneve",
        ("net", "tunnels gre") => "net_tunnels_gre",
        ("net", "tunnels ipip") => "net_tunnels_ipip",
        ("net", "tunnels ipsec") => "net_tunnels_ipsec",
        ("net", "tunnels lw4o6") => "net_tunnels_lw4o6",
        ("net", "tunnels map") => "net_tunnels_map",
        ("net", "tunnels ppp") => "net_tunnels_ppp",
        ("net", "tunnels tcp-forward") => "net_tunnels_tcp_forward",
        ("net", "tunnels v6rd") => "net_tunnels_v6rd",
        ("net", "tunnels vxlan") => "net_tunnels_vxlan",
        ("net", "tunnels wccp") => "net_tunnels_wccp",
        ("net", "vlan-group") => "net_vlan_group",
        ("net", "wccp") => "net_wccp",
        ("pem", "global-settings analytics") => "pem_gs_analytics",
        ("pem", "global-settings gx") => "pem_gs_gx",
        ("pem", "global-settings hsl-flow") => "pem_gs_hsl_flow",
        ("pem", "global-settings hsl-report") => "pem_gs_hsl_report",
        ("pem", "global-settings insert-content") => "pem_gs_insert_content",
        ("pem", "global-settings policy") => "pem_gs_policy",
        ("pem", "global-settings quota-mgmt") => "pem_gs_quota_mgmt",
        ("pem", "global-settings session-mgmt-attributes") => "pem_gs_session_mgmt_attributes",
        ("pem", "global-settings subscriber-activity-log") => "pem_gs_subscriber_activity_log",
        ("pem", "protocol diameter-avp") => "pem_protocol_diameter_avp",
        ("pem", "protocol profile gx") => "pem_protocol_profile_gx",
        ("pem", "protocol profile radius") => "pem_protocol_profile_radius",
        ("pem", "protocol radius-avp") => "pem_protocol_radius_avp",
        ("pem", "reporting format-script") => "pem_reporting_format_script",
        ("pem", "subscriber") => "pem_subscriber",
        ("pem", "subscriber-attribute") => "pem_subscriber_attribute",
        ("security", "analytics settings") => "security_analytics_settings",
        ("security", "anti-fraud profile") => "security_anti_fraud_profiles",
        ("security", "anti-fraud signatures-update") => "security_anti_fraud_signatures_update",
        ("security", "blacklist-publisher category") => "security_blacklist_publisher_categories",
        ("security", "blacklist-publisher profile") => "security_blacklist_publisher_profiles",
        ("security", "bot-defense signature") => "security_bot_defense_signatures",
        ("security", "bot-defense signature-category") => {
            "security_bot_defense_signature_categories"
        }
        ("security", "cloud-services connector") => "security_cloud_services_connectors",
        ("security", "datasync background-tasks") => "security_datasync_background_tasks",
        ("security", "datasync global-profile") => "security_datasync_global_profiles",
        ("security", "datasync local-profile") => "security_datasync_local_profiles",
        ("security", "debug drop-redirect-stats") => "security_debug_drop_redirect_stats",
        ("security", "debug matcher") => "security_debug_matcher",
        ("security", "debug register") => "security_debug_register",
        ("security", "device device-context") => "security_device_device_context",
        ("security", "dos autodos-file-object") => "security_dos_autodos_file_objects",
        ("security", "dos behavioral-signature") => "security_dos_behavioral_signatures",
        ("security", "dos bot-signature") => "security_dos_bot_signatures",
        ("security", "dos bot-signature-category") => "security_dos_bot_signature_categories",
        ("security", "dos device-config") => "security_dos_device_config",
        ("security", "dos dns-nxdomain-stat") => "security_dos_dns_nxdomain_stat",
        ("security", "dos dos-signature") => "security_dos_dos_signatures",
        ("security", "dos dynamic-signatures") => "security_dos_dynamic_signatures",
        ("security", "dos ip-uncommon-protolist") => "security_dos_ip_uncommon_protolists",
        ("security", "dos ipv6-ext-hdr") => "security_dos_ipv6_ext_hdr",
        ("security", "dos l4bdos-file-object") => "security_dos_l4bdos_file_objects",
        ("security", "dos network-whitelist") => "security_dos_network_whitelists",
        ("security", "dos profile-signatures") => "security_dos_profile_signatures",
        ("security", "dos stress-stats") => "security_dos_stress_stats",
        ("security", "dos udp-portlist") => "security_dos_udp_portlists",
        ("security", "dos virtual") => "security_dos_virtuals",
        ("security", "flowspec-route-injector profile") => {
            "security_flowspec_route_injector_profiles"
        }
        ("security", "ip-intelligence blacklist-category") => {
            "security_ip_intelligence_blacklist_categories"
        }
        ("security", "protocol-inspection common-config") => {
            "security_protocol_inspection_common_config"
        }
        ("security", "protocol-inspection learning-stats") => {
            "security_protocol_inspection_learning_stats"
        }
        ("security", "protocol-inspection profile") => "security_protocol_inspection_profiles",
        ("security", "protocol-inspection signature") => "security_protocol_inspection_signatures",
        ("security", "scrubber profile") => "security_scrubber_profiles",
        ("security", "shared-objects address-list") => "security_shared_objects_address_lists",
        ("security", "shared-objects port-list") => "security_shared_objects_port_lists",
        ("security", "ssh ciphers") => "security_ssh_ciphers",
        ("sys", "alert lcd") => "sys_alert_lcd",
        ("sys", "aom") => "sys_aom",
        ("sys", "appiq config") => "sys_appiq_config",
        ("sys", "application apl-script") => "sys_application_apl_script",
        ("sys", "application custom-stat") => "sys_application_custom_stat",
        ("sys", "application service") => "sys_application_service",
        ("sys", "application template") => "sys_application_template",
        ("sys", "autoscale-group") => "sys_autoscale_group",
        ("sys", "cluster") => "sys_cluster",
        ("sys", "compatibility-level") => "sys_compatibility_level",
        ("sys", "config") => "sys_config",
        ("sys", "console") => "sys_console",
        ("sys", "crypto acceleration-strategy") => "sys_crypto_acceleration_strategy",
        ("sys", "crypto ca-bundle-manager") => "sys_crypto_ca_bundle_manager",
        ("sys", "crypto cert") => "sys_crypto_cert",
        ("sys", "crypto cert-order-manager") => "sys_crypto_cert_order_manager",
        ("sys", "crypto cert-validation-response ocsp") => {
            "sys_crypto_cert_validation_response_ocsp"
        }
        ("sys", "crypto cert-validator crl") => "sys_crypto_cert_validator_crl",
        ("sys", "crypto cert-validator ocsp") => "sys_crypto_cert_validator_ocsp",
        ("sys", "crypto client") => "sys_crypto_client",
        ("sys", "crypto crl") => "sys_crypto_crl",
        ("sys", "crypto csr") => "sys_crypto_csr",
        ("sys", "crypto fips external-hsm") => "sys_crypto_fips_external_hsm",
        ("sys", "crypto fips key") => "sys_crypto_fips_key",
        ("sys", "crypto key") => "sys_crypto_key",
        ("sys", "crypto master-key") => "sys_crypto_master_key",
        ("sys", "crypto server") => "sys_crypto_server",
        ("sys", "daemon-log-settings clusterd") => "sys_daemon_log_settings_clusterd",
        ("sys", "daemon-log-settings csyncd") => "sys_daemon_log_settings_csyncd",
        ("sys", "daemon-log-settings icr-eventd") => "sys_daemon_log_settings_icr_eventd",
        ("sys", "daemon-log-settings icrd") => "sys_daemon_log_settings_icrd",
        ("sys", "daemon-log-settings lind") => "sys_daemon_log_settings_lind",
        ("sys", "daemon-log-settings mcpd") => "sys_daemon_log_settings_mcpd",
        ("sys", "daemon-log-settings tmm") => "sys_daemon_log_settings_tmm",
        ("sys", "datastor") => "sys_datastor",
        ("sys", "db") => "sys_db",
        ("sys", "default-config") => "sys_default_config",
        ("sys", "diags ihealth") => "sys_diags_ihealth",
        ("sys", "dynad settings") => "sys_dynad_settings",
        ("sys", "ecm cloud-provider") => "sys_ecm_cloud_provider",
        ("sys", "failover") => "sys_failover",
        ("sys", "feature-module") => "sys_feature_module",
        ("sys", "file apache-ssl-cert") => "sys_file_apache_ssl_cert",
        ("sys", "file browser-capabilities-db") => "sys_file_browser_capabilities_db",
        ("sys", "file data-group") => "sys_file_data_group",
        ("sys", "file device-capabilities-db") => "sys_file_device_capabilities_db",
        ("sys", "file external-monitor") => "sys_file_external_monitor",
        ("sys", "file ifile") => "sys_file_ifile",
        ("sys", "file lwtunneltbl") => "sys_file_lwtunneltbl",
        ("sys", "file rewrite-rule") => "sys_file_rewrite_rule",
        ("sys", "file ssl-crl") => "sys_file_ssl_crl",
        ("sys", "fpga firmware-config") => "sys_fpga_firmware_config",
        ("sys", "ha-group") => "sys_ha_group",
        ("sys", "httpd") => "sys_httpd",
        ("sys", "icall handler periodic") => "sys_icall_handler_periodic",
        ("sys", "icall handler perpetual") => "sys_icall_handler_perpetual",
        ("sys", "icall handler triggered") => "sys_icall_handler_triggered",
        ("sys", "icall istats-trigger") => "sys_icall_istats_trigger",
        ("sys", "icall script") => "sys_icall_script",
        ("sys", "internal-proxy") => "sys_internal_proxy",
        ("sys", "ipfix destination") => "sys_ipfix_destination",
        ("sys", "ipfix element") => "sys_ipfix_element",
        ("sys", "ipfix irules") => "sys_ipfix_irules",
        ("sys", "log-config destination alertd") => "sys_log_config_destination_alertd",
        ("sys", "log-config destination arcsight") => "sys_log_config_destination_arcsight",
        ("sys", "log-config destination ipfix") => "sys_log_config_destination_ipfix",
        ("sys", "log-config destination local-database") => {
            "sys_log_config_destination_local_database"
        }
        ("sys", "log-config destination local-syslog") => "sys_log_config_destination_local_syslog",
        ("sys", "log-config destination management-port") => {
            "sys_log_config_destination_management_port"
        }
        ("sys", "log-config destination remote-high-speed-log") => {
            "sys_log_config_destination_remote_high_speed_log"
        }
        ("sys", "log-config destination remote-syslog") => {
            "sys_log_config_destination_remote_syslog"
        }
        ("sys", "log-config destination splunk") => "sys_log_config_destination_splunk",
        ("sys", "log-config filter") => "sys_log_config_filter",
        ("sys", "log-config publisher") => "sys_log_config_publisher",
        ("sys", "log-rotate") => "sys_log_rotate",
        ("sys", "management-dhcp") => "sys_management_dhcp",
        ("sys", "management-ip") => "sys_management_ip",
        ("sys", "management-ovsdb") => "sys_management_ovsdb",
        ("sys", "management-proxy-config") => "sys_management_proxy_config",
        ("sys", "outbound-smtp") => "sys_outbound_smtp",
        ("sys", "sflow global-settings http") => "sys_sflow_global_settings_http",
        ("sys", "sflow global-settings interface") => "sys_sflow_global_settings_interface",
        ("sys", "sflow global-settings system") => "sys_sflow_global_settings_system",
        ("sys", "sflow global-settings vlan") => "sys_sflow_global_settings_vlan",
        ("sys", "sflow receiver") => "sys_sflow_receiver",
        ("sys", "smtp-server") => "sys_smtp_server",
        ("sys", "software hotfix") => "sys_software_hotfix",
        ("sys", "software image") => "sys_software_image",
        ("sys", "software signature") => "sys_software_signature",
        ("sys", "software update") => "sys_software_update",
        ("sys", "software volume") => "sys_software_volume",
        ("sys", "sshd") => "sys_sshd",
        ("sys", "state-mirroring") => "sys_state_mirroring",
        ("sys", "syslog") => "sys_syslog",
        ("sys", "tmm-traffic") => "sys_tmm_traffic",
        ("sys", "traffic") => "sys_traffic",
        ("sys", "turboflex profile-config") => "sys_turboflex_profile_config",
        ("sys", "ucs") => "sys_ucs",
        ("sys", "url-db download-schedule") => "sys_url_db_download_schedule",
        ("sys", "url-db url-category") => "sys_url_db_url_category",
        ("vcmp", "guest") => "vcmp_guests",
        ("vcmp", "traffic-profile") => "vcmp_traffic_profiles",
        ("vcmp", "virtual-disk") => "vcmp_virtual_disks",
        ("vcmp", "virtual-disk-template") => "vcmp_virtual_disk_templates",
        ("wom", "endpoint-discovery") => "wom_endpoint_discovery",
        _ => return None,
    };
    let kind = format!("{module} {object_type}");
    Some((
        attr,
        ModelObject::Minimal(make_minimal(full_path, body, &kind, range)),
    ))
}

/// Generated rich ltm auth / message-routing dispatch.
#[must_use]
pub fn dispatch_ltm_tables(
    module: &str,
    object_type: &str,
    full_path: &str,
    body: &str,
    range: Range,
) -> Option<(&'static str, ModelObject)> {
    Some(match (module, object_type) {
        ("ltm", "auth crldp-server") => (
            "ltm_auth_crldp_servers",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth kerberos-delegation") => (
            "ltm_auth_kerberos_delegations",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth ldap") => (
            "ltm_auth_ldap",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth ocsp-responder") => (
            "ltm_auth_ocsp_responders",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth profile") => (
            "ltm_auth_profiles",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth radius") => (
            "ltm_auth_radius",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth radius-server") => (
            "ltm_auth_radius_servers",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth ssl-cc-ldap") => (
            "ltm_auth_ssl_cc_ldap",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth ssl-crldp") => (
            "ltm_auth_ssl_crldp",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth ssl-ocsp") => (
            "ltm_auth_ssl_ocsp",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "auth tacacs") => (
            "ltm_auth_tacacs",
            ModelObject::LtmAuthObject(parse_bigip_ltm_auth_object(full_path, body, range)),
        ),
        ("ltm", "message-routing diameter peer") => (
            "ltm_mr_diameter_peers",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing diameter profile router") => (
            "ltm_mr_diameter_profile_router",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing diameter profile session") => (
            "ltm_mr_diameter_profile_session",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing diameter route") => (
            "ltm_mr_diameter_routes",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing diameter transport-config") => (
            "ltm_mr_diameter_transport_config",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing generic peer") => (
            "ltm_mr_generic_peers",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing generic protocol") => (
            "ltm_mr_generic_protocols",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing generic route") => (
            "ltm_mr_generic_routes",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing generic router") => (
            "ltm_mr_generic_routers",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing generic transport-config") => (
            "ltm_mr_generic_transport_config",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing mqtt peer") => (
            "ltm_mr_mqtt_peers",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing mqtt profile router") => (
            "ltm_mr_mqtt_profile_router",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing mqtt profile session") => (
            "ltm_mr_mqtt_profile_session",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing mqtt route") => (
            "ltm_mr_mqtt_routes",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing mqtt transport-config") => (
            "ltm_mr_mqtt_transport_config",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing sip peer") => (
            "ltm_mr_sip_peers",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing sip profile router") => (
            "ltm_mr_sip_profile_router",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing sip profile session") => (
            "ltm_mr_sip_profile_session",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing sip route") => (
            "ltm_mr_sip_routes",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        ("ltm", "message-routing sip transport-config") => (
            "ltm_mr_sip_transport_config",
            ModelObject::LtmMessageRoutingObject(parse_bigip_ltm_message_routing_object(
                full_path, body, range,
            )),
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod header_quote_tests {
    use super::parse_header_strict;

    #[test]
    fn quoted_identifier_with_space_is_not_truncated() {
        // A quoted full-path containing a space must survive whole on the
        // typed-object (strict) path, matching the quote-aware generic path —
        // not truncate at the inner space (issue 188).
        let (module, otype, full_path) =
            parse_header_strict("security bot-defense signature \"/Common/Microsoft Access\"")
                .expect("known typed object");
        assert_eq!(module, "security");
        assert_eq!(otype, "bot-defense signature");
        assert_eq!(full_path, "/Common/Microsoft Access");
    }

    #[test]
    fn unquoted_identifier_still_parses() {
        let (module, otype, full_path) =
            parse_header_strict("security bot-defense signature /Common/plain")
                .expect("known typed object");
        assert_eq!(module, "security");
        assert_eq!(otype, "bot-defense signature");
        assert_eq!(full_path, "/Common/plain");
    }
}
