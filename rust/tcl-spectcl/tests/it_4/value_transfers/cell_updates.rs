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

//! Cell updates over the shipped packs: every route that reads its target declares the read.

use super::*;

/// A route that reads its target's prior value declares the read where
/// every consumer asks for it: [`Traits::READS_BEFORE_WRITE`] on the scope
/// that carries the route. The dictionary's keyed updates had the route but
/// not the trait, so a spelling the lowering reaches by head —
/// `::tcl::dict::set`, an alias of `dict set` — recorded no read, and O109
/// deleted the store it read: `set d {a 1}; ::tcl::dict::set d k v` printed
/// `k v` where tclsh 8.5 to 9.1 print `a 1 k v`.
#[test]
fn a_route_that_reads_its_target_declares_the_read() {
    let reg = full_registry();
    let mut names: Vec<&str> = reg.command_names().collect();
    names.sort_unstable();
    let mut checked = BTreeSet::new();
    for name in names {
        let Some(spec) = reg.get(name) else {
            continue;
        };
        let scopes = std::iter::once(None).chain(spec.subcommands.iter().map(Some));
        for sub in scopes {
            let resolved = resolve_semantics(spec, sub, None);
            let Some(identity) = resolved.semantics().map(CommandSemantics::identity) else {
                continue;
            };
            if !(identity.starts_with("cell-update:") || identity.starts_with("keyed-update:")) {
                continue;
            }
            let traits = spec.traits | sub.map_or_else(Traits::empty, |sub| sub.traits);
            let label = sub.map_or_else(
                || spec.name.to_owned(),
                |sub| format!("{} {}", spec.name, sub.name),
            );
            assert!(
                traits.contains(Traits::READS_BEFORE_WRITE),
                "{label} reads its target through `{identity}` but does not declare the read"
            );
            checked.insert(label);
        }
    }
    for label in [
        "incr",
        "append",
        "lappend",
        "dict set",
        "dict unset",
        "dict incr",
        "dict append",
        "dict lappend",
        "::tcl::dict::set",
        "::tcl::dict::lappend",
    ] {
        assert!(
            checked.contains(label),
            "{label} was not checked: {checked:?}"
        );
    }
}
