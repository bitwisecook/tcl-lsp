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

//! Resolving a subcommand word, and a second-level operation word, by unambiguous prefix under a dialect gate.

use super::CommandSpec;
use super::subcommand::{SubCommand, SubSubCommand, sub_subcommand_supports_dialect};
use crate::abbrev::{Keyword, KeywordMatch, KeywordTable, PrefixMatching};
use tcl_dialect::model::{SurfaceQuery, surface_admits};

impl CommandSpec {
    /// Look up a subcommand by exact name.
    #[must_use]
    pub fn subcommand(&self, name: &str) -> Option<&SubCommand> {
        self.subcommands.iter().find(|s| s.name == name)
    }

    /// This command's subcommand keyword table, filtered to what exists for
    /// `dialect` and `package_version`.
    ///
    /// The table is what [`crate::abbrev`] resolves against, so an
    /// abbreviated word is judged against exactly the keywords that exist at
    /// the target release — a prefix unique in 8.5 can be ambiguous in 8.6.
    ///
    /// `prefix_override` lets a caller force [`PrefixMatching::Strict`] for a
    /// call site the analyser saw configured with
    /// `namespace ensemble … -prefixes 0`; `None` uses the declared
    /// [`Self::prefix_matching`].
    #[must_use]
    pub fn subcommand_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        let parent = self.surface;
        KeywordTable::from_keywords(
            self.subcommands
                .iter()
                .filter(|sub| {
                    sub.surface
                        .or(parent)
                        .is_none_or(|have| surface_admits(have, dialect.as_ref()))
                })
                .filter(|sub| sub.available_for_version(package_version))
                .map(|sub| Keyword {
                    name: sub.name,
                    min_abbrev: sub.min_abbrev,
                }),
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// Resolve a subcommand *word* — exact, abbreviated, ambiguous, or
    /// unknown — against this command's table for `dialect` and
    /// `package_version`.
    ///
    /// This is the one entry point every consumer uses, so the analyser,
    /// semantic tokens, hover, completion, signature help, the formatter, and
    /// the minifier cannot drift apart on what an abbreviation means.
    #[must_use]
    pub fn resolve_subcommand_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.subcommand_table(dialect, package_version, prefix_override)
            .resolve(word)
    }

    /// Resolve a subcommand word to its [`SubCommand`], accepting a unique
    /// non-empty prefix the way Tcl's ensemble dispatch (`Tcl_GetIndexFromObj`)
    /// does: `string le` ⇒ `length`, `info ex` ⇒ `exists`. An exact match
    /// always wins over a prefix; an ambiguous prefix (several candidates, e.g.
    /// `string t`) resolves to `None`.
    ///
    /// Dialect-agnostic — every declared subcommand is a candidate. Prefer
    /// [`Self::resolve_subcommand_for_dialect`] where the active Tcl version is
    /// known, since a prefix's uniqueness can change between versions
    /// (`info class def` is `definition` in 8.6 but ambiguous with
    /// `definitionnamespace` in 9.0).
    #[must_use]
    pub fn resolve_subcommand(&self, word: &str) -> Option<&SubCommand> {
        self.resolve_subcommand_bytes(word.as_bytes())
    }

    /// Resolve selected operand bytes without Unicode conversion or an input
    /// extent policy. The caller supplies those independent native facts.
    #[must_use]
    pub fn resolve_subcommand_bytes(&self, word: &[u8]) -> Option<&SubCommand> {
        self.resolve_subcommand_filtered(word, |_| true)
    }

    /// Like [`Self::resolve_subcommand`] but only considers subcommands
    /// available in `dialect`, so prefix uniqueness matches the given Tcl
    /// version exactly.
    #[must_use]
    pub fn resolve_subcommand_for_dialect(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&SubCommand> {
        self.resolve_subcommand_bytes_for_dialect(word.as_bytes(), dialect)
    }

    /// Byte-valued counterpart of [`Self::resolve_subcommand_for_dialect`].
    /// Availability and prefix matching come from the same descriptor table.
    #[must_use]
    pub fn resolve_subcommand_bytes_for_dialect(
        &self,
        word: &[u8],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&SubCommand> {
        let parent = self.surface;
        self.resolve_subcommand_filtered(word, |s| match s.surface.or(parent) {
            Some(d) => surface_admits(d, dialect.as_ref()),
            None => true,
        })
    }

    /// Resolve the namespace dispatcher's separate exact and abbreviation
    /// extents against this actual dialect-filtered descriptor table. This
    /// selects metadata only; it grants no native table/header cache receipt.
    #[must_use]
    pub fn resolve_native_namespace_subcommand_bytes_for_dialect(
        &self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        original: &[u8],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&SubCommand> {
        let exact = protocol.namespace_subcommand_input(original);
        let canonical = self
            .subcommand_table(dialect, None, Some(PrefixMatching::Strict))
            .resolve_bytes(exact.selected())
            .unique()
            .or_else(|| {
                let prefix = protocol.namespace_subcommand_prefix_input(original);
                self.subcommand_table(dialect, None, None)
                    .resolve_bytes(prefix.selected())
                    .unique()
            })?;
        self.resolve_subcommand_bytes_for_dialect(canonical.as_bytes(), dialect)
    }

    fn resolve_subcommand_filtered(
        &self,
        word: &[u8],
        avail: impl Fn(&SubCommand) -> bool,
    ) -> Option<&SubCommand> {
        // One matcher for the whole codebase: build the visible table and ask
        // `crate::abbrev`. `Ambiguous` and `Unknown` both collapse to `None`
        // here because this accessor's contract is "the subcommand or
        // nothing"; callers that need to tell the two apart (the ambiguity
        // diagnostic, the formatter's expansion) use
        // [`Self::resolve_subcommand_word`] instead.
        let table = KeywordTable::from_keywords(
            self.subcommands
                .iter()
                .filter(|s| avail(s))
                .map(|s| Keyword {
                    name: s.name,
                    min_abbrev: s.min_abbrev,
                }),
            self.prefix_matching,
        );
        let canonical = table.resolve_bytes(word).unique()?;
        self.subcommands
            .iter()
            .find(|s| s.name == canonical && avail(s))
    }
}

impl SubCommand {
    /// Resolve a second-level subcommand word to its [`SubSubCommand`],
    /// using this subcommand's authored [`Self::prefix_matching`] policy.
    /// Enabled tables accept a unique non-empty prefix (`info object cl` ⇒
    /// `class`); strict tables require an exact word. An ambiguous prefix
    /// resolves to `None`.
    #[must_use]
    pub fn resolve_sub_subcommand(&self, word: &str) -> Option<&'static SubSubCommand> {
        self.resolve_sub_subcommand_filtered(word, |_| true)
    }

    /// Like [`Self::resolve_sub_subcommand`] but only considers second-level
    /// subcommands available in `dialect`, so a prefix's uniqueness matches the
    /// given Tcl version (`info class def` is `definition` in 8.6 but ambiguous
    /// with `definitionnamespace` in 9.0).
    #[must_use]
    pub fn resolve_sub_subcommand_for_dialect(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static SubSubCommand> {
        self.resolve_sub_subcommand_gated(word, dialect, None)
    }

    /// The second-level subcommands available for `dialect` and
    /// `package_version`, in declaration order — the table completion and the
    /// analyser judge a third-level word against, so an operation the target
    /// release does not have is neither offered nor accepted.
    ///
    /// `None` on either gate is permissive, matching every other availability
    /// query in this module.
    #[must_use]
    pub fn available_sub_subcommands(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Vec<&'static SubSubCommand> {
        let parent = self.surface;
        self.sub_subcommands
            .iter()
            .filter(|s| sub_subcommand_supports_dialect(s, dialect, parent))
            .filter(|s| s.available_for_version(package_version))
            .collect()
    }

    /// Whether `operation`, one of this member's operations, exists at
    /// `dialect`, as [`Self::resolve_sub_subcommand_gated`] judges it.
    pub(crate) fn admits_sub_subcommand(
        &self,
        operation: &SubSubCommand,
        dialect: SurfaceQuery<'_>,
    ) -> bool {
        sub_subcommand_supports_dialect(operation, Some(dialect), self.surface)
    }

    /// [`Self::resolve_sub_subcommand`] narrowed by both availability axes:
    /// the Tcl-core `dialect` and the owning package's `package_version`
    /// floor.
    ///
    /// Prefix uniqueness is judged against exactly the operations that exist
    /// at the target, so `info class def` can be unambiguous on one release
    /// and ambiguous on the next.
    #[must_use]
    pub fn resolve_sub_subcommand_gated(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Option<&'static SubSubCommand> {
        let parent = self.surface;
        self.resolve_sub_subcommand_filtered(word, |s| {
            sub_subcommand_supports_dialect(s, dialect, parent)
                && s.available_for_version(package_version)
        })
    }

    fn resolve_sub_subcommand_filtered(
        &self,
        word: &str,
        avail: impl Fn(&SubSubCommand) -> bool,
    ) -> Option<&'static SubSubCommand> {
        let available: Vec<&'static SubSubCommand> = self
            .sub_subcommands
            .iter()
            .filter(|sub| avail(sub))
            .collect();
        let names: Vec<&'static str> = available.iter().map(|sub| sub.name).collect();
        tcl_cmd_core::ensemble::resolve_subcommand(
            &names,
            word.as_bytes(),
            self.prefix_matching.accepts_prefixes(),
        )
        .map(|index| available[index])
    }

    /// Whether `word` resolves to a second-level subcommand of this
    /// two-level-ensemble subcommand (exact or unique-prefix; see
    /// [`Self::resolve_sub_subcommand`]).
    #[must_use]
    pub fn is_sub_subcommand(&self, word: &str) -> bool {
        self.resolve_sub_subcommand(word).is_some()
    }
}
