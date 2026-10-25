// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original declaration custody at an actual procedure compilation boundary.

use std::cell::OnceCell;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use crate::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};
use crate::{NameBytes, NativeCompilationEntry, SourceImage};

/// Inputs selected by a backend from its original declaration at preparation.
/// Only its live native role ledger can bind these into a retained request.
pub struct OriginalProcedureCompileInput {
    /// Exact whole body image obtained from the original body object.
    pub original: SourceImage,
    /// Actual bound formal storage keys in declaration order.
    pub parameters: Vec<NameBytes>,
    /// Actual constructed procedure namespace and its retained incarnation.
    pub namespace: NativeNamespaceContext,
    /// Complete prefix selected by the original compiler's parse plan.
    pub complete_prefix_len: usize,
    /// Reporting only; never a lookup or declaration identity.
    pub reported_name: String,
}

#[derive(Debug)]
pub(crate) struct OriginalProcedureSource {
    original: SourceImage,
    parameters: Vec<NameBytes>,
    namespace: NativeNamespaceContext,
    issuer: crate::native_compiler_pass::NativeCompilerPassIdentity,
    current: AtomicBool,
}

impl OriginalProcedureSource {
    pub(crate) fn withdraw(&self) {
        self.current.store(false, Ordering::Release);
    }

    fn matches(&self, other: &Self) -> bool {
        self.original == other.original
            && self.parameters == other.parameters
            && self.namespace == other.namespace
    }
}

/// Opaque identity of an actual native declaration. Cloning it retains no
/// native role and cannot postpone retirement of body objects or defaults.
#[derive(Debug, Clone)]
pub struct NativeProcedureDeclarationIdentity {
    lifetime: Weak<AtomicBool>,
}

impl NativeProcedureDeclarationIdentity {
    /// The original declaration still has at least one genuine native role.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.lifetime
            .upgrade()
            .is_some_and(|state| state.load(Ordering::Acquire))
    }
}

impl PartialEq for NativeProcedureDeclarationIdentity {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.lifetime, &other.lifetime)
    }
}
impl Eq for NativeProcedureDeclarationIdentity {}
impl Hash for NativeProcedureDeclarationIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.lifetime.as_ptr().hash(state);
    }
}

/// An immutable request issued by an actual declaration's native role owner.
/// The parser-selected prefix is derived here from the retained whole image;
/// neither an independently supplied target nor a copied declaration can join it.
#[derive(Debug, Clone)]
pub struct OriginalProcedureCompileRequest {
    declaration: NativeProcedureDeclarationIdentity,
    issuer: crate::native_compiler_pass::NativeCompilerPassIdentity,
    source: Arc<OriginalProcedureSource>,
    prefix: SourceImage,
    reported_name: String,
}

impl OriginalProcedureCompileRequest {
    pub(crate) fn from_ledger(
        retained: &OnceCell<Arc<OriginalProcedureSource>>,
        issuer: crate::native_compiler_pass::NativeCompilerPassIdentity,
        lifetime: Weak<AtomicBool>,
        input: OriginalProcedureCompileInput,
    ) -> Result<Self, OriginalProcedureCompileError> {
        if input.namespace.interpreter != issuer.interpreter {
            return Err(OriginalProcedureCompileError::ForeignIssuer);
        }
        let prefix = input
            .original
            .bytes()
            .get(..input.complete_prefix_len)
            .ok_or(OriginalProcedureCompileError::InvalidPrefix)?;
        let prefix = SourceImage::from_bytes(prefix, input.original.channel());
        let incoming = OriginalProcedureSource {
            original: input.original,
            parameters: input.parameters,
            namespace: input.namespace,
            issuer: issuer.clone(),
            current: AtomicBool::new(true),
        };
        if let Some(original) = retained.get() {
            if !original.issuer.same_owner(&issuer) {
                return Err(OriginalProcedureCompileError::ForeignIssuer);
            }
            if !original.current.load(Ordering::Acquire) || !original.matches(&incoming) {
                original.withdraw();
                return Err(OriginalProcedureCompileError::ChangedOriginal);
            }
        }
        let source = Arc::clone(retained.get_or_init(|| Arc::new(incoming)));
        Ok(Self {
            declaration: NativeProcedureDeclarationIdentity { lifetime },
            issuer,
            source,
            prefix,
            reported_name: input.reported_name,
        })
    }

    /// Borrow only the source, formals and namespace owned by this request.
    #[must_use]
    pub fn target(&self) -> crate::ProcedureCompileTargetBytes<'_> {
        crate::ProcedureCompileTargetBytes {
            source: &self.prefix,
            parameters: &self.source.parameters,
            namespace: &self.source.namespace.path,
        }
    }

    /// Whole original body, including an uncompiled malformed tail if present.
    #[must_use]
    pub fn original_source(&self) -> &SourceImage {
        &self.source.original
    }

    /// Actual declaration allocation; reporting names do not affect its identity.
    #[must_use]
    pub const fn declaration(&self) -> &NativeProcedureDeclarationIdentity {
        &self.declaration
    }

    /// Interpreter that supplied this original declaration.
    #[must_use]
    pub fn interpreter(&self) -> NativeInterpreterIdentity {
        self.source.namespace.interpreter
    }

    /// Reporting name only, independent of current command lookup.
    #[must_use]
    pub fn reported_name(&self) -> &str {
        &self.reported_name
    }

    /// The original declaration, compiler owner and captured source context remain current.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.declaration.is_live()
            && self.issuer.is_live()
            && self.source.current.load(Ordering::Acquire)
    }

    /// Relate the original body to the supplied entry's exact compiler owner
    /// and namespace incarnation. The caller must authenticate that entry;
    /// command, observer and operation currency retain their existing checks.
    #[must_use]
    pub fn matches_entry(&self, entry: &NativeCompilationEntry) -> bool {
        self.interpreter() == entry.interpreter
            && entry.current_namespace == self.source.namespace.token
            && entry
                .retained_namespace_context(self.source.namespace.token)
                .is_ok_and(|namespace| namespace == self.source.namespace)
            && self.matches_compiler_environment(entry.compiler_pass_environment.as_ref())
    }

    pub(crate) fn matches_compiler_environment(
        &self,
        environment: Option<&crate::native_compiler_pass::NativeCompilerPassEnvironment>,
    ) -> bool {
        self.is_live()
            && environment.is_some_and(|environment| self.issuer.matches_environment(environment))
    }
}

/// Why a backend could not retain original declaration custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalProcedureCompileError {
    /// The declaration has no live native role.
    Retired,
    /// This declaration previously supplied different original inputs.
    ChangedOriginal,
    /// The parser-selected prefix exceeds the whole original body.
    InvalidPrefix,
    /// The actual compiler owner belongs to another interpreter.
    ForeignIssuer,
}

impl std::fmt::Display for OriginalProcedureCompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Retired => "original procedure declaration has no live native role",
            Self::ChangedOriginal => "original procedure compilation inputs changed",
            Self::InvalidPrefix => "original procedure prefix exceeds its body",
            Self::ForeignIssuer => {
                "original procedure compiler owner belongs to another interpreter"
            }
        })
    }
}
impl std::error::Error for OriginalProcedureCompileError {}
