//! `ValueOps` for the VM — binds the portable `tcl-cmd-core` command logic to
//! the VM's `Rc<Obj>` value model.
//!
//! Value construction needs no interpreter state, so the seam is implemented
//! directly on [`Vm`] (the natural `ops` object a builtin already holds). The
//! copy-on-write asymmetry is explicit: the `Rc`-handle model cannot grow a
//! buffer in place, so [`ValueOps::try_append_bytes_in_place`] /
//! [`ValueOps::try_list_append_in_place`] keep their default (`false`) and
//! callers build a fresh value — the contrast with the WASM runtime's amortised
//! in-place growth that the contract is designed around. The VM's value is a
//! UTF-8 `Rc<str>`, so `as_bytes`/`new_bytes` use their (string-rep) defaults.

use std::rc::Rc;

use tcl_syntax::value::{ValueError, ValueOps};

use crate::interp::Vm;
use crate::value::Value;

impl ValueOps for Vm {
    type Value = Value;

    fn new_str(&mut self, s: &str) -> Value {
        Value::string(s)
    }

    fn new_string(&mut self, s: String) -> Value {
        Value::string(s)
    }

    fn new_int(&mut self, n: i64) -> Value {
        Value::int(n)
    }

    fn new_double(&mut self, f: f64) -> Value {
        Value::double(f)
    }

    fn new_bool(&mut self, b: bool) -> Value {
        Value::bool(b)
    }

    fn new_list(&mut self, items: Vec<Value>) -> Value {
        Value::list(items)
    }

    fn as_str(&mut self, v: &Value) -> Rc<str> {
        v.to_str()
    }

    fn as_int(&mut self, v: &Value) -> Result<i64, ValueError> {
        v.as_int()
            .map_err(|_| ValueError::NotInteger(v.to_str().to_string()))
    }

    fn as_double(&mut self, v: &Value) -> Result<f64, ValueError> {
        v.as_double()
            .map_err(|_| ValueError::NotDouble(v.to_str().to_string()))
    }

    fn as_bool(&mut self, v: &Value) -> Result<bool, ValueError> {
        v.as_bool()
            .map_err(|_| ValueError::NotBoolean(v.to_str().to_string()))
    }

    fn list_elements(&mut self, v: &Value) -> Result<Vec<Value>, ValueError> {
        v.as_list()
            .map(|items| items.as_ref().clone())
            .map_err(|e| ValueError::BadList(e.message))
    }
}
