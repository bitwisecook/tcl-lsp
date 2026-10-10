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

//! The sandbox an embedder that hosts this interpreter as an engine builds a
//! body into: the commands it may call, and the release it runs at. The rules
//! are the bytecode VM's engine's, so a body sees the same surface on either
//! engine, and they live here rather than with the native engine so a host
//! that drives this runtime compiled to `wasm32` applies them too.

use crate::interp::Interp;

impl Interp {
    /// Keep only the commands `allowed` names and those `kept` names (a host's
    /// commands, compiled units), as the bytecode VM's engine keeps them: an
    /// allowed `expr` keeps the math functions, which from 8.5 are commands
    /// (`tcl::mathfunc::abs`), but `rand` and `srand`, whose seed one
    /// evaluation would leave for the next; and an allowed ensemble keeps the
    /// commands its subcommands are (`tcl::dict::get` for `dict`). A whitelist,
    /// never a blacklist: a command the runtime gains later is not reachable.
    pub fn restrict_to(&mut self, allowed: &[&str], kept: &[String]) {
        let math = allowed.contains(&"expr");
        self.retain_commands(&|name: &str| {
            allowed.contains(&name)
                || kept.iter().any(|command| command == name)
                || (math
                    && name
                        .strip_prefix("tcl::mathfunc::")
                        .is_some_and(|function| !matches!(function, "rand" | "srand")))
                || is_subcommand_of_allowed(name, allowed)
        });
    }

    /// Pin the interpreter to the profile `profile` names ([`release_profile`])
    /// and answer its canonical name; `None`, and nothing pinned, for a name
    /// that is no release the runtime runs.
    pub fn pin_release(&mut self, profile: &str) -> Option<&'static str> {
        let resolved = release_profile(profile)?;
        self.set_dialect_profile(resolved);
        Some(resolved.name)
    }
}

/// The catalogue profile `profile` names, resolved through the one dialect
/// ingress, when it is a release the runtime runs: `None` for a name that
/// resolves to no catalogue profile or to one with no Tcl release under it
/// (the lenient sink, `tk`, a vendor configuration surface).
#[must_use]
pub fn release_profile(profile: &str) -> Option<&'static tcl_dialect::DialectProfile> {
    tcl_registry::model::resolve_known_environment(profile)
        .and_then(|environment| environment.catalogue_profile())
        .filter(|resolved| resolved.runtime_base.is_some())
}

/// Whether `name` is the direct form of a subcommand of an ensemble `allowed`
/// names: `tcl::ENSEMBLE::SUBCOMMAND`.
fn is_subcommand_of_allowed(name: &str, allowed: &[&str]) -> bool {
    name.strip_prefix("tcl::")
        .and_then(|rest| rest.split_once("::"))
        .is_some_and(|(ensemble, _)| allowed.contains(&ensemble))
}
