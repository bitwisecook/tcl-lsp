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

//! `SpecTcl` command packs and `SslicTcl` declarations: context-sensitive keywords and their bodies.

use super::*;

// SpecTcl — a `.tclspec` command pack. Every statement word below is
// registry data (the `spectcl` pack plus the `SPECTCL_*` definition-body
// grammars in `tcl_registry::definer`); this walk contains no SpecTcl code
// at all, which is the whole point of `spec-packs.md`'s "authoring a pack
// gets highlighting … from the same machinery it configures".

/// The nesting chain, painted: a `speclib` body's `command` word is a member
/// keyword, that command's body's `arity` / `arg` words are keywords too, and
/// a `hover` block's `summary` is a keyword one level deeper still — each
/// resolved from a *different* grammar.
#[test]
fn spectcl_pack_body_words_are_keywords_at_every_level() {
    let src = "speclib mylib 1.0 {\n\
               \x20   command with_var {\n\
               \x20       arity 2\n\
               \x20       arg 0 -role VarWrite\n\
               \x20       hover {\n\
               \x20           summary {Run a script with a caller variable bound.}\n\
               \x20       }\n\
               \x20   }\n\
               }\n";
    for word in ["command", "arity", "arg", "hover", "summary"] {
        assert_eq!(
            kind_of_word(src, "spectcl", word).as_deref(),
            Some("keyword"),
            "`{word}` must be a SpecTcl statement keyword: {:?}",
            decode(src, "spectcl"),
        );
    }
    // The declared command's own name is a member name, not a bare string.
    assert_ne!(
        kind_of_word(src, "spectcl", "with_var").as_deref(),
        Some("string"),
        "a declared command name must not paint as a plain string: {:?}",
        decode(src, "spectcl"),
    );
}

/// The same words outside a pack are *not* keywords — the vocabulary is
/// context-sensitive, exactly as `method` is inside a snit body and nowhere
/// else. `arity` here is an ordinary (unknown) command word.
#[test]
fn spectcl_statement_words_are_context_sensitive() {
    let outside = "arity 2\nsummary {hello}\n";
    for word in ["arity", "summary"] {
        assert_ne!(
            kind_of_word(outside, "tcl8.6", word).as_deref(),
            Some("keyword"),
            "`{word}` must mean nothing outside a pack: {:?}",
            decode(outside, "tcl8.6"),
        );
    }
}

/// A hook body is ordinary Tcl: the statement carries no grammar, so the walk
/// drops out of declaration context and recurses the script — `if` and `set`
/// resolve as the real commands they are, and the parameter list declares
/// parameters.
#[test]
fn spectcl_hook_bodies_recurse_as_tcl() {
    let src = "speclib mylib 1.0 {\n\
               \x20   command mylib::len {\n\
               \x20       const_fold {words ctx} {\n\
               \x20           if {![string is ascii [lindex $words 0]]} return\n\
               \x20           fold [string length [lindex $words 0]]\n\
               \x20       }\n\
               \x20   }\n\
               }\n";
    assert_eq!(
        kind_of_word(src, "spectcl", "const_fold").as_deref(),
        Some("keyword"),
        "the hook statement itself is a keyword: {:?}",
        decode(src, "spectcl"),
    );
    assert_eq!(
        kind_of_word(src, "spectcl", "if").as_deref(),
        Some("keyword"),
        "a hook body is real Tcl and must be recursed: {:?}",
        decode(src, "spectcl"),
    );
    assert_eq!(
        kind_of_word(src, "spectcl", "words").as_deref(),
        Some("parameter"),
        "the hook signature declares parameters: {:?}",
        decode(src, "spectcl"),
    );
}

// SslicTcl — a `.sslictcl` TLS declaration. Every declaration word below is
// registry data (the `sslictcl` pack plus the `SSLICTCL_*` definition-body
// grammars in `tcl_registry::definer`); this walk contains no SslicTcl code
// at all.

/// The nesting chain, painted: an `endpoint` body's `hostname` is a member
/// keyword, its nested `hsts` block's `max-age` is one level deeper, and a
/// `policy` body's `check` opens a third vocabulary — each resolved from a
/// *different* grammar.
#[test]
fn sslictcl_body_words_are_keywords_at_every_level() {
    let src = "sslictcl 1\n\
               endpoint www {\n\
               \x20   hostname www.example.com\n\
               \x20   protocols {tls1.2 tls1.3}\n\
               \x20   hsts {\n\
               \x20       enabled true\n\
               \x20       max-age 31536000\n\
               \x20   }\n\
               }\n\
               policy baseline {\n\
               \x20   check no-legacy-tls {\n\
               \x20       severity error\n\
               \x20       forbid-protocols {tls1.0 tls1.1}\n\
               \x20   }\n\
               \x20   grade {\n\
               \x20       minimum A\n\
               \x20   }\n\
               }\n";
    for word in [
        "sslictcl",
        "endpoint",
        "hostname",
        "protocols",
        "hsts",
        "enabled",
        "max-age",
        "policy",
        "check",
        "severity",
        "forbid-protocols",
        "grade",
        "minimum",
    ] {
        assert_eq!(
            kind_of_word(src, "sslictcl", word).as_deref(),
            Some("keyword"),
            "`{word}` must be an SslicTcl declaration keyword: {:?}",
            decode(src, "sslictcl"),
        );
    }
    // A declared member's own name is a member name, not a bare string.
    assert_ne!(
        kind_of_word(src, "sslictcl", "no-legacy-tls").as_deref(),
        Some("string"),
        "a declared check id must not paint as a plain string: {:?}",
        decode(src, "sslictcl"),
    );
}

/// The same words outside a `.sslictcl` document are *not* keywords — the
/// vocabulary is context-sensitive, exactly as `method` is inside a snit body
/// and nowhere else.
#[test]
fn sslictcl_declaration_words_are_context_sensitive() {
    let outside = "hostname www.example.com\nseverity error\ngrade {minimum A}\n";
    for word in ["hostname", "severity", "grade"] {
        assert_ne!(
            kind_of_word(outside, "tcl9.0", word).as_deref(),
            Some("keyword"),
            "`{word}` must mean nothing outside a document: {:?}",
            decode(outside, "tcl9.0"),
        );
    }
    // …and inside the dialect the *grammar* is what places them: `hostname`
    // is a member of `endpoint` and of nothing else, so an `hsts` body does
    // not accept it as one of its own rows.
    assert!(
        !tcl_registry::definer::SSLICTCL_HSTS_GRAMMAR.is_member("hostname"),
        "`hostname` belongs to `endpoint`, not to `hsts`"
    );
    assert!(tcl_registry::definer::SSLICTCL_ENDPOINT_GRAMMAR.is_member("hostname"));
}

/// A **misplaced** member row is not painted as a keyword.
///
/// This is what the grammar buys and what a global `LANGUAGE_KEYWORD` trait
/// would have thrown away: the token walker falls back to the trait whenever
/// the enclosing grammar does not name the head, so a spec carrying it looks
/// correct everywhere. No `SslicTcl` spec carries it — every word is reachable
/// as a grammar member, the nine top-level declarations through the document
/// grammar.
#[test]
fn a_misplaced_sslictcl_member_is_not_painted_as_a_keyword() {
    // `hostname` belongs to `endpoint`. Inside `hsts` and at the root it is
    // just an unknown word.
    let inside_hsts = "sslictcl 1\n\
                       endpoint www {\n\
                       \x20   hsts {\n\
                       \x20       hostname www.example.com\n\
                       \x20   }\n\
                       }\n";
    assert_ne!(
        kind_of_word(inside_hsts, "sslictcl", "hostname").as_deref(),
        Some("keyword"),
        "`hostname` is not an `hsts` row: {:?}",
        decode(inside_hsts, "sslictcl"),
    );
    let at_root = "sslictcl 1\nhostname www.example.com\n";
    assert_ne!(
        kind_of_word(at_root, "sslictcl", "hostname").as_deref(),
        Some("keyword"),
        "`hostname` is not a top-level declaration: {:?}",
        decode(at_root, "sslictcl"),
    );
    // …while in its own block it still is.
    let in_place = "sslictcl 1\nendpoint www {\n    hostname www.example.com\n}\n";
    assert_eq!(
        kind_of_word(in_place, "sslictcl", "hostname").as_deref(),
        Some("keyword"),
        "`hostname` is an `endpoint` row: {:?}",
        decode(in_place, "sslictcl"),
    );
    // The document grammar is what paints the root declarations, so they are
    // still keywords with no spec carrying the global trait.
    for word in ["sslictcl", "endpoint"] {
        assert_eq!(
            kind_of_word(in_place, "sslictcl", word).as_deref(),
            Some("keyword"),
            "`{word}` is a top-level declaration: {:?}",
            decode(in_place, "sslictcl"),
        );
        assert!(
            !static_context_for("sslictcl")
                .commands()
                .get(word)
                .expect("a registered declaration")
                .traits
                .contains(tcl_registry::Traits::LANGUAGE_KEYWORD),
            "`{word}` is painted by the document grammar, not by a global trait"
        );
    }
}

/// A `predicate` body is neither a declaration block nor executable Tcl.
///
/// Its statement carries no grammar, so the walk drops out of declaration
/// context; and its word is `ArgRole::OpaqueScript` rather than `Body`, so
/// nothing descends into it at all — the words inside are data, not commands
/// and not member rows.
#[test]
fn sslictcl_predicate_bodies_are_opaque_data() {
    let src = "policy baseline {\n\
               \x20   check bespoke {\n\
               \x20       severity warning\n\
               \x20       predicate {\n\
               \x20           severity nonsense\n\
               \x20       }\n\
               \x20   }\n\
               }\n";
    assert_eq!(
        kind_of_word(src, "sslictcl", "predicate").as_deref(),
        Some("keyword"),
        "the predicate statement itself is a keyword: {:?}",
        decode(src, "sslictcl"),
    );
    // Nothing inside is painted as a declaration row or as a command: the
    // `severity` on line 4 is a real member row, the one on line 5 is inside
    // the retained script and must not be.
    let inside = decode(src, "sslictcl")
        .into_iter()
        .find(|t| t.line == 4 && t.character == 12);
    assert!(
        inside.as_ref().is_none_or(|t| t.ttype != "keyword"),
        "a word inside a retained predicate must not paint as a keyword: {inside:?}",
    );
    let spec = static_context_for("sslictcl")
        .commands()
        .get("predicate")
        .expect("predicate is a statement");
    assert!(
        spec.definition_body.is_none(),
        "a predicate body carries no member grammar, so nothing inside it is \
         painted as a declaration row"
    );
    assert_eq!(
        spec.arg_role_at(0),
        Some(tcl_registry::ArgRole::OpaqueScript),
        "the braced word is retained data, not an analysed body"
    );
}
