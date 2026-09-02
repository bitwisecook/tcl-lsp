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

//! The `SslicTcl` member rows — the statement words that carry operands and
//! open no block.
//!
//! A word that means one thing in several blocks is **one** row here
//! (`protocols` in `endpoint` and `cipher`, `status` in `protocol` and
//! `cipher`); grammar membership, not a duplicated spec, provides the context
//! sensitivity.

use crate::prelude::*;

use super::values;

/// A row taking one literal word.
fn text(
    name: &'static str,
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
) -> CommandSpec {
    super::statement(name, Arity::exact(1), summary, synopsis, snippet)
}

/// One `arg_values` table per closed domain: the field is `'static`, so the
/// tables are consts rather than a parameter the helper wraps.
const BOOL_ARG: &[(u8, &[ArgValue])] = &[(0, values::BOOLS)];
const CLIENT_ARG: &[(u8, &[ArgValue])] = &[(0, values::CLIENTS)];
const STATUS_ARG: &[(u8, &[ArgValue])] = &[(0, values::STATUSES)];
const SEVERITY_ARG: &[(u8, &[ArgValue])] = &[(0, values::SEVERITIES)];
const GRADE_ARG: &[(u8, &[ArgValue])] = &[(0, values::GRADES)];
const SCHEMA_ARG: &[(u8, &[ArgValue])] = &[(0, values::TESTSSL_SCHEMAS)];
/// Offered, not closed: the loader normalises documented aliases onto these.
const PROTOCOL_VERSION_ARG: &[(u8, &[ArgValue])] = &[(0, values::PROTOCOL_VERSIONS)];

/// A row whose one argument is drawn from a closed value set.
fn closed(
    name: &'static str,
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
    set: &'static [(u8, &'static [ArgValue])],
) -> CommandSpec {
    CommandSpec {
        arg_values: set,
        closed_value_args: &[0],
        ..text(name, summary, synopsis, snippet)
    }
}

/// A `BOOL` row.
fn boolean(
    name: &'static str,
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
) -> CommandSpec {
    closed(name, summary, synopsis, snippet, BOOL_ARG)
}

/// `predicate SCRIPT` — the one row whose word is a script.
///
/// Modelled exactly as a `SpecTcl` hook body is: the word is a body so it
/// folds and is not painted as data, and the row carries **no**
/// `definition_body`, so the walker drops out of declaration context for it.
/// Unlike a hook, this script is never evaluated at all — the loader retains
/// it verbatim.
fn predicate() -> CommandSpec {
    CommandSpec {
        traits: Traits::CREATES_BARRIER
            .union(Traits::NEVER_INLINE_BODY)
            .union(Traits::LANGUAGE_KEYWORD),
        arg_roles: &[(0, ArgRole::Body)],
        ..text(
            "predicate",
            "State a check condition the declarative members cannot express.",
            &["predicate { … }"],
            "One braced word, retained verbatim and **never evaluated** — not at load time, not at check time. It exists so a document can record a condition the vocabulary has no member for without the vocabulary growing an evaluator.",
        )
    }
}

/// The rows of a `certificate NAME { … }` block.
fn certificate_rows() -> Vec<CommandSpec> {
    vec![
        text(
            "pem",
            "Supply the enclosing certificate as PEM.",
            &["pem text"],
            "One literal word: the PEM text itself, or the path it is read from. Mutually exclusive with `material` in practice — a certificate states its bytes once.",
        ),
        text(
            "material",
            "Supply the enclosing certificate's raw material.",
            &["material text"],
            "One literal word carrying the certificate material directly, for a document that inlines rather than references it.",
        ),
        text(
            "key",
            "Name the private key the enclosing certificate is bound to.",
            &["key name"],
            "One name. The key itself is never part of a `.sslictcl` document — the name refers to material the deployment holds.",
        ),
    ]
}

/// The rows of an `endpoint NAME { … }` block that are not themselves blocks or references.
fn endpoint_rows() -> Vec<CommandSpec> {
    vec![
        text(
            "hostname",
            "Name the host the enclosing endpoint serves.",
            &["hostname text"],
            "One literal word: the DNS name presented in SNI and matched against the certificate. An endpoint states exactly one.",
        ),
        CommandSpec {
            // A version list, so the canonical spellings are offered inside
            // it; the word is a Tcl list, so the set is not closed.
            arg_values: PROTOCOL_VERSION_ARG,
            ..text(
                "protocols",
                "List the protocol versions in scope.",
                &["protocols {version …}"],
                "One braced Tcl list of literal words (a single bare word is accepted as a one-element list). Inside `endpoint` these are the versions the endpoint offers; inside `cipher` they are the versions the suite is defined for.",
            )
        },
        text(
            "ciphers",
            "List the cipher suites the enclosing endpoint offers.",
            &["ciphers {name …}"],
            "One braced Tcl list of cipher-suite names, in the order the endpoint prefers them.",
        ),
        text(
            "groups",
            "List the key-exchange groups the enclosing endpoint offers.",
            &["groups {name …}"],
            "One braced Tcl list of named groups (`x25519`, `secp256r1`, …), in preference order.",
        ),
        text(
            "signature-schemes",
            "List the signature schemes the enclosing endpoint offers.",
            &["signature-schemes {name …}"],
            "One braced Tcl list of signature-scheme names (`ecdsa_secp256r1_sha256`, `rsa_pss_rsae_sha256`, …).",
        ),
        text(
            "certificate-chain",
            "List the certificates the enclosing endpoint presents.",
            &["certificate-chain {name …}"],
            "One braced Tcl list of certificate names, leaf first. The alternative to naming a shared `chain`.",
        ),
    ]
}

/// The rows of an `hsts { … }` block.
fn hsts_rows() -> Vec<CommandSpec> {
    vec![
        boolean(
            "enabled",
            "State whether strict transport security is served.",
            &["enabled bool"],
            "When false, the remaining `hsts` members describe a posture that is declared but not served — which is what a `require-hsts` check reads.",
        ),
        text(
            "max-age",
            "State the strict-transport-security lifetime in seconds.",
            &["max-age int"],
            "An unsigned decimal number of seconds, the `max-age` directive verbatim. A `min-hsts-max-age` check compares against exactly this.",
        ),
        boolean(
            "include-subdomains",
            "State whether strict transport security covers subdomains.",
            &["include-subdomains bool"],
            "The `includeSubDomains` directive.",
        ),
        boolean(
            "preload",
            "State whether the host is submitted to the preload list.",
            &["preload bool"],
            "The `preload` directive. Declaring it is a statement about the deployment, not an action the document takes.",
        ),
    ]
}

/// The rows of a `testssl-import NAME { … }` block.
fn testssl_import_rows() -> Vec<CommandSpec> {
    vec![
        closed(
            "schema",
            "Pin the import format of the enclosing scan.",
            &["schema 1"],
            "The only defined value is `1`. A document naming another schema is rejected rather than half-read, because the payload's shape is what the number selects.",
            SCHEMA_ARG,
        ),
        text(
            "raw-json-hex",
            "Carry the imported scan's JSON as hex.",
            &["raw-json-hex hex"],
            "One literal word of even-length hex digits. Hex rather than the JSON itself so an arbitrary payload survives Tcl's quoting rules byte for byte and is never re-read as a declaration.",
        ),
    ]
}

/// The rows of a `trust-program NAME { … }` block.
fn trust_program_rows() -> Vec<CommandSpec> {
    vec![
        closed(
            "client",
            "Name the root program the enclosing trust program restates.",
            &["client name"],
            "The set is closed: a root program the vocabulary has no name for cannot be restated without a vocabulary revision, which is the point — a grader must know whose trust it is modelling.",
            CLIENT_ARG,
        ),
        text(
            "version",
            "State the version of the enclosing trust program.",
            &["version text"],
            "One literal word, the upstream program's own version string.",
        ),
        text(
            "generated-at",
            "State when the enclosing trust program was captured.",
            &["generated-at text"],
            "One literal word, an ISO-8601 timestamp. Provenance, not policy: nothing in the vocabulary compares it.",
        ),
        text(
            "source-name",
            "Name the source the enclosing trust program was taken from.",
            &["source-name text"],
            "One literal word (brace it when it contains spaces).",
        ),
        text(
            "source-url",
            "State the URL the enclosing trust program was taken from.",
            &["source-url text"],
            "One literal word. The document records where the data came from; nothing fetches it.",
        ),
        text(
            "source-revision",
            "State the upstream revision of the enclosing trust program.",
            &["source-revision text"],
            "One literal word — a commit id, a tag, or the upstream's own revision number.",
        ),
        text(
            "source-license",
            "State the licence the enclosing trust program's data carries.",
            &["source-license text"],
            "One literal word, an SPDX identifier where the upstream has one.",
        ),
    ]
}

/// The rows of an `anchor SHA256 { … }` block.
fn anchor_rows() -> Vec<CommandSpec> {
    vec![
        text(
            "subject",
            "State the enclosing anchor's subject.",
            &["subject text"],
            "One literal word (brace it — a distinguished name contains spaces and commas).",
        ),
        text(
            "der-base64",
            "Carry the enclosing anchor's certificate as base64 DER.",
            &["der-base64 text"],
            "One literal word. The SHA-256 over these bytes is the anchor's own name.",
        ),
        text(
            "purposes",
            "List the key purposes the enclosing anchor is trusted for.",
            &["purposes {name …}"],
            "One braced Tcl list (`server-auth`, `client-auth`, `email-protection`, …). An empty list is a distrusted anchor stated the long way.",
        ),
        boolean(
            "trusted",
            "State whether the enclosing anchor is trusted.",
            &["trusted bool"],
            "A root program lists anchors it has distrusted as well as ones it trusts, so the flag is stated rather than implied by presence.",
        ),
        text(
            "distrust-after",
            "State when the enclosing anchor stops being trusted.",
            &["distrust-after int"],
            "An unsigned decimal Unix timestamp. Certificates issued after it are not trusted even while the anchor itself is.",
        ),
    ]
}

/// The rows of a `protocol VERSION { … }` block — `status` is shared with `cipher`.
fn protocol_rows() -> Vec<CommandSpec> {
    vec![
        closed(
            "status",
            "Rate the enclosing protocol version or cipher suite.",
            &["status recommended|acceptable|deprecated|prohibited"],
            "One word from a closed set. The same word rates a `protocol` and a `cipher`, which is why it is one row and not two.",
            STATUS_ARG,
        ),
        text(
            "score",
            "Score the enclosing protocol version out of 100.",
            &["score int"],
            "An unsigned decimal in 0..=100. A grader weighs the offered versions by these.",
        ),
        text(
            "reference",
            "Cite the document that rates the enclosing protocol version.",
            &["reference text"],
            "One literal word (brace it when it contains spaces) — an RFC number, a standards-body reference, or a URL.",
        ),
    ]
}

/// The rows a `cipher NAME { … }` block does not share with `protocol` or `endpoint`.
fn cipher_rows() -> Vec<CommandSpec> {
    vec![
        text(
            "iana-name",
            "State the enclosing cipher suite's IANA name.",
            &["iana-name text"],
            "The registry spelling — `TLS_AES_128_GCM_SHA256`.",
        ),
        text(
            "openssl-name",
            "State the enclosing cipher suite's OpenSSL name.",
            &["openssl-name text"],
            "The OpenSSL spelling — `ECDHE-RSA-AES128-GCM-SHA256`. Stated separately because the two vocabularies do not agree.",
        ),
        text(
            "key-exchange",
            "State the enclosing cipher suite's key exchange.",
            &["key-exchange text"],
            "`ECDHE`, `DHE`, `RSA`, or `any` for a TLS 1.3 suite that leaves it to the handshake.",
        ),
        text(
            "authentication",
            "State the enclosing cipher suite's authentication algorithm.",
            &["authentication text"],
            "`RSA`, `ECDSA`, or `any` for a TLS 1.3 suite that leaves it to the certificate.",
        ),
        text(
            "encryption",
            "State the enclosing cipher suite's bulk cipher.",
            &["encryption text"],
            "`AESGCM`, `CHACHA20`, `AESCBC`, …",
        ),
        text(
            "bits",
            "State the enclosing cipher suite's effective key size.",
            &["bits int"],
            "An unsigned decimal number of bits of the bulk cipher's key — 128, 256.",
        ),
        boolean(
            "forward-secrecy",
            "State whether the enclosing cipher suite offers forward secrecy.",
            &["forward-secrecy bool"],
            "Read directly by a `require-forward-secrecy` check.",
        ),
        boolean(
            "aead",
            "State whether the enclosing cipher suite is authenticated encryption.",
            &["aead bool"],
            "True for GCM, CCM, and ChaCha20-Poly1305 suites; false for the CBC ones.",
        ),
    ]
}

/// The one row of a `chain NAME { … }` block.
fn chain_rows() -> Vec<CommandSpec> {
    vec![text(
        "certificates",
        "List the certificates of the enclosing chain.",
        &["certificates {name …}"],
        "One braced Tcl list of certificate names, leaf to root. Each names a `certificate` declared elsewhere in the document.",
    )]
}

/// The rows of a `check ID { … }` block.
fn check_rows() -> Vec<CommandSpec> {
    vec![
        closed(
            "severity",
            "State how a failure of the enclosing check is reported.",
            &["severity info|warning|error|critical"],
            "One word from a closed set. `critical` overrides the graded result rather than contributing to it.",
            SEVERITY_ARG,
        ),
        text(
            "message",
            "State what a failure of the enclosing check says.",
            &["message text"],
            "One literal word — brace it, since a readable message contains spaces. Shown verbatim; nothing is substituted into it.",
        ),
        text(
            "require-protocols",
            "Require the endpoint to offer these protocol versions.",
            &["require-protocols {version …}"],
            "One braced Tcl list. The check fails when a listed version is absent from the endpoint's `protocols`.",
        ),
        text(
            "forbid-protocols",
            "Forbid the endpoint from offering these protocol versions.",
            &["forbid-protocols {version …}"],
            "One braced Tcl list. The check fails when a listed version is present in the endpoint's `protocols`.",
        ),
        text(
            "forbid-ciphers",
            "Forbid the endpoint from offering these cipher suites.",
            &["forbid-ciphers {name …}"],
            "One braced Tcl list of cipher-suite names.",
        ),
        boolean(
            "require-forward-secrecy",
            "Require every offered cipher suite to give forward secrecy.",
            &["require-forward-secrecy bool"],
            "Reads each offered suite's `forward-secrecy` member.",
        ),
        text(
            "min-key-bits",
            "Require at least this many key bits.",
            &["min-key-bits int"],
            "An unsigned decimal. The check fails when the endpoint's key is smaller.",
        ),
        boolean(
            "require-hsts",
            "Require the endpoint to serve strict transport security.",
            &["require-hsts bool"],
            "Reads the endpoint's `hsts` block: the check fails when it is absent or its `enabled` is false.",
        ),
        text(
            "min-hsts-max-age",
            "Require a strict-transport-security lifetime of at least this many seconds.",
            &["min-hsts-max-age int"],
            "An unsigned decimal compared against the endpoint's `hsts` `max-age`.",
        ),
        predicate(),
    ]
}

/// The one row of a `grade { … }` block.
fn grade_rows() -> Vec<CommandSpec> {
    vec![closed(
        "minimum",
        "State the lowest grade the enclosing policy accepts.",
        &["minimum A+|A|B|C|D|E|F"],
        "One word from a closed set, best (`A+`) to worst (`F`). An endpoint graded below it fails its policy.",
        GRADE_ARG,
    )]
}

/// Every member row of every `SslicTcl` block.
pub(super) fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(certificate_rows());
    specs.extend(endpoint_rows());
    specs.extend(hsts_rows());
    specs.extend(testssl_import_rows());
    specs.extend(trust_program_rows());
    specs.extend(anchor_rows());
    specs.extend(protocol_rows());
    specs.extend(cipher_rows());
    specs.extend(chain_rows());
    specs.extend(check_rows());
    specs.extend(grade_rows());
    specs
}
