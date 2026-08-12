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

//! `tcl-spec-hooks` — the **SpecTcl hook host**.
//!
//! The upper of the two layers in `docs/design/spec-packs.md`. It owns
//! everything DSL-specific and speaks only [`tcl_engine_api`] beneath it, so
//! engine-agnosticism is structural rather than aspirational: nothing in
//! [`HookHost`] names an engine.
//!
//! What it owns:
//!
//! - **The emitter verbs** ([`emit`]) as native host commands in the sandboxed
//!   interpreter — `role`, `prefix`, `fold`, `sink-suppressed`, `reject`,
//!   `invalid`/`abstain`, the three clause-shape verbs, and `consume` — plus
//!   the conversion from what a body emitted to the registry's typed answer.
//! - **The sandbox** ([`sandbox`]): a closed builtin whitelist and the host
//!   builtins a body gets instead, including the conservative `foldlist`.
//! - **The calling conventions**: `words` with non-literal words blanked, the
//!   `ctx` dict, and the normative literal-only precondition for the fold
//!   families and the option-arity hook.
//! - **Abstention and error policy**: an error, a budget blowout, or a panic
//!   is the family's silence — never a diagnostic on the user's code.
//! - **Containment** ([`crash`]): `catch_unwind` on every invocation,
//!   quarantine on the first crash, and a structured crash record carrying
//!   word *shapes* rather than user code.
//! - **Isolation**: one engine per pack, built here and never shared.
//!
//! ## Where it plugs in
//!
//! The registry's [`pack_hooks`](tcl_registry::pack_hooks) module holds the
//! seam. A hook gets a slot; the slot's thunk is an ordinary function pointer
//! of the family's shipped type; the spec is built with that pointer. Every
//! consumer — the optimiser's const-subst engine above all — keeps calling
//! `run_const_fold` and friends, unaware that the answer came from a Tcl body.
//!
//! ```no_run
//! use std::rc::Rc;
//! use tcl_registry::pack_hooks::{self, HookFamily};
//! use tcl_spec_hooks::{HookHost, PackPrograms, HookProgram, tclvm_host};
//!
//! let host = Rc::new(tclvm_host());
//! let installed = host.load_pack(
//!     PackPrograms::new("mylib").with(HookProgram::new(
//!         "mylib::strlen",
//!         HookFamily::ConstFold,
//!         "fold [string length [lindex $words 0]]",
//!     )),
//! );
//! pack_hooks::install_host(host);
//! // `installed[0].slot` now names the thunk to put in the spec's `const_fold`.
//! let folder = installed[0].slot.and_then(pack_hooks::const_fold_fn);
//! ```

pub mod crash;
pub mod emit;
pub mod host;
mod intern;
pub mod program;
pub mod sandbox;

pub use crash::{CrashKind, CrashRecord};
pub use host::{HookHost, HostConfig};
pub use program::{HookInstallation, HookOwner, HookProgram, PackPrograms};
pub use sandbox::SANDBOX_COMMANDS;

use tcl_engine_tclvm::TclVmEngine;

/// A host running its packs on `tcl-vm` — the canonical engine everywhere
/// (`docs/design/spec-packs.md`: server, CLI, and studio alike).
///
/// The only place in this crate that names an engine, and it is a
/// convenience: [`HookHost::new`] takes any factory, so a caller that wants
/// the Tcl→WASM codegen runtime instead passes that and changes nothing else.
#[must_use]
pub fn tclvm_host() -> HookHost<TclVmEngine> {
    HookHost::new(TclVmEngine::new)
}

/// A host running on `tcl-vm` with a non-default budget or server version.
#[must_use]
pub fn tclvm_host_with(config: HostConfig) -> HookHost<TclVmEngine> {
    HookHost::with_config(TclVmEngine::new, config)
}
