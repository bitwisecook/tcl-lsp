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

//! Host storage policy applied after Tcl resolves a namespace cell.
//!
//! Namespace spelling, aliases and frame lookup remain Tcl semantics. The TMM
//! overlay describes lifetime and sharing of the resolved cell, so absolute
//! names and aliases cannot acquire different CMP effects.

use super::BigIpExecutionContext;

/// Host ownership of an already resolved Tcl namespace cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableStorageDomain {
    /// Ordinary namespace storage in one interpreter.
    InterpreterNamespace,
    /// Persistent namespace storage private to the executing TMM worker.
    WorkerNamespace,
    /// Legacy global storage requiring CMP compatibility processing.
    /// This classification grants no shared cell or cross-worker value equality.
    CmpGlobal,
}

/// A symbolic execution identity; different workers never share Tcl cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkerExecution {
    /// Initialisation generation, changed when the worker interpreters are replaced.
    /// Reusing a worker number in a new generation does not revive its old cells.
    pub initialisation_epoch: u64,
    /// A known worker, or no proof of the worker selected for this flow.
    pub worker: Option<u32>,
    /// A known connection, or no active/identified connection.
    pub connection: Option<u64>,
}

/// Namespaces guaranteed by the supported TMM environment (BIG-IP 15+).
///
/// This is availability evidence, not a value or writer assumption. Other
/// BIG-IP execution engines do not inherit the TMM namespace surface.
#[must_use]
pub const fn runtime_namespaces(context: BigIpExecutionContext) -> &'static [&'static str] {
    match context {
        BigIpExecutionContext::TmmIRule => &["::", "::static"],
        _ => &["::"],
    }
}

/// Apply host policy to a canonical namespace, never to a written variable.
///
/// Static namespace cells persist across rules and belong to their executing
/// worker. The independently selected authored `RuleInitPublication` contract
/// publishes each reached initialisation mutation to enrolled worker cells;
/// ordinary globals and event mutations remain local. This storage classifier
/// supplies neither that execution context nor a publication provider. Ordered
/// initialisation and recipient callbacks need not leave equal final values.
/// There is no virtual server component in this namespace's identity.
#[must_use]
pub fn namespace_storage_domain(
    context: BigIpExecutionContext,
    namespace: &str,
) -> VariableStorageDomain {
    namespace_domain(context, namespace == "::", namespace == "::static")
}

/// Classify an independently resolved namespace's retained components.
/// Written names, rendering, source advice and registry profiles cannot
/// supply either the hosted context or the namespace geometry.
#[must_use]
pub fn namespace_storage_domain_in_path(
    context: BigIpExecutionContext,
    namespace: &tcl_core_types::ByteNamespacePath,
) -> VariableStorageDomain {
    let static_namespace =
        matches!(namespace.as_segments(), [component] if component.as_bytes() == b"static");
    namespace_domain(context, namespace.is_root(), static_namespace)
}

fn namespace_domain(
    context: BigIpExecutionContext,
    root: bool,
    static_namespace: bool,
) -> VariableStorageDomain {
    match context {
        BigIpExecutionContext::TmmIRule if static_namespace => {
            VariableStorageDomain::WorkerNamespace
        }
        BigIpExecutionContext::TmmIRule if root => VariableStorageDomain::CmpGlobal,
        _ => VariableStorageDomain::InterpreterNamespace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Implementation contract: naming.variable.hosted-storage-context
    // docs/design/analysis/name-resolution-proofs/variable-hosted-storage-context.md
    fn hosted_domain_uses_exact_resolved_components() {
        use tcl_core_types::ByteNamespacePath;
        let root = ByteNamespacePath::root();
        let private = ByteNamespacePath::from_segments([b"static".as_slice()]);
        assert_eq!(
            namespace_storage_domain_in_path(BigIpExecutionContext::TmmIRule, &root),
            VariableStorageDomain::CmpGlobal
        );
        assert_eq!(
            namespace_storage_domain_in_path(BigIpExecutionContext::TmmIRule, &private),
            VariableStorageDomain::WorkerNamespace
        );
        for path in [
            ByteNamespacePath::from_segments([b"app".as_slice(), b"static".as_slice()]),
            ByteNamespacePath::from_segments([b"static\0tail".as_slice()]),
            ByteNamespacePath::from_segments([b"::static".as_slice()]),
        ] {
            assert_eq!(
                namespace_storage_domain_in_path(BigIpExecutionContext::TmmIRule, &path),
                VariableStorageDomain::InterpreterNamespace
            );
        }
        for context in BigIpExecutionContext::ALL {
            if context != BigIpExecutionContext::TmmIRule {
                assert_eq!(
                    namespace_storage_domain_in_path(context, &root),
                    VariableStorageDomain::InterpreterNamespace
                );
                assert_eq!(
                    namespace_storage_domain_in_path(context, &private),
                    VariableStorageDomain::InterpreterNamespace
                );
            }
        }
    }

    #[test]
    fn storage_overlay_requires_the_tmm_context_and_root_static_namespace() {
        assert_eq!(
            namespace_storage_domain(BigIpExecutionContext::TmmIRule, "::static"),
            VariableStorageDomain::WorkerNamespace,
        );
        for context in [
            BigIpExecutionContext::HostShellTcl,
            BigIpExecutionContext::TmshCliScript,
            BigIpExecutionContext::IAppImplementation,
        ] {
            assert_eq!(
                namespace_storage_domain(context, "::static"),
                VariableStorageDomain::InterpreterNamespace,
            );
        }
        assert_eq!(
            namespace_storage_domain(BigIpExecutionContext::TmmIRule, "::app::static"),
            VariableStorageDomain::InterpreterNamespace,
        );
    }
    #[test]
    fn root_globals_retain_cmp_classification_without_a_worker_sharing_grant() {
        assert_eq!(
            namespace_storage_domain(BigIpExecutionContext::TmmIRule, "::"),
            VariableStorageDomain::CmpGlobal,
        );
        for context in BigIpExecutionContext::ALL {
            if context != BigIpExecutionContext::TmmIRule {
                assert_eq!(
                    namespace_storage_domain(context, "::"),
                    VariableStorageDomain::InterpreterNamespace,
                );
            }
        }
        assert_eq!(
            runtime_namespaces(BigIpExecutionContext::TmmIRule),
            &["::", "::static"],
        );
        assert_eq!(
            runtime_namespaces(BigIpExecutionContext::HostShellTcl),
            &["::"],
        );
    }
}
