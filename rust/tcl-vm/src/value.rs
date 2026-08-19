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

//! The VM value model — an idiomatic `Rc`-based dual-rep value reproducing Tcl
//! shimmering *behaviourally* without the 24-byte wasm32 ABI layout.
//!
//! A [`Value`] is a cheap-to-clone handle (`Rc` bump). It caches a lazily
//! generated string rep alongside a typed internal rep; reading a typed
//! accessor parses-and-caches (string→typed), `to_str` generates-and-caches
//! (typed→string). `Rc` strong-count is the shared/unshared signal for the
//! copy-on-write list ops. See the steering doc §4b.

#![allow(clippy::cast_precision_loss)] // i64→f64 numeric coercion is intentional

use std::cell::RefCell;
use std::rc::Rc;

use tcl_core_types::RecursionLimit;
use tcl_syntax::list;
use tcl_syntax::number::{self, Number};

use crate::error::TclError;

/// Depth cap for [`Value::to_str`]'s descent into nested `IntRep::List`
/// children — issue #996. A plain `for {set i 0} {$i<N} {incr i} {set v
/// [list $v]}` loop builds a value that nests one list inside another N
/// times; forcing its string form (`string length $v`, `puts $v`, a
/// comparison, …) used to recurse once per nesting level with no depth cap
/// — no `{*}` tricks needed, `dict`'s printing shares this path (a dict is
/// represented as a list here too). Empirically (a throwaway
/// `zzz_probe_depth to_str <depth>` harness, deleted before this fix
/// landed), unguarded input overflowed the native stack (SIGABRT) between
/// depth 1200 and 1250 on a 2 MiB thread (`cargo test`'s per-test default).
/// 256 leaves better than 4x margin under that measured crash floor while
/// staying far past any realistic nested-list depth. Past the cap, a
/// nested list element is rendered as [`TOO_DEEPLY_NESTED_PLACEHOLDER`]
/// instead of its true (recursively generated) string — see
/// [`Value::to_str_at_depth`].
const MAX_LIST_TO_STR_DEPTH: RecursionLimit = RecursionLimit(256);

/// The placeholder text substituted for a nested list element past
/// [`MAX_LIST_TO_STR_DEPTH`] — see [`Value::to_str_at_depth`]. Passed
/// through [`list::join_list`] like any other element, so it is quoted
/// exactly as safely as real data would be.
const TOO_DEEPLY_NESTED_PLACEHOLDER: &str = "<list too deeply nested to render, see issue #996>";

/// A Tcl value handle. Cloning bumps an `Rc`; the underlying object is shared.
#[derive(Clone)]
pub struct Value(Rc<Obj>);

struct Obj {
    /// Cached string rep (Tcl `bytes`). `None` until first generated from a
    /// typed rep. `RefCell` so a read can lazily cache without `&mut`.
    string: RefCell<Option<Rc<str>>>,
    /// Typed internal rep; `RefCell` so a value can shimmer in place.
    intrep: RefCell<IntRep>,
}

/// The typed internal representation — a closed enum (the VM does not need open
/// extension `Tcl_ObjType`s short-term).
#[derive(Clone)]
enum IntRep {
    /// No typed rep: the value *is* its string (string is always `Some`).
    Str,
    /// A wide integer.
    Int(i64),
    /// A double.
    Double(f64),
    /// A boolean (canonicalises to `"0"`/`"1"` string-side).
    Bool(bool),
    /// A list (`Rc` for O(1) clone + copy-on-write).
    List(Rc<Vec<Value>>),
}

impl Value {
    fn from_parts(string: Option<Rc<str>>, intrep: IntRep) -> Self {
        Self(Rc::new(Obj {
            string: RefCell::new(string),
            intrep: RefCell::new(intrep),
        }))
    }

    /// A pure-string value.
    pub fn string(s: impl Into<Rc<str>>) -> Self {
        Self::from_parts(Some(s.into()), IntRep::Str)
    }

    /// The empty string.
    #[must_use]
    pub fn empty() -> Self {
        Self::string("")
    }

    /// A wide-integer value.
    #[must_use]
    pub fn int(n: i64) -> Self {
        Self::from_parts(None, IntRep::Int(n))
    }

    /// A double value.
    #[must_use]
    pub fn double(f: f64) -> Self {
        Self::from_parts(None, IntRep::Double(f))
    }

    /// A boolean value.
    #[must_use]
    pub fn bool(b: bool) -> Self {
        Self::from_parts(None, IntRep::Bool(b))
    }

    /// A list value.
    #[must_use]
    pub fn list(items: Vec<Value>) -> Self {
        Self::from_parts(None, IntRep::List(Rc::new(items)))
    }

    /// The string representation, generating and caching it from the typed rep
    /// on first call (Tcl's `Tcl_GetString` / lazy `updateStringProc`).
    #[must_use]
    pub fn to_str(&self) -> Rc<str> {
        self.to_str_at_depth(0).0
    }

    /// [`Value::to_str`]'s recursive engine, with an explicit nesting-depth
    /// parameter — issue #996 (see [`MAX_LIST_TO_STR_DEPTH`]). `depth` is
    /// this value's nesting level within the *current* top-level `to_str()`
    /// call (0 at the root). Returns the string alongside whether rendering
    /// it anywhere in this subtree hit the depth cap — when it did, the
    /// result must not be cached (`self.0.string`): the same shared `Rc<Obj>`
    /// may also be reachable directly (or at a shallower offset) from
    /// elsewhere, where a fresh call starting back at depth 0 could
    /// legitimately render it in full, and caching the truncated result
    /// here would wrongly leak into that unrelated call. A value entirely
    /// within the cap is unaffected either way — same output, same caching
    /// — as the pre-fix implementation.
    fn to_str_at_depth(&self, depth: u32) -> (Rc<str>, bool) {
        if let Some(s) = self.0.string.borrow().as_ref() {
            return (Rc::clone(s), false);
        }
        let (generated, past_cap): (Rc<str>, bool) = match &*self.0.intrep.borrow() {
            IntRep::Str => (Rc::from(""), false),
            IntRep::Int(n) => (Rc::from(n.to_string().as_str()), false),
            IntRep::Double(f) => (Rc::from(number::format_double(*f).as_str()), false),
            IntRep::Bool(b) => (Rc::from(if *b { "1" } else { "0" }), false),
            IntRep::List(items) => {
                if MAX_LIST_TO_STR_DEPTH.exceeded(depth) {
                    (Rc::from(TOO_DEEPLY_NESTED_PLACEHOLDER), true)
                } else {
                    let mut any_past_cap = false;
                    let parts: Vec<String> = items
                        .iter()
                        .map(|v| {
                            let (s, capped) = v.to_str_at_depth(depth + 1);
                            any_past_cap |= capped;
                            s.to_string()
                        })
                        .collect();
                    (
                        Rc::from(list::join_list(parts.iter().map(String::as_str)).as_str()),
                        any_past_cap,
                    )
                }
            }
        };
        if !past_cap {
            *self.0.string.borrow_mut() = Some(Rc::clone(&generated));
        }
        (generated, past_cap)
    }

    /// The value as a wide integer (`Tcl_GetWideIntFromObj`), caching the typed
    /// rep when it parses from the string side.
    pub fn as_int(&self) -> Result<i64, TclError> {
        match &*self.0.intrep.borrow() {
            IntRep::Int(n) => return Ok(*n),
            IntRep::Bool(b) => return Ok(i64::from(*b)),
            IntRep::Str | IntRep::Double(_) | IntRep::List(_) => {}
        }
        let s = self.to_str();
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => {
                *self.0.intrep.borrow_mut() = IntRep::Int(n);
                Ok(n)
            }
            _ => Err(TclError::new(format!(
                "expected integer but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value truncated to a 64-bit wide (`wide()`): an in-range integer
    /// as-is, else a bignum literal reduced modulo 2^64 and bit-cast to `i64`
    /// (two's-complement wrap — what C's `wide()` does, e.g.
    /// `wide(0x8000000000000000)` is `-9223372036854775808`). The VM has no
    /// arbitrary-precision rep, so this is the truncating window onto one.
    pub fn as_wide(&self) -> Result<i64, TclError> {
        if let Ok(n) = self.as_int() {
            return Ok(n);
        }
        let s = self.to_str();
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = number::parse_whole(s.trim())
        {
            let base = radix as u32;
            let mut acc: u64 = 0;
            for ch in digits.chars() {
                let d = ch.to_digit(base).unwrap_or(0);
                acc = acc.wrapping_mul(u64::from(base)).wrapping_add(u64::from(d));
            }
            let signed = acc.cast_signed();
            return Ok(if negative {
                signed.wrapping_neg()
            } else {
                signed
            });
        }
        Err(TclError::new(format!("expected integer but got \"{s}\"")))
    }

    /// The value as a 128-bit integer, parsing an out-of-`i64`-range integer
    /// literal whose magnitude fits `i128`. This is the VM's bounded stand-in
    /// for Tcl's arbitrary-precision integers: it covers the common large-value
    /// range (`2**70`, `10**21`, …) so `expr`/`mathop` arithmetic promotes on
    /// `i64` overflow instead of wrapping. `None` for non-integers or magnitudes
    /// beyond `i128`.
    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        if let Ok(n) = self.as_int() {
            return Some(i128::from(n));
        }
        let s = self.to_str();
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = number::parse_whole(s.trim())
        {
            let base = radix as u32;
            let mut acc: u128 = 0;
            for ch in digits.chars() {
                let d = ch.to_digit(base)?;
                acc = acc
                    .checked_mul(u128::from(base))?
                    .checked_add(u128::from(d))?;
            }
            let mag = i128::try_from(acc).ok()?;
            return Some(if negative { -mag } else { mag });
        }
        None
    }

    /// The value as a double (`Tcl_GetDoubleFromObj`).
    pub fn as_double(&self) -> Result<f64, TclError> {
        match &*self.0.intrep.borrow() {
            IntRep::Int(n) => return Ok(*n as f64),
            IntRep::Double(f) => return Ok(*f),
            IntRep::Bool(b) => return Ok(f64::from(i32::from(*b))),
            IntRep::Str | IntRep::List(_) => {}
        }
        let s = self.to_str();
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n as f64),
            Some(Number::Double(f)) => Ok(f),
            _ => Err(TclError::new(format!(
                "expected floating-point number but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value as a boolean (`Tcl_GetBooleanFromObj`).
    pub fn as_bool(&self) -> Result<bool, TclError> {
        match &*self.0.intrep.borrow() {
            IntRep::Bool(b) => return Ok(*b),
            IntRep::Int(n) => return Ok(*n != 0),
            IntRep::Double(f) => return Ok(*f != 0.0),
            IntRep::Str | IntRep::List(_) => {}
        }
        let s = self.to_str();
        let t = s.trim();
        if let Some(num) = number::parse_whole(t) {
            return match num {
                Number::Int(n) => Ok(n != 0),
                Number::Double(f) => Ok(f != 0.0),
                // A parsed `Big` is beyond `i64`, hence never zero.
                Number::Big { .. } => Ok(true),
                // NaN is a domain error in a boolean context (tclsh 8.6/9.0:
                // "floating point value is Not a Number"), never truthy.
                Number::Nan { .. } => Err(TclError::new("floating point value is Not a Number")),
            };
        }
        // The canonical word acceptor (`ParseBoolean`, tclObj.c): any
        // unambiguous case-insensitive prefix of the six boolean words —
        // one home in `tcl_syntax::boolean`, oracle-table-pinned. The word is
        // matched **untrimmed**: unlike the numeric branch above, C's
        // `ParseBoolean` does no whitespace stripping, so `" true"` is
        // `expected boolean value but got " true"` on tclsh 8.6.16 / 9.0.4.
        match tcl_syntax::boolean::parse_boolean_word(&s) {
            Some(b) => Ok(b),
            None => Err(TclError::new(format!(
                "expected boolean value but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value as a list of elements (`Tcl_ListObjGetElements`), caching the
    /// typed rep on first parse.
    pub fn as_list(&self) -> Result<Rc<Vec<Value>>, TclError> {
        if let IntRep::List(items) = &*self.0.intrep.borrow() {
            return Ok(Rc::clone(items));
        }
        let s = self.to_str();
        let elems = list::split_list(&s).map_err(|e| TclError::new(e.full_message(&s)))?;
        let items: Rc<Vec<Value>> =
            Rc::new(elems.iter().map(|c| Value::string(c.as_ref())).collect());
        *self.0.intrep.borrow_mut() = IntRep::List(Rc::clone(&items));
        Ok(items)
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({:?})", &*self.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_shimmers_to_string() {
        assert_eq!(&*Value::int(42).to_str(), "42");
        assert_eq!(&*Value::double(2.0).to_str(), "2.0");
        assert_eq!(&*Value::bool(true).to_str(), "1");
        assert_eq!(&*Value::empty().to_str(), "");
    }

    #[test]
    fn string_shimmers_to_int() {
        assert_eq!(Value::string("42").as_int().unwrap(), 42);
        assert_eq!(Value::string("  -7 ").as_int().unwrap(), -7);
        assert!(Value::string("abc").as_int().is_err());
    }

    #[test]
    fn list_roundtrip() {
        let v = Value::list(vec![Value::int(1), Value::string("a b")]);
        assert_eq!(&*v.to_str(), "1 {a b}");
        let parsed = Value::string("1 {a b}").as_list().unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(&*parsed[1].to_str(), "a b");
    }

    #[test]
    fn boolean_words() {
        assert!(Value::string("yes").as_bool().unwrap());
        assert!(!Value::string("off").as_bool().unwrap());
        assert!(Value::string("3").as_bool().unwrap());
    }

    /// The boolean-context acceptor, oracle-pinned (tclsh 8.6/9.0): word
    /// prefixes resolve, any number compares against zero, `Inf` is truthy —
    /// and `NaN` is a domain error ("floating point value is Not a Number"),
    /// never a truth value.
    #[test]
    fn boolean_context_matches_oracle() {
        assert!(Value::string("tru").as_bool().unwrap());
        assert!(!Value::string("of").as_bool().unwrap());
        assert!(Value::string("0.5").as_bool().unwrap());
        assert!(!Value::string("0x0").as_bool().unwrap());
        assert!(Value::string("Inf").as_bool().unwrap());
        assert!(Value::string("18446744073709551616").as_bool().unwrap());
        let err = Value::string("NaN").as_bool().unwrap_err();
        assert!(
            err.message.contains("Not a Number"),
            "NaN must be the C domain error, got: {}",
            err.message
        );
        assert!(Value::string("o").as_bool().is_err(), "ambiguous prefix");
    }

    /// Regression coverage for issue #996: `Value::to_str`'s descent into
    /// `IntRep::List` children recurses once per nesting level, with no
    /// depth cap before this fix — a plain `for {set i 0} {$i<N} {incr i}
    /// {set v [list $v]}` loop builds the input, no `{*}` tricks needed.
    /// Empirically (a throwaway `zzz_probe_depth to_str <depth>` harness,
    /// deleted before this fix landed), unguarded input overflowed the
    /// native stack (SIGABRT) between depth 1200 and 1250 on a 2 MiB thread
    /// (`cargo test`'s per-test default). 2000 is comfortably past both
    /// that crash range and `MAX_LIST_TO_STR_DEPTH` (256); the assertion is
    /// that `to_str` returns at all, not what it returns.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain
    /// `to_str` used to (empirically, SIGABRT between depth 3500 and 4000
    /// on a 2 MiB thread for construction+drop alone, independent of
    /// `to_str` or any other operation). That is a separate, genuinely
    /// unbounded-depth concern in `Value`'s representation itself — out of
    /// scope for this fix.
    #[test]
    fn deeply_nested_list_to_str_survives() {
        const DEPTH: usize = 2_000;
        let mut v = Value::string("leaf");
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
        }
        let s = v.to_str();
        assert!(!s.is_empty());
    }

    /// A moderately nested `[list $v]` chain (well under
    /// `MAX_LIST_TO_STR_DEPTH`) is byte-for-byte unaffected by the depth
    /// cap: its output matches an independent oracle (hand-driving
    /// `list::join_list` the same number of times, entirely outside
    /// `Value`) exactly.
    #[test]
    fn moderately_nested_list_to_str_matches_naive_join() {
        const DEPTH: usize = 20;
        let mut v = Value::string("a b");
        let mut expected = "a b".to_string();
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
            expected = list::join_list(std::iter::once(expected.as_str()));
        }
        assert_eq!(&*v.to_str(), expected.as_str());
    }

    /// The depth-cap fallback must not poison the shared string cache: a
    /// value that only exceeds `MAX_LIST_TO_STR_DEPTH` because of *where*
    /// it happens to be nested inside a deeper structure (reached via a
    /// shared `Rc<Obj>`, e.g. the same sublist embedded in two different
    /// places) must still render its true value when addressed directly
    /// afterwards, starting fresh at depth 0 — not a cached placeholder
    /// left over from the capped render.
    #[test]
    fn depth_cap_fallback_does_not_poison_the_shared_string_cache() {
        // `chain`: 10 levels of `[list $v]` around a value that needs
        // brace-quoting ("a b"), well within the cap on its own.
        let mut chain = Value::string("a b");
        let mut expected = "a b".to_string();
        for _ in 0..10 {
            chain = Value::list(vec![chain]);
            expected = list::join_list(std::iter::once(expected.as_str()));
        }
        // `far_outer`: the *same* `chain` object (`Rc` clone), wrapped 300
        // more levels deep — deep enough that reaching `chain` from here
        // exceeds `MAX_LIST_TO_STR_DEPTH` (256), even though `chain` itself
        // is shallow.
        let mut far_outer = chain.clone();
        for _ in 0..300 {
            far_outer = Value::list(vec![far_outer]);
        }
        // Force the deep, capped render first: `chain`'s own `Obj` must come
        // out of this untouched (not cached with the placeholder).
        let far_str = far_outer.to_str();
        assert!(
            far_str.contains(TOO_DEEPLY_NESTED_PLACEHOLDER),
            "expected the placeholder somewhere in: {far_str}"
        );
        // A fresh call directly on the same shared value, starting back at
        // depth 0, must render its true (well-within-cap) value — not the
        // placeholder that would have leaked in had the capped render above
        // wrongly cached it.
        assert_eq!(&*chain.to_str(), expected.as_str());
    }
}
