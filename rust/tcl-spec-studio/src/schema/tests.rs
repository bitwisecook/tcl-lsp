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

use super::*;

#[test]
fn field_keys_are_unique_and_documented() {
    for list in [COMMAND_FIELDS, SUBCOMMAND_FIELDS] {
        let mut seen: Vec<&str> = Vec::new();
        for field in list {
            assert!(!seen.contains(&field.key), "duplicate field {}", field.key);
            assert!(!field.label.is_empty(), "{} has no label", field.key);
            assert!(!field.doc.is_empty(), "{} has no help text", field.key);
            assert!(
                GROUPS.contains(&field.group),
                "{} is in unknown group {}",
                field.key,
                field.group
            );
            seen.push(field.key);
        }
    }
}

#[test]
fn every_nested_property_names_the_form_row_it_is_edited_in() {
    for nested in NESTED_FIELDS {
        assert!(
            command_field(nested.field).is_some() || subcommand_field(nested.field).is_some(),
            "{} is edited under {}, which is not a spec field",
            nested.key,
            nested.field
        );
        assert_eq!(
            to_json()["nestedFields"]
                .as_array()
                .and_then(|rows| rows.iter().find(|row| row["key"] == nested.key))
                .map(|row| row["field"].clone()),
            Some(json!(nested.field)),
            "{} must carry its owning row on the wire",
            nested.key
        );
    }
}

#[test]
fn every_named_catalogue_exists() {
    let cats = catalogues();
    let cats = cats.as_object().expect("catalogues is an object");
    for list in [COMMAND_FIELDS, SUBCOMMAND_FIELDS] {
        for field in list {
            if let FieldKind::Enum { catalogue, .. }
            | FieldKind::FlagSet { catalogue, .. }
            | FieldKind::EnumList { catalogue } = field.kind
            {
                assert!(
                    cats.contains_key(catalogue),
                    "{} names missing catalogue {catalogue}",
                    field.key
                );
            }
        }
    }
}

#[test]
fn schema_json_carries_both_tables() {
    let schema = to_json();
    assert_eq!(
        schema["command"].as_array().map(Vec::len),
        Some(COMMAND_FIELDS.len())
    );
    assert_eq!(
        schema["subcommand"].as_array().map(Vec::len),
        Some(SUBCOMMAND_FIELDS.len())
    );
    assert_eq!(schema["command"][0]["key"], "name");
    assert_eq!(
        schema["nestedFields"].as_array().map(Vec::len),
        Some(NESTED_FIELDS.len())
    );
    assert_eq!(schema["nestedFields"][0]["key"], "taints_var_write");
    assert!(schema["nestedFields"][0]["help"].as_str().is_some());
    assert!(
        schema["nestedFields"][0]["example"]["code"]
            .as_str()
            .is_some()
    );
}
