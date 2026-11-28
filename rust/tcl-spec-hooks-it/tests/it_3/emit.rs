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

//! The emitter verbs a pack body calls, with the pinned engine's captured
//! metadata-list receiver.

use std::rc::Rc;

use tcl_engine_api::{Engine, EngineError, HostMetadataListReader, Value};
use tcl_registry::pack_hooks::HookFamily;
use tcl_spec_hooks::emit::{Emission, Reading, Sink, verbs_for_with_metadata_reader};

/// The `-allowed` words the literal validator's `invalid` verb reads from
/// `allowed` under `reader`.
fn allowed(allowed: &str, reader: &HostMetadataListReader) -> Result<Vec<String>, EngineError> {
    let sink = Rc::new(Sink::default());
    let reading = Rc::new(Reading::default());
    let verbs = verbs_for_with_metadata_reader(
        HookFamily::LiteralArgumentValidator,
        &sink,
        &reading,
        Some(reader),
    );
    let (_, verb) = verbs.iter().find(|(name, _)| *name == "invalid").unwrap();
    verb.invoke(&[
        Value::string("-index"),
        Value::Int(0),
        Value::string("-subject"),
        Value::string("colour"),
        Value::string("-allowed"),
        Value::string(allowed),
    ])?;
    let emitted = sink.drain();
    let [Emission::Invalid(issue)] = emitted.as_slice() else {
        panic!("actual metadata emitter")
    };
    Ok(issue
        .allowed_values
        .iter()
        .map(|word| (*word).to_owned())
        .collect())
}

fn actual_receiver<E: Engine>(mut engine: E) {
    engine.set_release("tcl8.5").unwrap();
    let original_reader = engine.host_metadata_list_reader().unwrap();
    assert_eq!(
        allowed("{red green} café", &original_reader).unwrap(),
        ["red green", "café"]
    );
    assert_eq!(
        allowed("{red green} café {[inert]}", &original_reader).unwrap(),
        ["red green", "café", "[inert]"]
    );
    assert_eq!(allowed(r"\x4142", &original_reader).unwrap(), ["B"]);
    assert!(allowed("{unclosed", &original_reader).is_err());
    engine.set_release("tcl9.0").unwrap();
    let current_reader = engine.host_metadata_list_reader().unwrap();
    assert_eq!(allowed(r"\x4142", &current_reader).unwrap(), ["A42"]);
    // A decoder is an immutable selected grammar snapshot, not a claim
    // about a later mutable engine or its current native list objects.
    assert_eq!(allowed(r"\x4142", &original_reader).unwrap(), ["B"]);
}

#[test]
fn actual_vm_and_runtime_metadata_receivers_keep_grouping_and_release() {
    crate::install_surfaces();
    actual_receiver(tcl_engine_tclvm::TclVmEngine::new());
    actual_receiver(tcl_runtime::engine::RuntimeEngine::new());
}
