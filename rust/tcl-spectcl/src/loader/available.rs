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

//! The `SpecTcl` 2.0 `available` word — the §4 availability algebra.
//!
//! `docs/design/dialect-and-package-registry-redesign.md` §6.2 gives every
//! scope that accepts `dialects` today a second spelling:
//!
//! ```text
//! available {tcl 8.6-} {package Tk}
//! option -x -available {tcl 8.4-9.0}
//! ```
//!
//! Each `{PROVIDER SPEC…}` row names one provider — a core family (`tcl`,
//! `f5-irules`, `jim`) or a `package NAME` — and, for the families that
//! have a ladder, a window in Tcl requirement syntax (`8.6-`, `8.4-9.0`,
//! or a bare `8.5` naming that one release line).
//!
//! ## Why this is a translation, not a second representation
//!
//! §6.1 is explicit that 2.0 changes meaning through *a new word plus a
//! translation of the legacy word*, never through per-version dispatch.
//! The legacy direction (1.x `dialects` → the new algebra) is what the
//! loader already does; this module is the **other** direction, and it is
//! total: an `available` row is projected
//! onto exactly the fields `dialects` and `required_package` feed, so a
//! command body spelled either way loads to a byte-equal
//! [`CommandSpec`](tcl_registry::spec::CommandSpec). `tcl spec upgrade`'s
//! U2 rewrite and the round-trip tests beside it are the proof.
//!
//! ## What the projection still cannot hold
//!
//! A **per-package window**. `required_package` is a bare name, so
//! `{package Tk 8.5-8.6}` carries the name and reports that the window is
//! not representable yet.
//!
//! Jim used to be the other one: the retired `SpecSurface` had no Jim bit,
//! so `available {jim 0.78-}` contributed nothing and the command was
//! gated off. A row names its family directly now (Q13), so a Jim window
//! projects exactly as a Tcl one does.

use tcl_dialect::model::{SpecWindow};
use std::sync::LazyLock;

use tcl_dialect::model::{Family, Version, VersionAxisId, VersionSet};

use super::{Log, Stmt, leak_str, list_words};
use tcl_dialect::model::{SpecSurface};

/// What one `available` row set translates to in the 1.x fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Availability {
    /// The projected dialect set, `None` when no row parsed (the same
    /// "said nothing" answer [`super::parse_dialects`] gives).
    pub(super) surface: Option<&'static [SpecSurface]>,
    /// The package a `{package NAME}` row requires, when the name is not
    /// one the 1.x dialect bits already cover.
    pub(super) required_package: Option<&'static str>,
}

/// The provider keywords a spec row may open with. Closed: an unknown
/// first word is an unknown provider, not a free-form name.
const PROVIDERS: &[&str] = &["tcl", "f5-irules", "jim", "package"];

/// Packages a `SpecTcl` 1.x dialect bit already names, so an `available`
/// row spelling them lands on that bit rather than on `required_package`.
///
/// This is the exact inverse of `tcl spec upgrade`'s U2 rule `tk` →
/// `{package Tk}`, and it is what makes the two spellings byte-equal.
const PACKAGE_DIALECT_SURFACES: &[(&str, &[SpecSurface])] = &[("Tk", SpecSurface::TK)];

/// The **environment-derived** half of the same table (upgrade spec U3):
/// each compiled environment whose surface is one ambient package and
/// whose id has a 1.x dialect bit pairs that package's name with the bit.
///
/// This is what makes `available {package f5-iapps-cmds}` load byte-equal
/// to the 1.x `dialects f5-iapps` it replaces: the environment registry
/// answers *which package at which placement* stands behind each
/// environment-membership token, and this projection carries the answer
/// back onto the closed 1.x bit vocabulary. Derived, not hand-written, so
/// a seeded environment cannot drift from its own translation.
static ENVIRONMENT_PACKAGE_SURFACES: LazyLock<Vec<(String, &'static [SpecSurface])>> =
    LazyLock::new(|| {
    tcl_dialect::model::compiled_definitions()
        .into_iter()
        .filter(|definition| definition.core.is_some())
        .filter_map(|definition| {
            let bit = crate::catalogue::dialect_surface(definition.id.as_str())?;
            let ambient: Vec<_> = definition
                .expected_packages
                .iter()
                .filter(|placement| placement.ambient)
                .collect();
            let [sole] = ambient.as_slice() else {
                return None;
            };
            Some((sole.package.as_ref().to_owned(), bit))
        })
        .collect()
});

/// The 1.x dialect bit standing behind `package`, when one does.
pub(crate) fn package_surface(package: &str) -> Option<&'static [SpecSurface]> {
    PACKAGE_DIALECT_SURFACES
        .iter()
        .find(|(name, _)| *name == package)
        .map(|(_, bit)| *bit)
        .or_else(|| {
            ENVIRONMENT_PACKAGE_SURFACES
                .iter()
                .find(|(name, _)| name == package)
                .map(|(_, bit)| *bit)
        })
}

/// `available SPEC ?SPEC…?` at property scope, taking every word after the
/// first `skip`.
pub(super) fn from_statement(stmt: &Stmt, skip: usize, log: &mut Log) -> Availability {
    let words: Vec<String> = stmt
        .words
        .iter()
        .skip(skip)
        .map(|word| word.text.clone())
        .collect();
    from_texts(&words, stmt.line, log)
}

/// `-available VALUE` at flag scope, where `VALUE` is either one spec row
/// or a list of them.
pub(super) fn from_flag(text: &str, line: u32, log: &mut Log) -> Availability {
    from_texts(&list_words(text), line, log)
}

/// Translate a whole `available` word into the 1.x fields.
pub(super) fn from_texts(words: &[String], line: u32, log: &mut Log) -> Availability {
    if words.is_empty() {
        log.say(line, "`available` needs at least one `{PROVIDER …}` row");
        return Availability::default();
    }
    let mut rows: Vec<SpecSurface> = Vec::new();
    let mut required_package = None;
    let mut parsed_any = false;
    for row in split_rows(words) {
        let Some(row) = parse_row(&row, line, log) else {
            continue;
        };
        parsed_any = true;
        rows.extend(row.surface);
        if let Some(name) = row.package {
            match required_package {
                None => required_package = Some(name),
                Some(prior) if prior == name => {}
                Some(prior) => log.say(
                    line,
                    format!(
                        "`available` names package `{name}`, but this declaration already \
                         requires `{prior}`; the first is kept"
                    ),
                ),
            }
        }
    }
    Availability {
        surface: parsed_any.then(|| &*Box::leak(rows.into_boxed_slice())),
        required_package,
    }
}

/// Split the word's contents into provider-spec rows.
///
/// One row (`available {tcl 8.6-}`, `-available {tcl 8.6-}`) and a list of
/// rows (`available {tcl 8.6-} {jim 0.78-}`, `-available {{f5-irules} {tcl
/// 8.4-}}`) are told apart by the closed provider vocabulary: the words
/// are a single row exactly when the first is a provider keyword and no
/// later word *starts* with one. Nothing else can be ambiguous, because a
/// row's non-leading words are versions and package names, never provider
/// keywords.
fn split_rows(words: &[String]) -> Vec<Vec<String>> {
    let leads_with_provider = |text: &str| {
        list_words(text)
            .first()
            .is_some_and(|first| PROVIDERS.contains(&first.as_str()))
    };
    let single = words
        .first()
        .is_some_and(|first| PROVIDERS.contains(&first.as_str()))
        && !words[1..].iter().any(|word| leads_with_provider(word));
    if single {
        return vec![words.to_vec()];
    }
    words.iter().map(|word| list_words(word)).collect()
}

/// One parsed provider row, in the spec fields it feeds.
struct Row {
    surface: Vec<SpecSurface>,
    package: Option<&'static str>,
}

fn parse_row(words: &[String], line: u32, log: &mut Log) -> Option<Row> {
    let provider = words.first().map(String::as_str).unwrap_or_default();
    let rest = &words[1.min(words.len())..];
    match provider {
        "tcl" => core_row(Family::Tcl, rest, line, log),
        "jim" => core_row(Family::Jim, rest, line, log),
        "f5-irules" => {
            if !rest.is_empty() {
                log.say(
                    line,
                    "`available {f5-irules …}` takes no version window — the family has \
                     one release; the window is ignored",
                );
            }
            Some(Row {
                surface: SpecSurface::IRULES.to_vec(),
                package: None,
            })
        }
        // Q3: `f5-bigip` is a configuration surface off the Tcl axis
        // entirely, so it can never be an availability window. Translating
        // it would claim a Tcl command exists there.
        "f5-bigip" => {
            log.say(
                line,
                "`f5-bigip` is a configuration surface off the Tcl axis and is not an \
                 `available` provider (design §2, Q3); the row is dropped",
            );
            None
        }
        "package" => package_row(rest, line, log),
        "" => {
            log.say(line, "an empty `available` row was dropped");
            None
        }
        other => {
            log.say(
                line,
                format!(
                    "unknown `available` provider `{other}` dropped; the providers are \
                     `tcl`, `f5-irules`, `jim`, and `package NAME`"
                ),
            );
            None
        }
    }
}

/// `tcl RANGE` / `jim RANGE` — a window on a core family's ladder.
fn core_row(family: Family, rest: &[String], line: u32, log: &mut Log) -> Option<Row> {
    let window = window_of(VersionAxisId::core(family), rest, line, log)?;
    let ladder: Vec<&str> = family
        .releases()
        .iter()
        .map(|release| release.as_str())
        .collect();
    // The ladder positions the window covers, merged into runs, so a
    // contiguous span is one window rather than one per release — the same
    // shape a compiled spec writes by hand.
    let covered: Vec<usize> = ladder
        .iter()
        .enumerate()
        .filter(|(_, release)| {
            Version::parse(release).is_ok_and(|version| window.contains(&version))
        })
        .map(|(position, _)| position)
        .collect();
    if covered.is_empty() {
        log.say(
            line,
            format!(
                "`available {{{} …}}` covers no release of the {} ladder; the row is dropped",
                family.name(),
                family.name()
            ),
        );
        return None;
    }
    let mut windows: Vec<SpecWindow> = Vec::new();
    let mut run = (covered[0], covered[0]);
    for &position in &covered[1..] {
        if position == run.1 + 1 {
            run.1 = position;
        } else {
            windows.push(ladder_window(ladder[run.0], ladder[run.1]));
            run = (position, position);
        }
    }
    windows.push(ladder_window(ladder[run.0], ladder[run.1]));
    Some(Row {
        surface: vec![SpecSurface::core_in(family, leak_windows(windows))],
        package: None,
    })
}

/// The half-open window a run of ladder releases spans: from the first
/// release to just past the last.
///
/// "Just past" is the next minor of the run's last release — `8.6` ends at
/// `8.7`, not at the ladder's next entry `9.0` — so a patch release inside
/// the line (`8.6.16`) stays covered and a later line does not leak in.
fn ladder_window(first: &str, last: &str) -> SpecWindow {
    (leak_str(first), Some(leak_str(&next_minor(last))))
}

/// `8.6` → `8.7`, `0.84` → `0.85`.
fn next_minor(release: &str) -> String {
    match release.rsplit_once('.') {
        Some((major, minor)) => minor
            .parse::<u32>()
            .map_or_else(|_| release.to_owned(), |n| format!("{major}.{}", n + 1)),
        None => release.to_owned(),
    }
}

/// Intern a window list for the process lifetime, for the same reason.
fn leak_windows(windows: Vec<SpecWindow>) -> &'static [SpecWindow] {
    Box::leak(windows.into_boxed_slice())
}

/// `package NAME ?RANGE?`.
fn package_row(rest: &[String], line: u32, log: &mut Log) -> Option<Row> {
    let Some(name) = rest.first() else {
        log.say(line, "`available {package …}` needs a package name");
        return None;
    };
    let range = &rest[1..];
    if !range.is_empty() && window_of(VersionAxisId::package(name), range, line, log).is_none() {
        return None;
    }
    if !range.is_empty() {
        log.say(
            line,
            format!(
                "`available {{package {name} {}}}`: a per-package version window has no \
                 SpecTcl 1.x field, so only the package name is carried",
                range.join(" ")
            ),
        );
    }
    if let Some(rows) = package_surface(name) {
        return Some(Row {
            surface: rows.to_vec(),
            package: None,
        });
    }
    Some(Row {
        surface: Vec::new(),
        package: Some(leak_str(name)),
    })
}

/// The version set a window's requirements describe, on `axis`.
///
/// An absent window is the whole axis. A bare `X.Y` names **that release
/// line only** (upgrade spec U2's `tclX.Y` → single-point rule), which is
/// where availability windows deliberately part company with `package
/// require`'s "bounded at the next major" reading: `8.5` there means
/// 8.5-through-9, and a `dialects tcl8.5` row has never meant that.
fn window_of(
    axis: VersionAxisId,
    requirements: &[String],
    line: u32,
    log: &mut Log,
) -> Option<VersionSet> {
    if requirements.is_empty() {
        return VersionSet::from_requirements(axis, &["0-"]).ok();
    }
    let spelled: Vec<String> = requirements
        .iter()
        .map(|requirement| {
            if requirement.contains('-') {
                requirement.clone()
            } else {
                format!("{requirement}-{requirement}")
            }
        })
        .collect();
    match VersionSet::from_requirements(axis, &spelled) {
        Ok(set) => Some(set),
        Err(err) => {
            log.say(
                line,
                format!(
                    "`available {{… {}}}` is not a version window ({err}); the row is dropped",
                    requirements.join(" ")
                ),
            );
            None
        }
    }
}

