#[cfg(any(bundled_lsp, bundled_mcp))]
use std::fs;
#[cfg(any(bundled_lsp, bundled_mcp))]
use std::path::PathBuf;
use std::sync::LazyLock;
use zed_extension_api::{self as zed, LanguageServerId, Result};

/// Extension version used to namespace bundled binary directories.
#[cfg(any(bundled_lsp, bundled_mcp))]
const BUNDLED_VERSION: &str = match option_env!("TCL_LSP_BUNDLED_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// On Windows the bundled/dev binaries carry a `.exe` suffix.
#[cfg(target_os = "windows")]
const EXE_SUFFIX: &str = ".exe";
#[cfg(not(target_os = "windows"))]
const EXE_SUFFIX: &str = "";

#[cfg(bundled_lsp)]
const BUNDLED_LSP_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tcl-lsp-server"));
#[cfg(bundled_mcp)]
const BUNDLED_MCP_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tcl-mcp"));

struct TclExtension {
    cached_server_id: Option<LanguageServerId>,
}

// Helpers

/// Convert a relative path in the extension sandbox to an absolute path.
/// Zed runs language server commands with the project folder as CWD, so
/// any paths we return from the extension must be absolute.
#[cfg(any(bundled_lsp, bundled_mcp))]
fn abs_path(relative: &str) -> String {
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.join(relative).to_string_lossy().into_owned()
}

/// Materialise a compile-time-embedded native binary to the extension's
/// writable dir (once per version) with executable permissions, and return
/// its absolute path.
#[cfg(any(bundled_lsp, bundled_mcp))]
fn ensure_bundled_binary(name: &str, bytes: &[u8]) -> Result<String> {
    let file_name = format!("{name}{EXE_SUFFIX}");
    let dir = format!("tcl-lsp-bundled-{BUNDLED_VERSION}");
    let path = format!("{dir}/{file_name}");

    if fs::metadata(&path).is_err() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create bundled dir {dir}: {e}"))?;
        fs::write(&path, bytes).map_err(|e| format!("failed to write bundled {name}: {e}"))?;
    }

    let absolute = abs_path(&path);
    zed::make_file_executable(&absolute)
        .map_err(|e| format!("failed to make bundled {name} executable: {e}"))?;
    Ok(absolute)
}

/// Resolve a dev native binary from the worktree PATH (non-bundled builds).
#[cfg(not(bundled_lsp))]
fn find_dev_binary(worktree: &zed::Worktree, name: &str) -> Result<String> {
    let file_name = format!("{name}{EXE_SUFFIX}");
    worktree.which(&file_name).ok_or_else(|| {
        format!(
            "`{file_name}` was not found on PATH. This is a dev build of the Tcl \
             extension without a bundled native server. Build the native server \
             (`make build` in the tcl-lsp repo) and install it on your PATH, or \
             install a released build of the extension. \
             See https://github.com/bitwisecook/tcl-lsp/blob/main/INSTALL.md"
        )
    })
}

/// Resolve the language server binary path: bundled native binary if present,
/// otherwise a dev binary on PATH.
#[cfg(bundled_lsp)]
fn resolve_lsp_path(_worktree: &zed::Worktree) -> Result<String> {
    ensure_bundled_binary("tcl-lsp-server", BUNDLED_LSP_BYTES)
}

/// Resolve the language server binary path: bundled native binary if present,
/// otherwise a dev binary on PATH.
#[cfg(not(bundled_lsp))]
fn resolve_lsp_path(worktree: &zed::Worktree) -> Result<String> {
    find_dev_binary(worktree, "tcl-lsp-server")
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
        let server_path = resolve_lsp_path(worktree)?;

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
        // Prefer the bundled native MCP binary; in dev builds fall back to a
        // `tcl-mcp` binary on PATH (resolved by the OS, since no worktree is
        // available here to call `which`). The MCP server speaks over stdio
        // with no args.
        #[cfg(bundled_mcp)]
        let mcp_path = ensure_bundled_binary("tcl-mcp", BUNDLED_MCP_BYTES)?;
        #[cfg(not(bundled_mcp))]
        let mcp_path = format!("tcl-mcp{EXE_SUFFIX}");

        Ok(zed::Command {
            command: mcp_path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(TclExtension);
