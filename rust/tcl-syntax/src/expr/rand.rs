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

//! `expr rand()` / `srand()` — the **single** shared transcription of C's
//! Park–Miller generator.
//!
//! [`super::mathfunc::dispatch`] deliberately excludes `rand`/`srand`: they
//! carry per-interpreter PRNG state, so the *storage* and the
//! non-deterministic first-seed policy stay with each engine. Everything that
//! is pure — the recurrence, the seed normalisation, the final scaling, and
//! the release-dependent surface of a rejected operand — lives here, so the
//! engines cannot drift the way they did in issue #1432 (a true divide instead
//! of C's reciprocal-multiply made `expr {srand(251)}` differ in the last
//! digit between the two backends).
//!
//! Source: `ExprRandFunc` / `ExprSrandFunc`, `tcl9.0.4/generic/tclBasic.c:7789`
//! and `:7961` (8.6.16: `tclBasic.c:7793` / `:7965`).

/// The multiplier `IA` of the recurrence `seed = (IA * seed) mod IM`.
pub const RAND_IA: i64 = 16807;
/// The modulus `IM` — `2^31 - 1`, the Park–Miller prime.
pub const RAND_IM: i64 = 2_147_483_647;
/// `IQ`, from Schrage's overflow-avoiding factorisation `IM = IA*IQ + IR`.
pub const RAND_IQ: i64 = 127_773;
/// `IR`, from Schrage's factorisation.
pub const RAND_IR: i64 = 2_836;
/// The nudge applied to a seed that landed on one of the generator's two
/// fixed points (`0` and `IM`).
pub const RAND_MASK: i64 = 123_459_876;

/// Normalise a wide seed the way `ExprSrandFunc` does
/// (`tcl9.0.4/generic/tclBasic.c:7991-7995`): keep the low 31 bits, then step
/// off the generator's two fixed points, which would otherwise make every
/// draw identical.
#[must_use]
pub fn seed_from_wide(seed: i64) -> i64 {
    let normalised = seed & 0x7fff_ffff;
    if normalised == 0 || normalised == RAND_IM {
        normalised ^ RAND_MASK
    } else {
        normalised
    }
}

/// Advance the generator one step, returning `(next_seed, draw)`.
///
/// The draw is C's `(double)randSeed * (1.0/RAND_IM)`
/// (`tcl9.0.4/generic/tclBasic.c:7870`) — a **reciprocal-multiply**, not a
/// true divide. `1.0/2147483647` is not exactly representable, so the two
/// forms differ by one ulp for a large fraction of seeds, and Tcl's
/// shortest-round-trip float formatting makes that ulp visible: seed 251
/// prints as `0.001964418684115828` here and `0.0019644186841158285` under a
/// divide.
// The seed stays in `[1, RAND_IM - 1]` and `RAND_IM` is `2^31 - 1`, both far
// inside `f64`'s exact-integer range, so the cast is lossless.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn next(seed: i64) -> (i64, f64) {
    let tmp = seed / RAND_IQ;
    let mut next_seed = RAND_IA * (seed - tmp * RAND_IQ) - RAND_IR * tmp;
    if next_seed < 0 {
        next_seed += RAND_IM;
    }
    (next_seed, next_seed as f64 * (1.0 / RAND_IM as f64))
}

/// The error `srand` raises for an operand that is not an integer.
///
/// Both reference releases reject a float — `srand(1.5)` is an error, never a
/// truncation — but they report it differently, so the surface is release
/// parameterised:
///
/// * up to 8.6, `ExprSrandFunc` falls back to `Tcl_GetBignumFromObj` with a
///   **real** interpreter (`tcl8.6.16/generic/tclBasic.c:7986`), producing
///   `expected integer but got "1.5"`;
/// * from 9.0 it reads the operand with `TclGetWideBitsFromObj(NULL, …)`
///   (`tcl9.0.4/generic/tclBasic.c:7961`) — a **NULL** interpreter, so the
///   message and `-errorcode` are simply never set. `expr {srand(1.5)}` on
///   tclsh 9.0.4 fails with an empty message and `errorCode` `NONE`. That is
///   an upstream quirk, reproduced here deliberately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrandOperandError {
    /// The error message (empty from Tcl 9.0).
    pub message: String,
    /// The `-errorcode`, or empty when the release sets none.
    pub error_code: String,
}

/// Build [`SrandOperandError`] for `operand` under `release`.
///
/// `operand_is_number` distinguishes the two 8.x codes: a well-formed
/// non-integer number (`1.5`) is `TCL VALUE INTEGER`, while a value that is no
/// number at all (`tru`) is `TCL VALUE NUMBER` — both verified on tclsh
/// 8.6.16.
#[must_use]
pub fn srand_operand_error(
    operand: &str,
    operand_is_number: bool,
    release: tcl_dialect::TclVersion,
) -> SrandOperandError {
    if release >= tcl_dialect::TclVersion::V9_0 {
        return SrandOperandError {
            message: String::new(),
            error_code: String::new(),
        };
    }
    SrandOperandError {
        message: format!("expected integer but got \"{operand}\""),
        error_code: if operand_is_number {
            "TCL VALUE INTEGER".to_owned()
        } else {
            "TCL VALUE NUMBER".to_owned()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scaling is C's reciprocal-multiply. Every row is a tclsh 8.6.16 /
    /// 9.0.4 oracle value for `expr {srand(N)}` — the dense family from issue
    /// #1432 where a true divide differs in the last digit.
    #[test]
    fn srand_first_draw_matches_the_oracle() {
        for (seed, want) in [
            (251_i64, "0.001964418684115828"),
            (255, "0.0019957241611535306"),
            (259, "0.002027029638191233"),
            (263, "0.0020583351152289354"),
            (267, "0.002089640592266638"),
            (271, "0.0021209460693043403"),
            (1, "7.826369259425611e-6"),
            (2, "1.5652738518851222e-5"),
            (3, "2.3479107778276833e-5"),
            (42, "0.00032870750889587566"),
            // Both fixed points take the nudge before the first step.
            (0, "0.24257829889775176"),
            (2_147_483_647, "0.7574217011022483"),
            // A negative seed keeps only its low 31 bits.
            (-5, "0.9999686945229623"),
            // A beyond-wide seed folds to its low bits — `2^64 + 1` is seed 1.
            (1, "7.826369259425611e-6"),
        ] {
            let (_, draw) = next(seed_from_wide(seed));
            assert_eq!(
                crate::number::format_double(draw),
                want,
                "srand({seed}) first draw"
            );
        }
    }

    /// A seeded stream reproduces the oracle draw for draw — the 145th draw
    /// from `srand(1)` is where a true divide first diverges within one
    /// stream, so a short prefix would not catch a scaling regression.
    #[test]
    fn seeded_stream_matches_the_oracle() {
        let mut seed = seed_from_wide(1);
        let mut draw = 0.0;
        for _ in 0..4 {
            let (next_seed, value) = next(seed);
            seed = next_seed;
            draw = value;
        }
        // tclsh: `expr {srand(1)}` then three more `rand()` calls.
        assert_eq!(crate::number::format_double(draw), "0.4586501319234493");
    }

    /// The recurrence never leaves `[1, IM - 1]`, so no draw is 0.0 or 1.0.
    #[test]
    fn recurrence_stays_in_range() {
        let mut seed = seed_from_wide(1);
        for _ in 0..10_000 {
            let (next_seed, draw) = next(seed);
            assert!((1..RAND_IM).contains(&next_seed), "seed {next_seed}");
            assert!(draw > 0.0 && draw < 1.0, "draw {draw}");
            seed = next_seed;
        }
    }

    #[test]
    fn srand_operand_error_is_release_shaped() {
        use tcl_dialect::TclVersion;
        let old = srand_operand_error("1.5", true, TclVersion::V8_6);
        assert_eq!(old.message, "expected integer but got \"1.5\"");
        assert_eq!(old.error_code, "TCL VALUE INTEGER");
        let word = srand_operand_error("tru", false, TclVersion::V8_6);
        assert_eq!(word.error_code, "TCL VALUE NUMBER");
        for release in [TclVersion::V9_0, TclVersion::V9_1] {
            let new = srand_operand_error("1.5", true, release);
            assert!(new.message.is_empty(), "9.x message is never set");
            assert!(new.error_code.is_empty());
        }
    }
}
