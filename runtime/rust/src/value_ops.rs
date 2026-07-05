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

//! `ValueOps` for the WASM runtime — binds the portable `tcl-cmd-core` command
//! logic to the runtime's 24-byte C-ABI `*mut TclObj` value model.
//!
//! This is the **opposite** value model from the bytecode VM's `Rc<Obj>`:
//! manually refcounted raw objects over the shared linear memory. The same
//! shared command helpers run over it unchanged — that is the entire point of
//! the value seam. Coercion reuses `tcl_syntax::number`, so it is byte-for-byte
//! identical to the VM's `ValueOps`.
//!
//! The copy-on-write asymmetry the contract is designed around is visible here:
//! [`ValueOps::try_append_str_in_place`] performs the runtime's amortised
//! in-place string growth when the object is an unshared plain string (the
//! EXP-STRING decision in `cmd_string.rs`), whereas the VM always copies.

use std::rc::Rc;

use tcl_syntax::number::{self, Number};
use tcl_syntax::value::{ValueError, ValueOps};

use crate::interp::{obj_bytes, Interp};
use crate::list;
use crate::obj::{self, TclObj};

impl ValueOps for Interp {
    type Value = *mut TclObj;

    fn new_str(&mut self, s: &str) -> *mut TclObj {
        obj::new_string_bytes(s.as_bytes())
    }

    fn new_string(&mut self, s: String) -> *mut TclObj {
        obj::new_string_bytes(s.as_bytes())
    }

    fn new_int(&mut self, n: i64) -> *mut TclObj {
        obj::new_wide_int_obj(n)
    }

    fn new_double(&mut self, f: f64) -> *mut TclObj {
        obj::new_double_obj(f)
    }

    fn new_bool(&mut self, b: bool) -> *mut TclObj {
        obj::new_boolean_obj(i32::from(b))
    }

    fn new_list(&mut self, items: Vec<*mut TclObj>) -> *mut TclObj {
        list::new_list_obj(&items)
    }

    fn as_str(&mut self, v: &*mut TclObj) -> Rc<str> {
        Rc::from(String::from_utf8_lossy(&obj_bytes(*v)).as_ref())
    }

    fn as_int(&mut self, v: &*mut TclObj) -> Result<i64, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n),
            _ => Err(ValueError::NotInteger(s.into_owned())),
        }
    }

    fn as_double(&mut self, v: &*mut TclObj) -> Result<f64, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        match number::parse_whole(s.trim()) {
            Some(Number::Int(n)) => Ok(n as f64),
            Some(Number::Double(f)) => Ok(f),
            _ => Err(ValueError::NotDouble(s.into_owned())),
        }
    }

    fn as_bool(&mut self, v: &*mut TclObj) -> Result<bool, ValueError> {
        let bytes = obj_bytes(*v);
        let s = String::from_utf8_lossy(&bytes);
        let t = s.trim();
        if let Some(num) = number::parse_whole(t) {
            return Ok(match num {
                Number::Int(n) => n != 0,
                Number::Double(f) => f != 0.0,
                _ => true,
            });
        }
        match t.to_ascii_lowercase().as_str() {
            "true" | "yes" | "on" => Ok(true),
            "false" | "no" | "off" => Ok(false),
            _ => Err(ValueError::NotBoolean(s.into_owned())),
        }
    }

    /// Bignum-aware integer addition (the `incr` step) — overrides the default's
    /// fixed-`i64` path. Both operands must read as integers (a wide or a bignum;
    /// a float or non-number is the canonical "expected integer" error). The sum
    /// goes over the numeric tower, so it **widens to a bignum on overflow**
    /// instead of erroring — Tcl integers never wrap — and demotes back to a wide
    /// when it fits. A `None` left operand is an unset variable, treated as 0.
    ///
    /// Only when the libtommath FFI is linked (`have_tommath`). A build without it
    /// (the wasm32 target) keeps the trait's fixed-`i64` default — overflow then
    /// errors, exactly like the bytecode VM, since there is no bignum tower.
    #[cfg(have_tommath)]
    fn int_add(
        &mut self,
        a: Option<&*mut TclObj>,
        b: &*mut TclObj,
    ) -> Result<*mut TclObj, ValueError> {
        fn not_int(obj: *mut TclObj) -> ValueError {
            ValueError::NotInteger(String::from_utf8_lossy(&obj_bytes(obj)).into_owned())
        }
        // Coercion order matches C's `TclIncrObj`: the current value first, then
        // the increment (both must read as an integer — a wide or a bignum).
        if let Some(av) = a {
            if !crate::bignum::is_integer(*av) {
                return Err(not_int(*av));
            }
        }
        if !crate::bignum::is_integer(*b) {
            return Err(not_int(*b));
        }
        // Both integers → tower add (never fails for two integer operands except
        // on allocation failure, mapped to the overflow error). The left operand
        // is a transient zero when the variable was unset.
        match a {
            Some(av) => crate::bignum::add(*av, *b).map_err(|_| ValueError::IntegerOverflow),
            None => {
                let zero = obj::new_wide_int_obj(0);
                let sum = crate::bignum::add(zero, *b).map_err(|_| ValueError::IntegerOverflow);
                crate::interp::drop_fresh(zero);
                sum
            }
        }
    }

    fn list_elements(&mut self, v: &*mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
        list::list_elements(*v)
            .map_err(|e| ValueError::BadList(String::from_utf8_lossy(e.message()).into_owned()))
    }

    /// Byte-exact, unlike the lossy `as_str` — this is why `append` can route
    /// through the shared core without corrupting a value's non-UTF-8 bytes.
    fn as_bytes(&mut self, v: &*mut TclObj) -> Rc<[u8]> {
        Rc::from(obj_bytes(*v).as_slice())
    }

    /// Byte-exact construction (the `obj::new_string_bytes` path).
    fn new_bytes(&mut self, bytes: &[u8]) -> *mut TclObj {
        obj::new_string_bytes(bytes)
    }

    fn try_append_bytes_in_place(&mut self, v: &mut *mut TclObj, bytes: &[u8]) -> bool {
        // Amortised O(1) growth when the object is an unshared plain string —
        // the EXP-STRING in-place path the VM cannot take (it always copies).
        if obj::is_plain_string(*v) && !obj::is_shared(*v) {
            obj::string_append_inplace(*v, bytes);
            true
        } else {
            false
        }
    }

    fn try_list_append_in_place(&mut self, list: &mut *mut TclObj, item: &*mut TclObj) -> bool {
        // Mutate the list's backing vector in place when it is uniquely owned —
        // the COW fast path the runtime's hand-rolled `lappend` used. A shared or
        // non-list value falls back to a rebuild (the caller copies). `list_append`
        // validates the list first, so a malformed value leaves it untouched.
        if obj::is_shared(*list) {
            return false;
        }
        crate::list::list_append(*list, *item).is_ok()
    }
}
