//! Dependency-free shared vocabulary for the Tcl runtimes and command cores.
//!
//! These are the value-less types every Tcl runtime agrees on regardless of its
//! value representation or execution model: the completion [`Code`], the generic
//! [`Completion`] container, and the opaque arena handles
//! ([`NsId`]/[`FrameId`]/[`CommandId`]/[`VarId`]).
//!
//! They live in their own leaf crate (depending on nothing) so that pure
//! command logic — `tcl-cmd-core`'s helpers — can name a completion code
//! without transitively pulling in `tcl-bytecode` (which `tcl-runtime-api` needs
//! for `CompileService`). See
//! `docs/design/common-runtime-emitter-architecture.md` (§6).

#![no_std]

/// A Tcl completion code (`tcl.h` `TCL_OK`..`TCL_CONTINUE`, plus arbitrary user
/// codes from `return -code N` / `try on N`).
///
/// The named variants are `0..=4`; [`Code::Other`] carries any other `int`
/// produced by `return -code N`. It propagates like an exception until a
/// `catch`/`try` reports it, and is never `0..=4` (those canonicalise to the
/// named variants via [`Code::from_int`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// `TCL_OK` — normal completion.
    Ok,
    /// `TCL_ERROR` — an error; `result` is the message, `options` the dict.
    Error,
    /// `TCL_RETURN` — a `return` propagating to the enclosing proc boundary.
    Return,
    /// `TCL_BREAK` — a loop `break`.
    Break,
    /// `TCL_CONTINUE` — a loop `continue`.
    Continue,
    /// A non-standard completion code (any `int` other than `0..=4`), produced
    /// by `return -code N`.
    Other(i32),
}

impl Code {
    /// The integer completion code (`TCL_OK` = 0 … `TCL_CONTINUE` = 4, or the
    /// raw value for [`Code::Other`]) — what `catch` returns and the `-code`
    /// options-dict entry uses.
    #[must_use]
    pub fn as_int(self) -> i64 {
        match self {
            Code::Ok => 0,
            Code::Error => 1,
            Code::Return => 2,
            Code::Break => 3,
            Code::Continue => 4,
            Code::Other(n) => i64::from(n),
        }
    }

    /// Map an integer completion code to a [`Code`]: `0..=4` to the named
    /// variants, anything else to [`Code::Other`] (`TclProcessReturn` /
    /// `TclGetCompletionCodeFromObj`).
    #[must_use]
    pub fn from_int(n: i32) -> Code {
        match n {
            0 => Code::Ok,
            1 => Code::Error,
            2 => Code::Return,
            3 => Code::Break,
            4 => Code::Continue,
            other => Code::Other(other),
        }
    }

    /// Whether this is `TCL_OK`.
    #[must_use]
    pub fn is_ok(self) -> bool {
        matches!(self, Code::Ok)
    }
}

/// A command/script completion: a code, the result value, and the return
/// options dict. The "result is not a bare string" contract — every dispatch
/// yields this. Generic over the runtime's value type `V`.
#[derive(Debug, Clone)]
pub struct Completion<V> {
    /// The completion code.
    pub code: Code,
    /// The result value (the message when `code == Error`).
    pub result: V,
    /// The return-options dict (carries `-code`/`-level`/`-errorinfo`/…).
    pub options: V,
}

impl<V> Completion<V> {
    /// Construct a completion from its parts.
    pub fn new(code: Code, result: V, options: V) -> Self {
        Self {
            code,
            result,
            options,
        }
    }

    /// Whether the completion is `TCL_OK`.
    pub fn is_ok(&self) -> bool {
        self.code.is_ok()
    }
}

// -- Opaque handles --
//
// Arena indices into the runtime's storage; they cross the trait boundary, the
// concrete storage behind them does not.

/// A namespace handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NsId(pub u32);

/// A call-frame handle (absolute level / arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(pub usize);

/// A command handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u32);

/// A variable-cell handle (arena id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

/// The global call frame (level 0).
pub const GLOBAL_FRAME: FrameId = FrameId(0);
/// The root (`::`) namespace handle.
pub const ROOT_NS: NsId = NsId(0);
