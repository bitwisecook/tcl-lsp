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

//! The generated half of the `CommandSpec` reference manual stays in step
//! with the studio schema it is rendered from.
//!
//! Regenerate with
//! `UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`.

use std::path::PathBuf;

fn doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/references/command-spec/fields.md")
}

#[test]
fn fields_doc_matches_the_schema() {
    let rendered = tcl_spec_studio::reference::render_fields_doc();
    let path = doc_path();
    if std::env::var_os("UPDATE_REFERENCE").is_some() {
        std::fs::create_dir_all(path.parent().expect("doc dir")).expect("create doc dir");
        std::fs::write(&path, &rendered).expect("write fields.md");
        return;
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == rendered,
        "docs/references/command-spec/fields.md is out of date with the \
         studio schema — regenerate with UPDATE_REFERENCE=1 cargo test -p \
         tcl-spec-studio --test reference_doc"
    );
}
