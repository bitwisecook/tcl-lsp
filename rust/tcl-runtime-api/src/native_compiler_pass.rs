// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original interpreter environment for whole-unit native compiler passes.

use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

use crate::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};

/// Actual command-owning namespace of the selected original procedure.
/// Anonymous procedures and source declaration candidates have no such receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilerPassProcedure {
    /// Same interpreter and retained namespace incarnation.
    pub namespace: NativeNamespaceContext,
    /// Original native namespace full name used by the compiler's library test.
    /// This is a presentation comparison, never a lookup input.
    pub namespace_full_name: crate::NameBytes,
}

/// Metadata lifetime owner belonging to one actual interpreter state.
/// It owns no native headers, procedure declarations or namespace resources.
#[derive(Debug)]
pub struct NativeCompilerPassOwner {
    interpreter: NativeInterpreterIdentity,
    token: u64,
    lifetime: Arc<()>,
}

impl NativeCompilerPassOwner {
    /// Create an independent owner for the actual interpreter state.
    ///
    /// # Panics
    /// Panics if process-unique compiler environment identities are exhausted.
    #[must_use]
    pub fn new(interpreter: NativeInterpreterIdentity) -> Self {
        use std::sync::atomic::AtomicU64;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            interpreter,
            token: crate::checked_counter::allocate(&NEXT)
                .expect("native compiler pass owner identities exhausted"),
            lifetime: Arc::new(()),
        }
    }

    /// Capture independently observed parent, enabled limits and procedure owner.
    /// The backend must use its actual interpreter state, not a logical profile.
    #[must_use]
    pub fn capture(
        &self,
        root_interpreter: bool,
        command_limit_enabled: bool,
        time_limit_enabled: bool,
        procedure: Option<NativeCompilerPassProcedure>,
    ) -> NativeCompilerPassEnvironment {
        NativeCompilerPassEnvironment {
            interpreter: self.interpreter,
            token: self.token,
            root_interpreter,
            command_limit_enabled,
            time_limit_enabled,
            procedure,
            lifetime: Arc::downgrade(&self.lifetime),
        }
    }
}

/// Original compiler environment observed while its interpreter remains alive.
/// Capturing this receipt does not retain the interpreter or native resources.
#[derive(Debug, Clone)]
pub struct NativeCompilerPassEnvironment {
    interpreter: NativeInterpreterIdentity,
    token: u64,
    root_interpreter: bool,
    command_limit_enabled: bool,
    time_limit_enabled: bool,
    procedure: Option<NativeCompilerPassProcedure>,
    lifetime: Weak<()>,
}

impl NativeCompilerPassEnvironment {
    /// Same actual interpreter and unretired state owner.
    #[must_use]
    pub fn is_current_for(&self, interpreter: NativeInterpreterIdentity) -> bool {
        self.interpreter == interpreter
            && self.lifetime.strong_count() != 0
            && self
                .procedure
                .as_ref()
                .is_none_or(|procedure| procedure.namespace.interpreter == interpreter)
    }

    /// Actual interpreter has no parent.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.root_interpreter
    }

    /// Either native command-count or time limit is enabled.
    #[must_use]
    pub const fn has_enabled_limits(&self) -> bool {
        self.command_limit_enabled || self.time_limit_enabled
    }

    /// Actual selected original procedure's command-owning namespace, if any.
    #[must_use]
    pub const fn procedure(&self) -> Option<&NativeCompilerPassProcedure> {
        self.procedure.as_ref()
    }

    /// Retain the observed environment without borrowing a procedure owner.
    /// Source declaration candidates cannot claim the actual library exception.
    #[must_use]
    pub fn without_procedure(&self) -> Self {
        Self {
            procedure: None,
            ..self.clone()
        }
    }
}

impl PartialEq for NativeCompilerPassEnvironment {
    fn eq(&self, other: &Self) -> bool {
        self.interpreter == other.interpreter
            && self.token == other.token
            && self.root_interpreter == other.root_interpreter
            && self.command_limit_enabled == other.command_limit_enabled
            && self.time_limit_enabled == other.time_limit_enabled
            && self.procedure == other.procedure
    }
}
impl Eq for NativeCompilerPassEnvironment {}
impl Hash for NativeCompilerPassEnvironment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.interpreter.hash(state);
        self.token.hash(state);
        self.root_interpreter.hash(state);
        self.command_limit_enabled.hash(state);
        self.time_limit_enabled.hash(state);
        self.procedure.hash(state);
    }
}
