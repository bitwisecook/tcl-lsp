// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly observers of actual owned storage, including after a Host refusal.
//! These observers grant no name resolution, getter or native object access.

use crate::{interp::Interp, namespace::GLOBAL, obj::TclObj};

/// Observe a known actual global scalar tail without executing variable lookup.
pub(crate) fn global_scalar(interpreter: &Interp, tail: &[u8]) -> Option<*mut TclObj> {
    interpreter.namespaces().var_table(GLOBAL).load_scalar(tail)
}
