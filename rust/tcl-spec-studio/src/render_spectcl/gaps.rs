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

//! Why a draft field does not survive as `SpecTcl`: the gap kinds and the table of every field a rendered pack cannot carry.

// Why a field did not survive

/// Why a draft key cannot be written as `SpecTcl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// The DSL has a spelling and the loader reads it, but the **draft** holds
    /// no value to write: seeding records the field as
    /// [`UNRENDERABLE`](crate::draft::UNRENDERABLE_KEY) — "set, but the
    /// defining expression could not be recovered" — because the spec field is
    /// a function pointer or a reference to a named `&'static` descriptor.
    DraftOpaque,
    /// The draft holds the value and the design memo documents a spelling, but
    /// the loader has no reader for that property word yet, so writing it
    /// would only produce an unknown-property notice.
    LoaderGap,
    /// The design excludes the field from what a pack may author at all.
    Excluded,
    /// The field is a row of a pack-level block (`environment`, `dialect`),
    /// and a draft is one command's model, so no draft key holds it. The
    /// studio carries the block through an edit unchanged, and `tcl spec
    /// export` writes its rows as the pack declared them.
    PackLevel,
}

impl GapKind {
    /// One clause explaining this kind, used in the `TODO` line.
    pub(super) const fn why(self) -> &'static str {
        match self {
            Self::DraftOpaque => {
                "a draft records only that the field is set, never the expression that set it"
            }
            Self::LoaderGap => "the loader has no reader for that property word yet",
            Self::Excluded => "a pack may not author this field",
            Self::PackLevel => "a draft is one command's model, and this is a pack-level row",
        }
    }
}

/// One field the renderer cannot carry, with its documented DSL spelling.
#[derive(Debug, Clone, Copy)]
pub struct Gap {
    /// The draft / schema key.
    pub key: &'static str,
    /// How `docs/design/spec-dsl-examples/README.md`'s coverage matrix spells
    /// the field, or `""` where it spells none.
    pub spelling: &'static str,
    /// Why the value did not survive.
    pub kind: GapKind,
}

/// Every field a draft can hold that a rendered pack cannot carry, and every
/// pack-level row that has no draft key.
///
/// This is the renderer's half of the round-trip contract: the gate in
/// `tests/spectcl_roundtrip.rs` allows a rendered-then-reloaded draft to differ
/// from its source **only** on these keys, and reports any other difference as
/// a failure.
pub const GAPS: &[Gap] = &[
    Gap {
        key: "callback_protocols",
        spelling: "callback_protocol (command/subcommand operand)",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "callback_protocol",
        spelling: "option -callback-protocol",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "definition_body.profile",
        spelling: "object_system / definition_body profile",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "evaluation_contexts",
        spelling: "evaluation_context (command/subcommand/form)",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "completion_body_boundary",
        spelling: "completion -body-boundary",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "deferred_execution",
        spelling: "body_execution -entry-scope -trigger -family",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "source_arity_with_option",
        spelling: "",
        kind: GapKind::Excluded,
    },
    // Only the audited built-in handler may opt into this conditional source
    // phase; a loadable declaration cannot establish the C implementation.
    Gap {
        key: "conditional_source_paired_loop_handler",
        spelling: "conditional source paired-loop handler (built-in only)",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "pack_source_sandbox_helper",
        spelling: "stock host provisioning capability (not source-authored)",
        kind: GapKind::Excluded,
    },
    // The builtin survey relates exact selected query operands to an explicit
    // hypothetical target. A custom declaration cannot assert that relation.
    Gap {
        key: "source_package_query",
        spelling: "source_package_query ProvidedVersion|RequiredVersion|VersionSatisfies (subcommand/refine; excluded)",
        kind: GapKind::Excluded,
    },
    Gap {
        // Shipped names preserve the entire original grammar, including its
        // native object-method operation. Inline authoring cannot assert that
        // operation, so a custom grammar that retains one is withheld whole.
        key: "definition_body",
        spelling: "definition_body NAME|{ … }",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "nested_native_compilation",
        spelling: "sub_subcommand NAME -native_compilation CONTRACT",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "form_value_effects",
        spelling: "refine NAME {return_type|arg_types|byte_array_effect|var_elements_effect|safe_on_uninit OVERRIDE}",
        kind: GapKind::Excluded,
    },
    // A loadable declaration cannot certify an actual native handler's
    // storage and observer equivalence across an unknown compiler protocol.
    Gap {
        key: "successful_handler",
        spelling: "successful_handler -native Leaf|ExpressionArguments|DictionaryConstructor|EnsembleLeaf|EnsemblePathLeaf -direct-provider F5String|F5Binary|VariableOperands|InitialiseEmptyVariable|CatchOutputs|PossibleBodies|CommandBindingTransition",
        kind: GapKind::Excluded,
    },
    // Native result contracts require runtime identity, observer boundaries and
    // representation proofs; the loadable authoring surface cannot assert those.
    Gap {
        key: "completion",
        spelling: "completion {CODE …} ?-partial? ?-complete WINDOW?…",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        // Spelled on the opaque `completion` row, so it travels with it.
        key: "completion_knowledge",
        spelling: "completion … -complete WINDOW",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "native_result",
        spelling: "native_result -native CONTRACT",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "procedure_definition",
        spelling: "procedure_definition -native CONTRACT",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "native_compilation",
        spelling: "native_compilation -native CONTRACT",
        kind: GapKind::DraftOpaque,
    },
    // A phase descriptor alone cannot author the live implementation/hook
    // prerequisites required by semantic expansion. Keep the loss explicit.
    Gap {
        key: "body_execution",
        spelling: "body_execution -native CONTRACT",
        kind: GapKind::DraftOpaque,
    },
    // The value is not in the draft.
    //
    // `object_class` left this bucket. It looked like the others — an
    // `Option<&'static …>` reference — but the descriptor behind it is plain
    // data all the way down, and its method table *is* `&[SubCommand]`, so
    // seeding now carries the whole thing and the renderer writes it as
    // `object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }`.
    // `definition_body` followed once every member row states its effect: a
    // grammar is seeded as the shipped name it equals or as the whole block,
    // and written as `definition_body NAME` or `definition_body { … }`.
    // `semantic_operation` is a closed vocabulary seeded as `{kind, detail}`
    // and written in the loader's spelling. What still lives here is
    // genuinely opaque: a function pointer, or a reference to a shared
    // registry constant a pack can only name.
    Gap {
        key: "frame_effect",
        spelling: "frame_effect -level-word W -layout L",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        // Unsupported typed world accesses withdraw strong semantics; they cannot
        // become an empty footprint or derive native write proof from a spelling.
        key: "world_effects",
        spelling: "world_effects none|NAME|{ … } ?-partial? ?-complete WINDOW?…",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        // Its completeness is spelled on the `world_effects` row itself, so
        // it travels with that opaque row and has no row of its own.
        key: "world_effects_knowledge",
        spelling: "world_effects … -complete WINDOW",
        kind: GapKind::DraftOpaque,
    },
    // Every row of the block loads — `resolver` a body of the
    // `state_transitions` resolver family, `from-frame-effect`, `none` or
    // `-native ID` — but a draft holds no hook body for any family: a body
    // survives a form edit because the pack store carries its statement
    // forward verbatim. So the block stays one row here and travels the same
    // way, rather than splitting into drafted rows around a resolver the
    // draft could only hold as opaque.
    Gap {
        key: "state_transitions",
        spelling: "state_transitions NAME|{ composition … argument_shape … resolver \
                   none|from-frame-effect|-native ID|{words ctx} {…} widen … covers … commit … } \
                   ?-partial? ?-complete WINDOW?…",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        // Spelled on the `state_transitions` row itself, so it travels with
        // that opaque row.
        key: "state_transitions_knowledge",
        spelling: "state_transitions … -complete WINDOW",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requires",
        spelling: "event_requires NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "case_list",
        spelling: "case_list NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "body_scope",
        spelling: "body_scope NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "bpf_op",
        spelling: "bpf_op -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "data_collection",
        spelling: "data_collection -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requirement_forms",
        spelling: "event_requirement_form {word …} ?-only-in {E …}? ?{ … }?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_emits",
        spelling: "event_emits {E …} ?-possible|-async?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_emission_forms",
        spelling: "event_emission_form {word …} {E …} ?-possible|-async?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "side_switch_target",
        spelling: "side_switch_target Client|Server",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_handler_priority",
        spelling: "event_handler_priority -default N ?-warn-implicit?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "irules_top_level_effect",
        spelling: "irules_top_level_effect Priority|Timing",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "result_stability",
        spelling: "result_stability Unknown|ReferentiallyTransparent|Volatile|{ReadsVersionedWorld {D …}} ?-partial? ?-complete WINDOW?…",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        // Spelled on the opaque `result_stability` row, so it travels with it.
        key: "result_stability_knowledge",
        spelling: "result_stability … -complete WINDOW",
        kind: GapKind::DraftOpaque,
    },
    // The draft has it; the loader does not read it yet.
    //
    // Eleven keys left this bucket as the loader grew their readers:
    // the three `{VARIANT payload …}` element-structure facts,
    // `default_form_first_word`, `setter_constraints`, `format_string_type`,
    // `deprecation_fix`, `byte_array_payload`, `defines_symbol`,
    // `byte_array_effect`'s payload-carrying `Rebinarifies` variant, and — with
    // vocabulary 1.1 — the option row's `-deprecation-fix {…}`. What a pack
    // still cannot say is above (no value in the draft) or below (excluded by
    // design), not here.
    //
    // `remote_method` belongs here: the descriptor is plain data
    // and the draft carries its whole Rust expression, but the SpecTcl
    // vocabulary has no property word for it yet, so writing one would only
    // produce an unknown-property notice.  A pack authoring a second RPC family
    // is what will earn the reader.
    Gap {
        key: "remote_method",
        spelling: "",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "variable_write_min_args",
        spelling: "variable_write_min_args N",
        kind: GapKind::LoaderGap,
    },
    Gap {
        key: "body_interpreter",
        spelling: "body_interpreter Current|{Argument INDEX}|{ThreadArgument INDEX}|ProcessShared|Unknown",
        kind: GapKind::LoaderGap,
    },
    // Excluded by design.
    Gap {
        key: "dispatch_dependencies",
        spelling: "",
        kind: GapKind::Excluded,
    },
    // Registry source operand geometry is built-in metadata for the static Tk
    // lifecycle consumer; SpecTcl packs cannot declare or prove this layout.
    Gap {
        key: "teardown_argument_layout",
        spelling: "",
        kind: GapKind::Excluded,
    },
    // `substitution_resolver` and `pattern_arg_resolver` left this bucket with
    // the option-effect descriptor: `subst`'s switch families and `lsearch`'s
    // match styles are option rows declaring their effects, the first field is
    // gone from `CommandSpec`, and the second remains only as an escape hatch
    // no shipped spec sets — so the round trip never sees it set.
    // The native-lowering descriptor is the compiler's own account of how it
    // may compile a command to native code — a representation lattice and the
    // framing it is allowed to elide, meaningful only to `tcl-compiler`'s
    // native tier. It is a compiler-internal fact rather than a description of
    // the command's *language* surface, so a pack has nothing to say about it
    // and authoring one could only weaken a proof the compiler makes for
    // itself.
    Gap {
        key: "native_lowering",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "native_lowering_windows",
        spelling: "",
        kind: GapKind::Excluded,
    },
    // `semantics` is not in this bucket: the structural half is plain data all the
    // way down (like `object_class`), and a route with no body renders in
    // full. What still cannot survive a bare `CommandSpec` — a declared
    // implementation's body, which lives only in the loader's pack-hook
    // table, and an option-level `-evaluate` decline, not yet carried back
    // onto its option row — goes through [`native_hook`]'s existing
    // unrecovered-field path, exactly as `const_fold`'s body does, so it
    // needs no register entry of its own.
    //
    // Rows of the pack-level `environment NAME { … }` block. The studio's
    // forms edit commands; an environment block is carried through an edit
    // untouched, and `tcl spec export` writes its rows as declared.
    Gap {
        key: "environment_kind",
        spelling: "kind language|packages",
        kind: GapKind::PackLevel,
    },
    Gap {
        key: "environment_short_name",
        spelling: "short_name TEXT",
        kind: GapKind::PackLevel,
    },
    Gap {
        key: "environment_selecting_identity",
        spelling: "selecting_identity ID",
        kind: GapKind::PackLevel,
    },
];

/// The [`Gap`] for `key`, if the renderer cannot carry it.
#[must_use]
pub fn gap(key: &str) -> Option<&'static Gap> {
    GAPS.iter().find(|gap| gap.key == key)
}
