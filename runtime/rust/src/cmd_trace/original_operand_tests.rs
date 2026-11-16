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

#[cfg(debug_assertions)]
use super::*;
#[cfg(debug_assertions)]
use crate::cmd_alias::original_operand_tests::{
    first, native, seed, unchanged, words, Update, UpdaterOwner,
};
#[cfg(debug_assertions)]
use crate::obj::Owned;
#[cfg(debug_assertions)]
#[test]
fn original_variable_trace_queries_preserve_cells_and_first_host() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for legacy in [false, true] {
        for mode in [Update::Host, Update::Restore] {
            let mut interp = native("tcl8.6");
            assert_eq!(
                interp.eval_str(b"trace add variable v write callback"),
                Code::Ok
            );
            let registrations: Vec<_> =
                interp.traces.borrow().traces.iter().map(|t| t.id).collect();
            assert!(!registrations.is_empty());
            let _updater = UpdaterOwner::enter(&interp, mode);
            let failed = first();
            let owned = words(if legacy {
                &[b"trace", b"vinfo", b"name"]
            } else {
                &[b"trace", b"info", b"variable", b"name"]
            });
            let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
            *argv.last_mut().unwrap() = failed.as_ptr();
            let prior = seed(&mut interp);
            assert_eq!(super::trace_cmd(&mut interp, &argv), Code::Error);
            unchanged(&interp, &prior, mode);
            assert_eq!(
                interp
                    .traces
                    .borrow()
                    .traces
                    .iter()
                    .map(|t| t.id)
                    .collect::<Vec<_>>(),
                registrations
            );
        }
    }
}
