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

//! The `SslicTcl` statements that take a **block**, plus the mandatory
//! `sslictcl VERSION` header.
//!
//! Each block statement carries a `definition_body` grammar naming the words
//! legal inside it, which is what makes the vocabulary context-sensitive and
//! gives every block folding, keyword painting, and completion generically.
//!
//! Three shapes appear here:
//!
//! * a bare block (`hsts { … }`, `grade { … }`) — the block is argument 0;
//! * a named block (`endpoint NAME { … }`, `check ID { … }`) — the name
//!   comes first;
//! * a named block that is also a **reference** (`chain NAME`,
//!   `policy NAME`), which is how an `endpoint` names a chain or a policy
//!   declared elsewhere. Its arity therefore starts at 1, and the body role
//!   at index 1 is only claimed when a second word is actually present.

use crate::prelude::*;

use super::SOURCE;
use super::values;

/// The canonical protocol-version spellings, offered on `protocol`'s name
/// word. Deliberately not closed: the loader normalises documented aliases.
const PROTOCOL_VERSION_ARG: &[(u8, &[ArgValue])] = &[(0, values::PROTOCOL_VERSIONS)];
use tcl_dialect::model::SpecSurface;

/// The traits every block statement carries: a declaration keyword whose
/// braced word is a structural body, never inlined into the caller.
const BLOCK_TRAITS: Traits = Traits::CREATES_BARRIER
    .union(Traits::NEVER_INLINE_BODY)
    .union(Traits::LANGUAGE_KEYWORD);

/// A bare-block statement: `hsts { … }`.
fn block(
    name: &'static str,
    grammar: &'static DefinitionBodyGrammar,
    hover: HoverSnippet,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: BLOCK_TRAITS,
        surface: Some(SpecSurface::SSLICTCL),
        arity: Arity::exact(1),
        hover: Some(hover),
        arg_roles: &[(0, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        definition_body: Some(grammar),
        ..CommandSpec::DEFAULT
    }
}

/// A named-block statement: `endpoint NAME { … }`.
fn named_block(
    name: &'static str,
    grammar: &'static DefinitionBodyGrammar,
    hover: HoverSnippet,
) -> CommandSpec {
    CommandSpec {
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        ..block(name, grammar, hover)
    }
}

/// A named block that is also a reference form — `chain NAME` inside an
/// `endpoint`, `chain NAME { … }` at the top level.
fn named_block_or_reference(
    name: &'static str,
    grammar: &'static DefinitionBodyGrammar,
    hover: HoverSnippet,
) -> CommandSpec {
    CommandSpec {
        arity: Arity::new(1, 2),
        ..named_block(name, grammar, hover)
    }
}

const fn hover(
    summary: &'static str,
    synopsis: &'static [&'static str],
    snippet: &'static str,
    examples: &'static str,
) -> HoverSnippet {
    HoverSnippet {
        summary,
        synopsis,
        snippet,
        source: SOURCE,
        examples,
        return_value: "",
    }
}

/// `sslictcl VERSION` — the mandatory first declaration of every document.
fn header() -> CommandSpec {
    CommandSpec {
        arg_roles: &[(0, ArgRole::Keyword)],
        ..super::statement(
            "sslictcl",
            Arity::exact(1),
            "Declare the vocabulary version this document is written against.",
            &["sslictcl version"],
            "The mandatory header, and the first declaration in the file. VERSION is a plain integer naming the vocabulary revision — a document stating a version the loader does not know is rejected rather than half-read. The word appears nowhere else in the vocabulary, which is what lets dialect detection recognise a document saved under a `.tcl` name.",
        )
    }
}

/// The blocks that describe a deployment: what is served, and from what.
fn deployment_blocks() -> Vec<CommandSpec> {
    vec![
        named_block(
            "certificate",
            &crate::definer::SSLICTCL_CERTIFICATE_GRAMMAR,
            hover(
                "Declare a certificate and the key material it is bound to.",
                &["certificate name { … }"],
                "An open block: `pem` or `material` carries the certificate itself and `key` names the private key it belongs to. Unknown members are retained as forwards-compatibility notices rather than rejected.",
                "certificate www-leaf {\n    pem /etc/tls/www.example.com.pem\n    key www-example-com\n}",
            ),
        ),
        named_block(
            "endpoint",
            &crate::definer::SSLICTCL_ENDPOINT_GRAMMAR,
            hover(
                "Declare a TLS endpoint and the surface it offers.",
                &["endpoint name { … }"],
                "An open block, and the centre of a document: `hostname` names what is served, the four list members (`protocols`, `ciphers`, `groups`, `signature-schemes`) state the offered surface, `certificate-chain` or `chain` supplies the chain, `policy` names the policy that grades it, and a nested `hsts { … }` states the strict-transport headers.",
                "endpoint www {\n    hostname www.example.com\n    protocols {tls1.2 tls1.3}\n    ciphers {TLS_AES_128_GCM_SHA256 TLS_AES_256_GCM_SHA384}\n    chain www-chain\n    policy baseline\n    hsts {\n        enabled true\n        max-age 31536000\n    }\n}",
            ),
        ),
        block(
            "hsts",
            &crate::definer::SSLICTCL_HSTS_GRAMMAR,
            hover(
                "Declare the HTTP Strict-Transport-Security posture of the enclosing endpoint.",
                &["hsts { … }"],
                "A closed block of four scalars: `enabled`, `max-age`, `include-subdomains`, and `preload`. `require-hsts` and `min-hsts-max-age` checks read exactly these.",
                "hsts {\n    enabled true\n    max-age 63072000\n    include-subdomains true\n    preload false\n}",
            ),
        ),
        named_block(
            "testssl-import",
            &crate::definer::SSLICTCL_TESTSSL_IMPORT_GRAMMAR,
            hover(
                "Carry an imported testssl.sh scan verbatim.",
                &["testssl-import name { … }"],
                "A closed block of exactly two members: `schema 1` pins the import format and `raw-json-hex` carries the scan's JSON as hex, so an arbitrary payload survives the Tcl quoting rules byte for byte and is never re-parsed as a declaration.",
                "testssl-import nightly-scan {\n    schema 1\n    raw-json-hex 7b22736361...\n}",
            ),
        ),
        named_block(
            "trust-program",
            &crate::definer::SSLICTCL_TRUST_PROGRAM_GRAMMAR,
            hover(
                "Declare a root program and the anchors it carries.",
                &["trust-program name { … }"],
                "An open block: `client` names the root program, the `source-*` and `generated-at` members record provenance, and each `anchor SHA256 { … }` restates one root by the SHA-256 of its DER.",
                "trust-program mozilla-2026-06 {\n    client mozilla\n    version 2.72\n    generated-at 2026-06-01T00:00:00Z\n    source-name {Trust Stores Observatory}\n    anchor 0b1c2d...64hex {\n        subject {CN=Example Root CA}\n        trusted true\n    }\n}",
            ),
        ),
        named_block(
            "anchor",
            &crate::definer::SSLICTCL_ANCHOR_GRAMMAR,
            hover(
                "Declare one trust anchor of the enclosing trust program.",
                &["anchor sha256 { … }"],
                "A closed block. The name word is the 64 hex digits of the SHA-256 over the anchor's DER, which is the anchor's identity — `der-base64` carries the certificate itself, `purposes` the key purposes it is trusted for, and `distrust-after` a distrust date as a Unix timestamp.",
                "anchor 0b1c2d3e4f506172839405a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7f80 {\n    subject {CN=Example Root CA, O=Example}\n    purposes {server-auth}\n    trusted true\n}",
            ),
        ),
    ]
}

/// The catalogue blocks: what the protocol versions and cipher suites a
/// deployment offers are worth.
fn rating_blocks() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            // The canonical spellings, offered but deliberately not closed:
            // the loader normalises documented aliases onto them.
            arg_values: PROTOCOL_VERSION_ARG,
            ..named_block(
                "protocol",
                &crate::definer::SSLICTCL_PROTOCOL_GRAMMAR,
                hover(
                    "Rate one protocol version in the document's catalogue.",
                    &["protocol version { … }"],
                    "A closed block of three members: `status`, `score` (0-100), and `reference`. The name word is a protocol version — `ssl2`, `ssl3`, `tls1.0`, `tls1.1`, `tls1.2`, `tls1.3` are the canonical spellings, and the loader normalises the common aliases onto them.",
                    "protocol tls1.0 {\n    status deprecated\n    score 20\n    reference {RFC 8996}\n}",
                ),
            )
        },
        named_block(
            "cipher",
            &crate::definer::SSLICTCL_CIPHER_GRAMMAR,
            hover(
                "Describe and rate one cipher suite.",
                &["cipher name { … }"],
                "A closed block. The two name members give the IANA and OpenSSL spellings, the four algorithm members decompose the suite, `bits` / `forward-secrecy` / `aead` state its strength properties, `status` rates it, and `protocols` lists the versions it is defined for.",
                "cipher TLS_AES_128_GCM_SHA256 {\n    iana-name TLS_AES_128_GCM_SHA256\n    key-exchange any\n    encryption AESGCM\n    bits 128\n    forward-secrecy true\n    aead true\n    status recommended\n    protocols {tls1.3}\n}",
            ),
        ),
    ]
}

/// The chain and policy blocks — the two that are also *reference* forms
/// inside an `endpoint`, plus the policy's own nested blocks.
fn policy_blocks() -> Vec<CommandSpec> {
    vec![
        named_block_or_reference(
            "chain",
            &crate::definer::SSLICTCL_CHAIN_GRAMMAR,
            hover(
                "Declare a certificate chain, or name one from an endpoint.",
                &["chain name { … }", "chain name"],
                "At the top level a closed block whose one member, `certificates`, lists the certificate names in leaf-to-root order. Inside an `endpoint` the same word is a *reference*: one bare name and no block.",
                "chain www-chain {\n    certificates {www-leaf example-intermediate}\n}",
            ),
        ),
        named_block_or_reference(
            "policy",
            &crate::definer::SSLICTCL_POLICY_GRAMMAR,
            hover(
                "Declare an assurance policy, or name one from an endpoint.",
                &["policy name { … }", "policy name"],
                "At the top level a closed block of `check ID { … }` rows and one `grade { … }` block. Inside an `endpoint` the same word is a *reference*: one bare name and no block.",
                "policy baseline {\n    check no-legacy-tls {\n        severity error\n        message {TLS 1.0 and 1.1 must not be offered}\n        forbid-protocols {tls1.0 tls1.1}\n    }\n    grade {\n        minimum A\n    }\n}",
            ),
        ),
        named_block(
            "check",
            &crate::definer::SSLICTCL_CHECK_GRAMMAR,
            hover(
                "Declare one check of the enclosing policy.",
                &["check id { … }"],
                "A closed block. `severity` and `message` say how a failure is reported; the `require-*` / `forbid-*` / `min-*` members state the condition declaratively; `predicate` states one that the declarative members cannot, as a braced script the loader retains verbatim and never evaluates.",
                "check strong-keys {\n    severity warning\n    message {RSA keys must be at least 3072 bits}\n    min-key-bits 3072\n}",
            ),
        ),
        block(
            "grade",
            &crate::definer::SSLICTCL_GRADE_GRAMMAR,
            hover(
                "Declare the grade the enclosing policy demands.",
                &["grade { … }"],
                "A closed block with one member, `minimum`, naming the lowest acceptable grade from `A+` down to `F`.",
                "grade {\n    minimum A\n}",
            ),
        ),
    ]
}

/// Every block-shaped `SslicTcl` statement, plus the header.
pub(super) fn specs() -> Vec<CommandSpec> {
    let mut specs = vec![header()];
    specs.extend(deployment_blocks());
    specs.extend(rating_blocks());
    specs.extend(policy_blocks());
    specs
}
