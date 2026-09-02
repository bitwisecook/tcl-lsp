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

//! **Issue #1643, as `SpecTcl` 2.0 answers it**: an ambient package version
//! that applies under one of a pack's environments and not another.
//!
//! The issue asked for `ambient_package NAME VERSION -dialects {…}` — a
//! flag narrowing a pack-wide claim to some of the file's dialects. 2.0
//! states the same fact where it belongs instead, as an `ambient` placement
//! **inside** the environment that has the package, and a version several
//! environments share is an ordinary Tcl variable substituted into each
//! row. This file is the acceptance: one pack, two environments, one
//! variable, and a version floor that reaches a document resolved to the
//! first environment and not the second.
//!
//! It runs in its own test binary on purpose. Environment registration is
//! process-global, and `publish_pack_set` in a sibling file retires what
//! this one registers.

use tcl_compiler::analyser::Analyser;
use tcl_dialect::model::Placement;
use tcl_spectcl::registration::register_pack_environments;
use tcl_spectcl::{Tier, evaluate_pack};

/// A pack declaring one Tk version and two environments: one that has Tk
/// ambient at that version, one that says nothing about Tk at all.
const PACK: &str = "speclib ambientpack 2.0 {\n\
                    set tkver 8.6\n\
                    environment ambientpack-shell \"\n\
                    display_name {Ambientpack Shell}\n\
                    core tcl 8.6\n\
                    ambient Tk $tkver\n\
                    \"\n\
                    environment ambientpack-plain {\n\
                    \x20   display_name {Ambientpack Plain}\n\
                    \x20   core tcl 8.6\n\
                    }\n\
                    }\n";

/// `-placeholder` on `entry` arrived in Tk 8.7, so a document with a Tk
/// floor of 8.6 draws W136 and one with no floor at all draws nothing.
const DOCUMENT: &str = "entry .e -placeholder hi\n";

fn version_gate_codes(source: &str, environment: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, environment)
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code.as_str(), "W135" | "W136" | "W139"))
        .map(|diagnostic| diagnostic.code.to_string())
        .collect()
}

fn codes(source: &str, environment: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, environment)
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .collect()
}

#[test]
fn an_environment_scoped_ambient_version_floors_only_its_own_environment() {
    let pack = evaluate_pack(PACK);
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert!(pack.load_error.is_none(), "{:?}", pack.load_error);

    // The shared variable reached the placement as a version, not a
    // spelling — the whole point of writing it once.
    let placement = pack
        .environments
        .iter()
        .find(|environment| environment.id == "ambientpack-shell")
        .expect("the ambient environment loads")
        .placements
        .iter()
        .find(|row| row.package == "Tk")
        .expect("the Tk placement loads");
    assert!(placement.ambient);
    assert!(
        matches!(&placement.version, Placement::Pinned(version) if version.to_string() == "8.6"),
        "{:?}",
        placement.version
    );

    let outcome = register_pack_environments(&pack, Tier::User).expect("registration succeeds");
    assert_eq!(outcome.declared, 2);

    // The floor applies where the pack placed the package…
    assert_eq!(
        version_gate_codes(DOCUMENT, "ambientpack-shell"),
        vec!["W136".to_owned()],
        "the environment's own ambient Tk 8.6 is the floor `-placeholder` fails"
    );
    // …and nowhere else. The second environment shares the pack, the core
    // release, and the Tk-using document, and differs only in having no
    // placement — which is exactly the scoping #1643 asked for.
    assert!(
        version_gate_codes(DOCUMENT, "ambientpack-plain").is_empty(),
        "an environment with no Tk placement has no Tk floor: {:?}",
        version_gate_codes(DOCUMENT, "ambientpack-plain")
    );

    // The same placement decides the *other* ambient question — whether the
    // document needs a `package require` — so the two answers cannot drift.
    assert!(
        !codes("entry .e\n", "ambientpack-shell").contains(&"W120".to_owned()),
        "Tk is ambient here, so nothing asks for a require: {:?}",
        codes("entry .e\n", "ambientpack-shell")
    );
    assert!(
        codes("entry .e\n", "ambientpack-plain").contains(&"W120".to_owned()),
        "Tk is not placed here, so the missing require is real: {:?}",
        codes("entry .e\n", "ambientpack-plain")
    );
}
