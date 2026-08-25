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
const FIELD_TK_GEOMETRY: Example = Example {
    code: "frame .panel\nlabel .name -text Name\npack .name -in .panel\npack configure .name -padx 8\npack forget .name",
    focuses: &[
        focus(2, "pack .name", "the direct form places the widget"),
        focus(2, "-in .panel", "selects the effective container"),
        focus(
            3,
            "configure .name",
            "the placement subcommand reconfigures it",
        ),
        focus(4, "forget .name", "a release subcommand stops managing it"),
    ],
};
const FIELD_TAINTS_VAR_WRITE: Example = Example {
    code: "ttk::combobox .country -textvariable country -values {UK US}\neval $country",
    focuses: &[
        focus(
            0,
            "-textvariable country",
            "lets user input update this variable",
        ),
        focus(
            1,
            "$country",
            "carries the untrusted value to a code-evaluation sink",
        ),
    ],
};
const FIELD_VARIABLE_SCOPE: Example = Example {
    code: "proc build {} {\n    ttk::entry .country -textvariable country\n}\nbuild\nputs $::country",
    focuses: &[
        focus(
            1,
            "-textvariable country",
            "Global resolves the unqualified link as ::country",
        ),
        focus(
            4,
            "$::country",
            "reads the same linked variable outside the procedure",
        ),
    ],
};
const FIELD_SCRIPT_TIMING: Example = Example {
    code: "button .save -command {save_document}\nputs ready",
    focuses: &[
        focus(
            0,
            "-command {save_document}",
            "stores this script for a later button event",
        ),
        focus(
            1,
            "puts ready",
            "runs when construction returns, before any future click",
        ),
    ],
};
const FIELD_SCRIPT_TIMING_RESOLVER: Example = Example {
    code: "send other {work now}\nsend -async other {work later}",
    focuses: &[
        focus(
            0,
            "{work now}",
            "the resolver reports SameInvocation without -async",
        ),
        focus(
            1,
            "{work later}",
            "the resolver reports Deferred when -async is present",
        ),
    ],
};
const FIELD_CALLBACK_TAINT_INPUTS: Example = Example {
    code: "entry .password -validatecommand {set proposed %P; eval $proposed}\nbind .password <Key> {set typed %A; eval $typed}",
    focuses: &[
        focus(
            0,
            "%P",
            "the proposed editable value is external input when validation runs",
        ),
        focus(
            0,
            "$proposed",
            "carries that value to the code-evaluation sink",
        ),
        focus(
            1,
            "%A",
            "the typed event character is external input for this binding",
        ),
    ],
};
const FIELD_METHOD_PREFIX_MATCHING: Example = Example {
    code: "entry .editor\n.editor g\n.editor c",
    focuses: &[
        focus(
            1,
            "g",
            "resolves to the one matching method, get, when Enabled",
        ),
        focus(
            2,
            "c",
            "stays unresolved because cget and configure are ambiguous",
        ),
    ],
};
const FIELD_COMMAND_FORMS: Example = Example {
    code: "cache get document\ncache set document contents",
    focuses: &[
        focus(0, "get", "a literal selector can choose the read-only form"),
        focus(
            1,
            "set",
            "a sibling selector can choose replacement mutation effects",
        ),
    ],
};
const FIELD_SUBCOMMAND_FORMS: Example = Example {
    code: "entry .editor\n.editor selection present\n.editor selection clear",
    focuses: &[
        focus(1, "present", "selects the nested read-only operation form"),
        focus(2, "clear", "keeps the parent method's mutation effects"),
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
        "taints_var_write" => Some(FIELD_TAINTS_VAR_WRITE),
        "variable_scope" => Some(FIELD_VARIABLE_SCOPE),
        "script_timing" => Some(FIELD_SCRIPT_TIMING),
        "script_timing_resolver" => Some(FIELD_SCRIPT_TIMING_RESOLVER),
        "callback_taint_inputs" => Some(FIELD_CALLBACK_TAINT_INPUTS),
        "method_prefix_matching" => Some(FIELD_METHOD_PREFIX_MATCHING),
        "command_forms" => Some(FIELD_COMMAND_FORMS),
        "subcommand_forms" => Some(FIELD_SUBCOMMAND_FORMS),
        "tk_geometry"
        | "container_policy"
        | "container_option"
        | "direct_form"
        | "placement_subcommand"
        | "release_subcommands" => Some(FIELD_TK_GEOMETRY),
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
        "scriptTiming" => Some(FIELD_SCRIPT_TIMING),
        "variableScope" => Some(FIELD_VARIABLE_SCOPE),
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
    let owner_annotation = example.carrier.and_then(|carrier| {
        example
            .annotations
            .iter()
            .position(|item| item.line == carrier.line && item.needle == carrier.needle)
            .or_else(|| {
                example.annotations.iter().position(|item| {
                    item.line == carrier.line && item.needle.contains(carrier.needle)
                })
            })
    });
    let annotations: Vec<Value> = example
        .annotations
        .iter()
        .enumerate()
        .map(|(index, item)| {
            json!({
                "line": item.line,
                "needle": item.needle,
                "label": if Some(index) == owner_annotation {
                    format!("{subject} — {}", item.label)
                } else {
                    item.label.to_owned()
                },
            })
        })
        .collect();
    let mut value = json!({ "code": example.code, "annotations": annotations });
    if let Some(carrier) = example.carrier {
        value["carrier"] = json!({
            "line": carrier.line,
            "needle": carrier.needle,
            "label": format!("{subject} is carried by this command"),
        });
    }
    value
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
    use std::collections::HashSet;

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
        if let Some(carrier) = example.get("carrier") {
            let line = usize::try_from(carrier["line"].as_u64().expect("carrier line"))
                .expect("carrier line fits usize");
            let needle = carrier["needle"].as_str().expect("carrier needle");
            if needle.is_empty() {
                errors.push(format!("{owner} has an empty carrier token"));
            } else if !lines.get(line).is_some_and(|text| text.contains(needle)) {
                errors.push(format!(
                    "{owner}: carrier line {line} does not contain {needle:?} in {code:?}"
                ));
            }
            if carrier["label"].as_str().is_none_or(str::is_empty) {
                errors.push(format!("{owner} has an empty carrier label"));
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
        for field in schema::NESTED_FIELDS {
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
                if id == "traits" && example.get("carrier").is_none() {
                    errors.push(format!("{id}/{key} has no command-token carrier"));
                }
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

    #[test]
    fn trait_subject_labels_only_the_carrier_arrow() {
        for &item in Trait::ALL {
            let subject = format!("{}: {}", item.name(), item.summary());
            let example = registry_example_json(item.example(), &subject);
            let carrier = example["carrier"].as_object().expect("trait carrier");
            let carrier_line = carrier["line"].as_u64().expect("carrier line");
            let carrier_needle = carrier["needle"].as_str().expect("carrier needle");
            let labelled: Vec<_> = example["annotations"]
                .as_array()
                .expect("annotations")
                .iter()
                .filter(|annotation| {
                    annotation["label"]
                        .as_str()
                        .is_some_and(|label| label.starts_with(&subject))
                })
                .collect();
            assert_eq!(
                labelled.len(),
                1,
                "{} must label exactly its owning command arrow",
                item.name()
            );
            assert_eq!(labelled[0]["line"].as_u64(), Some(carrier_line));
            assert!(
                labelled[0]["needle"]
                    .as_str()
                    .is_some_and(|needle| needle.contains(carrier_needle)),
                "{} labels an arrow that does not contain its carrier",
                item.name()
            );
        }
    }

    #[test]
    fn registry_examples_are_distinct_and_source_aligned() {
        let mut trait_programs = HashSet::new();
        for &item in Trait::ALL {
            let example = item.example();
            assert!(
                trait_programs.insert(example.code),
                "{} reuses another trait's worked example",
                item.name()
            );
            assert!(
                example.annotations.len() >= 2,
                "{} has too little flow",
                item.name()
            );
            let labels: HashSet<_> = example.annotations.iter().map(|item| item.label).collect();
            assert!(
                labels.len() >= 2,
                "{} has boilerplate-only arrows",
                item.name()
            );
            let carrier = example
                .carrier
                .expect("trait examples must identify their carrier");
            let lines: Vec<_> = example.code.lines().collect();
            assert!(
                lines
                    .get(carrier.line)
                    .is_some_and(|line| line.contains(carrier.needle)),
                "{} carrier is not source-aligned",
                item.name()
            );
            assert!(
                example.annotations.iter().any(|annotation| {
                    annotation.line == carrier.line && annotation.needle.contains(carrier.needle)
                }),
                "{} carrier is not explained by an arrow",
                item.name()
            );
        }

        let mut effect_programs = HashSet::new();
        for &target in SideEffectTarget::ALL {
            let example = target.example();
            assert!(
                effect_programs.insert(example.code),
                "{} reuses another side-effect target's worked example",
                target.name()
            );
            assert!(
                example.annotations.len() >= 2,
                "{} has too little flow",
                target.name()
            );
            let labels: HashSet<_> = example.annotations.iter().map(|item| item.label).collect();
            assert!(
                labels.len() >= 2,
                "{} has boilerplate-only arrows",
                target.name()
            );
        }
    }

    #[test]
    fn catchable_throw_numbers_the_throw_before_the_interception() {
        let example = Trait::from_name("CATCHABLE_THROW")
            .expect("catchable throw trait")
            .example();
        let needles: Vec<_> = example
            .annotations
            .iter()
            .map(|annotation| annotation.needle)
            .collect();
        assert_eq!(needles, ["error failure", "catch", "$status $message"]);
    }
}
