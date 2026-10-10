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

//! The conservative fact for a command a native extension registers
//! (`docs/design/compiler/registry-consumer-contracts.md` § *C Tcl
//! extensions*).
//!
//! Nothing the registry holds says what C code behind `Tcl_CreateObjCommand`
//! does, so the one fact it can state is the top of every axis: the command
//! may be handed a script or a variable name at any argument and run it at any
//! level; may read and write any state, create, rename and delete commands
//! (itself among them) and establish traces; may complete with any code and so
//! may complete normally; is a taint sink and a taint source; is unsafe and
//! hidden in a safe interpreter; and is never pure. It stays a command no code
//! generator specialises: its backing is [`RuntimeBacking::HostNative`], and it
//! carries no stamp and no window.
//!
//! The facts are stated once, here, and read by every way an extension command
//! is described: [`CommandSpec::extension_default`] for a registry command, and
//! [`DeclaredCommand::extension`](crate::model::DeclaredCommand::extension) for
//! a command a document or a workspace declares. A fact a description states
//! *narrows* the default on its own axis and on no other
//! ([`DeclaredCommand::narrowed_by`](crate::model::DeclaredCommand::narrowed_by)).
//!
//! Two axes are the registry's own wildcard and are left unstated on purpose.
//! A command with no state-transition descriptor resolves to
//! [`StateTransitionKnowledge::UnknownInvocation`](crate::state_transition::StateTransitionKnowledge),
//! which every consumer reads as a possible change to every identity domain
//! (command bindings, namespaces, interpreters, every kind of trace); a closed
//! statement of "an unknown rebinding" would be narrower than that, so the
//! default declares no `command_table_effect`, no `state_transitions` and no
//! `world_effects`.
//!
//! [`RuntimeBacking::HostNative`]: crate::RuntimeBacking::HostNative
//! [`CommandSpec::extension_default`]: crate::CommandSpec::extension_default

use crate::side_effects::{SideEffect, SideEffectTarget};
use crate::traits::Traits;

/// The code-evaluation axis: any argument may be run as a script or name a
/// variable, at any level, so a call is a dynamic barrier.
pub const EVALUATION: Traits = Traits::EVALUATES_CODE
    .union(Traits::CREATES_BARRIER)
    .union(Traits::CREATES_DYNAMIC_BARRIER);

/// The trace axis: the command may establish a variable trace.
pub const TRACES: Traits = Traits::ESTABLISHES_VARIABLE_TRACE;

/// The taint axis: attacker-influenced input may reach it, and what it returns
/// may be attacker-influenced.
pub const TAINT: Traits = Traits::TAINT_SINK.union(Traits::TAINT_SOURCE);

/// The safety axis: native code is unsafe in a safe interpreter and hidden
/// there.
pub const SAFETY: Traits = Traits::UNSAFE.union(Traits::SAFE_INTERP_HIDDEN);

/// The axes a statement of effects replaces: a declaration that says what the
/// command does to state, or that it does nothing, has said it evaluates no
/// code and establishes no trace beyond what it states.
pub const EFFECT_AXES: Traits = EVALUATION.union(TRACES);

/// Every trait of the default.
pub const TRAITS: Traits = EFFECT_AXES.union(TAINT).union(SAFETY);

/// What the command may do to state it does not name: read and write any of
/// it.
pub const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];
