// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lossless import of testssl.sh JSON variants.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One normalised testssl.sh result. Common fields are projected for analysis;
/// every other field remains in [`Self::unknown`], and the import also retains
/// the complete source JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSslFinding {
    /// testssl check identifier.
    pub id: String,
    /// Target IP/address label.
    pub ip: Option<String>,
    /// Target port.
    pub port: Option<String>,
    /// testssl severity string.
    pub severity: Option<String>,
    /// Human-readable result.
    pub finding: Option<String>,
    /// CVE identifier(s), in source form.
    pub cve: Option<String>,
    /// CWE identifier(s), in source form.
    pub cwe: Option<String>,
    /// All non-common result fields, with JSON types intact.
    pub unknown: BTreeMap<String, Value>,
}

/// Lossless imported testssl document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSslImport {
    /// Importer wire schema.
    pub schema: u32,
    /// Normalised check records found anywhere in supported output envelopes.
    pub findings: Vec<TestSslFinding>,
    /// Complete parsed source. This is the forwards-compatibility guarantee:
    /// no future testssl field is discarded even if it is not projected yet.
    pub raw: Value,
}

impl TestSslImport {
    /// Emit an equivalent, deterministic `SslicTcl` declaration.
    ///
    /// The complete JSON is hexadecimal to avoid Tcl substitution and brace
    /// balancing semantics. Consumers can reconstruct the byte-identical
    /// compact JSON and re-run a newer importer later.
    #[must_use]
    pub fn to_sslictcl(&self, name: &str) -> String {
        let compact = serde_json::to_vec(&self.raw).unwrap_or_default();
        let mut output = String::new();
        output.push_str("sslictcl 1\n");
        output.push_str("testssl-import ");
        output.push_str(&tcl_word(name));
        output.push_str(" {\n    schema 1\n    raw-json-hex ");
        output.push_str(&hex_lower(&compact));
        output.push_str("\n}\n");
        output
    }
}

/// Invalid JSON input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSslError {
    /// Human-readable parser detail.
    pub message: String,
}

impl fmt::Display for TestSslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for TestSslError {}

/// Import array, `scanResult`, and nested testssl JSON output variants.
pub fn import_testssl_json(source: &str) -> Result<TestSslImport, TestSslError> {
    let raw: Value = serde_json::from_str(source).map_err(|error| TestSslError {
        message: format!("invalid testssl JSON: {error}"),
    })?;
    let mut findings = Vec::new();
    collect_findings(&raw, &mut findings);
    Ok(TestSslImport {
        schema: 1,
        findings,
        raw,
    })
}

fn collect_findings(value: &Value, output: &mut Vec<TestSslFinding>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_findings(item, output);
            }
        }
        Value::Object(object) => {
            if looks_like_finding(object) {
                output.push(project_finding(object));
            } else {
                for nested in object.values() {
                    if matches!(nested, Value::Array(_) | Value::Object(_)) {
                        collect_findings(nested, output);
                    }
                }
            }
        }
        _ => {}
    }
}

fn looks_like_finding(object: &Map<String, Value>) -> bool {
    object.contains_key("id")
        && (object.contains_key("finding")
            || object.contains_key("severity")
            || object.contains_key("scanTime"))
}

fn project_finding(object: &Map<String, Value>) -> TestSslFinding {
    const KNOWN: [&str; 7] = ["id", "ip", "port", "severity", "finding", "cve", "cwe"];
    let unknown = object
        .iter()
        .filter(|(key, _)| !KNOWN.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    TestSslFinding {
        id: field_text(object, "id").unwrap_or_default(),
        ip: field_text(object, "ip"),
        port: field_text(object, "port"),
        severity: field_text(object, "severity"),
        finding: field_text(object, "finding"),
        cve: field_text(object, "cve"),
        cwe: field_text(object, "cwe"),
        unknown,
    }
}

fn field_text(object: &Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(|value| match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    })
}

fn tcl_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._/-".contains(&byte))
    {
        return value.to_owned();
    }
    if !value.contains('{') && !value.contains('}') && !value.contains("\\\n") {
        return format!("{{{value}}}");
    }
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' | '"' | '$' | '[' | ']' => {
                output.push('\\');
                output.push(character);
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_array_and_preserves_unknown_typed_fields() {
        let source = r#"[{"id":"TLS1_3","severity":"OK","finding":"offered","future":{"x":1}}]"#;
        let imported = import_testssl_json(source).expect("valid JSON");
        assert_eq!(imported.findings.len(), 1);
        assert_eq!(imported.findings[0].unknown["future"]["x"], 1);
        assert_eq!(
            serde_json::to_value(&imported).unwrap()["raw"][0]["future"]["x"],
            1
        );
    }

    #[test]
    fn accepts_nested_scan_result_envelope() {
        let source = r#"{"targetHost":"example.test","scanResult":[{"id":"BREACH","severity":"WARN","finding":"potentially vulnerable"}]}"#;
        let imported = import_testssl_json(source).expect("valid JSON");
        assert_eq!(imported.findings[0].id, "BREACH");
        assert_eq!(imported.raw["targetHost"], "example.test");
    }

    #[test]
    fn equivalent_dsl_is_static_and_round_trippable() {
        let imported = import_testssl_json(
            r#"[{"id":"x","severity":"WARN","finding":"$x [exec id] { brace"}]"#,
        )
        .expect("valid JSON");
        let dsl = imported.to_sslictcl("scan $one");
        let loaded = crate::dsl::load(&dsl).expect("emitted DSL is declarative");
        let reloaded = &loaded.model.testssl_imports["scan $one"];
        assert_eq!(reloaded.findings, imported.findings);
        assert_eq!(reloaded.raw, imported.raw);
        let encoded = dsl
            .lines()
            .find(|line| line.contains("raw-json-hex"))
            .unwrap();
        assert!(!encoded.contains("exec"));
    }
}
