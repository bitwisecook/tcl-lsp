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

//! `tcl-pkg` — the `tclpkg` package manager.
//!
//! Manifest loader, MVS resolver, lockfile I/O, content-addressable store,
//! source fetchers, registry client, virtual
//! environments, and Dockerfile generation. The `tcl pkg` / `tcl venv` /
//! `tcl docker` CLI verb groups in `tcl-cli` drive these modules; behaviour and
//! on-disk formats are stable and canonical.

pub mod cas;
pub mod docker;
pub mod errors;
pub mod exec;
pub mod fetchers;
pub mod hooks;
pub mod installer;
pub mod json;
pub mod lockfile;
pub mod manifest;
pub mod policy;
pub mod registry;
pub mod resolver;
pub mod ui;
pub mod venv;
pub mod version;

pub use errors::{Category, TclPkgError};
pub use version::{Version, VersionError, max_version, parse_version};

use std::path::PathBuf;

/// Return the cache directory using platform-native conventions.
/// `$XDG_CACHE_HOME` always wins; otherwise
/// `%LOCALAPPDATA%/tcl-lsp/Cache` (native Windows), `~/Library/Caches/tcl-lsp`
/// (macOS), or `~/.cache/tcl-lsp` (Linux/BSD/WSL/MSYS).
#[must_use]
pub fn cache_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("tcl-lsp");
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("tcl-lsp").join("Cache");
        }
    }
    let home = home_dir();
    #[cfg(target_os = "macos")]
    {
        return home.join("Library").join("Caches").join("tcl-lsp");
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".cache").join("tcl-lsp")
    }
}

/// The discoverable venv pool directory (`~/.tcl-venvs`), mirroring
/// `tcl venv list`'s home-pool scan.
#[must_use]
pub fn venv_pool_dir() -> PathBuf {
    home_dir().join(".tcl-venvs")
}

/// The per-user configuration directory. `$XDG_CONFIG_HOME` wins, otherwise
/// `%APPDATA%/tcl-lsp` (Windows), `~/Library/Application Support/tcl-lsp`
/// (macOS), or `~/.config/tcl-lsp` (Linux/BSD). This is where the per-user
/// `pkg.toml` policy layer lives.
#[must_use]
pub fn config_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("tcl-lsp");
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("tcl-lsp");
        }
    }
    let home = home_dir();
    #[cfg(target_os = "macos")]
    {
        return home
            .join("Library")
            .join("Application Support")
            .join("tcl-lsp");
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".config").join("tcl-lsp")
    }
}

/// The system-wide (operator) configuration directory. `/etc/tcl-lsp` on
/// POSIX (including macOS, deliberately, so corporate provisioning lands in one
/// well-known root-owned place) and `%PROGRAMDATA%/tcl-lsp` on Windows. The
/// locked-down policy layer lives here; it is honoured only when the file is
/// owned by a privileged account and not world-writable.
#[must_use]
pub fn system_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(pd) = std::env::var_os("PROGRAMDATA") {
            return PathBuf::from(pd).join("tcl-lsp");
        }
        return PathBuf::from(r"C:\ProgramData\tcl-lsp");
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/etc/tcl-lsp")
    }
}

/// The directory for mutable runtime state (the sandbox audit log, etc.):
/// `$XDG_STATE_HOME/tcl-lsp` when set, otherwise alongside the cache.
#[must_use]
pub fn state_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("tcl-lsp");
    }
    cache_dir().join("state")
}

fn home_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME")
        && !home.is_empty()
    {
        return PathBuf::from(home);
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(profile);
    }
    PathBuf::from(".")
}
