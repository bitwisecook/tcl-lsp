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

//! The shim's `Tcl_Obj`: a reference-counted, dual-representation value.
//!
//! An [`Obj`] carries an optional string representation and an internal
//! representation, exactly as C Tcl's does, so that an integer or a list
//! built by C code never takes a detour through text: `Tcl_NewIntObj(5)` is
//! `Rep::Int(5)` with no string until someone asks for one, and it crosses the
//! interface as [`Value::Int`]. The string rep is generated lazily and cached,
//! and the pointer `Tcl_GetString` hands out stays valid until the object is
//! mutated or freed — the same contract C Tcl gives.
//!
//! Which representation crosses the boundary is decided by `canonical`: when
//! a string rep exists that did *not* come from the internal rep (the object
//! was created from text and merely parsed), the text is authoritative and is
//! what the engine sees, so `0x10` stays `0x10`. When the string was rendered
//! from the rep, or there is no string at all, the typed rep crosses.
//!
//! Reference counts map onto Rust ownership through [`ObjRef`]: cloning one is
//! `Tcl_IncrRefCount`, dropping one is `Tcl_DecrRefCount`, and a count of zero
//! frees the object — including the C convention that a freshly created
//! object has count zero and is owned by whoever first takes a reference.

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::ptr::NonNull;

use tcl_engine_api::Value;
use tcl_syntax::list::{self, ListError};
use tcl_syntax::number::{self, Number, ParseFlags, Radix};

/// A Tcl error a value conversion raises: the message and the `-errorcode`,
/// worded as C Tcl words them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TclError {
    /// The error message.
    pub message: String,
    /// The `-errorcode` list, when C Tcl sets one.
    pub code: Option<String>,
}

impl TclError {
    /// An error with a code.
    #[must_use]
    pub fn with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// `expected <what> but got "<text>"` with the `TCL VALUE NUMBER` code.
    fn expected(what: &str, text: &str) -> Self {
        Self::with_code(
            format!("expected {what} but got \"{text}\""),
            "TCL VALUE NUMBER",
        )
    }

    /// The integer-overflow error, message and code as C Tcl reports them.
    pub(crate) fn overflow() -> Self {
        const MESSAGE: &str = "integer value too large to represent";
        Self::with_code(MESSAGE, list::join_list(["ARITH", "IOVERFLOW", MESSAGE]))
    }

    /// The NaN domain error, as `Tcl_GetDoubleFromObj` and
    /// `Tcl_GetBooleanFromObj` both report it.
    fn nan() -> Self {
        Self::with_code(
            "floating point value is Not a Number",
            "TCL VALUE DOUBLE NAN",
        )
    }

    fn list(error: ListError, text: &str) -> Self {
        let kind = match error {
            ListError::UnmatchedBrace => "BRACE",
            ListError::UnmatchedQuote => "QUOTE",
            ListError::BraceFollowedByJunk | ListError::QuoteFollowedByJunk => "JUNK",
        };
        Self::with_code(error.full_message(text), format!("TCL VALUE LIST {kind}"))
    }
}

/// The internal representation.
enum Rep {
    /// A pure string.
    None,
    Int(i64),
    Double(f64),
    List(Vec<ObjRef>),
    /// The table entry `Tcl_GetIndexFromObj` resolved this value to, kept so
    /// `Tcl_WrongNumArgs` can print the full option name of an abbreviation.
    Index(Box<str>),
}

/// A Tcl value: `Tcl_Obj` on the C side of the header.
///
/// The first five fields are the layout `runtime/rust/include/tcl.h` declares
/// for `Tcl_Obj` (`refCount`, `bytes`, `length`, `typePtr`, `internalRep`), at
/// the offsets the header's reference-count macros and an extension's reads of
/// `objPtr->bytes` use; the shim's own state follows that prefix, which C never
/// sees. Those fields are cells because C writes `refCount` directly and the
/// shim keeps `bytes` and `length` in step with the string rep it owns.
#[repr(C)]
pub struct Obj {
    /// `refCount`: one unit per holder. `Tcl_IncrRefCount` and `Tcl_DecrRefCount`
    /// are macros over this field.
    refcount: Cell<isize>,
    /// `bytes`: the first byte of the string rep, which is NUL-terminated at
    /// `length`, or null while there is none.
    bytes: Cell<*mut c_char>,
    /// `length`: the bytes of the string rep, the terminator excluded.
    length: Cell<isize>,
    /// `typePtr`: always null. The shim keeps its internal representation to
    /// itself, so C sees an object with no type.
    type_ptr: Cell<*const c_void>,
    /// `internalRep`: room for the union's two pointers, unused.
    internal_rep: Cell<[usize; 2]>,
    /// The string rep as NUL-terminated bytes, once generated.
    string: RefCell<Option<Box<[u8]>>>,
    rep: RefCell<Rep>,
    /// Whether `string` was rendered from `rep` (so the typed rep may cross the
    /// boundary) rather than `rep` being parsed from `string`.
    canonical: Cell<bool>,
}

/// Tcl's modified UTF-8 spells an interior NUL as `C0 80` so a string rep is
/// always a valid C string. The interface's strings are ordinary Rust text.
fn encode_text(text: &str) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    for byte in text.bytes() {
        if byte == 0 {
            bytes.extend_from_slice(&[0xC0, 0x80]);
        } else {
            bytes.push(byte);
        }
    }
    bytes.push(0);
    bytes.into_boxed_slice()
}

/// Decode string-rep bytes, restoring interior NULs from `C0 80`.
pub(crate) fn decode_bytes(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    if bytes.windows(2).any(|pair| pair == [0xC0, 0x80]) {
        // Rare path: restore the interior NULs the encoding spelled out.
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == 0xC0 && bytes.get(index + 1) == Some(&0x80) {
                out.push(0);
                index += 2;
            } else {
                out.push(bytes[index]);
                index += 1;
            }
        }
        return String::from_utf8_lossy(&out).into_owned();
    }
    text.into_owned()
}

impl Obj {
    fn with_rep(rep: Rep, string: Option<Box<[u8]>>, canonical: bool) -> Self {
        let obj = Self {
            refcount: Cell::new(0),
            bytes: Cell::new(std::ptr::null_mut()),
            length: Cell::new(0),
            type_ptr: Cell::new(std::ptr::null()),
            internal_rep: Cell::new([0; 2]),
            string: RefCell::new(None),
            rep: RefCell::new(rep),
            canonical: Cell::new(canonical),
        };
        obj.set_string(string);
        obj
    }

    /// Replace the string rep, keeping the `bytes` and `length` fields C reads
    /// in step with it: null and zero while there is none.
    fn set_string(&self, string: Option<Box<[u8]>>) {
        if let Some(bytes) = &string {
            self.bytes.set(bytes.as_ptr().cast_mut().cast::<c_char>());
            self.length
                .set(isize::try_from(bytes.len() - 1).unwrap_or(isize::MAX));
        } else {
            self.bytes.set(std::ptr::null_mut());
            self.length.set(0);
        }
        *self.string.borrow_mut() = string;
    }

    /// A string value from raw bytes (no NUL terminator in `bytes`).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut owned = Vec::with_capacity(bytes.len() + 1);
        owned.extend_from_slice(bytes);
        owned.push(0);
        Self::with_rep(Rep::None, Some(owned.into_boxed_slice()), false)
    }

    /// A string value from text.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::with_rep(Rep::None, Some(encode_text(text)), false)
    }

    /// An integer, with no string rep until one is asked for.
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::with_rep(Rep::Int(value), None, true)
    }

    /// A double, likewise.
    #[must_use]
    pub fn double(value: f64) -> Self {
        Self::with_rep(Rep::Double(value), None, true)
    }

    /// A list, likewise.
    #[must_use]
    pub fn list(items: Vec<ObjRef>) -> Self {
        Self::with_rep(Rep::List(items), None, true)
    }

    /// The reference count, as `Tcl_IsShared` reads it.
    #[must_use]
    pub fn refcount(&self) -> usize {
        usize::try_from(self.refcount.get()).unwrap_or(0)
    }

    /// Whether more than one reference holds this object.
    #[must_use]
    pub fn is_shared(&self) -> bool {
        self.refcount.get() > 1
    }

    /// Render the internal rep as string-rep bytes (NUL-terminated).
    fn render(&self) -> Box<[u8]> {
        let rep = self.rep.borrow();
        let text = match &*rep {
            Rep::Int(value) => value.to_string(),
            Rep::Double(value) => number::format_double(*value),
            Rep::List(items) => list::join_list(items.iter().map(|item| item.get().text())),
            // Both invariants keep a string rep: a pure string always has one,
            // and an index rep is only ever installed on a value that had one.
            Rep::None | Rep::Index(_) => String::new(),
        };
        encode_text(&text)
    }

    /// Ensure the string rep exists, then run `with` over its bytes (including
    /// the terminator).
    fn with_string<T>(&self, with: impl FnOnce(&[u8]) -> T) -> T {
        if self.string.borrow().is_none() {
            let rendered = self.render();
            self.set_string(Some(rendered));
            self.canonical.set(true);
        }
        let string = self.string.borrow();
        with(string.as_deref().unwrap_or(&[0]))
    }

    /// The string rep as a C pointer plus its length in bytes (terminator
    /// excluded). Valid until the object is mutated or freed.
    pub fn c_string(&self) -> (*const c_char, usize) {
        self.with_string(|bytes| (bytes.as_ptr().cast::<c_char>(), bytes.len() - 1))
    }

    /// The string rep as Rust text.
    #[must_use]
    pub fn text(&self) -> String {
        self.with_string(|bytes| decode_bytes(&bytes[..bytes.len() - 1]))
    }

    /// Forget the string rep after a mutation of the internal rep.
    fn invalidate_string(&self) {
        self.set_string(None);
        self.canonical.set(true);
    }

    /// The value as a wide integer — `Tcl_GetWideIntFromObj`.
    pub fn get_wide(&self) -> Result<i64, TclError> {
        if let Rep::Int(value) = &*self.rep.borrow() {
            return Ok(*value);
        }
        let text = self.text();
        let flags = ParseFlags {
            integer_only: true,
            ..ParseFlags::default()
        };
        match number::parse_whole_with(&text, flags) {
            Some(Number::Int(value)) => {
                self.set_parsed_rep(Rep::Int(value));
                Ok(value)
            }
            Some(Number::Big { .. }) => Err(TclError::overflow()),
            Some(Number::Double(_) | Number::Nan { .. }) | None => {
                Err(TclError::expected("integer", &text))
            }
        }
    }

    /// The value as a double — `Tcl_GetDoubleFromObj`.
    pub fn get_double(&self) -> Result<f64, TclError> {
        match &*self.rep.borrow() {
            Rep::Double(value) => return Ok(*value),
            Rep::Int(value) => return Ok(int_to_double(*value)),
            Rep::None | Rep::List(_) | Rep::Index(_) => {}
        }
        let text = self.text();
        match number::parse_whole(&text) {
            Some(Number::Double(value)) => {
                self.set_parsed_rep(Rep::Double(value));
                Ok(value)
            }
            Some(Number::Int(value)) => Ok(int_to_double(value)),
            Some(Number::Big {
                negative,
                radix,
                digits,
            }) => Ok(big_to_double(negative, radix, &digits)),
            Some(Number::Nan { .. }) => Err(TclError::nan()),
            None => Err(TclError::expected("floating-point number", &text)),
        }
    }

    /// The value as a boolean — `Tcl_GetBooleanFromObj`: the boolean words,
    /// or any number compared against zero, with `NaN` a domain error.
    pub fn get_boolean(&self) -> Result<bool, TclError> {
        match &*self.rep.borrow() {
            Rep::Int(value) => return Ok(*value != 0),
            Rep::Double(value) if value.is_nan() => return Err(TclError::nan()),
            Rep::Double(value) => return Ok(*value != 0.0),
            Rep::None | Rep::List(_) | Rep::Index(_) => {}
        }
        let text = self.text();
        if matches!(number::parse_whole(&text), Some(Number::Nan { .. })) {
            return Err(TclError::nan());
        }
        tcl_syntax::boolean::truthiness(&text)
            .ok_or_else(|| TclError::expected("boolean value", &text))
    }

    /// Install a rep parsed from the string rep: the string stays
    /// authoritative.
    fn set_parsed_rep(&self, rep: Rep) {
        *self.rep.borrow_mut() = rep;
        self.canonical.set(false);
    }

    /// Ensure the list rep exists, parsing the string rep if needed.
    fn ensure_list(&self) -> Result<(), TclError> {
        if matches!(&*self.rep.borrow(), Rep::List(_)) {
            return Ok(());
        }
        let text = self.text();
        let elements = list::split_list(&text).map_err(|error| TclError::list(error, &text))?;
        let items = elements
            .into_iter()
            .map(|element| ObjRef::new(Obj::from_text(&element)))
            .collect();
        self.set_parsed_rep(Rep::List(items));
        Ok(())
    }

    /// Run `with` over the list elements — `Tcl_ListObjGetElements`.
    pub fn with_list<T>(&self, with: impl FnOnce(&[ObjRef]) -> T) -> Result<T, TclError> {
        self.ensure_list()?;
        let rep = self.rep.borrow();
        match &*rep {
            Rep::List(items) => Ok(with(items)),
            Rep::None | Rep::Int(_) | Rep::Double(_) | Rep::Index(_) => {
                unreachable!("ensure_list installed a list rep")
            }
        }
    }

    /// Append an element — `Tcl_ListObjAppendElement`. The caller has checked
    /// the object is unshared.
    pub fn append_element(&self, element: ObjRef) -> Result<(), TclError> {
        self.ensure_list()?;
        if let Rep::List(items) = &mut *self.rep.borrow_mut() {
            items.push(element);
        }
        self.invalidate_string();
        Ok(())
    }

    /// Record the option-table entry this value resolved to.
    pub fn set_index_entry(&self, entry: &str) {
        // Rendering first keeps the invariant that an index rep always sits
        // beside a string rep, which is what `render` relies on.
        self.with_string(|_| ());
        *self.rep.borrow_mut() = Rep::Index(Box::from(entry));
        self.canonical.set(false);
    }

    /// The option-table entry recorded by [`Self::set_index_entry`].
    #[must_use]
    pub fn index_entry(&self) -> Option<String> {
        match &*self.rep.borrow() {
            Rep::Index(entry) => Some(entry.to_string()),
            Rep::None | Rep::Int(_) | Rep::Double(_) | Rep::List(_) => None,
        }
    }

    /// A fresh, unshared copy — `Tcl_DuplicateObj`.
    #[must_use]
    pub fn duplicate(&self) -> Self {
        let rep = match &*self.rep.borrow() {
            Rep::None => Rep::None,
            Rep::Int(value) => Rep::Int(*value),
            Rep::Double(value) => Rep::Double(*value),
            Rep::List(items) => Rep::List(items.clone()),
            Rep::Index(entry) => Rep::Index(entry.clone()),
        };
        Self::with_rep(rep, self.string.borrow().clone(), self.canonical.get())
    }

    /// The value as the interface sees it: typed when the rep is
    /// authoritative, text otherwise.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let typed_is_authoritative = self.string.borrow().is_none() || self.canonical.get();
        if typed_is_authoritative {
            match &*self.rep.borrow() {
                Rep::Int(value) => return Value::Int(*value),
                Rep::Double(value) => return Value::Double(*value),
                Rep::List(items) => {
                    return Value::list(items.iter().map(|item| item.get().to_value()));
                }
                Rep::None | Rep::Index(_) => {}
            }
        }
        Value::string(self.text())
    }

    /// An object from an interface value, keeping its structure.
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Empty => Self::from_text(""),
            Value::Str(text) => Self::from_text(text),
            Value::Int(number) => Self::int(*number),
            Value::Double(number) => Self::double(*number),
            Value::List(items) => Self::list(
                items
                    .iter()
                    .map(|item| ObjRef::new(Self::from_value(item)))
                    .collect(),
            ),
            Value::Dict(entries) => Self::list(
                entries
                    .iter()
                    .flat_map(|(key, item)| [key, item])
                    .map(|part| ObjRef::new(Self::from_value(part)))
                    .collect(),
            ),
        }
    }
}

/// C's `(double)` conversion of a wide integer, correctly rounded. Goes
/// through decimal text rather than `as f64`: the same rounding, and no
/// precision-loss cast for the lint to flag.
fn int_to_double(value: i64) -> f64 {
    value.to_string().parse().unwrap_or(f64::NAN)
}

fn big_to_double(negative: bool, radix: Radix, digits: &str) -> f64 {
    let magnitude = match radix {
        Radix::Dec => digits.parse::<f64>().unwrap_or(f64::INFINITY),
        Radix::Bin | Radix::Oct | Radix::Hex => {
            let base = f64::from(radix as u8);
            digits
                .bytes()
                .filter_map(|digit| (digit as char).to_digit(16))
                .fold(0.0, |acc, digit| acc * base + f64::from(digit))
        }
    };
    if negative { -magnitude } else { magnitude }
}

/// An owning reference to an [`Obj`]: one unit of its reference count.
///
/// `#[repr(transparent)]` over the pointer so a `Vec<ObjRef>` is exactly the
/// `Tcl_Obj **` array `Tcl_ListObjGetElements` hands to C.
#[repr(transparent)]
pub struct ObjRef(NonNull<Obj>);

impl ObjRef {
    /// Allocate `obj` and take the first reference to it.
    #[must_use]
    pub fn new(obj: Obj) -> Self {
        // SAFETY: the pointer is freshly allocated and non-null.
        unsafe { Self::adopt(Obj::into_raw(obj)) }
    }

    /// Take a reference to an object C created or handed over
    /// (`Tcl_IncrRefCount` in Rust ownership terms).
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`].
    pub unsafe fn adopt(raw: *mut Obj) -> Self {
        let pointer = NonNull::new(raw).expect("a Tcl_Obj pointer is never null");
        // SAFETY: the caller guarantees the object is live.
        let obj = unsafe { pointer.as_ref() };
        obj.refcount.set(obj.refcount.get() + 1);
        Self(pointer)
    }

    /// The raw pointer, for handing to C.
    #[must_use]
    pub fn as_ptr(&self) -> *mut Obj {
        self.0.as_ptr()
    }

    /// The object.
    #[must_use]
    pub fn get(&self) -> &Obj {
        // SAFETY: an `ObjRef` holds one reference, so the object is live.
        unsafe { self.0.as_ref() }
    }
}

impl Clone for ObjRef {
    fn clone(&self) -> Self {
        // SAFETY: this reference keeps the object live.
        unsafe { Self::adopt(self.as_ptr()) }
    }
}

impl Drop for ObjRef {
    fn drop(&mut self) {
        // SAFETY: this reference is one unit of the count being released.
        unsafe { Obj::decr_ref_count(self.as_ptr()) }
    }
}

impl Obj {
    /// Move the object to the heap with a reference count of zero: the C
    /// convention for a value nobody holds yet.
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    /// Release one reference and free the object when none remain, as the
    /// header's `Tcl_DecrRefCount` macro does: a count of zero going down frees
    /// too.
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`]; it is
    /// dangling afterwards if this released the last reference.
    pub unsafe fn decr_ref_count(raw: *mut Self) {
        // SAFETY: the caller guarantees the object is live.
        let remaining = unsafe { &*raw }.refcount.get() - 1;
        // SAFETY: as above.
        unsafe { &*raw }.refcount.set(remaining);
        if remaining <= 0 {
            // SAFETY: no reference remains; the allocation came from `into_raw`.
            unsafe { Self::free(raw) };
        }
    }

    /// `TclFreeObj`: free an object whose count has fallen to zero. The header's
    /// `Tcl_DecrRefCount` macro lowers the count itself and calls this when the
    /// count was one.
    ///
    /// # Safety
    ///
    /// `raw` must point to a live object allocated by [`Obj::into_raw`] that no
    /// reference holds; it is dangling afterwards.
    pub unsafe fn free(raw: *mut Self) {
        // SAFETY: the caller guarantees the allocation came from `into_raw` and is
        // no longer referenced.
        drop(unsafe { Box::from_raw(raw) });
    }
}

/// Counts the objects destroyed on this thread, for the tests that need to see
/// an object freed.
#[cfg(test)]
impl Drop for Obj {
    fn drop(&mut self) {
        tests::DESTROYED.with(|destroyed| destroyed.set(destroyed.get() + 1));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{Obj, ObjRef};
    use tcl_engine_api::Value;

    thread_local! {
        /// How many objects have been destroyed on this thread.
        pub(super) static DESTROYED: Cell<usize> = const { Cell::new(0) };
    }

    // Over `void *`: an `extern` block naming `Obj`, whose private state is not
    // C's, would trip the FFI-safety lint.
    #[cfg(cshim_c_tests)]
    unsafe extern "C" {
        static tclshim_test_layout: [usize; 7];
        fn tclshim_test_bytes_of(obj: *mut std::ffi::c_void) -> *const std::ffi::c_char;
        fn tclshim_test_length_of(obj: *mut std::ffi::c_void) -> isize;
        fn tclshim_test_ref_count_of(obj: *mut std::ffi::c_void) -> isize;
        fn tclshim_test_shared_of(obj: *mut std::ffi::c_void) -> std::ffi::c_int;
        fn tclshim_test_incr_of(obj: *mut std::ffi::c_void);
        fn tclshim_test_decr_of(obj: *mut std::ffi::c_void);
    }

    #[cfg(cshim_c_tests)]
    fn c_obj(obj: *mut Obj) -> *mut std::ffi::c_void {
        obj.cast()
    }

    /// The prefix the header declares is the prefix `Obj` has: `Tcl_Obj`'s fields
    /// at their declared offsets, then room for the union, and only then the
    /// shim's own state.
    #[cfg(cshim_c_tests)]
    #[test]
    fn the_declared_prefix_is_the_layout_the_header_declares() {
        use std::mem::offset_of;
        // SAFETY: a constant array the C compiler emitted.
        let [
            size,
            ref_count,
            bytes,
            length,
            type_ptr,
            internal_rep,
            union_size,
        ] = unsafe { tclshim_test_layout };
        assert_eq!(offset_of!(Obj, refcount), ref_count);
        assert_eq!(offset_of!(Obj, bytes), bytes);
        assert_eq!(offset_of!(Obj, length), length);
        assert_eq!(offset_of!(Obj, type_ptr), type_ptr);
        assert_eq!(offset_of!(Obj, internal_rep), internal_rep);
        assert_eq!(
            size_of::<std::cell::Cell<[usize; 2]>>(),
            union_size,
            "room for the union"
        );
        assert_eq!(
            offset_of!(Obj, string),
            size,
            "the shim's own state starts where the declared struct ends"
        );
    }

    /// C reads `bytes` and `length` straight from the object: they follow the
    /// string rep, and are null and zero while there is none.
    #[cfg(cshim_c_tests)]
    #[test]
    fn the_fields_c_reads_follow_the_string_rep() {
        let raw = Obj::from_text("héllo").into_raw();
        // SAFETY: `raw` is a live object; the functions only read it.
        unsafe {
            assert_eq!(
                tclshim_test_length_of(c_obj(raw)),
                6,
                "bytes, not characters"
            );
            let bytes =
                std::slice::from_raw_parts(tclshim_test_bytes_of(c_obj(raw)).cast::<u8>(), 7);
            assert_eq!(bytes, "héllo\0".as_bytes());
            Obj::free(raw);
        }

        let number = Obj::int(42).into_raw();
        // SAFETY: as above.
        unsafe {
            assert!(
                tclshim_test_bytes_of(c_obj(number)).is_null(),
                "an int has no text yet"
            );
            assert_eq!(tclshim_test_length_of(c_obj(number)), 0);
            let (pointer, length) = (&*number).c_string();
            assert_eq!(tclshim_test_bytes_of(c_obj(number)), pointer);
            assert_eq!(
                tclshim_test_length_of(c_obj(number)),
                isize::try_from(length).expect("small")
            );
            assert_eq!(length, 2);
            Obj::free(number);
        }

        let list = Obj::list(vec![ObjRef::new(Obj::int(1)), ObjRef::new(Obj::int(22))]).into_raw();
        // SAFETY: as above; the element is a fresh object.
        unsafe {
            (&*list).with_string(|_| ());
            assert!(!tclshim_test_bytes_of(c_obj(list)).is_null());
            assert_eq!(tclshim_test_length_of(c_obj(list)), 4, "`1 22`");
            (&*list)
                .append_element(ObjRef::new(Obj::int(1)))
                .expect("a list takes an element");
            assert!(
                tclshim_test_bytes_of(c_obj(list)).is_null(),
                "changing the value withdrew the text C could read"
            );
            assert_eq!(tclshim_test_length_of(c_obj(list)), 0);
            Obj::free(list);
        }
    }

    /// The header's `Tcl_IncrRefCount`, `Tcl_DecrRefCount` and `Tcl_IsShared` are
    /// macros over `refCount`: they move the count the Rust side reads, and the
    /// last release frees through `TclFreeObj`.
    #[cfg(cshim_c_tests)]
    #[test]
    fn the_header_macros_move_the_count_rust_reads_and_the_last_release_frees() {
        let raw = Obj::int(1).into_raw();
        // SAFETY: `raw` is live until the last release below; `adopt` takes the
        // first reference.
        unsafe {
            let first = ObjRef::adopt(raw);
            assert_eq!(tclshim_test_ref_count_of(c_obj(raw)), 1);
            assert_eq!(tclshim_test_shared_of(c_obj(raw)), 0);
            tclshim_test_incr_of(c_obj(raw));
            assert_eq!(first.get().refcount(), 2);
            assert_eq!(tclshim_test_shared_of(c_obj(raw)), 1);
            tclshim_test_decr_of(c_obj(raw));
            assert_eq!(first.get().refcount(), 1);
            assert_eq!(tclshim_test_shared_of(c_obj(raw)), 0);

            let freed = DESTROYED.with(Cell::get);
            std::mem::forget(first);
            tclshim_test_decr_of(c_obj(raw));
            assert_eq!(
                DESTROYED.with(Cell::get),
                freed + 1,
                "the macro called TclFreeObj"
            );
        }
    }

    /// Tcl's macro frees at a count of one or less, so one release of an object
    /// nobody has taken a reference to frees it.
    #[cfg(cshim_c_tests)]
    #[test]
    fn a_single_release_of_a_fresh_object_through_the_macro_frees_it() {
        let raw = Obj::int(1).into_raw();
        let before = DESTROYED.with(Cell::get);
        // SAFETY: `raw` is live, and this call frees it.
        unsafe { tclshim_test_decr_of(c_obj(raw)) };
        assert_eq!(DESTROYED.with(Cell::get), before + 1);
    }

    #[test]
    fn a_rust_release_of_a_fresh_object_frees_it_as_in_c_tcl() {
        let raw = Obj::int(1).into_raw();
        let before = DESTROYED.with(Cell::get);
        // SAFETY: `raw` is live, and this call frees it.
        unsafe { Obj::decr_ref_count(raw) };
        assert_eq!(DESTROYED.with(Cell::get), before + 1);
    }

    #[test]
    fn a_rust_reference_and_a_c_macro_free_once_between_them() {
        let freed = DESTROYED.with(Cell::get);
        let first = ObjRef::new(Obj::int(1));
        let second = first.clone();
        drop(first);
        assert_eq!(
            DESTROYED.with(Cell::get),
            freed,
            "one reference is still held"
        );
        drop(second);
        assert_eq!(DESTROYED.with(Cell::get), freed + 1);
    }

    #[test]
    fn an_int_crosses_typed_and_renders_lazily() {
        let obj = Obj::int(42);
        assert!(matches!(obj.to_value(), Value::Int(42)));
        assert_eq!(obj.text(), "42");
        assert!(
            matches!(obj.to_value(), Value::Int(42)),
            "a rendered string is canonical"
        );
    }

    #[test]
    fn a_parsed_string_keeps_its_spelling() {
        let obj = Obj::from_text("0x10");
        assert_eq!(obj.get_wide().expect("hex parses"), 16);
        assert_eq!(obj.to_value().as_str(), Some("0x10"));
        assert_eq!(
            obj.get_double().expect("via the int rep").to_bits(),
            16.0_f64.to_bits()
        );
    }

    #[test]
    fn conversions_report_c_tcl_messages() {
        let bad = Obj::from_text("abc");
        let error = bad.get_wide().expect_err("not an integer");
        assert_eq!(error.message, "expected integer but got \"abc\"");
        assert_eq!(error.code.as_deref(), Some("TCL VALUE NUMBER"));
        let big = Obj::from_text("99999999999999999999");
        assert_eq!(
            big.get_wide().expect_err("overflows").message,
            "integer value too large to represent"
        );
        assert_eq!(
            big.get_double().expect("as a double").to_bits(),
            1e20_f64.to_bits()
        );
        let nan = Obj::from_text("NaN");
        assert_eq!(
            nan.get_double().expect_err("NaN").code.as_deref(),
            Some("TCL VALUE DOUBLE NAN")
        );
        assert!(
            Obj::from_text("1.5")
                .get_boolean()
                .expect("numbers are booleans")
        );
        assert_eq!(
            Obj::from_text("NaN")
                .get_boolean()
                .expect_err("NaN")
                .code
                .as_deref(),
            Some("TCL VALUE DOUBLE NAN")
        );
        assert_eq!(
            Obj::double(f64::NAN)
                .get_boolean()
                .expect_err("NaN rep")
                .message,
            "floating point value is Not a Number"
        );
        assert_eq!(
            Obj::from_text("").get_boolean().expect_err("empty").message,
            "expected boolean value but got \"\""
        );
    }

    #[test]
    fn lists_are_structural_until_text_is_authoritative() {
        let obj = Obj::list(vec![
            ObjRef::new(Obj::int(1)),
            ObjRef::new(Obj::from_text("b c")),
        ]);
        assert_eq!(obj.text(), "1 {b c}");
        assert_eq!(obj.to_value().as_list().map(<[Value]>::len), Some(2));
        let parsed = Obj::from_text("a  b");
        assert_eq!(parsed.with_list(<[ObjRef]>::len).expect("a list"), 2);
        assert_eq!(parsed.to_value().as_str(), Some("a  b"), "spacing survives");
        let bad = Obj::from_text("{a}b");
        assert_eq!(
            bad.with_list(<[ObjRef]>::len).expect_err("junk").message,
            "list element in braces followed by \"b\" instead of space"
        );
    }

    #[test]
    fn reference_counts_follow_ownership() {
        let raw = Obj::int(1).into_raw();
        // SAFETY: `raw` is live; `adopt` takes the first reference.
        let first = unsafe { ObjRef::adopt(raw) };
        assert!(!first.get().is_shared());
        let second = first.clone();
        assert!(second.get().is_shared());
        drop(first);
        assert!(!second.get().is_shared());
    }

    #[test]
    fn interior_nul_round_trips_through_modified_utf8() {
        let obj = Obj::from_text("a\0b");
        let (pointer, length) = obj.c_string();
        assert_eq!(length, 4);
        // SAFETY: the pointer addresses `length + 1` live bytes.
        let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), length) };
        assert_eq!(bytes, b"a\xC0\x80b");
        assert_eq!(obj.text(), "a\0b");
    }
}
