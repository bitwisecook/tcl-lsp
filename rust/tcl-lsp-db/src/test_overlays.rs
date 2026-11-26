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

//! Workspace command overlays for the database's tests: specs installed over
//! a dialect's registry under an overlay key, as a workspace pack's install
//! leaves them, so a test can change what a command declares and watch the
//! database re-key. The database reads the installed registry by its overlay
//! key and never the pack that produced it, so the tests build the specs in
//! Rust rather than load `SpecTcl`.

use std::sync::Arc;

use tcl_registry::native_compilation::{
    NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
};
use tcl_registry::pack_origin::PackOrigin;
use tcl_registry::registry::CommandRegistry;
use tcl_registry::spec::CommandSpec;

/// Install `specs` over `dialect`'s registry under `overlay`, each recorded as
/// a trusted workspace pack `pack`'s and, when it declares no compiler, given
/// the plain invocation a pack install gives an added command.
///
/// The registry is cached by `(dialect, overlay)`: a test that changes the
/// specs uses a new key, as an edited pack's content key changes.
pub(crate) fn install(
    dialect: &str,
    pack: &str,
    overlay: u64,
    specs: Vec<CommandSpec>,
) -> Arc<CommandRegistry> {
    let profile = tcl_registry::model::resolve_environment(dialect).analyser_profile();
    tcl_registry::model::profile_store_with_overlay(profile, overlay, |registry| {
        for mut spec in specs {
            spec.native_compilation = spec.native_compilation.or(Some(NativeCompilationSpec {
                grammar: NativeCompilationGrammar::NoHook,
                operation: tcl_registry::SemanticOperationId::Invoke,
                body: NativeBodyCompilation::Inherit,
            }));
            let installed: &'static CommandSpec = Box::leak(Box::new(spec));
            registry.insert_static(installed);
            registry.insert_pack_origin(
                installed,
                PackOrigin {
                    pack: pack.to_owned(),
                    content_hash: overlay,
                    vocabulary_version: "2.0".to_owned(),
                    provenance: tcl_dialect::model::Provenance::WorkspaceTrusted,
                },
            );
        }
    })
}
