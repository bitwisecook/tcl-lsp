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

//! The text a pack's `runtime_backing tcl-body {-package-source PATH}` points at,
//! read at load.
//!
//! A reference body is code the compiler inlines into whatever calls the
//! command, so its text has to be in hand before anything compiles, and nothing
//! that compiles reads a file: the loader resolves the path once, against the
//! package that ships the pack, through the same [`SourceStore`] that discovered
//! the pack. The text is carried on the command ([`PackCommand::reference_text`])
//! and installed beside it ([`tcl_registry::CommandRegistry::insert_reference_text`]),
//! and what was read moves the pack set's key, so a registry built for the set
//! before the package's file changed is not the one built after.
//!
//! The path is the package's own: relative, with no `..`, and read beneath the
//! nearest directory above the pack that holds a `tclpkg.tcl`. A pack no package
//! ships has no such directory and no body.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use tcl_lsp_core::vfs::SourceStore;
use tcl_registry::{BodySource, RuntimeBacking};

use crate::discovery::PACKAGE_MANIFEST;
use crate::loader::PackCommand;
use crate::pack::{PackNotice, Severity};

/// Read the file behind every `PackageSource` backing in `commands` and keep
/// the text on the command. A file that cannot be read is a warning on the
/// command's row and the command keeps its backing: it is declared, and nothing
/// can inline it.
///
/// Returns a digest of every `(path, text)` read, or `None` when nothing was.
pub(crate) fn provision(
    commands: &mut [PackCommand],
    store: &dyn SourceStore,
    notices: &mut Vec<PackNotice>,
) -> Option<u64> {
    let mut digest: Option<xxhash_rust::xxh3::Xxh3> = None;
    for command in commands {
        let RuntimeBacking::TclBody {
            source: BodySource::PackageSource { relative_path },
            ..
        } = command.spec.runtime_backing
        else {
            continue;
        };
        match read(store, &command.file, relative_path) {
            Ok(text) => {
                let hasher = digest.get_or_insert_with(xxhash_rust::xxh3::Xxh3::new);
                hasher.update(relative_path.as_bytes());
                hasher.update(&[0]);
                hasher.update(text.as_bytes());
                hasher.update(&[0]);
                command.reference_text = Some(Arc::from(text));
            }
            Err(reason) => notices.push(PackNotice {
                subject: None,
                path: command.file.clone(),
                line: command.line,
                context: format!("command {}", command.spec.name),
                message: format!(
                    "`runtime_backing tcl-body {{-package-source {relative_path}}}` cannot be \
                     read ({reason}), so a call to `{}` is not inlined",
                    command.spec.name
                ),
                severity: Severity::Warning,
            }),
        }
    }
    digest.map(|hasher| hasher.digest())
}

/// `relative_path` read from the package that ships `pack`.
fn read(store: &dyn SourceStore, pack: &Path, relative_path: &str) -> Result<String, String> {
    let relative = Path::new(relative_path);
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("a package source is a relative path inside the package".to_owned());
    }
    let Some(dir) = package_dir(store, pack) else {
        return Err(format!("no `{PACKAGE_MANIFEST}` above the pack ships it"));
    };
    store
        .read_to_string(&dir.join(relative))
        .map_err(|error| error.to_string())
}

/// The directory of the package that ships `pack`: the nearest one above it
/// that holds a manifest.
fn package_dir(store: &dyn SourceStore, pack: &Path) -> Option<PathBuf> {
    pack.ancestors()
        .skip(1)
        .find(|dir| store.is_file(&dir.join(PACKAGE_MANIFEST)))
        .map(Path::to_path_buf)
}
