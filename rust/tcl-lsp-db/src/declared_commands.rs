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

//! The command declarations a document's unit builds under: its own stubs and the workspace procedures that leave the command world unchanged.

use super::compilation_units::document_compilation_unit;
use super::{SourceFile, TclDb};
use std::sync::Arc;
use tcl_registry::CommandRegistry;
use tcl_registry::model::DeclaredSurface;

/// The document's own command declarations — its inline `# tcl-lsp: stub`
/// block and the nearest `<dialect>.tcl.stubs` sidecar, ingested through the
/// analyser's one stub-ingestion path
/// ([`tcl_compiler::analyser::utils::document_declared_surface`]).
///
/// Every unit built for this file declares the same thing, so a stubbed
/// command's `body` / `var` argument roles reach lowering and the
/// interprocedural scan exactly as a shipped `CommandSpec`'s do. Cache
/// invalidation rides the ordinary inputs — the document's text and path for
/// an inline block, [`SourceFile::sidecar_stubs_epoch`] for a sidecar.
#[salsa::tracked(returns(clone))]
fn own_declared_command_surface(db: &dyn TclDb, file: SourceFile) -> Arc<DeclaredSurface> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    Arc::new(tcl_compiler::analyser::utils::document_declared_surface(
        file.text(db),
        file.path(db).as_deref(),
        file.dialect(db),
    ))
}

/// [`own_declared_command_surface`] over the workspace procedures the
/// document's `registry` admits
/// ([`DeclaredSurface::workspace_procedures_under`]): a workspace procedure
/// never shadows a shipped or pack command of the same name.
pub fn declared_command_surface(
    db: &dyn TclDb,
    file: SourceFile,
    registry: &CommandRegistry,
) -> Arc<DeclaredSurface> {
    let own = own_declared_command_surface(db, file);
    let Some(workspace) = file.workspace_declared_commands(db) else {
        return own;
    };
    Arc::new(Arc::unwrap_or_clone(own).over_workspace_procedures(Some(workspace), registry))
}

/// [`world_preserving_procedure_stubs`] as the declarations another document
/// adopts. A procedure this document also declares by stub, inline or in its
/// sidecar, travels as that stated declaration; every other one is inferred
/// from its formals ([`DeclaredCommand::inferred_from_procedure`]), which a
/// user pack covering the same command may answer for instead.
///
/// [`DeclaredCommand::inferred_from_procedure`]: tcl_registry::model::DeclaredCommand::inferred_from_procedure
#[salsa::tracked(returns(clone))]
pub fn world_preserving_procedure_declarations(
    db: &dyn TclDb,
    file: SourceFile,
) -> Arc<DeclaredSurface> {
    let rooted = |name: &str| {
        if tcl_syntax::naming::is_absolute(name) {
            name.to_owned()
        } else {
            tcl_syntax::naming::root_unrooted_key(name)
        }
    };
    let inferred = tcl_compiler::analyser::utils::workspace_procedure_declarations(
        &world_preserving_procedure_stubs(db, file).join("\n"),
    );
    let own = own_declared_command_surface(db, file);
    let mut surface = DeclaredSurface::new();
    for (name, command) in inferred.iter() {
        let stated = own
            .iter()
            .find(|(own_name, _)| rooted(own_name) == rooted(name))
            .map(|(_, stated)| {
                let mut stated = stated.clone();
                name.clone_into(&mut stated.name);
                stated.declaration.provenance = command.declaration.provenance;
                stated
            });
        surface.declare(stated.unwrap_or_else(|| command.clone()));
    }
    Arc::new(surface)
}

/// The workspace declarations a document adopts from the other documents'
/// [`world_preserving_procedure_declarations`], merged in `providers` order so
/// a name two documents define resolves the same way every time; `None` when
/// they declare nothing.
pub fn merged_workspace_procedure_declarations(
    db: &dyn TclDb,
    providers: impl IntoIterator<Item = SourceFile>,
) -> Option<Arc<DeclaredSurface>> {
    let mut merged = DeclaredSurface::new();
    for provider in providers {
        for (_, command) in world_preserving_procedure_declarations(db, provider).iter() {
            merged.declare(command.clone());
        }
    }
    (!merged.is_empty()).then(|| Arc::new(merged))
}

/// The procedures this document defines that a call from another document
/// may trust, as sidecar-syntax `stub` lines: every declaration-time walk of
/// the body leaves the command world unchanged. The argument counts follow the
/// document's own parameter grammar
/// ([`tcl_syntax::formal_params::formal_argument_count_shape`]): C Tcl's rest
/// formal is a final `args` and only its trailing defaults are optional; Jim's
/// `args` is a rest formal anywhere and any default is optional. A rest formal
/// has no stub spelling, so such a procedure is not declared, nor is any when
/// the grammar is unknown.
#[salsa::tracked(returns(clone))]
pub fn world_preserving_procedure_stubs(db: &dyn TclDb, file: SourceFile) -> Arc<Vec<String>> {
    let unit = document_compilation_unit(db, file);
    let Some(bindings) = unit.ir_module.retained_source_bindings.as_deref() else {
        return Arc::default();
    };
    let Some(grammar) = unit.ir_module.parameter_grammar() else {
        return Arc::default();
    };
    let mut lines = bindings
        .world_preserving_procedures()
        .filter_map(|(identity, formals)| {
            let shape = tcl_syntax::formal_params::formal_argument_count_shape(
                formals
                    .iter()
                    .map(|formal| (formal.name == "args", formal.default.is_some())),
                grammar,
            );
            shape.maximum?;
            let words = formals
                .iter()
                .enumerate()
                .map(|(index, formal)| {
                    let optional = formal.default.is_some()
                        && (grammar == tcl_dialect::ParameterGrammar::Jim
                            || index >= shape.minimum);
                    if optional {
                        format!("?{}?", formal.name)
                    } else {
                        formal.name.clone()
                    }
                })
                .collect::<Vec<_>>();
            // A global procedure is declared under the name a bare call
            // spells; a namespaced one keeps its qualified name.
            let name = tcl_syntax::naming::unroot_rooted_key(identity)
                .filter(|tail| !tcl_syntax::naming::is_qualified(tail.as_bytes()))
                .unwrap_or(identity);
            Some(format!("stub {name} {{{}}}", words.join(" ")))
        })
        .collect::<Vec<_>>();
    lines.sort_unstable();
    Arc::new(lines)
}
