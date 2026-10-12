// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native literal and command-name cache identities, without callable ownership.

use crate::native_compilation::{NativeCompilationBinding, NativeInterpreterIdentity};
use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};

/// Actual compilation context of a registered command literal.
/// The path is a reporting/checking field; it cannot replace the namespace token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeLiteralContext {
    /// Interpreter that supplied the original compilation entry.
    pub interpreter: NativeInterpreterIdentity,
    /// Never-reused namespace incarnation in which compilation occurs.
    pub namespace_token: u64,
    /// Mutation epoch of the retained compilation entry.
    pub entry_epoch: u64,
    /// Constructed path observed for this same namespace incarnation.
    pub namespace_path: ByteNamespacePath,
}

/// Selected native compiler's command-name priming action.
/// This records a lookup already performed at compilation, independently of
/// which allocation first occupied the locally deduplicated literal slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandNamePriming {
    /// Original native compilation context.
    pub context: NativeLiteralContext,
    /// Original complete command-head bytes, before `CString` name projection.
    pub original: NameBytes,
    /// Actual named registration selected by the original compiler lookup.
    pub binding: NativeCompilationBinding,
    /// Actual native C compiler release.
    pub version: tcl_dialect::TclVersion,
    /// Original selected name has a leading root qualifier.
    pub fully_qualified: bool,
    /// Original lookup or separately retained compiler-selected worker authority.
    pub authority: NativeCommandNamePrimingAuthority,
}

/// Admission purpose of a compiler command-name cache action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeCommandNamePrimingAuthority {
    /// The original literal bytes supplied the original command lookup.
    OriginalLookup,
    /// The compiler already selected a worker from this exact registration.
    /// Reporting bytes must never be used to look the worker up again.
    CompilerSelected(Box<crate::native_compilation::NativeCommandCompilerPrerequisite>),
}

impl NativeCommandNamePriming {
    /// Validate this original compiler action against the same immutable entry.
    /// Compiler-selected workers are checked by node/configuration identity;
    /// their reported full-name bytes never become a new lookup operand.
    #[must_use]
    pub fn matches_entry(&self, entry: &crate::NativeCompilationEntry) -> bool {
        if self.context.interpreter != entry.interpreter
            || self.context.entry_epoch != entry.epoch
            || !entry
                .namespaces
                .iter()
                .filter(|namespace| namespace.token == self.context.namespace_token)
                .map(|namespace| &namespace.path)
                .eq(std::iter::once(&self.context.namespace_path))
        {
            return false;
        }
        match &self.authority {
            NativeCommandNamePrimingAuthority::OriginalLookup => {
                entry
                    .lookup_command_bytes(self.context.namespace_token, self.original.as_bytes())
                    .ok()
                    .flatten()
                    == Some(&self.binding)
            }
            NativeCommandNamePrimingAuthority::CompilerSelected(required) => {
                required.interpreter == entry.interpreter
                    && required.selected_worker.as_ref() == Some(&self.binding)
                    && required
                        .matches_registration_with(|namespace, word| {
                            entry.lookup_command_bytes(namespace, word.as_bytes()).map(
                            Option::<&crate::native_compilation::NativeCompilationBinding>::cloned,
                        )
                        })
                        .unwrap_or(false)
                    && entry
                        .commands
                        .iter()
                        .filter(|binding| binding.token == self.binding.token)
                        .eq(std::iter::once(&self.binding))
            }
        }
    }
}

/// Referencing namespace of a native command-name cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCommandNameReference {
    /// Never-reused namespace incarnation.
    pub namespace_token: u64,
    /// Actual command-reference epoch of this namespace.
    pub command_reference_epoch: u64,
}

/// Native command-name primary cache, independent of its worker/body lifetime.
/// Retaining this record never retains a procedure, bytecode, or callable.
/// A consumer must validate interpreter, command epoch and reference context
/// against its actual command world before using the named registration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandNameCache {
    /// Interpreter owning the original native command node.
    pub interpreter: NativeInterpreterIdentity,
    /// Actual native C object-type origin.
    pub version: tcl_dialect::TclVersion,
    /// Captured named command's structured placement.
    pub slot: NativeByteCommandSlot,
    /// Never-reused namespace incarnation owning the named command.
    pub namespace_token: u64,
    /// Original command node identity.
    pub token: u64,
    /// Implementation incarnation observed for that node.
    pub implementation_generation: u64,
    /// Actual rename/delete/hide/expose epoch of the command node.
    pub command_epoch: u64,
    /// Relative lookup context; absolute names carry no referencing namespace.
    pub reference: Option<NativeCommandNameReference>,
}

/// Live named-command node observed by the actual lookup owner.
/// No callable/body is retained by this value-only observation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandNameTarget {
    /// Actual original command node identity.
    pub token: u64,
    /// Actual worker incarnation, distinct from reporting bytes.
    pub implementation_generation: u64,
    /// Actual command rename/delete/hide/expose epoch.
    pub command_epoch: u64,
    /// Namespace incarnation currently containing this node.
    pub namespace_token: u64,
    /// Native `NS_DYING` state prohibits a cached command hit.
    pub namespace_dying: bool,
}

/// Actual world observations needed to validate a named-command cache hit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandNameLookupState {
    /// Interpreter doing the lookup.
    pub interpreter: NativeInterpreterIdentity,
    /// Actual current namespace and its command-reference epoch.
    pub reference: NativeCommandNameReference,
    /// Current original node, or absence after binding retirement.
    pub target: Option<NativeCommandNameTarget>,
}
