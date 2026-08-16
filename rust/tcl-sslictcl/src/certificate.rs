// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Rich, owned X.509 projection used by the offline analysis engine.

use std::error::Error;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x509_parser::extensions::{DistributionPointName, GeneralName, ParsedExtension};
use x509_parser::pem::Pem;
use x509_parser::prelude::{X509Certificate, parse_x509_certificate};
use x509_parser::public_key::PublicKey;

/// An owned certificate projection. Raw DER is retained for path signature
/// verification but deliberately omitted from serialized report models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificate {
    /// SHA-256 of the complete DER certificate, lowercase hexadecimal.
    pub fingerprint_sha256: String,
    /// SHA-256 of `SubjectPublicKeyInfo` DER, lowercase hexadecimal.
    pub spki_sha256: String,
    /// RFC4514-like subject name.
    pub subject: String,
    /// RFC4514-like issuer name.
    pub issuer: String,
    /// Subject common names, retained for inventory (not hostname fallback).
    pub common_names: Vec<String>,
    /// Certificate serial as uppercase hexadecimal.
    pub serial: String,
    /// Seconds since Unix epoch.
    pub not_before: i64,
    /// Seconds since Unix epoch.
    pub not_after: i64,
    /// Public-key algorithm OID.
    pub public_key_algorithm: String,
    /// Public key strength where it can be derived.
    pub public_key_bits: Option<u32>,
    /// Signature algorithm OID.
    pub signature_algorithm: String,
    /// Subject key identifier, lowercase hexadecimal.
    pub subject_key_id: Option<String>,
    /// Authority key identifier, lowercase hexadecimal.
    pub authority_key_id: Option<String>,
    /// DNS subject alternative names.
    pub dns_names: Vec<String>,
    /// IP subject alternative names.
    pub ip_addresses: Vec<String>,
    /// Key Usage names.
    pub key_usage: Vec<String>,
    /// Extended Key Usage OIDs.
    pub extended_key_usage: Vec<String>,
    /// Whether Basic Constraints marks the certificate as a CA.
    pub is_ca: bool,
    /// Basic Constraints path length.
    pub path_len_constraint: Option<u32>,
    /// Certificate Policy OIDs.
    pub policy_oids: Vec<String>,
    /// Permitted name subtrees rendered as stable strings.
    pub permitted_name_subtrees: Vec<String>,
    /// Excluded name subtrees rendered as stable strings.
    pub excluded_name_subtrees: Vec<String>,
    /// OCSP responder URIs.
    pub ocsp_uris: Vec<String>,
    /// CA Issuers URIs.
    pub ca_issuer_uris: Vec<String>,
    /// CRL distribution point URIs.
    pub crl_uris: Vec<String>,
    /// Critical extension OIDs the parser could not understand.
    pub unknown_critical_extensions: Vec<String>,
    /// Complete DER, used only while analysing.
    #[serde(skip)]
    pub(crate) raw_der: Vec<u8>,
}

impl Certificate {
    /// True when the subject and issuer names are identical.
    #[must_use]
    pub fn is_self_issued(&self) -> bool {
        normalise_name(&self.subject) == normalise_name(&self.issuer)
    }

    /// True when `hostname` is covered by a DNS or IP SAN.
    ///
    /// Common Name fallback is intentionally not implemented: modern Web PKI
    /// clients require SAN, and silently accepting CN would overstate trust.
    #[must_use]
    pub fn covers_hostname(&self, hostname: &str) -> bool {
        if hostname.parse::<std::net::IpAddr>().is_ok() {
            return self.ip_addresses.iter().any(|name| name == hostname);
        }
        self.dns_names
            .iter()
            .any(|pattern| dns_pattern_matches(pattern, hostname))
    }

    /// Whether this certificate was valid at `unix_time`.
    #[must_use]
    pub const fn valid_at(&self, unix_time: i64) -> bool {
        self.not_before <= unix_time && unix_time <= self.not_after
    }

    /// Whether Key Usage, if present, permits certificate signing.
    #[must_use]
    pub fn permits_cert_signing(&self) -> bool {
        self.key_usage.is_empty() || self.key_usage.iter().any(|usage| usage == "keyCertSign")
    }

    /// Whether Extended Key Usage, if present, permits a TLS server leaf.
    #[must_use]
    pub fn permits_server_auth(&self) -> bool {
        self.extended_key_usage.is_empty()
            || self
                .extended_key_usage
                .iter()
                .any(|usage| usage == "2.5.29.37.0" || usage == "1.3.6.1.5.5.7.3.1")
    }
}

/// Certificate parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateError {
    /// Human-readable detail, containing no certificate material.
    pub message: String,
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CertificateError {}

/// Parse one DER certificate or every `CERTIFICATE` block in a PEM bundle.
pub fn parse_certificates(input: &[u8]) -> Result<Vec<Certificate>, CertificateError> {
    const PEM_BEGIN: &[u8] = b"-----BEGIN";
    if input
        .windows(PEM_BEGIN.len())
        .any(|window| window == PEM_BEGIN)
    {
        let mut certificates = Vec::new();
        for pem in Pem::iter_from_buffer(input) {
            let pem = pem.map_err(|error| CertificateError {
                message: format!("invalid PEM: {error}"),
            })?;
            if pem.label == "CERTIFICATE" || pem.label == "X509 CERTIFICATE" {
                certificates.push(parse_der(&pem.contents)?);
            }
        }
        if certificates.is_empty() {
            return Err(CertificateError {
                message: "PEM contains no CERTIFICATE blocks".to_owned(),
            });
        }
        Ok(certificates)
    } else {
        Ok(vec![parse_der(input)?])
    }
}

fn parse_der(input: &[u8]) -> Result<Certificate, CertificateError> {
    let (remaining, certificate) =
        parse_x509_certificate(input).map_err(|error| CertificateError {
            message: format!("invalid X.509 DER: {error}"),
        })?;
    if !remaining.is_empty() {
        return Err(CertificateError {
            message: format!("X.509 DER has {} trailing byte(s)", remaining.len()),
        });
    }
    project(&certificate, input)
}

// Keeping the extension projection in one exhaustive match makes critical-
// extension handling auditable alongside every supported extension.
#[allow(clippy::too_many_lines)]
fn project(cert: &X509Certificate<'_>, raw_der: &[u8]) -> Result<Certificate, CertificateError> {
    // Reject duplicate extensions. Selecting the first occurrence is unsafe
    // because attackers can place incompatible critical values later.
    cert.extensions_map().map_err(|error| CertificateError {
        message: format!("invalid X.509 extensions: {error}"),
    })?;

    let spki = cert.public_key();
    let (public_key_algorithm, public_key_bits) = key_algorithm_and_bits(spki);
    let mut certificate = Certificate {
        fingerprint_sha256: hex_digest(raw_der),
        spki_sha256: hex_digest(spki.raw),
        subject: cert.subject().to_string(),
        issuer: cert.issuer().to_string(),
        common_names: cert
            .subject()
            .iter_attributes()
            .filter(|attribute| attribute.attr_type().to_id_string() == "2.5.4.3")
            .filter_map(|attribute| attribute.as_str().ok().map(str::to_owned))
            .collect(),
        serial: hex_upper(cert.raw_serial()),
        not_before: cert.validity().not_before.timestamp(),
        not_after: cert.validity().not_after.timestamp(),
        public_key_algorithm,
        public_key_bits,
        signature_algorithm: cert.signature_algorithm.algorithm.to_id_string(),
        subject_key_id: None,
        authority_key_id: None,
        dns_names: Vec::new(),
        ip_addresses: Vec::new(),
        key_usage: Vec::new(),
        extended_key_usage: Vec::new(),
        is_ca: false,
        path_len_constraint: None,
        policy_oids: Vec::new(),
        permitted_name_subtrees: Vec::new(),
        excluded_name_subtrees: Vec::new(),
        ocsp_uris: Vec::new(),
        ca_issuer_uris: Vec::new(),
        crl_uris: Vec::new(),
        unknown_critical_extensions: Vec::new(),
        raw_der: raw_der.to_vec(),
    };

    for extension in cert.extensions() {
        match extension.parsed_extension() {
            ParsedExtension::SubjectKeyIdentifier(identifier) => {
                certificate.subject_key_id = Some(hex_lower(identifier.0));
            }
            ParsedExtension::AuthorityKeyIdentifier(identifier) => {
                certificate.authority_key_id = identifier
                    .key_identifier
                    .as_ref()
                    .map(|identifier| hex_lower(identifier.0));
            }
            ParsedExtension::KeyUsage(usage) => {
                collect_key_usage(*usage, &mut certificate.key_usage);
            }
            ParsedExtension::ExtendedKeyUsage(usage) => {
                collect_extended_key_usage(usage, &mut certificate.extended_key_usage);
            }
            ParsedExtension::BasicConstraints(constraints) => {
                certificate.is_ca = constraints.ca;
                certificate.path_len_constraint = constraints.path_len_constraint;
            }
            ParsedExtension::SubjectAlternativeName(names) => {
                for name in &names.general_names {
                    match name {
                        GeneralName::DNSName(name) => {
                            certificate.dns_names.push((*name).to_owned());
                        }
                        GeneralName::IPAddress(bytes) => {
                            if let Some(address) = ip_string(bytes) {
                                certificate.ip_addresses.push(address);
                            }
                        }
                        _ => {}
                    }
                }
            }
            ParsedExtension::CertificatePolicies(policies) => {
                certificate.policy_oids.extend(
                    policies
                        .iter()
                        .map(|policy| policy.policy_id.to_id_string()),
                );
            }
            ParsedExtension::NameConstraints(constraints) => {
                if let Some(permitted) = &constraints.permitted_subtrees {
                    certificate.permitted_name_subtrees.extend(
                        permitted
                            .iter()
                            .map(|subtree| general_name_string(&subtree.base)),
                    );
                }
                if let Some(excluded) = &constraints.excluded_subtrees {
                    certificate.excluded_name_subtrees.extend(
                        excluded
                            .iter()
                            .map(|subtree| general_name_string(&subtree.base)),
                    );
                }
            }
            ParsedExtension::AuthorityInfoAccess(access) => {
                for description in &access.accessdescs {
                    let Some(uri) = general_name_uri(&description.access_location) else {
                        continue;
                    };
                    match description.access_method.to_id_string().as_str() {
                        "1.3.6.1.5.5.7.48.1" => certificate.ocsp_uris.push(uri),
                        "1.3.6.1.5.5.7.48.2" => certificate.ca_issuer_uris.push(uri),
                        _ => {}
                    }
                }
            }
            ParsedExtension::CRLDistributionPoints(points) => {
                for point in &points.points {
                    let Some(DistributionPointName::FullName(names)) = &point.distribution_point
                    else {
                        continue;
                    };
                    certificate
                        .crl_uris
                        .extend(names.iter().filter_map(general_name_uri));
                }
            }
            ParsedExtension::UnsupportedExtension { .. } | ParsedExtension::ParseError { .. }
                if extension.critical =>
            {
                certificate
                    .unknown_critical_extensions
                    .push(extension.oid.to_id_string());
            }
            _ => {}
        }
    }

    Ok(certificate)
}

fn key_algorithm_and_bits(
    spki: &x509_parser::x509::SubjectPublicKeyInfo<'_>,
) -> (String, Option<u32>) {
    let oid = spki.algorithm.algorithm.to_id_string();
    let bits = match spki.parsed() {
        Ok(PublicKey::RSA(key)) => u32::try_from(key.key_size()).ok(),
        Ok(PublicKey::EC(key)) => {
            ec_curve_bits(spki).or_else(|| u32::try_from(key.key_size()).ok())
        }
        _ => None,
    };
    (oid, bits)
}

fn ec_curve_bits(spki: &x509_parser::x509::SubjectPublicKeyInfo<'_>) -> Option<u32> {
    let oid = spki.algorithm.parameters.as_ref()?.as_oid().ok()?;
    match oid.to_id_string().as_str() {
        "1.2.840.10045.3.1.1" => Some(192),
        "1.3.132.0.33" => Some(224),
        "1.2.840.10045.3.1.7" | "1.3.132.0.10" => Some(256),
        "1.3.132.0.34" => Some(384),
        "1.3.132.0.35" => Some(521),
        _ => None,
    }
}

fn collect_key_usage(usage: x509_parser::extensions::KeyUsage, output: &mut Vec<String>) {
    let flags = [
        (usage.digital_signature(), "digitalSignature"),
        (usage.non_repudiation(), "nonRepudiation"),
        (usage.key_encipherment(), "keyEncipherment"),
        (usage.data_encipherment(), "dataEncipherment"),
        (usage.key_agreement(), "keyAgreement"),
        (usage.key_cert_sign(), "keyCertSign"),
        (usage.crl_sign(), "cRLSign"),
        (usage.encipher_only(), "encipherOnly"),
        (usage.decipher_only(), "decipherOnly"),
    ];
    output.extend(
        flags
            .into_iter()
            .filter(|(enabled, _)| *enabled)
            .map(|(_, name)| name.to_owned()),
    );
}

fn collect_extended_key_usage(
    usage: &x509_parser::extensions::ExtendedKeyUsage<'_>,
    output: &mut Vec<String>,
) {
    let flags = [
        (usage.any, "2.5.29.37.0"),
        (usage.server_auth, "1.3.6.1.5.5.7.3.1"),
        (usage.client_auth, "1.3.6.1.5.5.7.3.2"),
        (usage.code_signing, "1.3.6.1.5.5.7.3.3"),
        (usage.email_protection, "1.3.6.1.5.5.7.3.4"),
        (usage.time_stamping, "1.3.6.1.5.5.7.3.8"),
        (usage.ocsp_signing, "1.3.6.1.5.5.7.3.9"),
    ];
    output.extend(
        flags
            .into_iter()
            .filter(|(enabled, _)| *enabled)
            .map(|(_, oid)| oid.to_owned()),
    );
    for oid in &usage.other {
        output.push(oid.to_id_string());
    }
}

fn general_name_uri(name: &GeneralName<'_>) -> Option<String> {
    match name {
        GeneralName::URI(uri) => Some((*uri).to_owned()),
        _ => None,
    }
}

fn general_name_string(name: &GeneralName<'_>) -> String {
    match name {
        GeneralName::DNSName(name) => format!("dns:{name}"),
        GeneralName::IPAddress(bytes) => {
            format!(
                "ip:{}",
                ip_string(bytes).unwrap_or_else(|| hex_lower(bytes))
            )
        }
        GeneralName::RFC822Name(name) => format!("email:{name}"),
        GeneralName::URI(uri) => format!("uri:{uri}"),
        GeneralName::DirectoryName(name) => format!("dn:{name}"),
        other => format!("{other:?}"),
    }
}

fn ip_string(bytes: &[u8]) -> Option<String> {
    match bytes {
        [a, b, c, d] => Some(Ipv4Addr::new(*a, *b, *c, *d).to_string()),
        bytes if bytes.len() == 16 => {
            let mut octets = [0; 16];
            octets.copy_from_slice(bytes);
            Some(Ipv6Addr::from(octets).to_string())
        }
        _ => None,
    }
}

fn dns_pattern_matches(pattern: &str, hostname: &str) -> bool {
    let pattern = pattern.trim_end_matches('.').to_ascii_lowercase();
    let hostname = hostname.trim_end_matches('.').to_ascii_lowercase();
    if pattern == hostname {
        return true;
    }
    let Some(suffix) = pattern.strip_prefix("*.") else {
        return false;
    };
    let Some(prefix) = hostname.strip_suffix(suffix) else {
        return false;
    };
    prefix.ends_with('.') && !prefix[..prefix.len() - 1].contains('.')
}

pub(crate) fn signed_by(child: &Certificate, issuer: &Certificate) -> Result<bool, String> {
    let (_, child_cert) =
        parse_x509_certificate(&child.raw_der).map_err(|error| error.to_string())?;
    let (_, issuer_cert) =
        parse_x509_certificate(&issuer.raw_der).map_err(|error| error.to_string())?;
    Ok(child_cert
        .verify_signature(Some(issuer_cert.public_key()))
        .is_ok())
}

pub(crate) fn normalise_name(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02X}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matches_exactly_one_label() {
        assert!(dns_pattern_matches("*.example.test", "www.example.test"));
        assert!(!dns_pattern_matches("*.example.test", "example.test"));
        assert!(!dns_pattern_matches("*.example.test", "a.b.example.test"));
        assert!(dns_pattern_matches("EXAMPLE.TEST", "example.test."));
    }

    #[test]
    fn bad_der_is_an_error() {
        assert!(parse_certificates(b"not a certificate").is_err());
    }
}
