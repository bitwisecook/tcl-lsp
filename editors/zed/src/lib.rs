use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const GITHUB_REPO: &str = "bitwisecook/tcl-lsp";
const LSP_ASSET_PREFIX: &str = "tcl-lsp-server-";
const MCP_ASSET_PREFIX: &str = "tcl-lsp-mcp-server-";

/// Extension version used to namespace bundled asset directories.
const BUNDLED_VERSION: &str = match option_env!("TCL_LSP_BUNDLED_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[cfg(bundled_lsp)]
const BUNDLED_LSP_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/tcl-lsp-server.pyz"));
#[cfg(bundled_mcp)]
const BUNDLED_MCP_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/tcl-lsp-mcp-server.pyz"));

/// Python candidates in descending version order.
const PYTHON_CANDIDATES: &[&str] = &[
    "python3.15",
    "python3.14",
    "python3.13",
    "python3.12",
    "python3.11",
    "python3.10",
    "python3",
];

struct TclExtension {
    cached_server_path: Option<String>,
    cached_mcp_path: Option<String>,
    cached_server_id: Option<LanguageServerId>,
}

// Helpers

/// Find the best Python 3.10+ interpreter via the worktree PATH.
fn find_python(worktree: &zed::Worktree) -> Result<String> {
    for candidate in PYTHON_CANDIDATES {
        if let Some(path) = worktree.which(candidate) {
            return Ok(path);
        }
    }
    Err("Python 3.10+ not found on PATH. Install Python and restart Zed.".into())
}

/// Find the best Python 3.10+ by probing common names without a worktree.
fn find_python_global() -> Result<String> {
    // Without a worktree we cannot call `which`, so try the most common name.
    Ok("python3".to_string())
}

/// Convert a relative path in the extension sandbox to an absolute path.
/// Zed runs language server commands with the project folder as CWD, so
/// any paths we return from the extension must be absolute.
fn abs_path(relative: &str) -> String {
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.join(relative).to_string_lossy().into_owned()
}

/// Download a .pyz asset from the latest GitHub release, caching by version.
fn ensure_asset_downloaded(
    language_server_id: &LanguageServerId,
    asset_prefix: &str,
    cached: &mut Option<String>,
) -> Result<String> {
    // Return cached path if still present on disk.
    if let Some(ref path) = cached {
        if fs::metadata(path).is_ok() {
            return Ok(path.clone());
        }
    }

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    let release = zed::latest_github_release(
        GITHUB_REPO,
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.starts_with(asset_prefix) && a.name.ends_with(".pyz"))
        .ok_or_else(|| {
            format!(
                "No {asset_prefix}*.pyz asset found in release {}",
                release.version
            )
        })?;

    let version_dir = format!("tcl-lsp-{}", release.version);
    let download_path = format!("{version_dir}/{}", asset.name);

    // Already have this version?
    if fs::metadata(&download_path).is_ok() {
        let absolute = abs_path(&download_path);
        *cached = Some(absolute.clone());
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );
        return Ok(absolute);
    }

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
    );

    let _ = fs::create_dir_all(&version_dir);

    zed::download_file(
        &asset.download_url,
        &download_path,
        zed::DownloadedFileType::Uncompressed,
    )?;

    let absolute = abs_path(&download_path);
    *cached = Some(absolute.clone());

    zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::None,
    );

    Ok(absolute)
}

/// Write a compile-time-embedded .pyz to the working directory (once per version).
#[allow(dead_code)]
fn ensure_bundled_asset(name: &str, bytes: &[u8]) -> Option<String> {
    let dir = format!("tcl-lsp-bundled-{BUNDLED_VERSION}");
    let path = format!("{dir}/{name}");
    if fs::metadata(&path).is_ok() {
        return Some(abs_path(&path));
    }
    let _ = fs::create_dir_all(&dir);
    match fs::write(&path, bytes) {
        Ok(()) => Some(abs_path(&path)),
        Err(_) => None,
    }
}

// Tcl/iRules reference data for slash-command argument completions.
// Loaded from generated catalogs at compile time — run `make generate`
// to update after registry changes.

static TCL_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("generated/tcl_commands.json"))
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
    let json: serde_json::Value =
        serde_json::from_str(include_str!("generated/irule_events.json"))
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
            cached_server_path: None,
            cached_mcp_path: None,
            cached_server_id: None,
        }
    }


    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        self.cached_server_id = Some(language_server_id.clone());
        let python = find_python(worktree)?;

        // Prefer workspace-local, then bundled, then auto-download.
        let server_path = match worktree.which("tcl-lsp-server.pyz") {
            Some(local) => local,
            None => {
                #[cfg(bundled_lsp)]
                let bundled =
                    ensure_bundled_asset("tcl-lsp-server.pyz", BUNDLED_LSP_BYTES);
                #[cfg(not(bundled_lsp))]
                let bundled: Option<String> = None;

                match bundled {
                    Some(path) => path,
                    None => ensure_asset_downloaded(
                        language_server_id,
                        LSP_ASSET_PREFIX,
                        &mut self.cached_server_path,
                    )?,
                }
            }
        };

        Ok(zed::Command {
            command: python,
            args: vec![server_path],
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
                if label.starts_with('$') {
                    spans.push(zed::CodeLabelSpan::literal(
                        "$",
                        Some("punctuation.special".into()),
                    ));
                    if label.len() > 1 {
                        spans.push(zed::CodeLabelSpan::literal(
                            &label[1..],
                            Some("variable".into()),
                        ));
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
                    spans.push(zed::CodeLabelSpan::literal(
                        *part,
                        Some("function".into()),
                    ));
                }
                // Append detail (signature) if present.
                let code = if let Some(ref detail) = completion.detail {
                    spans.push(zed::CodeLabelSpan::literal(
                        &format!(" {detail}"),
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
                if label.starts_with('-') {
                    spans.push(zed::CodeLabelSpan::literal(
                        "-",
                        Some("punctuation".into()),
                    ));
                    if label.len() > 1 {
                        spans.push(zed::CodeLabelSpan::literal(
                            &label[1..],
                            Some("keyword".into()),
                        ));
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
                spans.push(zed::CodeLabelSpan::literal(
                    "proc ",
                    Some("keyword".into()),
                ));
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
                     The `command_info` MCP tool accepts `{{\"name\": \"{cmd_name}\"}}` \
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
                     The `event_info` MCP tool accepts `{{\"name\": \"{event}\"}}` and \
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
        let python = find_python_global()?;

        // Prefer bundled MCP server, then fall back to auto-download.
        #[cfg(bundled_mcp)]
        let bundled = ensure_bundled_asset("tcl-lsp-mcp-server.pyz", BUNDLED_MCP_BYTES);
        #[cfg(not(bundled_mcp))]
        let bundled: Option<String> = None;

        let mcp_path = match bundled {
            Some(path) => path,
            None => {
                let server_id = self
                    .cached_server_id
                    .as_ref()
                    .ok_or("language server not yet initialised")?;
                ensure_asset_downloaded(server_id, MCP_ASSET_PREFIX, &mut self.cached_mcp_path)?
            }
        };

        Ok(zed::Command {
            command: python,
            args: vec![mcp_path],
            env: Default::default(),
        })
    }
}

zed::register_extension!(TclExtension);
