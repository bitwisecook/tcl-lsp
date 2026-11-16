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

//! The draft model — a JSON mirror of a `CommandSpec` or `SubCommand`.
//!
//! A draft is a JSON object keyed by Rust field name, holding exactly what
//! [`crate::schema`] says each field's editor produces. Seeding a draft from a
//! live registry spec ([`from_command_spec`]) is what makes the studio a
//! *browser* of the registry as well as an editor: pick a command, get its
//! real spec in the form, adjust it, render the `.rs` back out.
//!
//! ## Fields that cannot round-trip
//!
//! A handful of spec fields hold a function pointer (`arg_role_resolver`,
//! `const_fold`, `taint_sink_gate`, …) or a reference to a **named** `&'static`
//! descriptor the registry shares between commands (`case_list`,
//! `body_scope`, `frame_effect`, `bpf_op`, `event_requires`,
//! `command_forms`, and the effect descriptors).
//! Rust can tell that such a field is set, but not recover the *expression* —
//! the constant's path — that set it.
//! Seeding records those keys under [`UNRENDERABLE_KEY`] so the form can flag
//! them and the renderer can emit a `TODO` rather than silently dropping
//! behaviour the source command had.
//!
//! A descriptor that is **plain data** is a different case and does round-trip:
//! `repeated_args`, `binds_handle`, `byte_array_payload`, `defines_symbol`,
//! `oo_context_facts`, and a command's or subcommand's `versioned_arg_values`
//! are rendered
//! back out as full struct literals (every field spelled, never a defaulting
//! constructor), so drafting and re-rendering a command that sets one loses
//! nothing. [`crate::coverage`] is what keeps those literals complete.
//! `object_class` is the same case one level deeper — a class name, a flag,
//! superclass names, and a method table that *is* `&[SubCommand]` — so it is
//! seeded as a JSON object whose methods are ordinary subcommand drafts.
//! `definition_body` is too: a grammar is plain data once every member states
//! its effect, so it is seeded as the name of the shipped grammar it equals, or
//! as the whole grammar. `semantic_operation` is a closed vocabulary seeded as
//! `{kind, detail}`.

use serde_json::{Map, Value, json};
#[cfg(test)]
use tcl_registry::BodyInterpreter;
#[cfg(test)]
use tcl_registry::arg_role::ArgRole;
#[cfg(test)]
use tcl_registry::arity::Arity;
#[cfg(test)]
use tcl_registry::definer::ManufacturerMethod;
#[cfg(test)]
use tcl_registry::spec::ObjectClassSpec;
use tcl_registry::spec::{CommandSpec, SubCommand};
#[cfg(test)]
use tcl_registry::traits::Traits;
#[cfg(test)]
use tcl_registry::types::{ReturnElements, VarElementsEffect, VarWriteTyping};
#[cfg(test)]
use tcl_registry::value_transfer::EvalRoute;

#[cfg(test)]
use tcl_dialect::model::SpecSurface;

mod command_seed;
mod definition_grammar;
mod field_values;
mod options_and_forms;
mod rust_expressions;
mod subcommand_seed;

use command_seed::{
    command_advanced, command_docs, command_hooks, command_identity, command_options,
    command_taint, command_types,
};
#[cfg(test)]
pub(crate) use definition_grammar::clause_grammar;
pub use definition_grammar::definition_body_block;
#[cfg(test)]
use field_values::dialects;
#[cfg(test)]
pub(crate) use field_values::{
    arg_type_map, arg_value, hover, insert_lifecycle, setter_constraint,
};
pub(crate) use field_values::{arity, selected_variable_receivers, side_effect};
#[cfg(test)]
pub(crate) use options_and_forms::{form_spec, option_spec};
#[cfg(test)]
pub(crate) use subcommand_seed::sub_subcommand;
use subcommand_seed::subcommand_body;

/// Draft key listing the fields a live spec sets but whose defining Rust
/// expression could not be recovered. Absent (or empty) when everything
/// round-tripped.
pub const UNRENDERABLE_KEY: &str = "__unrenderable";

/// Draft key naming the option-arity hooks that need supplying.
///
/// Distinct from a plain field key because the thing to fill in is nested in
/// an option row, not a top-level field: the renderer resolves it against the
/// `options` array so the note clears once every hook holds an expression.
pub const OPTION_HOOK_KEY: &str = "options.arity_hook";

/// Draft key naming option lifecycle callbacks whose Rust function path must
/// be supplied by the author.
pub const OPTION_DEPRECATION_FIX_HOOK_KEY: &str = "options.deprecation_fix_hook";

/// Internal option-row marker for a callback hook that seeding could not name.
pub const OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY: &str = "__deprecationFixUnrecoverable";

/// Draft key holding the dialect the draft was seeded from, when it came from
/// the live registry.
pub const SOURCE_DIALECT_KEY: &str = "__sourceDialect";

/// A draft is a plain JSON object; the schema names its keys.
pub type Draft = Map<String, Value>;

/// Records the keys whose defining expression could not be recovered.
#[derive(Debug, Default)]
struct Unrecovered(Vec<&'static str>);

impl Unrecovered {
    /// Note `key` as unrecoverable when `present` holds, and return the JSON
    /// placeholder (always `null` — the author supplies the expression).
    fn expr(&mut self, key: &'static str, present: bool) -> Value {
        if present {
            self.0.push(key);
        }
        Value::Null
    }

    fn note(&mut self, key: &'static str) {
        if !self.0.contains(&key) {
            self.0.push(key);
        }
    }

    /// Nothing was lost — the "complete" answer a nested row reports upward
    /// when it seeds a sub-draft of its own (a second-level subcommand's
    /// option table) rather than sharing its parent's accumulator.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn into_value(self) -> Value {
        Value::Array(self.0.into_iter().map(|k| json!(k)).collect())
    }
}

/// Seed a draft from a live [`SubCommand`].
#[must_use]
pub fn from_subcommand(sub: &SubCommand) -> Draft {
    let mut lost = Unrecovered::default();
    let draft = subcommand_body(sub, &mut lost);
    let mut draft = draft;
    draft.insert(UNRENDERABLE_KEY.to_owned(), lost.into_value());
    draft
}

/// Seed a draft from a live [`CommandSpec`].
///
/// Every field the schema names is present in the result, so the form never
/// has to guess a default. Fields whose defining expression could not be
/// recovered are listed under [`UNRENDERABLE_KEY`].
#[must_use]
pub fn from_command_spec(spec: &CommandSpec) -> Draft {
    let mut lost = Unrecovered::default();
    let mut d = Map::new();
    command_identity(&mut d, spec, &mut lost);
    command_types(&mut d, spec, &mut lost);
    command_docs(&mut d, spec, &mut lost);
    command_hooks(&mut d, spec, &mut lost);
    command_options(&mut d, spec, &mut lost);
    command_taint(&mut d, spec, &mut lost);
    command_advanced(&mut d, spec, &mut lost);
    d.insert(UNRENDERABLE_KEY.to_owned(), lost.into_value());
    d
}

/// A draft of a brand-new command: every field at its `CommandSpec::DEFAULT`
/// value.
#[must_use]
pub fn default_command_draft() -> Draft {
    from_command_spec(&CommandSpec::DEFAULT)
}

/// A draft of a brand-new subcommand: every field at its
/// `SubCommand::DEFAULT` value.
#[must_use]
pub fn default_subcommand_draft() -> Draft {
    from_subcommand(&SubCommand::DEFAULT)
}

#[cfg(test)]
mod tests;
