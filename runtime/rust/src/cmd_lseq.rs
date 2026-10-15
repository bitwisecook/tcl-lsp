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

//! `lseq` (Tcl 8.7/9.0) — arithmetic-sequence list generator.
//!
//! ```text
//! lseq start ?(..|to)? end ??by? step?
//! lseq start count count ??by? step?
//! lseq count ?by step?
//! ```
//!
//! A thin adapter over [`tcl_cmd_core::lseq`] — the shared core runs the
//! argument-decode key (`..`/`to`/`count`/`by`), the reads C makes of each
//! value, the int-vs-double selection, and the precision-matched generation;
//! this adapter only builds the elements over the bignum runtime's `ValueOps`.
//! The module is gated on `have_tommath` with the other numeric commands.
//!
//! Semantics match tclsh 9.0.
//!
#![cfg(have_tommath)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;
use tcl_cmd_core::lseq;

/// Register `lseq`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"lseq", lseq);
}

fn lseq(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let args: Vec<Vec<u8>> = argv[1..].iter().map(|&a| obj_bytes(a)).collect();
    let refs: Vec<&[u8]> = args.iter().map(Vec::as_slice).collect();
    let generated = lseq::decode(&refs).and_then(|plan| lseq::generate(interp, &plan));
    match generated {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.report_cmd_error(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    /// Evaluate `src` leak-checked, asserting success, and return the result.
    fn ok(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(counters::double_free_count(), 0);
        assert_eq!(
            code,
            Code::Ok,
            "result={:?}",
            String::from_utf8_lossy(&bytes)
        );
        bytes
    }

    fn err(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(code, Code::Error);
        bytes
    }

    #[test]
    fn lseq_integer_forms() {
        assert_eq!(ok(b"lseq 5"), b"0 1 2 3 4");
        assert_eq!(ok(b"lseq 0"), b"");
        assert_eq!(ok(b"lseq -5"), b""); // negative count → empty
        assert_eq!(ok(b"lseq 1 .. 10"), b"1 2 3 4 5 6 7 8 9 10");
        assert_eq!(ok(b"lseq 10 .. 1"), b"10 9 8 7 6 5 4 3 2 1");
        assert_eq!(ok(b"lseq 1 to 10 by 2"), b"1 3 5 7 9");
        assert_eq!(ok(b"lseq 1 to 10 by -2"), b""); // wrong-sign step → empty
        assert_eq!(ok(b"lseq 1 to 5 by 0"), b"1"); // zero step → one element
        assert_eq!(ok(b"lseq 5 count 5"), b"5 6 7 8 9");
        assert_eq!(ok(b"lseq 5 count 5 by -2"), b"5 3 1 -1 -3");
        assert_eq!(ok(b"lseq 3 by 2"), b"0 2 4");
        assert_eq!(ok(b"lseq 1 5"), b"1 2 3 4 5");
        assert_eq!(ok(b"lseq 5 1"), b"5 4 3 2 1");
    }

    #[test]
    fn lseq_double_precision() {
        // Precision matches the inputs' fractional digits (`maxObjPrecision`).
        assert_eq!(ok(b"lseq 0 0.5 by 0.1"), b"0.0 0.1 0.2 0.3 0.4 0.5");
        assert_eq!(ok(b"lseq 25. to 5. by -5"), b"25.0 20.0 15.0 10.0 5.0");
        assert_eq!(
            ok(b"lseq 3.5 18.5 1.5"),
            b"3.5 5.0 6.5 8.0 9.5 11.0 12.5 14.0 15.5 17.0 18.5"
        );
        // A double-valued count is used as an integer and stays an int sequence.
        assert_eq!(ok(b"lseq 5 count 5.0"), b"5 6 7 8 9");
    }

    #[test]
    fn lseq_words_are_numbers_not_expressions() {
        // tclsh 9.0.4/9.1.0 evaluate no expression in an `lseq` word.
        assert_eq!(err(b"lseq 1+2 to 10"), b"expected number but got \"1+2\"");
        assert_eq!(
            err(b"set n 3; lseq $n*2"),
            b"expected number but got \"3*2\""
        );
    }

    #[test]
    fn lseq_errors() {
        assert_eq!(
            err(b"lseq"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
        assert_eq!(
            err(b"lseq 12 to 24 by 2 count"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
        // Huge series are capped rather than OOM-aborting.
        assert_eq!(
            err(b"lseq 10 2147483647"),
            b"max length of a Tcl list exceeded"
        );
    }
}
