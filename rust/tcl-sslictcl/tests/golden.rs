// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as _;

use tcl_sslictcl::{import_testssl_json, parse_certificates};

#[test]
fn testssl_projection_matches_golden() {
    let imported = import_testssl_json(include_str!("fixtures/testssl-sample.json"))
        .expect("fixture must import");
    let mut actual = String::new();
    for finding in &imported.findings {
        let unknown = finding
            .unknown
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            actual,
            "{}\t{}\t{}\t{}\t{}\t{}",
            finding.id,
            finding.ip.as_deref().unwrap_or(""),
            finding.port.as_deref().unwrap_or(""),
            finding.severity.as_deref().unwrap_or(""),
            finding.finding.as_deref().unwrap_or(""),
            unknown
        )
        .unwrap();
    }
    assert_eq!(actual, include_str!("fixtures/testssl-sample.golden.tsv"));
    assert_eq!(imported.raw["futureEnvelope"]["schema"], 7);
}

#[test]
fn x509_projection_matches_golden() {
    let certificates = parse_certificates(include_bytes!(
        "../../tcl-bigip-query/tests/fixtures/x509_example.pem"
    ))
    .expect("fixture must parse");
    let certificate = &certificates[0];
    let actual = format!(
        concat!(
            "fingerprint_sha256\t{}\n",
            "spki_sha256\t{}\n",
            "subject\t{}\n",
            "issuer\t{}\n",
            "serial\t{}\n",
            "not_before\t{}\n",
            "not_after\t{}\n",
            "public_key_algorithm\t{}\n",
            "public_key_bits\t{}\n",
            "signature_algorithm\t{}\n",
            "subject_key_id\t{}\n",
            "is_ca\t{}\n",
            "dns_names\t{}\n",
            "ip_addresses\t{}\n"
        ),
        certificate.fingerprint_sha256,
        certificate.spki_sha256,
        certificate.subject,
        certificate.issuer,
        certificate.serial,
        certificate.not_before,
        certificate.not_after,
        certificate.public_key_algorithm,
        certificate.public_key_bits.unwrap_or_default(),
        certificate.signature_algorithm,
        certificate.subject_key_id.as_deref().unwrap_or(""),
        certificate.is_ca,
        certificate.dns_names.join(","),
        certificate.ip_addresses.join(",")
    );
    assert_eq!(actual, include_str!("fixtures/x509-example.golden.tsv"));
}
