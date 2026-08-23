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

//! Small, annotated Tcl examples for the studio's help surfaces.
//!
//! Prose explains a registry fact, but an author also needs to see *where* it
//! attaches to a call.  This module gives every field, group, catalogue, and
//! catalogue variant a compact Tcl snippet plus one or more focused spans.
//! The browser draws a bracket and arrow beneath each span.  The editor's
//! **?** panels and the searchable Reference tab consume the same JSON, so an
//! example cannot explain one surface differently from the other.

use serde_json::{Value, json};

use tcl_registry::documentation::DocumentationExample;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::taint::TaintColourAtom;
use tcl_registry::traits::Trait;

/// One span in a source line that the browser annotates.
#[derive(Debug, Clone, Copy)]
struct Focus {
    /// Zero-based line in [`Example::code`].
    line: usize,
    /// Exact text to bracket. Tests verify that it occurs on `line`.
    needle: &'static str,
    /// What the bracket means before the field/variant name is prefixed.
    note: &'static str,
}

/// A short Tcl snippet and the spans to point at.
#[derive(Debug, Clone, Copy)]
struct Example {
    code: &'static str,
    focuses: &'static [Focus],
}

const fn focus(line: usize, needle: &'static str, note: &'static str) -> Focus {
    Focus { line, needle, note }
}

const IDENTITY: Example = Example {
    code: "mycommand value",
    focuses: &[focus(0, "mycommand", "describes the command word")],
};
const AVAILABILITY: Example = Example {
    code: "mycommand value",
    focuses: &[focus(
        0,
        "mycommand",
        "decides whether this command exists here",
    )],
};
const ARGUMENTS: Example = Example {
    code: "mycommand first second",
    focuses: &[focus(0, "first", "describes this argument position")],
};
const TYPES: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "describes values flowing through this call",
    )],
};
const SUBCOMMANDS: Example = Example {
    code: "mycommand action value",
    focuses: &[focus(0, "action", "selects the subcommand specification")],
};
const DOCUMENTATION: Example = Example {
    code: "mycommand -mode fast value",
    focuses: &[focus(
        0,
        "mycommand -mode fast value",
        "is the invocation readers see documented",
    )],
};
const OPTIONS: Example = Example {
    code: "mycommand -mode fast value",
    focuses: &[focus(
        0,
        "-mode fast",
        "describes the option and its value words",
    )],
};
const BEHAVIOUR: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "describes the behaviour of the whole invocation",
    )],
};
const EFFECTS: Example = Example {
    code: "set result [mycommand $state]",
    focuses: &[focus(
        0,
        "[mycommand $state]",
        "records state read or changed by this invocation",
    )],
};
const HOOKS: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "selects special handling for this invocation",
    )],
};
const TAINT: Example = Example {
    code: "set safe [mycommand $untrusted]\nputs $safe",
    focuses: &[
        focus(
            0,
            "[mycommand $untrusted]",
            "colours or checks data at this call",
        ),
        focus(1, "$safe", "the resulting proof follows this value"),
    ],
};
const DEPRECATION: Example = Example {
    code: "oldcommand value",
    focuses: &[focus(
        0,
        "oldcommand",
        "reports or translates this deprecated invocation",
    )],
};
const ADVANCED: Example = Example {
    code: "mycommand $value",
    focuses: &[focus(
        0,
        "mycommand $value",
        "applies custom registry behaviour to this call",
    )],
};

const FIELD_TRAITS: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "traits describe the whole invocation",
    )],
};
const FIELD_ARG_ROLES: Example = Example {
    code: "myloop item $items {\n    puts $item\n}",
    focuses: &[
        focus(0, "item", "a variable-name role applies to this word"),
        focus(0, "{", "a body role starts at this script argument"),
    ],
};
const FIELD_RETURN_TYPE: Example = Example {
    code: "set count [mycommand $items]",
    focuses: &[focus(
        0,
        "[mycommand $items]",
        "types the value returned by this call",
    )],
};
const FIELD_TAINT_SOURCE: Example = Example {
    code: "set user_input [mycommand]",
    focuses: &[focus(
        0,
        "[mycommand]",
        "marks this returned value as untrusted",
    )],
};
const FIELD_TAINT_TRANSFORM: Example = Example {
    code: "set safe [mycommand $user_input]",
    focuses: &[focus(
        0,
        "[mycommand $user_input]",
        "adds a sanitising proof to this result",
    )],
};
const FIELD_TAINT_SINK: Example = Example {
    code: "mycommand $user_input",
    focuses: &[focus(
        0,
        "$user_input",
        "checks taint arriving in this argument",
    )],
};
const FIELD_COMMAND_PREFIX: Example = Example {
    code: "mycommand callback\nproc callback {value status} { ... }",
    focuses: &[
        focus(0, "callback", "is the callback command prefix"),
        focus(
            1,
            "{value status}",
            "must accept the arguments appended at invocation time",
        ),
    ],
};

/// The example inherited by every field in a form group.
fn group_template(group: &str) -> Option<Example> {
    match group {
        "Identity" => Some(IDENTITY),
        "Availability" => Some(AVAILABILITY),
        "Arity and arguments" => Some(ARGUMENTS),
        "Types" => Some(TYPES),
        "Subcommands" => Some(SUBCOMMANDS),
        "Documentation" => Some(DOCUMENTATION),
        "Options and values" => Some(OPTIONS),
        "Behaviour" => Some(BEHAVIOUR),
        "Side effects" => Some(EFFECTS),
        "Compiler hooks" => Some(HOOKS),
        "Taint and security" => Some(TAINT),
        "Deprecation and translation" => Some(DEPRECATION),
        "Advanced" => Some(ADVANCED),
        _ => None,
    }
}

/// A more precise example where a group-level example would hide the useful
/// attachment point.
fn field_template(key: &str, group: &str) -> Option<Example> {
    match key {
        "traits" => Some(FIELD_TRAITS),
        "arg_roles" | "arg_role_resolver" | "repeated_args" => Some(FIELD_ARG_ROLES),
        "return_type" | "return_elements" | "var_write_typing" => Some(FIELD_RETURN_TYPE),
        "command_prefixes" | "command_prefix_resolver" | "start_cmd_arg" => {
            Some(FIELD_COMMAND_PREFIX)
        }
        "taint_source" => Some(FIELD_TAINT_SOURCE),
        "taint_transform" | "taint_double_encode_colour" => Some(FIELD_TAINT_TRANSFORM),
        "taint_output_sink"
        | "taint_output_sink_subcommands"
        | "taint_log_sink"
        | "taint_network_sink_args"
        | "taint_code_sink_args"
        | "taint_interp_eval_subcommands"
        | "taint_sink_safe_colour"
        | "taint_sink_gate" => Some(FIELD_TAINT_SINK),
        _ => group_template(group),
    }
}

const CATALOGUE_ARG_ROLE: Example = Example {
    code: "mycommand ARGUMENT",
    focuses: &[focus(0, "ARGUMENT", "classifies this argument word")],
};
const CATALOGUE_TYPE: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(0, "$value", "describes the value at this position")],
};
const CATALOGUE_PRESENTATION: Example = Example {
    code: "mycommand {script body}",
    focuses: &[focus(
        0,
        "{script body}",
        "controls how this script is laid out",
    )],
};
const CATALOGUE_EFFECT: Example = Example {
    code: "mycommand $state",
    focuses: &[focus(
        0,
        "mycommand $state",
        "classifies the effect of this invocation",
    )],
};
const CATALOGUE_HOOK: Example = Example {
    code: "set result [mycommand $value]",
    focuses: &[focus(
        0,
        "[mycommand $value]",
        "selects special handling for this call",
    )],
};
const CATALOGUE_TAINT: Example = Example {
    code: "set checked [validate $user_input]\nmy_sink $checked",
    focuses: &[
        focus(0, "$user_input", "starts as data that may be untrusted"),
        focus(1, "$checked", "carries the selected proof into this sink"),
    ],
};
const CATALOGUE_DIALECT: Example = Example {
    code: "mycommand value",
    focuses: &[focus(
        0,
        "mycommand",
        "is available in the selected language surface",
    )],
};
const CATALOGUE_OPTION: Example = Example {
    code: "mycommand -option VALUE",
    focuses: &[focus(
        0,
        "-option VALUE",
        "controls these option value words",
    )],
};
const CATALOGUE_PREFIX: Example = Example {
    code: "mycommand callback\nproc callback {appended args} { ... }",
    focuses: &[focus(
        0,
        "callback",
        "receives the selected appended-argument shape",
    )],
};

fn catalogue_template(id: &str) -> Option<Example> {
    match id {
        "argRole" => Some(CATALOGUE_ARG_ROLE),
        "tclType" | "storageType" | "byteArrayEffect" => Some(CATALOGUE_TYPE),
        "bodyKind" | "argPresentation" => Some(CATALOGUE_PRESENTATION),
        "commandTableEffect" | "definedSymbolKind" | "sideEffectTarget" | "connectionSide"
        | "formKind" => Some(CATALOGUE_EFFECT),
        "patternType" | "formatType" | "defaultFormFirstWord" | "prefixMatching" => {
            Some(CATALOGUE_ARG_ROLE)
        }
        "loweringHook" | "codegenHook" | "inlineCodegenHook" | "analyserHook" => {
            Some(CATALOGUE_HOOK)
        }
        "traits" => Some(FIELD_TRAITS),
        "taintColour" => Some(CATALOGUE_TAINT),
        "dialects" => Some(CATALOGUE_DIALECT),
        "appendedArity" => Some(CATALOGUE_PREFIX),
        "optionArity" => Some(CATALOGUE_OPTION),
        _ => None,
    }
}

fn example_json(example: Example, subject: &str) -> Value {
    let annotations: Vec<Value> = example
        .focuses
        .iter()
        .map(|item| {
            json!({
                "line": item.line,
                "needle": item.needle,
                "label": format!("{subject} — {}", item.note),
            })
        })
        .collect();
    json!({ "code": example.code, "annotations": annotations })
}

fn registry_example_json(example: DocumentationExample, subject: &str) -> Value {
    let annotations: Vec<Value> = example
        .annotations
        .iter()
        .map(|item| {
            json!({
                "line": item.line,
                "needle": item.needle,
                "label": format!("{subject} — {}", item.label),
            })
        })
        .collect();
    json!({ "code": example.code, "annotations": annotations })
}

/// Annotated example for a field's **?** panel and Reference row.
#[must_use]
pub fn field_example(key: &str, label: &str, group: &str) -> Option<Value> {
    field_template(key, group).map(|example| example_json(example, label))
}

/// Annotated example for a group heading's **?** panel.
#[must_use]
pub fn group_example(group: &str) -> Option<Value> {
    group_template(group).map(|example| example_json(example, group))
}

/// Annotated example introducing one picker catalogue.
#[must_use]
pub fn catalogue_example(id: &str, title: &str) -> Option<Value> {
    catalogue_template(id).map(|example| example_json(example, title))
}

/// Annotated example for one catalogue variant. Trait variants get a
/// category-specific snippet; other catalogues inherit their catalogue's
/// attachment point.
#[must_use]
pub fn variant_example(id: &str, key: &str, doc: &str) -> Option<Value> {
    let subject = format!("{key}: {doc}");
    match id {
        "traits" => Trait::from_name(key)
            .map(Trait::example)
            .map(|example| registry_example_json(example, &subject)),
        "taintColour" => TaintColourAtom::from_name(key)
            .map(TaintColourAtom::example)
            .map(|example| registry_example_json(example, &subject)),
        "sideEffectTarget" => SideEffectTarget::from_name(key)
            .map(SideEffectTarget::example)
            .map(|example| registry_example_json(example, &subject)),
        _ => catalogue_template(id).map(|example| example_json(example, &subject)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, schema};

    fn validation_errors(example: &Value, owner: &str) -> Vec<String> {
        let mut errors = Vec::new();
        let code = example["code"].as_str().expect("example code");
        let lines: Vec<&str> = code.lines().collect();
        let annotations = example["annotations"].as_array().expect("annotations");
        if annotations.is_empty() {
            errors.push(format!("{owner} has no arrow annotations"));
        }
        for annotation in annotations {
            let line = usize::try_from(annotation["line"].as_u64().expect("line"))
                .expect("line fits usize");
            let needle = annotation["needle"].as_str().expect("needle");
            if !lines.get(line).is_some_and(|text| text.contains(needle)) {
                errors.push(format!(
                    "{owner}: line {line} does not contain {needle:?} in {code:?}"
                ));
            }
            if annotation["label"].as_str().is_none_or(str::is_empty) {
                errors.push(format!("{owner} has an empty arrow label"));
            }
        }
        errors
    }

    fn assert_valid(example: &Value, owner: &str) {
        let errors = validation_errors(example, owner);
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn every_group_and_field_has_a_valid_example() {
        for group in schema::GROUPS {
            let example = group_example(group).unwrap_or_else(|| panic!("no example for {group}"));
            assert_valid(&example, group);
        }
        for field in schema::COMMAND_FIELDS
            .iter()
            .chain(schema::SUBCOMMAND_FIELDS)
        {
            let example = field_example(field.key, field.label, field.group)
                .unwrap_or_else(|| panic!("no example for {}", field.key));
            assert_valid(&example, field.key);
        }
    }

    #[test]
    fn every_catalogue_and_variant_has_a_valid_example() {
        let mut errors = Vec::new();
        for (id, title, _) in crate::help::CATALOGUE_HELP {
            let example = catalogue_example(id, title)
                .unwrap_or_else(|| panic!("no catalogue example for {id}"));
            errors.extend(validation_errors(&example, id));
        }
        let catalogues = schema::catalogues();
        for (id, variants) in catalogues.as_object().expect("catalogues") {
            for variant in variants.as_array().expect("variants") {
                let key = variant["key"].as_str().expect("key");
                let doc = variant["doc"].as_str().expect("doc");
                let example = variant_example(id, key, doc)
                    .unwrap_or_else(|| panic!("no variant example for {id}/{key}"));
                errors.extend(validation_errors(&example, &format!("{id}/{key}")));
            }
        }
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn every_trait_is_in_an_organised_group() {
        for entry in catalogue::TRAITS.iter() {
            let item = Trait::from_name(entry.key).expect("catalogue trait is registered");
            assert!(!item.summary().is_empty(), "{} has no summary", entry.key);
            assert!(
                !item.category().label().is_empty(),
                "{} has no group",
                entry.key
            );
        }
    }
}
