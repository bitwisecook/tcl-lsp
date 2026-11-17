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

/// A self-signed certificate made by OpenSSL in a private temporary directory,
/// with the values OpenSSL itself reports for it. The repository holds no
/// certificate material (`tcl-bigip-query/tests/fixtures/certificate-generation.md`).
struct GeneratedCertificate {
    pem: Vec<u8>,
    fingerprint_sha256: String,
    spki_sha256: String,
    serial: String,
    directory: std::path::PathBuf,
}

impl Drop for GeneratedCertificate {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).expect("remove temporary certificate material");
    }
}

fn openssl(arguments: &[&str]) -> Vec<u8> {
    let output = std::process::Command::new("openssl")
        .args(arguments)
        .output()
        .expect("OpenSSL is required for the generated certificate test");
    assert!(
        output.status.success(),
        "OpenSSL failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn generated_certificate() -> GeneratedCertificate {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::DirBuilderExt;
    let directory =
        std::env::temp_dir().join(format!("tcl-sslictcl-certificate-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .expect("create private temporary certificate directory");
    let path = |name: &str| {
        directory
            .join(name)
            .to_str()
            .expect("UTF-8 path")
            .to_owned()
    };
    let (key, certificate, spki) = (path("key.pem"), path("certificate.pem"), path("spki.der"));
    openssl(&[
        "genpkey",
        "-algorithm",
        "RSA",
        "-pkeyopt",
        "rsa_keygen_bits:2048",
        "-out",
        &key,
    ]);
    openssl(&[
        "req",
        "-new",
        "-x509",
        "-key",
        &key,
        "-out",
        &certificate,
        "-sha256",
        "-days",
        "2",
        "-set_serial",
        "1",
        "-subj",
        "/C=US/O=Example Corp/CN=example.test",
        "-addext",
        "subjectAltName=DNS:example.test,DNS:www.example.test,IP:192.0.2.10",
    ]);
    openssl(&[
        "pkey", "-in", &key, "-pubout", "-outform", "DER", "-out", &spki,
    ]);
    let text = |bytes: Vec<u8>| String::from_utf8(bytes).expect("OpenSSL printed UTF-8");
    let after_equals = |line: String| {
        line.split_once('=')
            .expect("name=value")
            .1
            .trim()
            .to_owned()
    };
    GeneratedCertificate {
        pem: std::fs::read(&certificate).expect("read the generated certificate"),
        fingerprint_sha256: after_equals(text(openssl(&[
            "x509",
            "-in",
            &certificate,
            "-noout",
            "-fingerprint",
            "-sha256",
        ])))
        .replace(':', "")
        .to_lowercase(),
        spki_sha256: Sha256::digest(std::fs::read(&spki).expect("read the public key"))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        serial: after_equals(text(openssl(&[
            "x509",
            "-in",
            &certificate,
            "-noout",
            "-serial",
        ]))),
        directory,
    }
}

#[test]
fn x509_projection_matches_what_openssl_reports() {
    let generated = generated_certificate();
    let certificates = parse_certificates(&generated.pem).expect("generated certificate parses");
    let [certificate] = certificates.as_slice() else {
        panic!("one certificate, got {}", certificates.len());
    };
    assert_eq!(certificate.fingerprint_sha256, generated.fingerprint_sha256);
    assert_eq!(certificate.spki_sha256, generated.spki_sha256);
    assert_eq!(
        certificate.serial.trim_start_matches('0'),
        generated.serial.trim_start_matches('0')
    );
    assert_eq!(certificate.subject, "C=US, O=Example Corp, CN=example.test");
    assert_eq!(certificate.issuer, certificate.subject);
    assert_eq!(
        certificate.not_after - certificate.not_before,
        2 * 24 * 60 * 60
    );
    assert_eq!(certificate.public_key_algorithm, "1.2.840.113549.1.1.1");
    assert_eq!(certificate.public_key_bits, Some(2048));
    assert_eq!(certificate.signature_algorithm, "1.2.840.113549.1.1.11");
    assert!(certificate.is_ca);
    assert_eq!(certificate.dns_names, ["example.test", "www.example.test"]);
    assert_eq!(certificate.ip_addresses, ["192.0.2.10"]);
}
