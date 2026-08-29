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

//! Discovery of Tcl installations on disk and the `libraryPaths` config the
//! package resolver scans as its `auto_path`.
//!
//! Two concerns, both filesystem-only — **`tclsh` is never executed**:
//!
//! * [`discover`] finds Tcl installations by looking in common install
//!   locations ([`default_search_bases`]) for a Tcl *library* directory (one
//!   containing `init.tcl`). Each becomes a [`TclInstallation`] whose
//!   `auto_path` roots the resolver can scan for `pkgIndex.tcl` files —
//!   surfacing system packages like Tk / tcllib, not just workspace ones.
//!   The list is offered to the user so they can pick one (or supply their own
//!   path).
//!
//! * [`library_paths_from_ini`] / [`user_config_path`] / [`project_config_path`]
//!   read the LSP's own config: the `libraryPaths`
//!   key in the platform-native user config (`config.ini`, `[global]` section)
//!   and the per-workspace `.tcl-lsp.ini` (`[project]` section), plus the
//!   editor's `tclLsp.libraryPaths`.

use std::path::{Path, PathBuf};

/// A Tcl installation discovered on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TclInstallation {
    /// Best-effort version label (e.g. `8.6`, `9.0`), from the library
    /// directory name or an enclosing path component; empty when unknown.
    pub version: String,
    /// The Tcl library directory (the one containing `init.tcl`).
    pub tcl_library: PathBuf,
    /// `auto_path`-style roots to scan for `pkgIndex.tcl` — the library
    /// directory and its parent (where sibling packages such as `tcllib`,
    /// `tk8.6`, … live).
    pub auto_path: Vec<PathBuf>,
}

/// Common base directories Tcl libraries install under, per platform, plus
/// `$TCL_LIBRARY` and `$TCLLIBPATH` when set. Only existing directories are
/// worth scanning, but existence is checked by [`discover`], not here.
#[must_use]
pub fn default_search_bases() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();

    // A directly-pointed library dir wins: its parent is a base.
    if let Some(lib) = std::env::var_os("TCL_LIBRARY") {
        let lib = PathBuf::from(lib);
        if let Some(parent) = lib.parent() {
            bases.push(parent.to_path_buf());
        }
        bases.push(lib);
    }
    for entry in tcllibpath_dirs() {
        bases.push(entry);
    }

    let common: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Tcl\lib",
            r"C:\ActiveTcl\lib",
            r"C:\Program Files\Tcl\lib",
            r"C:\Program Files (x86)\Tcl\lib",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/Library/Frameworks/Tcl.framework/Versions/Current/Resources/Scripts",
            "/System/Library/Frameworks/Tcl.framework/Versions/Current/Resources/Scripts",
            "/opt/homebrew/lib",
            "/opt/homebrew/opt/tcl-tk/lib",
            "/usr/local/opt/tcl-tk/lib",
            "/opt/local/lib",
            "/usr/local/lib",
            "/usr/lib",
        ]
    } else {
        &[
            "/usr/lib",
            "/usr/lib64",
            "/usr/local/lib",
            "/usr/local/lib64",
            "/usr/share/tcltk",
            "/usr/lib/tcltk",
            "/usr/lib/x86_64-linux-gnu",
            "/opt/local/lib",
            "/opt/homebrew/lib",
        ]
    };
    bases.extend(common.iter().map(PathBuf::from));
    bases
}

/// Discover Tcl installations under `bases`.
///
/// A base directory is scanned for an immediate subdirectory that *is* a Tcl
/// library — i.e. contains `init.tcl` (`tcl8.6/`, `tcl9.0/`, the framework's
/// `Scripts/`, a bundled `library/`, …) — and the base itself is treated as a
/// library when it directly contains `init.tcl`. Duplicate installations
/// (same `tcl_library`) are returned once, in discovery order.
#[must_use]
pub fn discover(bases: &[PathBuf]) -> Vec<TclInstallation> {
    let mut out: Vec<TclInstallation> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    let consider = |lib: &Path, out: &mut Vec<TclInstallation>, seen: &mut Vec<PathBuf>| {
        if !lib.join("init.tcl").is_file() {
            return;
        }
        let lib = lib.to_path_buf();
        if seen.contains(&lib) {
            return;
        }
        seen.push(lib.clone());
        let mut auto_path = Vec::new();
        if let Some(parent) = lib.parent() {
            auto_path.push(parent.to_path_buf());
        }
        auto_path.push(lib.clone());
        out.push(TclInstallation {
            version: version_from_path(&lib),
            tcl_library: lib,
            auto_path,
        });
    };

    for base in bases {
        if !base.is_dir() {
            continue;
        }
        // The base may itself be a library dir.
        consider(base, &mut out, &mut seen);
        if let Ok(read) = std::fs::read_dir(base) {
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    consider(&path, &mut out, &mut seen);
                }
            }
        }
    }
    out
}

/// Best-effort Tcl version from a library path: the digits in the last path
/// component that starts with `tcl` (`tcl8.6` → `8.6`), else any `tcl<ver>`
/// component (a bundled `.../tcl9.0.3/library` → `9.0.3`), else empty.
fn version_from_path(lib: &Path) -> String {
    if let Some(name) = lib.file_name().and_then(|n| n.to_str())
        && let Some(rest) = name.strip_prefix("tcl")
        && rest.starts_with(|c: char| c.is_ascii_digit())
    {
        return rest.to_owned();
    }
    for comp in lib.components() {
        if let Some(s) = comp.as_os_str().to_str()
            && let Some(rest) = s.strip_prefix("tcl")
            && rest.starts_with(|c: char| c.is_ascii_digit())
        {
            return rest.to_owned();
        }
    }
    String::new()
}

/// The directories on `$TCLLIBPATH` (a Tcl list — whitespace- or
/// brace-separated). Empty when unset.
#[must_use]
pub fn tcllibpath_dirs() -> Vec<PathBuf> {
    let Ok(raw) = std::env::var("TCLLIBPATH") else {
        return Vec::new();
    };
    raw.split_whitespace()
        .map(|s| s.trim_matches(['{', '}']))
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

// Config-file `libraryPaths`.

/// The platform-native user config file (`config.ini`):
///
/// * `$XDG_CONFIG_HOME/tcl-lsp/config.ini` when `XDG_CONFIG_HOME` is set,
/// * Windows (native): `%APPDATA%\tcl-lsp\config.ini`,
/// * Windows under MSYS2 / Cygwin (`MSYSTEM` set): `~/.config/tcl-lsp/config.ini`,
/// * macOS: `~/Library/Application Support/tcl-lsp/config.ini`,
/// * else (Linux/BSD/WSL): `~/.config/tcl-lsp/config.ini`.
///
/// `None` when the home/appdata directory can't be determined.
#[must_use]
pub fn user_config_path() -> Option<PathBuf> {
    config_path_for(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("APPDATA").as_deref(),
        std::env::var_os("HOME").as_deref(),
        cfg!(target_os = "windows"),
        cfg!(target_os = "macos"),
        std::env::var_os("MSYSTEM").is_some(),
    )
}

/// Pure core of [`user_config_path`] — resolves the config file from the
/// relevant environment values and platform flags, so the precedence is
/// testable without mutating process environment. `posix_compat_windows` is
/// `true` under an MSYS2 / Cygwin shell (`MSYSTEM` set), where the XDG `~/.config`
/// convention is used instead of `%APPDATA%`.
fn config_path_for(
    xdg_config_home: Option<&std::ffi::OsStr>,
    appdata: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
    is_windows: bool,
    is_macos: bool,
    posix_compat_windows: bool,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg_config_home
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("tcl-lsp").join("config.ini"));
    }
    // Native Windows → %APPDATA%; under MSYS2 / Cygwin fall through to the XDG
    // `~/.config` default below.
    if is_windows && !posix_compat_windows {
        return appdata.map(|a| PathBuf::from(a).join("tcl-lsp").join("config.ini"));
    }
    let home = PathBuf::from(home?);
    if is_macos {
        return Some(
            home.join("Library")
                .join("Application Support")
                .join("tcl-lsp")
                .join("config.ini"),
        );
    }
    Some(home.join(".config").join("tcl-lsp").join("config.ini"))
}

/// Filename of the per-project config.
pub const PROJECT_CONFIG_FILENAME: &str = ".tcl-lsp.ini";

/// The per-project config file for `workspace_root` (`<root>/.tcl-lsp.ini`).
#[must_use]
pub fn project_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(PROJECT_CONFIG_FILENAME)
}

/// Parse the `libraryPaths` list from `section` of an INI `content`: a
/// newline-separated list (with indented continuation lines), falling back to
/// comma-split for a one-liner. Returns the paths in order; empty when absent.
#[must_use]
pub fn library_paths_from_ini(content: &str, section: &str) -> Vec<String> {
    let Some(raw) = ini_raw_value(content, section, "libraryPaths") else {
        return Vec::new();
    };
    let lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() <= 1 && raw.contains(',') {
        return raw
            .replace(',', " ")
            .split_whitespace()
            .map(str::to_owned)
            .collect();
    }
    lines
}

/// The raw value of `key` in `[section]` of an INI `content`, including
/// `configparser`-style indented continuation lines (joined with `\n`).
fn ini_raw_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    let mut value: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim_end();
        let lstripped = trimmed.trim_start();
        // A new section header.
        if lstripped.starts_with('[') && lstripped.ends_with(']') {
            // Stop collecting once we leave the section.
            if value.is_some() {
                break;
            }
            let name = &lstripped[1..lstripped.len() - 1];
            in_section = name == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(v) = value.as_mut() {
            // Continuation: an indented, non-empty, non-key line.
            let indented = line.starts_with([' ', '\t']);
            let is_new_key = lstripped.contains('=') || lstripped.contains(':');
            if indented && !lstripped.is_empty() && !is_new_key {
                v.push('\n');
                v.push_str(lstripped);
                continue;
            }
            // Any other line ends the value.
            break;
        }
        // Look for `key =` / `key:` (comment lines start with # or ;).
        if lstripped.starts_with('#') || lstripped.starts_with(';') {
            continue;
        }
        if let Some(rest) = lstripped
            .strip_prefix(key)
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix(['=', ':']))
        {
            value = Some(rest.trim().to_owned());
        }
    }
    value
}

#[cfg(test)]
mod tests;
