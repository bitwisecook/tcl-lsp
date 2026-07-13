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

//! Every place an iRule can hide a script must be walked for object references.
//!
//! The BIG-IP reference graph is not a highlighting nicety: `f5 grep`, `f5 irule
//! trace`, `f5 query rename` and the `bigip-cleanup` skill all read it, and
//! `bigip-cleanup` *deletes* the objects it reports as unreferenced. An object
//! that the walker cannot see looks unused, so a miss here is a deleted live
//! pool — not a wrong colour.
//!
//! That is not hypothetical. A `switch` case list is not an [`ArgRole::Body`],
//! so the walker never descended into one, and
//!
//! ```tcl
//! switch -glob [HTTP::uri] {
//!     "/api/*" { pool /Common/api_pool }
//! }
//! ```
//!
//! — an entirely ordinary iRule — hid its pool from every one of those tools.
//! It was found by accident while diffing highlighting, not by a test. These
//! tests exist so the next one is found by CI instead.

use tcl_irules::extract_irules_object_references;
use tcl_registry::prelude::ArgRole;
use tcl_registry::registry::CommandRegistry;

/// A pool name no real config would contain, so finding it means the walker
/// genuinely descended rather than matching something incidental.
const PROBE: &str = "/Common/zzz_refgap_probe";

fn irules_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    registry
}

/// Does the walker find the probe pool in `source`?
fn finds_probe(source: &str, registry: &CommandRegistry) -> bool {
    extract_irules_object_references(source, None, registry)
        .iter()
        .any(|r| r.name == PROBE)
}

/// The roles `walker.rs` actually descends into.
///
/// Kept beside the walker's behaviour rather than inferred from it, so that a
/// role which *starts* carrying a script has to be added here consciously.
const WALKED_ROLES: &[ArgRole] = &[ArgRole::Body, ArgRole::Expr];

/// The walker must descend into every role the registry says holds code.
///
/// [`ArgRole::carries_script`] is an exhaustive match, so a new variant cannot
/// be added without classifying it. This test closes the other half: once
/// classified as script-bearing, it must actually be *walked*, or an object
/// referenced only from that position silently leaves the graph.
#[test]
fn the_walker_descends_into_every_script_bearing_role() {
    for &role in ArgRole::ALL {
        assert_eq!(
            role.carries_script(),
            WALKED_ROLES.contains(&role),
            "ArgRole::{role:?} carries a script but the iRules object walker does \
             not descend into it (or vice versa). An object referenced only from \
             this position would be invisible to `f5 grep`, `f5 irule trace` and \
             `f5 query rename` — and `bigip-cleanup` would report it unreferenced \
             and generate a delete for it. Teach walker.rs to recurse it, then add \
             it to WALKED_ROLES."
        );
    }
}

/// Every argument the registry marks [`ArgRole::Body`] on every command reachable
/// from an iRule is descended into.
///
/// Generative rather than hand-written: it asks the registry which arguments are
/// bodies — using the very argument list the walker will see — and then requires
/// the probe placed there to come back out. A command added to the iRules dialect
/// with a body argument is covered the moment it lands, with no test to remember
/// to write.
#[test]
fn every_body_argument_of_every_irules_command_is_walked() {
    let registry = irules_registry();
    let mut checked = 0usize;
    let mut gaps: Vec<String> = Vec::new();

    let commands: Vec<String> = registry.command_names().map(str::to_owned).collect();
    for command in &commands {
        // Arity is unknown, so try a few shapes and let the registry tell us
        // which argument — if any — is a body for that shape.
        for argc in 1..=5usize {
            for probe_at in 0..argc {
                let mut words: Vec<String> = vec!["x".to_owned(); argc];
                words[probe_at] = format!("{{ pool {PROBE} }}");
                let argv: Vec<&str> = words.iter().map(String::as_str).collect();

                if !registry
                    .arg_indices_for_role(command, &argv, ArgRole::Body)
                    .contains(&probe_at)
                {
                    continue;
                }

                let source = format!(
                    "when HTTP_REQUEST {{\n    {command} {}\n}}\n",
                    words.join(" ")
                );
                checked += 1;
                if !finds_probe(&source, &registry) {
                    gaps.push(format!("{command}: arg {probe_at} of {argc}"));
                }
            }
        }
    }

    assert!(
        checked > 10,
        "the sweep synthesised only {checked} body arguments — it is not \
         exercising the registry, so it proves nothing"
    );
    assert!(
        gaps.is_empty(),
        "the registry marks these arguments as script bodies, but the iRules \
         object walker does not descend into them — a pool referenced only there \
         is invisible to `f5 grep` / `f5 irule trace` / `f5 query rename`, and \
         `bigip-cleanup` would delete it as unreferenced:\n  {}",
        gaps.join("\n  ")
    );
}

/// A clause list carries scripts that no [`ArgRole`] describes.
///
/// This is the mechanism the `switch` bug hid behind: the bodies live inside a
/// single word that the registry marks with `CommandSpec::case_list`, not with a
/// role. Any command carrying that spec must have its clause bodies walked.
#[test]
fn every_clause_list_body_is_walked() {
    let registry = irules_registry();
    let commands: Vec<String> = registry.command_names().map(str::to_owned).collect();
    let mut checked = 0usize;
    let mut gaps: Vec<String> = Vec::new();

    for name in &commands {
        if registry.get(name).and_then(|s| s.case_list).is_none() {
            continue;
        }
        let source = format!(
            "when HTTP_REQUEST {{\n    {name} [HTTP::uri] {{\n        \"/a/*\" {{ pool {PROBE} }}\n        default {{ pool /Common/other }}\n    }}\n}}\n"
        );
        checked += 1;
        if !finds_probe(&source, &registry) {
            gaps.push(name.clone());
        }
    }

    assert!(
        checked > 0,
        "no command in the iRules dialect carries a `case_list` spec — the sweep \
         is vacuous, which means it would not have caught the `switch` bug"
    );
    assert!(
        gaps.is_empty(),
        "these commands carry a clause list whose bodies are not walked: {}",
        gaps.join(", ")
    );
}

/// The regression itself, spelled out — the exact iRule that `bigip-cleanup`
/// would have deleted a live pool from.
#[test]
fn a_pool_used_only_in_a_switch_arm_is_referenced() {
    let registry = irules_registry();
    let source = r#"
when HTTP_REQUEST {
    switch -glob [HTTP::uri] {
        "/api/*" { pool /Common/zzz_refgap_probe }
        default  { pool /Common/web_pool }
    }
}
"#;
    let names: Vec<String> = extract_irules_object_references(source, None, &registry)
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert!(
        names.iter().any(|n| n == PROBE),
        "a pool referenced only from a `switch` arm is invisible to the reference \
         graph; got {names:?}"
    );
    assert!(names.iter().any(|n| n == "/Common/web_pool"));
}
