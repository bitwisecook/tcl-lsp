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

//! The install contract. Each check runs in a child process of this harness
//! with no surface set installed, because a process installs once. Without
//! the test-install variable a read before any install panics with the
//! documented message, an install is idempotent for the same list, and a
//! different list is a conflict; with it, a build with `debug_assertions`
//! installs the shipped list and a release build still panics.

use std::process::Command;

use tcl_registry::{CommandRegistry, InstallConflict, NOT_INSTALLED, Surface, commands::SHIPPED};

const CHILD: &str = "TCL_REGISTRY_INSTALL_CONTRACT_CHILD";
const TEST_INSTALL: &str = "TCL_REGISTRY_TEST_INSTALL";

static FIRST_ONLY: [&Surface; 1] = [SHIPPED[0]];

/// Run the test `name` in a child process; `fallback` says whether it sees the
/// test-install variable. Returns the child's success and its output.
fn in_child(name: &str, fallback: bool) -> (bool, String) {
    let mut command = Command::new(std::env::current_exe().expect("the harness path"));
    command.args(["--exact", name, "--nocapture", "--test-threads", "1"]);
    command.env(CHILD, name);
    if fallback {
        command.env(TEST_INSTALL, "1");
    } else {
        command.env_remove(TEST_INSTALL);
    }
    let output = command.output().expect("run the child");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

fn is_child(name: &str) -> bool {
    std::env::var(CHILD).is_ok_and(|child| child == name)
}

#[test]
fn without_the_variable_an_uninstalled_read_panics() {
    if !is_child("without_the_variable_an_uninstalled_read_panics") {
        let (ok, text) = in_child("without_the_variable_an_uninstalled_read_panics", false);
        assert!(ok, "{text}");
        return;
    }
    let read = std::panic::catch_unwind(CommandRegistry::build_default);
    let message = read.expect_err("a read before any install panics");
    let message = message
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| message.downcast_ref::<&str>().copied())
        .expect("a string panic");
    assert_eq!(message, NOT_INSTALLED);
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

/// The release-bundle guard: run under `cargo test --release`, this fails if
/// a build without `debug_assertions` still has the fallback.
#[test]
fn with_the_variable_only_a_debug_build_installs_the_shipped_list() {
    if !is_child("with_the_variable_only_a_debug_build_installs_the_shipped_list") {
        let (ok, text) = in_child(
            "with_the_variable_only_a_debug_build_installs_the_shipped_list",
            true,
        );
        assert!(ok, "{text}");
        return;
    }
    let read = std::panic::catch_unwind(CommandRegistry::build_default);
    if cfg!(debug_assertions) {
        let registry = read.expect("a debug build installs the shipped list");
        assert!(registry.get("set").is_some());
        let set = tcl_registry::try_installed().expect("the fallback installed");
        assert_eq!(set.surfaces().len(), SHIPPED.len());
    } else {
        assert!(read.is_err(), "a release build has no fallback");
        assert!(tcl_registry::try_installed().is_none());
    }
}
