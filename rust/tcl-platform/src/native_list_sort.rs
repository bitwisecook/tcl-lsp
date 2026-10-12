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

//! Independently supplied local C sorting producer. Index storage has no Tcl roles.
use core::cmp::Ordering;

/// A sorting capability refuses before inventing a producer or error frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeListSortFailure {
    /// This target does not supply the selected local C sorting ABI.
    Unavailable,
    /// Input is not a complete permutation of `0..len`.
    InvalidPermutation,
    /// The first refused comparison's actual permutation was restored.
    ComparisonRefused,
    /// The actual producer had an incomplete intermediate vector when refused.
    /// No faithful error frontier can be exported from that producer state.
    FrontierUnavailable,
}

/// Actual local `qsort` over pointer-sized index storage. This capability is
/// independent of numeric conversion, String getters and interpreter currency.
/// It grants no original Tcl object references, result roles or foreign libc.
///
/// A failed comparator stops all later comparator effects. The C producer still
/// completes with an inert comparator so its own scratch is retired normally.
/// A captured first-error vector is restored only if it remains a full original
/// permutation; otherwise the error frontier is explicitly unavailable.
pub trait NativeListSortingEnvironment {
    /// Sort a complete `0..len` permutation using this target's real C `qsort`.
    /// Zero/one elements invoke no comparator.
    ///
    /// # Errors
    /// Unsupported ABI, invalid input, or the precisely classified first refusal.
    fn sort_indices(
        &self,
        indices: &mut [usize],
        compare: &mut dyn FnMut(usize, usize) -> Result<Ordering, ()>,
    ) -> Result<(), NativeListSortFailure>;
}
