// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `SslicTcl` vocabulary as data.
//!
//! [`DECLARATIONS`] is the machine-readable statement of the table in
//! `docs/design/sslictcl-vocabulary.md`: every declaration, its key word, its
//! members and their value domains, and whether unknown members are preserved
//! (`open`) or rejected (`closed`). The loader is tested against this table so
//! the two cannot drift, and an out-of-crate drift gate can compare a registry
//! pack against it without re-deriving the grammar.

/// The kind of value a member (or a declaration's key word) accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDomain {
    /// `true|false|yes|no|on|off|1|0`, case-insensitive.
    Bool,
    /// An unsigned decimal integer.
    Int,
    /// An unsigned decimal integer within an inclusive range.
    IntRange(u64, u64),
    /// One braced Tcl list, split with the shared `tcl_syntax::list` grammar,
    /// or a single bare word.
    List,
    /// One literal word.
    Text,
    /// A [`crate::trust::ClientFamily`] spelling.
    Client,
    /// A [`crate::model::TlsStatus`] spelling.
    Status,
    /// An [`crate::estimate::EstimateSeverity`] spelling.
    Severity,
    /// A declarable [`crate::estimate::Grade`] spelling (`A+` … `F`).
    Grade,
    /// Any [`crate::model::ProtocolVersion`] spelling.
    Version,
    /// 64 hexadecimal digits, case-insensitive.
    Sha256,
    /// An even number of hexadecimal digits.
    Hex,
    /// One braced literal word, retained verbatim and never evaluated.
    Script,
    /// Exactly this literal word.
    Literal(&'static str),
    /// A nested braced block; see [`Member::nested`].
    Block,
}

/// One member of a declaration body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// The member word.
    pub name: &'static str,
    /// The domain of the member's value word.
    pub value: ValueDomain,
    /// The nested declaration, when [`Member::value`] is
    /// [`ValueDomain::Block`].
    pub nested: Option<&'static Declaration>,
}

/// One declaration: a statement word, an optional key word, and a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declaration {
    /// The declaration word.
    pub name: &'static str,
    /// The domain of the key word, when the declaration takes one.
    pub key: Option<ValueDomain>,
    /// Whether the declaration takes a braced body.
    pub body: bool,
    /// Whether unknown members are preserved as extensions (`true`) or
    /// rejected as `SSLIC1007` (`false`).
    pub open: bool,
    /// The declared members, in vocabulary order.
    pub members: &'static [Member],
}

const fn scalar(name: &'static str, value: ValueDomain) -> Member {
    Member {
        name,
        value,
        nested: None,
    }
}

const fn block(name: &'static str, nested: &'static Declaration) -> Member {
    Member {
        name,
        value: ValueDomain::Block,
        nested: Some(nested),
    }
}

/// `hsts { … }` inside an `endpoint`.
pub static HSTS: Declaration = Declaration {
    name: "hsts",
    key: None,
    body: true,
    open: false,
    members: &[
        scalar("enabled", ValueDomain::Bool),
        scalar("max-age", ValueDomain::Int),
        scalar("include-subdomains", ValueDomain::Bool),
        scalar("preload", ValueDomain::Bool),
    ],
};

/// `anchor SHA256 { … }` inside a `trust-program`.
pub static ANCHOR: Declaration = Declaration {
    name: "anchor",
    key: Some(ValueDomain::Sha256),
    body: true,
    open: false,
    members: &[
        scalar("subject", ValueDomain::Text),
        scalar("der-base64", ValueDomain::Text),
        scalar("purposes", ValueDomain::List),
        scalar("trusted", ValueDomain::Bool),
        scalar("distrust-after", ValueDomain::Int),
    ],
};

/// `check ID { … }` inside a `policy`.
///
/// `ID` may not be [`crate::dsl::RESERVED_CHECK_ID`] (`grade`): a policy
/// finding is identified by `(check_id, endpoint)` and the grade rule already
/// emits its finding under that identifier, so the loader reports `SSLIC1009`
/// for a `check grade`.
pub static CHECK: Declaration = Declaration {
    name: "check",
    key: Some(ValueDomain::Text),
    body: true,
    open: false,
    members: &[
        scalar("severity", ValueDomain::Severity),
        scalar("message", ValueDomain::Text),
        scalar("require-protocols", ValueDomain::List),
        scalar("forbid-protocols", ValueDomain::List),
        scalar("forbid-ciphers", ValueDomain::List),
        scalar("require-forward-secrecy", ValueDomain::Bool),
        scalar("min-key-bits", ValueDomain::Int),
        scalar("require-hsts", ValueDomain::Bool),
        scalar("min-hsts-max-age", ValueDomain::Int),
        scalar("predicate", ValueDomain::Script),
    ],
};

/// `grade { … }` inside a `policy`.
pub static GRADE: Declaration = Declaration {
    name: "grade",
    key: None,
    body: true,
    open: false,
    members: &[scalar("minimum", ValueDomain::Grade)],
};

/// Every top-level declaration, in vocabulary order. Nested declarations are
/// reachable through [`Member::nested`].
pub static DECLARATIONS: &[Declaration] = &[
    Declaration {
        name: "sslictcl",
        key: Some(ValueDomain::Int),
        body: false,
        open: false,
        members: &[],
    },
    Declaration {
        name: "certificate",
        key: Some(ValueDomain::Text),
        body: true,
        open: true,
        members: &[
            scalar("pem", ValueDomain::Text),
            scalar("material", ValueDomain::Text),
            scalar("key", ValueDomain::Text),
        ],
    },
    Declaration {
        name: "endpoint",
        key: Some(ValueDomain::Text),
        body: true,
        open: true,
        members: &[
            scalar("hostname", ValueDomain::Text),
            scalar("protocols", ValueDomain::List),
            scalar("ciphers", ValueDomain::List),
            scalar("groups", ValueDomain::List),
            scalar("signature-schemes", ValueDomain::List),
            scalar("certificate-chain", ValueDomain::List),
            scalar("chain", ValueDomain::Text),
            scalar("policy", ValueDomain::Text),
            block("hsts", &HSTS),
        ],
    },
    Declaration {
        name: "testssl-import",
        key: Some(ValueDomain::Text),
        body: true,
        open: false,
        members: &[
            scalar("schema", ValueDomain::Literal("1")),
            scalar("raw-json-hex", ValueDomain::Hex),
        ],
    },
    Declaration {
        name: "trust-program",
        key: Some(ValueDomain::Text),
        body: true,
        open: true,
        members: &[
            scalar("client", ValueDomain::Client),
            scalar("version", ValueDomain::Text),
            scalar("generated-at", ValueDomain::Text),
            scalar("source-name", ValueDomain::Text),
            scalar("source-url", ValueDomain::Text),
            scalar("source-revision", ValueDomain::Text),
            scalar("source-license", ValueDomain::Text),
            block("anchor", &ANCHOR),
        ],
    },
    Declaration {
        name: "protocol",
        key: Some(ValueDomain::Version),
        body: true,
        open: false,
        members: &[
            scalar("status", ValueDomain::Status),
            scalar("score", ValueDomain::IntRange(0, 100)),
            scalar("reference", ValueDomain::Text),
        ],
    },
    Declaration {
        name: "cipher",
        key: Some(ValueDomain::Text),
        body: true,
        open: false,
        members: &[
            scalar("iana-name", ValueDomain::Text),
            scalar("openssl-name", ValueDomain::Text),
            scalar("key-exchange", ValueDomain::Text),
            scalar("authentication", ValueDomain::Text),
            scalar("encryption", ValueDomain::Text),
            scalar("bits", ValueDomain::Int),
            scalar("forward-secrecy", ValueDomain::Bool),
            scalar("aead", ValueDomain::Bool),
            scalar("status", ValueDomain::Status),
            scalar("protocols", ValueDomain::List),
        ],
    },
    Declaration {
        name: "chain",
        key: Some(ValueDomain::Text),
        body: true,
        open: false,
        members: &[scalar("certificates", ValueDomain::List)],
    },
    Declaration {
        name: "policy",
        key: Some(ValueDomain::Text),
        body: true,
        open: false,
        members: &[block("check", &CHECK), block("grade", &GRADE)],
    },
];

/// The top-level declaration with this name, if the vocabulary declares one.
#[must_use]
pub fn declaration(name: &str) -> Option<&'static Declaration> {
    DECLARATIONS.iter().find(|entry| entry.name == name)
}

impl Declaration {
    /// The member with this name, if the declaration declares one.
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&'static Member> {
        self.members.iter().find(|member| member.name == name)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::dsl::{DslDiagnostic, load_with_diagnostics};
    use tcl_core_types::DiagCode;

    /// A value that satisfies `domain`, used to synthesise a fully-declared
    /// instance of every declaration.
    fn sample(domain: ValueDomain) -> String {
        match domain {
            ValueDomain::Bool => "true".to_owned(),
            ValueDomain::Int => "1".to_owned(),
            ValueDomain::IntRange(low, _) => low.to_string(),
            ValueDomain::List | ValueDomain::Script => "{}".to_owned(),
            ValueDomain::Text => "text".to_owned(),
            ValueDomain::Client => "mozilla".to_owned(),
            ValueDomain::Status => "recommended".to_owned(),
            ValueDomain::Severity => "warning".to_owned(),
            ValueDomain::Grade => "A".to_owned(),
            ValueDomain::Version => "tls1.2".to_owned(),
            ValueDomain::Sha256 => "ab".repeat(32),
            ValueDomain::Hex => "7b7d".to_owned(),
            ValueDomain::Literal(word) => word.to_owned(),
            ValueDomain::Block => String::new(),
        }
    }

    /// Render a fully-declared instance of `declaration`: every member present
    /// with a domain-satisfying value, nested blocks included.
    fn instance(declaration: &Declaration, depth: usize) -> String {
        let pad = "    ".repeat(depth);
        let mut out = format!("{pad}{}", declaration.name);
        if let Some(key) = declaration.key {
            out.push(' ');
            out.push_str(&sample(key));
        }
        if !declaration.body {
            out.push('\n');
            return out;
        }
        out.push_str(" {\n");
        let inner = "    ".repeat(depth + 1);
        for member in declaration.members {
            if let Some(nested) = member.nested {
                out.push_str(&instance(nested, depth + 1));
            } else {
                let _ = writeln!(out, "{inner}{} {}", member.name, sample(member.value));
            }
        }
        let _ = writeln!(out, "{pad}}}");
        out
    }

    fn codes(diagnostics: &[DslDiagnostic]) -> Vec<DiagCode> {
        diagnostics.iter().map(|item| item.code).collect()
    }

    /// The parser must know every declaration and member the table declares:
    /// a fully-declared instance may report domain or reference problems, but
    /// never "unknown member" (`SSLIC1007`) or "unknown declaration"
    /// (`SSLIC1101`).
    #[test]
    fn every_declared_word_is_known_to_the_parser() {
        for declaration in DECLARATIONS {
            let source = format!("sslictcl 1\n{}", instance(declaration, 0));
            let reported = codes(&load_with_diagnostics(&source).diagnostics);
            assert!(
                !reported.contains(&DiagCode::Sslic1007)
                    && !reported.contains(&DiagCode::Sslic1101),
                "`{}` has a member the loader does not know: {reported:?}\n{source}",
                declaration.name
            );
        }
    }

    /// The converse: a word the table does not declare must be rejected by a
    /// closed block and preserved by an open one.
    #[test]
    fn undeclared_words_are_rejected_or_preserved_by_openness() {
        let unknown_top = load_with_diagnostics("sslictcl 1\nnot-a-declaration value\n");
        assert!(codes(&unknown_top.diagnostics).contains(&DiagCode::Sslic1101));

        let closed = load_with_diagnostics(
            "sslictcl 1\nchain c {\n    certificates {}\n    not-a-member x\n}\n",
        );
        assert!(codes(&closed.diagnostics).contains(&DiagCode::Sslic1007));

        let open = load_with_diagnostics(
            "sslictcl 1\ncertificate c {\n    pem x\n    not-a-member y\n}\n",
        );
        let reported = codes(&open.diagnostics);
        assert!(reported.contains(&DiagCode::Sslic1101));
        assert!(!reported.contains(&DiagCode::Sslic1007));
    }

    #[test]
    fn table_lookup_helpers_agree_with_the_table() {
        assert_eq!(declaration("endpoint").map(|d| d.name), Some("endpoint"));
        assert!(declaration("nope").is_none());
        let endpoint = declaration("endpoint").unwrap();
        assert!(endpoint.open);
        assert_eq!(endpoint.member("hsts").and_then(|m| m.nested), Some(&HSTS));
        assert!(endpoint.member("nope").is_none());
        assert!(!declaration("chain").unwrap().open);
    }
}
