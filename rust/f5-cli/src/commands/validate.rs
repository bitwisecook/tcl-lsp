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

//! `f5 validate` (alias `lint`) — run BIG-IP best-practice / structural lint
//! rules across one or more configs.
//!
//! The lint engine ([`tcl_bigip::lint`]) is a *sibling* of the query engine:
//! it walks the parsed model directly and reuses the model + iRule walker +
//! event registry, not the query DSL. This verb only owns input loading, the
//! three output formats (`text` / `json` / `sarif`), and the severity-based
//! exit codes.

use std::io::Write as _;
use std::path::Path;

use tcl_bigip::lint::{Finding, SEVERITIES, run_lint};
use tcl_cli_support::{OutputTarget, write_text_output};

/// Run the lint rules over `inputs` and emit findings in the requested format.
///
/// Returns the process exit code: `2` if any error-severity finding, `1` if
/// any warning, else `0`.
pub fn run_validate(
    inputs: &[std::path::PathBuf],
    category: Option<&str>,
    severity: Option<&str>,
    format: &str,
    output: Option<&Path>,
) -> anyhow::Result<u8> {
    let paths: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    // Load inputs via the UCS-aware loader with the default passphrase
    // provider; on I/O error, print `error: {e}` and return 2.
    let opts = tcl_bigip_io::PassphraseOptions {
        explicit: None,
        env_var: tcl_bigip_io::DEFAULT_PASSPHRASE_ENV.to_owned(),
        allow_prompt: true,
        prompt: Some(tcl_cli_support::prompt::read_ucs_passphrase),
    };
    let loaded = match tcl_bigip_io::load_paths(&paths, &opts) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    let sources: Vec<(String, &str)> = loaded
        .iter()
        .map(|l| (l.uri.clone(), l.source.as_str()))
        .collect();
    let configs: Vec<(String, &tcl_bigip::parser::BigipConfig)> =
        loaded.iter().map(|l| (l.uri.clone(), &l.config)).collect();

    let findings = run_lint(&configs, &sources, category, severity);

    let output_text = match format {
        "json" => to_json(&findings),
        "sarif" => to_sarif(&findings),
        _ => to_text(&findings),
    };

    write_text_output(&OutputTarget::from_arg(output), &output_text)?;
    let _ = std::io::stdout().flush();

    let has_error = findings.iter().any(|f| f.severity == "error");
    let has_warning = findings.iter().any(|f| f.severity == "warning");
    if has_error {
        Ok(2)
    } else if has_warning {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Render the text report, with a trailing newline.
///
/// `pub(crate)` so the `irule lint` verb can reuse the exact same formatter.
pub(crate) fn to_text(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "validate: no findings\n".to_owned();
    }
    // Per-severity buckets in first-seen order (only used for counts +
    // membership; emission walks SEVERITIES).
    let mut lines: Vec<String> = Vec::new();
    let summary = SEVERITIES
        .iter()
        .filter_map(|sev| {
            let count = findings.iter().filter(|f| f.severity == *sev).count();
            (count > 0).then(|| format!("{sev}={count}"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!("validate: {} finding(s) {summary}", findings.len()));

    for sev in SEVERITIES {
        let mut bucket: Vec<&Finding> = findings.iter().filter(|f| f.severity == sev).collect();
        bucket.sort_by(|a, b| (&a.rule_id, &a.full_path).cmp(&(&b.rule_id, &b.full_path)));
        for f in bucket {
            lines.push(format!(
                "  [{}] {}: {}: {}",
                f.severity, f.rule_id, f.full_path, f.message
            ));
        }
    }
    lines.join("\n") + "\n"
}

/// Render the JSON report as 2-space-indented JSON, in a fixed key order and
/// emitting findings in run order.
///
/// `pub(crate)` so the `irule lint` verb can reuse the exact same formatter.
pub(crate) fn to_json(findings: &[Finding]) -> String {
    use std::fmt::Write as _;

    use tcl_bigip::jsonfmt::json_string as q;

    let total = findings.len();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"summary\": {\n");
    let _ = writeln!(out, "    \"total\": {total},");
    out.push_str("    \"bySeverity\": {\n");
    for (i, sev) in SEVERITIES.iter().enumerate() {
        let count = findings.iter().filter(|f| f.severity == *sev).count();
        let comma = if i + 1 == SEVERITIES.len() { "" } else { "," };
        let _ = writeln!(out, "      {}: {count}{comma}", q(sev));
    }
    out.push_str("    }\n");
    out.push_str("  },\n");

    out.push_str("  \"findings\": [");
    for (i, f) in findings.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let _ = write!(
            out,
            "    {{\n      \"ruleId\": {},\n      \"severity\": {},\n      \"category\": {},\n      \"fullPath\": {},\n      \"message\": {}\n    }}",
            q(&f.rule_id),
            q(f.severity),
            q(f.category.as_str()),
            q(&f.full_path),
            q(&f.message),
        );
    }
    out.push_str(if findings.is_empty() {
        "]\n"
    } else {
        "\n  ]\n"
    });
    out.push_str("}\n");
    out
}

/// Map a finding severity to a SARIF level.
fn sarif_level(severity: &str) -> &'static str {
    match severity {
        "error" => "error",
        "warning" => "warning",
        _ => "note",
    }
}

/// Render the SARIF 2.1.0 report as 2-space-indented JSON.
fn to_sarif(findings: &[Finding]) -> String {
    use std::fmt::Write as _;

    use tcl_bigip::jsonfmt::json_string as q;

    // Unique rules in first-seen order.
    let mut rule_order: Vec<&str> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for f in findings {
        if seen.insert(f.rule_id.as_str()) {
            rule_order.push(f.rule_id.as_str());
        }
    }
    // The level recorded against each rule is the first finding's level.
    let rule_level = |rule_id: &str| -> &'static str {
        findings
            .iter()
            .find(|f| f.rule_id == rule_id)
            .map_or("note", |f| sarif_level(f.severity))
    };

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"version\": \"2.1.0\",\n");
    out.push_str("  \"$schema\": \"https://json.schemastore.org/sarif-2.1.0.json\",\n");
    out.push_str("  \"runs\": [\n");
    out.push_str("    {\n");
    out.push_str("      \"tool\": {\n");
    out.push_str("        \"driver\": {\n");
    out.push_str("          \"name\": \"f5-validate\",\n");
    out.push_str("          \"rules\": [");
    for (i, rule_id) in rule_order.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let short = rule_id.replace('-', " ");
        let _ = write!(
            out,
            "            {{\n              \"id\": {},\n              \"shortDescription\": {{\n                \"text\": {}\n              }},\n              \"defaultConfiguration\": {{\n                \"level\": {}\n              }}\n            }}",
            q(rule_id),
            q(&short),
            q(rule_level(rule_id)),
        );
    }
    out.push_str(if rule_order.is_empty() {
        "]\n"
    } else {
        "\n          ]\n"
    });
    out.push_str("        }\n");
    out.push_str("      },\n");

    out.push_str("      \"results\": [");
    for (i, f) in findings.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        let _ = write!(
            out,
            "        {{\n          \"ruleId\": {},\n          \"level\": {},\n          \"message\": {{\n            \"text\": {}\n          }},\n          \"locations\": [\n            {{\n              \"physicalLocation\": {{\n                \"artifactLocation\": {{\n                  \"uri\": {}\n                }}\n              }}\n            }}\n          ]\n        }}",
            q(&f.rule_id),
            q(sarif_level(f.severity)),
            q(&f.message),
            q(&f.full_path),
        );
    }
    out.push_str(if findings.is_empty() {
        "]\n"
    } else {
        "\n      ]\n"
    });
    out.push_str("    }\n");
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}
