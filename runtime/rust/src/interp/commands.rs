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

//! The command model and the command table: surface visibility, registration, guards, rename, aliases, ensembles, procedures, hidden commands and imports.

use super::native_alias_roles::NativeAliasHeader;
use super::native_children::ChildInterpreterCommand;
use super::native_procedure_resources::NativeProcedureCommand;
use super::{BuiltinFn, ImportToken, ObjCommand};
use std::rc::Rc;
use tcl_core_types::OoId;

mod aliases;
mod command_replacement;
mod command_tokens;
mod ensemble_creation;
mod guards;
mod hidden_commands;
mod identity_retirement;
mod procedure_definition;
mod registration;
mod rename_and_delete;
mod surface_visibility;

/// Why a hidden-table move did or did not happen. The variants are in C's
/// own check order, which is observable when more than one applies —
/// `finish_command_visibility` is the only thing that words them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandVisibilityOutcome {
    Moved,
    /// The source did not resolve (hide) or is not in the hidden table
    /// (expose).
    Missing,
    /// The source resolved outside the global namespace (hide only).
    NonGlobal,
    /// The destination is already taken.
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandVisibilityOp {
    Hide,
    Expose,
}

/// Memoised [`Interp::builtin_command_visible_for_surface`] answers.
#[derive(Default)]
pub(super) struct BuiltinSurfaceVisibility {
    surface: Option<(tcl_dialect::TclVersion, usize, usize)>,
    pub(super) names: std::collections::HashMap<Vec<u8>, bool>,
}

#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// An extension's command, registered through `Tcl_CreateObjCommand`: a
    /// C procedure called with the call's words as `objv`. Its dispatch `Rc`
    /// remains independent of the selected C once-only delete callback. Jim
    /// and neutral deletion retain their final-record callback schedule.
    ObjCmd(Rc<ObjCommand>),
    /// An `interp alias`: dispatch re-resolves `target` **by name, anchored at
    /// the global namespace, on every call** (so it lazily observes the target's
    /// *deletion* but does NOT follow its *rename* — the stored name simply stops
    /// resolving), then prepends the frozen `prefix` words to the caller's args.
    /// See `docs/design/runtime/rename-alias.md` §4.
    Alias {
        /// Original member ownership; command-token clones do not duplicate
        /// the native List or its member references.
        original_prefix: Rc<NativeAliasHeader>,
        /// C retains the original registration name independently of its slot.
        registration_name: Option<Rc<NativeAliasHeader>>,
        /// Registration report retained independently from the lookup slot.
        publication_name: Vec<u8>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
    /// A `namespace import` redirect. The source generation is the retained
    /// command-token identity; `source` is its mutable Tcl-facing projection
    /// and the deliberate fallback after command replacement adopts a fresh
    /// generation. Together they let every import follow rename/hide/expose
    /// without switching to a replacement installed at the vacated spelling.
    /// Ensemble imports additionally retain the ensemble's configuration token.
    Imported {
        source: Vec<u8>,
        source_generation: u64,
        ensemble: Option<Rc<crate::ensemble::EnsembleToken>>,
        identity: Rc<ImportToken>,
    },
    /// A `namespace ensemble`: dispatch maps `argv[1]` (a subcommand) to a target
    /// command prefix (`-map`, else `<ns>::<sub>`) and forwards `argv[2..]` — the
    /// generalised `dict for`→`::tcl::dict::for` redirect. See [`crate::ensemble`].
    Ensemble(Rc<crate::ensemble::EnsembleToken>),
    /// A user procedure (`proc`). Dispatch pushes a call frame, binds the args to
    /// the params (defaults + an `args` catch-all), runs the body in the proc's
    /// defining namespace, and maps a body-level `return` to `Ok`. Behind an `Rc`
    /// so the dispatch-time clone of the command handle is O(1), not a body copy.
    Proc(NativeProcedureCommand),
    /// A child interpreter, addressable as a command (`$child eval …`). The
    /// `Vec<u8>` is the child's name; dispatch routes the subcommand to the child
    /// `Interp` stored in [`Interp::children`].
    ChildInterp(ChildInterpreterCommand),
    /// A TclOO object or class, addressable as a command (`$obj method …`,
    /// `Class new`). The opaque token remains stable across rename and same-name
    /// recreation; Tcl-facing names are projections owned by `OoState`.
    OoObject(tcl_core_types::OoId),
    /// The private per-object `my` dispatcher, carrying the same stable owner
    /// identity as the object command rather than rediscovering it by name.
    OoMy(tcl_core_types::OoId),
    /// The private per-object `myclass` dispatcher.
    OoMyClass(tcl_core_types::OoId),
    /// A cross-interp alias installed in a *child* interp that delegates to a
    /// command in the *parent* (`interp alias child name {} parentCmd …`). When
    /// invoked, it runs `target` (+ `prefix` + the call args) in the parent.
    ParentAlias {
        /// Original member ownership; command-token clones do not duplicate
        /// the native List or its member references.
        original_prefix: Rc<NativeAliasHeader>,
        /// Only C retains the original registration name in the alias record.
        registration_name: Option<Rc<NativeAliasHeader>>,
        /// Selected child registration spelling, independent of command lookup.
        publication_name: Vec<u8>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
}

/// Which command token owned by one TclOO identity a binding exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OoCommandRole {
    Object,
    My,
    MyClass,
}

const OO_COMMAND_RETIREMENT_ORDER: [OoCommandRole; 3] = [
    OoCommandRole::Object,
    OoCommandRole::MyClass,
    OoCommandRole::My,
];

const OO_PRIVATE_COMMAND_ROLES: [OoCommandRole; 2] = [OoCommandRole::MyClass, OoCommandRole::My];

impl Command {
    /// Stable TclOO identity and role carried by this command, if any.
    pub(crate) fn oo_binding(&self) -> Option<(OoId, OoCommandRole)> {
        match self {
            Self::OoObject(id) => Some((*id, OoCommandRole::Object)),
            Self::OoMy(id) => Some((*id, OoCommandRole::My)),
            Self::OoMyClass(id) => Some((*id, OoCommandRole::MyClass)),
            _ => None,
        }
    }

    /// The object lifecycle token this command owns. Private dispatchers carry
    /// the same identity but their replacement does not destroy the object.
    pub(crate) fn oo_object(&self) -> Option<OoId> {
        match self {
            Self::OoObject(id) => Some(*id),
            _ => None,
        }
    }

    /// Whether `self` and `other` are the **same** command binding rather than
    /// two bindings that happen to look alike.
    ///
    /// C answers this with the `Command *` token, and a command-delete trace is
    /// exactly where the difference shows: a callback that re-creates the
    /// command it is being told about (`proc foo {} …`) leaves a *different*
    /// command at the same name, and C's deletion — which owns a captured token
    /// whose hash entry the new command has taken over — must leave it alone.
    /// Every shape that owns an `Rc` compares by pointer for that reason; the
    /// rest carry their whole identity in their fields.
    pub(crate) fn is_same_binding(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Builtin(a), Self::Builtin(b)) => std::ptr::fn_addr_eq(*a, *b),
            (Self::ObjCmd(a), Self::ObjCmd(b)) => Rc::ptr_eq(a, b),
            (Self::Proc(a), Self::Proc(b)) => a.is_same_binding(b),
            (Self::Ensemble(a), Self::Ensemble(b)) => Rc::ptr_eq(a, b),
            (Self::Imported { identity: a, .. }, Self::Imported { identity: b, .. }) => {
                Rc::ptr_eq(a, b)
            }
            // Aliases carry a token like every other `Rc`-owning shape: a
            // delete trace that recreates `foo` as an *identical* alias
            // leaves a different command at the name, and C's deletion must
            // leave it alone. Structural equality said "same binding" and
            // deleted the new one.
            (Self::Alias { identity: a, .. }, Self::Alias { identity: b, .. })
            | (Self::ParentAlias { identity: a, .. }, Self::ParentAlias { identity: b, .. }) => {
                Rc::ptr_eq(a, b)
            }
            (Self::ChildInterp(a), Self::ChildInterp(b)) => a.same_allocation(b),
            (Self::OoObject(a), Self::OoObject(b))
            | (Self::OoMy(a), Self::OoMy(b))
            | (Self::OoMyClass(a), Self::OoMyClass(b)) => a == b,
            _ => false,
        }
    }
}
