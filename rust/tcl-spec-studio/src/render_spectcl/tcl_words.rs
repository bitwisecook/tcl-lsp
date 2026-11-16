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

//! Tcl words for a row: quoting, list words, the availability algebra, dialect-set shorthands and lifecycle flags.

use super::*;

// Tcl words

/// A descriptor word must retain both raw loader text and its Unicode value.
pub(super) fn word(text: &str) -> Option<String> {
    use tcl_compiler::parsing::spectcl_source::{
        PackWordSourcePurpose, PackWordSourceStyle, pack_verbatim_source_word,
    };
    pack_verbatim_source_word(
        text,
        PackWordSourcePurpose::StaticDescriptor,
        PackWordSourceStyle::Compact,
    )
}

/// Prose and blocks preserve the explicit braced presentation.
pub(super) fn braced(text: &str) -> Option<String> {
    use tcl_compiler::parsing::spectcl_source::{
        PackWordSourcePurpose, PackWordSourceStyle, pack_verbatim_source_word,
    };
    pack_verbatim_source_word(
        text,
        PackWordSourcePurpose::StaticDescriptor,
        PackWordSourceStyle::Braced,
    )
}

/// Each element and the enclosing word are checked by the same actual owner.
pub(super) fn list_word<S: AsRef<str>>(items: &[S]) -> Option<String> {
    tcl_compiler::parsing::spectcl_source::pack_descriptor_list_source_word(items)
}

/// The elements of a JSON string array as a list word.
pub(super) fn str_list_word(value: &Value) -> Option<String> {
    let items: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    list_word(&items)
}

/// The `SpecTcl` 2.0 availability algebra, as the rows of an `available`
/// word — the canonical spelling of what 1.x said with `dialects`.
///
/// `docs/design/registry/dialect-and-package-registry-redesign.md` §6.2 gives every
/// scope that accepts `dialects` a second spelling, and the loader projects
/// the two onto exactly the same fields (`tcl_spectcl::loader::available`),
/// so this is a **rewording, not a re-meaning**: the round trip cannot tell
/// which word carried the set, and the round-trip gate proves it command by
/// command.
///
/// `None` when the set has no faithful spelling in the algebra, and the
/// caller keeps `dialects`. Two cases, both real:
///
/// - **A dialect with no provider.** `f5-iapps`, `expect`, `bpf`,
///   `f5-tmsh`, `f5-bigip`, and `spectcl` are dialect names the algebra's
///   closed provider list (`tcl`, `f5-irules`, `jim`, `package NAME`) does
///   not name. §6.1 keeps `dialects` loading forever precisely so a set the new
///   word cannot say is still sayable.
/// - **The empty set.** `dialects {}` is *declared unavailable*, and
///   `available` with no rows is an error rather than an empty set.
pub(super) fn availability_rows(members: &[&str]) -> Option<Vec<String>> {
    if members.is_empty() {
        return None;
    }
    let mut rows: Vec<String> = Vec::new();
    let versions: Vec<usize> = TCL_LADDER
        .iter()
        .enumerate()
        .filter(|(_, version)| members.contains(*version))
        .map(|(index, _)| index)
        .collect();
    // One row per contiguous run of releases, so a set the ladder covers to
    // its end is written open (`{tcl 8.6-}`) exactly as `tcl8.6+` is.
    let mut at = 0;
    while at < versions.len() {
        let mut end = at;
        while end + 1 < versions.len() && versions[end + 1] == versions[end] + 1 {
            end += 1;
        }
        let low = release_of(versions[at]);
        rows.push(if at == end {
            // A bare `X.Y` names that release line only — an open window
            // would claim a release the set does not hold.
            format!("tcl {low}")
        } else if versions[end] + 1 == TCL_LADDER.len() {
            // A run that reaches the newest release is written open, the way
            // `tclX.Y+` is: both track the ladder as it grows.
            format!("tcl {low}-")
        } else {
            // **The upper bound is exclusive**, as it is in every Tcl version
            // requirement (`package require`'s own `min-max` reads
            // `min <= v < max`), so a closed run names the release *after*
            // its last one: {8.4, 8.5, 8.6} is `tcl 8.4-9.0`.
            format!("tcl {low}-{}", release_of(versions[end] + 1))
        });
        at = end + 1;
    }
    for member in members {
        if TCL_LADDER.contains(member) {
            continue;
        }
        match *member {
            "f5-irules" => rows.push("f5-irules".to_owned()),
            // The exact inverse of `tcl spec upgrade`'s U2 `tk` →
            // `{package Tk}`, and the loader's `PACKAGE_DIALECT_SURFACES`
            // maps it straight back.
            "tk" => rows.push("package Tk".to_owned()),
            _ => return None,
        }
    }
    Some(rows)
}

/// The release word of a ladder index, without its `tcl` prefix.
fn release_of(index: usize) -> &'static str {
    TCL_LADDER
        .get(index)
        .and_then(|version| version.strip_prefix("tcl"))
        .unwrap_or_default()
}

/// The `available …` statement's words, or `None` to keep `dialects`.
pub(super) fn available_statement(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    let rows = availability_rows(&members)?;
    Some(
        rows.iter()
            .map(|row| format!("{{{row}}}"))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The `-available VALUE` flag's value, or `None` to keep `-dialects`.
///
/// One row is written bare (`-available {tcl 8.6-}`) and several are wrapped
/// into one list word, which is exactly how the loader's `from_flag` tells
/// the two apart.
pub(super) fn available_flag(members: &[&str]) -> Option<String> {
    let rows = availability_rows(members)?;
    Some(match rows.as_slice() {
        [only] => format!("{{{only}}}"),
        many => format!(
            "{{{}}}",
            many.iter()
                .map(|row| format!("{{{row}}}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    })
}

/// [`available_flag`] over a JSON member array.
fn available_flag_of(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    available_flag(&members)
}

/// One availability flag on a row: the 2.0 `-available` spelling when the
/// algebra carries the set, and the legacy `-dialects` when it does not.
pub(super) fn push_availability_flag(
    row: &mut Vec<String>,
    lost: &mut bool,
    value: &Value,
    algebra: bool,
) {
    if let Some(spelling) = algebra.then(|| available_flag_of(value)).flatten() {
        push_flag(row, lost, "-available", Some(spelling));
    } else {
        push_flag(row, lost, "-dialects", dialect_set_word(value));
    }
}

/// The five Tcl releases, in ladder order — the members the DSL's dialect-set
/// shorthands close over.
const TCL_LADDER: &[&str] = &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

/// A dialect set written the way the ports write one: with the `all-tcl`,
/// `tcl8.x`, and `tclX.Y+` shorthands rather than five version bits.
///
/// `README.md` adds the shorthands "because writing five version bits per
/// option is unreadable", and every port uses them — `{all-tcl f5-irules}`,
/// `tcl8.6+`. They are exact: `parse_dialects` unions the same bits either
/// way, so the round trip does not notice, but the file reads like a
/// hand-written one.
pub(super) fn dialect_set_word(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    dialect_words(&members)
}

/// [`dialect_set_word`] over member names already in hand.
pub(super) fn dialect_words(members: &[&str]) -> Option<String> {
    let versions: Vec<&str> = TCL_LADDER
        .iter()
        .copied()
        .filter(|version| members.contains(version))
        .collect();
    let mut words: Vec<String> = Vec::new();
    if versions.len() == TCL_LADDER.len() {
        words.push("all-tcl".to_owned());
    } else if versions == TCL_LADDER[..3] {
        words.push("tcl8.x".to_owned());
    } else if versions.len() > 1
        && TCL_LADDER.ends_with(versions.as_slice())
        && let Some(first) = versions.first()
    {
        words.push(format!("{first}+"));
    } else {
        words.extend(versions.iter().map(|version| (*version).to_owned()));
    }
    words.extend(
        members
            .iter()
            .filter(|member| !TCL_LADDER.contains(member))
            .map(|member| (*member).to_owned()),
    );
    list_word(&words)
}

/// A declaration's own name word — a command, subcommand, or option name.
///
/// Unlike prose these are identifiers in every shipped spec, so the braced
/// form always works; a pathological name that even braces cannot hold would
/// break the pack loudly rather than lose a field silently.
pub(super) fn name_word(text: &str) -> String {
    word(text).unwrap_or_else(|| format!("{{{text}}}"))
}

/// The spelling a `speclib` / `command` header uses for `name`.
///
/// Published for [`crate::store`], which rebuilds a header line byte for byte
/// when it puts an `-override` flag back on a re-rendered command — the flag
/// lives on the *declaration*, not on `CommandSpec`, so the renderer has
/// nothing to render it from and the store has to add it afterwards.
#[must_use]
pub fn header_word(name: &str) -> String {
    name_word(name)
}

/// Push one word onto a row, remembering when it had no faithful spelling.
pub(super) fn push_word(row: &mut Vec<String>, lost: &mut bool, spelling: Option<String>) {
    match spelling {
        Some(spelling) => row.push(spelling),
        None => *lost = true,
    }
}

/// Push `flag value` onto a row, remembering when the value had no faithful
/// spelling.
pub(super) fn push_flag(
    row: &mut Vec<String>,
    lost: &mut bool,
    flag: &str,
    spelling: Option<String>,
) {
    match spelling {
        Some(spelling) => {
            row.push(flag.to_owned());
            row.push(spelling);
        }
        None => *lost = true,
    }
}

/// Push the three lifecycle flags a gateable row's draft object carries, in
/// the order `README.md` documents them.
///
/// Every level the registry can gate spells its lifecycle the same way — the
/// option row's flags, unchanged, reused by `form`, `side_effect`,
/// `option_conflict`, `sub_subcommand` and a `values` table row.
pub(super) fn push_lifecycle_flags(row: &mut Vec<String>, lost: &mut bool, entry: &Value) {
    for (key, flag) in [
        ("introduced_version", "-introduced"),
        ("deprecated_version", "-deprecated"),
        ("retired_version", "-retired"),
    ] {
        if let Some(version) = entry[key].as_str() {
            push_flag(row, lost, flag, word(version));
        }
    }
}

/// Whether a row's draft object declares any lifecycle release at all.
pub(super) fn has_lifecycle(entry: &Value) -> bool {
    [
        "introduced_version",
        "deprecated_version",
        "retired_version",
    ]
    .iter()
    .any(|key| entry[key].is_string())
}

/// Write a row, or report the field when one of its words had no faithful
/// spelling.
pub(super) fn emit_row(out: &mut Out, key: &str, row: &[String], lost: bool, break_before: &str) {
    if lost {
        unwritable(out, key);
    } else {
        out.row(row, break_before);
    }
}

pub(super) fn as_array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}
