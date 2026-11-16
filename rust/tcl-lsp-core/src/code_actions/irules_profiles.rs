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

//! The `# Profiles:` source action for an iRules file.

use super::context_diagnostics::{selected_event_handlers, source_action_spec};
use super::{ActionKind, CodeAction, diagnostic_context};
use crate::definition::LspRange;
use tcl_compiler::analyser::AnalysisResult;

// iRules `# Profiles:` header source action.

/// Compute the sorted required virtual-server profiles from the file's events
/// (`EventProps.implied_profiles`) and commands (`event_requires.profiles`).
fn compute_required_profiles(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut profiles: BTreeSet<String> = BTreeSet::new();
    let Some(commands) = diagnostic_context::current_commands(source, analysis, registry) else {
        return Vec::new();
    };
    let events = tcl_registry::events::EventRegistry::build();
    for (_, event) in selected_event_handlers(source, analysis, registry, &commands) {
        if let Some(props) = events.get_props(event) {
            for profile in props.implied_profiles {
                profiles.insert((*profile).to_owned());
            }
        }
    }
    let Some(profile) = analysis.resolved_profile() else {
        return Vec::new();
    };
    if !profile.is_irules() {
        return Vec::new();
    }
    for command in commands {
        if let Some(spec) = source_action_spec(analysis, registry, &command.canonical)
            && let Some(requirement) = spec.event_requires.as_ref()
        {
            for profile in requirement.profiles {
                profiles.insert((*profile).to_owned());
            }
        }
    }
    // FASTHTTP is an alternative to HTTP; keep only HTTP when both appear.
    if profiles.contains("HTTP") {
        profiles.remove("FASTHTTP");
    }
    // Drop the stack-implied transport / shared-TLS profiles — classified
    // from each profile's `layer` in the registry, not a hardcoded list.
    let profile_registry = tcl_registry::profiles::ProfileRegistry::build();
    profiles.retain(|p| !profile_registry.is_infrastructure_profile(p));
    // Emit in protocol-stack order (transport → TLS → application → …) using
    // the registry's `layer` metadata, not the `BTreeSet`'s alphabetical
    // order — alphabetical would list e.g. `HTTP` before `SERVERSSL` (an
    // application profile ahead of its TLS layer), or `ASM` before `HTTP`.
    let mut ordered: Vec<String> = profiles.into_iter().collect();
    ordered.sort_by_key(|p| (profile_registry.layer_rank(p), p.clone()));
    ordered
}

/// Scan leading comment lines for a `# Profiles: HTTP, CLIENTSSL` directive,
/// returning `(uppercased profile set, line index)`.
fn scan_profile_directive(source: &str) -> Option<(std::collections::BTreeSet<String>, u32)> {
    for (i, line) in source.split('\n').enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !t.starts_with('#') {
            break; // first non-comment content — stop scanning
        }
        let body = t.trim_start_matches('#').trim();
        let lower = body.to_ascii_lowercase();
        if lower.starts_with("profile")
            && let Some(colon) = body.find(':')
        {
            let set: std::collections::BTreeSet<String> = body[colon + 1..]
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_uppercase)
                .collect();
            return Some((set, u32::try_from(i).unwrap_or(0)));
        }
    }
    None
}

/// Build the `# Profiles:` source action (insert or update) for an iRules
/// document.  Returns `None` when no profiles are required or the existing
/// directive already matches.  The caller gates on the iRules dialect.
#[must_use]
pub fn profiles_action(
    source: &str,
    analysis: &AnalysisResult,
    registry: &tcl_registry::CommandRegistry,
) -> Option<CodeAction> {
    let required = compute_required_profiles(source, analysis, registry);
    if required.is_empty() {
        return None;
    }
    let new_text = format!("# Profiles: {}\n", required.join(", "));
    if let Some((existing, line_no)) = scan_profile_directive(source) {
        let required_set: std::collections::BTreeSet<String> = required.iter().cloned().collect();
        if existing == required_set {
            return None;
        }
        return Some(CodeAction {
            title: format!(
                "Update profile requirements \u{2192} {}",
                required.join(", ")
            ),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: line_no,
                    start_character: 0,
                    end_line: line_no + 1,
                    end_character: 0,
                },
                new_text,
            }],
            kind: ActionKind::Source,
            command: None,
            data_group_definition: None,
            disabled: None,
        });
    }
    Some(CodeAction {
        title: format!(
            "Generate profile requirements header ({})",
            required.join(", ")
        ),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            new_text,
        }],
        kind: ActionKind::Source,
        command: None,
        data_group_definition: None,
        disabled: None,
    })
}
