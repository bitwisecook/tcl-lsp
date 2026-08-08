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

//! **Registry self-consistency for IRULE1001.**
//!
//! `IRULE1001` (command-event validity) is entirely registry-driven: it reads
//! `CommandSpec::event_requires` — `client_side` / `server_side` /
//! `transport` / `profiles` / `also_in` — and the event legality matrix, and
//! never hard-codes a command name. That is the right architecture, but it
//! makes the check exactly as correct as its data: one wrong `profiles`
//! entry becomes a warning on perfectly ordinary iRules, everywhere, for
//! ever.
//!
//! The corpus sweep for issue #1316 found precisely that (`active_members`
//! claiming to need a `DNS` profile, `snat` claiming `FASTHTTP`/`MR`), and
//! the tell was general rather than per-command: **each offending spec's own
//! documented `hover.examples` triggers that spec's own diagnostic.** A
//! command's canonical usage example, written by the registry author from
//! F5's own documentation, is the strongest available statement of where the
//! command is legitimately used. If IRULE1001 fires on it, either the
//! example is wrong or the requirement data is — and either way the registry
//! is internally inconsistent.
//!
//! This test encodes that invariant over the whole iRules command set, so
//! the class is closed rather than the two instances the sweep happened to
//! surface. It is a data-quality gate, not a behavioural test of the check.
//!
//! **Limits.** This proves only self-consistency: that a spec does not
//! contradict its own example. It cannot prove a `profiles` list is *right*
//! — a command whose example omits the relevant event, or that carries no
//! example at all, is not covered, and a wrong-but-unexercised requirement
//! stays invisible. It also deliberately scores only IRULE1001 firings *on
//! the command that owns the example*: examples routinely call other
//! commands (`pool`, `log`) incidentally, and holding an author responsible
//! for those would make the gate unmaintainable.

use tcl_compiler::analyser::Analyser;
use tcl_registry::registry_for_dialect;

/// Every IRULE1001 message raised against `cmd_name` in `source`.
fn irule1001_against(source: &str, cmd_name: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(source, "f5-irules")
        .diagnostics
        .into_iter()
        .filter(|d| format!("{:?}", d.code) == "Irule1001")
        .map(|d| d.message)
        // Messages are `'<cmd>' …` — attribute only the owning command.
        .filter(|m| m.starts_with(&format!("'{cmd_name}'")))
        .collect()
}

#[test]
fn irules_spec_examples_do_not_trigger_their_own_irule1001() {
    let registry = registry_for_dialect("f5-irules");

    let mut offenders: Vec<String> = Vec::new();
    let mut checked = 0usize;

    let mut names: Vec<&str> = registry.command_names().collect();
    names.sort_unstable();
    names.dedup();

    for name in names {
        let Some(spec) = registry.get(name) else {
            continue;
        };
        // iRules-only commands: the shared Tcl core has no event context.
        if spec.dialects.is_none_or(|d| !d.is_irules()) {
            continue;
        }
        let Some(hover) = spec.hover.as_ref() else {
            continue;
        };
        let example = hover.examples;
        // Only examples that actually establish an event context can be
        // scored — IRULE1001 fires only inside a `when` block.
        if !example.contains("when ") {
            continue;
        }
        checked += 1;

        for message in irule1001_against(example, name) {
            offenders.push(format!("  {name}: {message}"));
        }
    }

    assert!(
        checked > 100,
        "expected the iRules registry to supply many `when`-bearing examples; \
         only {checked} were found — has the spec shape changed?"
    );

    assert!(
        offenders.is_empty(),
        "{} iRules command spec(s) fire IRULE1001 on their own documented \
         example — the spec's `event_requires` contradicts the usage the same \
         spec advertises (issue #1316). Fix the requirement data (or the \
         example), not the check:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
