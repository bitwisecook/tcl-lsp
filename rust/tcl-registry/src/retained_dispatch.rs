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

//! Which already-retained callbacks an invocation may run before it returns
//! (issue #1710).
//!
//! A registration such as `after 10 {…}` or `entry .e -validatecommand {…}`
//! stores a script; something later runs it. This fact names the later
//! commands: the event loop (`update`, `vwait`, `tkwait`), a direct delivery
//! to one window (`event generate`, `destroy`), or a widget method that drives
//! its own stored callback (`.b invoke`, `.e insert` under `-validate key`).
//!
//! It says nothing about callbacks an invocation runs through variable traces
//! or other observers it fires itself; those remain the business of
//! `world_effects` and the variable-observer model. Stating `None` here
//! therefore never claims an invocation is free of re-entrant callbacks.
//!
//! Evidence: `tests/data/tk_callback_dispatch/`, captured under each
//! release's own wish (Tk 8.4.20–9.1.0).

use crate::knowledge::Knowledge;

/// The window whose stored callbacks a [`RetainedDispatch::TargetWindow`]
/// invocation may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DispatchTarget {
    /// The widget whose instance command is invoked (`.e insert`).
    Receiver,
    /// The window named by this post-command argument (`event generate .w`).
    Argument(u8),
    /// Every window named from this post-command argument onward, and their
    /// descendants (`destroy .a .b`).
    ArgumentsFrom(u8),
}

/// Which already-retained callbacks an invocation may run synchronously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetainedDispatch {
    /// Runs no previously retained callback before it returns.
    None,
    /// May run callbacks retained by one target window: its bindings, its
    /// widget command callbacks, or its validation script.
    TargetWindow(DispatchTarget),
    /// Services the event loop: any pending timer, idle, file or window
    /// callback may run.
    AllPending,
}

impl RetainedDispatch {
    /// The fact a reader uses: an unknown row may dispatch anything pending.
    #[must_use]
    pub const fn read(knowledge: Knowledge<Self>) -> Self {
        match knowledge.stated() {
            Some(dispatch) => dispatch,
            None => Self::AllPending,
        }
    }
}

/// The releases the dispatch evidence covers: wish and tclsh 8.4.20–9.1.0.
pub const EVIDENCED: &[tcl_dialect::model::SpecSurface] = &[
    tcl_dialect::model::SpecSurface::core_in(tcl_dialect::model::Family::Tcl, &[("8.4", None)]),
];

/// Runs no retained callback, at every evidenced release.
pub const NONE: Knowledge<RetainedDispatch> = Knowledge::Complete(RetainedDispatch::None, EVIDENCED);

/// Services the event loop, at every evidenced release.
pub const ALL_PENDING: Knowledge<RetainedDispatch> =
    Knowledge::Complete(RetainedDispatch::AllPending, EVIDENCED);

/// Runs the receiving widget's own stored callback, at every evidenced release.
pub const RECEIVER: Knowledge<RetainedDispatch> = Knowledge::Complete(
    RetainedDispatch::TargetWindow(DispatchTarget::Receiver),
    EVIDENCED,
);
