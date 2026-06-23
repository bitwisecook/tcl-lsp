//! Lookup verbs: `command-info` (registry metadata).
//!
//! Drives the command registry in `tcl-registry` to report command metadata.

use std::path::Path;

use serde::Serialize;
use tcl_cli_support::{OutputTarget, registry_for_dialect, write_text_output};
use tcl_registry::dialects::DialectSet;

/// JSON payload when the command resolves (field order matches the Python dict).
#[derive(Serialize)]
struct FoundPayload {
    found: bool,
    command: String,
    dialect: String,
    summary: String,
    synopsis: Vec<String>,
    switches: Vec<String>,
    #[serde(rename = "validEvents")]
    valid_events: Vec<String>,
}

/// JSON payload when the command is unknown.
#[derive(Serialize)]
struct NotFoundPayload {
    found: bool,
    command: String,
    dialect: String,
}

/// `tcl command-info` — look up registry metadata for one command.
pub fn run_command_info(
    command: &str,
    dialect: &str,
    json: bool,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let query = command.trim();
    if query.is_empty() {
        anyhow::bail!("command name is required");
    }
    let registry = registry_for_dialect(dialect);
    let target = OutputTarget::from_arg(output);

    // Exact match, then a case-insensitive fallback (mirrors lookup_command_info).
    let resolved_name: Option<String> = if registry.get(query).is_some() {
        Some(query.to_owned())
    } else {
        let lowered = query.to_lowercase();
        registry
            .command_names()
            .find(|c| c.to_lowercase() == lowered)
            .map(str::to_owned)
    };

    let Some(resolved_name) = resolved_name else {
        if json {
            let payload = NotFoundPayload {
                found: false,
                command: query.to_owned(),
                dialect: dialect.to_owned(),
            };
            write_text_output(
                &target,
                &tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?),
            )?;
        } else {
            write_text_output(
                &target,
                &format!("command not found: {query} (dialect={dialect})"),
            )?;
        }
        return Ok(1);
    };

    let spec = registry.get(&resolved_name).expect("resolved command spec");
    let summary = spec
        .hover
        .as_ref()
        .map_or(String::new(), |h| h.summary.to_owned());
    let synopsis: Vec<String> = spec
        .hover
        .as_ref()
        .map(|h| h.synopsis.iter().map(|s| (*s).to_owned()).collect())
        .unwrap_or_default();
    let mut switches: Vec<String> = spec
        .switch_names(DialectSet::parse(dialect))
        .into_iter()
        .map(str::to_owned)
        .collect();
    switches.sort();
    // NOTE: iRules `validEvents` resolution (event_requires → events_matching)
    // is not implemented; non-iRules dialects resolve to an empty list anyway.
    let valid_events: Vec<String> = Vec::new();

    if json {
        let payload = FoundPayload {
            found: true,
            command: resolved_name.clone(),
            dialect: dialect.to_owned(),
            summary,
            synopsis,
            switches,
            valid_events,
        };
        write_text_output(
            &target,
            &tcl_cli_support::ensure_ascii(&serde_json::to_string_pretty(&payload)?),
        )?;
        return Ok(0);
    }

    let mut lines = vec![
        format!("command: {resolved_name}"),
        format!("dialect: {dialect}"),
    ];
    if !summary.is_empty() {
        lines.push(format!("summary: {summary}"));
    }
    for item in &synopsis {
        lines.push(format!("synopsis: {item}"));
    }
    if !switches.is_empty() {
        lines.push(format!("switches: {}", switches.join(", ")));
    }
    if !valid_events.is_empty() {
        let shown: Vec<&str> = valid_events.iter().take(20).map(String::as_str).collect();
        lines.push(format!(
            "valid events ({}): {}",
            valid_events.len(),
            shown.join(", ")
        ));
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
