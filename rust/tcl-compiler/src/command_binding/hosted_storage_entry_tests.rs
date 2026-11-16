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

use super::*;

#[test]
// Implementation contract: naming.variable.hosted-storage-context
// docs/design/analysis/name-resolution-proofs/variable-hosted-storage-context.md
fn hosted_storage_entry_is_independent_of_registry_and_source_name_policy() {
    use tcl_registry::f5::BigIpExecutionContext;
    let stock = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let irules = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let entry = SourceAnalysisEntry {
        hosted_execution_context: Some(BigIpExecutionContext::TmmIRule),
        ..Default::default()
    };
    assert_eq!(
        entry.options().hosted_execution_context,
        entry.hosted_execution_context
    );
    let selected = ModuleCommandBindings::initial_with_options(stock, entry.options(), None);
    assert_eq!(
        selected.source_variables.hosted_execution_context,
        entry.hosted_execution_context
    );
    assert!(
        selected
            .namespaces
            .contains(&SourceNamespaceKey::authored("::static"))
    );
    let catalogue =
        ModuleCommandBindings::initial_with_options(irules, SourceAnalysisOptions::default(), None);
    assert_eq!(catalogue.source_variables.hosted_execution_context, None);
    let different = SourceAnalysisEntry {
        hosted_execution_context: Some(BigIpExecutionContext::IAppImplementation),
        ..entry.clone()
    };
    let selected = ModuleCommandBindings::initial_with_options(irules, different.options(), None);
    assert_eq!(
        selected.source_variables.hosted_execution_context,
        different.hosted_execution_context
    );
    assert_ne!(entry, different);
    assert_ne!(
        entry.options().hosted_execution_context,
        different.options().hosted_execution_context
    );
    assert!(selected.source_variables.authored_tmm_static.is_none());
    assert!(selected.source_variables.execution.is_none());
}
