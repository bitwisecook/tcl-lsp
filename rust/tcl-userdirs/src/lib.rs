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

//! Per-user directory conventions for every tcl-lsp tool.
//!
//! One implementation of "where does this platform keep a user's cache /
//! config / state", so `tcl pkg`, the LSP server's spec-pack loader, and
//! anything else that needs a well-known path agree byte for byte.
//!
//! This started life inside `tcl-pkg` (which still re-exports it, so
//! `tcl_pkg::cache_dir()` keeps working). It was lifted out when the LSP
//! server needed the *same* directories for `SpecTcl` packs: depending on the
//! package manager — an HTTP client, a tar/zip reader, a resolver — to learn
//! where `$XDG_CACHE_HOME` points would have been a layering inversion.
//!
//! A true leaf: no dependencies, no filesystem access, no directory creation.
//! Every function is a pure computation over environment variables, so the
//! answer is testable and a caller decides when (and whether) to create
//! anything.

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
        home.join("Library").join("Caches").join("tcl-lsp")
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
/// `pkg.toml` policy layer and the user-tier `SpecTcl` packs live.
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
        home.join("Library")
            .join("Application Support")
            .join("tcl-lsp")
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every directory is *derived*, never invented: each one ends in the
    /// product name so an errant `rm -rf` target is obvious, and none of them
    /// touches the filesystem to answer.
    #[test]
    fn every_dir_is_product_scoped() {
        for dir in [
            cache_dir(),
            config_dir(),
            system_config_dir(),
            state_dir(),
        ] {
            assert!(
                dir.components().any(|c| c.as_os_str() == "tcl-lsp"),
                "{} is not scoped to the product",
                dir.display()
            );
        }
    }

    /// `state_dir` falls back *alongside* the cache rather than into it by
    /// name, so clearing the disposable cache root cannot take live state with
    /// it unless `$XDG_STATE_HOME` says otherwise.
    #[test]
    fn state_dir_defaults_under_the_cache_root() {
        if std::env::var_os("XDG_STATE_HOME").is_some_and(|v| !v.is_empty()) {
            return;
        }
        assert_eq!(state_dir(), cache_dir().join("state"));
    }
}
