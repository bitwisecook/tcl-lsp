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

//! Whether a command some other environment offers could be meant for a
//! document: the relation between a document's core family and the providers
//! of a name, which separates a command that is *disabled here* from one that
//! is *unknown here*.

use tcl_registry::CommandRegistry;
use tcl_registry::model::{package_hosting_families, resolve_environment};

/// Whether `name` is offered to a world related to `environment`'s.
fn related(environment: &str, name: &str) -> bool {
    let registry = CommandRegistry::build_default();
    let offered = registry
        .providers_in_any_dialect(name)
        .unwrap_or_else(|| panic!("{name} is offered somewhere"));
    resolve_environment(environment)
        .default_context_registry()
        .context()
        .is_related_to_a_provider_of(offered)
}

/// A core command of one release is related to every release of that family:
/// a `tcl8.4` document writing an 8.6 command has that command disabled.
#[test]
fn a_tcl_command_is_related_to_every_release_of_tcl() {
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl"] {
        assert!(related(environment, "lmap"), "{environment}");
    }
}

/// Jim reimplements Tcl's command surface, so a Tcl command Jim omits is on
/// Jim's line: `coroutine` is disabled in a `jim` document.
#[test]
fn a_tcl_core_command_is_related_to_jim() {
    assert!(related("jim", "coroutine"));
}

/// Expect's `system` is a package of Tcl-family environments. It is related to
/// the documents whose family ships or forks that ecosystem, and to none
/// whose family only reimplements Tcl.
#[test]
fn a_package_command_is_related_only_where_the_package_is_shared() {
    for environment in ["tcl8.6", "expect", "tk"] {
        assert!(related(environment, "system"), "{environment}");
    }
    assert!(!related("jim", "system"));
}

/// An iRules command sits on the Tcl line above the F5 fork, and on no line
/// with Jim.
#[test]
fn an_irules_command_is_related_along_the_fork_line_and_not_to_jim() {
    for environment in ["tcl8.6", "f5-tcl", "f5-irules"] {
        assert!(related(environment, "when"), "{environment}");
    }
    assert!(!related("jim", "when"));
}

/// A package ships from the environments that place or vend it, keyed by the
/// family of their core.
#[test]
fn a_package_is_hosted_by_the_families_of_the_environments_that_ship_it() {
    use tcl_dialect::model::Family;
    assert_eq!(package_hosting_families("expect"), [Family::Tcl]);
    assert!(package_hosting_families("Tk").contains(&Family::Tcl));
    assert!(package_hosting_families("no-such-package").is_empty());
}
