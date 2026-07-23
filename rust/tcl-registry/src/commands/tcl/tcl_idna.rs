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

//! `tcl::idna` — Internationalised Domain Name (IDNA/Punycode) helpers
//! (Tcl 9.0+, bundled package: `package require tcl::idna 1.0`).
//!
//! Two name forms are registered because Tcl callers may use the
//! namespace-relative spelling `tcl::idna` (inside `namespace eval
//! ::tcl`) or the fully-qualified `::tcl::idna`.
//!
//! Confirmed absent from the Tcl 8.4, 8.5, and 8.6 `TclCmd/idna.html`
//! manpage trees: each of those URLs serves the tcl-lang.org site's
//! soft-404 "URL Not Found" page (HTTP 200, but the same generic
//! not-found body as every other missing page on the site) rather than
//! real manpage content, so this package genuinely did not exist before
//! Tcl 9.0. Tcl 9.0's and 9.1's `idna.html` are byte-for-byte identical
//! once the doc-anchor line-number IDs and the version banner are
//! discounted, so `TCL90_PLUS` is an exact gate with no 9.1-only delta
//! to model separately. No dialect directory (irules/, expect/, tk/,
//! iapps/, itcl/, the eda_* vendors) references `idna` anywhere, so
//! `TCL90_PLUS` alone is the whole restriction — every one of those
//! dialects' base runtimes predates Tcl 9.0 regardless.
//!
//! Distinct from `tcl::idna::encode` / `tcl::idna::decode`
//! (`commands/stdlib/tcl__idna__encode.rs` /
//! `commands/stdlib/tcl__idna__decode.rs`) — those are the older,
//! unrelated single-word helper commands bundled with the `cookiejar`
//! package, not subcommands of this ensemble.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tcl::idna subcommand ?arg ...?",
    dialects: None,
}];

/// Second-level dispatch of `tcl::idna puny`. Real Tcl documents exactly
/// two operations here ("It supports two `subcommand`s:" — idna.n, Tcl
/// 9.0/9.1) rather than an open-ended set, so this is a closed pair
/// rather than a generic placeholder.
const IDNA_PUNY_SUBS: &[SubSubCommand] = &[
    SubSubCommand {
        name: "decode",
        detail: "Decode a punycode-encoded string, optionally case-folding the result.",
        synopsis: "tcl::idna puny decode string ?case?",
        dialects: None,
    },
    SubSubCommand {
        name: "encode",
        detail: "Encode a string as punycode, optionally case-folding the result.",
        synopsis: "tcl::idna puny encode string ?case?",
        dialects: None,
    },
];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90_PLUS),
        // No `Traits` bits apply: package-gated rather than a core
        // builtin the compiler special-cases (no `BYTE_COMPILED`); every
        // subcommand is a pure string transform with no filesystem,
        // network, or process access (no `OPENS_CHANNEL` / `TAINT_SINK`
        // / `TAINT_SOURCE`); not implemented as a dedicated tcl-vm
        // opcode, so a call falls through the generic invoke path (no
        // `FRAMELESS_RUNTIME`); and it is not on C Tcl's fixed
        // `unsafeEnsembleCommands` table (that list is core commands
        // only), so there is no positive evidence for
        // `SAFE_INTERP_HIDDEN` either — left unset rather than guessed.
        traits: Traits::empty(),
        required_package: Some("tcl::idna"),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Internationalised Domain Name (IDNA/Punycode) helpers.",
            synopsis: &[
                "tcl::idna decode hostname",
                "tcl::idna encode hostname",
                "tcl::idna puny decode string ?case?",
                "tcl::idna puny encode string ?case?",
                "tcl::idna version",
            ],
            snippet: "Implements the punycode encoding scheme (RFC 3492) used by Internationalised Domain Names, plus a couple of convenience wrappers. decode takes the name of a host that potentially contains punycode-encoded character sequences and returns the hostname as it should be displayed to a user; because many Unicode code points have near-identical glyphs, a decoded hostname should still be shown to users with care. encode takes a hostname as a user would see it and returns that hostname with any characters not permitted in a basic hostname replaced by their punycode encoding. puny gives direct access to the underlying encoder (puny encode) and decoder (puny decode) for a single string, rather than a whole dotted hostname; each optionally takes case, a boolean that folds the result to upper case (true) or lower case (false) — when case is omitted, no case folding is applied. version returns the tcl::idna package's own version string. Available only after `package require tcl::idna 1.0`, and only on Tcl 9.0 or later — the package does not exist in Tcl 8.4, 8.5, or 8.6.",
            source: "Tcl man page idna.n",
            examples: "package require tcl::idna 1.0\n\n# Low-level punycode round trip on a single label (RFC 3492 example)\nset enc [tcl::idna puny encode \"abc\\u2192def\"]\nputs $enc                          ;# abcdef-kn2c\nputs [tcl::idna puny decode $enc]  ;# abc\\u2192def\n\n# Hostname-aware helpers\nset wireForm    [tcl::idna encode $displayHostname]\nset displayForm [tcl::idna decode $wireForm]\nputs [tcl::idna version]",
            return_value: "Always a string: the display-form or punycode-encoded hostname (decode/encode), the transformed punycode text (puny decode/puny encode), or the tcl::idna package's version number (version).",
        }),
        ..CommandSpec::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "decode",
        arity: Arity::new(1, 1),
        detail: "Decode punycode-encoded sequences in hostname to the form displayed to a user. Similar-looking Unicode glyphs mean the result should still be shown to users with care.",
        synopsis: "tcl::idna decode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::new(1, 1),
        detail: "Encode hostname, replacing any characters not permitted in a basic hostname with their punycode form.",
        synopsis: "tcl::idna encode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puny",
        arity: Arity::new(2, 3),
        detail: "Direct access to the punycode encoder/decoder for a single string (decode|encode), bypassing the hostname-aware decode/encode above.",
        synopsis: "tcl::idna puny decode|encode string ?case?",
        return_type: Some(TclType::String),
        pure: true,
        // `puny` is itself a two-operation ensemble: the word after
        // `puny` selects `decode` or `encode` (idna.n: "It supports two
        // subcommands").
        sub_subcommands: IDNA_PUNY_SUBS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::new(0, 0),
        detail: "Return the tcl::idna package's version number.",
        synopsis: "tcl::idna version",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::idna`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::idna")
}

/// Command spec for the fully-qualified form `::tcl::idna`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::idna")
}
