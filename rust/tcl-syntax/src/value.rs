//! `ValueOps` — the value seam shared across Tcl runtimes.
//!
//! The evaluation parallel of [`crate::expr::ExprOps`], lifted from "evaluate an
//! `expr`" to "construct/inspect/shimmer a Tcl value". The pure command logic in
//! `tcl-cmd-core` (string/list/dict/…) is written **once**, generic over a
//! `ValueOps` implementor; each runtime plugs in its own value representation:
//!
//! - the **bytecode VM** over `Rc<Obj>` (cheap clone; copy-on-write list ops;
//!   `try_append_bytes_in_place` is always a no-op so the caller copies),
//! - the **WASM runtime** over `*mut TclObj` (24-byte C-ABI object; amortised
//!   in-place string growth via `try_append_bytes_in_place`).
//!
//! Two deliberate contract decisions (see
//! `docs/design/common-runtime-emitter-architecture.md` §4d and the red-team
//! findings):
//!
//! 1. **Char-correct strings.** [`ValueOps::as_str`] yields a UTF-8 `Rc<str>`,
//!    and all downstream indexing is by **character**, matching `tclsh`. A
//!    byte-oriented runtime conforms inside its own impl; the seam never exposes
//!    byte offsets. Byte-exact commands (`append`, `binary`) use the parallel
//!    [`ValueOps::as_bytes`]/[`ValueOps::new_bytes`] rung instead.
//! 2. **Copy-on-write is explicit, not implied.** The asymmetry between a runtime
//!    that can grow a buffer in place (when unshared) and one that always copies
//!    is encoded as the [`ValueOps::try_append_bytes_in_place`] /
//!    [`ValueOps::try_list_append_in_place`] capabilities (default: cannot),
//!    **not** as a hidden `strong_count` assumption.
//!
//! Coercion failures are a closed, runtime-agnostic set ([`ValueError`]) carrying
//! the canonical Tcl message, so a single shared body produces identical errors
//! across runtimes and `tcl-cmd-core` can lift them into its command error with
//! a plain `From`.

use std::rc::Rc;

/// A value-coercion / list-parse failure — the closed set Tcl reports with
/// canonical messages, independent of any runtime's value or error type.
///
/// Downstream command logic converts this into its command-level error (e.g.
/// `tcl_cmd_core::CmdError`) with a `From` impl; the canonical wording lives
/// here once via [`ValueError::message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueError {
    /// The value is not a wide integer (`expected integer but got "…"`).
    NotInteger(String),
    /// The value is not a float (`expected floating-point number but got "…"`).
    NotDouble(String),
    /// The value is not a boolean (`expected boolean value but got "…"`).
    NotBoolean(String),
    /// The value is not a well-formed list; carries the verbatim parser message
    /// (e.g. `unmatched open brace in list`).
    BadList(String),
    /// An integer operation overflowed the runtime's wide-integer range
    /// (`integer value too large to represent`). The bignum-capable runtime
    /// never raises this from [`ValueOps::int_add`] (it widens); the fixed-`i64`
    /// VM does.
    IntegerOverflow,
}

impl ValueError {
    /// The canonical Tcl error message for this coercion failure.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            ValueError::NotInteger(s) => format!("expected integer but got \"{s}\""),
            ValueError::NotDouble(s) => format!("expected floating-point number but got \"{s}\""),
            ValueError::NotBoolean(s) => format!("expected boolean value but got \"{s}\""),
            ValueError::BadList(msg) => msg.clone(),
            ValueError::IntegerOverflow => "integer value too large to represent".to_string(),
        }
    }
}

impl core::fmt::Display for ValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for ValueError {}

/// The value operations a Tcl command core supplies its runtime.
///
/// The shared command bodies in `tcl-cmd-core` drive these; construction,
/// shimmer caching, interning, and result-object building stay the runtime's
/// business (hence `&mut self`). Monomorphises per implementor — zero dynamic
/// dispatch, exactly like [`crate::expr::ExprOps`].
pub trait ValueOps {
    /// The runtime's value type (a cheap-to-clone handle).
    type Value: Clone;

    // -- construction / shimmer --

    /// A value from a borrowed string (copies).
    fn new_str(&mut self, s: &str) -> Self::Value;
    /// A value from an owned string (may avoid a copy).
    fn new_string(&mut self, s: String) -> Self::Value {
        self.new_str(&s)
    }
    /// The empty string value.
    fn empty(&mut self) -> Self::Value {
        self.new_str("")
    }
    /// A wide-integer value.
    fn new_int(&mut self, n: i64) -> Self::Value;
    /// A double value.
    fn new_double(&mut self, f: f64) -> Self::Value;
    /// A boolean value (string side canonicalises to `"0"`/`"1"`).
    fn new_bool(&mut self, b: bool) -> Self::Value;
    /// A list value from element handles.
    fn new_list(&mut self, items: Vec<Self::Value>) -> Self::Value;

    // -- string access (UTF-8; char-indexed downstream) --

    /// The string representation, generated and cached on first call
    /// (`Tcl_GetString`). Always valid UTF-8 — the seam never exposes bytes.
    fn as_str(&mut self, v: &Self::Value) -> Rc<str>;

    /// The character length (`string length` — counts code points, not bytes).
    fn char_len(&mut self, v: &Self::Value) -> usize {
        self.as_str(v).chars().count()
    }

    // -- numeric / boolean coercion (closed error set) --

    /// As a wide integer (`Tcl_GetWideIntFromObj`).
    fn as_int(&mut self, v: &Self::Value) -> Result<i64, ValueError>;
    /// As a double (`Tcl_GetDoubleFromObj`).
    fn as_double(&mut self, v: &Self::Value) -> Result<f64, ValueError>;
    /// As a boolean (`Tcl_GetBooleanFromObj`).
    fn as_bool(&mut self, v: &Self::Value) -> Result<bool, ValueError>;

    // -- integer arithmetic (the value-model boundary made explicit) --

    /// Integer sum `a + b` as a fresh value (`incr`'s arithmetic step), where a
    /// `None` left operand denotes an **absent value treated as zero** — `incr`
    /// of an unset variable starts at 0.
    ///
    /// This is a seam, not a convenience: the two runtimes have **different
    /// integer towers**. The default coerces both operands to `i64` and reports
    /// [`ValueError::IntegerOverflow`] on wrap — exactly the fixed-width VM's
    /// behaviour. A runtime with arbitrary-precision integers (the WASM runtime's
    /// bignum) overrides this to widen instead of overflowing, so `incr` shared
    /// in `tcl-cmd-core` stays faithful to each runtime's number model without
    /// the core ever naming a representation. Folding the unset → zero case into
    /// the seam keeps the shared core free of a throwaway zero value (the runtime
    /// would otherwise have to refcount-release it); each implementor supplies its
    /// own zero.
    fn int_add(
        &mut self,
        a: Option<&Self::Value>,
        b: &Self::Value,
    ) -> Result<Self::Value, ValueError> {
        let x = match a {
            Some(v) => self.as_int(v)?,
            None => 0,
        };
        let y = self.as_int(b)?;
        let sum = x.checked_add(y).ok_or(ValueError::IntegerOverflow)?;
        Ok(self.new_int(sum))
    }

    // -- list (copy-on-write) --

    /// The list elements (`Tcl_ListObjGetElements`), parsing+caching on first
    /// call. Element handles are cheap clones.
    fn list_elements(&mut self, v: &Self::Value) -> Result<Vec<Self::Value>, ValueError>;

    /// The list length (`llength`). Default walks [`ValueOps::list_elements`];
    /// impls with an O(1) length override.
    fn list_len(&mut self, v: &Self::Value) -> Result<usize, ValueError> {
        Ok(self.list_elements(v)?.len())
    }

    /// The element at `i` (`lindex`), or `None` if out of range. Default clones
    /// the element vector; impls with random access override.
    fn list_index(&mut self, v: &Self::Value, i: usize) -> Result<Option<Self::Value>, ValueError> {
        Ok(self.list_elements(v)?.into_iter().nth(i))
    }

    /// Append one element (`lappend` step), copy-on-write: the default rebuilds;
    /// an impl that owns the backing vector uniquely may mutate in place.
    fn list_append(
        &mut self,
        list: Self::Value,
        item: Self::Value,
    ) -> Result<Self::Value, ValueError> {
        let mut items = self.list_elements(&list)?;
        items.push(item);
        Ok(self.new_list(items))
    }

    // -- dict (a dict is an even-length list; keys compared by string rep) --

    /// The dict's **canonical** ordered key/value pairs (`Tcl_DictObjFirst`
    /// order: first-occurrence position, last value winning on a duplicate key).
    ///
    /// The default derives this from [`ValueOps::list_elements`] + the string
    /// rep — correct for any value model (the VM's list-backed dict and the WASM
    /// runtime's `TclDict`, which shimmers to a list). An impl with a native dict
    /// rep may override for efficiency. Errors with the canonical "missing value
    /// to go with key" when the list is odd-length.
    #[allow(clippy::type_complexity)] // a Self-dependent (key, value) pair vec
    fn dict_pairs(
        &mut self,
        v: &Self::Value,
    ) -> Result<Vec<(Self::Value, Self::Value)>, ValueError> {
        let elems = self.list_elements(v)?;
        if elems.len() % 2 != 0 {
            return Err(ValueError::BadList(
                "missing value to go with key".to_string(),
            ));
        }
        let mut keys: Vec<std::rc::Rc<str>> = Vec::new();
        let mut pairs: Vec<(Self::Value, Self::Value)> = Vec::new();
        for chunk in elems.chunks_exact(2) {
            let key = self.as_str(&chunk[0]);
            if let Some(pos) = keys.iter().position(|k| **k == *key) {
                pairs[pos].1 = chunk[1].clone(); // last value wins, keep position
            } else {
                keys.push(key);
                pairs.push((chunk[0].clone(), chunk[1].clone()));
            }
        }
        Ok(pairs)
    }

    /// Build a dict value from canonical key/value pairs. The default interleaves
    /// them into a list value (a dict *is* an even-length list); an impl with a
    /// native dict rep may override.
    fn new_dict(&mut self, pairs: Vec<(Self::Value, Self::Value)>) -> Self::Value {
        let mut items = Vec::with_capacity(pairs.len() * 2);
        for (k, v) in pairs {
            items.push(k);
            items.push(v);
        }
        self.new_list(items)
    }

    // -- bytes (byte-exact; the value-representation seam for append/binary) --

    /// The value's **raw bytes**, byte-exact — unlike [`as_str`](Self::as_str) it
    /// must not lose information for a value holding non-UTF-8 data. The default
    /// reuses the (UTF-8) string rep, correct for a string-only value model (the
    /// VM's `Rc<str>`); a byte-oriented runtime (the WASM `*mut TclObj`) overrides
    /// it to return the real bytes, so a shared `append` core stays byte-exact
    /// (`append data $binary` must not corrupt a byte > 127).
    fn as_bytes(&mut self, v: &Self::Value) -> Rc<[u8]> {
        Rc::from(self.as_str(v).as_bytes())
    }

    /// A value from raw bytes. The default routes through
    /// [`new_string`](Self::new_string) (lossy for non-UTF-8 on a string-only
    /// model, but such a runtime only ever builds from valid UTF-8); a byte
    /// runtime overrides it to be byte-exact.
    fn new_bytes(&mut self, bytes: &[u8]) -> Self::Value {
        self.new_string(String::from_utf8_lossy(bytes).into_owned())
    }

    // -- copy-on-write escape hatches (amortised in-place growth) --

    /// Try to append `bytes` to `v`'s value **in place** (amortised growth),
    /// returning whether it happened. A runtime whose object can be grown when
    /// unshared (the WASM `*mut TclObj`) overrides this; the default (the
    /// `Rc`-handle VM) returns `false`, signalling the caller to build a fresh
    /// value. This makes the COW asymmetry an explicit capability rather than a
    /// hidden `strong_count` assumption — and keeps `append` amortised O(1) per
    /// byte rather than O(n²) over a building loop.
    fn try_append_bytes_in_place(&mut self, _v: &mut Self::Value, _bytes: &[u8]) -> bool {
        false
    }

    /// Try to append `item` to `list`'s list value **in place** (the `lappend`
    /// analogue of [`try_append_bytes_in_place`](Self::try_append_bytes_in_place)),
    /// returning whether it happened. The default returns `false` (the VM
    /// rebuilds); a runtime owning the backing vector uniquely overrides it.
    fn try_list_append_in_place(&mut self, _list: &mut Self::Value, _item: &Self::Value) -> bool {
        false
    }
}
