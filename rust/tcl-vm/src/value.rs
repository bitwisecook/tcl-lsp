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

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_core_types::RecursionLimit;
use tcl_runtime_api::CommandEvaluation;
use tcl_syntax::list;
use tcl_syntax::number::{self, Number};
use tcl_syntax::value::canonical_dict_slots;

use crate::error::TclError;

/// Depth cap for [`Value::to_str`]'s descent into nested `IntRep::List`
/// children. A plain `for {set i 0} {$i<N} {incr i} {set v
/// [list $v]}` loop builds a value that nests one list inside another N
/// times; forcing its string form (`string length $v`, `puts $v`, a
/// comparison, …) with no depth cap would recurse once per nesting level
/// — no `{*}` tricks needed, `dict`'s printing shares this path (a dict is
/// represented as a list here too). Empirically, unguarded input overflows
/// the native stack (SIGABRT) between
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
    /// The bytes a typed value measured to ([`Value::measured_bytes`]),
    /// [`UNMEASURED`] until first asked. A value never changes what it
    /// renders to, so the answer holds for its lifetime and each value is
    /// measured once however often a limit asks.
    measure: Cell<u64>,
}

/// [`Obj::measure`] before the value is first measured.
const UNMEASURED: u64 = u64::MAX;

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
    /// A list (`Rc` for O(1) clone + copy-on-write) whose string, if it has
    /// one, was generated from it: a canonical list (`TclListObjIsCanonical`).
    List(Rc<Vec<Value>>),
    /// A list read from the string the value keeps, which need not be the
    /// list's canonical spelling.
    ParsedList(Rc<Vec<Value>>),
    /// A dictionary with its retained Tcl hash-table growth history.
    Dict(Rc<DictRep>),
}

#[derive(Clone, Debug)]
struct DictRep {
    pairs: Vec<(Value, Value)>,
    hash_order: TclStringHashOrder,
}

impl Value {
    fn from_parts(string: Option<Rc<str>>, intrep: IntRep) -> Self {
        Self(Rc::new(Obj {
            string: RefCell::new(string),
            intrep: RefCell::new(intrep),
            measure: Cell::new(UNMEASURED),
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

    /// A native dictionary value from canonical key/value pairs.
    #[must_use]
    pub(crate) fn dict(pairs: Vec<(Value, Value)>) -> Self {
        Self::dict_with_hash_bucket_count(pairs, None)
    }

    /// A native dictionary retaining the source table's bucket-array size.
    #[must_use]
    pub(crate) fn dict_with_hash_bucket_count(
        pairs: Vec<(Value, Value)>,
        bucket_count: Option<usize>,
    ) -> Self {
        let mut hash_order = TclStringHashOrder::default();
        if let Some(bucket_count) = bucket_count {
            hash_order.retain_bucket_count(bucket_count);
        }
        for (key, _) in &pairs {
            hash_order.insert(key.to_str().as_bytes());
        }
        Self::from_parts(None, IntRep::Dict(Rc::new(DictRep { pairs, hash_order })))
    }

    /// The string representation, generating and caching it from the typed rep
    /// on first call (Tcl's `Tcl_GetString` / lazy `updateStringProc`).
    #[must_use]
    pub fn to_str(&self) -> Rc<str> {
        self.to_str_at_depth(0).0
    }

    /// [`Value::to_str`]'s recursive engine, with an explicit nesting-depth
    /// parameter (see [`MAX_LIST_TO_STR_DEPTH`]). `depth` is
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
            IntRep::List(items) | IntRep::ParsedList(items) => {
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
            IntRep::Dict(dict) => {
                if MAX_LIST_TO_STR_DEPTH.exceeded(depth) {
                    (Rc::from(TOO_DEEPLY_NESTED_PLACEHOLDER), true)
                } else {
                    let mut any_past_cap = false;
                    let parts: Vec<String> = dict
                        .pairs
                        .iter()
                        .flat_map(|(key, value)| [key, value])
                        .map(|value| {
                            let (s, capped) = value.to_str_at_depth(depth + 1);
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
    /// rep when it parses from the string side. A failure carries C's message
    /// and `-errorcode`: an integer past the wide range is `ARITH IOVERFLOW`, a
    /// double is refused under `TCL VALUE INTEGER` and anything else under
    /// `TCL VALUE NUMBER`.
    pub fn as_int(&self) -> Result<i64, TclError> {
        let holds_double = match &*self.0.intrep.borrow() {
            IntRep::Int(n) => return Ok(*n),
            IntRep::Bool(b) => return Ok(i64::from(*b)),
            IntRep::Double(_) => true,
            IntRep::Str | IntRep::List(_) | IntRep::ParsedList(_) | IntRep::Dict(_) => false,
        };
        let s = self.to_str();
        match number::parse_whole(&s) {
            Some(Number::Int(n)) => {
                *self.0.intrep.borrow_mut() = IntRep::Int(n);
                Ok(n)
            }
            Some(Number::Big { .. }) => Err(TclError::with_error_code(
                tcl_syntax::expr::errors::IOVERFLOW_MESSAGE,
                tcl_syntax::expr::errors::IOVERFLOW_CODE,
            )),
            _ if holds_double => Err(TclError::with_error_code(
                format!("expected integer but got \"{s}\""),
                "TCL VALUE INTEGER",
            )),
            _ => Err(TclError::with_error_code(
                format!("expected integer but got {}", list::describe_bad_value(&s)),
                "TCL VALUE NUMBER",
            )),
        }
    }

    /// The value as a C `int` (`Tcl_GetIntFromObj`): [`Self::as_int`] narrowed
    /// as C narrows it ([`tcl_syntax::value::narrow_to_c_int`]), so
    /// `4294967295` reads -1 and `4294967296` is the overflow.
    pub fn as_c_int(&self) -> Result<i32, TclError> {
        tcl_syntax::value::narrow_to_c_int(self.as_int()?).map_err(|_| {
            TclError::with_error_code(
                tcl_syntax::expr::errors::IOVERFLOW_MESSAGE,
                tcl_syntax::expr::errors::IOVERFLOW_CODE,
            )
        })
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
        }) = number::parse_whole(&s)
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
        }) = number::parse_whole(&s)
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
            IntRep::Str | IntRep::List(_) | IntRep::ParsedList(_) | IntRep::Dict(_) => {}
        }
        let s = self.to_str();
        match number::parse_whole(&s) {
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
            IntRep::Str | IntRep::List(_) | IntRep::ParsedList(_) | IntRep::Dict(_) => {}
        }
        let s = self.to_str();
        if let Some(num) = number::parse_whole(&s) {
            return match num {
                Number::Int(n) => Ok(n != 0),
                Number::Double(f) => Ok(f != 0.0),
                // A parsed `Big` is beyond `i64`, hence never zero.
                Number::Big { .. } => Ok(true),
                // NaN is a domain error in a boolean context (tclsh 8.6/9.0:
                // "floating point value is Not a Number"), never truthy.
                Number::Nan { .. } => Err(TclError::with_error_code(
                    "floating point value is Not a Number",
                    "TCL VALUE DOUBLE NAN",
                )),
            };
        }
        // The canonical word acceptor (`ParseBoolean`, tclObj.c): any
        // unambiguous case-insensitive prefix of the six boolean words —
        // one home in `tcl_syntax::boolean`, oracle-table-pinned.
        match tcl_syntax::boolean::parse_boolean_word(&s) {
            Some(b) => Ok(b),
            None => Err(TclError::with_error_code(
                format!(
                    "expected boolean value but got {}",
                    list::describe_bad_value(&s)
                ),
                "TCL VALUE NUMBER",
            )),
        }
    }

    /// The value as a list of elements (`Tcl_ListObjGetElements`), caching the
    /// typed rep on first parse.
    pub fn as_list(&self) -> Result<Rc<Vec<Value>>, TclError> {
        if let IntRep::List(items) | IntRep::ParsedList(items) = &*self.0.intrep.borrow() {
            return Ok(Rc::clone(items));
        }
        let dict = match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Some(Rc::clone(dict)),
            _ => None,
        };
        if let Some(dict) = dict {
            let items = Rc::new(
                dict.pairs
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect(),
            );
            let list = if self.0.string.borrow().is_some() {
                IntRep::ParsedList(Rc::clone(&items))
            } else {
                IntRep::List(Rc::clone(&items))
            };
            *self.0.intrep.borrow_mut() = list;
            return Ok(items);
        }
        let s = self.to_str();
        let elems = list::split_list(&s)
            .map_err(|e| TclError::with_error_code(e.full_message(&s), e.error_code()))?;
        let items: Rc<Vec<Value>> =
            Rc::new(elems.iter().map(|c| Value::string(c.as_ref())).collect());
        *self.0.intrep.borrow_mut() = IntRep::ParsedList(Rc::clone(&items));
        Ok(items)
    }

    /// Whether the value is a canonical list (`TclListObjIsCanonical`): a
    /// list with no string, or one generated from the list. A script that is
    /// one runs as one command whose words are its elements, without being
    /// compiled (`TclEvalObjEx`'s canonical-list path).
    #[must_use]
    pub fn is_canonical_list(&self) -> bool {
        matches!(&*self.0.intrep.borrow(), IntRep::List(_))
    }

    /// The canonical list a compiler built from literal words, spelt
    /// `canonical` (C's `TclCompileListCmd` list literal). A spelling that is
    /// not a list stays a string.
    pub(crate) fn list_literal(canonical: &str) -> Self {
        match list::split_list(canonical) {
            Ok(elems) => Self::from_parts(
                Some(Rc::from(canonical)),
                IntRep::List(Rc::new(
                    elems.iter().map(|c| Value::string(c.as_ref())).collect(),
                )),
            ),
            Err(_) => Self::string(canonical),
        }
    }

    /// Compile the value as a script whatever it holds, as `INST_EVAL_STK`
    /// does: the bytecode replaces a list rep, so a canonical list is no longer
    /// one and a later evaluation of the same value compiles it too.
    pub(crate) fn compile_as_script(&self) {
        if !self.is_canonical_list() {
            return;
        }
        let _ = self.to_str();
        let mut intrep = self.0.intrep.borrow_mut();
        if let IntRep::List(items) = &*intrep {
            *intrep = IntRep::ParsedList(Rc::clone(items));
        }
    }

    /// Canonical key/value pairs, shimmering a string/list value to a native
    /// dictionary on first access.
    pub(crate) fn dict_pairs(&self) -> Result<Vec<(Value, Value)>, TclError> {
        if let IntRep::Dict(dict) = &*self.0.intrep.borrow() {
            return Ok(dict.pairs.clone());
        }
        let items = self.as_list()?;
        if items.len() % 2 != 0 {
            return Err(TclError::new("missing value to go with key"));
        }
        let keys: Vec<Rc<str>> = items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| chunk[0].to_str())
            .collect();
        let pairs: Vec<(Value, Value)> = canonical_dict_slots(keys.iter().map(AsRef::as_ref))
            .into_iter()
            .map(|(key_slot, value_slot)| {
                (
                    items[key_slot * 2].clone(),
                    items[value_slot * 2 + 1].clone(),
                )
            })
            .collect();
        let mut hash_order = TclStringHashOrder::default();
        for (key, _) in &pairs {
            hash_order.insert(key.to_str().as_bytes());
        }
        *self.0.intrep.borrow_mut() = IntRep::Dict(Rc::new(DictRep {
            pairs: pairs.clone(),
            hash_order,
        }));
        Ok(pairs)
    }

    /// Retained bucket count of the native dictionary representation.
    pub(crate) fn dict_hash_bucket_count(&self) -> Result<usize, TclError> {
        self.dict_pairs()?;
        match &*self.0.intrep.borrow() {
            IntRep::Dict(dict) => Ok(dict.hash_order.bucket_count()),
            _ => unreachable!("dict_pairs installs the dictionary representation"),
        }
    }

    /// Hash-table capacity to seed a rebuilt dictionary mutation, plus whether
    /// this object required Tcl copy-on-write.
    ///
    /// An unshared update retains the table's growth history. Tcl's
    /// `DupDictInternalRep`, however, creates a fresh four-bucket table and
    /// reinserts only live entries. `owned_references` discounts the handles
    /// which the immutable VM's current traversal necessarily owns; any
    /// further handle is a Tcl-level alias.
    pub(crate) fn dict_mutation_hash_state(
        &self,
        owned_references: usize,
        parent_was_copied: bool,
    ) -> Result<(Option<usize>, bool), TclError> {
        self.dict_pairs()?;
        let copied = parent_was_copied || Rc::strong_count(&self.0) > owned_references;
        let bucket_count = match &*self.0.intrep.borrow() {
            IntRep::Dict(_) if copied => None,
            IntRep::Dict(dict) => Some(dict.hash_order.bucket_count()),
            _ => unreachable!("dict_pairs installs the dictionary representation"),
        };
        Ok((bucket_count, copied))
    }

    /// The byte length this value has, or would have once rendered, counting
    /// no further than just past `cap`: a string's bytes, a number's digits,
    /// and a list's or dict's elements plus a separator between each (the
    /// quoting rendering may add is not counted). A list is measured through
    /// its elements rather than rendered — a list holding itself twice, nested
    /// a few dozen deep, renders to more than any host has — and a complete
    /// measure is kept, so a list built from measured elements costs one pass.
    pub(crate) fn measured_bytes(&self, cap: u64) -> u64 {
        self.measured_bytes_at_depth(cap, 0)
    }

    /// [`Value::measured_bytes`] at nesting `depth`; past the depth
    /// [`Value::to_str`] renders, an element counts as one byte.
    fn measured_bytes_at_depth(&self, cap: u64, depth: u32) -> u64 {
        if let Some(s) = self.0.string.borrow().as_ref() {
            return s.len() as u64;
        }
        let cached = self.0.measure.get();
        if cached != UNMEASURED {
            return cached;
        }
        let total = match &*self.0.intrep.borrow() {
            IntRep::Str => return 0,
            IntRep::Bool(_) => return 1,
            IntRep::Int(n) => {
                u64::from(n.unsigned_abs().checked_ilog10().unwrap_or(0) + 1) + u64::from(*n < 0)
            }
            IntRep::Double(f) => number::format_double(*f).len() as u64,
            _ if MAX_LIST_TO_STR_DEPTH.exceeded(depth) => return 1,
            IntRep::List(items) | IntRep::ParsedList(items) => {
                measure_elements(items.iter(), cap, depth)
            }
            IntRep::Dict(dict) => measure_elements(
                dict.pairs.iter().flat_map(|(key, value)| [key, value]),
                cap,
                depth,
            ),
        };
        if total > cap {
            return total;
        }
        self.0.measure.set(total);
        total
    }

    /// Whether two handles refer to the same Tcl value object.
    #[must_use]
    pub(crate) fn is_same_object(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The bytes `items` measure to with a separator between each, stopping once
/// past `cap`.
fn measure_elements<'a>(items: impl Iterator<Item = &'a Value>, cap: u64, depth: u32) -> u64 {
    let mut total = 0u64;
    for (i, item) in items.enumerate() {
        total = total
            .saturating_add(u64::from(i > 0))
            .saturating_add(item.measured_bytes_at_depth(cap.saturating_sub(total), depth + 1));
        if total > cap {
            break;
        }
    }
    total
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({:?})", &*self.to_str())
    }
}

/// How the emulated release evaluates the script `words` make, one word as
/// it is or several joined by `Tcl_ConcatObj`: `TclEvalObjEx` runs a canonical
/// list as one command whose words are its elements, without compiling it,
/// and compiles anything else. `Tcl_ConcatObj` keeps canonical lists and empty
/// words a canonical list, but for a later list whose first element starts a
/// comment.
pub(crate) fn script_evaluation(words: &[Value]) -> CommandEvaluation {
    let mut started = false;
    for word in words {
        if word.is_canonical_list() {
            let comment = word
                .as_list()
                .ok()
                .and_then(|items| items.first().map(|first| first.to_str().starts_with('#')))
                .unwrap_or(false);
            if started && comment {
                return CommandEvaluation::Compiled;
            }
            started = true;
        } else if words.len() == 1 || !word.to_str().is_empty() {
            return CommandEvaluation::Compiled;
        }
    }
    if started {
        CommandEvaluation::Direct
    } else {
        CommandEvaluation::Compiled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `TclEvalObjEx` runs a canonical list without compiling it, and
    /// `Tcl_ConcatObj` keeps canonical lists and empty words one; tclsh 8.6.18
    /// shows it with `eval [list string range $s 0 end-2147483649]` and
    /// `eval [list string range] [list $s 0 end-2147483649]` ranging nothing,
    /// where the string spelling of either, and a mix of a list and a string,
    /// range the whole string.
    #[test]
    fn a_canonical_list_script_is_evaluated_directly() {
        use CommandEvaluation::{Compiled, Direct};
        let list = || Value::list(vec![Value::string("string"), Value::string("range")]);
        assert_eq!(script_evaluation(&[list()]), Direct);
        assert_eq!(script_evaluation(&[list(), Value::empty(), list()]), Direct);
        let rendered = list();
        assert_eq!(&*rendered.to_str(), "string range");
        assert_eq!(script_evaluation(&[rendered]), Direct);
        let parsed = Value::string("string range");
        assert_eq!(parsed.as_list().map(|items| items.len()).ok(), Some(2));
        assert_eq!(script_evaluation(&[parsed]), Compiled);
        assert_eq!(script_evaluation(&[list(), Value::string("a b")]), Compiled);
        assert_eq!(script_evaluation(&[Value::empty()]), Compiled);
        let comment = Value::list(vec![Value::string("#c")]);
        assert_eq!(script_evaluation(&[list(), comment]), Compiled);
    }

    /// `TclCompileListCmd` pushes `[list …]` of literal words as its list
    /// object, and `INST_EVAL_STK` compiling a canonical list leaves it a
    /// string: tclsh 8.6.18 ranges nothing for `eval` of the one and the whole
    /// string for `catch $b; eval $b` in a procedure.
    #[test]
    fn a_list_literal_is_canonical_until_compiled_as_a_script() {
        use CommandEvaluation::{Compiled, Direct};
        let literal = Value::list_literal("string range {a b}");
        assert!(literal.is_canonical_list());
        assert_eq!(&*literal.to_str(), "string range {a b}");
        assert_eq!(literal.as_list().map(|items| items.len()).ok(), Some(3));
        assert_eq!(script_evaluation(std::slice::from_ref(&literal)), Direct);
        literal.compile_as_script();
        assert!(!literal.is_canonical_list());
        assert_eq!(&*literal.to_str(), "string range {a b}");
        assert_eq!(literal.as_list().map(|items| items.len()).ok(), Some(3));
        assert_eq!(script_evaluation(&[literal]), Compiled);
        assert!(!Value::list_literal("a {b").is_canonical_list());
        let parsed = Value::string("a b");
        parsed.compile_as_script();
        assert!(!parsed.is_canonical_list());
    }

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

    #[test]
    fn boolean_numeric_whitespace_matches_tcl() {
        assert!(Value::string("true").as_bool().unwrap());
        for value in [" true", "true ", "\ttrue\r"] {
            assert!(Value::string(value).as_bool().is_err(), "{value:?}");
        }
        assert!(Value::string("\t42\r").as_bool().unwrap());
        for value in ["\u{2003}42", "42\u{2003}"] {
            assert!(Value::string(value).as_bool().is_err(), "{value:?}");
        }
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

    /// `Value::to_str`'s descent into `IntRep::List` children recursing once
    /// per nesting level with no depth cap — a plain `for {set i 0} {$i<N}
    /// {incr i} {set v [list $v]}` loop builds the input, no `{*}` tricks
    /// needed — empirically overflows the native stack (SIGABRT) between
    /// depth 1200 and 1250 on a 2 MiB thread (`cargo test`'s per-test
    /// default). 2000 is comfortably past both that crash range and
    /// `MAX_LIST_TO_STR_DEPTH` (256); the assertion is that `to_str` returns
    /// at all, not what it returns.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain a
    /// naive `to_str` would (empirically, SIGABRT between depth 3500 and 4000
    /// on a 2 MiB thread for construction+drop alone, independent of
    /// `to_str` or any other operation). That is a separate, genuinely
    /// unbounded-depth concern in `Value`'s representation itself, and this
    /// test does not cover it.
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
