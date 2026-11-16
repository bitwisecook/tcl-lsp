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

//! Retiring and reviving source inputs.

use super::*;

/// Removing a source retires its salsa input with an emptied payload
/// instead of dropping it, and re-creating the same URI revives that handle
/// rather than allocating a second one salsa can never free.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn db_remove_source_retires_input_for_reuse() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///retired.tcl").unwrap();
    backend
        .db_set_source(&uri, "proc foo {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let first = *backend
        .db_files
        .lock()
        .await
        .get(&uri)
        .expect("the source is tracked once set");

    backend.db_remove_source(&uri).await;

    assert!(
        !backend.db_files.lock().await.contains_key(&uri),
        "a removed source must leave the live file set",
    );
    {
        let db = backend.db.lock().await;
        let tombstones = backend.db_tombstones.lock().await;
        let retired = *tombstones
            .get(&uri)
            .expect("the input must be retired, not dropped — salsa never frees one");
        assert!(retired == first, "the retired handle is the original input");
        assert!(
            retired.text(&*db).is_empty(),
            "a retired input must release its text: {:?}",
            retired.text(&*db),
        );
        assert!(
            retired.external_call_sites(&*db).is_none(),
            "a retired input must release its cross-file evidence",
        );
    }

    backend
        .db_set_source(&uri, "proc bar {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let second = *backend
        .db_files
        .lock()
        .await
        .get(&uri)
        .expect("the re-created source is tracked again");
    assert!(
        first == second,
        "re-creating a removed URI must revive its retired input",
    );
    assert!(
        backend.db_tombstones.lock().await.is_empty(),
        "a revived input must leave the tombstone map",
    );
    // The revived handle carries the new text and is the project's only file.
    {
        let db = backend.db.lock().await;
        assert_eq!(second.text(&*db), "proc bar {} {}\n");
        let project = backend.db_project.lock().await;
        let files = project
            .expect("a project exists once a file is tracked")
            .files(&*db);
        assert!(
            files.as_slice() == [second],
            "the project file set must hold exactly the one live input",
        );
    }
}
