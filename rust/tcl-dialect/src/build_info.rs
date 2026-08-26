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

//! `::tcl::build-info` — the build-identification string and the queries C
//! answers over it (`BuildInfoObjCmd`, `generic/tclBasic.c`, Tcl 9.0+).
//!
//! This lives beside the release vocabulary rather than inside either engine
//! because the leading component *is* the pinned release's patch level: an
//! engine that keeps its own copy reports a build identity that disagrees with
//! its own `[info patchlevel]` (ledger row B4). Both `tcl-vm` and
//! `runtime/rust` compose and query the string through here, so they cannot
//! answer `::tcl::build-info patchlevel` differently for the same pin.
//!
//! Reference transcripts (the interpreters on `PATH`):
//!
//! ```text
//! tclsh9.0 % ::tcl::build-info
//! 9.0.4+git-abe35fa7….gcc-1303.static.tommath-0103
//! tclsh9.0 % ::tcl::build-info version       → 9.0
//! tclsh9.0 % ::tcl::build-info patchlevel    → 9.0.4
//! tclsh9.1 % ::tcl::build-info version       → 9.1b0
//! tclsh9.1 % ::tcl::build-info patchlevel    → 9.1b0
//! tclsh8.6 % ::tcl::build-info               → invalid command name
//! ```
//!
//! 9.1's `version` answering `9.1b0` rather than `9.1` is not a special case:
//! C returns the text up to the *second* `.` and there is no second `.` in
//! `9.1b0+…`, so it falls back to the `+` boundary. The same rule reproduces
//! both releases.

use crate::TclVersion;

/// The commit field this project's engines report.
///
/// Neither engine is built from a Tcl git checkout, so there is no upstream
/// commit to name. C's own field is a 40-character hex SHA; an all-zero SHA is
/// the conventional "no commit" spelling and keeps the string's *shape* parseable
/// by scripts that split it, rather than inventing a plausible-looking hash.
const NO_COMMIT: &str = "0000000000000000000000000000000000000000";

/// The `::tcl::build-info` string for an engine pinned to `version`.
///
/// Shaped exactly like C's — `<patchlevel>+<commit>.<word>…` — with the
/// trailing words naming this project's engine family rather than a C
/// compiler and its options.
#[must_use]
pub fn build_info(version: TclVersion, engine: &str) -> String {
    format!("{}+{NO_COMMIT}.{engine}", version.patchlevel())
}

/// Answer `::tcl::build-info <option>` over an already-composed `data`
/// string, following `BuildInfoObjCmd`:
///
/// - `patchlevel` — everything before the `+`.
/// - `version` — everything before the second `.`, or before the `+` when the
///   patch level has no second `.` (a beta such as `9.1b0`).
/// - `commit` — the `+`…`.` segment.
/// - any other word — the value of a `name-value` suffix word, `1` for a bare
///   `name` suffix word, and `0` when no suffix word matches.
#[must_use]
pub fn query<'a>(data: &'a str, option: &str) -> &'a str {
    let plus = data.find('+');
    match option {
        "patchlevel" => &data[..plus.unwrap_or(data.len())],
        "version" => {
            let second_dot = data
                .match_indices('.')
                .nth(1)
                .map(|(i, _)| i)
                .filter(|i| plus.is_none_or(|p| *i < p));
            &data[..second_dot.or(plus).unwrap_or(data.len())]
        }
        "commit" => match plus {
            Some(p) => {
                let rest = &data[p + 1..];
                &rest[..rest.find('.').unwrap_or(rest.len())]
            }
            None => "",
        },
        word => suffix_word(data, word).unwrap_or("0"),
    }
}

/// The value a dot-separated suffix word contributes for `word`: the text
/// after `word-` for a `name-value` spelling, `"1"` for a bare `name`, and
/// `None` when no suffix word starts with `word` at a word boundary.
fn suffix_word<'a>(data: &'a str, word: &str) -> Option<&'a str> {
    data.match_indices('.')
        .map(|(dot, _)| &data[dot + 1..])
        .find_map(|rest| {
            let tail = rest.strip_prefix(word)?;
            match tail.as_bytes().first() {
                None | Some(b'.') => Some("1"),
                Some(b'-') => {
                    let value = &tail[1..];
                    Some(&value[..value.find('.').unwrap_or(value.len())])
                }
                // `word` is only a prefix of this suffix word (`tommath`
                // queried as `tom`), so it is not a match — keep scanning.
                Some(_) => None,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{build_info, query};
    use crate::TclVersion;

    /// C's own `version`/`patchlevel` split, reproduced for both 9.x
    /// reference builds from their real transcripts (see the module docs).
    #[test]
    fn version_and_patchlevel_match_the_reference_interpreters() {
        // TP: 9.0.4's three-component patch level splits at the second `.`.
        let ninety = build_info(TclVersion::V9_0, "rust");
        assert_eq!(query(&ninety, "patchlevel"), "9.0.4");
        assert_eq!(query(&ninety, "version"), "9.0");

        // TP: 9.1b0 has no second `.`, so `version` runs to the `+` — which is
        // exactly what `tclsh9.1` answers (`9.1b0`, not `9.1`).
        let ninety_one = build_info(TclVersion::V9_1, "rust");
        assert_eq!(query(&ninety_one, "patchlevel"), "9.1b0");
        assert_eq!(query(&ninety_one, "version"), "9.1b0");
    }

    #[test]
    fn commit_and_feature_words() {
        let data = build_info(TclVersion::V9_0, "rust");
        // TP: the commit field is the `+`..`.` segment.
        assert_eq!(query(&data, "commit"), super::NO_COMMIT);
        // TP: a bare suffix word is present.
        assert_eq!(query(&data, "rust"), "1");
        // TN: an absent build flag reports `0` rather than erroring.
        assert_eq!(query(&data, "debug"), "0");

        // TP/FP guard on the `name-value` form and on prefix-only matches,
        // using C's own shape (`…gcc-1303.static.tommath-0103`).
        let c_shaped = "9.0.4+git-abe35fa7.gcc-1303.static.tommath-0103";
        assert_eq!(query(c_shaped, "commit"), "git-abe35fa7");
        assert_eq!(query(c_shaped, "gcc"), "1303");
        assert_eq!(query(c_shaped, "static"), "1");
        assert_eq!(query(c_shaped, "tommath"), "0103");
        // FP guard: `tom` is a strict prefix of `tommath`, not a suffix word.
        assert_eq!(query(c_shaped, "tom"), "0");
    }
}
