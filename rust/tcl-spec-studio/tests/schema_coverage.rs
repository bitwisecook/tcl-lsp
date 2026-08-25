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

//! The studio must be able to edit *every* field of a command spec.
//!
//! `CommandSpec` gains fields regularly — that is the registry's whole design
//! (`AGENTS.md`: "extend `CommandSpec` … rather than teaching a consumer about
//! the command by name"). A field the schema does not list is a field the
//! studio silently drops, so a draft seeded from a real command would render
//! back out having lost behaviour.
//!
//! This test reads the registry's own `spec.rs` and compares the fields listed
//! in the `CommandSpec::DEFAULT` / `SubCommand::DEFAULT` initialisers against
//! [`tcl_spec_studio::schema`]. Adding a field to the registry without adding
//! it to the schema fails here, naming the field.

use tcl_spec_studio::schema;

/// The registry's spec definitions, read at compile time so the test needs no
/// filesystem access and cannot go stale against a moved file.
const SPEC_SOURCE: &str = include_str!("../../tcl-registry/src/spec.rs");

/// Field names from a `pub const DEFAULT: Self = Self { … };` initialiser.
///
/// The initialiser is the authoritative field list: every field must appear in
/// it (that is what makes `..CommandSpec::DEFAULT` work), and it is one field
/// per line in `key: value,` form.
fn default_initialiser_fields(source: &str, after_marker: &str) -> Vec<String> {
    let start = source
        .find(after_marker)
        .unwrap_or_else(|| panic!("spec.rs no longer contains {after_marker}"));
    let body = &source[start..];
    let open = body
        .find("pub const DEFAULT: Self = Self {")
        .expect("the impl block declares a DEFAULT initialiser");
    let body = &body[open..];
    let end = body
        .find("\n    };")
        .expect("the initialiser is terminated");
    let mut fields = Vec::new();
    for line in body[..end].lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // Only the outermost `key: value,` lines are fields; nested struct
        // literals are indented further and never start a field here.
        let Some((key, _)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !key.is_empty()
        {
            fields.push(key.to_owned());
        }
    }
    assert!(
        fields.len() > 20,
        "parsed only {} fields from {after_marker} — has the initialiser's shape changed?",
        fields.len()
    );
    fields
}

/// Spec fields the studio deliberately edits as several separate keys.
///
/// `lifecycle` is one `Lifecycle` value on the spec but three independent
/// releases and one quick-fix hook in the form (#1210), so the schema names
/// them separately and the draft carries all four. Everything else is one
/// field, one key.
const EXPANDED_FIELDS: &[(&str, &[&str])] = &[(
    "lifecycle",
    &[
        "introduced_version",
        "deprecated_version",
        "retired_version",
        "deprecation_fix",
    ],
)];

fn expansion_of(field: &str) -> &'static [&'static str] {
    EXPANDED_FIELDS
        .iter()
        .find(|(name, _)| *name == field)
        .map_or(&[], |(_, keys)| *keys)
}

fn assert_schema_covers(spec_fields: &[String], schema_keys: &[&str], what: &str) {
    let missing: Vec<&String> = spec_fields
        .iter()
        .filter(|f| {
            let expanded = expansion_of(f);
            if expanded.is_empty() {
                !schema_keys.contains(&f.as_str())
            } else {
                !expanded.iter().all(|k| schema_keys.contains(k))
            }
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{what} fields are missing from the studio schema: {missing:?}\n\
         Add a FieldSchema entry for each in rust/tcl-spec-studio/src/schema.rs, \
         and seed it in src/draft.rs — otherwise the studio drops it silently.",
    );

    let extra: Vec<&&str> = schema_keys
        .iter()
        .filter(|k| {
            !spec_fields.iter().any(|f| f == *k)
                && !spec_fields.iter().any(|f| expansion_of(f).contains(k))
        })
        .collect();
    assert!(
        extra.is_empty(),
        "the studio schema lists {what} fields that no longer exist: {extra:?}\n\
         Remove them from rust/tcl-spec-studio/src/schema.rs.",
    );
}

#[test]
fn schema_covers_every_command_spec_field() {
    let spec_fields = default_initialiser_fields(SPEC_SOURCE, "impl CommandSpec {");
    let schema_keys: Vec<&str> = schema::COMMAND_FIELDS.iter().map(|f| f.key).collect();
    assert_schema_covers(&spec_fields, &schema_keys, "CommandSpec");
}

#[test]
fn schema_covers_every_subcommand_field() {
    let spec_fields = default_initialiser_fields(SPEC_SOURCE, "impl SubCommand {");
    let schema_keys: Vec<&str> = schema::SUBCOMMAND_FIELDS.iter().map(|f| f.key).collect();
    assert_schema_covers(&spec_fields, &schema_keys, "SubCommand");
}

#[test]
fn the_schema_and_the_default_draft_agree() {
    // The draft seeder writes one key per schema field; if the two ever drift
    // the form shows an editor with nothing behind it.
    let draft = tcl_spec_studio::draft::default_command_draft();
    for field in schema::COMMAND_FIELDS {
        assert!(
            draft.contains_key(field.key),
            "draft::from_command_spec does not seed `{}`",
            field.key
        );
    }
    let sub = tcl_spec_studio::draft::default_subcommand_draft();
    for field in schema::SUBCOMMAND_FIELDS {
        assert!(
            sub.contains_key(field.key),
            "draft::from_subcommand does not seed `{}`",
            field.key
        );
    }
}

#[test]
fn manufacturer_methods_has_a_typed_list_editor_and_empty_default() {
    let field = schema::command_field("manufacturer_methods")
        .expect("manufacturer_methods is part of the command schema");
    assert_eq!(field.kind, schema::FieldKind::ManufacturerMethods);
    assert_eq!(field.kind.to_json()["tag"], "manufacturerMethods");
    assert_eq!(
        tcl_spec_studio::draft::default_command_draft()["manufacturer_methods"],
        serde_json::json!([]),
    );
}

/// Every field kind the schema can emit must have a front-end editor.
///
/// The two halves are joined by a string — `FieldKind::tag` on the Rust side,
/// a key of the `makeEditors` map on the TypeScript side — so nothing but this
/// test makes them agree. When they disagree the field does not fail loudly:
/// the form renders the literal text "no editor for <tag>" in place of the
/// control, which is how `objectClass` shipped editable-in-principle and
/// uneditable in fact. Everything else about that field was already there —
/// schema entry, draft seeding, `SpecTcl` renderer, help text — which is
/// exactly why the gap survived review.
#[test]
fn every_field_kind_has_a_front_end_editor() {
    const EDITORS_TS: &str = include_str!("../web/src/editors.ts");

    // The map body — `const editors: Record<string, Editor> = { … }` — so a
    // tag named only in a comment or a helper cannot satisfy this.
    let start = EDITORS_TS
        .find("const editors: Record<string, Editor> = {")
        .expect("editors.ts declares the editor map");
    let body = &EDITORS_TS[start..];

    let mut missing: Vec<&str> = Vec::new();
    for field in schema::COMMAND_FIELDS
        .iter()
        .chain(schema::SUBCOMMAND_FIELDS)
    {
        let tag = field.kind.tag();
        // Keys are written bare (`subCommands: (…) => …`), one per entry.
        if !body.contains(&format!("\n    {tag}: (")) {
            missing.push(tag);
        }
    }
    missing.sort_unstable();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "field kinds with no editor in web/src/editors.ts: {missing:?} — \
         the form will render `no editor for <tag>` instead of a control"
    );
}

#[test]
fn structured_forms_expose_effect_refinements_only_through_the_opaque_rust_surface() {
    use tcl_spec_studio::render_spectcl::{GapKind, gap};
    use tcl_spec_studio::schema::FieldKind;

    for (key, fields) in [
        ("command_forms", schema::COMMAND_FIELDS),
        ("subcommand_forms", schema::SUBCOMMAND_FIELDS),
    ] {
        let field = fields
            .iter()
            .find(|field| field.key == key)
            .unwrap_or_else(|| panic!("{key} remains in the Studio schema"));
        let FieldKind::RustExpr { hint } = field.kind else {
            panic!("{key} must remain one opaque Rust expression")
        };
        let help = tcl_spec_studio::help::field_help(key).expect("form help");
        for refinement in [
            "literal_argument_prefix",
            "traits",
            "mutator",
            "side_effects",
        ] {
            assert!(
                hint.contains(refinement),
                "{key}'s Rust example must expose {refinement}"
            );
            assert!(
                help.contains(refinement),
                "{key}'s help must explain {refinement}"
            );
        }
        assert!(help.contains("opaque Rust expression"));
        assert!(help.contains("SpecTcl cannot"));
        assert_eq!(
            gap(key).map(|gap| gap.kind),
            Some(GapKind::Excluded),
            "the Studio must not claim a partial SpecTcl round trip for {key}"
        );
        let example = tcl_spec_studio::examples::field_example(key, field.label, field.group)
            .unwrap_or_else(|| panic!("{key} must have an annotated form-selection example"));
        let code = example["code"].as_str().expect("example code");
        let annotations = example["annotations"]
            .as_array()
            .expect("example annotations");
        assert!(
            annotations.len() >= 2,
            "{key}'s example must contrast query and mutation forms"
        );
        match key {
            "command_forms" => assert!(code.contains("get") && code.contains("set")),
            "subcommand_forms" => {
                assert!(code.contains("present") && code.contains("clear"));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn tk_geometry_is_a_structured_editor_not_an_opaque_rust_expression() {
    let field = schema::command_field("tk_geometry").expect("Tk geometry field");
    assert_eq!(field.kind, schema::FieldKind::TkGeometry);
    assert!(
        tcl_spec_studio::help::field_help("tk_geometry")
            .expect("Tk geometry help")
            .contains("Static preview")
    );
    for key in [
        "container_policy",
        "container_option",
        "direct_form",
        "placement_subcommand",
        "release_subcommands",
    ] {
        assert!(
            schema::NESTED_FIELDS.iter().any(|field| field.key == key),
            "Tk geometry editor must document nested field {key}"
        );
    }
}

/// A structural kind rebuilds its subtree on change; a plain input must not,
/// or the caret jumps to the end of the field mid-word. `objectClass` adds and
/// removes method rows, so it belongs to the first group.
#[test]
fn object_class_is_declared_structural() {
    const EDITORS_TS: &str = include_str!("../web/src/editors.ts");
    let start = EDITORS_TS
        .find("export const STRUCTURAL_KINDS = new Set([")
        .expect("editors.ts declares STRUCTURAL_KINDS");
    let end = start + EDITORS_TS[start..].find("]);").expect("the set is closed");
    assert!(
        EDITORS_TS[start..end].contains("\"objectClass\""),
        "objectClass adds and removes method rows, so it must rebuild on change"
    );
}
