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

//! How a test harness installs its surface set: one line at the top of the
//! harness, run once per process before any test.
//!
//! ```ignore
//! tcl_registry::testing::install_shipped!();
//! ```
//!
//! The line places a start-up function in the platform's constructor
//! section, as the `ctor` crate does; that crate cannot be used under the
//! workspace's `forbid(unsafe_code)`, because its expansion carries an
//! `allow` the `forbid` rejects. A harness that installs two different sets
//! aborts at start-up with the [`crate::InstallConflict`].

pub use crate::__testing_install_shipped as install_shipped;

/// Install [`crate::commands::SHIPPED`] at process start-up; aborts on an
/// [`crate::InstallConflict`].
#[doc(hidden)]
pub extern "C" fn install_shipped_at_start_up() {
    if let Err(conflict) = crate::install_shipped() {
        eprintln!("{conflict}");
        std::process::abort();
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __testing_install_shipped {
    () => {
        #[used]
        #[cfg_attr(
            any(
                target_os = "linux",
                target_os = "android",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly",
                target_os = "illumos"
            ),
            unsafe(link_section = ".init_array")
        )]
        #[cfg_attr(
            target_vendor = "apple",
            unsafe(link_section = "__DATA,__mod_init_func")
        )]
        #[cfg_attr(windows, unsafe(link_section = ".CRT$XCU"))]
        static __TCL_REGISTRY_INSTALL_SHIPPED: extern "C" fn() =
            $crate::testing::install_shipped_at_start_up;
    };
}
