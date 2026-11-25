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

//! Conformance rows: every probe fixture under C Tcl 8.4–9.1, jimsh, the VM
//! and the runtime, and through the analyser, the folder (run before and
//! after its rewrites) and the editor features at marked sites. Each fixture
//! is one test; it fails when a committed probe output disagrees with it.
//! `cargo xtask conformance --update` rewrites the outputs.

mod artefacts;
mod engines;
mod rows;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use libtest_mimic::{Arguments, Failed, Trial};

const UPDATE: &str = "TCL_LSP_CONFORMANCE_UPDATE";

fn probe_root() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/work/name-handling/handover/oo-dispatch-probe"
    ))
    .canonicalize()
    .expect("probe root")
}

fn main() -> ExitCode {
    let arguments = Arguments::from_args();
    let root = probe_root();
    let repository = root.join("../../../../..").canonicalize().expect("repository root");
    let update = std::env::var_os(UPDATE).is_some();
    let fixtures = rows::fixtures(&root);
    let artefacts = Arc::new(artefacts::catalogue(&root, &fixtures));
    let interpreters = Arc::new(std::sync::OnceLock::<engines::Interpreters>::new());
    let rendered: Arc<Mutex<BTreeMap<usize, BTreeMap<String, Vec<String>>>>> = Arc::default();

    let mut trials: Vec<Trial> = fixtures
        .iter()
        .cloned()
        .map(|fixture| {
            let artefacts = Arc::clone(&artefacts);
            let interpreters = Arc::clone(&interpreters);
            let rendered = Arc::clone(&rendered);
            let repository = repository.clone();
            Trial::test(format!("row::{}", fixture.name()), move || {
                let interpreters = interpreters.get_or_init(engines::Interpreters::from_environment);
                let row = rows::compute(&fixture, interpreters, &repository);
                let mut differences = String::new();
                for (index, artefact) in artefacts.iter().enumerate() {
                    if !artefact.covers(&fixture) {
                        continue;
                    }
                    let lines = artefact.render(&fixture, &row);
                    if lines != artefact.committed(&fixture) {
                        differences.push_str(&format!(
                            "{}:\n  committed: {:#?}\n  produced:  {:#?}\n",
                            artefact.name,
                            artefact.committed(&fixture),
                            lines
                        ));
                    }
                    rendered
                        .lock()
                        .expect("rendered lines")
                        .entry(index)
                        .or_default()
                        .insert(fixture.name(), lines);
                }
                if differences.is_empty() || update {
                    Ok(())
                } else {
                    Err(Failed::from(differences))
                }
            })
        })
        .collect();
    for artefact in artefacts.iter() {
        let errors = artefact.structure_errors(&fixtures);
        let name = format!("artefact::{}", artefact.name);
        trials.push(Trial::test(name, move || {
            if errors.is_empty() || update {
                Ok(())
            } else {
                Err(Failed::from(errors.join("\n")))
            }
        }));
    }

    let conclusion = libtest_mimic::run(&arguments, trials);
    if update {
        let rendered = rendered.lock().expect("rendered lines");
        for (index, artefact) in artefacts.iter().enumerate() {
            artefact.write(&fixtures, rendered.get(&index).unwrap_or(&BTreeMap::new()));
        }
    }
    conclusion.exit_code()
}
