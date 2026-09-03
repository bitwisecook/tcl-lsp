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

//! `expr rand()` / `srand(n)` — the one **shared owner** of Tcl's Park–Miller
//! minimal-standard generator (C's `ExprRandFunc` / `ExprSrandFunc`,
//! `tclBasic.c`).
//!
//! `mathfunc::dispatch` deliberately excludes `rand`/`srand` because they read
//! and write interpreter state, and for a long time that meant each engine
//! transcribed C's generator independently. The transcriptions drifted on the
//! final scaling step: C multiplies by the *reciprocal*
//! (`(double)randSeed * (1.0/RAND_IM)`) and `1.0/2147483647` is not exactly
//! representable, so a true divide `seed / IM` differs by one ulp for a dense
//! family of seeds — visible in the result string, because Tcl formats a
//! double with the shortest round-tripping decimal (`srand(251)` is
//! `0.001964418684115828` in C and `0.0019644186841158285` under a true
//! divide; within one seeded stream from `srand(1)` the 145th draw already
//! differs). Issue #1432.
//!
//! Everything here is pure: the *step*, the *seed nudge* and the *scaling*.
//! What stays per-engine is only the seed **storage** and the nondeterministic
//! first-seed policy for a stream that was never `srand`'d.

/// C's `RAND_IM` — the generator's modulus, `2^31 - 1`. The recurrence maps a
/// seed in `[1, IM - 1]` to another seed in that range.
pub const RAND_IM: i64 = 2_147_483_647;

/// C's `RAND_IA` (the multiplier), `RAND_IQ`/`RAND_IR` (Schrage's
/// overflow-avoiding factorisation, `IM = IA*IQ + IR`), and `RAND_MASK` (the
/// value XOR-ed into a seed that landed on one of the two fixed points).
const RAND_IA: i64 = 16807;
const RAND_IQ: i64 = 127_773;
const RAND_IR: i64 = 2836;
const RAND_MASK: i64 = 0x075b_d924; // C's decimal 123459876

/// The seed C installs for `srand(w)` — the low 31 bits of the wide, nudged
/// off the generator's two fixed points (`0` maps to itself, `IM` maps to `0`),
/// so the result is always in `[1, IM - 1]`.
///
/// The operand is C's `TclGetWideBitsFromObj` result, i.e. the **low 64 bits**
/// of an integer of any width — `srand(2**64 + 7)` seeds as `7`. Turning the
/// operand into that wide is the caller's job (it is an object-level
/// conversion, and C refuses a non-integer operand there).
#[must_use]
pub fn seed_from_wide(w: i64) -> i64 {
    let s = w & 0x7FFF_FFFF;
    if s == 0 || s == RAND_IM {
        s ^ RAND_MASK
    } else {
        s
    }
}

/// One step of the recurrence `seed = (IA * seed) mod IM`, in Schrage's form
/// so no intermediate exceeds a 32-bit signed range (C computes it in `long`).
#[must_use]
pub fn step(seed: i64) -> i64 {
    let tmp = seed / RAND_IQ;
    let next = RAND_IA * (seed - tmp * RAND_IQ) - RAND_IR * tmp;
    if next < 0 { next + RAND_IM } else { next }
}

/// The seed as C scales it into `(0, 1)`: a **reciprocal multiply**, not a
/// divide. `1.0/RAND_IM` is not exactly representable, so the two spellings
/// disagree by one ulp on a large fraction of seeds and Tcl's shortest
/// round-trip formatting makes that difference visible (#1432) — this is the
/// spelling the oracle produces.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "the seed is in [1, 2^31-2], far inside f64's exact-integer range"
)]
pub fn scale(seed: i64) -> f64 {
    seed as f64 * (1.0 / RAND_IM as f64)
}

/// Advance `seed` in place and return the draw — the whole of C's
/// `ExprRandFunc` body once the seed exists.
pub fn next_draw(seed: &mut i64) -> f64 {
    *seed = step(*seed);
    scale(*seed)
}

/// `srand(w)`: install the seed, then return the stream's first draw (C
/// tail-calls `ExprRandFunc`).
pub fn seed_and_draw(seed: &mut i64, w: i64) -> f64 {
    *seed = seed_from_wide(w);
    next_draw(seed)
}

#[cfg(test)]
mod tests {
    use super::{RAND_IM, next_draw, seed_and_draw, seed_from_wide};
    use crate::number::format_double;

    /// The seed nudge: `0` and `IM` are the recurrence's fixed points and are
    /// masked away; every other value keeps its low 31 bits. (tclsh
    /// 8.6.16/9.0.4: `srand(0)`, `srand(2147483647)` and `srand(-1)` all
    /// produce a live stream, and the last two produce the *same* one because
    /// `-1 & 0x7FFFFFFF` is `IM`.)
    #[test]
    fn the_seed_nudge_avoids_the_two_fixed_points() {
        assert_eq!(seed_from_wide(0), 0x075b_d924);
        assert_eq!(seed_from_wide(RAND_IM), RAND_IM ^ 0x075b_d924);
        assert_eq!(seed_from_wide(-1), seed_from_wide(RAND_IM));
        assert_eq!(seed_from_wide(251), 251);
        // C reads the operand's low 64 bits, then its low 31: `2**64 + 7`
        // seeds as 7.
        assert_eq!(seed_from_wide(7), 7);
    }

    /// The scaling step, pinned to the oracle. `srand(251)` is the smallest
    /// member of the dense family where C's reciprocal multiply and a true
    /// divide differ by one ulp; a true divide gives
    /// `0.0019644186841158285` here (tclsh 8.6.16/9.0.4 print the value
    /// below).
    #[test]
    fn srand_251_is_the_reciprocal_multiply_value() {
        let mut seed = 0;
        let draw = seed_and_draw(&mut seed, 251);
        assert_eq!(format_double(draw), "0.001964418684115828");
        assert_ne!(format_double(draw), "0.0019644186841158285");
    }

    /// A whole seeded stream, pinned to the oracle: `srand(1)`'s first draw
    /// and the 145th draw of that stream — the first index at which the two
    /// scalings disagree, so this row is the drift gate for the family.
    #[test]
    fn the_srand_1_stream_matches_the_oracle_at_draw_1_and_145() {
        let mut seed = 0;
        let first = seed_and_draw(&mut seed, 1);
        assert_eq!(format_double(first), "7.826369259425611e-6");
        let mut draw = first;
        for _ in 2..=145 {
            draw = next_draw(&mut seed);
        }
        assert_eq!(format_double(draw), "0.9833050970841688");
    }

    /// The other seeds the issue names, all read off tclsh 8.6.16/9.0.4.
    #[test]
    fn the_documented_seeds_match_the_oracle() {
        for (w, want) in [
            (0_i64, "0.24257829889775176"),
            (RAND_IM, "0.7574217011022483"),
            (-1, "0.7574217011022483"),
            // `srand(2**64 + 7)`: the low 64 bits of the operand are 7.
            (7, "5.4784584815979276e-5"),
        ] {
            let mut seed = 0;
            assert_eq!(
                format_double(seed_and_draw(&mut seed, w)),
                want,
                "srand({w})"
            );
        }
    }
}
