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

//! The field schema driving the studio's form.
//!
//! One [`FieldSchema`] per `CommandSpec` / `SubCommand` field, carrying the
//! field's Rust name (which is also the draft's JSON key and what the `.rs`
//! renderer emits), a human label, a one-line help string, the editor
//! [`FieldKind`], and the group it belongs to.
//!
//! The form, the draft model, and the renderer all read this one table, so
//! adding a field to `CommandSpec` means adding one entry here — not touching
//! the UI, the serialiser, and the renderer separately. The
//! `schema_covers_every_spec_field` test in `tests/schema_coverage.rs` fails
//! when the two drift apart.

use serde_json::{Value, json};

mod catalogues;
mod command_fields;
mod dsl_display;
mod subcommand_fields;

pub use catalogues::catalogues;
pub use command_fields::COMMAND_FIELDS;
pub use dsl_display::{DslDisplayContext, DslDisplayGrammar, DslStatementDisplay, dsl_word_value};
pub use subcommand_fields::SUBCOMMAND_FIELDS;

/// How the studio edits (and the renderer emits) one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Plain `bool`.
    Bool,
    /// `Option<bool>` — unset / true / false.
    OptBool,
    /// A non-negative count (`u8` / `usize`).
    Count,
    /// `Option<u16>` — a non-negative count, or unset.
    OptCount,
    /// `Option<u8>` — a 0-based argument index, or unset.
    OptIndex,
    /// `&'static str`.
    Text,
    /// `Option<&'static str>`.
    OptText,
    /// `&'static str` holding a paragraph or more.
    Prose,
    /// `&'static [&'static str]`.
    TextList,
    /// `&'static [u8]` — a list of argument indices.
    IndexList,
    /// `Option<&'static [u8]>` — tri-state index list (unset vs empty).
    OptIndexList,
    /// Optional argv positions with their selected variable naming purpose.
    VariableReceivers,
    /// One variant of a named catalogue; `optional` wraps it in `Option`.
    Enum {
        /// Catalogue id — a key of [`catalogues`].
        catalogue: &'static str,
        /// Whether the field is `Option<T>`.
        optional: bool,
    },
    /// A bitflag union over a named catalogue; `optional` wraps it in `Option`.
    FlagSet {
        /// Catalogue id — a key of [`catalogues`].
        catalogue: &'static str,
        /// Whether the field is `Option<T>`.
        optional: bool,
    },
    /// `&'static [T]` where `T` is a named catalogue enum.
    EnumList {
        /// Catalogue id — a key of [`catalogues`].
        catalogue: &'static str,
    },
    /// [`tcl_registry::Arity`] — min, max, step, and the extra exact count.
    Arity,
    /// [`tcl_registry::BodyInterpreter`] — current or argument-selected.
    BodyInterpreter,
    /// `&'static [(u8, ArgRole)]`.
    RoleMap,
    /// `&'static [(u8, AppendedArity)]` — command-prefix positions.
    PrefixMap,
    /// `&'static [(u8, &'static [CallbackTaintInput])]` — externally
    /// controlled substitutions injected into a deferred callback argument.
    CallbackTaintMap,
    /// `&'static [(u8, ArgPresentation)]` — formatter layout overrides.
    PresentationMap,
    /// `&'static [(u8, ArgTypeHint)]`.
    ArgTypeMap,
    /// `&'static [(u8, &'static [ArgValue])]`.
    ArgValueMap,
    /// `&'static [OptionSpec]`.
    Options,
    /// `&'static [FormSpec]`.
    Forms,
    /// `&'static [CommandForm]` — named invocation forms that override the
    /// command's arity, roles and options.
    Refinements,
    /// `&'static [SideEffect]`.
    SideEffects,
    /// How completely a sibling facts field is known: `{level, complete}`
    /// (issue #1712). Rendered into that field's `Knowledge` literal.
    Knowledge,
    /// `&'static [SetterConstraint]`.
    SetterConstraints,
    /// `&'static [ManufacturerMethod]` — class-command instance factories.
    ManufacturerMethods,
    /// `Option<HoverSnippet>`.
    Hover,
    /// `&'static [DocumentationExample]` — code with a carrier and notes.
    DocumentationExamples,
    /// `&'static [SubCommand]` — edited with the subcommand schema.
    SubCommands,
    /// `&'static [SubSubCommand]`.
    SubSubCommands,
    /// `Option<&'static ObjectClassSpec>` — a class name, its superclasses,
    /// an unknown-method flag, and an instance-method table edited with the
    /// subcommand schema.
    ObjectClass,
    /// `Option<TkGeometryManagerSpec>` — static geometry-container and
    /// placement semantics, edited field by field.
    TkGeometry,
    /// `Option<&'static ClauseGrammarSpec>` — a clause chain's word grammar:
    /// the head, the keyword and keywordless rows, the tail, and the chain's
    /// fall-through, default and selection rules. Plain data, carried field by
    /// field and shown read-only in the form.
    ClauseGrammar,
    /// `Option<&'static DefinitionBodyGrammar>` — a definer's body grammar:
    /// the name of the shipped grammar it is (`tcloo`, `snit`, …), or the
    /// whole grammar, every member row stating its effect. Plain data; a
    /// shipped name is picked, an inline grammar is shown read-only.
    DefinitionBody,
    /// `Option<SemanticOperationId>` — the target-neutral operation identity,
    /// a closed vocabulary: `invoke`, an intrinsic, or a structured lowering.
    SemanticOperation,
    /// [`tcl_registry::RuntimeBacking`] — how the command's behaviour reaches
    /// the runtime, held as the statement's own spelling (`none`,
    /// `host-native`, `shipped-builtin ID`, `tcl-body {-package-source PATH
    /// ?-evaluate?}`, `tcl-body {-pack-text {TEXT} ?-evaluate?}`) and edited as
    /// text.
    RuntimeBacking,
    /// A field the studio cannot model as data — a function pointer or a
    /// reference to a `&'static` descriptor. Held (and emitted) as a verbatim
    /// Rust expression the author supplies.
    RustExpr {
        /// Placeholder showing the expression shape expected.
        hint: &'static str,
    },
}

impl FieldKind {
    /// The wire tag the front-end switches on.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::OptBool => "optBool",
            Self::Count => "count",
            Self::OptCount => "optCount",
            Self::OptIndex => "optIndex",
            // A backing is edited as text: its draft value is the statement's
            // own spelling.
            Self::Text | Self::RuntimeBacking => "text",
            Self::OptText => "optText",
            Self::Prose => "prose",
            Self::TextList => "textList",
            Self::IndexList => "indexList",
            Self::OptIndexList => "optIndexList",
            Self::VariableReceivers => "variableReceivers",
            Self::Enum { .. } => "enum",
            Self::FlagSet { .. } => "flagSet",
            Self::EnumList { .. } => "enumList",
            Self::Arity => "arity",
            Self::BodyInterpreter => "bodyInterpreter",
            Self::RoleMap => "roleMap",
            Self::PrefixMap => "prefixMap",
            Self::CallbackTaintMap => "callbackTaintMap",
            Self::PresentationMap => "presentationMap",
            Self::ArgTypeMap => "argTypeMap",
            Self::ArgValueMap => "argValueMap",
            Self::Options => "options",
            Self::Forms => "forms",
            Self::Refinements => "refinements",
            Self::SideEffects => "sideEffects",
            Self::Knowledge => "knowledge",
            Self::SetterConstraints => "setterConstraints",
            Self::ManufacturerMethods => "manufacturerMethods",
            Self::Hover => "hover",
            Self::DocumentationExamples => "documentationExamples",
            Self::SubCommands => "subCommands",
            Self::SubSubCommands => "subSubCommands",
            Self::ObjectClass => "objectClass",
            Self::TkGeometry => "tkGeometry",
            Self::ClauseGrammar => "clauseGrammar",
            Self::DefinitionBody => "definitionBody",
            Self::SemanticOperation => "semanticOperation",
            Self::RustExpr { .. } => "rustExpr",
        }
    }

    /// The JSON description the front-end renders an editor from.
    #[must_use]
    pub fn to_json(self) -> Value {
        let mut out = json!({ "tag": self.tag() });
        match self {
            Self::Enum {
                catalogue,
                optional,
            }
            | Self::FlagSet {
                catalogue,
                optional,
            } => {
                out["catalogue"] = json!(catalogue);
                out["optional"] = json!(optional);
            }
            Self::EnumList { catalogue } => out["catalogue"] = json!(catalogue),
            Self::RustExpr { hint } => out["hint"] = json!(hint),
            _ => {}
        }
        out
    }
}

/// One editable field of `CommandSpec` or `SubCommand`.
#[derive(Debug, Clone, Copy)]
pub struct FieldSchema {
    /// The Rust field name — also the draft's JSON key and the identifier the
    /// `.rs` renderer emits.
    pub key: &'static str,
    /// Human label shown above the editor.
    pub label: &'static str,
    /// One-line help, drawn from the field's doc comment.
    pub doc: &'static str,
    /// Group heading the field is filed under.
    pub group: &'static str,
    /// How the field is edited and emitted.
    pub kind: FieldKind,
}

/// One editable property nested below a top-level spec field.
///
/// Nested properties use the same help/example pipeline as `CommandSpec`
/// fields so composite editors and the Reference tab cannot document them
/// differently.
#[derive(Debug, Clone, Copy)]
pub struct NestedFieldSchema {
    /// Rust field name on the nested registry type.
    pub key: &'static str,
    /// Human label shown beside the composite control.
    pub label: &'static str,
    /// One-line meaning shown in the Reference row.
    pub doc: &'static str,
    /// Owning registry type.
    pub owner: &'static str,
    /// Top-level `COMMAND_FIELDS` / `SUBCOMMAND_FIELDS` key whose editor this
    /// property is edited inside.
    ///
    /// The owning *type* cannot be navigated to; the form row can. A surface
    /// that links a nested key — the studio's dock does — needs the row, and
    /// the schema is where every other "where is this edited" answer lives.
    pub field: &'static str,
    /// Form group containing the composite editor.
    pub group: &'static str,
}

impl NestedFieldSchema {
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "key": self.key,
            "label": self.label,
            "doc": self.doc,
            "owner": self.owner,
            "field": self.field,
            "group": self.group,
            "help": crate::help::field_help(self.key).unwrap_or(self.doc),
            "example": crate::examples::field_example(self.key, self.label),
            // Which other settings this one is read with, so the help surface
            // can link them rather than describe the field as standalone.
            "related": crate::relations::to_json(self.key),
        })
    }
}

/// Properties edited inside composite field rows rather than as top-level
/// `CommandSpec` controls.
pub const NESTED_FIELDS: &[NestedFieldSchema] = &[
    NestedFieldSchema {
        key: "taints_var_write",
        label: "External input link",
        doc: "Whether this VarWrite option links a variable that external input can update.",
        owner: "OptionArg",
        field: "options",
        group: "Options and values",
    },
    NestedFieldSchema {
        key: "variable_scope",
        label: "Variable scope",
        doc: "Where an unqualified variable-name option value resolves.",
        owner: "OptionArg",
        field: "options",
        group: "Options and values",
    },
    NestedFieldSchema {
        key: "script_timing",
        label: "Script timing",
        doc: "Whether this script option runs now or is stored for a later callback.",
        owner: "OptionArg",
        field: "options",
        group: "Options and values",
    },
    NestedFieldSchema {
        key: "callback_taint_inputs",
        label: "Callback external inputs",
        doc: "User-controlled callback substitutions that must be treated as taint sources.",
        owner: "OptionArg",
        field: "options",
        group: "Options and values",
    },
    NestedFieldSchema {
        key: "method_prefix_matching",
        label: "Method prefix matching",
        doc: "Whether this object's instance methods accept unique-prefix abbreviations.",
        owner: "ObjectClassSpec",
        field: "object_class",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "container_policy",
        label: "Container policy",
        doc: "Whether the geometry manager claims exclusive ownership of a container.",
        owner: "TkGeometryManagerSpec",
        field: "tk_geometry",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "container_option",
        label: "Container option",
        doc: "Option whose value overrides the widget pathname's parent container.",
        owner: "TkGeometryManagerSpec",
        field: "tk_geometry",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "direct_form",
        label: "Direct placement form",
        doc: "Whether the manager's no-subcommand form places widgets.",
        owner: "TkGeometryManagerSpec",
        field: "tk_geometry",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "placement_subcommand",
        label: "Placement subcommand",
        doc: "Subcommand that places or reconfigures widgets.",
        owner: "TkGeometryManagerSpec",
        field: "tk_geometry",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "release_subcommands",
        label: "Release subcommands",
        doc: "Subcommands that stop managing their widget arguments.",
        owner: "TkGeometryManagerSpec",
        field: "tk_geometry",
        group: "Advanced",
    },
    NestedFieldSchema {
        key: "route",
        label: "Evaluation route",
        doc: "How the declaration computes its answer: none, a shipped direct evaluator, the shared expression engine, or a declared implementation.",
        owner: "DeclaredSemantics",
        field: "semantics",
        group: HOOKS,
    },
    NestedFieldSchema {
        key: "body",
        label: "Implementation body",
        doc: "The declared implementation's Tcl body, run in the bounded host under its declared inputs and budget.",
        owner: "DeclaredSemantics",
        field: "semantics",
        group: HOOKS,
    },
];

impl FieldSchema {
    /// The JSON the front-end builds its editor from.
    ///
    /// `help` is the long-form text behind the field's **?** button, from
    /// [`crate::help::field_help`]; the one-line `doc` stays under the label.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "key": self.key,
            "label": self.label,
            "doc": self.doc,
            "group": self.group,
            "help": crate::help::field_help(self.key).unwrap_or(self.doc),
            "example": crate::examples::field_example(self.key, self.label),
            // Which other settings this one is read with, so the help surface
            // can link them rather than describe the field as standalone.
            "related": crate::relations::to_json(self.key),
            "kind": self.kind.to_json(),
        })
    }
}

const fn f(
    key: &'static str,
    label: &'static str,
    group: &'static str,
    kind: FieldKind,
    doc: &'static str,
) -> FieldSchema {
    FieldSchema {
        key,
        label,
        doc,
        group,
        kind,
    }
}

/// Group headings, in the order the form presents them.
pub const GROUPS: &[&str] = &[
    "Identity",
    "Availability",
    "Arity and arguments",
    "Types",
    "Subcommands",
    "Documentation",
    "Options and values",
    "Behaviour",
    "Side effects",
    "Compiler hooks",
    "Taint and security",
    "Deprecation and translation",
    "Advanced",
];

const IDENTITY: &str = "Identity";
const AVAILABILITY: &str = "Availability";
const ARGS: &str = "Arity and arguments";
const TYPES: &str = "Types";
const SUBS: &str = "Subcommands";
const DOCS: &str = "Documentation";
const OPTS: &str = "Options and values";
const BEHAVIOUR: &str = "Behaviour";
const EFFECTS: &str = "Side effects";
const HOOKS: &str = "Compiler hooks";
const TAINT: &str = "Taint and security";
const DEPRECATION: &str = "Deprecation and translation";
const ADVANCED: &str = "Advanced";

/// The whole schema — catalogues, group order, both field tables, and the
/// long-form help behind the form's **?** buttons and the Reference tab.
#[must_use]
pub fn to_json() -> Value {
    fn fields(list: &[FieldSchema]) -> Value {
        Value::Array(list.iter().map(FieldSchema::to_json).collect())
    }
    let group_help: serde_json::Map<String, Value> = crate::help::GROUP_HELP
        .iter()
        .map(|(group, text)| ((*group).to_owned(), json!(text)))
        .collect();
    let group_examples: serde_json::Map<String, Value> = GROUPS
        .iter()
        .filter_map(|group| {
            crate::examples::group_example(group).map(|example| ((*group).to_owned(), example))
        })
        .collect();
    let catalogue_help: serde_json::Map<String, Value> = crate::help::CATALOGUE_HELP
        .iter()
        .map(|(id, title, intro)| {
            (
                (*id).to_owned(),
                json!({
                    "title": title,
                    "intro": intro,
                    "example": crate::examples::catalogue_example(id, title),
                }),
            )
        })
        .collect();
    json!({
        "groups": GROUPS,
        "groupHelp": group_help,
        "groupExamples": group_examples,
        "catalogues": catalogues(),
        "catalogueHelp": catalogue_help,
        "nestedFields": Value::Array(NESTED_FIELDS.iter().map(NestedFieldSchema::to_json).collect()),
        "command": fields(COMMAND_FIELDS),
        "subcommand": fields(SUBCOMMAND_FIELDS),
    })
}

/// Look up a command field by its Rust name.
#[must_use]
pub fn command_field(key: &str) -> Option<&'static FieldSchema> {
    COMMAND_FIELDS.iter().find(|f| f.key == key)
}

/// Look up a subcommand field by its Rust name.
#[must_use]
pub fn subcommand_field(key: &str) -> Option<&'static FieldSchema> {
    SUBCOMMAND_FIELDS.iter().find(|f| f.key == key)
}

#[cfg(test)]
mod tests;
