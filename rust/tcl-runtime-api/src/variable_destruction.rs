// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical variable teardown phases shared by native adapters and analysis.

/// A reached phase of an originally captured variable destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableDestructionPhase {
    /// The original root lookup and root registrations have been detached.
    /// Captured array members remain accessible through existing direct aliases.
    RootLookupDetached,
    /// One captured member has become undefined before its frozen callbacks.
    /// Other members retain their own contents and active read/write guards.
    MemberRetired,
    /// Every captured member and its callbacks has completed teardown.
    Complete,
}

/// Native ordering of contents teardown and variable callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableDestructionProtocol {
    /// Retire one cell's contents before invoking its captured unset callbacks.
    CellContentsThenCallbacks,
    /// Detach the root lookup and root traces, invoke captured root callbacks,
    /// then retire each old member immediately before that member's callbacks.
    /// Member order is not a portable guarantee. Recreated names cannot redirect
    /// these captured receivers or callback registrations.
    ArrayLookupThenRootCallbacksThenMembers,
}
