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

//! The compiled extensions a pack's `-host wasm_extension` implementations
//! run (`evaluate -implementation ID -host wasm_extension { extension FILE
//! PREFIX … }`), read at load.
//!
//! Nothing that evaluates reads a file, so the artefact is read once, beside
//! the pack that names it, through the [`SourceStore`] that read the pack.
//! The implementation carries its bytes, and its identity the artefact's
//! content hash ([`tcl_registry::extension_host::artefact_hash`]), so the
//! memo key names the artefact and an edited one is another implementation;
//! what was read moves the pack set's key. An artefact that cannot be read
//! leaves its implementation without one, which declines `Transient` as a
//! worker with no extension host does, and says so on the command's row.

use std::path::Path;

use tcl_lsp_core::vfs::SourceStore;
use tcl_registry::spec::SubCommand;
use tcl_registry::value_transfer::{DeclaredEvaluation, ExtensionArtefact, SemanticsDeclaration};

use crate::loader::PackCommand;
use crate::pack::{PackNotice, Severity};

/// Read the artefact of every `-host wasm_extension` implementation the
/// commands of pack `pack` declare, at command or subcommand scope.
///
/// Returns a digest of every `(file, bytes)` read, or `None` when nothing was.
pub(crate) fn provision(
    pack: &str,
    commands: &mut [PackCommand],
    store: &dyn SourceStore,
    notices: &mut Vec<PackNotice>,
) -> Option<u64> {
    let mut digest: Option<xxhash_rust::xxh3::Xxh3> = None;
    let mut pack_name: Option<&'static str> = None;
    for command in commands {
        let spec = command.spec;
        let mut read = |declaration: SemanticsDeclaration| {
            let artefact = artefact_of(declaration)?;
            let path = command
                .file
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(artefact.file);
            Some(match store.read(&path) {
                Ok(bytes) => {
                    let hasher = digest.get_or_insert_with(xxhash_rust::xxh3::Xxh3::new);
                    hasher.update(artefact.file.as_bytes());
                    hasher.update(&[0]);
                    hasher.update(&bytes);
                    let pack = *pack_name.get_or_insert_with(|| crate::loader::leak_str(pack));
                    Ok(with_artefact(declaration, pack, bytes))
                }
                Err(error) => Err(format!(
                    "the extension `{}` its `-host wasm_extension` implementation runs cannot \
                     be read ({error}), so a call to `{}` declines that implementation",
                    artefact.file, spec.name
                )),
            })
        };
        let mut declared = None;
        let mut failures = Vec::new();
        match read(spec.semantics) {
            Some(Ok(provisioned)) => declared = Some(provisioned),
            Some(Err(why)) => failures.push(why),
            None => {}
        }
        let mut subcommands: Option<Vec<SubCommand>> = None;
        for (at, sub) in spec.subcommands.iter().enumerate() {
            match read(sub.semantics) {
                Some(Ok(provisioned)) => {
                    subcommands.get_or_insert_with(|| spec.subcommands.to_vec())[at].semantics =
                        provisioned;
                }
                Some(Err(why)) => failures.push(why),
                None => {}
            }
        }
        for message in failures {
            notices.push(PackNotice {
                subject: None,
                path: command.file.clone(),
                line: command.line,
                context: format!("command {}", spec.name),
                message,
                severity: Severity::Warning,
            });
        }
        if declared.is_some() || subcommands.is_some() {
            let mut clone = spec.clone();
            if let Some(declared) = declared {
                clone.semantics = declared;
            }
            if let Some(subcommands) = subcommands {
                clone.subcommands = Box::leak(subcommands.into_boxed_slice());
            }
            command.spec = Box::leak(Box::new(clone));
        }
    }
    digest.map(|hasher| hasher.digest())
}

/// The extension a declaration's implementation runs, while its artefact is
/// still to be read.
fn artefact_of(declaration: SemanticsDeclaration) -> Option<ExtensionArtefact> {
    let SemanticsDeclaration::Declared(semantics) = declaration else {
        return None;
    };
    let DeclaredEvaluation::Implementation(implementation) = semantics.as_declared()?.evaluation
    else {
        return None;
    };
    implementation
        .extension
        .filter(|artefact| artefact.bytes.is_none())
}

/// `declaration` with its extension's artefact read: the bytes on the
/// implementation, and the identity naming the pack and the artefact's hash.
fn with_artefact(
    declaration: SemanticsDeclaration,
    pack: &'static str,
    bytes: Vec<u8>,
) -> SemanticsDeclaration {
    let SemanticsDeclaration::Declared(semantics) = declaration else {
        return declaration;
    };
    let Some(&declared) = semantics.as_declared() else {
        return declaration;
    };
    let mut declared = declared;
    if let DeclaredEvaluation::Implementation(implementation) = &mut declared.evaluation
        && let Some(artefact) = &mut implementation.extension
    {
        let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        artefact.bytes = Some(bytes);
        implementation.capability.identity.pack = pack;
        implementation.capability.identity.content_hash =
            tcl_registry::extension_host::artefact_hash(bytes);
    }
    SemanticsDeclaration::Declared(Box::leak(Box::new(declared)))
}
