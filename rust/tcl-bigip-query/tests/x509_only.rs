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

//! Contract tests for the report's socket-free x509 feature graph.

use tcl_bigip_query::Value;
use tcl_bigip_query::builtins::lookup;
use tcl_bigip_query::eval::{EvalContext, Root, evaluate};
use tcl_bigip_query::parser::parse_query;

fn evaluate_query(query: &str) -> Result<Vec<tcl_bigip_query::Value>, String> {
    let root = Root::json("x509-only.json", tcl_bigip_query::Value::Null);
    let mut context = EvalContext::new(root);
    let program = parse_query(query).map_err(|error| error.to_string())?;
    evaluate(&program, &mut context).map_err(|error| error.to_string())
}

#[test]
fn pure_x509_builtins_are_registered_without_network_probes() {
    for name in [
        "ucs_cert",
        "x509_parse",
        "x509_from_config",
        "x509_eq",
        "cert_load",
    ] {
        assert!(lookup(name).is_some(), "{name} must be available with x509");
    }
    #[cfg(not(feature = "probes"))]
    for name in [
        "dns",
        "rev_dns",
        "ping",
        "portping",
        "traceroute",
        "socket_get",
        "tls_handshake",
        "url_get",
        "url_head",
        "url_options",
        "url_post",
    ] {
        assert!(
            lookup(name).is_none(),
            "{name} must be absent without probes"
        );
    }
}

#[test]
fn cert_load_dispatches_to_the_real_x509_implementation() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/x509_example.pem"
    );
    let query = format!(
        "cert_load({})",
        serde_json::to_string(fixture).expect("fixture path is JSON-encodable")
    );
    let values = evaluate_query(&query).expect("valid PEM must load with x509 only");
    let Some(Value::Object(certificate)) = values.first() else {
        panic!("cert_load must return one certificate object: {values:?}");
    };
    let Some(Value::Str(subject)) = certificate.get("subject") else {
        panic!("cert_load subject is missing or not a string: {certificate:?}");
    };
    assert_eq!(subject, "CN=example.test,O=Example Corp,C=US");
    let Some(Value::Str(fingerprint)) = certificate.get("fingerprint_sha256") else {
        panic!("cert_load fingerprint is missing or not a string: {certificate:?}");
    };
    assert_eq!(
        fingerprint,
        "051B08743FE24044500609876DC3F008773FD0B91EDADD94A63DB5482D1FD8FE"
    );
}

#[test]
fn x509_parse_dispatches_through_the_x509_only_registry() {
    let error = evaluate_query("x509_parse(\"not a certificate\")")
        .expect_err("invalid PEM must be rejected by x509_parse");
    assert!(
        error.starts_with("x509_parse: not a PEM certificate ("),
        "unexpected x509_parse error: {error}"
    );
}

#[test]
fn x509_projection_and_equality_dispatch_without_probes() {
    let values = evaluate_query(
        "x509_eq(\
            x509_from_config({fingerprint: \"AA:BB\", subject: \"CN=leaf\", issuer: \"CN=root\"}), \
            {fingerprint_sha256: \"AABB\"})",
    )
    .expect("x509 projection/equality must be available");
    assert_eq!(values.len(), 1);
    assert!(matches!(values.first(), Some(Value::Bool(true))));
}
