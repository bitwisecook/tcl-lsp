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

//! Consolidated LSP end-to-end suite.
//!
//! All `*_e2e` suites share one test binary so the harness (`common/`) is
//! compiled + linked once (not per-file) and every test runs in one parallel
//! pool — a large build- and run-time win over one binary per suite. Files
//! live under `tests/e2e/` (a non-target subdir); `#[path]` points at them.

#[path = "e2e/bigip.rs"]
mod bigip;
#[path = "e2e/code_actions.rs"]
mod code_actions;
#[path = "e2e/commands.rs"]
mod commands;
#[path = "e2e/common/mod.rs"]
mod common;
#[path = "e2e/completion.rs"]
mod completion;
#[path = "e2e/config.rs"]
mod config;
#[path = "e2e/definition.rs"]
mod definition;
#[path = "e2e/diagnostic_matrix.rs"]
mod diagnostic_matrix;
#[path = "e2e/diagnostics.rs"]
mod diagnostics;
#[path = "e2e/document_highlight.rs"]
mod document_highlight;
#[path = "e2e/document_symbols.rs"]
mod document_symbols;
#[path = "e2e/edit_tracking_stress.rs"]
mod edit_tracking_stress;
#[path = "e2e/editor_features.rs"]
mod editor_features;
#[path = "e2e/hover.rs"]
mod hover;
#[path = "e2e/invariants.rs"]
mod invariants;
#[path = "e2e/irules.rs"]
mod irules;
#[path = "e2e/issue923_class_refs.rs"]
mod issue923_class_refs;
#[path = "e2e/issue945.rs"]
mod issue945;
#[path = "e2e/issue954_followup.rs"]
mod issue954_followup;
#[path = "e2e/name_resolution.rs"]
mod name_resolution;
#[path = "e2e/navigation.rs"]
mod navigation;
#[path = "e2e/navigation_extras.rs"]
mod navigation_extras;
#[path = "e2e/precision_review.rs"]
mod precision_review;
#[path = "e2e/recovery.rs"]
mod recovery;
#[path = "e2e/references.rs"]
mod references;
#[path = "e2e/rename.rs"]
mod rename;
#[path = "e2e/semantic_tokens.rs"]
mod semantic_tokens;
#[path = "e2e/semantic_tokens_reference_client.rs"]
mod semantic_tokens_reference_client;
#[path = "e2e/server_version.rs"]
mod server_version;
#[path = "e2e/signature_help.rs"]
mod signature_help;
#[path = "e2e/structure.rs"]
mod structure;
#[path = "e2e/tcl91.rs"]
mod tcl91;
#[path = "e2e/tk_dialect.rs"]
mod tk_dialect;
#[path = "e2e/unicode_positions.rs"]
mod unicode_positions;
#[path = "e2e/vscode_parity.rs"]
mod vscode_parity;
