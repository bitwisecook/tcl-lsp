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

//! Keyword abbreviation: one resolver for every prefix-spelled word.
//!
//! Tcl's `Tcl_GetIndexFromObj` dispatch means an enum-ish word — an ensemble
//! subcommand, an `-option`, a boolean value — is legal under **any unique
//! prefix** of its table entry. Real code uses those spellings, so the
//! registry models them here rather than leaving each consumer to invent its
//! own matcher.
//!
//! The minimal unique abbreviation is a pure function of the table, so
//! nothing is hand-authored: build a [`KeywordTable`] from the canonical
//! keywords and ask [`KeywordTable::resolve`]. Two authoring facts *are*
//! declared, because they cannot be derived:
//!
//! * a per-table **strictness** flag ([`PrefixMatching::Strict`]) for tables
//!   whose C side opts out of prefix matching (`TCL_INDEX_STRICT`), and for
//!   script-level ensembles the analyser saw configured with
//!   `namespace ensemble … -prefixes 0`;
//! * an optional per-keyword **`min_abbrev`** for the rare command whose
//!   documentation promises a longer minimum than uniqueness requires.
//!
//! Resolution is three-valued — [`KeywordMatch::Unique`],
//! [`KeywordMatch::Ambiguous`], [`KeywordMatch::Unknown`] — because the three
//! outcomes need different treatment: `Unique` feeds the canonical keyword's
//! facts downstream unchanged, `Ambiguous` is a guaranteed runtime error
//! worth its own diagnostic, and `Unknown` is the pre-existing
//! unknown-keyword path.
//!
//! Version ranges matter twice. The table contents change between releases
//! (`string cat` arrived in 8.6.2 and shortened what `c…` could mean), and a
//! word is only safely `Unique` for a *range* when it resolves to the same
//! keyword in **every** version of that range —
//! [`resolve_over_versions`] enforces that.
//!
//! Command **names** are never prefix-matched: `str length` is a genuine
//! unknown command, not `string length`. This module is only for keyword
//! tables.
//!
//! tclsh-proof (8.6.16): `string le abc` → `3`; `string l abc` → `unknown or
//! ambiguous subcommand "l"`; `lsearch -noc {a b} a` → `0`; `lsearch -a …` →
//! `ambiguous option "-a"`; `expr {t ? 1 : 0}` → `1`; `expr {o ? 1 : 0}` →
//! `expected boolean value but got "o"`.

/// Whether a keyword table's dispatch honours unique-prefix abbreviation.
///
/// The default is [`Self::Enabled`]: C Tcl reaches every ensemble and option
/// table through `Tcl_GetIndexFromObj`, which prefix-matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PrefixMatching {
    /// Any unique prefix of a keyword resolves to it (the `Tcl_GetIndexFromObj`
    /// default).
    #[default]
    Enabled,
    /// Only the exact spelling resolves — a `TCL_INDEX_STRICT` table, or a
    /// script-level ensemble configured with `-prefixes 0`.
    Strict,
}

impl PrefixMatching {
    /// Whether abbreviated spellings resolve in this table.
    #[must_use]
    pub const fn accepts_prefixes(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

/// One entry of a keyword table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyword<'a> {
    /// The canonical spelling.
    pub name: &'a str,
    /// Documented minimum abbreviation length, when the command promises a
    /// longer minimum than uniqueness alone requires. `None` = uniqueness is
    /// the only constraint.
    pub min_abbrev: Option<u8>,
}

impl<'a> Keyword<'a> {
    /// A keyword whose only constraint is uniqueness.
    #[must_use]
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            min_abbrev: None,
        }
    }
}

/// The outcome of resolving a possibly-abbreviated word against a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordMatch<'a> {
    /// The word resolves to exactly one keyword — its canonical spelling.
    ///
    /// Everything downstream (arity, arg roles, side-effect traits,
    /// safe-interp hiding, version gates) must treat this exactly as if the
    /// canonical word had been written.
    Unique(&'a str),
    /// The word prefixes more than one keyword. Sorted candidate spellings;
    /// always at least two. A guaranteed runtime error in real Tcl.
    Ambiguous(Vec<&'a str>),
    /// The word prefixes nothing in the table.
    Unknown,
}

impl<'a> KeywordMatch<'a> {
    /// The canonical keyword when resolution was unambiguous.
    #[must_use]
    pub fn unique(&self) -> Option<&'a str> {
        match self {
            Self::Unique(name) => Some(name),
            _ => None,
        }
    }

    /// The candidate set when the word was ambiguous.
    #[must_use]
    pub fn candidates(&self) -> &[&'a str] {
        match self {
            Self::Ambiguous(names) => names,
            _ => &[],
        }
    }

    /// Whether the word prefixes more than one keyword.
    #[must_use]
    pub const fn is_ambiguous(&self) -> bool {
        matches!(self, Self::Ambiguous(_))
    }
}

/// A set of canonical keywords plus how its dispatch treats prefixes.
///
/// Built per lookup from whatever the caller has — a `SubCommand` slice, an
/// `OptionSpec` slice, the built-in boolean table — after the caller has
/// already applied its dialect and lifecycle filters, so the table describes
/// exactly the keywords that exist at the target release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordTable<'a> {
    keywords: Vec<Keyword<'a>>,
    prefix_matching: PrefixMatching,
}

impl<'a> KeywordTable<'a> {
    /// A table from canonical spellings, with uniqueness the only constraint.
    #[must_use]
    pub fn new(names: impl IntoIterator<Item = &'a str>, prefix_matching: PrefixMatching) -> Self {
        Self {
            keywords: names.into_iter().map(Keyword::new).collect(),
            prefix_matching,
        }
    }

    /// A table from entries that may carry `min_abbrev` overrides.
    #[must_use]
    pub fn from_keywords(
        keywords: impl IntoIterator<Item = Keyword<'a>>,
        prefix_matching: PrefixMatching,
    ) -> Self {
        Self {
            keywords: keywords.into_iter().collect(),
            prefix_matching,
        }
    }

    /// How this table's dispatch treats prefixes.
    #[must_use]
    pub const fn prefix_matching(&self) -> PrefixMatching {
        self.prefix_matching
    }

    /// The canonical spellings in this table, in declaration order.
    pub fn names(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.keywords.iter().map(|k| k.name)
    }

    /// Whether the table holds no keywords.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }

    /// Resolve `word` against this table.
    ///
    /// An exact spelling always wins, even when it is also a proper prefix of
    /// a longer keyword (`string last` is `last`, never ambiguous with
    /// nothing; `info c` would be ambiguous but `info class` is exact). A
    /// strict table resolves nothing but exact spellings, so an abbreviation
    /// there is [`KeywordMatch::Unknown`], not `Ambiguous` — the user's error
    /// is an unknown keyword, and the existing unknown-keyword path already
    /// says so.
    #[must_use]
    pub fn resolve(&self, word: &str) -> KeywordMatch<'a> {
        if word.is_empty() {
            return KeywordMatch::Unknown;
        }
        if let Some(exact) = self.keywords.iter().find(|k| k.name == word) {
            return KeywordMatch::Unique(exact.name);
        }
        if !self.prefix_matching.accepts_prefixes() {
            return KeywordMatch::Unknown;
        }
        let mut hits: Vec<&'a str> = self
            .keywords
            .iter()
            .filter(|k| {
                k.name.starts_with(word)
                    && k.min_abbrev
                        .is_none_or(|min| word.len() >= usize::from(min))
            })
            .map(|k| k.name)
            .collect();
        match hits.len() {
            0 => KeywordMatch::Unknown,
            1 => KeywordMatch::Unique(hits[0]),
            _ => {
                hits.sort_unstable();
                hits.dedup();
                if hits.len() == 1 {
                    KeywordMatch::Unique(hits[0])
                } else {
                    KeywordMatch::Ambiguous(hits)
                }
            }
        }
    }

    /// The shortest prefix of `keyword` that this table resolves back to
    /// `keyword`, or `None` when `keyword` is not in the table or the table
    /// is strict (no abbreviation is legal, so the "shortest" spelling is the
    /// full word — reported as `None` so callers cannot accidentally shorten
    /// it).
    ///
    /// This is what an emitter (the minifier, #1230) asks for. It respects
    /// `min_abbrev` and never returns a prefix that another keyword also
    /// matches.
    #[must_use]
    pub fn minimal_unique_prefix(&self, keyword: &str) -> Option<&'a str> {
        if !self.prefix_matching.accepts_prefixes() {
            return None;
        }
        let entry = self.keywords.iter().find(|k| k.name == keyword)?;
        let floor = entry.min_abbrev.map_or(1, usize::from).max(1);
        // Character boundaries only: a keyword is ASCII in practice, but a
        // byte slice of a multi-byte name would not be a legal prefix.
        for (end, _) in entry
            .name
            .char_indices()
            .skip(1)
            .chain(std::iter::once((entry.name.len(), '\0')))
        {
            if end < floor {
                continue;
            }
            let candidate = &entry.name[..end];
            if self.resolve(candidate) == KeywordMatch::Unique(entry.name) {
                return Some(&entry.name[..end]);
            }
        }
        None
    }
}

/// Resolve `word` against every version's table in a target range.
///
/// The word is [`KeywordMatch::Unique`] only when it resolves to the *same*
/// keyword in every version — a prefix that is unique in 8.5 but ambiguous
/// once a later release adds a keyword must not be treated as unique for a
/// range spanning both. When the versions disagree the union of every
/// version's candidate set is reported as [`KeywordMatch::Ambiguous`], so the
/// diagnostic can name what it collides with.
///
/// An empty range is `Unknown` — there is no table to resolve against.
#[must_use]
pub fn resolve_over_versions<'a>(tables: &[KeywordTable<'a>], word: &str) -> KeywordMatch<'a> {
    if tables.is_empty() {
        return KeywordMatch::Unknown;
    }
    let mut candidates: Vec<&'a str> = Vec::new();
    let mut any_ambiguous = false;
    let mut any_unknown = false;
    let mut resolved: Option<&'a str> = None;
    let mut disagreed = false;
    for table in tables {
        match table.resolve(word) {
            KeywordMatch::Unique(name) => {
                push_unique(&mut candidates, name);
                match resolved {
                    None => resolved = Some(name),
                    Some(prev) if prev == name => {}
                    Some(_) => disagreed = true,
                }
            }
            KeywordMatch::Ambiguous(names) => {
                any_ambiguous = true;
                for name in names {
                    push_unique(&mut candidates, name);
                }
            }
            KeywordMatch::Unknown => any_unknown = true,
        }
    }
    // Ambiguity in *any* version is the interesting outcome: the word is a
    // guaranteed runtime error somewhere in the range, and the union of every
    // version's candidates is what the user needs to disambiguate against.
    if any_ambiguous || disagreed {
        candidates.sort_unstable();
        return KeywordMatch::Ambiguous(candidates);
    }
    // A keyword that does not exist in *every* version of the range cannot be
    // resolved for the range.
    if any_unknown {
        return KeywordMatch::Unknown;
    }
    resolved.map_or(KeywordMatch::Unknown, KeywordMatch::Unique)
}

fn push_unique<'a>(candidates: &mut Vec<&'a str>, name: &'a str) {
    if !candidates.contains(&name) {
        candidates.push(name);
    }
}

/// The canonical boolean spellings Tcl's `Tcl_GetBoolean` accepts, longest
/// forms first for readability. `0` and `1` are in the table but can only be
/// written exactly (they have no proper prefixes).
pub const BOOLEAN_KEYWORDS: &[&str] = &["true", "false", "yes", "no", "on", "off", "0", "1"];

/// The built-in boolean keyword table.
///
/// Prefix matching over `true`/`false`/`yes`/`no`/`on`/`off` reproduces
/// `Tcl_GetBoolean` exactly, including `o` being ambiguous between `on` and
/// `off` — the one boolean prefix Tcl rejects.
///
/// tclsh-proof (8.6.16): `expr {t ? 1 : 0}` → `1`; `expr {of ? 1 : 0}` → `0`;
/// `expr {o ? 1 : 0}` → `expected boolean value but got "o"`;
/// `string is boolean tru` → `1`, `string is boolean o` → `0`.
#[must_use]
pub fn boolean_table() -> KeywordTable<'static> {
    KeywordTable::new(BOOLEAN_KEYWORDS.iter().copied(), PrefixMatching::Enabled)
}

/// The boolean value a resolved boolean keyword denotes, or `None` when the
/// word is not a boolean at all.
///
/// Accepts the same spellings as [`boolean_table`], case-insensitively (Tcl
/// lower-cases the word before dispatch).
#[must_use]
pub fn resolve_boolean(word: &str) -> Option<bool> {
    let lowered = word.to_ascii_lowercase();
    match boolean_table().resolve(&lowered) {
        KeywordMatch::Unique(name) => Some(matches!(name, "true" | "yes" | "on" | "1")),
        _ => None,
    }
}

/// Whether `word` is a boolean word at all — an exact or prefix spelling, or
/// an ambiguous one (`o`). Used by consumers that must decide whether a word
/// is *in the boolean family* before deciding what to do with it.
#[must_use]
pub fn is_boolean_word(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();
    !matches!(boolean_table().resolve(&lowered), KeywordMatch::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Tcl 8.6 `string` ensemble table, in the order tclsh reports it.
    const STRING_86: &[&str] = &[
        "bytelength",
        "cat",
        "compare",
        "equal",
        "first",
        "index",
        "is",
        "last",
        "length",
        "map",
        "match",
        "range",
        "repeat",
        "replace",
        "reverse",
        "tolower",
        "totitle",
        "toupper",
        "trim",
        "trimleft",
        "trimright",
        "wordend",
        "wordstart",
    ];

    /// Tcl 8.5's `string` table: no `cat` (added in 8.6.2).
    const STRING_85: &[&str] = &[
        "bytelength",
        "compare",
        "equal",
        "first",
        "index",
        "is",
        "last",
        "length",
        "map",
        "match",
        "range",
        "repeat",
        "replace",
        "reverse",
        "tolower",
        "totitle",
        "toupper",
        "trim",
        "trimleft",
        "trimright",
        "wordend",
        "wordstart",
    ];

    fn string_86() -> KeywordTable<'static> {
        KeywordTable::new(STRING_86.iter().copied(), PrefixMatching::Enabled)
    }

    #[test]
    fn unique_prefixes_resolve_to_the_canonical_keyword() {
        let table = string_86();
        // tclsh-proof (8.6.16): `string le abc` → 3, i.e. `string length`.
        for word in ["le", "len", "leng", "lengt", "length"] {
            assert_eq!(
                table.resolve(word),
                KeywordMatch::Unique("length"),
                "{word}"
            );
        }
        assert_eq!(table.resolve("eq"), KeywordMatch::Unique("equal"));
        assert_eq!(table.resolve("ca"), KeywordMatch::Unique("cat"));
    }

    #[test]
    fn ambiguous_prefixes_report_every_candidate() {
        let table = string_86();
        // tclsh-proof (8.6.16): `string l abc` → `unknown or ambiguous
        // subcommand "l"`. `l` prefixes both `last` and `length`.
        assert_eq!(
            table.resolve("l"),
            KeywordMatch::Ambiguous(vec!["last", "length"])
        );
        // `c` prefixes cat/compare in 8.6.
        assert_eq!(
            table.resolve("c"),
            KeywordMatch::Ambiguous(vec!["cat", "compare"])
        );
        assert!(table.resolve("l").is_ambiguous());
        assert_eq!(table.resolve("l").unique(), None);
    }

    #[test]
    fn an_exact_spelling_always_wins_over_a_prefix() {
        // `trim` is an exact keyword *and* a prefix of trimleft/trimright.
        // tclsh-proof (8.6.16): `string trim " x "` → `x`, not ambiguous.
        let table = string_86();
        assert_eq!(table.resolve("trim"), KeywordMatch::Unique("trim"));
        assert_eq!(
            table.resolve("tri"),
            KeywordMatch::Ambiguous(vec!["trim", "trimleft", "trimright"])
        );
    }

    #[test]
    fn unknown_words_are_not_ambiguous() {
        let table = string_86();
        assert_eq!(table.resolve("zzz"), KeywordMatch::Unknown);
        assert_eq!(table.resolve(""), KeywordMatch::Unknown);
    }

    #[test]
    fn strict_tables_accept_only_exact_spellings() {
        let table = KeywordTable::new(STRING_86.iter().copied(), PrefixMatching::Strict);
        assert_eq!(table.resolve("length"), KeywordMatch::Unique("length"));
        // Not `Ambiguous`: in a strict table an abbreviation is simply not a
        // keyword, so it takes the unknown-keyword path.
        assert_eq!(table.resolve("le"), KeywordMatch::Unknown);
        assert_eq!(table.resolve("l"), KeywordMatch::Unknown);
        assert_eq!(table.minimal_unique_prefix("length"), None);
    }

    #[test]
    fn min_abbrev_raises_the_floor_above_uniqueness() {
        let table = KeywordTable::from_keywords(
            [
                Keyword {
                    name: "verbose",
                    min_abbrev: Some(4),
                },
                Keyword::new("quiet"),
            ],
            PrefixMatching::Enabled,
        );
        assert_eq!(table.resolve("v"), KeywordMatch::Unknown);
        assert_eq!(table.resolve("ver"), KeywordMatch::Unknown);
        assert_eq!(table.resolve("verb"), KeywordMatch::Unique("verbose"));
        assert_eq!(table.minimal_unique_prefix("verbose"), Some("verb"));
        assert_eq!(table.minimal_unique_prefix("quiet"), Some("q"));
    }

    #[test]
    fn minimal_unique_prefix_is_the_shortest_legal_spelling() {
        let table = string_86();
        assert_eq!(table.minimal_unique_prefix("length"), Some("le"));
        assert_eq!(table.minimal_unique_prefix("last"), Some("la"));
        assert_eq!(table.minimal_unique_prefix("cat"), Some("ca"));
        assert_eq!(table.minimal_unique_prefix("equal"), Some("e"));
        // A keyword that is a proper prefix of another can only be written in
        // full — `trim` shortened to `tri` would be ambiguous.
        assert_eq!(table.minimal_unique_prefix("trim"), Some("trim"));
        assert_eq!(table.minimal_unique_prefix("nosuch"), None);
        // Every keyword's minimal prefix round-trips to that keyword.
        for name in STRING_86 {
            let short = table.minimal_unique_prefix(name).expect(name);
            assert_eq!(table.resolve(short), KeywordMatch::Unique(name), "{name}");
        }
    }

    #[test]
    fn a_range_is_unique_only_when_every_version_agrees() {
        let v85 = KeywordTable::new(STRING_85.iter().copied(), PrefixMatching::Enabled);
        let v86 = string_86();
        // `ca` is unique in both (nothing else starts with `ca` in 8.5 either,
        // where it resolves to nothing at all) — check the interesting case
        // instead: `co` is `compare` in both.
        assert_eq!(
            resolve_over_versions(&[v85.clone(), v86.clone()], "co"),
            KeywordMatch::Unique("compare")
        );
        // `c` is unique (`compare`) in 8.5 but ambiguous in 8.6 once `cat`
        // exists, so the range must not treat it as unique.
        assert_eq!(v85.resolve("c"), KeywordMatch::Unique("compare"));
        assert!(v86.resolve("c").is_ambiguous());
        assert_eq!(
            resolve_over_versions(&[v85.clone(), v86.clone()], "c"),
            KeywordMatch::Ambiguous(vec!["cat", "compare"])
        );
        // A keyword missing from an older version is not resolvable for a
        // range that includes it.
        assert_eq!(
            resolve_over_versions(&[v85, v86], "cat"),
            KeywordMatch::Unknown
        );
        assert_eq!(resolve_over_versions(&[], "length"), KeywordMatch::Unknown);
    }

    #[test]
    fn boolean_prefixes_match_tcl_getboolean() {
        // tclsh-proof (8.6.16): each of these is what `expr {$w ? 1 : 0}`
        // returns for the word, with `o` raising `expected boolean value`.
        for word in ["t", "tr", "tru", "true", "y", "ye", "yes", "on", "1"] {
            assert_eq!(resolve_boolean(word), Some(true), "{word}");
        }
        for word in [
            "f", "fa", "fal", "fals", "false", "n", "no", "of", "off", "0",
        ] {
            assert_eq!(resolve_boolean(word), Some(false), "{word}");
        }
        // `o` is the one ambiguous boolean prefix.
        assert_eq!(resolve_boolean("o"), None);
        assert!(boolean_table().resolve("o").is_ambiguous());
        assert_eq!(
            boolean_table().resolve("o"),
            KeywordMatch::Ambiguous(vec!["off", "on"])
        );
        assert!(is_boolean_word("o"));
        // Non-booleans.
        for word in ["x", "tx", "", "2", "onn"] {
            assert_eq!(resolve_boolean(word), None, "{word}");
            assert!(!is_boolean_word(word), "{word}");
        }
        // Case-insensitive, like Tcl's own lower-casing.
        assert_eq!(resolve_boolean("TRUE"), Some(true));
        assert_eq!(resolve_boolean("Off"), Some(false));
    }

    #[test]
    fn boolean_minimal_prefixes_are_the_single_letters() {
        let table = boolean_table();
        assert_eq!(table.minimal_unique_prefix("true"), Some("t"));
        assert_eq!(table.minimal_unique_prefix("false"), Some("f"));
        assert_eq!(table.minimal_unique_prefix("yes"), Some("y"));
        assert_eq!(table.minimal_unique_prefix("no"), Some("n"));
        // `on`/`off` collide at one letter, so they need two/three.
        assert_eq!(table.minimal_unique_prefix("on"), Some("on"));
        assert_eq!(table.minimal_unique_prefix("off"), Some("of"));
        assert_eq!(table.minimal_unique_prefix("0"), Some("0"));
        assert_eq!(table.minimal_unique_prefix("1"), Some("1"));
    }

    #[test]
    fn option_tables_prefix_match_like_subcommand_tables() {
        // tclsh-proof (8.6.16): `lsearch -noc {a b} a` → 0;
        // `lsearch -a {a b} a` → `ambiguous option "-a"`.
        let table = KeywordTable::new(
            [
                "-all",
                "-ascii",
                "-bisect",
                "-decreasing",
                "-dictionary",
                "-exact",
                "-glob",
                "-increasing",
                "-index",
                "-inline",
                "-integer",
                "-nocase",
                "-not",
                "-real",
                "-regexp",
                "-sorted",
                "-start",
                "-subindices",
            ],
            PrefixMatching::Enabled,
        );
        assert_eq!(table.resolve("-noc"), KeywordMatch::Unique("-nocase"));
        assert_eq!(table.resolve("-e"), KeywordMatch::Unique("-exact"));
        assert_eq!(
            table.resolve("-a"),
            KeywordMatch::Ambiguous(vec!["-all", "-ascii"])
        );
        assert_eq!(table.minimal_unique_prefix("-nocase"), Some("-noc"));
    }
}
