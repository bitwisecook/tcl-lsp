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

// Representative static data — the full tables will be generated from Python.

#[allow(clippy::too_many_lines)]
fn profile_specs() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "TCP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "UDP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &["TCP"],
            capabilities: &[],
        },
        ProfileSpec {
            name: "FASTL4",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &["TCP", "UDP"],
            capabilities: &[],
        },
        ProfileSpec {
            name: "SCTP",
            layer: "transport",
            side: "both",
            requires: &[],
            conflicts: &["TCP", "UDP"],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CLIENTSSL",
            layer: "tls",
            side: "client",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[
                "sni",
                "extensions",
                "sessionid",
                "cipher",
                "cert",
                "tls_data",
                "tls_control",
            ],
        },
        ProfileSpec {
            name: "SERVERSSL",
            layer: "tls",
            side: "server",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &["cert", "cipher"],
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
            name: "FASTHTTP",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &["HTTP"],
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
            name: "SIP",
            layer: "application",
            side: "both",
            requires: &["TCP"],
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
            name: "DIAMETER",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "MQTT",
            layer: "message",
            side: "both",
            requires: &["TCP"],
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
            name: "ACCESS",
            layer: "security",
            side: "both",
            requires: &[],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "PERSIST",
            layer: "tls_shared",
            side: "both",
            requires: &[],
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
            name: "STREAM",
            layer: "application",
            side: "both",
            requires: &["TCP"],
            conflicts: &[],
            capabilities: &[],
        },
        ProfileSpec {
            name: "CLASSIFICATION",
            layer: "application",
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
    ]
}

#[allow(clippy::too_many_lines)]
fn protocol_namespace_specs() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "HTTP",
            profiles: &["HTTP", "FASTHTTP", "HTTP_PROXY_CONNECT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SSL",
            profiles: &["CLIENTSSL", "SERVERSSL", "PERSIST", "SSL_PERSISTENCE"],
            layer: "tls",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "TCP",
            profiles: &["TCP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "UDP",
            profiles: &["UDP"],
            layer: "transport",
            side: "both",
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
            prefix: "LB",
            profiles: &[],
            layer: "load_balance",
            side: "global",
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
            prefix: "SIP",
            profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
            layer: "application",
            side: "both",
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
            prefix: "FIX",
            profiles: &["FIX"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MQTT",
            profiles: &["MQTT"],
            layer: "message",
            side: "both",
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
            prefix: "ACCESS",
            profiles: &["ACCESS"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CACHE",
            profiles: &["CACHE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "COMPRESS",
            profiles: &["HTTPCOMPRESSION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "STREAM",
            profiles: &["STREAM"],
            layer: "application",
            side: "both",
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
            prefix: "ISTATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TMM",
            profiles: &[],
            layer: "utility",
            side: "global",
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
