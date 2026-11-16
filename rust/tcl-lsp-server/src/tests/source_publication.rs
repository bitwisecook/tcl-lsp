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

//! Publication of live buffers and disk files to the query database and the workspace index.

use super::*;

#[path = "source_publication/db_sources.rs"]
mod db_sources;
#[path = "source_publication/dependency_waits.rs"]
mod dependency_waits;
#[path = "source_publication/disk_files.rs"]
mod disk_files;
#[path = "source_publication/live_buffers.rs"]
mod live_buffers;
#[path = "source_publication/live_index.rs"]
mod live_index;
#[path = "source_publication/lock_bundles.rs"]
mod lock_bundles;

async fn wait_for_publication_intents_1854(backend: &Backend, expected: usize, failure: &str) {
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while backend
            .db
            .snapshots
            .publication_intent
            .load(std::sync::atomic::Ordering::Acquire)
            < expected
        {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect(failure);
}
