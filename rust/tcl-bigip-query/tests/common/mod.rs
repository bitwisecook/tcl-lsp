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

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct CertificateFixture {
    pub path: PathBuf,
    pub fingerprint: String,
    pub public_key: String,
    directory: PathBuf,
}

impl Drop for CertificateFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).expect("remove temporary certificate material");
    }
}

fn openssl(arguments: &[&str]) -> String {
    let output = Command::new("openssl")
        .args(arguments)
        .output()
        .expect("OpenSSL is required for generated certificate tests");
    assert!(
        output.status.success(),
        "OpenSSL failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("OpenSSL returned UTF-8 metadata")
}

pub fn certificate() -> CertificateFixture {
    let directory = std::env::temp_dir().join(format!(
        "tcl-query-certificate-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&directory)
        .expect("create private temporary certificate directory");
    let mut fixture = CertificateFixture {
        path: directory.join("certificate.pem"),
        fingerprint: String::new(),
        public_key: String::new(),
        directory,
    };
    let key = fixture.directory.join("key.pem");
    let path = fixture.path.clone();
    let key_name = key.to_str().expect("temporary key path is UTF-8");
    let certificate_name = path.to_str().expect("temporary certificate path is UTF-8");
    openssl(&[
        "genpkey",
        "-algorithm",
        "RSA",
        "-pkeyopt",
        "rsa_keygen_bits:2048",
        "-out",
        key_name,
    ]);
    openssl(&[
        "req",
        "-new",
        "-x509",
        "-key",
        key_name,
        "-out",
        certificate_name,
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
    openssl(&["pkey", "-in", key_name, "-check", "-noout"]);
    openssl(&["verify", "-CAfile", certificate_name, certificate_name]);
    let fingerprint = openssl(&[
        "x509",
        "-in",
        certificate_name,
        "-noout",
        "-fingerprint",
        "-sha256",
    ])
    .split_once('=')
    .expect("OpenSSL fingerprint output")
    .1
    .trim()
    .replace(':', "");
    let public_key = openssl(&["pkey", "-in", key_name, "-pubout"]);
    fixture.fingerprint = fingerprint;
    fixture.public_key = public_key;
    fixture
}
