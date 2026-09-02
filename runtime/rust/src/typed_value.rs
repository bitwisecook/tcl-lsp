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

//! **The typed-scalar read seam** — one owner for "read this `TclObj` as an
//! integer / double / boolean", with C Tcl's exact message and error code.
//!
//! This is the runtime half of the `Tcl_Get{WideInt,Double,Boolean}FromObj`
//! family. Three properties define it, and every consumer gets all three by
//! coming through here rather than re-deriving one:
//!
//! 1. **Write-back.** A successful read caches the parsed internal rep onto the
//!    object ([`crate::obj::may_cache_parsed_rep`] owns the policy, which is C's:
//!    convert in place, keep the string rep). A loop reading the same variable
//!    parses its spelling once, not once per iteration.
//! 2. **The boolean words come from one table.** Boolean acceptance is
//!    [`tcl_syntax::boolean`]'s — the shared owner named in the semantic owner
//!    map — so `tru`, `ye`, `of` and the ambiguous `o` behave identically
//!    everywhere in the runtime instead of in as many private `match` arms as
//!    there are call sites (issue #1425's runtime half).
//! 3. **The failure text is C's**, including the `a list` rendering
//!    ([`tcl_syntax::list::describe_bad_value`], `tclStrToD.c`'s `formaterr`)
//!    and the `-errorcode` the same site sets.
//!
//! `boolean` implements the **boolean-context** acceptor
//! ([`tcl_syntax::boolean::truthiness`]): a boolean word, else any number
//! compared against zero. That is `Tcl_GetBooleanFromObj`, the acceptor behind
//! `if`, `while`, and `expr`'s `?:`/`&&`/`||`/`!` — not the stricter
//! `string is boolean` one.

use crate::obj::{self, TclObj};

/// A failed typed read: C's interpreter result and its `-errorcode` list text.
pub(crate) struct TypedError {
    /// The message C leaves as the interpreter result.
    pub message: Vec<u8>,
    /// The `-errorcode` list C sets alongside it.
    pub code: &'static [u8],
}

impl TypedError {
    /// `expected <what> but got <value>` + `TCL VALUE NUMBER` — the error
    /// `TclParseNumber`'s `formaterr` raises for every unparsable spelling.
    fn expected(what: &str, obj: *mut TclObj) -> Self {
        let bytes = obj::bytes_of(obj);
        let text = String::from_utf8_lossy(&bytes);
        let mut message = format!("expected {what} but got ").into_bytes();
        message.extend_from_slice(tcl_syntax::list::describe_bad_value(&text).as_bytes());
        TypedError {
            message,
            code: b"TCL VALUE NUMBER",
        }
    }
}

/// Read `obj` as a Tcl wide integer — `Tcl_GetWideIntFromObj`.
///
/// A double-typed object is refused under C's own wording and `TCL VALUE
/// INTEGER` code (`tclObj.c`); an integer past the wide range is C's
/// `ARITH IOVERFLOW`; anything else unparsable is `TclParseNumber`'s
/// `TCL VALUE NUMBER`.
pub(crate) fn wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    if obj::obj_type_ptr(obj) == &obj::TCL_DOUBLE_TYPE {
        let bytes = obj::bytes_of(obj);
        let mut message = b"expected integer but got \"".to_vec();
        message.extend_from_slice(&bytes);
        message.push(b'"');
        return Err(TypedError {
            message,
            code: b"TCL VALUE INTEGER",
        });
    }
    read_wide_int(obj)
}

/// Read `obj` as a Tcl double — `Tcl_GetDoubleFromObj`. An integer or bignum
/// widens; `NaN` is a value here (the boolean context is where it is an error).
pub(crate) fn double(obj: *mut TclObj) -> Result<f64, TypedError> {
    read_double(obj).ok_or_else(|| TypedError::expected("floating-point number", obj))
}

/// Read `obj` as a Tcl boolean in **boolean context** —
/// `Tcl_GetBooleanFromObj`, the acceptor `if`/`while`/`expr`'s logical
/// operators use.
///
/// A boolean word (or unique prefix: `tru`, `ye`, `of`; the ambiguous `o` is
/// refused) resolves to its value; otherwise any number is compared against
/// zero, with the numeric read taking the typed rep when the object already has
/// one and caching one when it does not. `NaN` is C's domain error, not a
/// truthy value.
pub(crate) fn boolean(obj: *mut TclObj) -> Result<bool, TypedError> {
    let bytes = obj::bytes_of(obj);
    let text = String::from_utf8_lossy(&bytes);
    // The words first: they are release-invariant and never numeric, so this
    // never disturbs an object's rep.
    if let Some(value) = tcl_syntax::boolean::parse_boolean_word(text.trim()) {
        return Ok(value);
    }
    match read_double(obj) {
        Some(value) if value.is_nan() => Err(TypedError {
            message: b"floating point value is Not a Number".to_vec(),
            code: b"TCL VALUE DOUBLE NAN",
        }),
        // Comparing the double widening against zero is exact for every value
        // the tower produces: a bignum is non-zero by construction, and no
        // integer rounds to zero under widening.
        Some(value) => Ok(value != 0.0),
        None => Err(TypedError::expected("boolean value", obj)),
    }
}

/// [`boolean`] over a raw spelling, for a caller that holds bytes rather than an
/// object — the same shared acceptor
/// ([`tcl_syntax::boolean::truthiness`]: a boolean word, else any number
/// against zero), minus the write-back there is no object to perform.
pub(crate) fn boolean_bytes(bytes: &[u8]) -> Result<bool, TypedError> {
    let text = String::from_utf8_lossy(bytes);
    tcl_syntax::boolean::truthiness(text.trim()).ok_or_else(|| {
        let mut message = b"expected boolean value but got ".to_vec();
        message.extend_from_slice(tcl_syntax::list::describe_bad_value(&text).as_bytes());
        TypedError {
            message,
            code: b"TCL VALUE NUMBER",
        }
    })
}

/// The wide-integer classification over the numeric tower.
#[cfg(have_tommath)]
fn read_wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    use crate::bignum::WideRead;
    match crate::bignum::read_wide(obj) {
        WideRead::Wide(value) => Ok(value),
        WideRead::Overflow => Err(TypedError {
            message: b"integer value too large to represent".to_vec(),
            code: b"ARITH IOVERFLOW",
        }),
        WideRead::NotInteger | WideRead::NotNumeric => Err(TypedError::expected("integer", obj)),
    }
}

/// The double read over the numeric tower.
#[cfg(have_tommath)]
fn read_double(obj: *mut TclObj) -> Option<f64> {
    crate::bignum::read_double(obj)
}

/// Without the tower (a wasm build whose libtommath cross-compile was
/// unavailable) the same classification runs over the shared number grammar
/// alone: the typed reps still short-circuit, a beyond-wide integer is the same
/// overflow, and the write-back still happens for the two reps this build has.
#[cfg(not(have_tommath))]
fn read_wide_int(obj: *mut TclObj) -> Result<i64, TypedError> {
    use tcl_syntax::number::Number;
    if obj::obj_type_ptr(obj) == &obj::TCL_INT_TYPE {
        return Ok(obj::wide_of(obj));
    }
    match parse_string_rep(obj) {
        Some(Number::Int(value)) => {
            obj::cache_wide_rep(obj, value);
            Ok(value)
        }
        Some(Number::Big { .. }) => Err(TypedError {
            message: b"integer value too large to represent".to_vec(),
            code: b"ARITH IOVERFLOW",
        }),
        _ => Err(TypedError::expected("integer", obj)),
    }
}

/// [`read_double`] without the numeric tower — see [`read_wide_int`]'s note.
#[cfg(not(have_tommath))]
fn read_double(obj: *mut TclObj) -> Option<f64> {
    use tcl_syntax::number::Number;
    let type_ptr = obj::obj_type_ptr(obj);
    if type_ptr == &obj::TCL_INT_TYPE {
        return Some(obj::wide_of(obj) as f64);
    }
    if type_ptr == &obj::TCL_DOUBLE_TYPE {
        return Some(obj::double_of(obj));
    }
    match parse_string_rep(obj)? {
        Number::Int(value) => {
            obj::cache_wide_rep(obj, value);
            Some(value as f64)
        }
        Number::Double(value) => {
            obj::cache_double_rep(obj, value);
            Some(value)
        }
        // A parsed `Big` is beyond `i64` by construction; without the tower the
        // magnitude is only needed for its sign and non-zeroness.
        Number::Big { negative, .. } => Some(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }),
        Number::Nan { .. } => Some(f64::NAN),
    }
}

/// The shared number grammar over an object's string rep (tower-less build).
#[cfg(not(have_tommath))]
fn parse_string_rep(obj: *mut TclObj) -> Option<tcl_syntax::number::Number> {
    let bytes = obj::bytes_of(obj);
    let text = core::str::from_utf8(&bytes).ok()?;
    tcl_syntax::number::parse_whole(text)
}
