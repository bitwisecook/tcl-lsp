// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic model → `.sslictcl` text.
//!
//! The emitter is the inverse of [`crate::dsl::load`] for everything the
//! vocabulary can say: declarations are written in vocabulary order, every map
//! is a `BTreeMap` so iteration is sorted, bodies are indented four spaces per
//! level, and quoting goes through one shared [`tcl_word`] helper. Loading the
//! emitted text reproduces the same [`SslicModel`].
//!
//! The one documented exception is [`TlsValue::Object`], which no `.sslictcl`
//! document can produce — only the nginx adapter builds it. It is emitted as a
//! nested braced block so nothing is lost from the text, but the vocabulary has
//! no nested extension form, so re-loading such a block yields it as a scalar.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::model::{
    CertificateDeclaration, ChainDeclaration, CipherFact, Endpoint, HstsPolicy, Policy,
    PolicyCheck, ProtocolFact, SslicModel, TlsValue, TrustAnchorDeclaration,
    TrustProgramDeclaration,
};

/// Quote `value` as one Tcl word: bare when it is already a safe literal,
/// braced when it contains no brace or line-continuation trouble, and
/// backslash-escaped inside double quotes otherwise.
pub(crate) fn tcl_word(value: &str) -> String {
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

/// Lowercase hexadecimal, two digits per byte.
pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// One braced Tcl list.
fn tcl_list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let words: Vec<String> = items.into_iter().map(tcl_word).collect();
    format!("{{{}}}", words.join(" "))
}

fn indent(depth: usize) -> String {
    "    ".repeat(depth)
}

/// One `name value` member line.
fn member(out: &mut String, depth: usize, name: &str, value: &str) {
    let _ = writeln!(out, "{}{name} {value}", indent(depth));
}

/// One `name <text>` member line, quoted.
fn text_member(out: &mut String, depth: usize, name: &str, value: &str) {
    member(out, depth, name, &tcl_word(value));
}

/// One `name <list>` member line, omitted when the list is empty.
fn list_member(out: &mut String, depth: usize, name: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    member(
        out,
        depth,
        name,
        &tcl_list(values.iter().map(String::as_str)),
    );
}

fn open_block(out: &mut String, depth: usize, head: &str) {
    let _ = writeln!(out, "{}{head} {{", indent(depth));
}

fn close_block(out: &mut String, depth: usize) {
    let _ = writeln!(out, "{}}}", indent(depth));
}

impl SslicModel {
    /// Render the model as a deterministic `.sslictcl` document.
    ///
    /// Loading the result yields an equal model, so
    /// `load(load(text).model.to_sslictcl()).model == load(text).model`.
    #[must_use]
    pub fn to_sslictcl(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "sslictcl {}", self.vocabulary);
        for declaration in self.certificates.values() {
            emit_certificate(&mut out, declaration);
        }
        for endpoint in self.endpoints.values() {
            emit_endpoint(&mut out, endpoint);
        }
        for (name, import) in &self.testssl_imports {
            out.push_str(&import.declaration(name));
        }
        for program in self.trust_programs.values() {
            emit_trust_program(&mut out, program);
        }
        for (version, fact) in &self.facts.protocols {
            emit_protocol_fact(&mut out, version.as_str(), fact);
        }
        for (name, fact) in &self.facts.ciphers {
            emit_cipher_fact(&mut out, name, fact);
        }
        for chain in self.chains.values() {
            emit_chain(&mut out, chain);
        }
        for policy in self.policies.values() {
            emit_policy(&mut out, policy);
        }
        emit_extensions(&mut out, 0, &self.extensions);
        out
    }
}

fn emit_certificate(out: &mut String, declaration: &CertificateDeclaration) {
    open_block(
        out,
        0,
        &format!("certificate {}", tcl_word(&declaration.name)),
    );
    text_member(out, 1, "pem", &declaration.material);
    if let Some(key) = &declaration.key {
        text_member(out, 1, "key", key);
    }
    emit_extensions(out, 1, &declaration.extensions);
    close_block(out, 0);
}

fn emit_endpoint(out: &mut String, endpoint: &Endpoint) {
    open_block(out, 0, &format!("endpoint {}", tcl_word(&endpoint.name)));
    if let Some(hostname) = &endpoint.hostname {
        text_member(out, 1, "hostname", hostname);
    }
    let protocols: Vec<&str> = endpoint
        .protocols
        .iter()
        .map(|protocol| protocol.as_str())
        .collect();
    if !protocols.is_empty() {
        member(out, 1, "protocols", &tcl_list(protocols));
    }
    list_member(out, 1, "ciphers", &endpoint.ciphers);
    list_member(out, 1, "groups", &endpoint.groups);
    list_member(out, 1, "signature-schemes", &endpoint.signature_schemes);
    // A named chain owns the certificate list: re-emitting both would be the
    // mutually-exclusive pair the loader rejects (SSLIC1012), and the list is
    // rebuilt from the chain on load.
    match &endpoint.chain {
        Some(chain) => text_member(out, 1, "chain", chain),
        None => list_member(out, 1, "certificate-chain", &endpoint.certificate_chain),
    }
    if let Some(policy) = &endpoint.policy {
        text_member(out, 1, "policy", policy);
    }
    if let Some(hsts) = &endpoint.hsts {
        emit_hsts(out, hsts);
    }
    emit_extensions(out, 1, &endpoint.extensions);
    close_block(out, 0);
}

fn emit_hsts(out: &mut String, hsts: &HstsPolicy) {
    open_block(out, 1, "hsts");
    member(out, 2, "enabled", bool_word(hsts.enabled));
    if let Some(max_age) = hsts.max_age {
        member(out, 2, "max-age", &max_age.to_string());
    }
    member(
        out,
        2,
        "include-subdomains",
        bool_word(hsts.include_subdomains),
    );
    member(out, 2, "preload", bool_word(hsts.preload));
    close_block(out, 1);
}

const fn bool_word(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn emit_trust_program(out: &mut String, program: &TrustProgramDeclaration) {
    open_block(
        out,
        0,
        &format!("trust-program {}", tcl_word(&program.name)),
    );
    member(out, 1, "client", program.client.as_str());
    for (name, value) in [
        ("version", &program.version),
        ("generated-at", &program.generated_at),
        ("source-name", &program.source_name),
        ("source-url", &program.source_url),
        ("source-revision", &program.source_revision),
        ("source-license", &program.source_license),
    ] {
        if !value.is_empty() {
            text_member(out, 1, name, value);
        }
    }
    for anchor in program.anchors.values() {
        emit_anchor(out, anchor);
    }
    emit_extensions(out, 1, &program.extensions);
    close_block(out, 0);
}

fn emit_anchor(out: &mut String, anchor: &TrustAnchorDeclaration) {
    open_block(out, 1, &format!("anchor {}", anchor.fingerprint_sha256));
    if !anchor.subject.is_empty() {
        text_member(out, 2, "subject", &anchor.subject);
    }
    if let Some(der) = &anchor.der_base64 {
        text_member(out, 2, "der-base64", der);
    }
    if !anchor.purposes.is_empty() {
        member(
            out,
            2,
            "purposes",
            &tcl_list(anchor.purposes.iter().map(|purpose| purpose.as_str())),
        );
    }
    member(out, 2, "trusted", bool_word(anchor.trusted));
    if let Some(distrust_after) = anchor.distrust_after {
        member(out, 2, "distrust-after", &distrust_after.to_string());
    }
    close_block(out, 1);
}

fn emit_protocol_fact(out: &mut String, version: &str, fact: &ProtocolFact) {
    open_block(out, 0, &format!("protocol {version}"));
    if let Some(status) = fact.status {
        member(out, 1, "status", status.as_str());
    }
    if let Some(score) = fact.score {
        member(out, 1, "score", &score.to_string());
    }
    if let Some(reference) = &fact.reference {
        text_member(out, 1, "reference", reference);
    }
    close_block(out, 0);
}

fn emit_cipher_fact(out: &mut String, name: &str, fact: &CipherFact) {
    open_block(out, 0, &format!("cipher {}", tcl_word(name)));
    for (member_name, value) in [
        ("iana-name", &fact.iana_name),
        ("openssl-name", &fact.openssl_name),
        ("key-exchange", &fact.key_exchange),
        ("authentication", &fact.authentication),
        ("encryption", &fact.encryption),
    ] {
        if let Some(value) = value {
            text_member(out, 1, member_name, value);
        }
    }
    if let Some(bits) = fact.bits {
        member(out, 1, "bits", &bits.to_string());
    }
    if let Some(forward_secrecy) = fact.forward_secrecy {
        member(out, 1, "forward-secrecy", bool_word(forward_secrecy));
    }
    if let Some(aead) = fact.aead {
        member(out, 1, "aead", bool_word(aead));
    }
    if let Some(status) = fact.status {
        member(out, 1, "status", status.as_str());
    }
    if !fact.protocols.is_empty() {
        member(
            out,
            1,
            "protocols",
            &tcl_list(fact.protocols.iter().map(|protocol| protocol.as_str())),
        );
    }
    close_block(out, 0);
}

fn emit_chain(out: &mut String, chain: &ChainDeclaration) {
    open_block(out, 0, &format!("chain {}", tcl_word(&chain.name)));
    // `certificates` is required, so it is emitted even when empty.
    member(
        out,
        1,
        "certificates",
        &tcl_list(chain.certificates.iter().map(String::as_str)),
    );
    close_block(out, 0);
}

fn emit_policy(out: &mut String, policy: &Policy) {
    open_block(out, 0, &format!("policy {}", tcl_word(&policy.name)));
    for check in policy.checks.values() {
        emit_policy_check(out, check);
    }
    if let Some(rule) = policy.grade {
        open_block(out, 1, "grade");
        member(out, 2, "minimum", &rule.minimum.to_string());
        close_block(out, 1);
    }
    close_block(out, 0);
}

fn emit_policy_check(out: &mut String, check: &PolicyCheck) {
    open_block(out, 1, &format!("check {}", tcl_word(&check.id)));
    if let Some(severity) = check.severity {
        member(out, 2, "severity", severity.as_str());
    }
    if let Some(message) = &check.message {
        text_member(out, 2, "message", message);
    }
    for (name, protocols) in [
        ("require-protocols", &check.require_protocols),
        ("forbid-protocols", &check.forbid_protocols),
    ] {
        if !protocols.is_empty() {
            member(
                out,
                2,
                name,
                &tcl_list(protocols.iter().map(|protocol| protocol.as_str())),
            );
        }
    }
    list_member(out, 2, "forbid-ciphers", &check.forbid_ciphers);
    if let Some(required) = check.require_forward_secrecy {
        member(out, 2, "require-forward-secrecy", bool_word(required));
    }
    if let Some(bits) = check.min_key_bits {
        member(out, 2, "min-key-bits", &bits.to_string());
    }
    if let Some(required) = check.require_hsts {
        member(out, 2, "require-hsts", bool_word(required));
    }
    if let Some(seconds) = check.min_hsts_max_age {
        member(out, 2, "min-hsts-max-age", &seconds.to_string());
    }
    if let Some(predicate) = &check.predicate {
        // The script came from a braced word, so its brace nesting is balanced
        // by construction and re-bracing reproduces the same word verbatim.
        member(out, 2, "predicate", &format!("{{{predicate}}}"));
    }
    close_block(out, 1);
}

fn emit_extensions(out: &mut String, depth: usize, extensions: &BTreeMap<String, Vec<TlsValue>>) {
    for (name, values) in extensions {
        for value in values {
            emit_extension_value(out, depth, name, value);
        }
    }
}

fn emit_extension_value(out: &mut String, depth: usize, name: &str, value: &TlsValue) {
    match value {
        TlsValue::Scalar(word) => member(out, depth, name, &tcl_word(word)),
        TlsValue::List(words) => {
            let rendered: Vec<String> = words.iter().map(|word| tcl_word(word)).collect();
            if rendered.is_empty() {
                let _ = writeln!(out, "{}{name}", indent(depth));
            } else {
                member(out, depth, name, &rendered.join(" "));
            }
        }
        TlsValue::Object(fields) => {
            open_block(out, depth, &tcl_word(name));
            emit_extensions(out, depth + 1, fields);
            close_block(out, depth);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dsl::{DslSeverity, load, load_with_diagnostics};
    use tcl_core_types::DiagCode;

    /// The shipped sample exercises every declaration and member of
    /// vocabulary 1, plus an unknown word in each open block.
    const SAMPLE: &str = include_str!("../../../samples/sslictcl/example.sslictcl");

    #[test]
    fn sample_document_loads_with_no_errors() {
        let loaded = load_with_diagnostics(SAMPLE);
        let errors: Vec<&str> = loaded
            .diagnostics
            .iter()
            .filter(|item| item.severity == DslSeverity::Error)
            .map(|item| item.message.as_str())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        let codes: Vec<DiagCode> = loaded.diagnostics.iter().map(|item| item.code).collect();
        assert!(codes.contains(&DiagCode::Sslic1101));
        assert!(codes.contains(&DiagCode::Sslic1103));
        assert!(
            codes
                .iter()
                .all(|code| matches!(code, DiagCode::Sslic1101 | DiagCode::Sslic1103)),
            "the sample's only notices are the documented ones: {codes:?}"
        );
    }

    #[test]
    fn emitting_and_reloading_the_sample_is_a_fixpoint() {
        let first = load(SAMPLE).expect("sample loads");
        let emitted = first.model.to_sslictcl();
        let second = load(&emitted).expect("emitted document loads");
        assert_eq!(first.model, second.model, "emitted:\n{emitted}");
        assert_eq!(
            emitted,
            second.model.to_sslictcl(),
            "emission is idempotent on an already-emitted document"
        );
    }

    #[test]
    fn emitted_text_reloads_without_error_diagnostics() {
        let emitted = load(SAMPLE).expect("sample loads").model.to_sslictcl();
        let reloaded = load_with_diagnostics(&emitted);
        assert!(
            reloaded
                .diagnostics
                .iter()
                .all(|item| item.severity != DslSeverity::Error),
            "{:?}",
            reloaded.diagnostics
        );
    }

    /// The same round trip over a document with no extension words and no
    /// predicate re-loads with literally zero diagnostics.
    #[test]
    fn emitted_text_without_extensions_reloads_silently() {
        let source = concat!(
            "sslictcl 1\n",
            "certificate leaf {\n    pem leaf-material\n}\n",
            "chain c {\n    certificates {leaf}\n}\n",
            "endpoint e {\n    hostname e.example.test\n    protocols {tls1.3}\n",
            "    ciphers {TLS_AES_128_GCM_SHA256}\n    chain c\n    policy p\n",
            "    hsts {\n        enabled true\n        max-age 31536000\n    }\n}\n",
            "protocol tls1.3 {\n    status recommended\n    score 100\n}\n",
            "cipher TLS_AES_128_GCM_SHA256 {\n    bits 128\n    aead true\n}\n",
            "policy p {\n    check fs {\n        require-forward-secrecy true\n    }\n",
            "    grade {\n        minimum A\n    }\n}\n",
        );
        let model = load(source).expect("loads").model;
        let emitted = model.to_sslictcl();
        let reloaded = load_with_diagnostics(&emitted);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{:?}",
            reloaded.diagnostics
        );
        assert_eq!(reloaded.document.unwrap().model, model);
    }

    #[test]
    fn raw_source_stays_byte_exact() {
        assert_eq!(load(SAMPLE).expect("sample loads").raw_source, SAMPLE);
    }
}
