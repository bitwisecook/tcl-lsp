// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::PathBuf;
use zed_extension_api::{self as zed, LanguageServerId, Result};

/// GitHub repository that publishes the native server release assets.
const REPO: &str = "bitwisecook/tcl-lsp";

/// Zed's registry builds this source tree directly, so the checked-in
/// extension manifest is the release-version source of truth.
const EXTENSION_MANIFEST: &str = include_str!("../extension.toml");

struct TclExtension;

// Helpers
//
// Historically the extension embedded a single native `tcl-lsp-server`
// binary at compile time and materialised it on first use. That is
// fundamentally broken: a Zed extension is one cross-platform WASM module,
// so whatever host built the release (Linux x86_64 in CI) had *its* binary
// baked in, and every other platform received a binary it could not run
// ("%1 is not a valid Win32 application" on Windows — see issue #826).
//
// Instead we detect the user's platform at runtime and download the matching
// prebuilt binary from the GitHub release, exactly like most native Zed
// extensions. Dev builds still fall back to a binary on PATH.

/// True only for an exact release-tag version — three dot-separated, non-empty,
/// all-digit components (`2.1.4`). This deliberately rejects the `0.0.0-dev`
/// sentinel, `git describe` strings (`2.1.4-3-gabc1234`), and bare commit shas,
/// none of which correspond to a `v<version>` release that carries assets.
fn is_release_version(v: &str) -> bool {
    let mut parts = v.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    [major, minor, patch]
        .iter()
        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The concrete release version to resolve assets for, or `None` when this is
/// a from-source / dev build that should use the PATH / latest-release fallback.
fn pinned_version() -> Option<&'static str> {
    EXTENSION_MANIFEST.lines().find_map(|line| {
        let value = line.strip_prefix("version = \"")?.strip_suffix('"')?;
        is_release_version(value).then_some(value)
    })
}

/// Map a Zed platform to the Rust target triple used in our release asset
/// names (kept in sync with `SERVER_TARGET_MAP` in the Makefile). Returns
/// `None` for platforms we do not publish a prebuilt binary for.
fn target_triple(os: zed::Os, arch: zed::Architecture) -> Option<&'static str> {
    use zed::{Architecture as A, Os as O};
    Some(match (os, arch) {
        (O::Mac, A::Aarch64) => "aarch64-apple-darwin",
        (O::Mac, A::X8664) => "x86_64-apple-darwin",
        (O::Linux, A::Aarch64) => "aarch64-unknown-linux-gnu",
        (O::Linux, A::X8664) => "x86_64-unknown-linux-gnu",
        (O::Windows, A::Aarch64) => "aarch64-pc-windows-msvc",
        (O::Windows, A::X8664) => "x86_64-pc-windows-msvc",
        _ => return None,
    })
}

/// Executable suffix for the target platform. This is derived from the
/// runtime [`zed::current_platform`] OS, NOT `#[cfg(target_os)]` — the
/// extension is compiled to `wasm32-wasip2`, so a `cfg` check always reports
/// the WASM host and would wrongly drop the `.exe` on Windows.
fn exe_suffix(os: zed::Os) -> &'static str {
    match os {
        zed::Os::Windows => ".exe",
        _ => "",
    }
}

/// Release asset name for a server binary on a given platform, e.g.
/// `tcl-lsp-server-x86_64-pc-windows-msvc.exe`.
fn asset_name(base: &str, triple: &str, exe: &str) -> String {
    format!("{base}-{triple}{exe}")
}

/// Direct download URL for a release asset pinned to a specific tag.
fn release_download_url(repo: &str, tag: &str, asset: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
}

/// Convert a relative path in the extension sandbox to an absolute path.
/// Zed runs language server commands with the project folder as CWD, so
/// any paths we return from the extension must be absolute.
fn abs_path(relative: &str) -> String {
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.join(relative).to_string_lossy().into_owned()
}

/// Remove cached binary directories from previous versions (best effort), so
/// the extension's writable dir does not accumulate every server it has ever
/// downloaded. Also sweeps up the legacy `tcl-lsp-bundled-*` dirs.
fn prune_stale_versions(keep: &str) {
    let Ok(entries) = fs::read_dir(".") else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("tcl-lsp-") && name != keep {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Resolve the release version and download URL for `asset`. When packaged
/// with a concrete version we hit the pinned tag directly; otherwise we ask
/// GitHub for the latest release and locate the asset by name.
fn resolve_asset_url(asset: &str) -> Result<(String, String)> {
    if let Some(v) = pinned_version() {
        let tag = format!("v{v}");
        return Ok((v.to_string(), release_download_url(REPO, &tag, asset)));
    }

    let release = zed::latest_github_release(
        REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            // Include pre-releases: the active 2.x line ships as GitHub
            // pre-releases, so stable-only lookups would miss it entirely.
            pre_release: true,
        },
    )?;
    let url = release
        .assets
        .iter()
        .find(|a| a.name == asset)
        .map(|a| a.download_url.clone())
        .ok_or_else(|| format!("release {} has no asset named `{asset}`", release.version))?;
    Ok((release.version, url))
}

/// Download (once per version) the prebuilt native binary for the current
/// platform, make it executable, and return its absolute path.
fn ensure_downloaded_binary(server_id: &LanguageServerId, base: &str) -> Result<String> {
    let (os, arch) = zed::current_platform();
    let triple = target_triple(os, arch).ok_or_else(|| {
        format!("tcl-lsp does not publish a prebuilt `{base}` for this platform ({os:?}/{arch:?})")
    })?;
    let exe = exe_suffix(os);
    let asset = asset_name(base, triple, exe);

    zed::set_language_server_installation_status(
        server_id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    let (version, url) = resolve_asset_url(&asset)?;

    let dir = format!("tcl-lsp-{version}");
    let file_name = format!("{base}{exe}");
    let path = format!("{dir}/{file_name}");

    let already_present = fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false);
    if !already_present {
        zed::set_language_server_installation_status(
            server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        fs::create_dir_all(&dir).map_err(|e| format!("failed to create dir {dir}: {e}"))?;
        zed::download_file(&url, &path, zed::DownloadedFileType::Uncompressed)
            .map_err(|e| format!("failed to download {asset}: {e}"))?;
        prune_stale_versions(&dir);
    }

    zed::make_file_executable(&path)
        .map_err(|e| format!("failed to make {file_name} executable: {e}"))?;
    Ok(abs_path(&path))
}

/// Resolve the language server binary path. An explicit PATH installation
/// wins; otherwise the extension downloads the build matching its manifest.
fn resolve_lsp_path(server_id: &LanguageServerId, worktree: &zed::Worktree) -> Result<String> {
    let (os, _) = zed::current_platform();
    let file_name = format!("tcl-lsp-server{}", exe_suffix(os));
    if let Some(path) = worktree.which(&file_name) {
        return Ok(path);
    }
    ensure_downloaded_binary(server_id, "tcl-lsp-server")
}

// Extension trait implementation

impl zed::Extension for TclExtension {
    fn new() -> Self {
        TclExtension
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // The native server speaks LSP over stdio with no args.
        let server_path = resolve_lsp_path(language_server_id, worktree)?;

        Ok(zed::Command {
            command: server_path,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = zed::settings::LspSettings::for_worktree("tcl-lsp", worktree)?;
        Ok(settings.settings)
    }
}

zed::register_extension!(TclExtension);

#[cfg(test)]
mod tests {
    use super::*;
    use zed::{Architecture, Os};

    #[test]
    fn triple_covers_every_published_platform() {
        assert_eq!(
            target_triple(Os::Mac, Architecture::Aarch64),
            Some("aarch64-apple-darwin")
        );
        assert_eq!(
            target_triple(Os::Mac, Architecture::X8664),
            Some("x86_64-apple-darwin")
        );
        assert_eq!(
            target_triple(Os::Linux, Architecture::Aarch64),
            Some("aarch64-unknown-linux-gnu")
        );
        assert_eq!(
            target_triple(Os::Linux, Architecture::X8664),
            Some("x86_64-unknown-linux-gnu")
        );
        assert_eq!(
            target_triple(Os::Windows, Architecture::Aarch64),
            Some("aarch64-pc-windows-msvc")
        );
        assert_eq!(
            target_triple(Os::Windows, Architecture::X8664),
            Some("x86_64-pc-windows-msvc")
        );
    }

    #[test]
    fn unsupported_platforms_have_no_triple() {
        // 32-bit x86 is not a target we publish for any OS.
        assert_eq!(target_triple(Os::Linux, Architecture::X86), None);
        assert_eq!(target_triple(Os::Windows, Architecture::X86), None);
        assert_eq!(target_triple(Os::Mac, Architecture::X86), None);
    }

    #[test]
    fn only_windows_carries_an_exe_suffix() {
        assert_eq!(exe_suffix(Os::Windows), ".exe");
        assert_eq!(exe_suffix(Os::Mac), "");
        assert_eq!(exe_suffix(Os::Linux), "");
    }

    #[test]
    fn windows_asset_name_keeps_the_exe_suffix() {
        // Regression guard for #826: the Windows asset must be a `.exe`.
        let triple = target_triple(Os::Windows, Architecture::X8664).unwrap();
        let exe = exe_suffix(Os::Windows);
        assert_eq!(
            asset_name("tcl-lsp-server", triple, exe),
            "tcl-lsp-server-x86_64-pc-windows-msvc.exe"
        );
    }

    #[test]
    fn unix_asset_name_has_no_suffix() {
        let triple = target_triple(Os::Linux, Architecture::X8664).unwrap();
        let exe = exe_suffix(Os::Linux);
        assert_eq!(
            asset_name("tcl-lsp-server", triple, exe),
            "tcl-lsp-server-x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn download_url_points_at_the_tagged_release() {
        assert_eq!(
            release_download_url(
                "bitwisecook/tcl-lsp",
                "v2.1.5",
                "tcl-lsp-server-x86_64-pc-windows-msvc.exe"
            ),
            "https://github.com/bitwisecook/tcl-lsp/releases/download/\
             v2.1.5/tcl-lsp-server-x86_64-pc-windows-msvc.exe"
        );
    }

    #[test]
    fn only_exact_release_tags_pin_downloads() {
        // Exact release tags pin.
        assert!(is_release_version("2.1.5"));
        assert!(is_release_version("0.0.1"));
        assert!(is_release_version("10.20.30"));

        // Everything a non-release build might inject must fall back instead
        // of trying to download from a `v<version>` tag that does not exist.
        assert!(!is_release_version("0.0.0-dev")); // dev sentinel
        assert!(!is_release_version("2.1.4-3-gabc1234")); // git describe
        assert!(!is_release_version("abc1234")); // bare commit sha
        assert!(!is_release_version("2.1")); // too few components
        assert!(!is_release_version("2.1.5.1")); // too many components
        assert!(!is_release_version("2.1.x")); // non-numeric
        assert!(!is_release_version("v2.1.5")); // leading v not stripped
        assert!(!is_release_version("")); // empty
    }
}
