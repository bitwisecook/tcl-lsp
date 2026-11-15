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

use tcl_registry::arg_role::{AppendedArity, AppendedAritySet, ArgRole};
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

const FIELD_ARG_ROLE_RESOLVER_ROLES: Example = Example {
    code: "if {$ready} { run } elseif {$waiting} { wait }",
    focuses: &[
        focus(0, "{$ready}", "Expr is one role the resolver can return"),
        focus(0, "{ run }", "Body is another role the resolver can return"),
    ],
};

mod catalogues;
mod fields_behaviour;
mod fields_core;

use catalogues::{
    CATALOGUE_CONFIGURATION_REFERENCE, CATALOGUE_CONNECTION_SIDE, CATALOGUE_DEFINITION_BODY,
    CATALOGUE_SCRIPT_LOOKUP_SCOPE,
};
use fields_behaviour::{FIELD_SCRIPT_TIMING, FIELD_TRAITS, FIELD_VARIABLE_SCOPE};

/// The example inherited by every field in a form group.
/// The example for one form group: the example of the field that best
/// stands for what the group is about.
fn group_template(group: &str) -> Option<Example> {
    let field = match group {
        "Identity" => "name",
        "Availability" => "surface",
        "Arity and arguments" => "arity",
        "Types" => "return_type",
        "Subcommands" => "subcommands",
        "Documentation" => "hover",
        "Options and values" => "options",
        "Behaviour" => "completion",
        "Side effects" => "side_effects",
        "Compiler hooks" => "codegen_hook",
        "Taint and security" => "taint_source",
        "Deprecation and translation" => "deprecation_fix",
        "Advanced" => "arg_role_layout_resolver",
        _ => return None,
    };
    field_template(field)
}

/// The example for one spec field.
///
/// Exhaustive by test: `every_group_and_field_has_a_valid_example` fails by
/// name for a field with no entry. There is deliberately no group-level
/// fallback — inheriting the group's snippet would let a field ship
/// showing something other than itself.
fn field_template(key: &str) -> Option<Example> {
    if key == "arg_role_resolver_roles" {
        return Some(FIELD_ARG_ROLE_RESOLVER_ROLES);
    }
    fields_core::ENTRIES
        .iter()
        .chain(fields_behaviour::ENTRIES)
        .find(|(entry, _)| *entry == key)
        .map(|(_, example)| *example)
}

/// The example for one catalogue: the example of the field whose values the
/// catalogue supplies, so it shows a real command using one of them.
fn catalogue_template(id: &str) -> Option<Example> {
    let field = match id {
        "scriptTiming" => return Some(FIELD_SCRIPT_TIMING),
        "variableScope" => return Some(FIELD_VARIABLE_SCOPE),
        "traits" => return Some(FIELD_TRAITS),
        "sourceConfigurationReference" => return Some(CATALOGUE_CONFIGURATION_REFERENCE),
        "definitionBody" => return Some(CATALOGUE_DEFINITION_BODY),
        "scriptLookupScope" => return Some(CATALOGUE_SCRIPT_LOOKUP_SCOPE),
        "connectionSide" => return Some(CATALOGUE_CONNECTION_SIDE),
        "irulesSourceFlow" => "irules_source_flow",
        "tclType" => "return_type",
        "storageType" => "inferred_storage_type",
        "byteArrayEffect" => "byte_array_effect",
        "bodyKind" => "body_kind",
        "argPresentation" => "arg_presentation",
        "commandTableEffect" => "command_table_effect",
        "definedSymbolKind" => "defines_symbol",
        "sideEffectTarget" => "side_effects",
        "formKind" => "forms",
        "argRole" => "arg_roles",
        "patternType" => "pattern_type",
        "formatType" => "format_string_type",
        "sourceDeprecationAdvice" => "source_deprecation_advice",
        "sourcePathOperation" => "source_path_operation",
        "sourceIndexBounds" => "source_index_bounds",
        "channelConfiguration" => "channel_configuration",
        "defaultFormFirstWord" => "default_form_first_word",
        "prefixMatching" => "prefix_matching",
        "optionPlacement" => "option_placement",
        "loweringHook" => "lowering_hook",
        "codegenHook" => "codegen_hook",
        "inlineCodegenHook" => "inline_codegen_hook",
        "analyserHook" => "analyser_hook",
        "returnTypeHook" => "return_type_hook",
        "semanticOperation" => "semantic_operation",
        "taintColour" => "taint_transform",
        "taintTransformCondition" => "taint_transform_when",
        "taintNumericCoercion" => "taint_numeric_coercion",
        "dialects" => "surface",
        "appendedArity" => "command_prefixes",
        "optionArity" => "options",
        _ => return None,
    };
    field_template(field)
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
            "OneOf" => Some(AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(
                &[2, 4],
            ))),
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
    /// numbered steps, so their order is a claim about *when things happen*.
    /// Three rules follow, all checkable against the source:
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

    /// The words of the command that starts at `text`, ending where an
    /// unmatched `]` or `}` closes the enclosing substitution or body.
    fn carrier_command_words(text: &str) -> Vec<String> {
        let mut depth = 0usize;
        let mut end = text.len();
        for (index, byte) in text.bytes().enumerate() {
            match byte {
                b'[' | b'{' => depth += 1,
                b']' | b'}' if depth == 0 => {
                    end = index;
                    break;
                }
                b']' | b'}' => depth -= 1,
                _ => {}
            }
        }
        let command = &text[..end];
        let config = tcl_lexer::LexerConfig::default();
        tcl_syntax::event_handler::script_commands(command, config)
            .into_iter()
            .next()
            .map_or_else(Vec::new, |parsed| {
                parsed
                    .words
                    .iter()
                    .map(|tokens| {
                        let start = tokens.first().map_or(0, |t| t.span.start()) as usize;
                        let stop = tokens.last().map_or(0, |t| t.span.end()) as usize;
                        command[start..stop.min(command.len())].to_owned()
                    })
                    .collect()
            })
    }

    /// Tcl 8.4 to 9.1 (each with the same Tk release provided), `f5-irules`,
    /// `sslictcl`, and the unprojected catalogue.
    fn audit_registries() -> Vec<std::sync::Arc<tcl_registry::CommandRegistry>> {
        let releases = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];
        releases
            .iter()
            .map(|release| {
                let profile = tcl_dialect::DialectProfile::find(release).expect("Tcl release");
                let mut registry =
                    tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
                registry.insert_ambient_package("Tk", &release[3..]);
                std::sync::Arc::new(registry)
            })
            .chain(
                ["f5-irules", "f5-iapps", "f5-tmsh", "sslictcl"]
                    .iter()
                    .map(|dialect| {
                        std::sync::Arc::new(
                            tcl_registry::CommandRegistry::build_default().project_for_profile(
                                tcl_dialect::DialectProfile::find(dialect).expect("dialect"),
                            ),
                        )
                    }),
            )
            .chain(["jim", "spectcl"].iter().map(|dialect| {
                std::sync::Arc::clone(
                    tcl_registry::model::ingress::static_context_for(dialect).commands(),
                )
            }))
            .chain(std::iter::once(std::sync::Arc::new(
                tcl_registry::CommandRegistry::build_default(),
            )))
            .collect()
    }

    /// Every command an example runs: each line's first command and each
    /// command substitution, as `(head, args)`.
    fn example_commands(code: &str) -> Vec<(String, Vec<String>)> {
        let mut out = Vec::new();
        for line in code.lines() {
            let starts = std::iter::once(line.len() - line.trim_start().len())
                .chain(line.match_indices('[').map(|(index, _)| index + 1))
                .chain(line.match_indices("{ ").map(|(index, _)| index + 2));
            for start in starts {
                let mut words = carrier_command_words(&line[start..]);
                if !words.is_empty() {
                    let head = words.remove(0);
                    out.push((head, words));
                }
            }
        }
        out
    }

    /// The command each annotation's needle starts, as `(head, args)`.
    fn annotated_commands(example: &DocumentationExample) -> Vec<(String, Vec<String>)> {
        example
            .annotations
            .iter()
            .filter_map(|annotation| {
                let line = example.code.lines().nth(annotation.line)?;
                let start = line.find(annotation.needle)?;
                let text = line[start..].strip_prefix('[').unwrap_or(&line[start..]);
                let mut words = carrier_command_words(text);
                (!words.is_empty()).then(|| {
                    let head = words.remove(0);
                    (head, words)
                })
            })
            .collect()
    }

    /// Every side-effect target's worked example annotates at least one
    /// command that declares an effect on that target, under at least one
    /// of [`audit_registries`].
    #[test]
    fn every_side_effect_example_annotates_a_command_declaring_that_target() {
        let registries = audit_registries();
        let declares = |effects: &[tcl_registry::side_effects::SideEffect], target| {
            effects.iter().any(|effect| effect.target == target)
        };
        let declared_anywhere = |target| {
            registries.iter().any(|registry| {
                registry.command_names().any(|name| {
                    registry.get(name).is_some_and(|spec| {
                        declares(spec.side_effects, target)
                            || spec
                                .subcommands
                                .iter()
                                .any(|sub| declares(sub.side_effects, target))
                            || spec.object_class.is_some_and(|class| {
                                class
                                    .instance_methods
                                    .iter()
                                    .any(|method| declares(method.side_effects, target))
                            })
                    })
                })
            })
        };
        let mut errors = Vec::new();
        for &target in SideEffectTarget::ALL {
            let example = target.example();
            let commands = annotated_commands(&example);
            if !declared_anywhere(target) {
                // Pack-only vocabulary must not be pinned on a shipped command.
                if let Some((head, _)) = commands.iter().find(|(head, _)| {
                    registries
                        .iter()
                        .any(|registry| registry.get(head).is_some())
                }) {
                    errors.push(format!(
                        "{}: pack-only, yet annotates `{head}`",
                        target.name()
                    ));
                }
                continue;
            }
            let found = commands.iter().any(|(head, args)| {
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                let class = example.code.lines().find_map(|earlier| {
                    let mut words = earlier.split_whitespace();
                    let class = words.next()?;
                    (words.next() == Some(head.as_str())).then_some(class)
                });
                registries.iter().any(|registry| {
                    let facts = match (registry.get(head), class) {
                        (Some(spec), _) => {
                            let sub = args.first().and_then(|word| spec.resolve_subcommand(word));
                            if declares(spec.side_effects, target)
                                || sub.is_some_and(|sub| declares(sub.side_effects, target))
                            {
                                return true;
                            }
                            registry.resolve_invocation(head, &args, None)
                        }
                        (None, Some(class)) => {
                            registry.resolve_instance_invocation(class, head, &args, None)
                        }
                        (None, None) => None,
                    };
                    facts.is_some_and(|invocation| {
                        declares(&invocation.facts().effects.legacy().side_effects, target)
                    })
                })
            });
            if !found {
                errors.push(format!(
                    "{}: {}",
                    target.name(),
                    example.code.replace('\n', "; ")
                ));
            }
        }
        assert!(
            errors.is_empty(),
            "side-effect examples with no annotated command declaring the target:\n{}",
            errors.join("\n")
        );
    }

    /// Every taint colour's worked example shows what really produces it.
    ///
    /// A registry colour must sit on an annotated command, as the colour its
    /// result is sourced with, a transform it adds, or the colour its sink
    /// accepts; `TAINTED` may also come from a `TAINT_SOURCE` command. Two
    /// colours are derived by the compiler's taint pass itself, and their
    /// examples must annotate the construct it reads: a literal non-dash
    /// prefix (`leading_literal_prefix_char`) and a `string match` branch
    /// guard (`compute_branch_guard_map`). A colour no shipped spec produces
    /// is shown on a pack's own `mypkg::` command.
    #[test]
    fn every_taint_colour_example_annotates_what_produces_it() {
        const ANALYSER_DERIVED: &[(&str, &str)] = &[
            ("NON_DASH_PREFIXED", "value:$user"),
            ("PATH_BOUNDED", "string match"),
        ];
        let registries = audit_registries();
        let produces = |spec: &tcl_registry::CommandSpec,
                        sub: Option<&tcl_registry::spec::SubCommand>,
                        colour: tcl_registry::TaintColour| {
            [
                spec.taint_source,
                spec.taint_transform,
                spec.taint_sink_safe_colour,
                sub.and_then(|sub| sub.taint_transform),
            ]
            .into_iter()
            .flatten()
            .any(|declared| declared.contains(colour))
        };
        let shipped = |colour| {
            registries.iter().any(|registry| {
                registry.command_names().any(|name| {
                    registry.get(name).is_some_and(|spec| {
                        produces(spec, None, colour)
                            || spec
                                .subcommands
                                .iter()
                                .any(|sub| produces(spec, Some(sub), colour))
                    })
                })
            })
        };
        let mut errors = Vec::new();
        for atom in TaintColourAtom::ALL {
            let colour = atom.colour();
            let example = atom.example();
            if let Some((_, needle)) = ANALYSER_DERIVED
                .iter()
                .find(|(name, _)| *name == atom.name())
            {
                if !example
                    .annotations
                    .iter()
                    .any(|annotation| annotation.needle.contains(needle))
                {
                    errors.push(format!("{}: does not annotate `{needle}`", atom.name()));
                }
                continue;
            }
            let commands = annotated_commands(&example);
            if !shipped(colour) {
                if !commands.iter().any(|(head, _)| head.starts_with("mypkg::")) {
                    errors.push(format!(
                        "{}: no shipped producer, so show a pack command",
                        atom.name()
                    ));
                }
                continue;
            }
            let found = commands.iter().any(|(head, args)| {
                registries.iter().any(|registry| {
                    let Some(spec) = registry.get(head) else {
                        return false;
                    };
                    let sub = args.first().and_then(|word| spec.resolve_subcommand(word));
                    produces(spec, sub, colour)
                        || (atom.name() == "TAINTED"
                            && spec.traits.contains(tcl_registry::Traits::TAINT_SOURCE))
                })
            });
            if !found {
                errors.push(format!(
                    "{}: {}",
                    atom.name(),
                    example.code.replace('\n', "; ")
                ));
            }
        }
        assert!(
            errors.is_empty(),
            "taint-colour examples that misstate their producer:\n{}",
            errors.join("\n")
        );
    }

    /// Every argument role's worked example annotates a command that can
    /// assign that role: statically, through its resolver's declared roles,
    /// through an option's value, or in its resolved invocation.
    #[test]
    fn every_arg_role_example_annotates_a_command_assigning_it() {
        let registries = audit_registries();
        let assigns = |spec: &tcl_registry::CommandSpec, role: ArgRole| {
            let options = spec.options.iter().any(|option| {
                option.value_role() == Some(role) || option.value_also_role() == Some(role)
            });
            spec.arg_roles.iter().any(|(_, held)| *held == role)
                || spec.arg_role_resolver_roles.contains(&role)
                || options
                || spec.subcommands.iter().any(|sub| {
                    sub.arg_roles.iter().any(|(_, held)| *held == role)
                        || sub.arg_role_resolver_roles.contains(&role)
                        || sub
                            .options
                            .iter()
                            .any(|option| option.value_role() == Some(role))
                })
        };
        let shipped = |role| {
            registries.iter().any(|registry| {
                registry
                    .command_names()
                    .any(|name| registry.get(name).is_some_and(|spec| assigns(spec, role)))
            })
        };
        let mut errors = Vec::new();
        for &role in ArgRole::ALL {
            let example = role.example();
            let commands = example_commands(example.code);
            // Structural roles come from the command's tables, not its role
            // rows: a declared subcommand name, and a declared `--` option.
            let structural = match role {
                ArgRole::Subcommand => example.carrier.is_some_and(|carrier| {
                    commands.iter().any(|(head, _)| {
                        registries.iter().any(|registry| {
                            registry.get(head).is_some_and(|spec| {
                                spec.resolve_subcommand(carrier.needle).is_some()
                            })
                        })
                    })
                }),
                ArgRole::OptionTerminator => commands.iter().any(|(head, _)| {
                    registries.iter().any(|registry| {
                        registry.get(head).is_some_and(|spec| {
                            spec.options.iter().any(|option| option.name == "--")
                        })
                    })
                }),
                _ => false,
            };
            if structural {
                continue;
            }
            let found = commands.iter().any(|(head, args)| {
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                registries.iter().any(|registry| {
                    registry.get(head).is_some_and(|spec| assigns(spec, role))
                        || registry.resolve_invocation(head, &args, None).is_some_and(
                            |invocation| {
                                invocation
                                    .facts()
                                    .arg_roles
                                    .iter()
                                    .chain(&invocation.authored_source_argument_roles().0)
                                    .any(|(_, held)| *held == role)
                            },
                        )
                })
            });
            if !found && !shipped(role) {
                // Pack-only vocabulary must not be pinned on a shipped command.
                if let Some((head, _)) = commands.iter().find(|(head, _)| {
                    head != "set"
                        && registries
                            .iter()
                            .any(|registry| registry.get(head).is_some())
                }) {
                    errors.push(format!("{role:?}: pack-only, yet shown on `{head}`"));
                }
                continue;
            }
            if !found {
                errors.push(format!("{role:?}: {}", example.code.replace('\n', "; ")));
            }
        }
        assert!(
            errors.is_empty(),
            "argument-role examples with no annotated command assigning the role:\n{}",
            errors.join("\n")
        );
    }

    /// The geometry examples state what each manager's registry descriptor
    /// declares, in every Tk release from 8.4 to 9.1. `tkGrid.c`,
    /// `tkPack.c` and `tkPlace.c` agree: only `grid` has `remove`, and only
    /// `pack` and `grid` claim their container (Tk 8.6 onwards checks it).
    #[test]
    fn geometry_examples_match_the_manager_descriptors() {
        use tcl_registry::tk_geometry::TkGeometryContainerPolicy;

        for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{release}")).expect("Tcl release");
            let mut registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            registry.insert_ambient_package("Tk", release);
            let manager = |name: &str| {
                registry
                    .get(name)
                    .and_then(|spec| spec.tk_geometry)
                    .unwrap_or_else(|| panic!("Tk {release} {name} geometry"))
            };
            for key in [
                "tk_geometry",
                "container_policy",
                "container_option",
                "direct_form",
                "placement_subcommand",
                "release_subcommands",
            ] {
                let example = field_template(key).expect("geometry example");
                for focus in example.focuses {
                    let line = example.code.lines().nth(focus.line).expect("line");
                    let mut words = line.split_whitespace();
                    let Some(head) = words.next() else { continue };
                    let Some(spec) = registry.get(head).and_then(|spec| spec.tk_geometry) else {
                        continue;
                    };
                    let word = focus.needle.split_whitespace().next().unwrap_or_default();
                    if word.starts_with('-') {
                        assert_eq!(spec.container_option, Some(word), "Tk {release} {key}");
                    } else if !word.starts_with('.') && word != head {
                        assert!(
                            spec.placement_subcommand == Some(word)
                                || spec.release_subcommands.contains(&word),
                            "Tk {release} {key}: `{head} {word}` is neither a placement nor a release subcommand"
                        );
                    }
                    if focus.note.contains("Exclusive") {
                        assert_eq!(spec.container_policy, TkGeometryContainerPolicy::Exclusive);
                    }
                    if focus.note.starts_with("Independent") {
                        assert_eq!(
                            spec.container_policy,
                            TkGeometryContainerPolicy::Independent
                        );
                    }
                }
            }
            assert!(manager("grid").release_subcommands.contains(&"remove"));
            assert!(!manager("pack").release_subcommands.contains(&"remove"));
            assert_eq!(
                manager("place").container_policy,
                TkGeometryContainerPolicy::Independent
            );
        }
    }

    /// Every trait example's carrier resolves, under at least one Tcl
    /// release from 8.4 to 9.1 (with Tk of the same release ambient), to an
    /// invocation whose registry facts contain that trait.
    #[test]
    fn every_trait_carrier_resolves_to_an_invocation_with_that_trait() {
        let registries = audit_registries();

        let mut errors = Vec::new();
        for &item in Trait::ALL {
            let example = item.example();
            let Some(carrier) = example.carrier else {
                continue;
            };
            let line = example
                .code
                .lines()
                .nth(carrier.line)
                .expect("carrier line");
            let start = line.find(carrier.needle).expect("carrier needle");
            let words = carrier_command_words(&line[start..]);
            let Some((head, args)) = words.split_first() else {
                errors.push(format!(
                    "{}: no command at `{}`",
                    item.name(),
                    carrier.needle
                ));
                continue;
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let flag = tcl_registry::Traits::of(&[item]);
            let declared = |registry: &tcl_registry::CommandRegistry| {
                let spec = registry.get(head)?;
                let sub = args.first().and_then(|word| spec.resolve_subcommand(word));
                Some(
                    spec.traits
                        .union(sub.map_or_else(tcl_registry::Traits::empty, |sub| sub.traits)),
                )
            };
            if registries
                .iter()
                .all(|registry| registry.get(head).is_none())
            {
                // A widget path carries an instance-method trait: resolve the
                // method through the class whose constructor created that
                // path on an earlier line. Any other head must be registered.
                let class = example.code.lines().take(carrier.line).find_map(|earlier| {
                    let mut words = earlier.split_whitespace();
                    let class = words.next()?;
                    (words.next() == Some(head.as_str())).then_some(class)
                });
                let instance_carries = class.is_some_and(|class| {
                    registries.iter().any(|registry| {
                        registry
                            .resolve_instance_invocation(class, head, &args, None)
                            .is_some_and(|invocation| invocation.facts().traits.contains(flag))
                    })
                });
                if !instance_carries {
                    errors.push(format!(
                        "{}: `{head}` is neither registered nor an instance whose class method carries the trait",
                        item.name()
                    ));
                }
                continue;
            }
            let holds_trait = registries.iter().any(|registry| {
                declared(registry).is_some_and(|traits| traits.contains(flag))
                    || registry
                        .resolve_invocation(head, &args, None)
                        .is_some_and(|invocation| invocation.facts().traits.contains(flag))
            });
            if !holds_trait {
                errors.push(format!("{}: `{head} {}`", item.name(), args.join(" ")));
            }
        }
        assert!(
            errors.is_empty(),
            "trait carriers whose resolved invocation lacks the trait:\n{}",
            errors.join("\n")
        );
    }

    fn focus_labelled<'a>(example: &'a Example, prefix: &str) -> &'a Focus {
        example
            .focuses
            .iter()
            .find(|focus| focus.note.starts_with(prefix))
            .unwrap_or_else(|| panic!("no focus labelled {prefix}"))
    }

    /// The `traits` field example must put `PURE` on a command whose own
    /// `traits` field declares it. `string length` is pure only through its
    /// subcommand row; `llength` (Tcl 8.4 to 9.1) carries the trait itself.
    #[test]
    fn traits_field_pure_arrow_sits_on_a_command_declaring_the_trait() {
        use tcl_dialect::model::{Family, SurfaceQuery};

        let pure = focus_labelled(&FIELD_TRAITS, "PURE:");
        let head = pure
            .needle
            .trim_start_matches('[')
            .split_whitespace()
            .next()
            .expect("carrier head");
        let registry = tcl_registry::CommandRegistry::build_default();
        let spec = registry.get(head).expect("registered carrier");
        assert!(spec.is_pure(), "{head}");
        for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
            let resolved = registry
                .resolve_invocation(
                    head,
                    &["$items"],
                    Some(SurfaceQuery::core(Family::Tcl, release)),
                )
                .unwrap_or_else(|| panic!("Tcl {release} resolves {head}"));
            assert!(resolved.facts().is_pure(), "Tcl {release} {head} is pure");
        }
        assert!(
            !pure.note.contains("fold"),
            "a call with a variable argument is not folded"
        );
    }

    /// `switch -integer` is in the option table of Tcl 9.1's `tclCmdMZ.c`
    /// only; 8.4, 8.5, 8.6 and 9.0 reject it. The example that uses it must
    /// say which release it describes.
    #[test]
    fn numeric_coercion_example_names_the_release_with_switch_integer() {
        use tcl_dialect::model::{Family, SurfaceQuery};

        let registry = tcl_registry::CommandRegistry::build_default();
        let switch = registry.get("switch").expect("switch");
        for (release, present) in [
            ("8.4", false),
            ("8.5", false),
            ("8.6", false),
            ("9.0", false),
            ("9.1", true),
        ] {
            assert_eq!(
                switch
                    .find_option(
                        "-integer",
                        Some(SurfaceQuery::core(Family::Tcl, release)),
                        None
                    )
                    .is_some(),
                present,
                "Tcl {release} switch -integer"
            );
        }
        let example = field_template("taint_numeric_coercion").expect("example");
        let integer = example
            .focuses
            .iter()
            .find(|focus| focus.needle == "-integer")
            .expect("-integer arrow");
        assert!(integer.note.contains("9.1"), "{}", integer.note);
    }

    /// `clock seconds` returns the same value twice within one second in
    /// every release (Tcl 8.4 to 9.1), so volatility means "may differ",
    /// never "never repeats".
    #[test]
    fn volatile_examples_do_not_claim_a_result_never_repeats() {
        for key in ["result_stability", "pure", "mutator"] {
            let example = field_template(key).expect("example");
            for focus in example.focuses {
                assert!(
                    !focus.note.contains("never repeats"),
                    "{key}: {}",
                    focus.note
                );
                assert!(!focus.note.contains("fold"), "{key}: {}", focus.note);
                assert!(!focus.note.contains("removable"), "{key}: {}", focus.note);
            }
        }
    }

    /// Every group and catalogue example shows a real command, and no two
    /// catalogues share one snippet: a shared placeholder is how a vocabulary
    /// ships illustrated by something other than itself.
    #[test]
    fn group_and_catalogue_examples_are_real_and_distinct() {
        let mut seen = std::collections::HashMap::new();
        for (id, _, _) in crate::help::CATALOGUE_HELP {
            let example = catalogue_template(id).expect("catalogue example");
            assert!(
                !example.code.contains("mycommand"),
                "{id} uses a placeholder"
            );
            if let Some(other) = seen.insert(example.code, *id) {
                panic!("{id} and {other} share one example");
            }
        }
        for group in schema::GROUPS {
            let example = group_template(group).expect("group example");
            assert!(
                !example.code.contains("mycommand"),
                "{group} uses a placeholder"
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
