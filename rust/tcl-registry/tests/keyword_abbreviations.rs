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

//! Keyword-abbreviation resolution over the **real** registry tables (#1231).
//!
//! The unit tests in `tcl_registry::abbrev` pin the algorithm against
//! hand-written tables; these pin it against what the registry actually
//! ships, so a registry data change that breaks an abbreviation shows up
//! here.

use tcl_registry::abbrev::{KeywordMatch, PrefixMatching, resolve_over_versions};
use tcl_registry::model::ingress::static_context_for;

/// `string le` … `string length` all resolve; `string l` is ambiguous with
/// `string last`.
///
/// tclsh-proof (8.6.16):
/// `string le abc` → `3`;
/// `string l abc` → `unknown or ambiguous subcommand "l": must be bytelength,
/// cat, compare, equal, first, index, is, last, length, …`.
#[test]
fn string_length_abbreviations_resolve_and_l_is_ambiguous() {
    let reg = static_context_for("tcl8.6").commands();
    let spec = reg.get("string").expect("string command");
    let dialect = tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query);

    for word in ["le", "len", "leng", "lengt", "length"] {
        assert_eq!(
            spec.resolve_subcommand_word(word, dialect, None, None),
            KeywordMatch::Unique("length"),
            "string {word}"
        );
        // The resolved subcommand must carry the *canonical* keyword's facts.
        let sub = spec.resolve_subcommand(word).expect("resolves");
        assert_eq!(sub.name, "length", "string {word}");
    }

    let ambiguous = spec.resolve_subcommand_word("l", dialect, None, None);
    assert!(ambiguous.is_ambiguous(), "string l: {ambiguous:?}");
    let candidates = ambiguous.candidates();
    assert!(candidates.contains(&"last"), "{candidates:?}");
    assert!(candidates.contains(&"length"), "{candidates:?}");
    // An ambiguous word never resolves to a SubCommand.
    assert!(spec.resolve_subcommand("l").is_none());
}

/// An exact spelling that is also a proper prefix of a longer keyword still
/// wins. tclsh-proof (8.6.16): `string trim " x "` → `x` (not ambiguous with
/// `trimleft`/`trimright`).
#[test]
fn an_exact_subcommand_wins_over_the_prefix_it_shares() {
    let reg = static_context_for("tcl8.6").commands();
    let spec = reg.get("string").expect("string command");
    let dialect = tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query);
    assert_eq!(
        spec.resolve_subcommand_word("trim", dialect, None, None),
        KeywordMatch::Unique("trim")
    );
    assert!(
        spec.resolve_subcommand_word("tri", dialect, None, None)
            .is_ambiguous()
    );
}

/// A keyword added in a later release shrinks what an older prefix can mean,
/// so a range spanning both must report `Ambiguous`.
///
/// tclsh-proof (8.6.16): `string c abc` → `unknown or ambiguous subcommand
/// "c"` — `cat` (8.6.2+) collides with `compare`.
#[test]
fn a_prefix_unique_in_one_version_is_ambiguous_for_a_range_that_adds_a_keyword() {
    let reg85 = static_context_for("tcl8.5").commands();
    let reg86 = static_context_for("tcl8.6").commands();
    let t85 = reg85
        .get("string")
        .expect("string in 8.5")
        .subcommand_table(tcl_dialect::DialectProfile::find("tcl8.5").map(tcl_dialect::DialectProfile::surface_query), None, None);
    let t86 = reg86
        .get("string")
        .expect("string in 8.6")
        .subcommand_table(tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query), None, None);

    // `cat` exists only from 8.6, so it is not resolvable for an 8.5–8.6 range.
    assert_eq!(t86.resolve("cat"), KeywordMatch::Unique("cat"));
    assert_eq!(t85.resolve("cat"), KeywordMatch::Unknown);
    assert_eq!(
        resolve_over_versions(&[t85.clone(), t86.clone()], "cat"),
        KeywordMatch::Unknown
    );

    // `c` is ambiguous once `cat` exists, so it must not be unique for a
    // range that includes 8.6 — whatever it meant in 8.5.
    assert!(t86.resolve("c").is_ambiguous());
    let ranged = resolve_over_versions(&[t85, t86], "c");
    assert!(ranged.is_ambiguous(), "{ranged:?}");

    // A keyword present in both versions still resolves for the range.
    let t85 = reg85
        .get("string")
        .expect("string in 8.5")
        .subcommand_table(tcl_dialect::DialectProfile::find("tcl8.5").map(tcl_dialect::DialectProfile::surface_query), None, None);
    let t86 = reg86
        .get("string")
        .expect("string in 8.6")
        .subcommand_table(tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query), None, None);
    assert_eq!(
        resolve_over_versions(&[t85, t86], "le"),
        KeywordMatch::Unique("length")
    );
}

/// Option tables prefix-match exactly like subcommand tables.
///
/// tclsh-proof (8.6.16): `lsearch -noc {a b} a` → `0`;
/// `lsearch -a {a b} a` → `ambiguous option "-a": must be -all, -ascii, …`.
#[test]
fn option_abbreviations_resolve_over_the_real_option_table() {
    let reg = static_context_for("tcl8.6").commands();
    let spec = reg.get("lsearch").expect("lsearch command");
    let dialect = tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query);

    assert_eq!(
        spec.resolve_option_word("-noc", dialect, None, None),
        KeywordMatch::Unique("-nocase")
    );
    let ambiguous = spec.resolve_option_word("-a", dialect, None, None);
    assert!(ambiguous.is_ambiguous(), "lsearch -a: {ambiguous:?}");
    assert!(ambiguous.candidates().contains(&"-all"));
    assert!(ambiguous.candidates().contains(&"-ascii"));

    let table = spec.option_table(dialect, None, None);
    assert_eq!(table.minimal_unique_prefix("-nocase"), Some("-noc"));
}

/// A strict table (a `TCL_INDEX_STRICT` command, or an ensemble the analyser
/// saw configured with `-prefixes 0`) resolves only exact spellings, and an
/// abbreviation there is *unknown*, never ambiguous.
#[test]
fn a_strict_override_disables_prefix_matching_at_the_call_site() {
    let reg = static_context_for("tcl8.6").commands();
    let spec = reg.get("string").expect("string command");
    let dialect = tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query);
    let strict = Some(PrefixMatching::Strict);

    assert_eq!(
        spec.resolve_subcommand_word("length", dialect, None, strict),
        KeywordMatch::Unique("length")
    );
    assert_eq!(
        spec.resolve_subcommand_word("le", dialect, None, strict),
        KeywordMatch::Unknown
    );
    assert_eq!(
        spec.resolve_subcommand_word("l", dialect, None, strict),
        KeywordMatch::Unknown
    );
    assert_eq!(
        spec.subcommand_table(dialect, None, strict)
            .minimal_unique_prefix("length"),
        None
    );
}

/// Every subcommand of every ensemble in every dialect has a minimal prefix
/// that round-trips to it, and no keyword's minimal prefix resolves to a
/// different keyword.
///
/// registry-metadata: a structural invariant of the derived prefix map, not a
/// C-Tcl fact.
#[test]
fn every_minimal_prefix_round_trips_to_its_keyword() {
    for dname in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "f5-irules"] {
        let reg = static_context_for(dname).commands();
        let dialect = tcl_dialect::DialectProfile::find(dname).map(tcl_dialect::DialectProfile::surface_query);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            let table = spec.subcommand_table(dialect, None, None);
            for keyword in table.names() {
                let short = table
                    .minimal_unique_prefix(keyword)
                    .unwrap_or_else(|| panic!("{dname}/{name} {keyword}: no minimal prefix"));
                assert!(
                    keyword.starts_with(short),
                    "{dname}/{name} {keyword}: {short} is not a prefix"
                );
                assert_eq!(
                    table.resolve(short),
                    KeywordMatch::Unique(keyword),
                    "{dname}/{name} {keyword}"
                );
            }
        }
    }
}
