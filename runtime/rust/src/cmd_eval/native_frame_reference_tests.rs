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

//! Original numeric frame caches retain no selected frame incarnation.

use super::{Interp, OriginalLevel};
use crate::obj::{self, Owned};
use tcl_registry::frame_effect::NativeFrameLevelObject;

fn actual(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn push_frame(interp: &mut Interp) {
    let namespace = interp.current_ns();
    interp.enter_namespace_activation(namespace);
    interp.frames.borrow_mut().push(namespace);
}

fn pop_frame(interp: &mut Interp) {
    let namespace = interp.pop_native_call_frame();
    interp.leave_namespace_activation(namespace);
}

// Native proof: naming.variable.absolute-level-cache-reselects-current-frame
// docs/design/analysis/name-resolution-proofs/variable.absolute-level-cache-reselects-current-frame.md
#[test]
fn level_reference_reselects_current_frames_in_thirty_native_windows() {
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.4.20.tsv"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.5.19.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../rust/tcl-syntax/tests/data/native_frame_reference/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!("../../../../rust/tcl-syntax/tests/data/native_frame_reference/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../rust/tcl-syntax/tests/data/native_frame_reference/9.1.0.tsv"),
        ),
    ] {
        let mut interp = actual(engine);
        let mut other = actual(engine);
        let original = Owned::fresh(obj::new_string_bytes(b"#1"));
        let mut records = rows.lines();
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        pop_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut interp);
        compare(&mut interp, &original, records.next().unwrap());
        push_frame(&mut other);
        compare(&mut other, &original, records.next().unwrap());
        let duplicate = Owned::fresh(obj::duplicate(original.as_ptr()));
        drop(original);
        compare(&mut other, &duplicate, records.next().unwrap());
        assert!(records.next().is_none());
        pop_frame(&mut other);
        pop_frame(&mut interp);
        compared += 6;
    }
    assert_eq!(compared, 30);
}

fn compare(interp: &mut Interp, value: &Owned, record: &str) {
    let fields: Vec<_> = record.split('\t').collect();
    let dialect = interp.eval_frame_dialect();
    let mut original = OriginalLevel {
        value: value.as_ptr(),
        dialect,
    };
    let result = dialect
        .native_frame_level_protocol()
        .unwrap()
        .resolve_object(interp.current_level(), &mut original)
        .unwrap();
    assert_eq!(
        result.as_ref().map_or(-1, |_| 1),
        fields[1].parse::<i32>().unwrap()
    );
    assert_eq!(
        result
            .as_ref()
            .map_or(-1, |selected| i32::try_from(selected.target).unwrap()),
        fields[2].parse::<i32>().unwrap()
    );
    let snapshot = obj::native_object_snapshot(value.as_ptr()).unwrap();
    assert_eq!(snapshot.resident.is_some(), fields[4] == "1");
    if fields[3] == "levelReference" {
        assert!(matches!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::FrameReference {
                relative: false,
                level: 1,
                ..
            }
        ));
        assert_eq!(
            original.native_frame_cache().unwrap(),
            Some(tcl_registry::NativeFrameLevelCache::Absolute(1))
        );
    } else {
        assert_eq!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::None
        );
    }
    // SAFETY: the owning original remains live throughout this header observation.
    unsafe {
        assert_eq!(
            (*value.as_ptr()).ref_count,
            fields[5].parse::<isize>().unwrap()
        );
        let descriptor = (*value.as_ptr()).type_ptr;
        let hooks = if descriptor.is_null() {
            [false; 4]
        } else {
            [
                (*descriptor).free_int_rep_proc.is_some(),
                (*descriptor).dup_int_rep_proc.is_some(),
                (*descriptor).update_string_proc.is_some(),
                (*descriptor).set_from_any_proc.is_some(),
            ]
        };
        for (actual, expected) in hooks.iter().zip(&fields[6..]) {
            assert_eq!(*actual, *expected == "1");
        }
    }
}

#[test]
fn frame_reference_cache_refuses_foreign_issuers_and_retired_headers() {
    let interp = actual("tcl8.5");
    let original = Owned::fresh(obj::new_string_bytes(b"#1"));
    let dialect = interp.eval_frame_dialect();
    obj::install_native_frame_level_cache(
        original.as_ptr(),
        tcl_registry::NativeFrameLevelCache::Absolute(1),
        dialect,
    )
    .unwrap();
    let foreign = actual("tcl8.6");
    assert!(
        foreign
            .native_object_string_bytes(original.as_ptr())
            .is_err()
    );
    let lifetime = obj::ProcedureObject::retain_lifetime(&original);
    drop(original);
    assert!(obj::native_frame_level_cache_in(lifetime.as_ptr(), dialect).is_err());
    assert!(
        obj::install_native_frame_level_cache(
            lifetime.as_ptr(),
            tcl_registry::NativeFrameLevelCache::Absolute(1),
            dialect
        )
        .is_err()
    );
    // The normal live interpreter's unrelated objects retain their own recipe.
    let plain = Owned::fresh(obj::new_string_bytes(b"plain"));
    assert_eq!(
        interp
            .native_object_string_bytes(plain.as_ptr())
            .unwrap()
            .as_ref(),
        b"plain"
    );
}

#[test]
fn required_upvar_level_does_not_borrow_optional_leading_frame_default() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // Case11's unchanged original C8.6/9.0/9.1 native tables require bad level k.
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = actual(engine);
        push_frame(&mut interp);
        let original = Owned::fresh(obj::new_string_bytes(b"k"));
        let dialect = interp.eval_frame_dialect();
        let protocol = dialect.native_frame_level_protocol().unwrap();
        let mut operand = OriginalLevel {
            value: original.as_ptr(),
            dialect,
        };
        let optional = protocol
            .resolve_object(interp.current_level(), &mut operand)
            .unwrap()
            .unwrap();
        assert!(!optional.explicit);
        assert_eq!(optional.target, 0);
        for current in [0, interp.current_level()] {
            let result = protocol
                .resolve_required_object(current, &mut operand)
                .unwrap();
            assert!(matches!(result,
                Err(tcl_registry::frame_effect::NativeFrameLevelFailure::BadLevel { name, lookup_code: true, .. })
                    if name == b"k"
            ));
        }
        let x = Owned::fresh(obj::new_string_bytes(b"x"));
        let alias = Owned::fresh(obj::new_string_bytes(b"alias"));
        assert!(
            super::select_frame(
                &mut interp,
                &[original.as_ptr(), x.as_ptr(), alias.as_ptr()],
                tcl_registry::FrameEffectSpec::UPVAR,
            )
            .is_err()
        );
        assert!(matches!(
            obj::native_object_snapshot(interp.result_obj()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { protocol: producer, .. }
                if Some(producer) == protocol.bad_level_string_result()
        ));
        assert!(obj::has_string_rep(interp.result_obj()));
        assert_eq!(interp.result_bytes(), b"bad level \"k\"");
        assert!(!interp.host_refusal_pending());
        pop_frame(&mut interp);
    }
}
