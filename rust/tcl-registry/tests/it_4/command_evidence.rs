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

//! Every row the registry closes is evidenced from its implementation's
//! source (issue #1712).
//!
//! `scripts/dev/tcl_command_evidence.py` reads each Tcl release's
//! `generic/*.c` and records, per command and subcommand, whether its
//! procedure's transitive closure is `closed`: no world-touching interface,
//! no script, no interpreter state beyond its result and error, and no
//! completion code but `TCL_OK` and `TCL_ERROR`. A registry row that claims
//! a closed domain at a release — side effects complete and empty, world
//! effects complete and touching nothing, completion complete with `Ok` and
//! `Error` only, or a complete referentially transparent result — must be a
//! `closed` command or subcommand of that release. A tcllib procedure is
//! evidenced by `scripts/dev/tcl_script_evidence.py` over tcllib 2.0, whose
//! closure holds on every release it supports, Tcl 8.5 to 9.1.

use tcl_dialect::model::{Family, SurfaceQuery};
use tcl_registry::CommandRegistry;
use tcl_registry::purity::PurityDomains;

const RELEASES: &[(&str, &str)] = &[
    ("8.4", include_str!("../data/command_evidence/tcl8.4.txt")),
    ("8.5", include_str!("../data/command_evidence/tcl8.5.txt")),
    ("8.6", include_str!("../data/command_evidence/tcl8.6.txt")),
    ("9.0", include_str!("../data/command_evidence/tcl9.0.txt")),
    ("9.1", include_str!("../data/command_evidence/tcl9.1.txt")),
];
const TCLLIB: &str = include_str!("../data/command_evidence/tcllib.txt");
const TCLLIB_RELEASES: &[&str] = &["8.5", "8.6", "9.0", "9.1"];

fn closed(evidence: &str) -> std::collections::BTreeSet<&str> {
    evidence
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?;
            (fields.next()? == "closed").then_some(name)
        })
        .collect()
}

/// The domains `domains` claims closed at `query`, by name.
fn closed_claims(domains: &PurityDomains, query: &SurfaceQuery<'_>) -> Vec<&'static str> {
    let query = Some(query);
    [
        ("side_effects", domains.side_effects_closed_at(query)),
        ("world_effects", domains.world_effects_closed_at(query)),
        ("completion", domains.completion_closed_at(query)),
        (
            "result_stability",
            domains.result_stability_closed_at(query),
        ),
    ]
    .into_iter()
    .filter_map(|(domain, claimed)| claimed.then_some(domain))
    .collect()
}

#[test]
fn complete_rows_match_the_c_evidence() {
    let mut unevidenced = Vec::new();
    let mut claims = 0;
    let tcllib = closed(TCLLIB);
    for &(release, evidence) in RELEASES {
        let mut closed = closed(evidence);
        if TCLLIB_RELEASES.contains(&release) {
            closed.extend(&tcllib);
        }
        let profile = tcl_dialect::TclVersion::profile_name_for(release)
            .and_then(tcl_dialect::DialectProfile::find)
            .expect("a Tcl release profile");
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let query = SurfaceQuery::core(Family::Tcl, release);
        for name in registry
            .command_names()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        {
            let Some(spec) = registry.get(&name) else {
                continue;
            };
            // `::tcl::dict::get` is the `dict get` the ensemble map creates.
            let bare = name.trim_start_matches("::");
            let bare = bare
                .strip_prefix("tcl::dict::")
                .map_or_else(|| bare.to_owned(), |sub| format!("dict {sub}"));
            // A bare operator row is the `tcl::mathop` command a `namespace
            // path` reaches it through.
            let operator = format!("tcl::mathop::{bare}");
            let bare = if !bare.contains("::") && closed.contains(operator.as_str()) {
                operator
            } else {
                bare
            };
            let bare = bare.as_str();
            let mut check = |label: String, domains: PurityDomains| {
                let claimed = closed_claims(&domains, &query);
                if claimed.is_empty() {
                    return;
                }
                claims += 1;
                if !closed.contains(label.as_str()) {
                    unevidenced.push(format!("Tcl {release} `{label}`: {}", claimed.join(", ")));
                }
            };
            check(bare.to_owned(), PurityDomains::of_command(spec));
            for sub in spec.subcommands {
                check(
                    format!("{bare} {}", sub.name),
                    PurityDomains::of_subcommand(spec, sub),
                );
            }
        }
    }
    assert!(
        unevidenced.is_empty(),
        "{} closed claim(s) of {claims} have no `closed` line in \
         rust/tcl-registry/tests/data/command_evidence (regenerate it with \
         scripts/dev/tcl_command_evidence.py, or withdraw the claim):\n{}",
        unevidenced.len(),
        unevidenced.join("\n")
    );
}

/// The issue #1712 audit: no `tmsh::*` iApps command states its effects, so
/// every one is `Unknown` — never verified empty, never pure. What
/// `tmsh::create`, `modify` and `delete` change needs a device to evidence.
#[test]
fn tmsh_commands_state_their_effects_as_unknown() {
    let tmsh: Vec<_> = tcl_registry::installed()
        .specs_of("iapps")
        .iter()
        .filter(|spec| spec.name.starts_with("tmsh::"))
        .collect();
    assert!(tmsh.len() >= 30, "{}", tmsh.len());
    for spec in &tmsh {
        assert!(spec.side_effects.is_unknown(), "{}", spec.name);
        assert!(
            tcl_registry::purity::PurityDomains::of_command(spec)
                .pure_releases(tcl_dialect::model::Family::Tcl)
                .is_empty(),
            "{}",
            spec.name
        );
    }
}
