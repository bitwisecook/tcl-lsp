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

//! Definition, references, hierarchies, hover routing and workspace symbols across documents.

use super::*;

mod native_pattern_hover;

#[path = "navigation/declaration_identity.rs"]
mod declaration_identity;
#[path = "navigation/declaration_sources.rs"]
mod declaration_sources;
#[path = "navigation/definition.rs"]
mod definition;
#[path = "navigation/hierarchies.rs"]
mod hierarchies;
#[path = "navigation/method_targets.rs"]
mod method_targets;
#[path = "navigation/namespaces.rs"]
mod namespaces;
#[path = "navigation/references.rs"]
mod references;
#[path = "navigation/workspace_symbols.rs"]
mod workspace_symbols;

/// Build the three-file namespace-path workspace: `::mymod::helper`, an unrelated
/// `::other::helper`, and an `app.tcl` that reaches `::mymod::helper` via
/// `namespace path`.  Returns the backend and the three URIs.
async fn register_namespace_path_workspace() -> (Backend, Uri, Uri, Uri) {
    let backend = test_backend();
    let mymod = Uri::from_str("file:///mymod.tcl").unwrap();
    let other = Uri::from_str("file:///other.tcl").unwrap();
    let app = Uri::from_str("file:///app.tcl").unwrap();
    register(
        &backend,
        &mymod,
        "namespace eval ::mymod { proc helper {} {} }\n",
    )
    .await;
    register(
        &backend,
        &other,
        "namespace eval ::other { proc helper {} {} }\n",
    )
    .await;
    register(
        &backend,
        &app,
        "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
    )
    .await;
    (backend, mymod, other, app)
}
