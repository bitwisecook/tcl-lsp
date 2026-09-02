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

//! The `SslicTcl` pack: the `.sslictcl` TLS-assurance vocabulary as registry
//! data.
//!
//! registry-metadata: every assertion here reads registry data or the
//! vocabulary table in `docs/design/sslictcl-vocabulary.md`, not C-Tcl
//! behaviour — `SslicTcl` is our own DSL, so the table *is* its oracle.
use tcl_dialect::model::SpecSurface;
use tcl_registry::ArgRole;
use tcl_registry::definer::{DefinerFamily, SSLICTCL_GRAMMARS};
use tcl_registry::model::ingress::static_context_for;

/// The complete `(statement → members)` map of the vocabulary, in the order
/// the design table states it. The pack and the table cannot drift apart
/// without this failing.
const VOCABULARY: &[(&str, &[&str])] = &[
    ("certificate", &["pem", "material", "key"]),
    (
        "endpoint",
        &[
            "hostname",
            "protocols",
            "ciphers",
            "groups",
            "signature-schemes",
            "certificate-chain",
            "chain",
            "policy",
            "hsts",
        ],
    ),
    (
        "hsts",
        &["enabled", "max-age", "include-subdomains", "preload"],
    ),
    ("testssl-import", &["schema", "raw-json-hex"]),
    (
        "trust-program",
        &[
            "client",
            "version",
            "generated-at",
            "source-name",
            "source-url",
            "source-revision",
            "source-license",
            "anchor",
        ],
    ),
    (
        "anchor",
        &[
            "subject",
            "der-base64",
            "purposes",
            "trusted",
            "distrust-after",
        ],
    ),
    ("protocol", &["status", "score", "reference"]),
    (
        "cipher",
        &[
            "iana-name",
            "openssl-name",
            "key-exchange",
            "authentication",
            "encryption",
            "bits",
            "forward-secrecy",
            "aead",
            "status",
            "protocols",
        ],
    ),
    ("chain", &["certificates"]),
    ("policy", &["check", "grade"]),
    (
        "check",
        &[
            "severity",
            "message",
            "require-protocols",
            "forbid-protocols",
            "forbid-ciphers",
            "require-forward-secrecy",
            "min-key-bits",
            "require-hsts",
            "min-hsts-max-age",
            "predicate",
        ],
    ),
    ("grade", &["minimum"]),
];

/// The pack loads by profile identity, through exactly the same
/// `base_layers` path every other dialect pack takes.
#[test]
fn the_pack_loads_for_the_sslictcl_profile_and_nowhere_else() {
    let sslictcl = static_context_for("sslictcl").commands();
    assert!(
        sslictcl.get("sslictcl").is_some(),
        "the sslictcl registry carries the pack's header statement"
    );
    // Base Tcl stays underneath: a `.sslictcl` document is an ordinary Tcl
    // script, and the grammar is what says a word is not a declaration.
    for core in ["set", "if", "string", "lindex"] {
        assert!(
            sslictcl.get(core).is_some(),
            "{core} must still resolve inside a document"
        );
    }
    // …and the declaration words reach no other dialect. `sslictcl` is the
    // one word that can only ever be this vocabulary's, so it is the honest
    // probe, and the generic ones are checked separately below.
    for other in ["tcl8.6", "tcl9.0", "f5-irules", "expect", "bpf", "spectcl"] {
        assert!(
            static_context_for(other)
                .commands()
                .get("sslictcl")
                .is_none(),
            "{other} must not see SslicTcl's statement words"
        );
    }
}

/// Extremely generic declaration words resolve to the `SslicTcl` spec under
/// the `SslicTcl` profile, and never leak the other way — `message` is a live
/// collision with Tk's widget command.
#[test]
fn generic_declaration_words_do_not_collide_across_dialects() {
    let sslictcl = static_context_for("sslictcl").commands();
    let mask = Some(
        tcl_registry::model::ingress::resolve_environment("sslictcl")
            .analyser_profile()
            .surface_query(),
    );
    for word in [
        "chain",
        "policy",
        "protocol",
        "cipher",
        "certificate",
        "check",
        "grade",
        "message",
        "status",
        "version",
        "key",
    ] {
        let spec = sslictcl
            .get_for_surface(word, mask)
            .unwrap_or_else(|| panic!("{word} resolves under sslictcl"));
        assert_eq!(
            spec.surface,
            Some(SpecSurface::SSLICTCL),
            "{word} must resolve to the SslicTcl spec, not another pack's"
        );
    }
    // Plain Tcl 9.0 never sees any of them, and keeps Tk's own `message`.
    let tcl = static_context_for("tcl9.0").commands();
    for word in [
        "chain",
        "policy",
        "protocol",
        "cipher",
        "certificate",
        "check",
        "grade",
    ] {
        assert!(
            tcl.get(word).is_none(),
            "`{word}` must not exist in plain Tcl"
        );
    }
    let tk_message = tcl.get("message").expect("Tk `message` still exists");
    assert_ne!(tk_message.surface, Some(SpecSurface::SSLICTCL));
}

/// Every `SslicTcl` declaration word is gated to the `SslicTcl` surface
/// alone, and no word is declared twice.
#[test]
fn every_declaration_word_is_gated_to_sslictcl() {
    let specs = tcl_registry::commands::sslictcl::sslictcl_command_specs();
    assert!(
        specs.len() > 50,
        "the pack covers the whole vocabulary, got {}",
        specs.len()
    );
    for spec in &specs {
        assert_eq!(
            spec.surface,
            Some(SpecSurface::SSLICTCL),
            "{}: declaration words are SslicTcl-only",
            spec.name
        );
        assert!(
            spec.traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD),
            "{}: every declaration word is a language keyword",
            spec.name
        );
        let hover = spec
            .hover
            .unwrap_or_else(|| panic!("{}: every word documents itself", spec.name));
        assert!(!hover.summary.is_empty(), "{}: summary", spec.name);
        assert!(!hover.synopsis.is_empty(), "{}: synopsis", spec.name);
        assert!(!hover.snippet.is_empty(), "{}: description", spec.name);
        assert_eq!(
            hover.source, "SslicTcl (docs/design/sslictcl-vocabulary.md)",
            "{}: source",
            spec.name
        );
    }
    let mut names: Vec<&str> = specs.iter().map(|s| s.name).collect();
    names.sort_unstable();
    let unique = names.len();
    names.dedup();
    assert_eq!(unique, names.len(), "a declaration word is declared twice");
}

/// The vocabulary table itself: every block statement carries the grammar of
/// its own body, that grammar's family is `SslicTcl`, and its member set is
/// **exactly** the table's row — no more and no less.
#[test]
fn the_vocabulary_table_is_the_pack() {
    let reg = static_context_for("sslictcl").commands();
    for &(statement, members) in VOCABULARY {
        let grammar = reg
            .get(statement)
            .unwrap_or_else(|| panic!("{statement} is a registered statement"))
            .definition_body
            .unwrap_or_else(|| panic!("{statement} carries its body's grammar"));
        assert_eq!(grammar.family, DefinerFamily::SslicTcl, "{statement}");
        let declared: Vec<&str> = grammar.members.iter().map(|m| m.keyword).collect();
        assert_eq!(
            declared, members,
            "{statement}'s member table must be exactly the vocabulary row"
        );
        for member in members {
            assert!(
                grammar.is_member(member),
                "{statement}'s grammar must know `{member}`"
            );
            assert!(
                reg.get(member).is_some(),
                "`{member}` is a member of {statement} but has no spec of its own"
            );
        }
    }
    // Every word the pack declares is either a block statement of the table,
    // the header, or a member of some block — nothing floats free.
    let table_blocks: Vec<&str> = VOCABULARY.iter().map(|&(name, _)| name).collect();
    for spec in tcl_registry::commands::sslictcl::sslictcl_command_specs() {
        let known = spec.name == "sslictcl"
            || table_blocks.contains(&spec.name)
            || VOCABULARY
                .iter()
                .any(|&(_, members)| members.contains(&spec.name));
        assert!(known, "`{}` belongs to no block of the table", spec.name);
    }
}

/// The nesting chain, as arities: a block claims its body word, a nested
/// block claims its own, and the two reference forms (`chain` / `policy`
/// inside an `endpoint`) claim no body when only a name is written.
#[test]
fn block_statements_claim_their_body_word() {
    let reg = static_context_for("sslictcl").commands();
    for named in [
        "certificate",
        "endpoint",
        "testssl-import",
        "trust-program",
        "anchor",
        "protocol",
        "cipher",
        "chain",
        "policy",
        "check",
    ] {
        let call = ["a-name", "{ }"];
        assert_eq!(
            reg.arg_indices_for_role(named, &call, ArgRole::Name),
            vec![0],
            "{named}: the first word names the declaration"
        );
        assert_eq!(
            reg.arg_indices_for_role(named, &call, ArgRole::Body),
            vec![1],
            "{named}: the braced word is the body"
        );
    }
    for bare in ["hsts", "grade"] {
        assert_eq!(
            reg.arg_indices_for_role(bare, &["{ }"], ArgRole::Body),
            vec![0],
            "{bare}: a bare block's body is argument 0"
        );
    }
    // `chain NAME` / `policy NAME` inside an endpoint are references, so
    // there is no body to claim and the arity still admits them.
    for reference in ["chain", "policy"] {
        assert!(
            reg.arg_indices_for_role(reference, &["www-chain"], ArgRole::Body)
                .is_empty(),
            "{reference}: a bare reference claims no block"
        );
        let spec = reg.get(reference).expect("registered statement");
        assert!(
            spec.arity.accepts(1) && spec.arity.accepts(2),
            "{reference}: both the reference and the declaration form"
        );
    }
    // The header takes exactly one word and opens nothing.
    let header = reg.get("sslictcl").expect("the header statement");
    assert!(header.definition_body.is_none());
    assert!(header.arity.accepts(1) && !header.arity.accepts(2));
}

/// `predicate { … }` carries `ArgRole::OpaqueScript` and **no** grammar: it
/// folds like a body, nothing descends into it, and the definition-body walker
/// drops out of declaration context for it.
#[test]
fn a_predicate_body_is_opaque_and_not_a_declaration_block() {
    let reg = static_context_for("sslictcl").commands();
    let spec = reg.get("predicate").expect("predicate is a statement");
    assert!(
        spec.definition_body.is_none(),
        "a predicate body is not a declaration block"
    );
    assert_eq!(spec.arg_role_at(0), Some(ArgRole::OpaqueScript));
    assert!(
        !ArgRole::OpaqueScript.carries_script(),
        "the loader never evaluates it, so no analysis may claim it does"
    );
    assert!(
        ArgRole::OpaqueScript.folds_as_block(),
        "a reader still collapses the braced word"
    );
    // The one word in the whole pack that is script-shaped.
    let opaque: Vec<&str> = tcl_registry::commands::sslictcl::sslictcl_command_specs()
        .iter()
        .filter(|spec| spec.arg_role_at(0) == Some(ArgRole::OpaqueScript))
        .map(|spec| spec.name)
        .collect();
    assert_eq!(opaque, ["predicate"]);
    assert!(spec.arity.accepts(1) && !spec.arity.accepts(2));
}

/// The enumerated value domains offer exactly their canonical spellings, and
/// only the one exact-match domain is marked closed.
///
/// `closed_value_args` is an exact-match check (W127), while the loader
/// matches every enumerated domain but `schema` case-insensitively — so
/// closing one of those would report `enabled TRUE`, which the loader accepts,
/// as invalid. `rust/tcl-sslictcl/tests/registry_pack_drift.rs` pins the
/// spellings themselves against the loader's own parsers.
#[test]
fn enumerated_value_domains_offer_their_canonical_spellings() {
    let reg = static_context_for("sslictcl").commands();
    let offered: &[(&str, &[&str])] = &[
        (
            "enabled",
            &["true", "false", "yes", "no", "on", "off", "1", "0"],
        ),
        (
            "client",
            &[
                "mozilla",
                "chrome",
                "apple",
                "microsoft",
                "android",
                "openjdk",
            ],
        ),
        (
            "status",
            &["recommended", "acceptable", "deprecated", "prohibited"],
        ),
        ("severity", &["info", "warning", "error", "critical"]),
        ("minimum", &["A+", "A", "B", "C", "D", "E", "F"]),
        (
            "protocol",
            &["ssl2", "ssl3", "tls1.0", "tls1.1", "tls1.2", "tls1.3"],
        ),
        (
            "protocols",
            &["ssl2", "ssl3", "tls1.0", "tls1.1", "tls1.2", "tls1.3"],
        ),
    ];
    for &(word, expected) in offered {
        let spec = reg.get(word).expect("registered statement");
        let values: Vec<&str> = spec.arg_values_at(0).iter().map(|v| v.value).collect();
        assert_eq!(values, expected, "{word}: the offered value set");
        assert!(
            spec.closed_value_args.is_empty(),
            "{word}: the loader's legal set is wider than the canonical \
             spellings, so an exact-match closed set would misreport it"
        );
    }
    // Every BOOL row shares one table, so a new spelling lands everywhere.
    for boolean in [
        "enabled",
        "include-subdomains",
        "preload",
        "trusted",
        "forward-secrecy",
        "aead",
        "require-forward-secrecy",
        "require-hsts",
    ] {
        let spec = reg.get(boolean).expect("registered statement");
        assert_eq!(spec.arg_values_at(0).len(), 8, "{boolean}: eight spellings");
        assert!(spec.closed_value_args.is_empty(), "{boolean}");
    }
    // `schema` is the one exact-match domain, and so the one closed argument.
    let schema = reg.get("schema").expect("registered statement");
    assert_eq!(schema.closed_value_args, &[0]);
    let values: Vec<&str> = schema.arg_values_at(0).iter().map(|v| v.value).collect();
    assert_eq!(values, ["1"]);
    // No other row closes an argument.
    for spec in tcl_registry::commands::sslictcl::sslictcl_command_specs() {
        assert_eq!(
            spec.closed_value_args.is_empty(),
            spec.name != "schema",
            "{}: only `schema` is an exact-match domain",
            spec.name
        );
    }
}

/// `SslicTcl` is a *declaration* family, not a class system: it manufactures
/// nothing and dispatches nothing, so every field a consumer would read to
/// create an instance is empty.
#[test]
fn the_sslictcl_family_manufactures_nothing() {
    assert_eq!(
        SSLICTCL_GRAMMARS.len(),
        VOCABULARY.len(),
        "one grammar per block of the table"
    );
    for grammar in SSLICTCL_GRAMMARS {
        assert_eq!(grammar.family, DefinerFamily::SslicTcl);
        assert!(grammar.manufacturers.is_empty());
        assert!(grammar.builtin_object_methods.is_empty());
        assert!(grammar.builtin_type_methods.is_empty());
        assert!(grammar.builtin_terminating_methods.is_empty());
        assert!(grammar.member_body_commands.is_empty());
        assert!(grammar.implicit_vars.is_empty());
        assert!(!grammar.bare_word_construction);
        assert!(!grammar.dynamic_method_dispatch);
        assert!(grammar.unknown_dispatch_method.is_none());
        assert!(!grammar.members.is_empty(), "a grammar with no members");
    }
}
