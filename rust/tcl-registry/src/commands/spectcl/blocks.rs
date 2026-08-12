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

//! The SpecTcl statements that take a **block**.
//!
//! Each carries a `definition_body` grammar naming the words legal inside it,
//! which is what makes the DSL's vocabulary context-sensitive and gives every
//! block folding, keyword painting, and completion generically.
//!
//! Two shapes appear here:
//!
//! * a bare block (`hover { … }`, `world_effects { … }`) — the block is
//!   argument 0;
//! * a named block (`values NAME { … }`, `descriptor KEY NAME { … }`) — the
//!   name comes first.
//!
//! Every one of these properties may *instead* name a shipped descriptor or
//! constant (`case_list switch`, `definition_body snit`, `world_effects none`)
//! rather than spell a block out. That is why their arities start at 1 and
//! why the body role is only claimed when a block word is actually present:
//! a walker paints a script only for a braced word, so the reference form is
//! left alone by construction.
use crate::prelude::*;

use super::SOURCE;

/// A bare-block statement: `hover { … }` / `NAME` / `none`.
fn block(
    name: &'static str,
    grammar: &'static DefinitionBodyGrammar,
    summary: &'static str,
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary,
            synopsis: &[],
            snippet,
            source: SOURCE,
            examples: "",
            return_value: "",
        }),
        arg_roles: &[(0, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        definition_body: Some(grammar),
        ..CommandSpec::DEFAULT
    }
}

/// A named-block statement: `values NAME { … }`, `object_class NAME { … }`.
fn named_block(
    name: &'static str,
    grammar: &'static DefinitionBodyGrammar,
    arity: Arity,
    summary: &'static str,
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        arity,
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        ..block(name, grammar, summary, snippet)
    }
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }`.
const OBJECT_CLASS_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-superclass",
        value: OptionValue::value("classes"),
        detail: "the class's superclasses",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-allow-unknown",
        detail: "an unrecognised method on this class is not flagged",
        ..OptionSpec::DEFAULT
    },
];

/// The name is the first word and the block, when present, is the last —
/// which is what keeps the block's position right however many of the two
/// optional flags sit between them. A resolver rather than a fixed pair
/// because `OptionSpec`-shaped flags of differing widths can intervene, so
/// the index is genuinely a function of the call's own words.
fn object_class_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = vec![(0u8, ArgRole::Name)];
    if args.len() >= 2
        && let Ok(last) = u8::try_from(args.len() - 1)
    {
        roles.push((last, ArgRole::Body));
    }
    roles
}

fn object_class() -> CommandSpec {
    CommandSpec {
        arity: Arity::new(1, 4),
        options: OBJECT_CLASS_OPTIONS,
        arg_role_resolver: Some(object_class_arg_roles),
        ..block(
            "object_class",
            &crate::definer::SPECTCL_OBJECT_CLASS_GRAMMAR,
            "Declare the class this command's instances belong to.",
            "`object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }`. The NAME word is the class name, which is not always the command name — a factory command may manufacture a differently-named class. `method` rows reuse the `subcommand` body grammar.",
        )
    }
}

/// `descriptor KEY NAME { … }` — the schema key first, then the name the
/// descriptor is declared under.
///
/// It is the one block whose *inner* vocabulary depends on another word on
/// its own line (the key), which no single grammar can express. Rather than
/// guess, it carries no grammar: its block still folds and recurses (the
/// `Body` role says so), and the loader — which reads the key — is the layer
/// that knows which descriptor vocabulary applies. Recorded as a known limit
/// of the pack rather than papered over with a union of every block's words,
/// which would accept `summary` inside a `state_transitions` descriptor.
fn descriptor() -> CommandSpec {
    CommandSpec {
        name: "descriptor",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Declare a shared, named block-valued descriptor.",
            synopsis: &["descriptor key name { … }"],
            snippet: "Declares one of the block-valued properties under a name the pack's commands can then reference by that name. A name declared in the pack shadows a shipped catalogue name of the same kind, and the loader says so.",
            source: SOURCE,
            examples: "descriptor world_effects class-factory-effects { … }",
            return_value: "",
        }),
        arg_roles: &[
            (0, ArgRole::Keyword),
            (1, ArgRole::Name),
            (2, ArgRole::Body),
        ],
        body_kind: BodyKind::Structural,
        ..CommandSpec::DEFAULT
    }
}

/// `hook NAME {params} { … }` — a shared hook body, declared once and named
/// by the hook property statements that use it. Its body is ordinary Tcl (the
/// sandbox whitelist), so it carries no grammar and the walker drops back out
/// of definition context for it, exactly as it does for a `method` body.
fn hook() -> CommandSpec {
    CommandSpec {
        name: "hook",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Declare a shared hook body.",
            synopsis: &["hook name {words ctx} { … }"],
            snippet: "A small pure Tcl body with a proc-shaped signature, evaluated only at query time and only in the sandbox: no open/exec/source/socket/after/interp/uplevel/upvar/trace/namespace/proc/rename/info/subst, a step budget, and a wall-clock cap. Its own return value is ignored — a hook speaks by calling its family's emitter verbs, and calling none is an abstention. Any error raised inside a hook body is an abstention too.",
            source: SOURCE,
            examples: "hook ascii-only-length {words ctx} {\n    if {![string is ascii [lindex $words 0]]} return\n    fold [string length [lindex $words 0]]\n}",
            return_value: "",
        }),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        ..CommandSpec::DEFAULT
    }
}

/// `default KEY VALUE…` — a pack-wide default for one availability /
/// identity key.
fn default_statement() -> CommandSpec {
    super::statement(
        "default",
        Arity::at_least(2),
        "Set a pack-wide default for one availability key.",
        "Takes only availability / identity keys (`dialects`, `required_package`, `tcllib_package`, the three versions, `warn_missing_import`, `is_namespace_exported`); a command stating the key itself wins.",
    )
}

/// Every block-shaped SpecTcl statement.
pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        // --- pack level ---
        named_block(
            "values",
            &crate::definer::SPECTCL_VALUES_GRAMMAR,
            Arity::exact(2),
            "Declare a shared argument-value table.",
            "A named table of `value` rows that `arg -values-from NAME` and `option -values-from NAME` reference. Declaring one shadows a shipped catalogue table of the same name, and the loader says so.",
        ),
        hook(),
        descriptor(),
        default_statement(),
        // --- documentation ---
        block(
            "hover",
            &crate::definer::SPECTCL_HOVER_GRAMMAR,
            "Documentation for the enclosing command or subcommand.",
            "Three words are renamed from their Rust field names — `snippet` becomes `description`, `examples` becomes `example` (repeatable), `return_value` becomes `returns`. A braced word keeps real newlines and literal backslashes, so a multi-line example is written as itself; nothing is stripped, joined, dedented, or wrapped.",
        ),
        // --- clause and case shapes ---
        block(
            "clause_grammar",
            &crate::definer::SPECTCL_CLAUSE_GRAMMAR_GRAMMAR,
            "Declare a clause-chain word grammar (the `if` shape).",
            "Both `arg_role_resolver` and `clause_shape_check` are derived from this one declaration. Normative: a keyword is matched only at a clause boundary and at a `?noise?` position; every other slot is filled positionally and consumes whatever word is there, including one spelled like a keyword. `STRUCTURALLY_CHECKED_ARITY` is not implied — the pack still declares it.",
        ),
        block(
            "case_list",
            &crate::definer::SPECTCL_CASE_LIST_GRAMMAR,
            "Declare the pattern/body list shape of a `switch`-like command.",
            "A case list is a *value* (`{pattern body …}` inside one word) rather than a word grammar, which is why it is a separate field from `clause_grammar`. `case_list switch` names the shipped descriptor; the block spells out all thirteen plain-data fields.",
        ),
        // --- iRules event surface ---
        block(
            "event_requires",
            &crate::definer::SPECTCL_EVENT_REQUIRES_GRAMMAR,
            "Declare which event contexts the command is legal in.",
            "Eight plain-data scalars, so it is fully declarative. `event_requirement_form` rows attach a nested variant of this block to a specific argument form.",
        ),
        // --- world / state effects ---
        block(
            "world_effects",
            &crate::definer::SPECTCL_WORLD_EFFECTS_GRAMMAR,
            "Declare what the command reads and writes in the analysed world.",
            "The surrounding plain data is authorable; only `resolver` is reference-only (`-native ID`, `none`, or a derivation keyword), because it produces typed transition facts the DSL has no vocabulary for. `world_effects none` is the one-word empty descriptor.",
        ),
        block(
            "state_transitions",
            &crate::definer::SPECTCL_STATE_TRANSITIONS_GRAMMAR,
            "Declare the state transitions the command commits.",
            "`resolver from-frame-effect` derives the resolver from the command's own `frame_effect`. That derivation is a proof obligation: a dynamic level word aborts the whole derivation (zero transitions, not \"assume the default frame\"), while a dynamic member of an alias pair skips that pair and continues.",
        ),
        // --- definer grammars and scoped bodies ---
        block(
            "definition_body",
            &crate::definer::SPECTCL_DEFINITION_BODY_GRAMMAR,
            "Declare (or name) the member grammar of this command's definition body.",
            "`definition_body NAME` names a shipped grammar (`tcloo`, `tcloo-configurable`, `snit`, `snit-widget`, `itcl`); the block spells one out from `DefinitionBodyGrammar`'s own fields. `-roles {N ROLE …}` *is* the `MemberSpec::arg_roles` field, which is why it replaced the sketched one-flag-per-role spelling.",
        ),
        block(
            "body_scope",
            &crate::definer::SPECTCL_BODY_SCOPE_GRAMMAR,
            "Declare (or name) the curated command environment of this command's body.",
            "Inside a `body_scope` block, `command` is a scoped-command row rather than a pack-level command declaration — the same context rule that makes `method` a member row inside `object_class`. Its nested `subcommand` blocks are ordinary subcommand bodies, reused unchanged.",
        ),
        object_class(),
    ]
}
