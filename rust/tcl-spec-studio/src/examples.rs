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

use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::byte_array_effect::ByteArrayEffect;
use tcl_registry::documentation::DocumentationExample;
use tcl_registry::patterns::{FormatType, PatternType};
use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget, StorageType};
use tcl_registry::taint::TaintColourAtom;
use tcl_registry::traits::Trait;
use tcl_registry::types::TclType;

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

mod catalogues;
mod fields_behaviour;
mod fields_core;
mod groups;

use catalogues::{
    CATALOGUE_ARG_ROLE, CATALOGUE_DIALECT, CATALOGUE_EFFECT, CATALOGUE_HOOK, CATALOGUE_OPTION,
    CATALOGUE_PREFIX, CATALOGUE_PRESENTATION, CATALOGUE_TAINT, CATALOGUE_TYPE,
};
use fields_behaviour::{FIELD_SCRIPT_TIMING, FIELD_TRAITS, FIELD_VARIABLE_SCOPE};
use groups::{
    ADVANCED, ARGUMENTS, AVAILABILITY, BEHAVIOUR, DEPRECATION, DOCUMENTATION, EFFECTS, HOOKS,
    IDENTITY, OPTIONS, SUBCOMMANDS, TAINT, TYPES,
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

/// The example for one spec field.
///
/// Exhaustive by test: `every_group_and_field_has_a_valid_example` fails by
/// name for a field with no entry. There is deliberately no group-level
/// fallback — inheriting the group's snippet is how a hundred settings shipped
/// showing something other than themselves (#1714).
fn field_template(key: &str) -> Option<Example> {
    fields_core::ENTRIES
        .iter()
        .chain(fields_behaviour::ENTRIES)
        .find(|(entry, _)| *entry == key)
        .map(|(_, example)| *example)
}

fn catalogue_template(id: &str) -> Option<Example> {
    match id {
        "tclType" | "storageType" | "byteArrayEffect" => Some(CATALOGUE_TYPE),
        "bodyKind" | "argPresentation" => Some(CATALOGUE_PRESENTATION),
        "scriptTiming" => Some(FIELD_SCRIPT_TIMING),
        "variableScope" => Some(FIELD_VARIABLE_SCOPE),
        "commandTableEffect" | "definedSymbolKind" | "sideEffectTarget" | "connectionSide"
        | "formKind" => Some(CATALOGUE_EFFECT),
        // The plain "pick one variant" shape: every closed catalogue whose
        // example is just "here is one of its values".
        "argRole"
        | "patternType"
        | "formatType"
        | "defaultFormFirstWord"
        | "prefixMatching"
        | "optionPlacement" => Some(CATALOGUE_ARG_ROLE),
        "loweringHook" | "codegenHook" | "inlineCodegenHook" | "analyserHook"
        | "returnTypeHook" => Some(CATALOGUE_HOOK),
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
pub fn field_example(key: &str, label: &str) -> Option<Value> {
    field_template(key).map(|example| example_json(example, label))
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
    let owned = |example: DocumentationExample| registry_example_json(example, &subject);
    match id {
        "traits" => Trait::from_name(key).map(Trait::example).map(owned),
        "taintColour" => TaintColourAtom::from_name(key)
            .map(TaintColourAtom::example)
            .map(owned),
        "sideEffectTarget" => SideEffectTarget::from_name(key)
            .map(SideEffectTarget::example)
            .map(owned),
        "argRole" => named(ArgRole::ALL, key).map(ArgRole::example).map(owned),
        // Payload-carrying, so there is no `ALL` to search: the catalogue
        // names the shape and the payload here is only a stand-in for it.
        "appendedArity" => match key {
            "Exactly" => Some(AppendedArity::Exactly(2)),
            "AtLeast" => Some(AppendedArity::AtLeast(1)),
            "Unknown" => Some(AppendedArity::Unknown),
            _ => None,
        }
        .map(AppendedArity::example)
        .map(owned),
        "tclType" => named(TclType::ALL, key).map(TclType::example).map(owned),
        "storageType" => named(StorageType::ALL, key)
            .map(StorageType::example)
            .map(owned),
        "connectionSide" => named(ConnectionSide::ALL, key)
            .map(ConnectionSide::example)
            .map(owned),
        "byteArrayEffect" => named(ByteArrayEffect::ALL, key)
            .map(ByteArrayEffect::example)
            .map(owned),
        "patternType" => named(PatternType::ALL, key)
            .map(PatternType::example)
            .map(owned),
        "formatType" => named(FormatType::ALL, key)
            .map(FormatType::example)
            .map(owned),
        _ => catalogue_template(id).map(|example| example_json(example, &subject)),
    }
}

/// Resolve a catalogue key back to the enum variant it names.
///
/// `catalogue::Variant::key` is documented as the Rust variant spelling, which
/// is what `Debug` prints — so the catalogue and the enum agree by
/// construction, and no per-enum `from_name` has to be written and kept in
/// step. A payload-carrying variant prints as `Exactly(2)`; the catalogue
/// names the shape, so the payload is cut off.
fn named<T: Copy + std::fmt::Debug>(all: &[T], key: &str) -> Option<T> {
    all.iter().copied().find(|item| {
        let printed = format!("{item:?}");
        printed.split('(').next().unwrap_or(printed.as_str()) == key
    })
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
        errors.extend(causal_order_errors(annotations, &lines, owner));
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

    /// Arrows are numbered by their position in the array and drawn as
    /// numbered steps, so their order is a claim about *when things happen*
    /// (#1714). Three rules follow, all checkable against the source:
    ///
    /// 1. **Numbering runs forwards through the program.** An arrow on an
    ///    earlier line may not be numbered after one on a later line — that
    ///    tells the reader the consequence before the cause.
    /// 2. **A substitution is numbered before the word that consumes it.**
    ///    `[gets stdin]` inside `puts [gets stdin]` is evaluated first, so it
    ///    is step 1. This is why left-to-right is deliberately *not* the rule:
    ///    `error` before `catch` on one line is right, and only containment
    ///    can say so. Only a `[…]` or `$…` needle counts — a `%b` inside a
    ///    braced format string is contained but not substituted, and Tcl's own
    ///    rule is the one to follow here.
    /// 3. **Two arrows on a line may not start at the same column.** The
    ///    browser finds a needle with `indexOf` and draws its bracket from
    ///    there, so `$item` on a line holding `$items`, or `set local` beside
    ///    `set local value`, stack two brackets on one token and at least one
    ///    label describes something the reader is not being shown.
    ///
    /// Spans that merely overlap, or sit side by side, are left alone: their
    /// order is the author's knowledge of the flow, which no rule here can
    /// recover.
    fn causal_order_errors(annotations: &[Value], lines: &[&str], owner: &str) -> Vec<String> {
        fn needle(annotation: &Value) -> &str {
            annotation["needle"].as_str().unwrap_or_default()
        }
        let mut errors = Vec::new();
        let at = |annotation: &Value| {
            usize::try_from(annotation["line"].as_u64().expect("line")).expect("line fits usize")
        };
        // The span the browser will actually bracket: the needle's first
        // occurrence, which is what `indexOf` finds.
        let span = |annotation: &Value| -> Option<(usize, usize)> {
            let text = lines.get(at(annotation))?;
            let start = text.find(needle(annotation))?;
            Some((start, start + needle(annotation).len()))
        };

        for pair in annotations.windows(2) {
            if at(&pair[0]) > at(&pair[1]) {
                errors.push(format!(
                    "{owner}: an arrow on line {} is numbered after one on line {} — \
                     number them in execution order",
                    at(&pair[1]),
                    at(&pair[0])
                ));
            }
        }

        for (index, first) in annotations.iter().enumerate() {
            for second in &annotations[index + 1..] {
                if at(first) != at(second) {
                    continue;
                }
                let (Some(outer), Some(inner)) = (span(first), span(second)) else {
                    continue;
                };
                if outer.0 == inner.0 {
                    errors.push(format!(
                        "{owner}: {:?} and {:?} both bracket line {} from column {} — the \
                         browser finds a needle by its first occurrence, so the two arrows \
                         land on one token",
                        needle(first),
                        needle(second),
                        at(first),
                        outer.0
                    ));
                } else if outer.0 < inner.0
                    && inner.1 <= outer.1
                    && (needle(second).starts_with('[') || needle(second).starts_with('$'))
                {
                    errors.push(format!(
                        "{owner}: {:?} is substituted before {:?} consumes it, so it must be \
                         the earlier arrow",
                        needle(second),
                        needle(first)
                    ));
                }
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
            let example = field_example(field.key, field.label)
                .unwrap_or_else(|| panic!("no example for {}", field.key));
            assert_valid(&example, field.key);
        }
        for field in schema::NESTED_FIELDS {
            let example = field_example(field.key, field.label)
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
