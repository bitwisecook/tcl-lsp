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

use super::*;

fn fixture() -> (Uri, DocumentState, Arc<Mutex<HashMap<Uri, DiagSlot>>>) {
    let uri = Uri::from_file_path("/proj/orphan.tcl").unwrap();
    let mut document = DocumentState::new("helper 1\n".to_owned(), "tcl8.6".to_owned());
    document.backing_file_deleted = true;
    // Exercise the same producer setup as the original fixed fixture.
    let mut analyser = Analyser::new();
    let analysis = Arc::new(analyser.analyse(&document.text, &document.dialect));
    let index = core_workspace_index::WorkspaceIndex::default();
    let queries = settle_cross_file_calls(
        &index,
        &analysis,
        analysis.resolved_registry().unwrap(),
        uri.as_str(),
    )
    .command_queries;
    let slots = Arc::new(Mutex::new(HashMap::new()));
    document.orphan_command_dependencies = Some(Arc::new(OrphanCommandDependencies {
        uri: uri.clone(),
        source: Arc::clone(&document.text),
        dialect: document.dialect.clone(),
        revision: document.revision,
        version: document.version,
        analysis,
        expected_epoch: 0,
        current_epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        inputs_gate: Arc::new(tokio::sync::RwLock::new(())),
        slots: Arc::downgrade(&slots),
        slot_generation: None,
        factory_generation: Arc::new(tokio::sync::RwLock::new(0)),
        expected_factory_generation: 0,
        queries,
    }));
    (uri, document, slots)
}

fn unrelated() -> IndexedDiagnosticChange {
    IndexedDiagnosticChange {
        command_names: HashSet::from(["::unrelated".to_owned()]),
        ..IndexedDiagnosticChange::default()
    }
}

#[test]
fn absent_or_broad_receipts_wake_instead_of_claiming_source_absence() {
    let (uri, mut document, _slots) = fixture();
    let dependency = document.orphan_command_dependencies.take().unwrap();
    assert!(!dependency.may_read_commands(&uri, &document));
    let mut docs = HashMap::from([(uri.clone(), document)]);
    assert_eq!(
        orphaned_fact_consumers(&docs, &unrelated()),
        vec![uri.as_str().to_owned()]
    );
    let document = docs.get_mut(&uri).unwrap();
    let mut broad = Arc::try_unwrap(dependency).ok().unwrap();
    broad.queries = WorkspaceCommandQueries::Broad;
    document.orphan_command_dependencies = Some(Arc::new(broad));
    assert_eq!(
        orphaned_fact_consumers(&docs, &unrelated()),
        vec![uri.as_str().to_owned()]
    );
}

#[test]
fn source_epoch_busy_gate_factory_and_pending_slot_retire_noread() {
    let (uri, mut document, _slots) = fixture();
    let dependency = document.orphan_command_dependencies.as_ref().unwrap();
    assert!(!dependency.may_read_commands(&uri, &document));
    {
        let _write = dependency.inputs_gate.try_write().unwrap();
        assert!(dependency.may_read_commands(&uri, &document));
    }
    dependency
        .current_epoch
        .store(1, std::sync::atomic::Ordering::Release);
    assert!(dependency.may_read_commands(&uri, &document));
    dependency
        .current_epoch
        .store(0, std::sync::atomic::Ordering::Release);
    *dependency.factory_generation.try_write().unwrap() = 1;
    assert!(dependency.may_read_commands(&uri, &document));
    *dependency.factory_generation.try_write().unwrap() = 0;
    dependency
        .slots
        .upgrade()
        .unwrap()
        .try_lock()
        .unwrap()
        .insert(uri.clone(), DiagSlot::default());
    assert!(dependency.may_read_commands(&uri, &document));
    dependency
        .slots
        .upgrade()
        .unwrap()
        .try_lock()
        .unwrap()
        .clear();
    document.text = "changed 1\n".into();
    assert!(
        document
            .orphan_command_dependencies
            .as_ref()
            .unwrap()
            .may_read_commands(&uri, &document)
    );
}
