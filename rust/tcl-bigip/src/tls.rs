// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Product-neutral TLS projection of BIG-IP SSL profiles and virtual servers.
//!
//! BIG-IP stores an SSL listener across several object kinds: a virtual names
//! one or more client/server SSL profiles, each profile inherits from another
//! profile or a versioned factory default, and modern profiles may carry
//! several SNI certificate/key/chain bindings. This adapter resolves that
//! graph while retaining field-level evidence for every effective value.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::BuildHasher;

use serde::{Deserialize, Serialize};
use tcl_registry::event_facts::events_for_profile;
use tcl_registry::profile_defaults::{BigipVersion, profile_field_defaults};
use tcl_sslictcl::{
    Certificate, Endpoint, KeyMatch, ProtocolVersion, TlsValue, evaluate_private_key_match,
    parse_certificates,
};

use crate::model::{BigipProfile, BigipVirtualServer, ModelObject, ProfileType};
use crate::parser::{BigipConfig, parse_keyed_block_entries, parse_properties};
use crate::value::{CertKeyChain, ListItemValue};

const TLS_FIELDS: &[&str] = &[
    "ciphers",
    "cipher-group",
    "cert",
    "key",
    "chain",
    "ca-file",
    "crl-file",
    "options",
    "peer-cert-mode",
    "sni-default",
    "sni-require",
    "server-name",
    "renegotiation",
    "secure-renegotiation",
];

/// Direction in which an SSL profile terminates or originates TLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TlsProfileSide {
    /// TLS offered to clients connecting to the virtual server.
    Client,
    /// TLS used by BIG-IP when connecting to pool members.
    Server,
}

/// Origin of one effective BIG-IP SSL profile field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectiveValueOrigin {
    /// Declared directly on this profile.
    Explicit,
    /// Inherited from another configured profile.
    Inherited,
    /// Resolved from the versioned TMOS factory-default table.
    FactoryDefault,
}

/// Effective field value with the profile or factory snapshot that supplied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveTlsField {
    /// tmsh field name.
    pub field: String,
    /// Resolved value.
    pub value: String,
    /// How the value was obtained.
    pub origin: EffectiveValueOrigin,
    /// Configured profile path, or versioned factory profile name.
    pub source: String,
}

/// One named certificate/key/chain entry on an SSL profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateBinding {
    /// `cert-key-chain` item name, or `legacy` for scalar profile fields.
    pub name: String,
    /// `sys file ssl-cert` leaf reference.
    pub cert: String,
    /// `sys file ssl-key` reference.
    pub key: String,
    /// Optional `sys file ssl-cert` chain/bundle reference.
    pub chain: String,
    /// Parsed leaf plus supplied intermediates, leaf first.
    #[serde(default)]
    pub certificates: Vec<Certificate>,
    /// Verified certificate/private-key SPKI relationship, or why it could
    /// not be established from the supplied offline material.
    pub key_match: KeyMatch,
}

/// One effective client-ssl or server-ssl profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveTlsProfile {
    /// Profile full path.
    pub full_path: String,
    /// TLS direction.
    pub side: TlsProfileSide,
    /// Direct `defaults-from` value.
    pub defaults_from: String,
    /// Configured inheritance chain, nearest parent first.
    pub inheritance_chain: Vec<String>,
    /// Effective fields indexed by tmsh field name.
    pub fields: BTreeMap<String, EffectiveTlsField>,
    /// All effective certificate bindings, including multi-certificate SNI sets.
    pub certificate_bindings: Vec<CertificateBinding>,
    /// iRule events made available by this profile type.
    pub events: Vec<String>,
    /// Virtual servers that attach this profile.
    pub used_by_virtuals: Vec<String>,
}

/// One TLS endpoint variant correlated to a virtual and an attached profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualTlsEndpoint {
    /// Virtual server full path.
    pub virtual_server: String,
    /// Listener destination as canonical tmsh text.
    pub destination: String,
    /// Attached SSL profile path.
    pub profile: String,
    /// TLS direction.
    pub side: TlsProfileSide,
    /// SNI certificate binding name, when there are multiple bindings.
    pub binding: Option<String>,
    /// Product-neutral effective endpoint.
    pub endpoint: Endpoint,
    /// Certificates used by this endpoint variant, leaf first.
    #[serde(default)]
    pub certificates: Vec<Certificate>,
    /// Certificate/private-key SPKI relationship for this SNI variant.
    pub key_match: KeyMatch,
}

/// Stable non-fatal adapter notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BigipTlsNotice {
    /// Machine-readable notice kind.
    pub kind: String,
    /// Profile or virtual server path.
    pub object: String,
    /// Human-readable detail.
    pub message: String,
}

/// Complete offline TLS projection for one BIG-IP configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BigipTlsImport {
    /// Effective SSL profiles.
    pub profiles: Vec<EffectiveTlsProfile>,
    /// Per-virtual, per-profile and per-SNI endpoint variants.
    pub endpoints: Vec<VirtualTlsEndpoint>,
    /// Missing references, inheritance cycles, and intentionally unexpanded expressions.
    pub notices: Vec<BigipTlsNotice>,
}

/// Import SSL profiles and virtual-server correlations from a parsed BIG-IP config.
///
/// `certificate_pems` is keyed by the UCS member/cache path used by
/// `sys file ssl-cert`. `source-path`, full path, and leaf-name keys are also
/// accepted so bare-config callers can supply material without emulating UCS.
#[must_use]
pub fn import_bigip_tls<S: BuildHasher>(
    config: &BigipConfig,
    version: Option<BigipVersion>,
    certificate_pems: &HashMap<String, String, S>,
) -> BigipTlsImport {
    let profiles: BTreeMap<String, &BigipProfile> = config
        .objects
        .iter()
        .filter_map(|placed| match &placed.object {
            ModelObject::Profile(profile)
                if matches!(
                    profile.profile_type,
                    ProfileType::ClientSsl | ProfileType::ServerSsl
                ) =>
            {
                Some((profile.full_path.clone(), profile))
            }
            _ => None,
        })
        .collect();
    let raw_profiles = raw_profile_properties(config);
    let pem_by_object = certificate_material(config, certificate_pems);
    let key_by_object = private_key_material(config, certificate_pems);
    let mut notices = Vec::new();
    let mut effective = BTreeMap::new();

    for (path, profile) in &profiles {
        let resolved = resolve_profile(
            path,
            profile,
            &profiles,
            &raw_profiles,
            &pem_by_object,
            &key_by_object,
            version,
            &mut notices,
        );
        effective.insert(path.clone(), resolved);
    }

    let virtuals: Vec<&BigipVirtualServer> = config
        .objects
        .iter()
        .filter_map(|placed| match &placed.object {
            ModelObject::VirtualServer(virtual_server) => Some(virtual_server),
            _ => None,
        })
        .collect();
    let mut endpoints =
        import_virtual_endpoints(&virtuals, &profiles, &mut effective, version, &mut notices);

    let used: BTreeMap<String, BTreeSet<String>> =
        endpoints.iter().fold(BTreeMap::new(), |mut acc, endpoint| {
            acc.entry(endpoint.profile.clone())
                .or_default()
                .insert(endpoint.virtual_server.clone());
            acc
        });
    let mut profiles: Vec<EffectiveTlsProfile> = effective.into_values().collect();
    for profile in &mut profiles {
        profile.used_by_virtuals = used
            .get(&profile.full_path)
            .map(|names| names.iter().cloned().collect())
            .unwrap_or_default();
    }
    profiles.sort_by(|left, right| left.full_path.cmp(&right.full_path));
    endpoints.sort_by(|left, right| {
        (&left.virtual_server, &left.profile, &left.binding).cmp(&(
            &right.virtual_server,
            &right.profile,
            &right.binding,
        ))
    });
    notices.sort_by(|left, right| {
        (&left.object, &left.kind, &left.message).cmp(&(&right.object, &right.kind, &right.message))
    });
    notices.dedup();
    BigipTlsImport {
        profiles,
        endpoints,
        notices,
    }
}

fn import_virtual_endpoints(
    virtuals: &[&BigipVirtualServer],
    profiles: &BTreeMap<String, &BigipProfile>,
    effective: &mut BTreeMap<String, EffectiveTlsProfile>,
    version: Option<BigipVersion>,
    notices: &mut Vec<BigipTlsNotice>,
) -> Vec<VirtualTlsEndpoint> {
    let mut endpoints = Vec::new();
    for virtual_server in virtuals {
        for item in &virtual_server.profiles.items {
            let ListItemValue::Profile(attachment) = &item.value else {
                continue;
            };
            let Some(path) = resolve_reference(&attachment.path, profiles)
                .or_else(|| factory_profile_path(&attachment.path))
            else {
                continue;
            };
            if !effective.contains_key(&path) {
                let Some(profile) = factory_profile(&path, version, notices) else {
                    continue;
                };
                effective.insert(path.clone(), profile);
            }
            let profile = &effective[&path];
            let allowed = match profile.side {
                TlsProfileSide::Client => attachment.context != "serverside",
                TlsProfileSide::Server => attachment.context != "clientside",
            };
            if allowed {
                endpoints.extend(endpoint_variants(virtual_server, profile));
            }
        }
    }
    endpoints
}

#[allow(clippy::too_many_arguments)]
fn resolve_profile(
    path: &str,
    profile: &BigipProfile,
    profiles: &BTreeMap<String, &BigipProfile>,
    raw_profiles: &BTreeMap<String, BTreeMap<String, String>>,
    pem_by_object: &BTreeMap<String, Vec<Certificate>>,
    key_by_object: &BTreeMap<String, String>,
    version: Option<BigipVersion>,
    notices: &mut Vec<BigipTlsNotice>,
) -> EffectiveTlsProfile {
    let side = profile_side(profile.profile_type);
    let type_name = type_name(side);
    let mut fields = factory_default_fields(side, version);
    add_unknown_version_notice(path, version, notices);
    let mut inheritance_chain = Vec::new();
    let mut ancestors = Vec::new();
    let mut current = profile.defaults_from.clone();
    let mut seen = BTreeSet::from([path.to_owned()]);
    while !current.is_empty() {
        let Some(parent_path) = resolve_reference(&current, profiles) else {
            if !is_factory_profile(&current, side) {
                notices.push(BigipTlsNotice {
                    kind: "missing-parent".to_owned(),
                    object: path.to_owned(),
                    message: format!(
                        "defaults-from profile `{current}` is not present in this configuration"
                    ),
                });
            }
            break;
        };
        if !seen.insert(parent_path.clone()) {
            notices.push(BigipTlsNotice {
                kind: "inheritance-cycle".to_owned(),
                object: path.to_owned(),
                message: format!("profile inheritance loops at `{parent_path}`"),
            });
            break;
        }
        inheritance_chain.push(parent_path.clone());
        let parent = profiles[&parent_path];
        ancestors.push((parent_path, parent));
        current.clone_from(&parent.defaults_from);
    }
    ancestors.reverse();
    let mut effective_raw_properties = None;
    for (ancestor_path, ancestor) in ancestors {
        overlay_profile(
            &mut fields,
            ancestor,
            &ancestor_path,
            EffectiveValueOrigin::Inherited,
        );
        if let Some(properties) = cert_key_chain_properties(raw_profiles, &ancestor_path) {
            effective_raw_properties = Some(properties);
        }
    }
    overlay_profile(&mut fields, profile, path, EffectiveValueOrigin::Explicit);
    if let Some(properties) = cert_key_chain_properties(raw_profiles, path) {
        effective_raw_properties = Some(properties);
    }

    let certificate_bindings = resolve_bindings(
        path,
        &fields,
        effective_raw_properties,
        pem_by_object,
        key_by_object,
        notices,
    );
    add_unexpanded_cipher_notice(path, &fields, notices);

    EffectiveTlsProfile {
        full_path: path.to_owned(),
        side,
        defaults_from: profile.defaults_from.clone(),
        inheritance_chain,
        fields,
        certificate_bindings,
        events: events_for_profile(type_name)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        used_by_virtuals: Vec::new(),
    }
}

fn factory_profile(
    path: &str,
    version: Option<BigipVersion>,
    notices: &mut Vec<BigipTlsNotice>,
) -> Option<EffectiveTlsProfile> {
    let side = factory_profile_side(path)?;
    add_unknown_version_notice(path, version, notices);
    Some(EffectiveTlsProfile {
        full_path: path.to_owned(),
        side,
        defaults_from: String::new(),
        inheritance_chain: Vec::new(),
        fields: factory_default_fields(side, version),
        certificate_bindings: Vec::new(),
        events: events_for_profile(type_name(side))
            .into_iter()
            .map(str::to_owned)
            .collect(),
        used_by_virtuals: Vec::new(),
    })
}

fn factory_default_fields(
    side: TlsProfileSide,
    version: Option<BigipVersion>,
) -> BTreeMap<String, EffectiveTlsField> {
    let type_name = type_name(side);
    version
        .map(|version| profile_field_defaults(type_name, Some(version)))
        .unwrap_or_default()
        .into_iter()
        .filter(|(field, _)| TLS_FIELDS.contains(field))
        .map(|(field, value)| {
            (
                field.to_owned(),
                EffectiveTlsField {
                    field: field.to_owned(),
                    value: value.to_owned(),
                    origin: EffectiveValueOrigin::FactoryDefault,
                    source: format!("TMOS {type_name} factory default"),
                },
            )
        })
        .collect()
}

fn add_unknown_version_notice(
    path: &str,
    version: Option<BigipVersion>,
    notices: &mut Vec<BigipTlsNotice>,
) {
    if version.is_none() {
        notices.push(BigipTlsNotice {
            kind: "unknown-tmos-version".to_owned(),
            object: path.to_owned(),
            message: "TMOS version is absent or malformed; factory TLS defaults were not guessed"
                .to_owned(),
        });
    }
}

fn add_unexpanded_cipher_notice(
    path: &str,
    fields: &BTreeMap<String, EffectiveTlsField>,
    notices: &mut Vec<BigipTlsNotice>,
) {
    let expression = field_value(fields, "ciphers");
    let group = field_value(fields, "cipher-group");
    if !expression.is_empty() || (!group.is_empty() && group != "none") {
        notices.push(BigipTlsNotice {
            kind: "unexpanded-cipher-policy".to_owned(),
            object: path.to_owned(),
            message: "BIG-IP cipher expressions/groups are preserved as evidence but are not graded as literal negotiated suites"
                .to_owned(),
        });
    }
}

fn cert_key_chain_properties<'a>(
    profiles: &'a BTreeMap<String, BTreeMap<String, String>>,
    path: &str,
) -> Option<&'a BTreeMap<String, String>> {
    profiles
        .get(path)
        .filter(|properties| properties.contains_key("cert-key-chain"))
}

fn resolve_bindings(
    path: &str,
    fields: &BTreeMap<String, EffectiveTlsField>,
    raw_properties: Option<&BTreeMap<String, String>>,
    pem_by_object: &BTreeMap<String, Vec<Certificate>>,
    key_by_object: &BTreeMap<String, String>,
    notices: &mut Vec<BigipTlsNotice>,
) -> Vec<CertificateBinding> {
    let mut bindings = raw_properties
        .and_then(|properties| properties.get("cert-key-chain"))
        .map(|value| {
            parse_keyed_block_entries(value)
                .into_iter()
                .map(|(name, body)| CertKeyChain::from_raw(&name, &body))
                .filter(|binding| !binding.cert.is_empty())
                .map(|binding| {
                    materialise_binding(binding, pem_by_object, key_by_object, path, notices)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let cert = field_value(fields, "cert");
    if bindings.is_empty() && !cert.is_empty() && cert != "none" {
        bindings.push(materialise_binding(
            CertKeyChain {
                name: "legacy".to_owned(),
                cert,
                key: field_value(fields, "key"),
                chain: field_value(fields, "chain"),
                ..CertKeyChain::default()
            },
            pem_by_object,
            key_by_object,
            path,
            notices,
        ));
    }
    bindings
}

fn overlay_profile(
    fields: &mut BTreeMap<String, EffectiveTlsField>,
    profile: &BigipProfile,
    source: &str,
    origin: EffectiveValueOrigin,
) {
    for (field, value) in explicit_profile_fields(profile) {
        if value.is_empty() {
            continue;
        }
        fields.insert(
            field.to_owned(),
            EffectiveTlsField {
                field: field.to_owned(),
                value: value.to_owned(),
                origin,
                source: source.to_owned(),
            },
        );
    }
}

fn explicit_profile_fields(profile: &BigipProfile) -> [(&'static str, &str); 15] {
    [
        ("ciphers", &profile.ciphers),
        ("cipher-group", &profile.cipher_group),
        ("cert", &profile.cert),
        ("key", &profile.key),
        ("chain", &profile.chain),
        ("ca-file", &profile.ca_file),
        ("crl-file", &profile.crl_file),
        ("options", &profile.options),
        ("peer-cert-mode", &profile.peer_cert_mode),
        ("sni-default", &profile.sni_default),
        ("sni-require", &profile.sni_require),
        ("server-name", &profile.server_name),
        ("renegotiation", &profile.renegotiation),
        ("secure-renegotiation", &profile.secure_renegotiation),
        ("cert-extension-includes", &profile.cert_extension_includes),
    ]
}

fn materialise_binding(
    binding: CertKeyChain,
    pem_by_object: &BTreeMap<String, Vec<Certificate>>,
    key_by_object: &BTreeMap<String, String>,
    profile: &str,
    notices: &mut Vec<BigipTlsNotice>,
) -> CertificateBinding {
    let mut certificates = lookup_material(&binding.cert, pem_by_object).unwrap_or_default();
    if certificates.is_empty() {
        notices.push(BigipTlsNotice {
            kind: "missing-certificate-material".to_owned(),
            object: profile.to_owned(),
            message: format!(
                "certificate `{}` has no supplied PEM material",
                binding.cert
            ),
        });
    }
    if !binding.chain.is_empty()
        && binding.chain != "none"
        && let Some(chain) = lookup_material(&binding.chain, pem_by_object)
    {
        for certificate in chain {
            if !certificates
                .iter()
                .any(|known| known.fingerprint_sha256 == certificate.fingerprint_sha256)
            {
                certificates.push(certificate);
            }
        }
    }
    let key_match = match (
        certificates.first(),
        lookup_key_material(&binding.key, key_by_object),
    ) {
        (Some(certificate), Some(key)) => evaluate_private_key_match(certificate, key.as_bytes()),
        (None, _) => KeyMatch::unknown("certificate material was not supplied"),
        (_, None) if binding.key.is_empty() || binding.key == "none" => {
            KeyMatch::unknown("profile has no private-key reference")
        }
        (_, None) => KeyMatch::unknown(format!(
            "private-key material `{}` was not supplied",
            binding.key
        )),
    };
    CertificateBinding {
        name: binding.name,
        cert: binding.cert,
        key: binding.key,
        chain: binding.chain,
        certificates,
        key_match,
    }
}

fn endpoint_variants(
    virtual_server: &BigipVirtualServer,
    profile: &EffectiveTlsProfile,
) -> Vec<VirtualTlsEndpoint> {
    let destination = virtual_server
        .destination
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let bindings: Vec<Option<&CertificateBinding>> = if profile.certificate_bindings.is_empty() {
        vec![None]
    } else {
        profile.certificate_bindings.iter().map(Some).collect()
    };
    bindings
        .into_iter()
        .map(|binding| {
            let binding_name = binding.map(|binding| binding.name.clone());
            let hostname = nonempty(field_value(&profile.fields, "server-name"));
            let mut extensions = BTreeMap::new();
            extensions.insert(
                "bigip-profile".to_owned(),
                vec![TlsValue::Scalar(profile.full_path.clone())],
            );
            extensions.insert(
                "bigip-side".to_owned(),
                vec![TlsValue::Scalar(match profile.side {
                    TlsProfileSide::Client => "client".to_owned(),
                    TlsProfileSide::Server => "server".to_owned(),
                })],
            );
            for field in [
                "cipher-group",
                "peer-cert-mode",
                "sni-default",
                "sni-require",
            ] {
                let value = field_value(&profile.fields, field);
                if !value.is_empty() {
                    extensions.insert(field.to_owned(), vec![TlsValue::Scalar(value)]);
                }
            }
            let cipher_expression = field_value(&profile.fields, "ciphers");
            if !cipher_expression.is_empty() {
                extensions.insert(
                    "bigip-cipher-expression".to_owned(),
                    vec![TlsValue::Scalar(cipher_expression)],
                );
            }
            if let Some(options) = profile.fields.get("options") {
                extensions.insert(
                    "bigip-options-origin".to_owned(),
                    vec![TlsValue::Scalar(format!(
                        "{}:{}",
                        effective_origin(options.origin),
                        options.source
                    ))],
                );
            }
            let certificates = binding
                .map(|binding| binding.certificates.clone())
                .unwrap_or_default();
            let key_match = binding.map_or_else(
                || KeyMatch::unknown("endpoint has no certificate/key binding"),
                |binding| binding.key_match.clone(),
            );
            VirtualTlsEndpoint {
                virtual_server: virtual_server.full_path.clone(),
                destination: destination.clone(),
                profile: profile.full_path.clone(),
                side: profile.side,
                binding: binding_name,
                endpoint: Endpoint {
                    name: format!("{}:{}", virtual_server.full_path, profile.full_path),
                    hostname,
                    protocols: protocols_from_profile(profile),
                    // BIG-IP uses an OpenSSL expression or a cipher-group
                    // reference here, not an enumerated negotiated-suite set.
                    // Preserve the expression above and leave this unknown so
                    // the estimator cannot award it a literal-suite score.
                    ciphers: Vec::new(),
                    certificate_chain: certificates
                        .iter()
                        .map(|certificate| certificate.fingerprint_sha256.clone())
                        .collect(),
                    extensions,
                    ..Endpoint::default()
                },
                certificates,
                key_match,
            }
        })
        .collect()
}

fn raw_profile_properties(config: &BigipConfig) -> BTreeMap<String, BTreeMap<String, String>> {
    config
        .generic_objects
        .iter()
        .filter(|(_, object)| {
            object.module == "ltm"
                && matches!(
                    object.object_type.as_str(),
                    "profile client-ssl" | "profile server-ssl"
                )
        })
        .map(|(_, object)| {
            (
                object.identifier.clone(),
                parse_properties(&object.body).into_iter().collect(),
            )
        })
        .collect()
}

fn certificate_material<S: BuildHasher>(
    config: &BigipConfig,
    certificate_pems: &HashMap<String, String, S>,
) -> BTreeMap<String, Vec<Certificate>> {
    let mut material = BTreeMap::new();
    for placed in &config.objects {
        let ModelObject::SysFileSslCert(certificate) = &placed.object else {
            continue;
        };
        let candidates = [
            certificate.cache_path.as_str(),
            certificate
                .source_path
                .strip_prefix("file://")
                .unwrap_or(&certificate.source_path),
            certificate.full_path.as_str(),
            certificate.name.as_str(),
        ];
        let parsed = candidates
            .into_iter()
            .filter(|key| !key.is_empty())
            .find_map(|key| certificate_pems.get(key))
            .and_then(|pem| parse_certificates(pem.as_bytes()).ok())
            .unwrap_or_default();
        if parsed.is_empty() {
            continue;
        }
        for key in candidates.into_iter().filter(|key| !key.is_empty()) {
            material.insert(key.to_owned(), parsed.clone());
        }
        material.insert(placed.full_path.clone(), parsed);
    }
    material
}

fn private_key_material<S: BuildHasher>(
    config: &BigipConfig,
    supplied_material: &HashMap<String, String, S>,
) -> BTreeMap<String, String> {
    let mut material = BTreeMap::new();
    for placed in &config.objects {
        let ModelObject::SysFileSslKey(key) = &placed.object else {
            continue;
        };
        let candidates = [
            key.cache_path.as_str(),
            key.source_path
                .strip_prefix("file://")
                .unwrap_or(&key.source_path),
            key.full_path.as_str(),
            key.name.as_str(),
        ];
        let Some(pem) = candidates
            .into_iter()
            .filter(|candidate| !candidate.is_empty())
            .find_map(|candidate| supplied_material.get(candidate))
        else {
            continue;
        };
        for candidate in candidates
            .into_iter()
            .filter(|candidate| !candidate.is_empty())
        {
            material.insert(candidate.to_owned(), pem.clone());
        }
        material.insert(placed.full_path.clone(), pem.clone());
    }
    material
}

fn lookup_material(
    reference: &str,
    material: &BTreeMap<String, Vec<Certificate>>,
) -> Option<Vec<Certificate>> {
    material.get(reference).cloned().or_else(|| {
        let leaf = reference.rsplit('/').next().unwrap_or(reference);
        material
            .iter()
            .find(|(path, _)| path.rsplit('/').next().unwrap_or(path) == leaf)
            .map(|(_, certificates)| certificates.clone())
    })
}

fn lookup_key_material<'a>(
    reference: &str,
    material: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    material.get(reference).map(String::as_str).or_else(|| {
        let leaf = reference.rsplit('/').next().unwrap_or(reference);
        material
            .iter()
            .find(|(path, _)| path.rsplit('/').next().unwrap_or(path.as_str()) == leaf)
            .map(|(_, key)| key.as_str())
    })
}

fn resolve_reference<T>(reference: &str, objects: &BTreeMap<String, T>) -> Option<String> {
    if objects.contains_key(reference) {
        return Some(reference.to_owned());
    }
    let leaf = reference.rsplit('/').next().unwrap_or(reference);
    let matches: Vec<&String> = objects
        .keys()
        .filter(|path| path.rsplit('/').next().unwrap_or(path) == leaf)
        .collect();
    (matches.len() == 1).then(|| matches[0].clone())
}

fn profile_side(profile_type: ProfileType) -> TlsProfileSide {
    match profile_type {
        ProfileType::ClientSsl => TlsProfileSide::Client,
        ProfileType::ServerSsl => TlsProfileSide::Server,
        _ => unreachable!("only SSL profiles reach TLS projection"),
    }
}

const fn type_name(side: TlsProfileSide) -> &'static str {
    match side {
        TlsProfileSide::Client => "CLIENTSSL",
        TlsProfileSide::Server => "SERVERSSL",
    }
}

fn is_factory_profile(reference: &str, side: TlsProfileSide) -> bool {
    factory_profile_side(reference).is_some_and(|factory_side| factory_side == side)
}

fn factory_profile_path(reference: &str) -> Option<String> {
    factory_profile_side(reference).map(|side| match side {
        TlsProfileSide::Client => "/Common/clientssl".to_owned(),
        TlsProfileSide::Server => "/Common/serverssl".to_owned(),
    })
}

fn factory_profile_side(reference: &str) -> Option<TlsProfileSide> {
    match reference {
        "clientssl" | "/Common/clientssl" => Some(TlsProfileSide::Client),
        "serverssl" | "/Common/serverssl" => Some(TlsProfileSide::Server),
        _ => None,
    }
}

fn field_value(fields: &BTreeMap<String, EffectiveTlsField>, name: &str) -> String {
    fields
        .get(name)
        .map(|field| normalise_scalar(&field.value))
        .unwrap_or_default()
}

fn normalise_scalar(value: &str) -> String {
    value
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(value.trim())
        .trim()
        .to_owned()
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty() && value != "none").then_some(value)
}

fn protocols_from_options(options: &str) -> Vec<ProtocolVersion> {
    let lower = normalise_scalar(options).to_ascii_lowercase();
    // Options may contain Tcl substitution in generated configuration.  A
    // static projection must not interpret a partial literal and then award a
    // protocol score for a value whose effective contents are runtime data.
    if lower.is_empty()
        || lower.contains('$')
        || lower.contains('[')
        || lower.contains(']')
        || lower.contains(';')
    {
        return Vec::new();
    }
    let disabled: BTreeSet<&str> = lower.split_whitespace().collect();
    [
        ("no-sslv2", ProtocolVersion::Ssl2),
        ("no-sslv3", ProtocolVersion::Ssl3),
        ("no-tlsv1", ProtocolVersion::Tls10),
        ("no-tlsv1.1", ProtocolVersion::Tls11),
        ("no-tlsv1.2", ProtocolVersion::Tls12),
        ("no-tlsv1.3", ProtocolVersion::Tls13),
    ]
    .into_iter()
    .filter_map(|(option, protocol)| {
        let broad_disable = match protocol {
            ProtocolVersion::Ssl2 | ProtocolVersion::Ssl3 => disabled.contains("no-ssl"),
            ProtocolVersion::Tls10
            | ProtocolVersion::Tls11
            | ProtocolVersion::Tls12
            | ProtocolVersion::Tls13 => disabled.contains("no-tls"),
        };
        (!broad_disable && !disabled.contains(option)).then_some(protocol)
    })
    .collect()
}

fn protocols_from_profile(profile: &EffectiveTlsProfile) -> Vec<ProtocolVersion> {
    profile
        .fields
        .get("options")
        .map_or_else(Vec::new, |field| {
            protocols_from_options(&normalise_scalar(&field.value))
        })
}

const fn effective_origin(origin: EffectiveValueOrigin) -> &'static str {
    match origin {
        EffectiveValueOrigin::Explicit => "explicit",
        EffectiveValueOrigin::Inherited => "inherited",
        EffectiveValueOrigin::FactoryDefault => "factory-default",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_bigip_conf;

    #[test]
    fn resolves_inheritance_multi_sni_and_event_facts() {
        let source = r"
ltm profile client-ssl tls-base {
    defaults-from /Common/clientssl
    ciphers HIGH:!RSA
    options { no-tlsv1 no-tlsv1.1 }
}
ltm profile client-ssl tls-sni {
    defaults-from /Common/tls-base
    sni-default true
    cert-key-chain {
        one { cert /Common/one.crt key /Common/one.key chain /Common/roots.crt }
        two { cert /Common/two.crt key /Common/two.key }
    }
}
ltm virtual https {
    destination /Common/192.0.2.1:443
    profiles { /Common/tls-sni { context clientside } }
}
sys file ssl-cert one.crt { cache-path /config/ssl/ssl.crt/one.crt }
sys file ssl-cert two.crt { cache-path /config/ssl/ssl.crt/two.crt }
sys file ssl-cert roots.crt { cache-path /config/ssl/ssl.crt/roots.crt }
";
        let config = parse_bigip_conf(source, "Common");
        let imported = import_bigip_tls(&config, BigipVersion::parse("17.1"), &HashMap::new());
        let profile = imported
            .profiles
            .iter()
            .find(|profile| profile.full_path == "/Common/tls-sni")
            .expect("SNI profile");
        assert_eq!(profile.inheritance_chain, ["/Common/tls-base"]);
        assert_eq!(profile.fields["ciphers"].value, "HIGH:!RSA");
        assert_eq!(
            profile.fields["ciphers"].origin,
            EffectiveValueOrigin::Inherited
        );
        assert_eq!(profile.certificate_bindings.len(), 2);
        assert!(
            profile
                .events
                .iter()
                .any(|event| event == "CLIENTSSL_HANDSHAKE")
        );
        assert_eq!(imported.endpoints.len(), 2);
        assert!(imported.endpoints.iter().all(|endpoint| {
            endpoint.endpoint.protocols
                == [
                    ProtocolVersion::Ssl2,
                    ProtocolVersion::Ssl3,
                    ProtocolVersion::Tls12,
                    ProtocolVersion::Tls13,
                ]
        }));
        assert!(
            imported
                .endpoints
                .iter()
                .all(|endpoint| endpoint.endpoint.ciphers.is_empty())
        );
    }

    #[test]
    fn inherits_cert_key_chain_bindings_from_parent_profile() {
        let source = r"
ltm profile client-ssl parent {
    defaults-from /Common/clientssl
    cert-key-chain {
        inherited { cert /Common/inherited.crt key /Common/inherited.key }
    }
}
ltm profile client-ssl child { defaults-from /Common/parent }
ltm virtual https {
    destination /Common/192.0.2.1:443
    profiles { /Common/child { context clientside } }
}
";
        let config = parse_bigip_conf(source, "Common");
        let imported = import_bigip_tls(&config, BigipVersion::parse("17.1"), &HashMap::new());
        let child = imported
            .profiles
            .iter()
            .find(|profile| profile.full_path == "/Common/child")
            .expect("child profile");
        assert_eq!(child.certificate_bindings.len(), 1);
        assert_eq!(child.certificate_bindings[0].name, "inherited");
        assert_eq!(imported.endpoints.len(), 1);
        assert_eq!(imported.endpoints[0].binding.as_deref(), Some("inherited"));
    }

    #[test]
    fn detects_profile_inheritance_cycles() {
        let source = r"
ltm profile client-ssl a { defaults-from /Common/b }
ltm profile client-ssl b { defaults-from /Common/a }
";
        let config = parse_bigip_conf(source, "Common");
        let imported = import_bigip_tls(&config, None, &HashMap::new());
        assert!(
            imported
                .notices
                .iter()
                .any(|notice| notice.kind == "inheritance-cycle")
        );
    }

    #[test]
    fn absent_version_and_options_leave_protocols_unknown() {
        let source = r"
ltm profile client-ssl custom { defaults-from /Common/clientssl }
ltm virtual https {
    destination /Common/192.0.2.1:443
    profiles { /Common/custom { context clientside } }
}
";
        let config = parse_bigip_conf(source, "Common");
        let imported = import_bigip_tls(&config, None, &HashMap::new());
        assert!(imported.endpoints[0].endpoint.protocols.is_empty());
        assert!(imported.notices.iter().any(|notice| {
            notice.kind == "unknown-tmos-version" && notice.object == "/Common/custom"
        }));
    }

    #[test]
    fn projects_direct_factory_profile_attachments_with_unknown_key_evidence() {
        let source = r"
ltm virtual https {
    destination /Common/192.0.2.1:443
    profiles { /Common/clientssl { context clientside } }
}
";
        let config = parse_bigip_conf(source, "Common");
        let imported = import_bigip_tls(&config, BigipVersion::parse("17.1"), &HashMap::new());
        let profile = imported
            .profiles
            .iter()
            .find(|profile| profile.full_path == "/Common/clientssl")
            .expect("factory profile is projected");
        assert_eq!(profile.side, TlsProfileSide::Client);
        assert_eq!(
            profile.fields["options"].origin,
            EffectiveValueOrigin::FactoryDefault
        );
        assert_eq!(profile.used_by_virtuals, ["/Common/https"]);
        assert_eq!(imported.endpoints.len(), 1);
        assert!(imported.endpoints[0].certificates.is_empty());
        assert_eq!(
            imported.endpoints[0].key_match.status,
            tcl_sslictcl::KeyMatchStatus::Unknown
        );

        let mutated = source.replace("/Common/clientssl", "/Common/not-a-factory-profile");
        let mutated_config = parse_bigip_conf(&mutated, "Common");
        let mutated_import = import_bigip_tls(
            &mutated_config,
            BigipVersion::parse("17.1"),
            &HashMap::new(),
        );
        assert!(mutated_import.endpoints.is_empty());
    }

    #[test]
    fn dynamic_options_do_not_project_a_partial_protocol_set() {
        assert!(protocols_from_options("no-tlsv1 $runtime_options").is_empty());
        assert!(protocols_from_options("[profile::options]").is_empty());
    }
}
