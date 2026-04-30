//! F5 profile and protocol namespace metadata.
//!
//! Static data tables describing the 57 profile types, 87 protocol
//! command namespaces, and stack modification commands.

use std::collections::HashMap;

/// Metadata for an F5 profile type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSpec {
    /// Profile type name (e.g. `"HTTP"`, `"CLIENTSSL"`, `"DNS"`).
    pub name: &'static str,
    /// Protocol stack layer.
    pub layer: &'static str,
    /// Connection side: `"client"`, `"server"`, `"both"`, `"global"`.
    pub side: &'static str,
    /// Required parent profiles.
    pub requires: &'static [&'static str],
    /// Conflicting profiles.
    pub conflicts: &'static [&'static str],
    /// Profile capabilities (e.g. `"sni"`, `"cipher"`, `"cert"`).
    pub capabilities: &'static [&'static str],
}

/// iRules protocol command namespace availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolNamespaceSpec {
    /// Namespace prefix (e.g. `"HTTP"`, `"SSL"`, `"TCP"`).
    pub prefix: &'static str,
    /// Profiles that provide this namespace.
    pub profiles: &'static [&'static str],
    /// Protocol layer.
    pub layer: &'static str,
    /// Default connection side.
    pub side: &'static str,
    /// Whether `clientside`/`serverside` qualifiers are supported.
    pub side_selectable: bool,
}

/// A command that changes the active profile stack at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackModification {
    /// Command name (e.g. `"SSL::disable"`).
    pub command: &'static str,
    /// Connection side affected.
    pub side: Option<&'static str>,
    /// Profile removed by this command.
    pub removes_profile: Option<&'static str>,
    /// Profile added by this command.
    pub adds_profile: Option<&'static str>,
}

/// Profile registry providing lookup over static profile tables.
pub struct ProfileRegistry {
    profiles: HashMap<&'static str, ProfileSpec>,
    namespaces: HashMap<&'static str, ProtocolNamespaceSpec>,
    modifications: Vec<StackModification>,
}

impl ProfileRegistry {
    /// Build the profile registry from static data.
    #[must_use]
    pub fn build() -> Self {
        let mut profiles = HashMap::new();
        for spec in profile_specs() {
            profiles.insert(spec.name, spec);
        }
        let mut namespaces = HashMap::new();
        for spec in protocol_namespace_specs() {
            namespaces.insert(spec.prefix, spec);
        }
        Self {
            profiles,
            namespaces,
            modifications: modification_specs(),
        }
    }

    /// Look up a profile spec by name.
    #[must_use]
    pub fn get_profile(&self, name: &str) -> Option<&ProfileSpec> {
        self.profiles.get(name)
    }

    /// Look up a protocol namespace by prefix.
    #[must_use]
    pub fn get_namespace(&self, prefix: &str) -> Option<&ProtocolNamespaceSpec> {
        self.namespaces.get(prefix)
    }

    /// All registered profile names.
    #[must_use]
    pub fn all_profile_names(&self) -> Vec<&str> {
        self.profiles.keys().copied().collect()
    }

    /// All registered namespace prefixes.
    #[must_use]
    pub fn all_namespace_prefixes(&self) -> Vec<&str> {
        self.namespaces.keys().copied().collect()
    }

    /// Stack modification commands.
    #[must_use]
    pub fn modifications(&self) -> &[StackModification] {
        &self.modifications
    }

    /// Number of registered profiles.
    #[must_use]
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    /// Number of registered namespaces.
    #[must_use]
    pub fn namespace_count(&self) -> usize {
        self.namespaces.len()
    }
}

// Full static data — auto-generated from Python namespace_data.py

// AUTO-GENERATED from Python namespace_data.py — do not edit manually

#[allow(clippy::too_many_lines)]
fn profile_specs() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "ACCESS",
            layer: "security",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "ANTIFRAUD",
            layer: "security",
            side: "client",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "ASM",
            layer: "security",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "AUTH",
            layer: "security",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "AVR",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "BOTDEFENSE",
            layer: "security",
            side: "client",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CACHE",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CATEGORY",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CLASSIFICATION",
            layer: "acceleration",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CLIENTSSL",
            layer: "tls",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[
                "cert",
                "cipher",
                "extensions",
                "sessionid",
                "sni",
                "tls_control",
                "tls_data",
            ],
        },
        ProfileSpec {
            name: "CONNECTOR",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DATAGRAM",
            layer: "application",
            side: "both",
            requires: &["UDP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DIAMETER",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DIAMETERSESSION",
            layer: "application",
            side: "both",
            requires: &["DIAMETER"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DIAMETER_ENDPOINT",
            layer: "application",
            side: "both",
            requires: &["DIAMETER"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DNS",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "DOSL7",
            layer: "security",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "ECA",
            layer: "security",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "FASTHTTP",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "FASTL4",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "FIX",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "FLOW",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "GENERICMSG",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "GTP",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "HTML",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "HTTP",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "HTTP2",
            layer: "application",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "HTTP_PROXY_CONNECT",
            layer: "application",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "ICAP",
            layer: "acceleration",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "IPS",
            layer: "security",
            side: "both",
            requires: &["PROTOCOL_INSPECTION"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "IVS_ENTRY",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "JSON",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "L7CHECK",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "LSN",
            layer: "application",
            side: "client",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "MQTT",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "MR",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "MSSQL",
            layer: "application",
            side: "both",
            requires: &["TDS"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "NAME",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "PCP",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "PEM",
            layer: "security",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "PERSIST",
            layer: "tls",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "PROTOCOL_INSPECTION",
            layer: "security",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "QOE",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "RADIUS",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "RADIUS_AAA",
            layer: "application",
            side: "both",
            requires: &["RADIUS"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "REQUESTADAPT",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "RESPONSEADAPT",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "REWRITE",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "RTSP",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SCTP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SERVERSSL",
            layer: "tls",
            side: "server",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[
                "cert",
                "cipher",
                "extensions",
                "sessionid",
                "sni",
                "tls_control",
                "tls_data",
            ],
        },
        ProfileSpec {
            name: "SIP",
            layer: "application",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SIPROUTER",
            layer: "application",
            side: "both",
            requires: &["SIP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SIPSESSION",
            layer: "application",
            side: "both",
            requires: &["SIP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SOCKS",
            layer: "application",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SSE",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SSL_PERSISTENCE",
            layer: "tls",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &["extensions", "sessionid", "sni"],
        },
        ProfileSpec {
            name: "STREAM",
            layer: "acceleration",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "TAP",
            layer: "security",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "TCP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "TDS",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "UDP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "WEBACCELERATION",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "WS",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "XML",
            layer: "acceleration",
            side: "both",
            requires: &["HTTP"],
            conflicts: &[],
            capabilities: &[],
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn protocol_namespace_specs() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "AAA",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS2",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACL",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ADAPT",
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AES",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AM",
            profiles: &[],
            layer: "acceleration",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ANTIFRAUD",
            profiles: &["ANTIFRAUD"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASM",
            profiles: &["ASM"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASN1",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AUTH",
            profiles: &["AUTH"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AVR",
            profiles: &["AVR"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BIGPROTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BIGTCP",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BOTDEFENSE",
            profiles: &["BOTDEFENSE"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BWC",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CACHE",
            profiles: &["CACHE", "WEBACCELERATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CATEGORY",
            profiles: &["CATEGORY"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFICATION",
            profiles: &["CLASSIFICATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFY",
            profiles: &["FASTHTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "COMPRESS",
            profiles: &["FASTHTTP", "HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CONNECTOR",
            profiles: &["CONNECTOR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CRYPTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DATAGRAM",
            profiles: &["DATAGRAM"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DECOMPRESS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DEMANGLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCP",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv4",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv6",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAG",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAMETER",
            profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNS",
            profiles: &["DNS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNSMSG",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DOSL7",
            profiles: &["DOSL7"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DSLITE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ECA",
            profiles: &["ECA"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FIX",
            profiles: &["FIX"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOW",
            profiles: &["FLOW"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOWTABLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FTP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GENERICMESSAGE",
            profiles: &["GENERICMSG"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GTP",
            profiles: &["GTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HA",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HSL",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTML",
            profiles: &["HTML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP",
            profiles: &["FASTHTTP", "HTTP", "HTTP_PROXY_CONNECT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP2",
            profiles: &["HTTP2"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTPLOG",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ICAP",
            profiles: &["ICAP"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IKE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ILX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IMAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IP",
            profiles: &[],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "IPFIX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISESSION",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISTATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IVS_ENTRY",
            profiles: &["IVS_ENTRY"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "JSON",
            profiles: &["JSON"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "L7CHECK",
            profiles: &["L7CHECK"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LB",
            profiles: &[],
            layer: "load_balance",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LDAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINK",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LSN",
            profiles: &["LSN"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MESSAGE",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MQTT",
            profiles: &["MQTT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MR",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NAME",
            profiles: &["NAME"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NSH",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NTLM",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "OFFBOX",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ONECONNECT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PCP",
            profiles: &["PCP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PEM",
            profiles: &["PEM"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PLUGIN",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POLICY",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POP3",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROFILE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROTOCOL_INSPECTION",
            profiles: &["IPS", "PROTOCOL_INSPECTION"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSC",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "QOE",
            profiles: &["QOE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RADIUS",
            profiles: &["RADIUS", "RADIUS_AAA"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RESOLV",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RESOLVER",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REST",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REWRITE",
            profiles: &["REWRITE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ROUTE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RTSP",
            profiles: &["RTSP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SCTP",
            profiles: &["SCTP"],
            layer: "transport",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SDP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIP",
            profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIPALG",
            profiles: &["MR", "SIP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SMTPS",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SOCKS",
            profiles: &["SOCKS"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SSE",
            profiles: &["SSE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SSL",
            profiles: &["CLIENTSSL", "PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
            layer: "tls",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "STATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "STREAM",
            profiles: &["STREAM"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TAP",
            profiles: &["TAP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TCP",
            profiles: &["TCP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "TDS",
            profiles: &["MSSQL", "TDS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TMM",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "UDP",
            profiles: &["UDP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "URI",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VALIDATE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VDI",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WAM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WEBSSO",
            profiles: &["ACCESS", "HTTP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WS",
            profiles: &["WS"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "X509",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XLAT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XML",
            profiles: &["XML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn modification_specs() -> Vec<StackModification> {
    vec![
        StackModification {
            command: "SSL::disable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "SSL::enable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::disable",
            side: None,
            removes_profile: Some("HTTP"),
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::enable",
            side: None,
            removes_profile: None,
            adds_profile: Some("HTTP"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_profile_registry() {
        let reg = ProfileRegistry::build();
        assert!(reg.profile_count() > 10);
        assert!(reg.namespace_count() > 10);
    }

    #[test]
    fn profile_lookup() {
        let reg = ProfileRegistry::build();
        let http = reg.get_profile("HTTP").unwrap();
        assert_eq!(http.layer, "application");
        assert!(http.requires.contains(&"TCP"));
    }

    #[test]
    fn namespace_lookup() {
        let reg = ProfileRegistry::build();
        let http_ns = reg.get_namespace("HTTP").unwrap();
        assert!(http_ns.profiles.contains(&"HTTP"));
        assert_eq!(http_ns.layer, "application");
    }

    #[test]
    fn ssl_namespace_side_selectable() {
        let reg = ProfileRegistry::build();
        let ssl = reg.get_namespace("SSL").unwrap();
        assert!(ssl.side_selectable);
    }

    #[test]
    fn modification_specs_exist() {
        let reg = ProfileRegistry::build();
        assert_eq!(reg.modifications().len(), 4);
    }

    #[test]
    fn clientssl_has_capabilities() {
        let reg = ProfileRegistry::build();
        let cs = reg.get_profile("CLIENTSSL").unwrap();
        assert!(cs.capabilities.contains(&"sni"));
        assert!(cs.capabilities.contains(&"cert"));
    }
}
