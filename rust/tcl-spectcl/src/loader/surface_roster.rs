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

//! **`include from SOURCE into TARGET ?-available {WINDOW…}? {names…}`** —
//! the surface-composition half of `SpecTcl` 2.0's `include`, design
//! **Q6** (§6.2's optional row, ruled 2026-08-28).
//!
//! `include NAME` composes *files*: it splices another `.tclspec`'s
//! declarations in. This row composes *surfaces*: it says which of one
//! family's command names another family, which reimplements it,
//! actually has. The two share a word because they are the same idea at
//! two scales, and they are told apart by the second word — `from` is
//! never a file name (`include_name` rejects it in one place, so the
//! discrimination cannot drift).
//!
//! ## Why the row names both ends
//!
//! §6.2 writes the word as `include from PROVIDER {names…}`, with the
//! target implied by the declaring pack. That works for a pack that
//! *is* a surface, but a roster is a two-ended fact — jim's roster of
//! Tcl's names — and the target is a compiled family the pack cannot
//! otherwise claim (`dialect jim { … }` is refused: compiled family
//! names are reserved, §6.4). Writing `into TARGET` says the second end
//! out loud rather than deriving it from which file the row happens to
//! sit in.
//!
//! ## Windows
//!
//! `-available {0.77-}` is a requirement list on the **target's** own
//! axis — the target is already named, so naming a provider again would
//! be noise. It is how the two names Jim grew mid-ladder are written:
//! `interp` from 0.77, `zlib` from 0.78. A row with no `-available`
//! covers the whole ladder.
//!
//! What the accepted rows become is
//! [`tcl_dialect::model::InheritedSurface`]; see
//! [`crate::surface_roster_conversion`].

use tcl_dialect::model::{Family, VersionAxisId, VersionSet};

use super::{Log, Stmt, VocabularyClass, list_words};

/// One name on a roster, with the window on the target's ladder over
/// which the target has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRosterName {
    /// The command name, as the ancestor provides it.
    pub name: String,
    /// Requirement spellings on the target's axis (`0.77-`), empty for
    /// the whole ladder.
    pub window: Vec<String>,
}

/// One parsed `include from … into … { … }` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSurfaceRoster {
    /// The family whose surface is enumerated (`tcl`).
    pub source: String,
    /// The reimplementing family the roster is for (`jim`).
    pub target: String,
    /// The names, in declaration order.
    pub names: Vec<PackRosterName>,
    /// The declaring line.
    pub line: u32,
}

/// Whether `words` (everything after the `include` word) is the
/// surface-composition form rather than the file form.
#[must_use]
pub(crate) fn is_surface_row(words: &[&str]) -> bool {
    words.first().is_some_and(|first| *first == "from")
}

/// Parse one surface row from the words after `include`.
///
/// Every rejection drops the row and says so: a roster that loaded
/// *partly* would narrow a family's surface by an amount nobody wrote.
pub(crate) fn parse(words: &[&str], line: u32, log: &mut Log) -> Option<PackSurfaceRoster> {
    let reject = |log: &mut Log, message: String| -> Option<PackSurfaceRoster> {
        log.say_classified(line, VocabularyClass::Semantic, message);
        None
    };
    // words: from SOURCE into TARGET ?-available {…}? {names…}
    if words.len() < 5 || words[0] != "from" || words[2] != "into" {
        return reject(
            log,
            "`include from` is written `include from SOURCE into TARGET ?-available {WINDOW}? \
             {names…}`; the row is dropped and no surface is enumerated"
                .to_owned(),
        );
    }
    let source = words[1];
    let target = words[3];
    if family_named(source).is_none() {
        return reject(
            log,
            format!(
                "`include from {source}` does not name a compiled family; the row is dropped \
                 and no surface is enumerated"
            ),
        );
    }
    if family_named(target).is_none() {
        return reject(
            log,
            format!(
                "`include … into {target}` does not name a compiled family; the row is dropped \
                 and no surface is enumerated"
            ),
        );
    }
    let mut window: Vec<String> = Vec::new();
    let mut index = 4;
    while index < words.len() {
        match words[index] {
            "-available" => {
                let Some(text) = words.get(index + 1) else {
                    return reject(
                        log,
                        "`-available` needs a requirement list on the target's axis \
                         (`-available {0.77-}`); the row is dropped and no surface is enumerated"
                            .to_owned(),
                    );
                };
                window = list_words(text);
                index += 2;
            }
            _ => break,
        }
    }
    let [body] = words[index..] else {
        return reject(
            log,
            "`include from` takes exactly one braced list of command names after its flags; \
             the row is dropped and no surface is enumerated"
                .to_owned(),
        );
    };
    let names = list_words(body);
    if names.is_empty() {
        return reject(
            log,
            "`include from` was given no command names; the row is dropped and no surface is \
             enumerated"
                .to_owned(),
        );
    }
    // A window that does not parse on the target's axis would silently
    // widen to the whole ladder, which is the wrong direction to guess in.
    if !window.is_empty() {
        let axis = VersionAxisId::core(family_named(target).expect("checked above"));
        if VersionSet::from_requirements(axis, &window).is_err() {
            return reject(
                log,
                format!(
                    "`-available {{{}}}` is not a requirement list on `{target}`'s axis; the row \
                     is dropped and no surface is enumerated",
                    window.join(" ")
                ),
            );
        }
    }
    Some(PackSurfaceRoster {
        source: source.to_owned(),
        target: target.to_owned(),
        names: names
            .into_iter()
            .map(|name| PackRosterName {
                name,
                window: window.clone(),
            })
            .collect(),
        line,
    })
}

/// The compiled [`Family`] `name` spells, when it spells one.
#[must_use]
pub fn family_named(name: &str) -> Option<Family> {
    Family::ALL
        .into_iter()
        .find(|family| family.name() == name)
}

/// Parse a whole statement's worth of words — the replay-side entry, for
/// a row that reached the readers instead of the capture layer.
pub(crate) fn from_statement(stmt: &Stmt, log: &mut Log) -> Option<PackSurfaceRoster> {
    let words: Vec<&str> = stmt
        .words
        .iter()
        .skip(1)
        .map(|word| word.text.as_str())
        .collect();
    parse(&words, stmt.line, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_row(text: &str) -> (Option<PackSurfaceRoster>, Vec<String>) {
        let words: Vec<String> = list_words(text);
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        let mut log = Log::default();
        let row = parse(&refs, 1, &mut log);
        let messages = log.notices.iter().map(|n| n.message.clone()).collect();
        (row, messages)
    }

    #[test]
    fn a_well_formed_row_carries_both_ends_and_its_names() {
        let (row, messages) = parse_row("from tcl into jim {set proc if}");
        let row = row.expect("the row parses");
        assert!(messages.is_empty(), "{messages:?}");
        assert_eq!(row.source, "tcl");
        assert_eq!(row.target, "jim");
        assert_eq!(
            row.names.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
            ["set", "proc", "if"]
        );
        assert!(row.names.iter().all(|n| n.window.is_empty()));
    }

    #[test]
    fn a_window_rides_every_name_in_its_row() {
        let (row, messages) = parse_row("from tcl into jim -available {0.77-} {interp}");
        let row = row.expect("the row parses");
        assert!(messages.is_empty(), "{messages:?}");
        assert_eq!(row.names[0].window, ["0.77-"]);
    }

    /// `from` is what tells a surface row from a file row, and the file
    /// form must stay unreachable through this parser.
    #[test]
    fn the_two_include_forms_are_told_apart_by_from() {
        assert!(is_surface_row(&["from", "tcl", "into", "jim", "{set}"]));
        assert!(!is_surface_row(&["shared.tclspec"]));
    }

    #[test]
    fn every_malformed_row_is_dropped_whole_with_a_notice() {
        for text in [
            "from tcl into jim",
            "from tcl jim {set}",
            "from nonesuch into jim {set}",
            "from tcl into nonesuch {set}",
            "from tcl into jim {}",
            "from tcl into jim -available {not-a-version} {interp}",
            "from tcl into jim -available",
            "from tcl into jim {set} {proc}",
        ] {
            let (row, messages) = parse_row(text);
            assert!(row.is_none(), "`{text}` should not parse");
            assert_eq!(messages.len(), 1, "`{text}`: {messages:?}");
            assert!(
                messages[0].contains("no surface is enumerated"),
                "`{text}`: {messages:?}"
            );
        }
    }
}
