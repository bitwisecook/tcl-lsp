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

//! Namespace package ownership from independently selected declaration rows.
//!
//! Constructed metadata only: no written-name parser, native namespace token,
//! package loader, source currency or callable-command authority is issued.

use std::collections::BTreeMap;

/// Immutable descriptive ownership under one actual context/store selection.
/// An unowned or conflicting row is retained as unknown, so a later narrower
/// hint cannot silently adopt another package's namespace.
#[derive(Debug, Clone, Default)]
pub struct NamespacePackageOwnership {
    namespaces: BTreeMap<String, Option<&'static str>>,
}

impl NamespacePackageOwnership {
    pub(crate) fn from_selected_specs<'a>(
        specs: impl IntoIterator<Item = &'a crate::CommandSpec>,
    ) -> Self {
        let mut result = Self::default();
        for spec in specs {
            let (holder, _) = tcl_syntax::naming::key_holder_and_tail(spec.name);
            let holder = tcl_syntax::naming::unroot_rooted_key(holder).unwrap_or(holder);
            if holder.is_empty() {
                continue;
            }
            let package = match (spec.required_package, spec.tcllib_package) {
                (Some(first), Some(second)) if first != second => None,
                (Some(package), _) | (_, Some(package)) if !package.is_empty() => Some(package),
                _ => None,
            };
            result
                .namespaces
                .entry(holder.to_owned())
                .and_modify(|previous| {
                    if *previous != package {
                        *previous = None;
                    }
                })
                .or_insert(package);
        }
        result
    }

    /// Unanimous package for an already constructed namespace metadata key.
    /// Exactly one optional root marker is removed; case/literal colons remain.
    #[must_use]
    pub fn package_for_namespace(&self, namespace: &str) -> Option<&'static str> {
        let key = tcl_syntax::naming::unroot_rooted_key(namespace).unwrap_or(namespace);
        self.namespaces.get(key).copied().flatten()
    }

    /// Most specific declared metadata prefix in a flat command key. This does
    /// not turn the key into a C namespace path; unknown/conflicting prefixes
    /// block a broader package hint. No name-input conversion is performed.
    #[must_use]
    pub fn package_for_flat_key(&self, key: &[u8]) -> Option<&'static str> {
        self.namespaces
            .iter()
            .filter(|(namespace, _)| {
                key.strip_prefix(namespace.as_bytes())
                    .is_some_and(|tail| tail.starts_with(b"::"))
            })
            .max_by_key(|(namespace, _)| namespace.len())
            .and_then(|(_, package)| *package)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandRegistry, CommandSpec};
    use std::sync::Arc;

    fn row(name: &'static str, package: Option<&'static str>) -> CommandSpec {
        CommandSpec {
            name,
            required_package: package,
            ..CommandSpec::DEFAULT
        }
    }

    #[test]
    fn namespace_package_consensus_keeps_unknown_nested_case_and_literal_colons() {
        let specs = vec![
            row("tools::fetch", Some("tools-runtime")),
            row("::tools::send", Some("tools-runtime")),
            row("tools::private::hidden", None),
            row("conflict::first", Some("provider-a")),
            row("conflict::second", Some("provider-b")),
            row("unknown::unowned", None),
            row("unknown::owned", Some("provider-a")),
            row("empty::member", Some("")),
            row("::茶:::::member", Some("tea-library")),
            CommandSpec {
                name: "mixed::member",
                required_package: Some("first"),
                tcllib_package: Some("second"),
                ..CommandSpec::DEFAULT
            },
            CommandSpec {
                name: "library::member",
                tcllib_package: Some("library-provider"),
                ..CommandSpec::DEFAULT
            },
        ];
        let ownership = NamespacePackageOwnership::from_selected_specs(&specs);
        assert_eq!(
            ownership.package_for_namespace("tools"),
            Some("tools-runtime")
        );
        assert_eq!(
            ownership.package_for_namespace("::tools"),
            Some("tools-runtime")
        );
        assert_eq!(
            ownership.package_for_flat_key(b"tools::missing"),
            Some("tools-runtime")
        );
        for key in [
            b"tools::private::missing".as_slice(),
            b"toolsmith::missing",
            b"Tools::missing",
            b"::tools::missing",
        ] {
            assert_eq!(ownership.package_for_flat_key(key), None);
        }
        for namespace in [
            "conflict",
            "unknown",
            "empty",
            "mixed",
            "Tools",
            ":::tools",
            "tools::private",
        ] {
            assert_eq!(
                ownership.package_for_namespace(namespace),
                None,
                "{namespace}"
            );
        }
        assert_eq!(
            ownership.package_for_namespace("library"),
            Some("library-provider")
        );
        assert_eq!(
            ownership.package_for_namespace("::茶:::"),
            Some("tea-library")
        );
        assert_eq!(ownership.package_for_namespace("::茶"), None);
        assert_eq!(
            ownership.package_for_flat_key("茶:::::missing".as_bytes()),
            Some("tea-library")
        );
        let reversed = NamespacePackageOwnership::from_selected_specs(specs.iter().rev());
        assert_eq!(ownership.namespaces, reversed.namespaces);
    }

    #[test]
    fn namespace_package_context_selects_real_rows_and_declines_foreign_release() {
        let mut store = CommandRegistry::build_default();
        for (name, package, surface) in [
            (
                "fixture-tools::fetch",
                "tools-runtime",
                tcl_dialect::model::SpecSurface::TCL86,
            ),
            (
                "fixture-choice::member",
                "release86-provider",
                tcl_dialect::model::SpecSurface::TCL86,
            ),
            (
                "fixture-choice::member",
                "release84-provider",
                tcl_dialect::model::SpecSurface::TCL84,
            ),
        ] {
            store.insert(CommandSpec {
                name,
                surface: Some(surface),
                required_package: Some(package),
                ..CommandSpec::DEFAULT
            });
        }
        let store = Arc::new(store);
        let context = crate::model::ingress::resolve_environment("tcl8.6")
            .default_context_registry()
            .with_command_store(Arc::clone(&store));
        let foreign = crate::model::ingress::resolve_environment("tcl8.4")
            .default_context_registry()
            .with_command_store(Arc::clone(&store));
        assert!(Arc::ptr_eq(context.commands(), foreign.commands()));
        assert_eq!(
            context
                .namespace_package_ownership()
                .package_for_namespace("fixture-tools"),
            Some("tools-runtime")
        );
        assert_eq!(
            foreign
                .namespace_package_ownership()
                .package_for_namespace("fixture-tools"),
            None
        );
        assert_eq!(
            context
                .namespace_package_ownership()
                .package_for_namespace("fixture-choice"),
            Some("release86-provider")
        );
        assert_eq!(
            foreign
                .namespace_package_ownership()
                .package_for_namespace("fixture-choice"),
            Some("release84-provider")
        );
        assert_eq!(
            context
                .namespace_package_ownership()
                .package_for_namespace("http"),
            Some("http")
        );
        assert_eq!(store.specs("fixture-tools::fetch").len(), 1);
    }
}
