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
#[path = "e2e/issue1001.rs"]
mod issue1001;
#[path = "e2e/issue1019_proc_args.rs"]
mod issue1019_proc_args;
#[path = "e2e/issue1088_namespace_symbols.rs"]
mod issue1088_namespace_symbols;
#[path = "e2e/issue1122_sticky_scroll.rs"]
mod issue1122_sticky_scroll;
#[path = "e2e/issue1137_call_site_resolution.rs"]
mod issue1137_call_site_resolution;
#[path = "e2e/issue1214_uri_canonicalisation.rs"]
mod issue1214_uri_canonicalisation;
#[path = "e2e/issue1281_ensemble_rename.rs"]
mod issue1281_ensemble_rename;
#[path = "e2e/issue1296_metaclass_chain.rs"]
mod issue1296_metaclass_chain;
#[path = "e2e/issue1302_import_builtin_shadow.rs"]
mod issue1302_import_builtin_shadow;
#[path = "e2e/issue1305_renamed_metaclass.rs"]
mod issue1305_renamed_metaclass;
#[path = "e2e/issue1312_named_object_dispatch.rs"]
mod issue1312_named_object_dispatch;
#[path = "e2e/issue1326_encoding.rs"]
mod issue1326_encoding;
#[path = "e2e/issue1333_diagnostic_tags.rs"]
mod issue1333_diagnostic_tags;
#[path = "e2e/issue1345_transport_liveness.rs"]
mod issue1345_transport_liveness;
#[path = "e2e/issue923_class_refs.rs"]
mod issue923_class_refs;
#[path = "e2e/issue923_crossdoc.rs"]
mod issue923_crossdoc;
#[path = "e2e/issue923_idx80_mathfunc.rs"]
mod issue923_idx80_mathfunc;
#[path = "e2e/issue945.rs"]
mod issue945;
#[path = "e2e/issue954_followup.rs"]
mod issue954_followup;
#[path = "e2e/issue996_stack_overflow.rs"]
mod issue996_stack_overflow;
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
#[path = "e2e/rename_safety.rs"]
mod rename_safety;
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
#[path = "e2e/tcloo_navigation.rs"]
mod tcloo_navigation;
#[path = "e2e/tk_dialect.rs"]
mod tk_dialect;
#[path = "e2e/unicode_positions.rs"]
mod unicode_positions;
#[path = "e2e/vscode_parity.rs"]
mod vscode_parity;
