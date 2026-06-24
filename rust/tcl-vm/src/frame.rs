//! The call-frame stack and per-frame variable storage.
//!
//! Frame 0 is the global scope. Each frame holds local variables; a [`Local`]
//! is either a scalar value or a cross-frame [`Local::Link`] (the
//! `upvar`/`global`/`variable` alias). Name resolution follows links to the
//! owning frame.

use std::collections::{BTreeMap, HashMap};

use tcl_runtime_api::NsId;

use crate::value::Value;

/// A variable cell in a frame: a scalar, an array, or a link to another frame's
/// variable.
pub(crate) enum Local {
    /// A scalar value owned by this frame.
    Scalar(Value),
    /// An associative array (element key → value). `BTreeMap` gives a
    /// deterministic `array names`/`array get` order.
    Array(BTreeMap<String, Value>),
    /// An alias to `name` in frame `level` (`upvar`/`global`/`variable`).
    Link {
        /// Target frame level.
        level: usize,
        /// Target variable name within that frame.
        name: String,
    },
}

/// One call frame.
pub(crate) struct CallFrame {
    /// Local variables by name.
    pub locals: HashMap<String, Local>,
    /// Names (within this frame) declared `const` — immutable scalars (TIP 677).
    /// Dropped with the frame, so a proc-local constant lasts one activation.
    pub consts: std::collections::HashSet<String>,
    /// The namespace this frame executes in (currently global-only).
    #[allow(dead_code)]
    pub ns: NsId,
    /// Absolute frame level (0 = global).
    pub level: usize,
    /// The proc this frame belongs to (for `errorInfo`/`info level`); `None` at
    /// top level.
    pub proc_name: Option<String>,
    /// The invocation argv (proc name + args) — used by `info level N`.
    pub call_argv: Vec<Value>,
    /// For a `namespace eval`/`inscope` body frame, the canonical namespace it
    /// runs in (no leading `::`; `""` = global). `None` for proc activations and
    /// the global frame. An unqualified variable accessed in such a frame is a
    /// *namespace* variable (`ns::name` in the global frame), not a local — see
    /// [`Vm::locate_from`](crate::interp::Vm). This is what makes `uplevel`/
    /// `upvar` into a namespace-eval body resolve to namespace variables.
    pub ns_eval: Option<String>,
}

impl CallFrame {
    /// A fresh frame at `level` in namespace `ns`.
    pub fn new(level: usize, ns: NsId, proc_name: Option<String>, call_argv: Vec<Value>) -> Self {
        Self {
            locals: HashMap::new(),
            consts: std::collections::HashSet::new(),
            ns,
            level,
            proc_name,
            call_argv,
            ns_eval: None,
        }
    }
}
