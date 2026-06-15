//! The call-frame stack and per-frame variable storage.
//!
//! Frame 0 is the global scope. Each frame holds local variables; a [`Local`]
//! is either a scalar value or a cross-frame [`Local::Link`] (the
//! `upvar`/`global`/`variable` alias). Name resolution follows links to the
//! owning frame, mirroring `tooling/vm/scope.py::CallFrame._resolve`.

use std::collections::HashMap;

use tcl_runtime_api::NsId;

use crate::value::Value;

/// A variable cell in a frame: a scalar, or a link to another frame's variable.
pub(crate) enum Local {
    /// A scalar value owned by this frame.
    Scalar(Value),
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
    /// The namespace this frame executes in (global-only for M2).
    #[allow(dead_code)]
    pub ns: NsId,
    /// Absolute frame level (0 = global).
    #[allow(dead_code)]
    pub level: usize,
    /// The proc this frame belongs to (for `errorInfo`/`info level`); `None` at
    /// top level. Retained now so the `info` family (M3) needs no rework.
    #[allow(dead_code)]
    pub proc_name: Option<String>,
    /// The invocation argv (proc name + args) — retained for `info level N`.
    /// Kept now so the `info` family (M3) needs no frame-model rework; this is
    /// exactly the metadata whose absence made `info.test` painful in the WASM
    /// work.
    #[allow(dead_code)]
    pub call_argv: Vec<Value>,
}

impl CallFrame {
    /// A fresh frame at `level` in namespace `ns`.
    pub fn new(level: usize, ns: NsId, proc_name: Option<String>, call_argv: Vec<Value>) -> Self {
        Self {
            locals: HashMap::new(),
            ns,
            level,
            proc_name,
            call_argv,
        }
    }
}
