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

//! Declarations a script makes: procedures, iRules `when` events and rule loaders, and the members of class definitions.

mod class_members;
mod definer_calls;
mod irules_events;
mod procedures;

#[cfg(test)]
pub(super) use class_members::{declared_member_vars, statement_words};
#[cfg(test)]
pub(super) use procedures::{parse_formal_param_names, qualify_proc_name};
#[cfg(test)]
pub(super) use procedures::{parse_var_list_names, proc_body_namespace};

/// One statically-extractable definer invocation, classified from the
/// command's **registry spec** — never a spelling match: the
/// definer's member grammar plus which argv words carry the definition
/// target and the static braced body.
///
/// Covers every command whose spec hangs a
/// [`tcl_registry::definer::DefinitionBodyGrammar`] off
/// `CommandSpec::definition_body`: the `TclOO` metaclasses (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`), the
/// `oo::define` / `oo::objdefine` script forms, snit's `type` / `widget` /
/// `widgetadaptor`, and itcl's `itcl::class`.  A new definer added to the
/// registry is picked up here with no lowering change.
#[derive(Clone, Copy)]
pub(super) struct DefinerCall {
    pub(super) definition_scope: Option<crate::ir::ProcedureId>,
    descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    /// The definer's member grammar (registry data).
    pub(super) grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    /// Full-argv index of the class / type / object-handle word.
    name_idx: usize,
    /// Full-argv index of the static braced definition body.
    pub(super) body_idx: usize,
    /// `oo::objdefine`: the target is an *object handle*, so members and
    /// `variable` declarations are **per-object** state: after
    /// `oo::objdefine $k { variable z }`, `my variable z; info exists z`
    /// yields `0` — per-object, not per-class.
    /// They home under a synthetic `::@objdefine@::…` class name and never
    /// join the real class's cross-block `class_instance_vars` union.
    per_object: bool,
}
