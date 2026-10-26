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

/// Captured declaration binding policy, independent of the active host policy.
/// Adapter frames and persistent bindings need their own alias contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginalProcedureBindingContext {
    /// Formal parsing and binding grammar retained at declaration creation.
    pub parameter_grammar: tcl_dialect::ParameterGrammar,
    /// The original declaration retains persistent variable bindings.
    pub has_static_bindings: bool,
    /// A method, lambda or interpreter adapter supplies additional frame context.
    pub has_adapter_bindings: bool,
}

/// Inputs selected by a backend from its original declaration at preparation.
/// Only its live native role ledger can bind these into a retained request.
pub struct OriginalProcedureCompileInput {
    /// Exact whole body image obtained from the original body object.
    pub original: SourceImage,
    /// Actual bound formal storage keys in declaration order.
    pub parameters: Vec<NameBytes>,
    /// Actual declaration's captured formal and frame binding policy.
    pub binding_context: OriginalProcedureBindingContext,
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
    binding_context: OriginalProcedureBindingContext,
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
            && self.binding_context == other.binding_context
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
    ordinary_frame_initialization: Option<OrdinaryFrameInitialization>,
}

#[derive(Debug, Clone)]
struct OrdinaryFrameInitialization {
    version: tcl_dialect::TclVersion,
    observation: crate::native_compiler_pass::OrdinaryEntryObservation,
}

// Policy projection only. The caller independently checks the actual engine,
// request owner and current ordinary-entry observation. Match the existing
// InvocationDialect::execution_point recipe without borrowing registry authority.
fn ordinary_initialization_policy_version(
    profile: &tcl_dialect::DialectProfile,
) -> Option<tcl_dialect::TclVersion> {
    use tcl_dialect::model::Family;
    let family = profile.runtime_core_family();
    let version = profile.runtime_version();
    if let Some(point) = profile.core_point {
        if family.is_some_and(|family| family != point.family())
            || (point.family() == Family::Tcl
                && version.is_some_and(|version| point.tcl_version() != Some(version)))
        {
            return None;
        }
        return (point.family() == Family::Tcl)
            .then_some(point.tcl_version())
            .flatten();
    }
    (family == Some(Family::Tcl)).then_some(version).flatten()
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
            binding_context: input.binding_context,
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
            ordinary_frame_initialization: None,
        })
    }

    /// Attach the ordinary C frame initialization algorithm selected by the
    /// actual backend at this exact declaration/body preparation boundary.
    /// This is an initialization plan, never an entered frame or local value.
    /// A different compiler owner cannot attach a plan to this request.
    #[must_use]
    pub fn with_ordinary_frame_initialization(
        mut self,
        owner: &crate::native_compiler_pass::NativeCompilerPassOwner,
        version: tcl_dialect::TclVersion,
    ) -> Self {
        self.ordinary_frame_initialization = None;
        if self.is_live()
            && self
                .issuer
                .same_owner(&owner.procedure_compilation_identity())
            && self.ordinary_formals().is_some()
        {
            self.ordinary_frame_initialization =
                owner
                    .ordinary_entry_generation()
                    .map(|observation| OrdinaryFrameInitialization {
                        version,
                        observation,
                    });
        }
        self
    }

    /// The same actual request and current observer-free environment carry
    /// the selected ordinary C initialization plan. The caller independently
    /// proves empty actual formals and the complete body/store induction.
    #[must_use]
    pub fn ordinary_frame_initialization_for(
        &self,
        entry: &NativeCompilationEntry,
        version: tcl_dialect::TclVersion,
    ) -> bool {
        let actual_c_version = |point: tcl_dialect::model::DialectPoint| {
            if point.family() == tcl_dialect::model::Family::Tcl {
                point.tcl_version()
            } else {
                None
            }
        };
        let actual_engine = entry.execution_point.and_then(actual_c_version);
        let actual_policy = entry
            .invocation_policy
            .and_then(|policy| ordinary_initialization_policy_version(policy.profile()));
        actual_engine == Some(version)
            && actual_policy == Some(version)
            && self
                .ordinary_frame_initialization
                .as_ref()
                .is_some_and(|plan| plan.version == version && plan.observation.is_current())
            && self.matches_entry(entry)
            && self.ordinary_formals().is_some()
            && entry
                .compiler_pass_environment
                .as_ref()
                .is_some_and(|environment| {
                    environment.ordinary_entry_observers_absent_for(entry.interpreter)
                        && !environment.has_enabled_limits()
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

    /// Captured declaration policy; this is independent of the active compiler entry.
    #[must_use]
    pub fn binding_context(&self) -> &OriginalProcedureBindingContext {
        &self.source.binding_context
    }

    /// Actual Tcl formal storage keys without persistent or adapter bindings.
    /// The caller still checks active compilation grammar, byte support and
    /// its original entry. This does not establish physical local-slot access.
    #[must_use]
    pub fn ordinary_formals(&self) -> Option<&[NameBytes]> {
        let context = self.binding_context();
        (self.is_live()
            && context.parameter_grammar == tcl_dialect::ParameterGrammar::Tcl
            && !context.has_static_bindings
            && !context.has_adapter_bindings)
            .then_some(self.source.parameters.as_slice())
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

#[cfg(test)]
mod ordinary_initialization_policy_tests {
    use super::ordinary_initialization_policy_version;
    use tcl_dialect::model::DialectPoint;
    use tcl_dialect::{DialectProfile, TclVersion};

    #[test]
    fn static_c_policy_releases_resolve_without_borrowing_foreign_or_conflicting_points() {
        for (name, version) in [
            ("tcl8.4", TclVersion::V8_4),
            ("tcl8.6", TclVersion::V8_6),
            ("tcl9.0", TclVersion::V9_0),
            ("tcl9.1", TclVersion::V9_1),
        ] {
            let profile = DialectProfile::find(name).unwrap();
            assert!(profile.core_point.is_none());
            assert_eq!(
                ordinary_initialization_policy_version(profile),
                Some(version)
            );
            let mut explicit = profile.clone();
            explicit.core_point = Some(DialectPoint::for_tcl_version(version));
            assert_eq!(
                ordinary_initialization_policy_version(&explicit),
                Some(version)
            );
            let other = if version == TclVersion::V8_6 {
                TclVersion::V9_0
            } else {
                TclVersion::V8_6
            };
            explicit.core_point = Some(DialectPoint::for_tcl_version(other));
            assert_eq!(ordinary_initialization_policy_version(&explicit), None);
        }
        let logical = DialectProfile::find("f5-irules").unwrap();
        assert!(logical.runtime_version().is_some());
        assert_eq!(ordinary_initialization_policy_version(logical), None);
        let mut contradictory = logical.clone();
        contradictory.core_point = Some(DialectPoint::for_tcl_version(TclVersion::V8_4));
        assert_eq!(ordinary_initialization_policy_version(&contradictory), None);
        assert_eq!(
            ordinary_initialization_policy_version(DialectProfile::plain_tcl()),
            None
        );
    }
}
