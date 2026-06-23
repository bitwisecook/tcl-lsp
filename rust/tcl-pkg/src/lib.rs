//! `tcl-pkg` — the `tclpkg` package manager.
//!
//! Manifest loader, MVS resolver, lockfile I/O, content-addressable store,
//! source fetchers, registry client, virtual
//! environments, and Dockerfile generation. The `tcl pkg` / `tcl venv` /
//! `tcl docker` CLI verb groups in `tcl-cli` drive these modules; behaviour and
//! on-disk formats match the Python implementation byte-for-byte.

pub mod cas;
pub mod docker;
pub mod errors;
pub mod fetchers;
pub mod installer;
pub mod json;
pub mod lockfile;
pub mod manifest;
pub mod registry;
pub mod resolver;
pub mod ui;
pub mod venv;
pub mod version;

pub use errors::{Category, TclPkgError};
pub use version::{Version, VersionError, max_version, parse_version};

use std::path::PathBuf;

/// Return the cache directory using platform-native conventions, mirroring
/// `shared.user_config._cache_dir()`. `$XDG_CACHE_HOME` always wins; otherwise
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
