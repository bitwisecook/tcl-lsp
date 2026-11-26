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

//! Explicit original-level evaluation, distinct from optional command parsing.

use crate::{Value, Vm};

// Native proof: naming.variable.absolute-level-cache-reselects-current-frame
// docs/design/analysis/name-resolution-proofs/variable.absolute-level-cache-reselects-current-frame.md
#[test]
fn level_reference_reselects_current_frames_in_thirty_native_windows() {
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/9.1.0.tsv"),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let mut other = crate::native_fixture::core(profile);
        let original = Value::new_native_string_bytes(b"#1".as_slice());
        let mut records = rows.lines();
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.push_call_frame(None, vec![]);
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.pop_call_frame();
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.push_call_frame(None, vec![]);
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        other.push_call_frame(None, vec![]);
        compare_level_reference(&mut other, &original, records.next().unwrap());
        let duplicate =
            original.duplicate_native_object_in(vm.actual_native_string_protocol().unwrap());
        drop(original);
        compare_level_reference(&mut other, &duplicate, records.next().unwrap());
        assert!(records.next().is_none());
        other.pop_call_frame();
        vm.pop_call_frame();
        compared += 6;
    }
    assert_eq!(compared, 30);
}

fn compare_level_reference(vm: &mut Vm, level: &Value, record: &str) {
    let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(record).collect();
    let target = crate::command::runtime_explicit_frame_selection(vm, level);
    assert_eq!(
        target.as_ref().map_or(-1, |_| 1),
        fields[1].parse::<i32>().unwrap(),
        "{}",
        fields[0]
    );
    assert_eq!(
        target
            .as_ref()
            .map_or(-1, |level| i32::try_from(*level).unwrap()),
        fields[2].parse::<i32>().unwrap(),
        "{}",
        fields[0]
    );
    assert_eq!(level.native_object_type_name(), fields[3], "{}", fields[0]);
    assert_eq!(level.resident_string_bytes().is_some(), fields[4] == "1");
    assert_eq!(
        level.native_object_reference_count(),
        fields[5].parse::<usize>().unwrap()
    );
    assert!(!level.native_primary_has_free_hook());
    if fields[3] == "levelReference" {
        assert_eq!(
            level
                .native_frame_level_cache_in(vm.actual_native_invocation_dialect())
                .unwrap(),
            Some(tcl_registry::NativeFrameLevelCache::Absolute(1))
        );
    }
}
