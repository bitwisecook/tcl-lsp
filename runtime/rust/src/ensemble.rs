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

//! Ensembles (T1.5) — the canonical `ens sub …` → `target …` redirect.
//!
//! An ensemble command maps its first argument (a subcommand) to a target
//! command prefix and forwards the rest — the generalisation of the
//! `dict for` → `::tcl::dict::for` rewrite (the A3 contract: "ensembles map
//! `ens sub` → target (default `::ens::sub` or `-map`, unambiguous-prefix unless
//! `-prefix 0`)"). Modelled on C Tcl 9's `tclEnsemble.c`.
//!
//! This module is the **pure** part: the config an ensemble carries and the
//! subcommand-resolution + error-wording rules. `namespace ensemble create`
//! builds an [`EnsembleConfig`] (`cmd_namespace.rs`); the dispatch trampoline
//! that re-dispatches to the target lives on the interp (`interp.rs`), the same
//! split as `interp alias` (build in `cmd_alias.rs`, dispatch in `interp.rs`).

use crate::namespace::NsId;

/// Stable command-token state shared by direct and in-flight ensemble
/// invocations. The lifecycle implementation is owned by `tcl-cmd-core` so
/// the native VM and this runtime apply the same Tcl rules.
pub type EnsembleToken = tcl_cmd_core::ensemble::EnsembleToken<EnsembleConfig, Vec<u8>>;

/// An ensemble `-map`: each entry is `(subcommand, target command prefix words)`.
pub type EnsembleMap = Vec<(Vec<u8>, Vec<Vec<u8>>)>;

/// An ensemble command's configuration (the payload of
/// [`Command::Ensemble`](crate::interp::Command)).
#[derive(Clone, Debug)]
pub struct EnsembleConfig {
    /// The namespace subcommands dispatch into (default targets are
    /// `<ns>::<sub>`); the ns `namespace ensemble create` ran in.
    pub ns: NsId,
    /// `-map`: subcommand → target command prefix (words). When present without
    /// an explicit `-subcommands`, its keys are the valid subcommand set.
    pub map: Option<EnsembleMap>,
    /// `-subcommands`: the explicit valid subcommand set. When `None` (and no
    /// `-map`), the set is the namespace's exported commands.
    pub subcommands: Option<Vec<Vec<u8>>>,
    /// `-prefixes`: allow unambiguous-prefix subcommand matching (default true).
    pub prefixes: bool,
    /// `-parameters`: formal parameter names that precede the subcommand in a
    /// call (`ens p1 p2 sub args…`); their values are threaded in after the
    /// resolved target (`target p1 p2 args…`). Empty for an ordinary ensemble.
    pub parameters: Vec<Vec<u8>>,
    /// `-unknown`: a handler command prefix invoked (as `handler… ensembleCmd
    /// subcommand args…`) when a subcommand doesn't resolve, before erroring.
    /// Empty for none.
    pub unknown: Vec<Vec<u8>>,
}

// Subcommand resolution and the `must be …` enumeration are the shared
// `tcl_cmd_core::ensemble` owner's (`resolve_subcommand` /
// `unknown_subcommand_message`); this module keeps only the config type. The
// dispatch trampoline in `interp.rs` calls the owner directly.
