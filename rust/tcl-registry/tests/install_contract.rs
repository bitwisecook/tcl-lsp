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

//! The install contract, in a harness that installs nothing at start-up:
//! a read before any install panics with the documented message, an install
//! is idempotent for the same list, and a different list is a conflict.

use tcl_registry::{CommandRegistry, InstallConflict, Surface, commands::SHIPPED};

static FIRST_ONLY: [&Surface; 1] = [SHIPPED[0]];

#[test]
fn the_install_contract() {
    let read = std::panic::catch_unwind(CommandRegistry::build_default);
    let message = read.expect_err("a read before any install panics");
    let message = message
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| message.downcast_ref::<&str>().copied())
        .expect("a string panic");
    assert!(message.contains("tcl_registry::install"), "{message}");
    assert!(message.contains("01-surface-input.md"), "{message}");
    assert!(tcl_registry::try_installed().is_none());

    let subset = CommandRegistry::from_surfaces(&FIRST_ONLY);
    assert!(
        subset.get("set").is_some(),
        "from_surfaces reads no installed set"
    );
    assert!(tcl_registry::try_installed().is_none());

    let set = tcl_registry::install_shipped().expect("the first install");
    assert!(std::ptr::eq(
        set,
        tcl_registry::install_shipped().expect("the same list again")
    ));
    let conflict: InstallConflict =
        tcl_registry::install(&FIRST_ONLY).expect_err("a different list");
    assert_eq!(conflict.offered, vec!["tcl"]);
    assert_eq!(conflict.installed.len(), SHIPPED.len());
    assert!(CommandRegistry::build_default().get("set").is_some());
}
