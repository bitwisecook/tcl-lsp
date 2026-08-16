// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

use tcl_sslictcl::model::{ProtocolVersion, TlsValue};
use tcl_sslictcl::nginx::import_nginx;
use tcl_sslictcl::openssl_config::import_openssl_config;

const NGINX: &str = include_str!("fixtures/nginx-tls.conf");
const OPENSSL: &str = include_str!("fixtures/openssl-tls.cnf");

#[test]
fn nginx_projects_server_tls_and_hsts() {
    let imported = import_nginx(NGINX).expect("representative nginx configuration");
    assert_eq!(imported.endpoints.len(), 2);

    let endpoint = &imported.endpoints[0];
    assert_eq!(endpoint.name, "example.test");
    assert_eq!(endpoint.hostname.as_deref(), Some("example.test"));
    assert_eq!(
        endpoint.protocols,
        [ProtocolVersion::Tls12, ProtocolVersion::Tls13]
    );
    assert_eq!(
        endpoint.ciphers,
        [
            "ECDHE-RSA-AES256-GCM-SHA384",
            "ECDHE-RSA-CHACHA20-POLY1305",
            "TLS_AES_256_GCM_SHA384",
            "TLS_CHACHA20_POLY1305_SHA256",
        ]
    );
    assert_eq!(
        endpoint.certificate_chain,
        ["/etc/nginx/tls/example-chain.pem"]
    );
    let hsts = endpoint.hsts.as_ref().expect("HSTS header projected");
    assert_eq!(hsts.max_age, Some(63_072_000));
    assert!(hsts.include_subdomains);
    assert!(hsts.preload);
    assert_eq!(
        endpoint.extensions["nginx.ssl_certificate_key"],
        [TlsValue::Scalar("/etc/nginx/tls/example.key".to_owned())]
    );
    assert!(endpoint.extensions.contains_key("nginx.quic_retry"));
    assert!(endpoint.extensions.contains_key("nginx.location"));
}

#[test]
fn nginx_applies_inheritance_and_records_override_evidence() {
    let imported = import_nginx(NGINX).expect("representative nginx configuration");
    let inherited = &imported.endpoints[1];
    assert_eq!(
        inherited.protocols,
        [ProtocolVersion::Tls10, ProtocolVersion::Tls12]
    );
    assert_eq!(inherited.ciphers, ["ECDHE-RSA-AES128-GCM-SHA256"]);
    assert_eq!(inherited.hsts.as_ref().unwrap().max_age, Some(300));

    let parent_protocol = imported
        .evidence
        .iter()
        .find(|item| {
            item.directive == "ssl_protocols"
                && item.values == ["TLSv1", "TLSv1.2"]
                && !item.effective
        })
        .expect("parent protocol evidence overridden for first server");
    assert!(parent_protocol.overridden_by_line.is_some());
    assert!(imported.evidence.iter().any(|item| {
        item.directive == "ssl_protocols" && item.values == ["TLSv1", "TLSv1.2"] && item.effective
    }));
}

#[test]
fn nginx_preserves_includes_unknowns_and_exact_input() {
    let imported = import_nginx(NGINX).expect("representative nginx configuration");
    assert_eq!(imported.raw, NGINX);
    assert!(imported.extensions.contains_key("nginx.user"));
    assert!(imported.extensions.contains_key("nginx.include"));
    assert!(
        imported
            .notices
            .iter()
            .filter(|notice| notice.message.contains("not followed"))
            .count()
            >= 2
    );
    assert!(imported.notices.iter().any(|notice| {
        notice
            .message
            .contains("unsupported nginx directive `quic_retry`")
    }));
}

#[test]
fn openssl_projects_tls_assignments_and_ranges() {
    let imported = import_openssl_config(OPENSSL).expect("representative OpenSSL configuration");
    assert_eq!(imported.endpoint.name, "openssl:system_default_sect");
    assert_eq!(
        imported.endpoint.protocols,
        [ProtocolVersion::Tls12, ProtocolVersion::Tls13]
    );
    assert_eq!(
        imported.endpoint.ciphers,
        [
            "ECDHE-RSA-AES256-GCM-SHA384",
            "TLS_AES_256_GCM_SHA384",
            "TLS_CHACHA20_POLY1305_SHA256",
        ]
    );
    assert_eq!(imported.endpoint.groups, ["X25519", "P-256"]);
    assert_eq!(
        imported.endpoint.signature_schemes,
        ["rsa_pss_rsae_sha256", "ecdsa_secp256r1_sha256"]
    );
    assert_eq!(
        imported.endpoint.certificate_chain,
        ["/etc/ssl/certs/example-chain.pem"]
    );
}

#[test]
fn openssl_tracks_overrides_and_preserves_unsupported_input() {
    let imported = import_openssl_config(OPENSSL).expect("representative OpenSSL configuration");
    let minimums = imported
        .evidence
        .iter()
        .filter(|item| item.directive == "MinProtocol")
        .collect::<Vec<_>>();
    assert_eq!(minimums.len(), 2);
    assert!(!minimums[0].effective);
    assert_eq!(minimums[0].overridden_by_line, Some(minimums[1].line));
    assert!(minimums[1].effective);

    assert_eq!(imported.raw, OPENSSL);
    assert!(imported.extensions.contains_key("openssl.default..include"));
    assert!(
        imported
            .extensions
            .contains_key("openssl.system_default_sect.FutureOption")
    );
    assert!(
        imported
            .extensions
            .contains_key("openssl.unrelated.ProviderSetting")
    );
    assert!(imported.notices.iter().any(|notice| {
        notice.message.contains("include") && notice.message.contains("not followed")
    }));
    assert!(imported.notices.iter().any(|notice| {
        notice
            .message
            .contains("unsupported OpenSSL assignment `FutureOption`")
    }));
}
