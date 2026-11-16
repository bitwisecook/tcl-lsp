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

//! The diagnostics pipeline, from scheduling to publication, and its workspace refinements.

use super::*;

#[path = "diagnostics/analysis_passes.rs"]
mod analysis_passes;
#[path = "diagnostics/closed_files.rs"]
mod closed_files;
#[path = "diagnostics/command_refinement.rs"]
mod command_refinement;
#[path = "diagnostics/consumers.rs"]
mod consumers;
#[path = "diagnostics/f5_models.rs"]
mod f5_models;
#[path = "diagnostics/inputs.rs"]
mod inputs;
#[path = "diagnostics/package_refinement.rs"]
mod package_refinement;
#[path = "diagnostics/publication.rs"]
mod publication;
#[path = "diagnostics/publisher.rs"]
mod publisher;
#[path = "diagnostics/pull.rs"]
mod pull;
#[path = "diagnostics/report.rs"]
mod report;
#[path = "diagnostics/run.rs"]
mod run;
#[path = "diagnostics/scheduling.rs"]
mod scheduling;

fn ws_index(docs: &[(&Uri, &str)]) -> core_workspace_index::WorkspaceIndex {
    let analyses: Vec<(String, tcl_compiler::analyser::AnalysisResult)> = docs
        .iter()
        .map(|(uri, src)| {
            let mut a = tcl_compiler::analyser::Analyser::new();
            ((*uri).as_str().to_owned(), a.analyse(src, "tcl8.6").clone())
        })
        .collect();
    core_workspace_index::WorkspaceIndex::from_documents(
        analyses.iter().map(|(u, a)| (u.as_str(), a)),
    )
}

fn has_code(diags: &[tower_lsp_server::ls_types::Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == code))
}

fn unassociated_delivery<'a>(
    backend: &'a Backend,
    uri: &'a Uri,
    currency: DiagCurrency,
    version: Option<i32>,
) -> DeliveryCtx<'a> {
    let sidecar_revision = backend.fresh_analysis_cache.lock().unwrap().revision();
    DeliveryCtx {
        sidecar_revision,
        inputs_epoch: Some(backend.diag_inputs_epoch()),
        configuration_epoch: backend.diag_inputs_epoch(),
        source_inputs_epoch: Arc::clone(&backend.diag_inputs_epoch),
        source_inputs_gate: Arc::clone(&backend.analyser_inputs_gate),
        diagnostic_pass: None,
        client: &backend.client,
        diagnostic_publisher: &backend.diagnostic_publisher,
        documents: &backend.documents,
        store: &backend.store,
        diag_slots: &backend.diag_slots,
        pull_diag_cache: &backend.pull_diag_cache,
        closed_diag_gen: &backend.closed_diag_gen,
        source: "",
        pack_seq: 0,
        pack_set: None,
        registry: None,
        spec_packs: None,
        uri,
        currency,
        version,
        client_supports_pull: false,
    }
}
