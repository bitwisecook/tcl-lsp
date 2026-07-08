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

use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use zed_extension_api::{self as zed, LanguageServerId, Result};

/// GitHub repository that publishes the native server release assets.
const REPO: &str = "bitwisecook/tcl-lsp";

/// The release version this extension was packaged for, injected by the
/// Makefile (`TCL_LSP_BUNDLED_VERSION`) at package time. Only an exact release
/// tag (`X.Y.Z`) is honoured — see [`pinned_version`]. When it is anything else
/// (the `0.0.0-dev` sentinel, a `git describe` string like `2.1.4-3-gabc1234`
/// from a local dev build, a bare commit sha, or absent because Zed's registry
/// builder compiled straight from source), we fall back to a binary on PATH /
/// the latest GitHub release instead of pinning to a tag that does not exist.
const PACKAGED_VERSION: Option<&str> = option_env!("TCL_LSP_BUNDLED_VERSION");

struct TclExtension {
    cached_server_id: Option<LanguageServerId>,
}

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
    PACKAGED_VERSION.filter(|v| is_release_version(v))
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
fn ensure_downloaded_binary(server_id: Option<&LanguageServerId>, base: &str) -> Result<String> {
    let (os, arch) = zed::current_platform();
    let triple = target_triple(os, arch).ok_or_else(|| {
        format!("tcl-lsp does not publish a prebuilt `{base}` for this platform ({os:?}/{arch:?})")
    })?;
    let exe = exe_suffix(os);
    let asset = asset_name(base, triple, exe);

    if let Some(id) = server_id {
        zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
    }

    let (version, url) = resolve_asset_url(&asset)?;

    let dir = format!("tcl-lsp-{version}");
    let file_name = format!("{base}{exe}");
    let path = format!("{dir}/{file_name}");

    let already_present = fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false);
    if !already_present {
        if let Some(id) = server_id {
            zed::set_language_server_installation_status(
                id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
        }
        fs::create_dir_all(&dir).map_err(|e| format!("failed to create dir {dir}: {e}"))?;
        zed::download_file(&url, &path, zed::DownloadedFileType::Uncompressed)
            .map_err(|e| format!("failed to download {asset}: {e}"))?;
        prune_stale_versions(&dir);
    }

    zed::make_file_executable(&path)
        .map_err(|e| format!("failed to make {file_name} executable: {e}"))?;
    Ok(abs_path(&path))
}

/// Resolve the language server binary path. Released builds download the
/// prebuilt binary for the user's platform; dev builds prefer a
/// locally-built `tcl-lsp-server` on PATH, falling back to a download of the
/// latest release.
fn resolve_lsp_path(server_id: &LanguageServerId, worktree: &zed::Worktree) -> Result<String> {
    if pinned_version().is_none() {
        let (os, _) = zed::current_platform();
        let file_name = format!("tcl-lsp-server{}", exe_suffix(os));
        if let Some(path) = worktree.which(&file_name) {
            return Ok(path);
        }
    }
    ensure_downloaded_binary(Some(server_id), "tcl-lsp-server")
}

// Tcl/iRules reference data for slash-command argument completions.
// Loaded from generated catalogs at compile time — run `make generate`
// to update after registry changes.

static TCL_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let json: serde_json::Value = serde_json::from_str(include_str!("generated/tcl_commands.json"))
        .expect("generated/tcl_commands.json is valid JSON");
    json["commands"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
});

static IRULE_EVENTS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let json: serde_json::Value = serde_json::from_str(include_str!("generated/irule_events.json"))
        .expect("generated/irule_events.json is valid JSON");
    json["events"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
});

// Extension trait implementation

impl zed::Extension for TclExtension {
    fn new() -> Self {
        TclExtension {
            cached_server_id: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        self.cached_server_id = Some(language_server_id.clone());

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

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: zed::lsp::Completion,
    ) -> Option<zed::CodeLabel> {
        let label = &completion.label;
        let kind = completion.kind?;

        match kind {
            zed::lsp::CompletionKind::Variable => {
                // Variable completions: highlight "$" prefix distinctly.
                let mut spans = Vec::new();
                if let Some(rest) = label.strip_prefix('$') {
                    spans.push(zed::CodeLabelSpan::literal(
                        "$",
                        Some("punctuation.special".into()),
                    ));
                    if !rest.is_empty() {
                        spans.push(zed::CodeLabelSpan::literal(rest, Some("variable".into())));
                    }
                } else {
                    spans.push(zed::CodeLabelSpan::literal(label, Some("variable".into())));
                }
                Some(zed::CodeLabel {
                    code: label.clone(),
                    spans,
                    filter_range: (0..label.len()).into(),
                })
            }

            zed::lsp::CompletionKind::Function => {
                // Commands: highlight namespace separator "::".
                let mut spans = Vec::new();
                let parts: Vec<&str> = label.split("::").collect();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        spans.push(zed::CodeLabelSpan::literal(
                            "::",
                            Some("punctuation.delimiter".into()),
                        ));
                    }
                    spans.push(zed::CodeLabelSpan::literal(*part, Some("function".into())));
                }
                // Append detail (signature) if present.
                let code = if let Some(ref detail) = completion.detail {
                    spans.push(zed::CodeLabelSpan::literal(
                        format!(" {detail}"),
                        Some("comment".into()),
                    ));
                    format!("{label} {detail}")
                } else {
                    label.clone()
                };
                Some(zed::CodeLabel {
                    code,
                    spans,
                    filter_range: (0..label.len()).into(),
                })
            }

            zed::lsp::CompletionKind::Keyword => {
                // Switches/keywords: highlight "-" prefix.
                let mut spans = Vec::new();
                if let Some(rest) = label.strip_prefix('-') {
                    spans.push(zed::CodeLabelSpan::literal("-", Some("punctuation".into())));
                    if !rest.is_empty() {
                        spans.push(zed::CodeLabelSpan::literal(rest, Some("keyword".into())));
                    }
                } else {
                    spans.push(zed::CodeLabelSpan::literal(label, Some("keyword".into())));
                }
                Some(zed::CodeLabel {
                    code: label.clone(),
                    spans,
                    filter_range: (0..label.len()).into(),
                })
            }

            _ => None,
        }
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: zed::lsp::Symbol,
    ) -> Option<zed::CodeLabel> {
        let name = &symbol.name;
        let mut spans = Vec::new();

        match symbol.kind {
            zed::lsp::SymbolKind::Function => {
                spans.push(zed::CodeLabelSpan::literal("proc ", Some("keyword".into())));
                let parts: Vec<&str> = name.split("::").collect();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        spans.push(zed::CodeLabelSpan::literal(
                            "::",
                            Some("punctuation.delimiter".into()),
                        ));
                    }
                    spans.push(zed::CodeLabelSpan::literal(
                        *part,
                        Some("entity.name.function".into()),
                    ));
                }
            }
            zed::lsp::SymbolKind::Variable => {
                spans.push(zed::CodeLabelSpan::literal(
                    "$",
                    Some("punctuation.special".into()),
                ));
                spans.push(zed::CodeLabelSpan::literal(name, Some("variable".into())));
            }
            zed::lsp::SymbolKind::Namespace => {
                spans.push(zed::CodeLabelSpan::literal(
                    "namespace ",
                    Some("keyword".into()),
                ));
                spans.push(zed::CodeLabelSpan::literal(
                    name,
                    Some("entity.name.namespace".into()),
                ));
            }
            _ => {
                spans.push(zed::CodeLabelSpan::literal(name, None));
            }
        }

        Some(zed::CodeLabel {
            code: format!("proc {name} {{}}"),
            spans,
            filter_range: (0..name.len()).into(),
        })
    }

    fn complete_slash_command_argument(
        &self,
        command: zed::SlashCommand,
        args: Vec<String>,
    ) -> Result<Vec<zed::SlashCommandArgumentCompletion>, String> {
        let query = args.first().map(|s| s.to_lowercase()).unwrap_or_default();

        match command.name.as_str() {
            "tcl-doc" => Ok(TCL_COMMANDS
                .iter()
                .filter(|c| query.is_empty() || c.to_lowercase().starts_with(&query))
                .map(|c| zed::SlashCommandArgumentCompletion {
                    label: c.to_string(),
                    new_text: c.to_string(),
                    run_command: true,
                })
                .collect()),

            "irule-event" => Ok(IRULE_EVENTS
                .iter()
                .filter(|e| query.is_empty() || e.to_lowercase().starts_with(&query))
                .map(|e| zed::SlashCommandArgumentCompletion {
                    label: e.to_string(),
                    new_text: e.to_string(),
                    run_command: true,
                })
                .collect()),

            _ => Ok(Vec::new()),
        }
    }

    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        args: Vec<String>,
        _worktree: Option<&zed::Worktree>,
    ) -> Result<zed::SlashCommandOutput, String> {
        match command.name.as_str() {
            "tcl-doc" => {
                let cmd_name = args
                    .first()
                    .ok_or("Provide a command name, e.g. /tcl-doc HTTP::host")?;
                let text = format!(
                    "# Tcl/iRules Command: {cmd_name}\n\n\
                     Hover over `{cmd_name}` in your code for inline documentation \
                     from tcl-lsp.\n\n\
                     For full analysis, use the **tcl-lsp-mcp** context server which \
                     exposes the `command_info` tool with synopsis, switches, valid \
                     events, and deprecation status.\n\n\
                     ## Usage\n\
                     The `command_info` MCP tool accepts `{{\"command_name\": \"{cmd_name}\"}}` \
                     and returns structured metadata."
                );
                let len = text.len();
                Ok(zed::SlashCommandOutput {
                    text,
                    sections: vec![zed::SlashCommandOutputSection {
                        range: (0..len).into(),
                        label: format!("tcl-doc: {cmd_name}"),
                    }],
                })
            }

            "irule-event" => {
                let event = args
                    .first()
                    .ok_or("Provide an event name, e.g. /irule-event HTTP_REQUEST")?;
                let text = format!(
                    "# iRules Event: {event}\n\n\
                     Use the **tcl-lsp-mcp** context server for detailed event metadata.\n\n\
                     The `event_info` MCP tool accepts `{{\"event_name\": \"{event}\"}}` and \
                     returns:\n\
                     - Valid commands for this event\n\
                     - Deprecation status\n\
                     - Event properties and firing order\n\
                     - Related events\n\n\
                     The `event_order` tool shows the canonical firing sequence for \
                     all events in an iRule."
                );
                let len = text.len();
                Ok(zed::SlashCommandOutput {
                    text,
                    sections: vec![zed::SlashCommandOutputSection {
                        range: (0..len).into(),
                        label: format!("irule-event: {event}"),
                    }],
                })
            }

            "tcl-validate" => {
                let text = "# Tcl/iRules Validation\n\n\
                    Check the **Diagnostics** panel for tcl-lsp validation results. \
                    The language server automatically validates open files and reports \
                    errors, warnings, security issues, and style suggestions.\n\n\
                    For programmatic validation, use the **tcl-lsp-mcp** context server \
                    which exposes these tools:\n\
                    - `validate` — categorised report (errors, security, taint, performance, style)\n\
                    - `review` — security-focused analysis (taint tracking, thread safety)\n\
                    - `analyze` — full analysis (diagnostics + symbols + events + event ordering)\n\
                    - `optimize` — optimisation opportunities with rewritten source"
                    .to_string();
                let len = text.len();
                Ok(zed::SlashCommandOutput {
                    text,
                    sections: vec![zed::SlashCommandOutputSection {
                        range: (0..len).into(),
                        label: "tcl-validate".into(),
                    }],
                })
            }

            "irule-test" => {
                let text = "# iRule Test Generation\n\n\
                    Generate a test script for your iRule using the **Event Orchestrator** \
                    framework. The test framework simulates the BIG-IP event lifecycle, \
                    pools, data groups, and multi-TMM CMP behavior.\n\n\
                    ## MCP tools\n\n\
                    Use the **tcl-lsp-mcp** context server with these tools:\n\
                    - `generate_irule_test` — generate a complete test script from iRule source\n\
                    - `fakecmp_which_tmm` — look up which TMM a connection tuple maps to\n\
                    - `fakecmp_suggest_sources` — find client addr/port combos that hit each TMM\n\n\
                    ## Quick start\n\n\
                    1. Select your iRule source code\n\
                    2. Ask the assistant to generate tests using `generate_irule_test`\n\
                    3. Run with `tclsh test_my_irule.tcl`\n\n\
                    ## Multi-TMM testing\n\n\
                    For iRules that use `static::` variables in hot events or `table` for \
                    shared state, the generator auto-detects CMP-sensitive patterns and adds \
                    multi-TMM test scenarios using **fakeCMP** (a simulated hash, not the real \
                    BIG-IP CMP algorithm).\n\n\
                    Use `fakecmp_suggest_sources` to plan which client addresses hit which TMMs \
                    before writing tests."
                    .to_string();
                let len = text.len();
                Ok(zed::SlashCommandOutput {
                    text,
                    sections: vec![zed::SlashCommandOutputSection {
                        range: (0..len).into(),
                        label: "irule-test".into(),
                    }],
                })
            }

            _ => Err(format!("Unknown slash command: {}", command.name)),
        }
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> Result<zed::Command> {
        // Download the platform-matched MCP binary — pinned to the packaged
        // tag for released builds, else the latest release. Unlike the language
        // server, there is no `Worktree` here to prefer a PATH build via
        // `which` (`Project` only exposes worktree ids), so a from-source /
        // registry build with nothing on PATH would otherwise be unable to
        // start the context server despite having the download plumbing. The
        // MCP server speaks over stdio with no args.
        let mcp_path = ensure_downloaded_binary(None, "tcl-mcp")?;

        Ok(zed::Command {
            command: mcp_path,
            args: vec![],
            env: Default::default(),
        })
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
            asset_name("tcl-mcp", triple, exe),
            "tcl-mcp-x86_64-unknown-linux-gnu"
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
