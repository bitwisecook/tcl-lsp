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

use crate::catalogue;

/// How the studio edits (and the renderer emits) one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Plain `bool`.
    Bool,
    /// `Option<bool>` — unset / true / false.
    OptBool,
    /// A non-negative count (`u8` / `usize`).
    Count,
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
    /// [`tcl_registry::Arity`] — min, max, step, and the extra exact count.
    Arity,
    /// `&'static [(u8, ArgRole)]`.
    RoleMap,
    /// `&'static [(u8, AppendedArity)]` — command-prefix positions.
    PrefixMap,
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
    /// `&'static [SideEffect]`.
    SideEffects,
    /// `&'static [SetterConstraint]`.
    SetterConstraints,
    /// `&'static [ManufacturerMethod]` — class-command instance factories.
    ManufacturerMethods,
    /// `Option<HoverSnippet>`.
    Hover,
    /// `&'static [SubCommand]` — edited with the subcommand schema.
    SubCommands,
    /// `&'static [SubSubCommand]`.
    SubSubCommands,
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
            Self::OptIndex => "optIndex",
            Self::Text => "text",
            Self::OptText => "optText",
            Self::Prose => "prose",
            Self::TextList => "textList",
            Self::IndexList => "indexList",
            Self::OptIndexList => "optIndexList",
            Self::Enum { .. } => "enum",
            Self::FlagSet { .. } => "flagSet",
            Self::Arity => "arity",
            Self::RoleMap => "roleMap",
            Self::PrefixMap => "prefixMap",
            Self::PresentationMap => "presentationMap",
            Self::ArgTypeMap => "argTypeMap",
            Self::ArgValueMap => "argValueMap",
            Self::Options => "options",
            Self::Forms => "forms",
            Self::SideEffects => "sideEffects",
            Self::SetterConstraints => "setterConstraints",
            Self::ManufacturerMethods => "manufacturerMethods",
            Self::Hover => "hover",
            Self::SubCommands => "subCommands",
            Self::SubSubCommands => "subSubCommands",
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

/// Every editable field of `CommandSpec`, in `CommandSpec::DEFAULT` order.
pub const COMMAND_FIELDS: &[FieldSchema] = &[
    f(
        "name",
        "Command name",
        IDENTITY,
        FieldKind::Text,
        "Command name as written in Tcl — `for`, `dict`, `HTTP::header`.",
    ),
    f(
        "traits",
        "Traits",
        BEHAVIOUR,
        FieldKind::FlagSet {
            catalogue: "traits",
            optional: false,
        },
        "Behavioural trait flags every consumer reads instead of naming the command.",
    ),
    f(
        "dialects",
        "Dialects",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialects the command exists in. Unset means every dialect.",
    ),
    f(
        "arity",
        "Arity",
        ARGS,
        FieldKind::Arity,
        "Argument-count constraint, counted after the command name.",
    ),
    f(
        "arg_roles",
        "Argument roles",
        ARGS,
        FieldKind::RoleMap,
        "Static role per 0-based argument index, for fixed-layout commands.",
    ),
    f(
        "arg_role_resolver",
        "Argument-role resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_resolver)",
        },
        "Callback assigning roles from the actual argument list; wins over `arg_roles`.",
    ),
    f(
        "arg_presentation",
        "Argument presentation",
        ARGS,
        FieldKind::PresentationMap,
        "Formatter layout override per 0-based argument index; body arguments are block-expanded unless declared inline.",
    ),
    f(
        "repeated_args",
        "Repeated argument layouts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[RepeatedArgLayout::strided(ArgRole::VarWrite, 0, 2)]",
        },
        "Roles that recur at a fixed stride over the argument tail (`global a b c`, `variable n v n v`).",
    ),
    f(
        "frame_effect",
        "Frame effect",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(frame_effect::UPVAR)",
        },
        "How the command crosses stack frames: level word, frame-selected variable args, and caller-frame scripts.",
    ),
    f(
        "clause_shape_check",
        "Clause-shape checker",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(clause_shape::if_chain)",
        },
        "Validator for a clause chain whose shapes are not a single min..=max range.",
    ),
    f(
        "command_prefixes",
        "Command-prefix positions",
        ARGS,
        FieldKind::PrefixMap,
        "Argument indices carrying a callback command prefix, with the arity appended to it.",
    ),
    f(
        "command_prefix_resolver",
        "Command-prefix resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_prefix_resolver)",
        },
        "Callback locating command-prefix positions that depend on the argument list.",
    ),
    f(
        "return_type",
        "Return type",
        TYPES,
        FieldKind::Enum {
            catalogue: "tclType",
            optional: true,
        },
        "The intrep of the value `[cmd …]` yields.",
    ),
    f(
        "var_write_typing",
        "Variable-write typing",
        TYPES,
        FieldKind::RustExpr {
            hint: "VarWriteTyping::Fixed(TclType::String)",
        },
        "How the command types the variables it writes, when that differs from its return value.",
    ),
    f(
        "return_elements",
        "Return elements",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(ReturnElements::ListOfArgs { from: 0 })",
        },
        "How the result relates to container element structure.",
    ),
    f(
        "var_elements_effect",
        "Variable element effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(VarElementsEffect::AppendsListElements { values_from: 1 })",
        },
        "How the command evolves the container elements of the variable it writes in place.",
    ),
    f(
        "representation_effect",
        "Representation effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(RepresentationEffect::copy_on_write_container(0, 2))",
        },
        "Effect on Tcl's dual string/internal representation or shared-object storage.",
    ),
    f(
        "arg_types",
        "Argument type hints",
        TYPES,
        FieldKind::ArgTypeMap,
        "Expected intrep, shimmer risk, and transparent source types per argument index.",
    ),
    f(
        "subcommands",
        "Subcommands",
        SUBS,
        FieldKind::SubCommands,
        "Ensemble subcommands, each a self-contained metadata bundle.",
    ),
    f(
        "allow_unknown_subcommands",
        "Allow unknown subcommands",
        SUBS,
        FieldKind::Bool,
        "Accept a subcommand word that is not declared, without a W001 warning.",
    ),
    f(
        "prefix_matching",
        "Prefix matching",
        SUBS,
        FieldKind::Enum {
            catalogue: "prefixMatching",
            optional: false,
        },
        "Whether this command's keyword tables accept unique-prefix \
         abbreviations (`Tcl_GetIndexFromObj`) or only exact spellings.",
    ),
    f(
        "default_form_first_word",
        "Default-form first word",
        SUBS,
        FieldKind::Enum {
            catalogue: "defaultFormFirstWord",
            optional: true,
        },
        "The value shape a non-subcommand first word may take (`after 200 …`).",
    ),
    f(
        "hover",
        "Hover documentation",
        DOCS,
        FieldKind::Hover,
        "Summary, synopsis lines, prose, source, examples, and return value.",
    ),
    f(
        "forms",
        "Invocation forms",
        DOCS,
        FieldKind::Forms,
        "Synopsis per invocation form, for completion and arity-dependent lookup.",
    ),
    f(
        "command_forms",
        "Structured command forms",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "COMMAND_FORMS",
        },
        "Per-form arity, roles, options, and hooks, for form-specific routing.",
    ),
    f(
        "semantic_operation",
        "Semantic operation",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(SemanticOperationId::Intrinsic(IntrinsicId::ListLength))",
        },
        "Target-neutral operation identity selected before backend dispatch.",
    ),
    f(
        "completion",
        "Completion contract",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(CompletionDescriptor::exact(COMPLETION_CODES))",
        },
        "Possible Tcl completion codes and result/options payload obligations.",
    ),
    f(
        "assigns_variable_at",
        "Assigns variable at",
        ARGS,
        FieldKind::OptIndex,
        "Legacy shorthand: the single argument index naming a variable the command writes.",
    ),
    f(
        "safe_on_uninit",
        "Safe on uninitialised",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialects where the command safely initialises an uninitialised variable.",
    ),
    f(
        "const_fold",
        "Constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_length)",
        },
        "Compile-time folder returning the command's constant result.",
    ),
    f(
        "const_fold_versioned",
        "Versioned constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_format)",
        },
        "Tcl-version-aware folder; takes priority over the plain folder.",
    ),
    f(
        "lowering_hook",
        "Lowering hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "loweringHook",
            optional: true,
        },
        "Per-command lowering specialisation in the compiler's dispatch table.",
    ),
    f(
        "codegen_hook",
        "Bytecode codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "codegenHook",
            optional: true,
        },
        "Per-command TclVM bytecode emitter. Unset uses the generic invoke emitter.",
    ),
    f(
        "inline_codegen_hook",
        "Inline codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "inlineCodegenHook",
            optional: true,
        },
        "Emitter for the value-position and catch-body paths.",
    ),
    f(
        "bpf_op",
        "BPF-Tcl lowering descriptor",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(&bpf_op::XDP_PASS)",
        },
        "Typed BPF-Tcl lowering descriptor; the BPF front-end dispatches on this, never on the command name.",
    ),
    f(
        "analyser_hook",
        "Analyser hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "analyserHook",
            optional: true,
        },
        "Per-command handler family in the analyser's central dispatch.",
    ),
    f(
        "command_table_effect",
        "Command-table effect",
        EFFECTS,
        FieldKind::Enum {
            catalogue: "commandTableEffect",
            optional: true,
        },
        "How the command mutates the interpreter's command table.",
    ),
    f(
        "side_effects",
        "Side effects",
        EFFECTS,
        FieldKind::SideEffects,
        "Structured declarations of the state the command reads and writes.",
    ),
    f(
        "world_effects",
        "World effects",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(WorldEffectDescriptor::EMPTY)",
        },
        "Target-neutral mutable-world footprint for common compiler analysis.",
    ),
    f(
        "state_transitions",
        "State transitions",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(StateTransitionDescriptor::EMPTY)",
        },
        "Target-neutral command, namespace, interpreter, trace, and alias transitions.",
    ),
    f(
        "dispatch_dependencies",
        "Dispatch dependencies",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(DispatchDependencyDescriptor::replace(DispatchDependencies::BASE))",
        },
        "Mutable Tcl domains that must remain stable before specialisation.",
    ),
    f(
        "result_stability",
        "Result stability",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(ResultStability::ReferentiallyTransparent)",
        },
        "Whether repeated calls return the same value, or depend on mutable or volatile state.",
    ),
    f(
        "literal_argument_validator",
        "Literal argument validator",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(validate_literal_arguments)",
        },
        "Registry callback for relationships and member sets within statically-known arguments.",
    ),
    f(
        "inferred_storage_type",
        "Inferred storage type",
        TYPES,
        FieldKind::Enum {
            catalogue: "storageType",
            optional: true,
        },
        "The container kind the target variable is inferred to hold.",
    ),
    f(
        "required_package",
        "Required package",
        AVAILABILITY,
        FieldKind::OptText,
        "Package that must be `require`d before the command is visible.",
    ),
    f(
        "excluded_events",
        "Excluded iRules events",
        AVAILABILITY,
        FieldKind::TextList,
        "iRules events the command is not valid in.",
    ),
    f(
        "unsafe_command",
        "Unsafe command",
        BEHAVIOUR,
        FieldKind::Bool,
        "Allows context escalation in sandboxed dialects — drives IRULE2003.",
    ),
    f(
        "closed_value_args",
        "Closed value arguments",
        OPTS,
        FieldKind::IndexList,
        "Argument indices whose declared values are an exhaustive legal set (W127).",
    ),
    f(
        "event_requires",
        "Event requirements",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(EventRequires { .. })",
        },
        "Layer-based iRules event requirements used by the IRULE1001 validity check.",
    ),
    f(
        "event_requirement_forms",
        "Event requirement forms",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[EventRequirementForm { .. }]",
        },
        "Argument-prefix-specific iRules event contracts that override the command-level requirements.",
    ),
    f(
        "data_collection",
        "Data collection",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(HTTP_COLLECT)",
        },
        "Registry descriptor for a collect, release, or payload operation.",
    ),
    f(
        "side_switch_target",
        "Side-switch target",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(SideSwitchTarget::Client)",
        },
        "Connection-side context selected while evaluating this command's body.",
    ),
    f(
        "event_handler_priority",
        "Event-handler priority",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(BIGIP_EVENT_HANDLER_PRIORITY)",
        },
        "Runtime default and implicit-priority policy for an event-handler command.",
    ),
    f(
        "options",
        "Options",
        OPTS,
        FieldKind::Options,
        "Declared option flags, their values, roles, and dialect gates.",
    ),
    f(
        "option_constraints",
        "Option constraints",
        OPTS,
        FieldKind::RustExpr {
            hint: "&[OptionConstraint { options: &[\"-a\", \"-b\"], dialects: None }]",
        },
        "Registry-declared sets of leading options that may not occur together.",
    ),
    f(
        "reserved_trailing_words",
        "Reserved trailing words",
        OPTS,
        FieldKind::Count,
        "Trailing words C Tcl's own option scan never treats as option candidates.",
    ),
    f(
        "arg_values",
        "Argument values",
        OPTS,
        FieldKind::ArgValueMap,
        "Enumerable positional values per argument index, for completion.",
    ),
    f(
        "body_kind",
        "Body kind",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "bodyKind",
            optional: false,
        },
        "Whether body arguments run in the caller's frame or a separate context.",
    ),
    f(
        "body_arg_implicit_args",
        "Body implicit arguments",
        ARGS,
        FieldKind::Count,
        "Runtime-supplied positional arguments the body's first command receives.",
    ),
    f(
        "taint_output_sink",
        "Output-sink code",
        TAINT,
        FieldKind::OptText,
        "Diagnostic code emitted when tainted data reaches the output position (`T101`).",
    ),
    f(
        "taint_output_sink_subcommands",
        "Output-sink subcommands",
        TAINT,
        FieldKind::TextList,
        "Restricts the output sink to these subcommands. Empty means every invocation.",
    ),
    f(
        "taint_log_sink",
        "Log-sink code",
        TAINT,
        FieldKind::OptText,
        "Log-injection sink diagnostic code (`IRULE3003`).",
    ),
    f(
        "taint_network_sink_args",
        "Network-sink arguments",
        TAINT,
        FieldKind::OptIndexList,
        "Argument indices taking a network address — SSRF sinks.",
    ),
    f(
        "taint_code_sink_args",
        "Code-sink arguments",
        TAINT,
        FieldKind::OptIndexList,
        "The specific slots where a tainted value reaches eval-style evaluation.",
    ),
    f(
        "taint_interp_eval_subcommands",
        "Cross-interpreter eval subcommands",
        TAINT,
        FieldKind::TextList,
        "Subcommands evaluating code in another interpreter (T105).",
    ),
    f(
        "taint_source",
        "Taint source colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits the return value carries when the command is a taint source.",
    ),
    f(
        "taint_transform",
        "Taint transform colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits the command adds to a tainted value it returns.",
    ),
    f(
        "taint_double_encode_colour",
        "Double-encode colour",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Input colour whose presence means this command would double-encode (T106).",
    ),
    f(
        "taint_sink_safe_colour",
        "Sink-safe colour",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour that suppresses the dangerous-sink warning for this sink.",
    ),
    f(
        "taint_sink_gate",
        "Taint-sink gate",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(subst_evaluates_commands)",
        },
        "Predicate over the call's own flags deciding whether the sink applies.",
    ),
    f(
        "credential_options",
        "Credential options",
        TAINT,
        FieldKind::TextList,
        "Option flags whose value carries a secret (W310).",
    ),
    f(
        "sensitive_headers",
        "Sensitive headers",
        TAINT,
        FieldKind::TextList,
        "HTTP header names whose values are secrets.",
    ),
    f(
        "setter_constraints",
        "Setter constraints",
        TAINT,
        FieldKind::SetterConstraints,
        "Required argument prefixes on setter forms (IRULE3101).",
    ),
    f(
        "pattern_type",
        "Pattern type",
        OPTS,
        FieldKind::Enum {
            catalogue: "patternType",
            optional: true,
        },
        "The pattern language the command's pattern argument uses.",
    ),
    f(
        "format_string_type",
        "Format-string type",
        OPTS,
        FieldKind::Enum {
            catalogue: "formatType",
            optional: true,
        },
        "The format-string language the command's format argument uses.",
    ),
    f(
        "tcllib_package",
        "Tcllib package",
        AVAILABILITY,
        FieldKind::OptText,
        "Tcllib package providing the command, for per-document activation.",
    ),
    f(
        "introduced_version",
        "Introduced in",
        AVAILABILITY,
        FieldKind::OptText,
        "Dotted version of the owning package that introduced the command.",
    ),
    f(
        "deprecated_version",
        "Deprecated in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version where the command still exists but should warn.",
    ),
    f(
        "retired_version",
        "Retired in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version without the command (exclusive upper bound).",
    ),
    f(
        "deprecation_fix",
        "Deprecation quick-fix hook",
        AVAILABILITY,
        FieldKind::RustExpr {
            hint: "Some(DeprecationFixHook::ReplaceMatchedWord { replacement: \"new\", description: \"Use new\", safety: DeprecationFixSafety::SemanticsEquivalent })",
        },
        "Registry-owned edit plan or contextual callback for replacing deprecated syntax.",
    ),
    f(
        "warn_missing_import",
        "Warn on missing import",
        AVAILABILITY,
        FieldKind::Bool,
        "Whether W120 fires when the command is used without a `package require`.",
    ),
    f(
        "is_namespace_exported",
        "Namespace exported",
        AVAILABILITY,
        FieldKind::Bool,
        "Whether the source namespace exports the bare name.",
    ),
    f(
        "xc_translatable",
        "XC translatable",
        DEPRECATION,
        FieldKind::OptBool,
        "Cross-compile translatability override. Unset uses the default rules.",
    ),
    f(
        "xc_operation",
        "XC operation",
        DEPRECATION,
        FieldKind::OptText,
        "The XC operation the command maps to when translatable.",
    ),
    f(
        "deprecated_replacement",
        "Deprecated replacement",
        DEPRECATION,
        FieldKind::OptText,
        "Replacement command name surfaced by the deprecation code action.",
    ),
    f(
        "deprecated_replacement_drop_in",
        "Replacement is drop-in",
        DEPRECATION,
        FieldKind::Bool,
        "Whether the replacement accepts the deprecated argument list unchanged.",
    ),
    f(
        "byte_array_payload",
        "Byte-array payload",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(BytePayloadSpec::DEFAULT)",
        },
        "`<proto>::payload` layout for the S110 byte-array-corruption check.",
    ),
    f(
        "byte_array_effect",
        "Byte-array effect",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "byteArrayEffect",
            optional: false,
        },
        "How the command transforms a byte-array operand it derives its result from.",
    ),
    f(
        "definition_body",
        "Definition-body grammar",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&definer::TCLOO_CLASS_BODY)",
        },
        "Body grammar for a class or type definer, so the generic walker can recurse.",
    ),
    f(
        "manufacturer_methods",
        "Manufacturer methods",
        ADVANCED,
        FieldKind::ManufacturerMethods,
        "Class-command methods that create an instance, including their name, body, and constructor-argument positions.",
    ),
    f(
        "case_list",
        "Case list",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&CaseListSpec::SWITCH)",
        },
        "The `{pattern body …}` clause list the command takes as its final braced word.",
    ),
    f(
        "oo_context_facts",
        "TclOO context facts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[(\"class\", OoContextFact::DefiningClass)]",
        },
        "Keyword words whose value the enclosing TclOO method frame fixes, so the optimiser can fold them.",
    ),
    f(
        "self_receiver_words",
        "TclOO self-receiver words",
        ADVANCED,
        FieldKind::TextList,
        "Argument-0 closed words for which a bracketed `[cmd ?word?]` dispatch head denotes the current TclOO receiving object, same target as `my` (e.g. `self`'s `object`).",
    ),
    f(
        "object_class",
        "Object class",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&MY_CLASS)",
        },
        "Class metadata for a factory whose `new`/`create` returns a dispatchable handle.",
    ),
    f(
        "defines_symbol",
        "Defines symbol",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(SymbolDef { .. })",
        },
        "Descriptor for an argument binding a name the document outline should list.",
    ),
    f(
        "body_scope",
        "Body scope",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&scoped::REPORT_DEFSTYLE)",
        },
        "Extra commands available only inside the command's body argument.",
    ),
    f(
        "binds_handle",
        "Binds handle",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&handle_binding::SET_BINDS_HANDLE)",
        },
        "Which argument becomes an object handle, and which says its class.",
    ),
    f(
        "creates_instance_at",
        "Creates instance at",
        SUBS,
        FieldKind::OptIndex,
        "Argument index naming an object command of this spec's own object class.",
    ),
    f(
        "defines_command_at",
        "Defines command at",
        SUBS,
        FieldKind::OptIndex,
        "Argument index whose literal value becomes a callable command name.",
    ),
    f(
        "context_gate",
        "Context gate",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(irules_return_gate)",
        },
        "Validity gate keyed on lexical or dispatch context rather than argument shape.",
    ),
    f(
        "implementation_namespace",
        "Implementation namespace",
        SUBS,
        FieldKind::OptText,
        "Namespace the ensemble's subcommands are also individually callable under.",
    ),
];

/// Every editable field of `SubCommand`, in `SubCommand::DEFAULT` order.
pub const SUBCOMMAND_FIELDS: &[FieldSchema] = &[
    f(
        "name",
        "Subcommand name",
        IDENTITY,
        FieldKind::Text,
        "Subcommand word — `create`, `length`, `for`.",
    ),
    f(
        "traits",
        "Traits",
        BEHAVIOUR,
        FieldKind::FlagSet {
            catalogue: "traits",
            optional: false,
        },
        "Subcommand-shaped trait bits; they compose with the command's own.",
    ),
    f(
        "arity",
        "Arity",
        ARGS,
        FieldKind::Arity,
        "Argument-count constraint, counted after the subcommand word.",
    ),
    f(
        "detail",
        "Detail",
        DOCS,
        FieldKind::Text,
        "Short description for the completion list.",
    ),
    f(
        "synopsis",
        "Synopsis",
        DOCS,
        FieldKind::Text,
        "Invocation synopsis.",
    ),
    f(
        "hover",
        "Hover documentation",
        DOCS,
        FieldKind::Hover,
        "Summary, synopsis lines, prose, source, examples, and return value.",
    ),
    f(
        "arg_roles",
        "Argument roles",
        ARGS,
        FieldKind::RoleMap,
        "Static role per index, counted after the subcommand word.",
    ),
    f(
        "arg_role_resolver",
        "Argument-role resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_resolver)",
        },
        "Callback assigning roles from the actual argument list.",
    ),
    f(
        "arg_presentation",
        "Argument presentation",
        ARGS,
        FieldKind::PresentationMap,
        "Formatter layout override per index, counted after the subcommand word.",
    ),
    f(
        "repeated_args",
        "Repeated argument layouts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[RepeatedArgLayout::strided(ArgRole::VarWrite, 1, 2)]",
        },
        "Roles that recur at a fixed stride over the tail, counted after the subcommand word.",
    ),
    f(
        "command_prefixes",
        "Command-prefix positions",
        ARGS,
        FieldKind::PrefixMap,
        "Callback command-prefix indices after the subcommand word, with appended arity.",
    ),
    f(
        "command_prefix_resolver",
        "Command-prefix resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_prefix_resolver)",
        },
        "Callback locating command-prefix positions after the subcommand word.",
    ),
    f(
        "return_type",
        "Return type",
        TYPES,
        FieldKind::Enum {
            catalogue: "tclType",
            optional: true,
        },
        "The intrep of the value the subcommand yields.",
    ),
    f(
        "var_write_typing",
        "Variable-write typing",
        TYPES,
        FieldKind::RustExpr {
            hint: "VarWriteTyping::Destructured",
        },
        "Overrides the command's variable-write typing when this subcommand matches.",
    ),
    f(
        "return_elements",
        "Return elements",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(ReturnElements::ElementOf { container: 0 })",
        },
        "Result-to-element-structure fact for this subcommand.",
    ),
    f(
        "var_elements_effect",
        "Variable element effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(VarElementsEffect::SetsDictValue)",
        },
        "In-place element evolution of the written variable for this subcommand.",
    ),
    f(
        "representation_effect",
        "Representation effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(RepresentationEffect::copy_on_write_container(0, 2))",
        },
        "Subcommand-specific Tcl representation or shared-object storage effect.",
    ),
    f(
        "arg_types",
        "Argument type hints",
        TYPES,
        FieldKind::ArgTypeMap,
        "Expected intrep and shimmer risk per argument index.",
    ),
    f(
        "pure",
        "Pure",
        BEHAVIOUR,
        FieldKind::Bool,
        "Side-effect free.",
    ),
    f(
        "mutator",
        "Mutator",
        BEHAVIOUR,
        FieldKind::Bool,
        "Mutates state.",
    ),
    f(
        "const_fold",
        "Constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_length)",
        },
        "Compile-time folder for this subcommand.",
    ),
    f(
        "const_fold_versioned",
        "Versioned constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_is)",
        },
        "Tcl-version-aware folder; takes priority over the plain folder.",
    ),
    f(
        "lowering_hook",
        "Lowering hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "loweringHook",
            optional: true,
        },
        "Lowering specialisation for this subcommand.",
    ),
    f(
        "codegen_hook",
        "Bytecode codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "codegenHook",
            optional: true,
        },
        "TclVM bytecode emitter for this subcommand.",
    ),
    f(
        "inline_codegen_hook",
        "Inline codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "inlineCodegenHook",
            optional: true,
        },
        "Value-position emitter, overriding the command's when this subcommand matches.",
    ),
    f(
        "analyser_hook",
        "Analyser hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "analyserHook",
            optional: true,
        },
        "Analyser handler family, overriding the command's for this subcommand.",
    ),
    f(
        "command_table_effect",
        "Command-table effect",
        EFFECTS,
        FieldKind::Enum {
            catalogue: "commandTableEffect",
            optional: true,
        },
        "Command-table mutation for this subcommand (`interp alias`).",
    ),
    f(
        "options",
        "Options",
        OPTS,
        FieldKind::Options,
        "Option flags declared on this subcommand.",
    ),
    f(
        "option_constraints",
        "Option constraints",
        OPTS,
        FieldKind::RustExpr {
            hint: "&[OptionConstraint { options: &[\"-a\", \"-b\"], dialects: None }]",
        },
        "Subcommand-specific sets of leading options that may not occur together.",
    ),
    f(
        "min_abbrev",
        "Minimum abbreviation",
        OPTS,
        FieldKind::OptIndex,
        "Documented minimum abbreviation length for this subcommand's name, \
         when longer than uniqueness alone requires. Unset = uniqueness only.",
    ),
    f(
        "prefix_matching",
        "Prefix matching",
        OPTS,
        FieldKind::Enum {
            catalogue: "prefixMatching",
            optional: false,
        },
        "Whether this subcommand's own keyword tables accept unique-prefix \
         abbreviations or only exact spellings.",
    ),
    f(
        "arg_values",
        "Argument values",
        OPTS,
        FieldKind::ArgValueMap,
        "Enumerable values per index after the subcommand word.",
    ),
    f(
        "versioned_arg_values",
        "Versioned argument values",
        AVAILABILITY,
        FieldKind::RustExpr {
            hint: "VERSIONED_ARG_VALUES",
        },
        "Package-version gates for literal positional argument values.",
    ),
    f(
        "subcommand_forms",
        "Structured subcommand forms",
        ADVANCED,
        FieldKind::RustExpr { hint: "SUB_FORMS" },
        "Per-form arity, roles, options, and hooks matched after the subcommand word.",
    ),
    f(
        "semantic_operation",
        "Semantic operation",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(SemanticOperationId::Intrinsic(IntrinsicId::DictGet))",
        },
        "Target-neutral operation identity overriding the parent command.",
    ),
    f(
        "completion",
        "Completion contract",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(CompletionDescriptor::exact(COMPLETION_CODES))",
        },
        "Subcommand-specific Tcl completion and payload obligations.",
    ),
    f(
        "dialects",
        "Dialects",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialect membership. Unset inherits the parent command's set.",
    ),
    f(
        "introduced_version",
        "Introduced in",
        AVAILABILITY,
        FieldKind::OptText,
        "Dotted version of the owning package that introduced the subcommand.",
    ),
    f(
        "deprecated_version",
        "Deprecated in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version where the subcommand still exists but should warn.",
    ),
    f(
        "retired_version",
        "Retired in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version without the subcommand (exclusive upper bound).",
    ),
    f(
        "deprecation_fix",
        "Deprecation quick-fix hook",
        AVAILABILITY,
        FieldKind::RustExpr {
            hint: "Some(DeprecationFixHook::ReplaceMatchedWord { replacement: \"new\", description: \"Use new\", safety: DeprecationFixSafety::SemanticsEquivalent })",
        },
        "Registry-owned edit plan or contextual callback for replacing deprecated syntax.",
    ),
    f(
        "safe_on_uninit",
        "Safe on uninitialised",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialects where the subcommand safely initialises an uninitialised variable.",
    ),
    f(
        "loop_list_header",
        "Loop-list header",
        BEHAVIOUR,
        FieldKind::Bool,
        "A CFG header whose arguments are list expressions (`foreach` / `lmap`).",
    ),
    f(
        "creates_scope_alias",
        "Creates scope alias",
        BEHAVIOUR,
        FieldKind::Bool,
        "Creates an upvar-like binding.",
    ),
    f(
        "inferred_storage_type",
        "Inferred storage type",
        TYPES,
        FieldKind::Enum {
            catalogue: "storageType",
            optional: true,
        },
        "The container kind the target variable is inferred to hold.",
    ),
    f(
        "body_kind",
        "Body kind",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "bodyKind",
            optional: false,
        },
        "Whether body arguments run in the caller's frame or a separate context.",
    ),
    f(
        "byte_array_effect",
        "Byte-array effect",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "byteArrayEffect",
            optional: false,
        },
        "How the subcommand transforms a byte-array operand (S110).",
    ),
    f(
        "closed_value_args",
        "Closed value arguments",
        OPTS,
        FieldKind::IndexList,
        "Indices after the subcommand word whose values are an exhaustive set (W127).",
    ),
    f(
        "arg_values_accept_prefix",
        "Values accept a prefix",
        OPTS,
        FieldKind::Bool,
        "Whether a closed value is accepted as a unique prefix rather than an exact match.",
    ),
    f(
        "body_arg_implicit_args",
        "Body implicit arguments",
        ARGS,
        FieldKind::Count,
        "Runtime-supplied positional arguments the body's first command receives.",
    ),
    f(
        "taint_transform",
        "Taint transform colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits this subcommand adds to a tainted value it returns.",
    ),
    f(
        "taint_double_encode_colour",
        "Double-encode colour",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Input colour whose presence means this subcommand would double-encode (T106).",
    ),
    f(
        "taint_output_sink",
        "Output-sink code",
        TAINT,
        FieldKind::OptText,
        "Diagnostic code for a subcommand-shaped XSS or header-injection sink.",
    ),
    f(
        "credential_arg",
        "Credential argument",
        TAINT,
        FieldKind::OptIndex,
        "Index after the subcommand word carrying a credential value.",
    ),
    f(
        "sensitive_headers",
        "Sensitive headers",
        TAINT,
        FieldKind::TextList,
        "HTTP header names whose values are secrets.",
    ),
    f(
        "pattern_type",
        "Pattern type",
        OPTS,
        FieldKind::Enum {
            catalogue: "patternType",
            optional: true,
        },
        "Pattern-language override, taking priority over the command's.",
    ),
    f(
        "format_string_type",
        "Format-string type",
        OPTS,
        FieldKind::Enum {
            catalogue: "formatType",
            optional: true,
        },
        "Format-string override, taking priority over the command's.",
    ),
    f(
        "xc_operation",
        "XC operation",
        DEPRECATION,
        FieldKind::OptText,
        "The XC operation this subcommand maps to.",
    ),
    f(
        "side_effects",
        "Side effects",
        EFFECTS,
        FieldKind::SideEffects,
        "Structured side-effect declarations for this subcommand.",
    ),
    f(
        "world_effects",
        "World effects",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(WorldEffectDescriptor::EMPTY)",
        },
        "Subcommand-specific mutable-world footprint.",
    ),
    f(
        "state_transitions",
        "State transitions",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(StateTransitionDescriptor::EMPTY)",
        },
        "Subcommand-specific Tcl state transitions.",
    ),
    f(
        "dispatch_dependencies",
        "Dispatch dependencies",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(DispatchDependencyDescriptor::replace(DispatchDependencies::BASE))",
        },
        "Subcommand-specific live-dispatch stability requirements.",
    ),
    f(
        "result_stability",
        "Result stability",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(ResultStability::ReferentiallyTransparent)",
        },
        "Subcommand-specific result stability declaration.",
    ),
    f(
        "literal_argument_validator",
        "Literal argument validator",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(validate_literal_arguments)",
        },
        "Subcommand-specific registry callback for related literal arguments.",
    ),
    f(
        "destructive",
        "Destructive",
        BEHAVIOUR,
        FieldKind::Bool,
        "An irreversible operation (`file delete`).",
    ),
    f(
        "returns_path",
        "Returns a path",
        BEHAVIOUR,
        FieldKind::Bool,
        "Returns a filesystem path.",
    ),
    f(
        "is_unescape",
        "Unescapes",
        BEHAVIOUR,
        FieldKind::Bool,
        "Performs unescaping or decoding.",
    ),
    f(
        "cfg_rewrite_name",
        "CFG rewrite name",
        HOOKS,
        FieldKind::OptText,
        "Lowered command name for an ensemble subcommand the lowering pass rewrites.",
    ),
    f(
        "sub_subcommands",
        "Second-level subcommands",
        SUBS,
        FieldKind::SubSubCommands,
        "Operations selected by the word after this subcommand (`info object <op>`).",
    ),
    f(
        "defines_command_at",
        "Defines command at",
        SUBS,
        FieldKind::OptIndex,
        "Index after the subcommand word whose literal value becomes a command name.",
    ),
    f(
        "max_leading_option_words",
        "Max leading option words",
        OPTS,
        FieldKind::OptIndex,
        "Cap on leading option words consumed; further option-shaped words are positional.",
    ),
];

/// The variant catalogues the form's pickers read, keyed by catalogue id.
#[must_use]
pub fn catalogues() -> Value {
    fn entries(items: &[catalogue::Variant]) -> Value {
        Value::Array(
            items
                .iter()
                .map(|v| json!({ "key": v.key, "doc": v.doc }))
                .collect(),
        )
    }
    json!({
        "argRole": entries(catalogue::ARG_ROLES),
        "tclType": entries(catalogue::TCL_TYPES),
        "bodyKind": entries(catalogue::BODY_KINDS),
        "argPresentation": entries(catalogue::ARG_PRESENTATIONS),
        "storageType": entries(catalogue::STORAGE_TYPES),
        "byteArrayEffect": entries(catalogue::BYTE_ARRAY_EFFECTS),
        "commandTableEffect": entries(catalogue::COMMAND_TABLE_EFFECTS),
        "patternType": entries(catalogue::PATTERN_TYPES),
        "formatType": entries(catalogue::FORMAT_TYPES),
        "formKind": entries(catalogue::FORM_KINDS),
        "definedSymbolKind": entries(catalogue::DEFINED_SYMBOL_KINDS),
        "sideEffectTarget": entries(catalogue::SIDE_EFFECT_TARGETS),
        "connectionSide": entries(catalogue::CONNECTION_SIDES),
        "loweringHook": entries(catalogue::LOWERING_HOOKS),
        "codegenHook": entries(catalogue::CODEGEN_HOOKS),
        "inlineCodegenHook": entries(catalogue::INLINE_CODEGEN_HOOKS),
        "analyserHook": entries(catalogue::ANALYSER_HOOKS),
        "traits": entries(catalogue::TRAITS),
        "taintColour": entries(catalogue::TAINT_COLOURS),
        "dialects": entries(catalogue::DIALECTS),
        "defaultFormFirstWord": json!([
            { "key": "Integer", "doc": "an integer first word selects the default form" }
        ]),
        "prefixMatching": json!([
            { "key": "Enabled", "doc": "any unique prefix resolves (Tcl_GetIndexFromObj)" },
            { "key": "Strict", "doc": "only the exact spelling resolves (TCL_INDEX_STRICT)" }
        ]),
        "appendedArity": json!([
            { "key": "Exactly", "doc": "exactly N arguments are appended" },
            { "key": "AtLeast", "doc": "at least N arguments are appended" },
            { "key": "Unknown", "doc": "indeterminate — no arity check" }
        ]),
        "optionArity": json!([
            { "key": "One", "doc": "consumes one value word" },
            { "key": "Fixed", "doc": "consumes a fixed number of value words" }
        ]),
    })
}

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
    let catalogue_help: serde_json::Map<String, Value> = crate::help::CATALOGUE_HELP
        .iter()
        .map(|(id, title, intro)| ((*id).to_owned(), json!({ "title": title, "intro": intro })))
        .collect();
    json!({
        "groups": GROUPS,
        "groupHelp": group_help,
        "catalogues": catalogues(),
        "catalogueHelp": catalogue_help,
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
mod tests {
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
    fn every_named_catalogue_exists() {
        let cats = catalogues();
        let cats = cats.as_object().expect("catalogues is an object");
        for list in [COMMAND_FIELDS, SUBCOMMAND_FIELDS] {
            for field in list {
                if let FieldKind::Enum { catalogue, .. } | FieldKind::FlagSet { catalogue, .. } =
                    field.kind
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
    }
}
