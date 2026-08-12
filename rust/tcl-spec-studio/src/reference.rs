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

//! Render the field half of the `CommandSpec` reference manual.
//!
//! `docs/references/command-spec/fields.md` is generated from the same
//! schema, help, and catalogue tables that drive the spec studio, so the
//! manual cannot drift from the studio (and, through the studio's coverage
//! gates, cannot drift from the registry). `tests/reference_doc.rs` asserts
//! the on-disk file matches; regenerate with
//! `UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`.

use std::fmt::Write as _;

use crate::{catalogue, help, schema};

/// The generated `fields.md` — group-by-group field reference plus the
/// catalogue vocabularies, in the studio's own order.
#[must_use]
pub fn render_fields_doc() -> String {
    let mut out = String::from(
        "# CommandSpec fields\n\n\
         > Generated from the spec studio's schema — do not edit by hand.\n\
         > Regenerate with `UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio \
         --test reference_doc`.\n\n\
         Every field of `CommandSpec` and `SubCommand`, grouped the way the \
         [Spec Studio](https://bitwisecook.github.io/tcl-lsp/spec-studio/) \
         groups its form. The same text sits behind the studio's **?** \
         buttons. Impact tables — which fields drive which diagnostics, \
         optimisations, and editor features — live in \
         [README.md](README.md).\n",
    );

    for group in schema::GROUPS {
        let _ = write!(out, "\n## {group}\n");
        if let Some(text) = help::group_help(group) {
            let _ = write!(out, "\n{}\n", paragraphs(text));
        }
        for (field, levels) in fields_in_group(group) {
            let _ = write!(out, "\n### `{}` — {}\n", field.key, field.label);
            let _ = write!(out, "\n*{}* — {}\n", levels, field.doc);
            if let Some(text) = help::field_help(field.key) {
                let _ = write!(out, "\n{}\n", paragraphs(text));
            }
        }
    }

    out.push_str("\n## Vocabularies\n");
    out.push_str(
        "\nThe closed value sets the fields above draw from. The studio's \
         Reference tab searches the same lists.\n",
    );
    let cats = schema::catalogues();
    let cats = cats.as_object().expect("catalogues is an object");
    let mut ids: Vec<&String> = cats.keys().collect();
    ids.sort_by_key(|id| help::catalogue_help(id).map_or(id.as_str(), |(title, _)| title));
    for id in ids {
        let Some((title, intro)) = help::catalogue_help(id) else {
            continue;
        };
        let _ = write!(out, "\n### {title}\n\n{}\n\n", paragraphs(intro));
        out.push_str("| Value | Meaning |\n|---|---|\n");
        for variant in cats[id.as_str()].as_array().into_iter().flatten() {
            let key = variant["key"].as_str().unwrap_or_default();
            let doc = variant["doc"].as_str().unwrap_or_default();
            let _ = writeln!(out, "| `{key}` | {doc} |");
        }
    }
    out
}

/// The fields of `group`, deduplicated across the two schemas, each tagged
/// with the level(s) it exists at.
fn fields_in_group(group: &str) -> Vec<(&'static schema::FieldSchema, &'static str)> {
    let mut rows = Vec::new();
    for field in schema::COMMAND_FIELDS {
        if field.group != group {
            continue;
        }
        let level = if schema::subcommand_field(field.key).is_some() {
            "command and subcommand"
        } else {
            "command only"
        };
        rows.push((field, level));
    }
    for field in schema::SUBCOMMAND_FIELDS {
        if field.group == group && schema::command_field(field.key).is_none() {
            rows.push((field, "subcommand only"));
        }
    }
    rows
}

/// Blank-line-separated help paragraphs, passed through verbatim.
fn paragraphs(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rendered_doc_covers_every_group_and_field() {
        let doc = render_fields_doc();
        for group in schema::GROUPS {
            assert!(doc.contains(&format!("\n## {group}\n")), "missing {group}");
        }
        for field in schema::COMMAND_FIELDS.iter().chain(schema::SUBCOMMAND_FIELDS) {
            assert!(
                doc.contains(&format!("### `{}`", field.key)),
                "missing field {}",
                field.key
            );
        }
        for entry in catalogue::TRAITS {
            assert!(doc.contains(entry.key), "missing trait {}", entry.key);
        }
    }
}
