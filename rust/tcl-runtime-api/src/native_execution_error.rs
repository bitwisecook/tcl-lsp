// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host refusals are independent of every Tcl completion code.

use crate::NativeCompilationAdmissionError;

/// A reached expression needs a genuine provider rather than a recovery AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExpressionRefusal {
    /// The selected expression grammar is outside the represented parser domain.
    UnsupportedGrammar,
    /// Syntax definitely fails, but its exact native presentation is not proved.
    UnpresentedSyntaxFailure,
    /// Original function topology has no independently installed dispatch policy.
    FunctionDispatchPolicyUnavailable,
}

/// Immutable diagnostic context at the reached expression boundary.
/// This is not an executable continuation or permission to replay prior effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpressionFailure {
    /// Why a genuine expression provider is required.
    pub reason: NativeExpressionRefusal,
    /// Actual source bytes resolved before parser entry.
    pub source: crate::SourceImage,
    /// Full source lexer/operator/host profile, independent of native policy.
    pub source_profile: tcl_dialect::DialectProfileKey,
    /// Full selected native engine/profile, including release and build policy.
    pub native_profile: tcl_dialect::DialectProfileKey,
    /// Interpreter identity, including the owning VM domain.
    pub interpreter: crate::native_compilation::NativeInterpreterIdentity,
    /// Logical frame at refusal. Not a lease on a future activation.
    pub frame: usize,
    /// Constructed namespace display at refusal.
    pub namespace: std::sync::Arc<str>,
    /// Namespace token distinguishes deletion/recreation at the same name.
    pub namespace_token: u64,
}

/// Immutable context where a compile service could not honour its contract.
/// This diagnostic is not an executable continuation or a permission to replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompileServiceRefusal {
    /// Exact operational diagnostic supplied by the compile service.
    pub reason: String,
    /// Actual source bytes handed to that compile-service request.
    pub source: crate::SourceImage,
    /// Target namespace handed to the compile service, which may differ from
    /// the active namespace before a procedure frame is entered.
    pub compilation_namespace: crate::ByteNamespacePath,
    /// Actual requested compiler entry, independent of the active variable frame.
    pub scope: crate::NativeCompilationAdmissionScope,
    /// Source lexer/operator/host profile selected by the caller.
    pub source_profile: tcl_dialect::DialectProfileKey,
    /// Actual selected native engine/profile, independent of lexical grammar.
    pub native_profile: tcl_dialect::DialectProfileKey,
    /// Interpreter including its owning VM domain.
    pub interpreter: crate::native_compilation::NativeInterpreterIdentity,
    /// Actual logical frame at refusal, without retaining an activation lease.
    pub frame: usize,
    /// Exact constructed namespace path at refusal.
    pub namespace: crate::ByteNamespacePath,
    /// Stable namespace token distinguishes deletion and recreation.
    pub namespace_token: u64,
}

/// Diagnostic context for a reached host command's explicit execution refusal.
/// It describes the stopped boundary, without a resumable continuation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHostCommandRefusal {
    /// Exact diagnostic supplied by the host command.
    pub reason: String,
    /// Selected source lexer/operator/host policy at the reached boundary.
    pub source_profile: tcl_dialect::DialectProfileKey,
    /// Actual selected native engine/profile.
    pub native_profile: tcl_dialect::DialectProfileKey,
    /// Owning VM and current interpreter identity.
    pub interpreter: crate::native_compilation::NativeInterpreterIdentity,
    /// Logical frame, without a lease on a later activation.
    pub frame: usize,
    /// Current constructed namespace display.
    pub namespace: std::sync::Arc<str>,
    /// Actual namespace token, distinguishing name reuse.
    pub namespace_token: u64,
}

/// Neutral host execution failure. Guest `catch` and `try` cannot observe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExecutionError {
    /// An unentered artifact requires native compiler admission.
    CompilationAdmission(NativeCompilationAdmissionError),
    /// The compile service cannot provide the requested execution capability.
    CompileServiceRefusal(Box<NativeCompileServiceRefusal>),
    /// A reached expression cannot be executed faithfully by this engine.
    ExpressionRefusal(Box<NativeExpressionFailure>),
    /// A reached native value operation lacks its actual execution capability.
    ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal),
    /// A host command explicitly refused its requested execution capability.
    HostCommandRefusal(Box<NativeHostCommandRefusal>),
}

impl From<NativeCompilationAdmissionError> for NativeExecutionError {
    fn from(error: NativeCompilationAdmissionError) -> Self {
        Self::CompilationAdmission(error)
    }
}

impl std::fmt::Display for NativeExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilationAdmission(error) => error.fmt(formatter),
            Self::CompileServiceRefusal(failure) => formatter.write_str(&failure.reason),
            Self::HostCommandRefusal(failure) => formatter.write_str(&failure.reason),
            Self::ValueAccessRefusal(failure) => failure.fmt(formatter),
            Self::ExpressionRefusal(failure) => formatter.write_str(match failure.reason {
                NativeExpressionRefusal::UnsupportedGrammar => {
                    "native expression provider required for unsupported grammar"
                }
                NativeExpressionRefusal::UnpresentedSyntaxFailure => {
                    "native expression provider required for error presentation"
                }
                NativeExpressionRefusal::FunctionDispatchPolicyUnavailable => {
                    "expression function dispatch policy is unavailable"
                }
            }),
        }
    }
}

impl std::error::Error for NativeExecutionError {}
