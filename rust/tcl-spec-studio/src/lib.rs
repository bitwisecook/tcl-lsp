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

//! The command-registry spec studio.
//!
//! Everything behind the studio's web UI, with no browser or WASM in the
//! picture: browse the live command registry, edit every field of a
//! `CommandSpec`, and render the result as a registry `.rs` source file or as
//! a Tcl dialect stub. The `tcl-spec-studio-wasm` crate is a thin
//! `wasm-bindgen` facade over the functions here, so the same code backs the
//! browser app and any native caller.
//!
//! The pieces:
//!
//! - [`schema`] — one entry per `CommandSpec` / `SubCommand` field, driving
//!   the form, the draft model, and the renderer from a single table.
//! - [`catalogue`] — the registry's enum and bitflag vocabularies, with
//!   compile-time witnesses that they stay complete.
//! - [`help`] — the long-form, Tcl-developer-facing help behind the form's
//!   **?** buttons and the Reference tab, with tests that keep it covering
//!   every field, group, and catalogue.
//! - [`coverage`] — the same idea one level down: exhaustive *destructurings*
//!   of the `CommandSpec` family, so a new registry **field** breaks the build
//!   until it is surfaced in the studio (or explicitly excluded).
//! - [`draft`] — the JSON draft model, and seeding a draft from a live spec.
//! - [`render_rs`] — draft → registry `.rs` source, copyright banner included.
//! - [`render_stub`] — draft → `# tcl-lsp: stub` block or `.tcl.stubs` file.
//! - [`render_spectcl`] — draft → `.tclspec` spec pack, the loader's inverse.
//! - [`infer`] — Tcl package sources → draft specs, via the real analyser.
//! - [`store`] — the studio's **models**, kept away from every UI: the
//!   immutable built-ins, the DSL-text-backed pack store, and the one
//!   resolution facade that merges them under the shipped collision policy.
//! - [`spectcl`] — `.tclspec` spec packs → live `CommandSpec`s, read from the
//!   CST and never executed.
//!
//! Two of those are re-exports rather than modules of this crate. [`spectcl`]
//! and [`catalogue`] were written here, where the DSL was designed, and now
//! live in `tcl-spectcl` — the LSP server loads packs at workspace init and
//! cannot reasonably depend on a draft model, a `.rs` renderer, and a schema
//! coverage gate to do it. The studio's surface is unchanged: same paths, same
//! types, and the equivalence gate in `tests/spectcl_ports.rs` still tests the
//! very loader the server runs.

pub mod coverage;
pub mod draft;
pub mod help;
pub mod infer;
pub mod reference;
pub mod render_rs;
pub mod render_spectcl;
pub mod render_stub;
pub mod schema;
pub mod store;

pub use tcl_spectcl::catalogue;
pub use tcl_spectcl::loader as spectcl;

use serde_json::{Value, json};
use tcl_registry::cache::registry_for_dialect;

/// Dialects the studio offers, as `(registry name, label)`.
///
/// These are the profile names [`registry_for_dialect`] resolves, not the
/// primitive `DialectSet` bits — a profile is what decides which commands are
/// actually visible.
pub const BROWSABLE_DIALECTS: &[(&str, &str)] = &[
    ("tcl9.1", "Tcl 9.1"),
    ("tcl9.0", "Tcl 9.0"),
    ("tcl8.6", "Tcl 8.6"),
    ("tcl8.5", "Tcl 8.5"),
    ("tcl8.4", "Tcl 8.4"),
    ("f5-irules", "F5 iRules"),
    ("f5-iapps", "F5 iApps"),
    ("tk", "Tk"),
    ("expect", "Expect"),
];

/// The command names available in `dialect`, sorted.
#[must_use]
pub fn command_names(dialect: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = registry_for_dialect(dialect).command_names().collect();
    names.sort_unstable();
    names
}

/// An index of `dialect`'s commands for the studio's browser: the name, a
/// short summary, and whether the spec declares subcommands or options.
#[must_use]
pub fn command_index(dialect: &str) -> Value {
    let registry = registry_for_dialect(dialect);
    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    let entries: Vec<Value> = names
        .into_iter()
        .filter_map(|name| {
            let spec = registry.get(name)?;
            Some(json!({
                "name": name,
                "summary": spec.hover.map_or("", |h| h.summary),
                "synopsis": spec.primary_synopsis().unwrap_or(""),
                "subcommands": spec.subcommands.len(),
                "options": spec.options.len(),
                "deprecated": spec.deprecated_replacement.is_some(),
            }))
        })
        .collect();
    json!({ "dialect": dialect, "commands": entries })
}

/// Seed a draft from the live registry's spec for `name` under `dialect`.
///
/// Returns `None` when the dialect does not have that command.
#[must_use]
pub fn load_command(name: &str, dialect: &str) -> Option<Value> {
    let spec = registry_for_dialect(dialect).get(name)?;
    let mut d = draft::from_command_spec(spec);
    d.insert(draft::SOURCE_DIALECT_KEY.to_owned(), json!(dialect));
    Some(Value::Object(d))
}

/// The dialect list the studio's picker shows.
#[must_use]
pub fn dialects() -> Value {
    Value::Array(
        BROWSABLE_DIALECTS
            .iter()
            .map(|(name, label)| json!({ "name": name, "label": label }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_index_is_populated_and_sorted() {
        let index = command_index("tcl9.0");
        let commands = index["commands"].as_array().expect("commands array");
        assert!(
            commands.len() > 100,
            "expected the full Tcl 9.0 command surface, got {}",
            commands.len()
        );
        let names: Vec<&str> = commands.iter().filter_map(|c| c["name"].as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert!(names.contains(&"lappend"));
    }

    #[test]
    fn loading_a_command_seeds_every_schema_field() {
        let loaded = load_command("lappend", "tcl9.0").expect("lappend exists in Tcl 9.0");
        for field in schema::COMMAND_FIELDS {
            assert!(
                loaded.get(field.key).is_some(),
                "loaded draft is missing {}",
                field.key
            );
        }
        assert_eq!(loaded["name"], json!("lappend"));
        assert_eq!(loaded[draft::SOURCE_DIALECT_KEY], json!("tcl9.0"));
    }

    #[test]
    fn an_unknown_command_loads_as_none() {
        assert!(load_command("definitely::not::a::command", "tcl9.0").is_none());
    }

    #[test]
    fn every_browsable_dialect_resolves_to_a_populated_registry() {
        for (name, _) in BROWSABLE_DIALECTS {
            assert!(
                !command_names(name).is_empty(),
                "{name} resolved to an empty registry"
            );
        }
    }
}
