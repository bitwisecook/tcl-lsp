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

use tcl_registry::pack_hooks::{self, HookAnswer, HookCall, HookFamily, HookInputs, HookSlot};

fn answer(slot: HookSlot) -> HookAnswer {
    pack_hooks::dispatch(
        slot,
        &HookCall {
            words: &[],
            version: None,
            in_event_body: false,
            option: None,
            constraints: None,
            dialect: None,
            targets: &[],
            budget: tcl_registry::value_transfer::ImplementationBudget::default(),
            depends: &[],
        },
    )
}

#[test]
fn retired_table_owner_cannot_publish_after_a_new_plan() {
    let slot = pack_hooks::allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
    let retired = pack_hooks::begin_tabulation_plan();
    assert!(pack_hooks::activate_tabulation_plan(&retired));
    assert!(pack_hooks::tabulate(
        &retired,
        slot,
        vec![HookAnswer::Fold("old".into())]
    ));
    assert_eq!(answer(slot), HookAnswer::Fold("old".into()));
    let retired_copy = retired.clone();
    let (ready, waiting) = std::sync::mpsc::sync_channel(0);
    let (resume, held) = std::sync::mpsc::sync_channel(0);
    let worker = std::thread::spawn(move || {
        ready.send(()).unwrap();
        held.recv().unwrap();
        pack_hooks::tabulate(&retired, slot, vec![HookAnswer::Fold("late old".into())])
    });
    waiting.recv().unwrap();
    let current = pack_hooks::begin_tabulation_plan();
    assert!(!pack_hooks::activate_tabulation_plan(&retired_copy));
    assert_eq!(answer(slot), HookAnswer::Abstain);
    assert!(pack_hooks::tabulate(
        &current,
        slot,
        vec![HookAnswer::Fold("new".into())]
    ));
    assert_eq!(
        answer(slot),
        HookAnswer::Abstain,
        "new tables remain staged before the plan commits"
    );
    assert!(pack_hooks::activate_tabulation_plan(&current));
    resume.send(()).unwrap();
    assert!(!worker.join().unwrap());
    assert_eq!(answer(slot), HookAnswer::Fold("new".into()));
    assert!(pack_hooks::quarantine_tabulations(&current, &[slot]));
    assert_eq!(answer(slot), HookAnswer::Abstain);
    assert!(!pack_hooks::tabulate(
        &current,
        slot,
        vec![HookAnswer::Fold("late current".into())]
    ));
    let recovered = pack_hooks::begin_tabulation_plan();
    assert!(pack_hooks::tabulate(
        &recovered,
        slot,
        vec![HookAnswer::Fold("recovered".into())]
    ));
    assert!(pack_hooks::activate_tabulation_plan(&recovered));
    assert_eq!(answer(slot), HookAnswer::Fold("recovered".into()));
}
