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
//! - [`relations`] — which settings are read together, so a help surface
//!   can link a field to the rest of its cluster rather than describing it
//!   as though it stood alone.
//! - [`draft`] — the JSON draft model, and seeding a draft from a live spec.
//! - [`render_rs`] — draft → registry `.rs` source, copyright banner included.
//! - [`render_stub`] — draft → `# tcl-lsp: stub` block or `.tcl.stubs` file.
//! - [`render_spectcl`] — draft → `.tclspec` spec pack, the loader's inverse.
//! - [`format_pack`] — `.tclspec` source → the shared Tcl formatter under the
//!   Tcl 9 + `SpecTcl` profile used by the studio's code editors.
//! - [`infer`] — Tcl package sources → draft specs, via the real analyser.
//! - [`versions`] — the same, over several *releases* of one package, so the
//!   lifecycle fields carry a range the releases actually witness rather than
//!   whatever version the newest snapshot happens to declare.
//! - [`corpus`] — the shape heuristics the importer layers on top: option
//!   tables, mode-word subcommands, closed value sets and callback arity, read
//!   deterministically out of a proc's body with an evidence line each.
//! - [`sample`] — the Test tab's engine: a sample of Tcl analysed with the pack
//!   installed, plus a per-word explanation of which spec field produced it.
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

pub mod corpus;
pub mod coverage;
pub mod draft;
pub mod environment;
pub mod examples;
pub mod help;
pub mod infer;
pub mod reference;
pub mod relations;
pub mod render_rs;
pub mod render_spectcl;
pub mod render_stub;
pub mod sample;
pub mod schema;
pub mod store;
pub mod versions;

pub use tcl_spectcl::catalogue;
pub use tcl_spectcl::loader as spectcl;

use serde_json::{Value, json};
use tcl_dialect::DialectProfile;
use tcl_lsp_core::formatting::{FormatterConfig, format_tcl};

/// Dialects the studio offers, as `(registry name, label)`, in catalogue order.
///
/// These are the profile names [`environment::store_for_dialect`] resolves, not the
/// primitive surface rows — a profile is what decides which commands are
/// actually visible. `tk` is therefore not here: it is a library pin rather
/// than a profile, so it resolves to the permissive fallback, and the Tk
/// commands are already browsable under every Tcl-version profile.
pub fn browsable_dialects() -> impl Iterator<Item = (&'static str, &'static str)> {
    DialectProfile::all()
        .iter()
        .map(|profile| (profile.name, profile.display_name))
}

/// Format a `SpecTcl` pack with the same Tcl formatter used by the LSP and CLI.
///
/// A `.tclspec` file is Tcl 9 source extended by the `SpecTcl` command layer,
/// so the formatter grammar and registry come from that resolved profile.
#[must_use]
pub fn format_pack(source: &str) -> String {
    let config = FormatterConfig::for_dialect("spectcl");
    format_tcl(source, &config, environment::store_for_dialect("spectcl"))
}

/// The command names available in `dialect`, sorted.
#[must_use]
pub fn command_names(dialect: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = environment::store_for_dialect(dialect)
        .command_names()
        .collect();
    names.sort_unstable();
    names
}

/// An index of `dialect`'s commands for the studio's browser: the name, a
/// short summary, and whether the spec declares subcommands or options.
#[must_use]
pub fn command_index(dialect: &str) -> Value {
    let registry = environment::store_for_dialect(dialect);
    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    let entries: Vec<Value> = names
        .into_iter()
        .filter_map(|name| {
            let spec = registry.get(name)?;
            Some(json!({
                "name": name,
                "summary": spec.hover.map_or("", |h| h.summary),
                // A catalogue listing shows what the spec declares, unfiltered:
                // there is no document, and so no resolved package-version floor.
                "synopsis": spec.primary_synopsis(None).unwrap_or(""),
                "subcommands": spec.subcommands.len(),
                "options": spec.options.len(),
                "deprecated": spec.deprecated_replacement.is_some(),
                // Provenance: the `commands/<pack>/` module this very spec is
                // declared in. Asked with the resolved spec, not the name, so
                // a dialect that re-declares a core command is filed under the
                // declaration it actually registered.
                "pack": tcl_registry::registry::spec_pack_of(spec),
                // The other packs that declare the same name, when there are
                // any — `close` is authored in `tcl`, `expect` and `irules`.
                "also_in": tcl_registry::registry::spec_packs_of(name)
                    .iter()
                    .filter(|id| Some(**id) != tcl_registry::registry::spec_pack_of(spec))
                    .collect::<Vec<_>>(),
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
    let spec = environment::store_for_dialect(dialect).get(name)?;
    let mut d = draft::from_command_spec(spec);
    d.insert(draft::SOURCE_DIALECT_KEY.to_owned(), json!(dialect));
    Some(Value::Object(d))
}

/// The authoring packs `dialect` actually browses, with the commands each
/// contributes — the studio's top-level navigation.
///
/// Only packs that reach this dialect are listed, so the Tcl 8.4 picker does
/// not offer an empty **F5 iRules** heading. Ordering is
/// [`SPEC_PACKS`](tcl_registry::commands::SPEC_PACKS)': core language, then
/// the libraries that layer on it, then the vendor and authoring surfaces.
#[must_use]
pub fn pack_catalogue(dialect: &str) -> Value {
    let registry = environment::store_for_dialect(dialect);
    let mut counts: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    for name in registry.command_names() {
        if let Some(spec) = registry.get(name)
            && let Some(pack) = tcl_registry::registry::spec_pack_of(spec)
        {
            *counts.entry(pack).or_default() += 1;
        }
    }
    let packs: Vec<Value> = tcl_registry::commands::SPEC_PACKS
        .iter()
        .filter_map(|pack| {
            let commands = counts.get(pack.id).copied()?;
            Some(json!({
                "id": pack.id,
                "label": pack.label,
                "blurb": pack.blurb,
                "commands": commands,
                "path": format!("rust/tcl-registry/src/commands/{}", pack.id),
            }))
        })
        .collect();
    json!({ "dialect": dialect, "packs": packs })
}

/// The dialect list the studio's picker shows.
#[must_use]
pub fn dialects() -> Value {
    Value::Array(
        browsable_dialects()
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
    fn every_indexed_command_names_the_pack_that_declares_it() {
        for dialect in ["tcl9.0", "f5-irules", "spectcl"] {
            let index = command_index(dialect);
            for entry in index["commands"].as_array().expect("commands array") {
                assert!(
                    entry["pack"].is_string(),
                    "{} has no pack in {dialect}",
                    entry["name"]
                );
            }
        }
    }

    #[test]
    fn the_pack_catalogue_covers_every_indexed_command() {
        for dialect in ["tcl8.4", "tcl9.0", "f5-irules"] {
            let catalogue = pack_catalogue(dialect);
            let packs = catalogue["packs"].as_array().expect("packs array");
            assert!(!packs.is_empty(), "{dialect} browses no packs");
            let total: usize = packs
                .iter()
                .map(|p| {
                    assert!(
                        p["commands"].as_u64().unwrap_or(0) > 0,
                        "{dialect} lists an empty pack {}",
                        p["id"]
                    );
                    usize::try_from(p["commands"].as_u64().unwrap_or(0)).unwrap_or(0)
                })
                .sum();
            let indexed = command_index(dialect)["commands"]
                .as_array()
                .expect("commands array")
                .len();
            assert_eq!(total, indexed, "{dialect} pack counts miss commands");
        }
    }

    /// iRules declares its own `close`, so browsing iRules files it under
    /// `irules` while Tcl 9.0 still files the core one under `tcl`.
    #[test]
    fn a_redeclared_command_follows_the_dialect_that_registered_it() {
        let pack_of = |dialect: &str, name: &str| {
            command_index(dialect)["commands"]
                .as_array()
                .expect("commands array")
                .iter()
                .find(|entry| entry["name"] == name)
                .map(|entry| entry["pack"].as_str().unwrap_or("").to_owned())
        };
        assert_eq!(pack_of("tcl9.0", "close").as_deref(), Some("tcl"));
        assert_eq!(pack_of("f5-irules", "close").as_deref(), Some("irules"));
    }

    #[test]
    fn an_unknown_command_loads_as_none() {
        assert!(load_command("definitely::not::a::command", "tcl9.0").is_none());
    }

    #[test]
    fn every_browsable_dialect_resolves_to_a_populated_registry() {
        for (name, _) in browsable_dialects() {
            assert!(
                !command_names(name).is_empty(),
                "{name} resolved to an empty registry"
            );
        }
    }

    #[test]
    fn pack_formatting_uses_the_shared_spectcl_profile() {
        let source = "speclib demo 1 {\ncommand foo {\nsynopsis {foo value}\narity 1\n}\n}";
        assert_eq!(
            format_pack(source),
            "speclib demo 1 {\n    command foo {\n        synopsis {foo value}\n        arity 1\n    }\n}\n"
        );
    }
}
