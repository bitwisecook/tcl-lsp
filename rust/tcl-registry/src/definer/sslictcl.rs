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

//! `SslicTcl`: the member tables and grammars of the `.sslictcl` TLS-assurance DSL.

use super::{DefinerFamily, DefinitionBodyGrammar, MemberSpec, setting, setting_word};
use crate::arg_role::ArgRole;

// SslicTcl — the `.sslictcl` TLS-assurance declaration DSL.
//
// Every block statement's member table is exactly the row vocabulary of that
// block, and a *nested* block (`hsts`, `anchor`, `check`, `grade`) appears
// twice: once as a member row of its parent's grammar, and once as its own
// statement carrying its own grammar. That pairing is what lets the shared
// walker switch vocabularies on the way down with no walker changes.

/// A nested bare-block member row: `hsts { … }`, `grade { … }`.
pub(super) const SSLICTCL_BLOCK_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Body)];

/// A nested named-block member row: `anchor SHA256 { … }`, `check ID { … }`.
pub(super) const SSLICTCL_NAMED_BLOCK_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::Name), (1, ArgRole::Body)];

/// A member row whose one word is a retained, never-evaluated script.
pub(super) const SSLICTCL_OPAQUE_SCRIPT_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::OpaqueScript)];

/// The rows of a `certificate NAME { … }` block.
pub(super) const SSLICTCL_CERTIFICATE_MEMBERS: &[MemberSpec] = &[
    setting_word("pem"),
    setting_word("material"),
    setting_word("key"),
];

/// The rows of an `endpoint NAME { … }` block.
pub(super) const SSLICTCL_ENDPOINT_MEMBERS: &[MemberSpec] = &[
    setting_word("hostname"),
    setting_word("protocols"),
    setting_word("ciphers"),
    setting_word("groups"),
    setting_word("signature-schemes"),
    setting_word("certificate-chain"),
    setting_word("chain"),
    setting_word("policy"),
    setting("hsts", SSLICTCL_BLOCK_ROLES),
];

/// The rows of an `hsts { … }` block.
pub(super) const SSLICTCL_HSTS_MEMBERS: &[MemberSpec] = &[
    setting_word("enabled"),
    setting_word("max-age"),
    setting_word("include-subdomains"),
    setting_word("preload"),
];

/// The rows of a `testssl-import NAME { … }` block.
pub(super) const SSLICTCL_TESTSSL_IMPORT_MEMBERS: &[MemberSpec] =
    &[setting_word("schema"), setting_word("raw-json-hex")];

/// The rows of a `trust-program NAME { … }` block.
pub(super) const SSLICTCL_TRUST_PROGRAM_MEMBERS: &[MemberSpec] = &[
    setting_word("client"),
    setting_word("version"),
    setting_word("generated-at"),
    setting_word("source-name"),
    setting_word("source-url"),
    setting_word("source-revision"),
    setting_word("source-license"),
    setting("anchor", SSLICTCL_NAMED_BLOCK_ROLES),
];

/// The rows of an `anchor SHA256 { … }` block.
pub(super) const SSLICTCL_ANCHOR_MEMBERS: &[MemberSpec] = &[
    setting_word("subject"),
    setting_word("der-base64"),
    setting_word("purposes"),
    setting_word("trusted"),
    setting_word("distrust-after"),
];

/// The rows of a `protocol VERSION { … }` block.
pub(super) const SSLICTCL_PROTOCOL_MEMBERS: &[MemberSpec] = &[
    setting_word("status"),
    setting_word("score"),
    setting_word("reference"),
];

/// The rows of a `cipher NAME { … }` block.
pub(super) const SSLICTCL_CIPHER_MEMBERS: &[MemberSpec] = &[
    setting_word("iana-name"),
    setting_word("openssl-name"),
    setting_word("key-exchange"),
    setting_word("authentication"),
    setting_word("encryption"),
    setting_word("bits"),
    setting_word("forward-secrecy"),
    setting_word("aead"),
    setting_word("status"),
    setting_word("protocols"),
];

/// The one row of a `chain NAME { … }` block.
pub(super) const SSLICTCL_CHAIN_MEMBERS: &[MemberSpec] = &[setting_word("certificates")];

/// The rows of a `policy NAME { … }` block.
pub(super) const SSLICTCL_POLICY_MEMBERS: &[MemberSpec] = &[
    setting("check", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("grade", SSLICTCL_BLOCK_ROLES),
];

/// The rows of a `check ID { … }` block. `predicate` carries a braced script
/// the loader retains verbatim and never evaluates, so its word is an
/// [`ArgRole::OpaqueScript`]: it folds like a body, and no analysis enters it.
pub(super) const SSLICTCL_CHECK_MEMBERS: &[MemberSpec] = &[
    setting_word("severity"),
    setting_word("message"),
    setting_word("require-protocols"),
    setting_word("forbid-protocols"),
    setting_word("forbid-ciphers"),
    setting_word("require-forward-secrecy"),
    setting_word("min-key-bits"),
    setting_word("require-hsts"),
    setting_word("min-hsts-max-age"),
    setting("predicate", SSLICTCL_OPAQUE_SCRIPT_ROLES),
];

/// The one row of a `grade { … }` block.
pub(super) const SSLICTCL_GRADE_MEMBERS: &[MemberSpec] = &[setting_word("minimum")];

/// Build one `SslicTcl` grammar from its member table. Every field a class
/// system uses is empty: a TLS declaration manufactures nothing, dispatches
/// nothing, and injects no commands or variables into its blocks.
pub(super) const fn sslictcl_grammar(members: &'static [MemberSpec]) -> DefinitionBodyGrammar {
    DefinitionBodyGrammar {
        family: DefinerFamily::SslicTcl,
        profile: super::DECLARATION_PROFILE,
        members,
        implicit_vars: &[],
        implicit_receiver_vars: &[],
        member_body_namespace_path: &[],
        builtin_type_methods: &[],
        builtin_object_methods: &[],
        builtin_terminating_methods: &[],
        member_body_commands: &[],
        bare_word_construction: false,
        bare_word_construction_hint: None,
        bare_word_construction_source_hint: None,
        dynamic_method_dispatch: false,
        manufacturers: &[],
        unknown_dispatch_method: None,
        property_accessor_methods: &[],
    }
}

/// The body of `certificate NAME { … }`.
pub const SSLICTCL_CERTIFICATE_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_CERTIFICATE_MEMBERS);

/// The body of `endpoint NAME { … }`.
pub const SSLICTCL_ENDPOINT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_ENDPOINT_MEMBERS);

/// The body of `hsts { … }`.
pub const SSLICTCL_HSTS_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_HSTS_MEMBERS);

/// The body of `testssl-import NAME { … }`.
pub const SSLICTCL_TESTSSL_IMPORT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_TESTSSL_IMPORT_MEMBERS);

/// The body of `trust-program NAME { … }`.
pub const SSLICTCL_TRUST_PROGRAM_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_TRUST_PROGRAM_MEMBERS);

/// The body of `anchor SHA256 { … }`.
pub const SSLICTCL_ANCHOR_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_ANCHOR_MEMBERS);

/// The body of `protocol VERSION { … }`.
pub const SSLICTCL_PROTOCOL_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_PROTOCOL_MEMBERS);

/// The body of `cipher NAME { … }`.
pub const SSLICTCL_CIPHER_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_CIPHER_MEMBERS);

/// The body of `chain NAME { … }`.
pub const SSLICTCL_CHAIN_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_CHAIN_MEMBERS);

/// The body of `policy NAME { … }`.
pub const SSLICTCL_POLICY_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_POLICY_MEMBERS);

/// The body of `check ID { … }`.
pub const SSLICTCL_CHECK_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_CHECK_MEMBERS);

/// The body of `grade { … }`.
pub const SSLICTCL_GRADE_GRAMMAR: DefinitionBodyGrammar = sslictcl_grammar(SSLICTCL_GRADE_MEMBERS);

/// The nine declarations legal at the **root** of a `.sslictcl` document.
///
/// A document is itself a definition body: `hostname` is a member of
/// `endpoint` and of nothing else, and by exactly the same rule `endpoint` is
/// a member of the *document* and `hostname` is not. Giving the root a grammar
/// is what lets the generic consumers answer both halves the same way —
/// completion offers these nine at the top level, and the token walker paints
/// them from membership rather than from a global keyword trait that would
/// also fire for a misplaced member row.
pub(super) const SSLICTCL_DOCUMENT_MEMBERS: &[MemberSpec] = &[
    // The header names nothing and opens nothing: its word is a version.
    setting_word("sslictcl"),
    setting("certificate", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("endpoint", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("testssl-import", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("trust-program", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("protocol", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("cipher", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("chain", SSLICTCL_NAMED_BLOCK_ROLES),
    setting("policy", SSLICTCL_NAMED_BLOCK_ROLES),
];

/// The body of a `.sslictcl` **document** — see [`SSLICTCL_DOCUMENT_MEMBERS`].
pub const SSLICTCL_DOCUMENT_GRAMMAR: DefinitionBodyGrammar =
    sslictcl_grammar(SSLICTCL_DOCUMENT_MEMBERS);

/// Every `SslicTcl` grammar, so a sweep can assert the family's invariants
/// without enumerating the constants by hand.
pub const SSLICTCL_GRAMMARS: &[&DefinitionBodyGrammar] = &[
    &SSLICTCL_DOCUMENT_GRAMMAR,
    &SSLICTCL_CERTIFICATE_GRAMMAR,
    &SSLICTCL_ENDPOINT_GRAMMAR,
    &SSLICTCL_HSTS_GRAMMAR,
    &SSLICTCL_TESTSSL_IMPORT_GRAMMAR,
    &SSLICTCL_TRUST_PROGRAM_GRAMMAR,
    &SSLICTCL_ANCHOR_GRAMMAR,
    &SSLICTCL_PROTOCOL_GRAMMAR,
    &SSLICTCL_CIPHER_GRAMMAR,
    &SSLICTCL_CHAIN_GRAMMAR,
    &SSLICTCL_POLICY_GRAMMAR,
    &SSLICTCL_CHECK_GRAMMAR,
    &SSLICTCL_GRADE_GRAMMAR,
];
