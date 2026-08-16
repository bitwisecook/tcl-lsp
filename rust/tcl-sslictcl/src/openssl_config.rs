// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Static OpenSSL configuration adapter.
//!
//! Sections and literal assignments are parsed from the supplied text only.
//! Includes and variable references are retained, but never resolved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config_adapter::{ConfigError, ConfigEvidence, ConfigNotice};
use crate::model::{Endpoint, ProtocolVersion, TlsValue};

/// TLS state projected from one OpenSSL configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSslConfigImport {
    /// Effective static TLS configuration.
    pub endpoint: Endpoint,
    /// Source assignments and their effective/overridden state.
    pub evidence: Vec<ConfigEvidence>,
    /// Non-fatal constructs that were retained but not interpreted.
    pub notices: Vec<ConfigNotice>,
    /// Unsupported assignments and directives, retained by section.
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
    /// Exact input, including comments and unsupported syntax.
    pub raw: String,
}

#[derive(Debug, Clone)]
struct Assignment {
    section: String,
    key: String,
    value: String,
    line: u32,
    dynamic: bool,
}

/// Parse OpenSSL configuration text without resolving references or files.
pub fn import_openssl_config(source: &str) -> Result<OpenSslConfigImport, ConfigError> {
    let assignments = parse_assignments(source)?;
    let mut notices = Vec::new();
    let mut evidence = Vec::new();

    let min_protocol = record_last(&assignments, "MinProtocol", &mut evidence);
    let max_protocol = record_last(&assignments, "MaxProtocol", &mut evidence);
    let cipher_string = record_last(&assignments, "CipherString", &mut evidence);
    let cipher_suites = record_last(&assignments, "Ciphersuites", &mut evidence);
    let groups = record_last(&assignments, "Groups", &mut evidence);
    let signatures = record_last(&assignments, "SignatureAlgorithms", &mut evidence);
    let certificate = record_last(&assignments, "Certificate", &mut evidence);

    let section = effective_section(&[
        min_protocol,
        max_protocol,
        cipher_string,
        cipher_suites,
        groups,
        signatures,
        certificate,
    ]);
    notice_cross_section_projection(&evidence, &section, &mut notices);

    let mut endpoint = Endpoint {
        name: format!("openssl:{section}"),
        ..Endpoint::default()
    };
    for item in &mut evidence {
        item.target = Some(endpoint.name.clone());
    }
    endpoint.protocols = project_protocol_range(min_protocol, max_protocol, &section, &mut notices);
    endpoint.ciphers = project_colon_assignments(
        [cipher_string, cipher_suites].into_iter().flatten(),
        &mut notices,
    );
    endpoint.groups = project_colon_assignments(groups.into_iter(), &mut notices);
    endpoint.signature_schemes = project_colon_assignments(signatures.into_iter(), &mut notices);
    endpoint.certificate_chain = project_certificate(certificate, &mut notices);

    let extensions = collect_extensions(&assignments, &mut notices);
    Ok(OpenSslConfigImport {
        endpoint,
        evidence,
        notices,
        extensions,
        raw: source.to_owned(),
    })
}

fn parse_assignments(source: &str) -> Result<Vec<Assignment>, ConfigError> {
    let logical_lines = logical_lines(source)?;
    let mut section = "default".to_owned();
    let mut assignments = Vec::new();

    for (line, text) in logical_lines {
        let uncommented = strip_comment(&text, line)?;
        let trimmed = uncommented.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('[') {
            section = parse_section(trimmed, line)?;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix(".include") {
            let value = value
                .trim()
                .strip_prefix('=')
                .map_or(value.trim(), str::trim);
            assignments.push(Assignment {
                section: section.clone(),
                key: ".include".to_owned(),
                value: normalise_value(value),
                line,
                dynamic: contains_reference(value),
            });
            continue;
        }
        if let Some((key, value)) = split_assignment(trimmed) {
            let value = normalise_value(value);
            assignments.push(Assignment {
                section: section.clone(),
                key: key.trim().to_owned(),
                dynamic: contains_reference(&value),
                value,
                line,
            });
        } else {
            assignments.push(Assignment {
                section: section.clone(),
                key: "_unparsed".to_owned(),
                value: trimmed.to_owned(),
                line,
                dynamic: contains_reference(trimmed),
            });
        }
    }
    Ok(assignments)
}

fn logical_lines(source: &str) -> Result<Vec<(u32, String)>, ConfigError> {
    let mut output = Vec::new();
    let mut pending = String::new();
    let mut pending_line = 1;
    let mut continuing = false;

    for (index, physical) in source.lines().enumerate() {
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if pending.is_empty() {
            pending_line = line;
        }
        let continued = has_continuation(physical);
        continuing = continued;
        let fragment = if continued {
            physical.trim_end().strip_suffix('\\').unwrap_or(physical)
        } else {
            physical
        };
        pending.push_str(fragment);
        if continued {
            continue;
        }
        output.push((pending_line, std::mem::take(&mut pending)));
    }
    if continuing {
        return Err(ConfigError {
            line: pending_line,
            message: "trailing OpenSSL line continuation".to_owned(),
        });
    }
    Ok(output)
}

fn has_continuation(line: &str) -> bool {
    line.trim_end()
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count()
        % 2
        == 1
}

fn strip_comment(line: &str, line_number: u32) -> Result<String, ConfigError> {
    let mut output = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' => {
                output.push(character);
                escaped = true;
            }
            '\'' | '"' if quote.is_none() => {
                quote = Some(character);
                output.push(character);
            }
            value if quote == Some(value) => {
                quote = None;
                output.push(character);
            }
            '#' if quote.is_none() => break,
            _ => output.push(character),
        }
    }
    if quote.is_some() {
        Err(ConfigError {
            line: line_number,
            message: "unterminated OpenSSL quoted value".to_owned(),
        })
    } else {
        Ok(output)
    }
}

fn parse_section(value: &str, line: u32) -> Result<String, ConfigError> {
    let Some(close) = value.find(']') else {
        return Err(ConfigError {
            line,
            message: "unterminated OpenSSL section header".to_owned(),
        });
    };
    if !value[close + 1..].trim().is_empty() {
        return Err(ConfigError {
            line,
            message: "unexpected text after OpenSSL section header".to_owned(),
        });
    }
    let section = value[1..close].trim();
    if section.is_empty() {
        return Err(ConfigError {
            line,
            message: "empty OpenSSL section name".to_owned(),
        });
    }
    Ok(section.to_owned())
}

fn split_assignment(value: &str) -> Option<(&str, &str)> {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '\'' | '"' if quote.is_none() => quote = Some(character),
            current if quote == Some(current) => quote = None,
            '=' if quote.is_none() => return Some((&value[..index], &value[index + 1..])),
            _ => {}
        }
    }
    None
}

fn normalise_value(value: &str) -> String {
    let value = value.trim();
    let mut characters = value.chars();
    let first = characters.next();
    let last = value.chars().next_back();
    if value.len() >= 2 && first == last && matches!(first, Some('\'' | '"')) {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

fn contains_reference(value: &str) -> bool {
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '$' {
            return true;
        }
    }
    false
}

fn record_last<'a>(
    assignments: &'a [Assignment],
    key: &str,
    evidence: &mut Vec<ConfigEvidence>,
) -> Option<&'a Assignment> {
    let matches = assignments
        .iter()
        .filter(|assignment| assignment.key.eq_ignore_ascii_case(key))
        .collect::<Vec<_>>();
    let effective = matches.last().copied();
    for assignment in matches {
        let is_effective = effective.is_some_and(|last| std::ptr::eq(last, assignment));
        evidence.push(ConfigEvidence {
            line: assignment.line,
            section: assignment.section.clone(),
            target: None,
            directive: assignment.key.clone(),
            values: vec![assignment.value.clone()],
            raw_value: assignment.value.clone(),
            effective: is_effective,
            overridden_by_line: (!is_effective)
                .then(|| effective.map(|last| last.line))
                .flatten(),
        });
    }
    effective
}

fn effective_section(assignments: &[Option<&Assignment>]) -> String {
    assignments
        .iter()
        .flatten()
        .max_by_key(|assignment| assignment.line)
        .map_or_else(
            || "default".to_owned(),
            |assignment| assignment.section.clone(),
        )
}

fn notice_cross_section_projection(
    evidence: &[ConfigEvidence],
    effective_section: &str,
    notices: &mut Vec<ConfigNotice>,
) {
    let mut sections = evidence
        .iter()
        .filter(|item| item.effective)
        .map(|item| item.section.as_str())
        .collect::<Vec<_>>();
    sections.sort_unstable();
    sections.dedup();
    if sections.len() > 1 {
        notices.push(ConfigNotice {
            line: 1,
            section: effective_section.to_owned(),
            message: format!(
                "TLS assignments from multiple sections ({}) were projected in source order; section activation is application-specific",
                sections.join(", ")
            ),
        });
    }
}

fn project_protocol_range(
    minimum: Option<&Assignment>,
    maximum: Option<&Assignment>,
    section: &str,
    notices: &mut Vec<ConfigNotice>,
) -> Vec<ProtocolVersion> {
    let Ok(minimum) = project_protocol_bound(minimum, "MinProtocol", section, notices) else {
        return Vec::new();
    };
    let Ok(maximum) = project_protocol_bound(maximum, "MaxProtocol", section, notices) else {
        return Vec::new();
    };
    if minimum.is_none() && maximum.is_none() {
        return Vec::new();
    }
    let minimum = minimum.unwrap_or(ProtocolVersion::Ssl2);
    let maximum = maximum.unwrap_or(ProtocolVersion::Tls13);
    if minimum > maximum {
        notices.push(ConfigNotice {
            line: 1,
            section: section.to_owned(),
            message: format!(
                "MinProtocol {minimum} is newer than MaxProtocol {maximum}; no protocol range was projected"
            ),
        });
        return Vec::new();
    }
    all_protocols()
        .into_iter()
        .filter(|protocol| *protocol >= minimum && *protocol <= maximum)
        .collect()
}

fn project_protocol_bound(
    assignment: Option<&Assignment>,
    key: &str,
    section: &str,
    notices: &mut Vec<ConfigNotice>,
) -> Result<Option<ProtocolVersion>, ()> {
    let Some(assignment) = assignment else {
        return Ok(None);
    };
    if assignment.dynamic {
        notices.push(ConfigNotice {
            line: assignment.line,
            section: section.to_owned(),
            message: format!(
                "dynamic {key} value `{}` was preserved but not expanded",
                assignment.value
            ),
        });
        return Err(());
    }
    if assignment.value.eq_ignore_ascii_case("None") {
        return Ok(None);
    }
    match assignment.value.parse() {
        Ok(protocol) => Ok(Some(protocol)),
        Err(message) => {
            notices.push(ConfigNotice {
                line: assignment.line,
                section: section.to_owned(),
                message: format!("{message}; {key} was preserved"),
            });
            Err(())
        }
    }
}

const fn all_protocols() -> [ProtocolVersion; 6] {
    [
        ProtocolVersion::Ssl2,
        ProtocolVersion::Ssl3,
        ProtocolVersion::Tls10,
        ProtocolVersion::Tls11,
        ProtocolVersion::Tls12,
        ProtocolVersion::Tls13,
    ]
}

fn project_colon_assignments<'a>(
    assignments: impl Iterator<Item = &'a Assignment>,
    notices: &mut Vec<ConfigNotice>,
) -> Vec<String> {
    let mut output = Vec::new();
    for assignment in assignments {
        if assignment.dynamic {
            notices.push(ConfigNotice {
                line: assignment.line,
                section: assignment.section.clone(),
                message: format!(
                    "dynamic {} value `{}` was preserved but not expanded",
                    assignment.key, assignment.value
                ),
            });
            continue;
        }
        for value in assignment
            .value
            .split([':', ','])
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if is_openssl_selector(value) {
                notices.push(ConfigNotice {
                    line: assignment.line,
                    section: assignment.section.clone(),
                    message: format!(
                        "OpenSSL selector `{value}` is retained as configured evidence; the adapter does not resolve OpenSSL name databases"
                    ),
                });
            } else if !output.iter().any(|current| current == value) {
                output.push(value.to_owned());
            }
        }
    }
    output
}

fn project_certificate(
    certificate: Option<&Assignment>,
    notices: &mut Vec<ConfigNotice>,
) -> Vec<String> {
    let Some(certificate) = certificate else {
        return Vec::new();
    };
    if certificate.dynamic {
        notices.push(ConfigNotice {
            line: certificate.line,
            section: certificate.section.clone(),
            message: format!(
                "dynamic Certificate path `{}` was preserved but not expanded",
                certificate.value
            ),
        });
        Vec::new()
    } else {
        vec![certificate.value.clone()]
    }
}

fn collect_extensions(
    assignments: &[Assignment],
    notices: &mut Vec<ConfigNotice>,
) -> BTreeMap<String, Vec<TlsValue>> {
    let mut extensions = BTreeMap::new();
    for assignment in assignments {
        if is_known_key(&assignment.key) {
            continue;
        }
        let key = format!("openssl.{}.{}", assignment.section, assignment.key);
        extensions
            .entry(key)
            .or_insert_with(Vec::new)
            .push(TlsValue::Scalar(assignment.value.clone()));
        let message = if assignment.key.eq_ignore_ascii_case(".include")
            || assignment.key.eq_ignore_ascii_case("include")
        {
            format!(
                "include `{}` was preserved but not followed",
                assignment.value
            )
        } else {
            format!(
                "unsupported OpenSSL assignment `{}` was preserved",
                assignment.key
            )
        };
        notices.push(ConfigNotice {
            line: assignment.line,
            section: assignment.section.clone(),
            message,
        });
    }
    extensions
}

fn is_known_key(key: &str) -> bool {
    [
        "MinProtocol",
        "MaxProtocol",
        "CipherString",
        "Ciphersuites",
        "Groups",
        "SignatureAlgorithms",
        "Certificate",
    ]
    .iter()
    .any(|known| key.eq_ignore_ascii_case(known))
}

fn is_openssl_selector(value: &str) -> bool {
    value.starts_with(['!', '-', '+', '@'])
        || [
            "DEFAULT",
            "ALL",
            "COMPLEMENTOFDEFAULT",
            "COMPLEMENTOFALL",
            "HIGH",
            "MEDIUM",
            "LOW",
        ]
        .iter()
        .any(|selector| value.eq_ignore_ascii_case(selector))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_section_headers() {
        assert!(import_openssl_config("[broken\nMinProtocol = TLSv1.2").is_err());
        assert!(import_openssl_config("[]\n").is_err());
        assert!(import_openssl_config("Certificate = \"unterminated\n").is_err());
        assert!(import_openssl_config("CipherString = HIGH:\\\n").is_err());
    }

    #[test]
    fn includes_and_references_are_never_resolved() {
        let imported = import_openssl_config(
            ".include /missing/openssl.cnf\n[ssl]\nCertificate = $ENV::CERT_PATH\n",
        )
        .expect("configuration is structurally valid");
        assert!(imported.endpoint.certificate_chain.is_empty());
        assert!(
            imported
                .notices
                .iter()
                .any(|notice| notice.message.contains("not followed"))
        );
        assert_eq!(
            imported.extensions["openssl.default..include"],
            [TlsValue::Scalar("/missing/openssl.cnf".to_owned())]
        );
    }

    #[test]
    fn dynamic_protocol_bound_does_not_create_a_guessed_range() {
        let imported =
            import_openssl_config("[ssl]\nMinProtocol = $ENV::MIN_TLS\nMaxProtocol = TLSv1.3\n")
                .expect("configuration is structurally valid");
        assert!(imported.endpoint.protocols.is_empty());
        assert!(
            imported
                .notices
                .iter()
                .any(|notice| notice.message.contains("not expanded"))
        );
    }
}
