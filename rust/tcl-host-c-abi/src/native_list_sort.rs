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

//! Safe owner for the compiled local C qsort bridge; no Tcl object pointers.
use tcl_platform::NativeListSortFailure;

/// This target's independently compiled local C list sorting producer.
pub struct NativeListSortingEnvironment;
impl NativeListSortingEnvironment {
    /// Availability comes from the actual C bridge build, not numeric capability.
    #[must_use]
    pub const fn supported() -> bool {
        cfg!(native_list_qsort)
    }
}

fn permutation(indices: &[usize]) -> bool {
    let mut seen = vec![false; indices.len()];
    for &index in indices {
        let Some(slot) = seen.get_mut(index) else {
            return false;
        };
        if *slot {
            return false;
        }
        *slot = true;
    }
    true
}

#[cfg(native_list_qsort)]
mod native {
    use super::{NativeListSortFailure, permutation};
    use core::cmp::Ordering;
    use std::{
        any::Any,
        ffi::c_void,
        panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    };
    struct Compare<'a> {
        callback: &'a mut dyn FnMut(usize, usize) -> Result<Ordering, ()>,
        panic: Option<Box<dyn Any + Send>>,
    }
    unsafe extern "C" {
        fn tcl_native_sort_indices(
            vector: *mut usize,
            length: usize,
            frontier: *mut usize,
            context: *mut c_void,
            compare: unsafe extern "C" fn(
                *mut c_void,
                usize,
                usize,
                *mut libc::c_int,
            ) -> libc::c_int,
        ) -> libc::c_int;
    }
    unsafe extern "C" fn compare(
        context: *mut c_void,
        left: usize,
        right: usize,
        order: *mut libc::c_int,
    ) -> libc::c_int {
        // SAFETY: C retains this exclusive callback state only for the synchronous
        // call, restores its TLS frame on every normal return, and supplies live output.
        let state = unsafe { &mut *context.cast::<Compare<'_>>() };
        match catch_unwind(AssertUnwindSafe(|| (state.callback)(left, right))) {
            Ok(Ok(value)) => {
                // SAFETY: the current C comparator supplies its live `int` output.
                unsafe {
                    *order = match value {
                        Ordering::Less => -1,
                        Ordering::Equal => 0,
                        Ordering::Greater => 1,
                    };
                }
                0
            }
            Ok(Err(())) => 1,
            Err(payload) => {
                state.panic = Some(payload);
                1
            }
        }
    }
    pub(super) fn sort(
        indices: &mut [usize],
        callback: &mut dyn FnMut(usize, usize) -> Result<Ordering, ()>,
    ) -> Result<(), NativeListSortFailure> {
        let mut frontier = vec![0; indices.len()];
        let mut state = Compare {
            callback,
            panic: None,
        };
        // SAFETY: the two disjoint usize buffers are live and complete for length;
        // C's static assertions check actual pointer-sized width/alignment. The
        // opaque exclusive state cannot escape this synchronous call. All panics
        // return refusal to C, which completes normally before Rust resumes unwinding.
        let refused = unsafe {
            tcl_native_sort_indices(
                indices.as_mut_ptr(),
                indices.len(),
                frontier.as_mut_ptr(),
                (&raw mut state).cast(),
                compare,
            )
        };
        if refused != 0 {
            let valid = permutation(&frontier);
            if valid {
                indices.copy_from_slice(&frontier);
            }
            if let Some(payload) = state.panic {
                resume_unwind(payload);
            }
            return Err(if valid {
                NativeListSortFailure::ComparisonRefused
            } else {
                NativeListSortFailure::FrontierUnavailable
            });
        }
        Ok(())
    }
}
impl tcl_platform::NativeListSortingEnvironment for NativeListSortingEnvironment {
    fn sort_indices(
        &self,
        indices: &mut [usize],
        compare: &mut dyn FnMut(usize, usize) -> Result<core::cmp::Ordering, ()>,
    ) -> Result<(), NativeListSortFailure> {
        if !Self::supported() {
            return Err(NativeListSortFailure::Unavailable);
        }
        if !permutation(indices) {
            return Err(NativeListSortFailure::InvalidPermutation);
        }
        if indices.len() <= 1 {
            return Ok(());
        }
        #[cfg(native_list_qsort)]
        {
            native::sort(indices, compare)
        }
        #[cfg(not(native_list_qsort))]
        {
            let _ = compare;
            Err(NativeListSortFailure::Unavailable)
        }
    }
}

#[cfg(all(test, native_list_qsort))]
mod controls {
    use super::*;
    use core::cmp::Ordering;
    use tcl_platform::NativeListSortingEnvironment as _;

    #[test]
    fn actual_local_sort_has_no_comparator_for_zero_or_one_and_refuses_invalid_permutations() {
        // Genuine local host controls only; native645/647 ran no software callbacks.
        let host = NativeListSortingEnvironment;
        let mut calls = 0;
        for mut order in [Vec::new(), vec![0]] {
            host.sort_indices(&mut order, &mut |_, _| {
                calls += 1;
                Ok(Ordering::Equal)
            })
            .unwrap();
        }
        assert_eq!(calls, 0);
        let mut invalid = vec![0, 0];
        assert_eq!(
            host.sort_indices(&mut invalid, &mut |_, _| {
                calls += 1;
                Ok(Ordering::Equal)
            }),
            Err(NativeListSortFailure::InvalidPermutation)
        );
        assert_eq!(calls, 0);
    }

    #[test]
    fn actual_local_sort_stops_callback_effects_and_classifies_the_real_error_frontier() {
        // Real local qsort, not a claim about a foreign Jim/libc callback sequence.
        // A mid-merge vector can contain duplicates; unsupported frontier is honest.
        let host = NativeListSortingEnvironment;
        let mut calls = 0;
        let mut order: Vec<usize> = (0..64).collect();
        let result = host.sort_indices(&mut order, &mut |left, right| {
            calls += 1;
            if calls == 5 {
                Err(())
            } else {
                Ok(right.cmp(&left))
            }
        });
        assert_eq!(calls, 5);
        assert!(matches!(
            result,
            Err(NativeListSortFailure::ComparisonRefused
                | NativeListSortFailure::FrontierUnavailable)
        ));
        assert!(permutation(&order));
        // No refusal clears later invocations or leaves the previous TLS receiver.
        host.sort_indices(&mut order, &mut |left, right| Ok(left.cmp(&right)))
            .unwrap();
        assert_eq!(order, (0..64).collect::<Vec<_>>());
    }

    #[test]
    fn nested_actual_sort_and_caught_panic_restore_the_selected_callback_owner() {
        // Source-owned TLS/reentrancy and panic cleanup control, no Native chronology.
        let host = NativeListSortingEnvironment;
        let mut nested = false;
        let mut order: Vec<usize> = (0..16).collect();
        host.sort_indices(&mut order, &mut |left, right| {
            if !nested {
                nested = true;
                let mut inner = vec![2, 0, 1];
                host.sort_indices(&mut inner, &mut |a, b| Ok(a.cmp(&b)))
                    .unwrap();
                assert_eq!(inner, vec![0, 1, 2]);
            }
            Ok(right.cmp(&left))
        })
        .unwrap();
        assert!(nested);
        assert_eq!(order, (0..16).rev().collect::<Vec<_>>());
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            host.sort_indices(&mut order, &mut |_, _| panic!("actual comparator panic"))
        }));
        assert!(panic.is_err());
        assert!(permutation(&order));
        host.sort_indices(&mut order, &mut |a, b| Ok(a.cmp(&b)))
            .unwrap();
        assert_eq!(order, (0..16).collect::<Vec<_>>());
    }
}
