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

//! The `workspace/executeCommand` request handler.

use crate::{Backend, ExecuteCommandParams, jsonrpc};

impl Backend {
    pub(crate) async fn serve_execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "tcl-lsp.minifyDocument" => self.minify_document_command(&params.arguments).await,
            "tcl-lsp.captureDocumentSourceContext" => {
                self.capture_document_source_context_command(&params.arguments)
                    .await
            }
            "tcl-lsp.validateDocumentSourceContext" => {
                self.validate_document_source_context_command(&params.arguments)
                    .await
            }
            "tcl-lsp.validateDocumentTransform" => {
                self.validate_document_transform_command(&params.arguments)
                    .await
            }
            "tcl-lsp.optimiseDocument" => self.optimise_document_command(&params.arguments).await,
            "tcl-lsp.unminifyError" => Ok(Self::unminify_error_command(&params.arguments)),
            "tcl-lsp.describeIruleEvent" => {
                self.describe_irule_event_command(&params.arguments).await
            }
            "tcl-lsp.describeIruleCommand" => {
                self.describe_irule_command_command(&params.arguments).await
            }
            "tcl-lsp.listIruleEvents" => Ok(Some(Self::list_irule_events_command())),
            "tcl-lsp.diagramData" => Ok(self.diagram_data_command(&params.arguments).await),
            "tcl-lsp.xcTranslate" => Ok(self.xc_translate_command(&params.arguments).await),
            "tcl-lsp.xcTranslateDocument" => {
                self.xc_translate_document_command(&params.arguments).await
            }
            "tcl-lsp.listRules" => self.list_rules_command(&params.arguments).await,
            "tcl-lsp.extractRule" => self.extract_rule_command(&params.arguments).await,
            "tcl-lsp.writeRuleBack" => self.write_rule_back_command(&params.arguments).await,
            "tcl-lsp.extractLinkedObjects" => {
                self.extract_linked_objects_command(&params.arguments).await
            }
            "tcl-lsp.bigipCleanup" => self.bigip_cleanup_command(&params.arguments).await,
            "tcl-lsp.minimizeDiagnostic" => {
                self.minimize_diagnostic_command(&params.arguments).await
            }
            "tcl-lsp.renamePartition" => self.rename_partition_command(&params.arguments).await,
            "tcl-lsp.validateDiagnosticPublication" => {
                self.validate_diagnostic_publication_command(&params.arguments)
                    .await
            }
            "tcl-lsp.currentDiagnostics" => {
                self.current_diagnostics_command(&params.arguments).await
            }
            "tcl-lsp.proposalDiagnostics" => {
                self.proposal_diagnostics_command(&params.arguments).await
            }
            "tcl-lsp.getEffectiveConfig" => {
                self.get_effective_config_command(&params.arguments).await
            }
            "tcl-lsp.fixAllSafeIssues" => self.fix_all_safe_issues_command(&params.arguments).await,
            "tcl-lsp.listSubcommands" => Ok(self.list_subcommands_command(&params.arguments).await),
            "tcl-lsp.insertPackageRequire" => {
                self.insert_package_require_command(&params.arguments).await
            }
            "tcl-lsp.listKnownPackages" => Ok(Some(Self::list_known_packages_command())),
            "tcl-lsp.suggestPackagesForSymbol" => Ok(Some(
                Self::suggest_packages_for_symbol_command(&params.arguments),
            )),
            "tcl-lsp.exportConfig" => Ok(Some(self.export_config_command().await)),
            "tcl-lsp.listTclInstallations" => Ok(Some(self.list_tcl_installations_command().await)),
            "tcl-lsp.listDialects" => Ok(Some(Self::list_dialects_command())),
            "tcl-lsp.setDialect" => self.set_dialect_command(&params.arguments).await,
            "tcl-lsp.setSessionDialectOverride" => {
                self.set_session_dialect_override_command(&params.arguments)
                    .await
            }
            "tcl-lsp.studioSource" => self.studio_source_command(&params.arguments).await,
            "tcl-lsp.setDocumentDialectOverride" => {
                self.set_document_dialect_override_command(&params.arguments)
                    .await
            }
            "tcl-lsp.compilerExplorer" => self.compiler_explorer_command(&params.arguments).await,
            "tcl-lsp.compilerExplorerView" => {
                self.compiler_explorer_view_command(&params.arguments).await
            }
            "tcl-lsp.tkPreview" => self.tk_preview_command(&params.arguments).await,
            "tcl-lsp.ilxReferences" => Ok(self.ilx_references_command(&params.arguments).await),
            _ => Ok(None),
        }
    }
}
