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

use super::*;

const SAMPLE: &str = r"
sslictcl 1
certificate leaf {
    pem {-----BEGIN CERTIFICATE-----
not-real
-----END CERTIFICATE-----}
    key leaf-key
    future-field {kept exactly}
}
endpoint /Common/example {
    hostname example.test
    protocols {tls1.2 tls1.3 tls1.2}
    ciphers {TLS_AES_128_GCM_SHA256 ECDHE-RSA-AES128-GCM-SHA256}
    certificate-chain {leaf intermediate}
    hsts {
        enabled true
        max-age 31536000
        include-subdomains yes
        preload false
    }
    adapter-source bigip
}
future-top value
";

fn errors(loaded: &DslLoad) -> Vec<&DslDiagnostic> {
    loaded
        .diagnostics
        .iter()
        .filter(|item| item.severity == DslSeverity::Error)
        .collect()
}

#[test]
fn loads_literal_document_and_preserves_unknowns() {
    let loaded = load(SAMPLE).expect("valid DSL");
    let endpoint = &loaded.model.endpoints["/Common/example"];
    assert_eq!(
        endpoint.protocols,
        vec![ProtocolVersion::Tls12, ProtocolVersion::Tls13]
    );
    assert_eq!(endpoint.certificate_chain, ["leaf", "intermediate"]);
    assert_eq!(
        endpoint.hsts.as_ref().and_then(|hsts| hsts.max_age),
        Some(31_536_000)
    );
    assert_eq!(loaded.notices.len(), 3);
    assert!(loaded.model.extensions.contains_key("future-top"));
}

#[test]
fn rejects_all_substitution_forms() {
    for source in [
        "sslictcl 1\nendpoint x { hostname $host }",
        "sslictcl 1\nendpoint x { hostname [exec id] }",
        "sslictcl 1\nendpoint x { ciphers {*}$values }",
    ] {
        let error = load(source).expect_err("dynamic DSL must be rejected");
        assert_eq!(error.code, DiagCode::Sslic1002);
    }
}

#[test]
fn requires_header_and_unique_names() {
    assert_eq!(load("endpoint x {}").unwrap_err().code, DiagCode::Sslic1003);
    assert_eq!(
        load("sslictcl 1\nendpoint x {}\nendpoint x {}")
            .unwrap_err()
            .code,
        DiagCode::Sslic1008
    );
    assert_eq!(
        load("sslictcl 1\nendpoint x {").unwrap_err().code,
        DiagCode::Sslic1001
    );
    assert_eq!(
        load("sslictcl 1\nsslictcl 1").unwrap_err().code,
        DiagCode::Sslic1004
    );
}

#[test]
fn nested_member_ranges_are_absolute_document_offsets() {
    let source = "sslictcl 1\nendpoint x {\n    hsts {\n        enabled maybe\n    }\n}\n";
    let loaded = load_with_diagnostics(source);
    let found = errors(&loaded);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, DiagCode::Sslic1009);
    let start = source.find("maybe").unwrap();
    assert_eq!(found[0].range.start() as usize, start);
    assert_eq!(found[0].range.end() as usize, start + "maybe".len());
}

#[test]
fn recovers_and_reports_every_independent_error() {
    let source = concat!(
        "sslictcl 1\n",
        "certificate a {\n    key only\n}\n",
        "endpoint good {\n    hostname good.test\n}\n",
        "endpoint bad {\n    hsts {\n        nonsense 1\n    }\n}\n",
        "endpoint good {\n}\n",
    );
    let loaded = load_with_diagnostics(source);
    let codes: Vec<DiagCode> = errors(&loaded).iter().map(|item| item.code).collect();
    assert_eq!(
        codes,
        vec![
            DiagCode::Sslic1010,
            DiagCode::Sslic1007,
            DiagCode::Sslic1008
        ]
    );
    let document = loaded.document.expect("recovered document");
    assert!(document.model.endpoints.contains_key("good"));
    assert!(document.model.endpoints.contains_key("bad"));
    assert!(!document.model.certificates.contains_key("a"));
}

#[test]
fn newer_vocabulary_is_a_warning_not_an_error() {
    let loaded = load_with_diagnostics("sslictcl 2\n");
    assert_eq!(errors(&loaded), [] as [&crate::dsl::DslDiagnostic; 0]);
    assert_eq!(loaded.diagnostics[0].code, DiagCode::Sslic1102);
    assert_eq!(loaded.diagnostics[0].severity, DslSeverity::Warning);
    assert_eq!(loaded.document.unwrap().model.vocabulary, 2);
}

/// Lists go through the shared `tcl_syntax::list` grammar, not the command
/// segmenter: a braced word is verbatim, so a glob with a bracket set is a
/// literal element rather than command substitution, `;` is not a
/// separator, and a list may span lines.
#[test]
fn braced_lists_use_the_shared_list_grammar() {
    let source = concat!(
        "sslictcl 1\n",
        "policy p {\n",
        "    check globs {\n",
        "        forbid-ciphers {[A-Z]*RC4 *_NULL_? ex;port}\n",
        "    }\n",
        "}\n",
        "endpoint e {\n",
        "    ciphers {\n",
        "        TLS_AES_256_GCM_SHA384\n",
        "        ECDHE-RSA-AES128-GCM-SHA256\n",
        "        {two words}\n",
        "    }\n",
        "}\n",
    );
    let loaded = load(source).expect("literal list values are not substitutions");
    assert_eq!(
        loaded.model.policies["p"].checks["globs"].forbid_ciphers,
        ["[A-Z]*RC4", "*_NULL_?", "ex;port"]
    );
    assert_eq!(
        loaded.model.endpoints["e"].ciphers,
        [
            "TLS_AES_256_GCM_SHA384",
            "ECDHE-RSA-AES128-GCM-SHA256",
            "two words"
        ]
    );
}

#[test]
fn malformed_and_substituting_lists_are_still_rejected() {
    // An unmatched brace inside a braced list is a list-grammar error.
    assert_eq!(
        load("sslictcl 1\nendpoint e { ciphers {a {b} }")
            .unwrap_err()
            .code,
        DiagCode::Sslic1001
    );
    assert_eq!(
        load("sslictcl 1\nendpoint e {\n    ciphers {{a}b}\n}")
            .unwrap_err()
            .code,
        DiagCode::Sslic1009
    );
    // An *unbraced* word carrying substitution is still SSLIC1002.
    assert_eq!(
        load("sslictcl 1\nendpoint e { ciphers $suite }")
            .unwrap_err()
            .code,
        DiagCode::Sslic1002
    );
}

#[test]
fn grade_is_reserved_as_a_check_identifier() {
    let loaded = load_with_diagnostics("sslictcl 1\npolicy p {\n    check grade {}\n}\n");
    let found = errors(&loaded);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, DiagCode::Sslic1009);
    assert!(found[0].message.contains("reserved"));
    let document = loaded.document.expect("the rest of the policy still loads");
    assert!(document.model.policies["p"].checks.is_empty());
}

#[test]
fn min_key_bits_keeps_its_full_unsigned_range() {
    let huge = u64::from(u32::MAX) + 1;
    let source = format!(
        "sslictcl 1\npolicy p {{\n    check k {{\n        min-key-bits {huge}\n    }}\n}}\n"
    );
    let loaded = load(&source).expect("an in-domain unsigned integer");
    assert_eq!(
        loaded.model.policies["p"].checks["k"].min_key_bits,
        Some(huge)
    );
}

#[test]
fn every_published_code_has_an_emitting_document() {
    const ANCHOR: &str =
        "sslictcl 1\npolicy p {\n    check c {\n        predicate {expr 1}\n    }\n}\n";
    let cases: &[(DiagCode, &str)] = &[
        (DiagCode::Sslic1001, "sslictcl 1\nendpoint x {"),
        (
            DiagCode::Sslic1002,
            "sslictcl 1\nendpoint x { hostname $h }",
        ),
        (DiagCode::Sslic1003, "endpoint x {}"),
        (DiagCode::Sslic1004, "sslictcl 1\nsslictcl 1"),
        (DiagCode::Sslic1005, "sslictcl 1\nendpoint x"),
        (DiagCode::Sslic1006, "sslictcl 1\nendpoint x body"),
        (
            DiagCode::Sslic1007,
            "sslictcl 1\nendpoint x { hsts { bogus 1 } }",
        ),
        (
            DiagCode::Sslic1008,
            "sslictcl 1\nendpoint x {}\nendpoint x {}",
        ),
        (
            DiagCode::Sslic1009,
            "sslictcl 1\nendpoint x { hsts { enabled maybe } }",
        ),
        (DiagCode::Sslic1010, "sslictcl 1\ncertificate c { key k }"),
        (
            DiagCode::Sslic1011,
            "sslictcl 1\nendpoint x { chain missing }",
        ),
        (
            DiagCode::Sslic1012,
            "sslictcl 1\nchain c { certificates {} }\nendpoint x {\n chain c\n certificate-chain {}\n}",
        ),
        (DiagCode::Sslic1101, "sslictcl 1\nfuture-word value"),
        (DiagCode::Sslic1102, "sslictcl 2"),
        (DiagCode::Sslic1103, ANCHOR),
    ];
    for (code, source) in cases {
        let reported: Vec<DiagCode> = load_with_diagnostics(source)
            .diagnostics
            .iter()
            .map(|item| item.code)
            .collect();
        assert!(
            reported.contains(code),
            "{code} was not emitted for:\n{source}\ngot {reported:?}"
        );
    }
}

#[test]
fn chain_and_policy_resolve_regardless_of_declaration_order() {
    let source = concat!(
        "sslictcl 1\n",
        "endpoint e {\n    chain c\n    policy p\n}\n",
        "chain c {\n    certificates {leaf issuer}\n}\n",
        "certificate leaf {\n    pem leaf-material\n}\n",
        "certificate issuer {\n    pem issuer-material\n}\n",
        "policy p {\n    grade {\n        minimum A\n    }\n}\n",
    );
    let loaded = load(source).expect("resolvable document");
    let endpoint = &loaded.model.endpoints["e"];
    assert_eq!(endpoint.chain.as_deref(), Some("c"));
    assert_eq!(endpoint.policy.as_deref(), Some("p"));
    assert_eq!(endpoint.certificate_chain, ["leaf", "issuer"]);
    assert_eq!(
        loaded.model.policies["p"].grade.map(|rule| rule.minimum),
        Some(Grade::A)
    );
}

#[test]
fn unresolved_chain_certificate_is_reported_once() {
    let loaded = load_with_diagnostics("sslictcl 1\nchain c {\n    certificates {ghost}\n}\n");
    let found = errors(&loaded);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, DiagCode::Sslic1011);
    assert!(found[0].message.contains("ghost"));
}

#[test]
fn catalogue_facts_are_typed_and_deduplicated() {
    let source = concat!(
        "sslictcl 1\n",
        "protocol tls1.0 {\n    status prohibited\n    score 0\n    reference RFC8996\n}\n",
        "cipher ECDHE-RSA-AES128-GCM-SHA256 {\n",
        "    iana-name TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256\n",
        "    openssl-name ECDHE-RSA-AES128-GCM-SHA256\n",
        "    key-exchange ECDHE\n    authentication RSA\n    encryption AES-128-GCM\n",
        "    bits 128\n    forward-secrecy true\n    aead true\n    status recommended\n",
        "    protocols {tls1.2}\n}\n",
    );
    let loaded = load(source).expect("valid catalogue");
    let protocol = &loaded.model.facts.protocols[&ProtocolVersion::Tls10];
    assert_eq!(protocol.status, Some(TlsStatus::Prohibited));
    assert_eq!(protocol.score, Some(0));
    let cipher = &loaded.model.facts.ciphers["ECDHE-RSA-AES128-GCM-SHA256"];
    assert_eq!(cipher.bits, Some(128));
    assert_eq!(cipher.forward_secrecy, Some(true));
    assert_eq!(cipher.protocols, vec![ProtocolVersion::Tls12]);
    assert_eq!(cipher.status, Some(TlsStatus::Recommended));
}

#[test]
fn policy_checks_are_typed_and_predicates_are_retained_verbatim() {
    let source = concat!(
        "sslictcl 1\n",
        "policy strict {\n",
        "    check modern {\n",
        "        severity error\n        message {TLS 1.3 required}\n",
        "        require-protocols {tls1.3}\n        forbid-protocols {tls1.0 tls1.1}\n",
        "        forbid-ciphers {RC4* *NULL*}\n        require-forward-secrecy yes\n",
        "        min-key-bits 2048\n        require-hsts on\n",
        "        min-hsts-max-age 15552000\n",
        "        predicate {expr {[llength $x] > 0}}\n",
        "    }\n",
        "    grade {\n        minimum A+\n    }\n",
        "}\n",
    );
    let loaded = load(source).expect("valid policy");
    let policy = &loaded.model.policies["strict"];
    let check = &policy.checks["modern"];
    assert_eq!(check.severity, Some(EstimateSeverity::Error));
    assert_eq!(check.message.as_deref(), Some("TLS 1.3 required"));
    assert_eq!(check.require_protocols, vec![ProtocolVersion::Tls13]);
    assert_eq!(
        check.forbid_protocols,
        vec![ProtocolVersion::Tls10, ProtocolVersion::Tls11]
    );
    assert_eq!(check.forbid_ciphers, ["RC4*", "*NULL*"]);
    assert_eq!(check.require_forward_secrecy, Some(true));
    assert_eq!(check.min_key_bits, Some(2048));
    assert_eq!(check.min_hsts_max_age, Some(15_552_000));
    assert_eq!(
        check.predicate.as_deref(),
        Some("expr {[llength $x] > 0}"),
        "the predicate is retained byte-for-byte and never parsed"
    );
    assert_eq!(policy.grade.map(|rule| rule.minimum), Some(Grade::APlus));
}

#[test]
fn trust_program_compiles_into_a_trust_store() {
    let source = concat!(
        "sslictcl 1\n",
        "trust-program mozilla-128 {\n",
        "    client mozilla\n    version 128\n",
        "    generated-at 2026-01-01T00:00:00Z\n",
        "    source-name mozilla\n    source-url https://example.test/mozilla\n",
        "    source-revision abc123\n    source-license CC0-1.0\n",
        "    anchor ",
        "abababababababababababababababababababababababababababababababab",
        " {\n",
        "        subject {CN=Example Root}\n        purposes {server-auth client-auth}\n",
        "        trusted true\n        distrust-after 1893456000\n",
        "    }\n",
        "}\n",
    );
    let loaded = load(source).expect("valid trust program");
    let program = &loaded.model.trust_programs["mozilla-128"];
    assert_eq!(program.client, crate::trust::ClientFamily::Mozilla);
    assert_eq!(program.anchors.len(), 1);
    let dataset = crate::dataset::compile_trust_snapshots(&[program.to_snapshot()])
        .expect("declared program compiles");
    assert_eq!(dataset.trust.anchors.len(), 1);
    assert_eq!(dataset.trust.anchors[0].subject, "CN=Example Root");
    assert_eq!(
        dataset.trust.anchors[0].memberships[0]
            .snapshot_version
            .as_deref(),
        Some("128")
    );
}

#[test]
fn wrong_word_count_and_unbraced_body_are_coded() {
    assert_eq!(
        load("sslictcl 1\nendpoint x").unwrap_err().code,
        DiagCode::Sslic1005
    );
    assert_eq!(
        load("sslictcl 1\nendpoint x body").unwrap_err().code,
        DiagCode::Sslic1006
    );
}

/// A byte-order mark is a file prologue, not part of the first word.
///
/// The `sslictcl` profile lexes with `GRAMMAR_TCL9X`, whose
/// `script_skips_leading_bom` is true, so a BOM-prefixed document must
/// load exactly as the same bytes without one — and every span must still
/// index the **original** string, mark included, or an editor would
/// underline the wrong bytes.
#[test]
fn a_leading_byte_order_mark_is_a_file_prologue() {
    let body = "sslictcl 1\nendpoint www {\n    hostname www.example.com\n}\n";
    let marked = format!("\u{FEFF}{body}");

    let plain = load_with_diagnostics(body);
    let with_mark = load_with_diagnostics(&marked);
    assert!(
        with_mark.diagnostics.is_empty(),
        "a BOM-prefixed document loads clean: {:?}",
        with_mark.diagnostics
    );
    assert_eq!(
        with_mark.document.map(|doc| doc.model),
        plain.document.map(|doc| doc.model),
        "the mark changes nothing about what was declared"
    );

    // Spans index the original string: the mark is three bytes, so every
    // range in the marked document sits three bytes later, and slicing the
    // marked source by a reported range yields the same text.
    let bad_plain = load_with_diagnostics("sslictcl 1\nchain c {\n    nope x\n}\n");
    let bad_marked = load_with_diagnostics("\u{FEFF}sslictcl 1\nchain c {\n    nope x\n}\n");
    let plain_first = bad_plain.diagnostics.first().expect("a diagnostic");
    let marked_first = bad_marked.diagnostics.first().expect("a diagnostic");
    assert_eq!(plain_first.code, marked_first.code);
    assert_eq!(
        marked_first.range.start(),
        plain_first.range.start() + 3,
        "the mark shifts every offset by its own width"
    );
    let source = "\u{FEFF}sslictcl 1\nchain c {\n    nope x\n}\n";
    let start = marked_first.range.start() as usize;
    let end = marked_first.range.end() as usize;
    assert!(
        source.get(start..end).is_some(),
        "a reported range must slice the original source"
    );
}
