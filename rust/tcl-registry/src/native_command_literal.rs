// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual generic C compiler command-literal registration and priming actions.

use tcl_runtime_api::native_command_name::{
    NativeCommandNamePriming, NativeCommandNamePrimingAuthority, NativeLiteralContext,
};
use tcl_runtime_api::native_compilation::{NativeCommandLookupUnavailable, NativeCompilationEntry};
use tcl_syntax::naming::{NamePolicyAuthority, NativeNameContext, NativeNameQualification};

use crate::native_compiler_words::NativeCompilerWords;

/// Selected actual native C command-name object protocol.
/// This selects pure rules and never grants a live binding or cache hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCommandNameProtocol(tcl_dialect::TclVersion);

impl NativeCommandNameProtocol {
    /// Actual selected object-type release.
    #[must_use]
    pub const fn version(self) -> tcl_dialect::TclVersion {
        self.0
    }

    /// Whether a transported descriptor has this exact native object-type origin.
    #[must_use]
    pub fn accepts_cache_origin(self, version: tcl_dialect::TclVersion) -> bool {
        self.0 == version
    }

    /// Validate actual command-node and referencing-namespace observations.
    /// A false answer requests original native lookup; it does not erase cmdName.
    #[must_use]
    pub fn cache_is_current(
        self,
        cache: &tcl_runtime_api::native_command_name::NativeCommandNameCache,
        state: &tcl_runtime_api::native_command_name::NativeCommandNameLookupState,
    ) -> bool {
        self.accepts_cache_origin(cache.version)
            && cache.interpreter == state.interpreter
            && cache
                .reference
                .is_none_or(|reference| reference == state.reference)
            && state.target.as_ref().is_some_and(|target| {
                (self.0 == tcl_dialect::TclVersion::V8_4 || !target.namespace_dying)
                    && cache.token == target.token
                    && cache.implementation_generation == target.implementation_generation
                    && cache.command_epoch == target.command_epoch
            })
    }

    /// Referencing namespace installed by compile-time `TclSetCmdNameObj`.
    #[must_use]
    pub fn priming_reference(
        self,
        absolute: bool,
        current: tcl_runtime_api::native_command_name::NativeCommandNameReference,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameReference> {
        (!absolute || self.0 == tcl_dialect::TclVersion::V8_4).then_some(current)
    }

    /// Referencing namespace installed by original `Tcl_GetCommandFromObj`.
    /// C8.4 executes absolute-name conversion in the actual global context.
    #[must_use]
    pub fn lookup_reference(
        self,
        absolute: bool,
        current: tcl_runtime_api::native_command_name::NativeCommandNameReference,
        global: tcl_runtime_api::native_command_name::NativeCommandNameReference,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameReference> {
        if absolute && self.0 == tcl_dialect::TclVersion::V8_4 {
            Some(global)
        } else {
            (!absolute).then_some(current)
        }
    }

    /// Failed actual name lookup installs a null cmdName descriptor only on C8.
    /// C9 preserves the existing primary representation when no named node exists.
    #[must_use]
    pub fn installs_unresolved_on_miss(self) -> bool {
        self.0 < tcl_dialect::TclVersion::V9_0
    }

    /// C8.5 object-command creation advances reverse-path reference epochs
    /// before resolving occupancy; later releases do so for fresh publication.
    #[must_use]
    pub fn invalidates_path_before_object_create(self) -> bool {
        self.0 == tcl_dialect::TclVersion::V8_5
    }

    /// `TclSetCmdNameObj`'s release-specific early return, independently of getters.
    #[must_use]
    pub fn preserves_primed_cache(
        self,
        existing: Option<&tcl_runtime_api::native_command_name::NativeCommandNameCache>,
        existing_unresolved: bool,
        incoming: &tcl_runtime_api::native_command_name::NativeCommandNameCache,
    ) -> bool {
        if self.0 <= tcl_dialect::TclVersion::V8_5 {
            existing.is_some() || existing_unresolved
        } else {
            existing.is_some_and(|cache| {
                cache.interpreter == incoming.interpreter && cache.token == incoming.token
            })
        }
    }
}

impl crate::InvocationDialect {
    /// Select the actual C command-name object protocol, independently of assistance
    /// and authored logical simulation. Unknown/Jim/vendor engines abstain.
    #[must_use]
    pub fn native_command_name_protocol(self) -> Option<NativeCommandNameProtocol> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => {
                Some(NativeCommandNameProtocol(version))
            }
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => None,
        }
    }
}

/// Retained generic command-head registration, independently of its cache action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCompiledCommandLiteral {
    /// Actual original compiler context, including its namespace incarnation.
    pub context: NativeLiteralContext,
    /// Complete original command-head value bytes.
    pub bytes: Vec<u8>,
    /// Absolute native lookup selects the release-specific root partition.
    pub fully_qualified: bool,
    /// C8.5 hides each generic command's single-word literal after priming.
    pub hide: bool,
    /// Named native binding found by the compiler; closed absence has no priming.
    pub priming: Option<NativeCommandNamePriming>,
}

/// Missing actual command-literal compilation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandLiteralUnavailable {
    /// Actual canonical C compiler/name policy or retained namespace is missing.
    Context,
    /// The actual command lookup cannot settle a named binding or closed absence.
    Lookup(NativeCommandLookupUnavailable),
}

/// Project the original generic compiler head using an independently retained
/// live lookup. The runtime supplies an exact native context and either the
/// selected raw node or closed absence; this function never performs lookup.
/// The returned actions grant no handler or compiler-hook authority.
pub fn native_compiled_command_literal_from_lookup(
    point: tcl_dialect::model::DialectPoint,
    names: tcl_syntax::naming::NamePolicyProtocol,
    context: NativeLiteralContext,
    words: &NativeCompilerWords<'_>,
    binding: Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
) -> Result<Option<NativeCompiledCommandLiteral>, NativeCommandLiteralUnavailable> {
    if names.authority() != NamePolicyAuthority::Native
        || tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(names)
    {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let version = names
        .recipe()
        .tcl_version()
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    if words
        .shapes()
        .first()
        .and_then(|shape| shape.compiler_head(Some(crate::InvocationDialect::for_version(version))))
        != Some(true)
    {
        return Ok(None);
    }
    let bytes = words
        .literal(0)
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let mut selected =
        native_compiled_command_name_literal_from_lookup(point, names, context, bytes, binding)?;
    selected.hide = version == tcl_dialect::TclVersion::V8_5 && words.shapes().len() == 1;
    Ok(Some(selected))
}

/// Register a compiler-selected name using the original issuer, namespace and
/// independently retained lookup. The supplied name is data, never new source.
/// The result owns only literal/cache actions, not dispatch or hook authority.
///
/// # Errors
/// Declines a mismatched actual naming issuer or unavailable namespace projection.
pub fn native_compiled_command_name_literal_from_lookup(
    point: tcl_dialect::model::DialectPoint,
    names: tcl_syntax::naming::NamePolicyProtocol,
    context: NativeLiteralContext,
    bytes: &[u8],
    binding: Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
) -> Result<NativeCompiledCommandLiteral, NativeCommandLiteralUnavailable> {
    if names.authority() != NamePolicyAuthority::Native
        || tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(names)
    {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let version = names
        .recipe()
        .tcl_version()
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let projection = names
        .recipe()
        .command_lookup_input(NativeNameContext::new(&context.namespace_path), bytes)
        .map_err(|_| NativeCommandLiteralUnavailable::Context)?;
    let fully_qualified = projection.qualification() == NativeNameQualification::Absolute;
    let priming = binding.map(|binding| NativeCommandNamePriming {
        context: context.clone(),
        original: bytes.into(),
        binding,
        version,
        fully_qualified,
        authority: NativeCommandNamePrimingAuthority::OriginalLookup,
    });
    Ok(NativeCompiledCommandLiteral {
        context,
        bytes: bytes.to_vec(),
        fully_qualified,
        hide: false,
        priming,
    })
}

/// Register a retained compiler-selected worker without looking up its reported
/// full name. The prerequisite owns the original public lookup/configuration
/// and selected worker; this result grants only literal allocation and priming.
///
/// # Errors
/// Declines mismatched interpreter, lookup context or missing selected worker.
pub fn native_compiled_selected_command_name_literal_from_lookup(
    point: tcl_dialect::model::DialectPoint,
    names: tcl_syntax::naming::NamePolicyProtocol,
    context: NativeLiteralContext,
    bytes: &[u8],
    prerequisite: tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
) -> Result<NativeCompiledCommandLiteral, NativeCommandLiteralUnavailable> {
    if prerequisite.interpreter != context.interpreter {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let binding = prerequisite
        .selected_worker
        .clone()
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let mut selected = native_compiled_command_name_literal_from_lookup(
        point,
        names,
        context,
        bytes,
        Some(binding),
    )?;
    selected
        .priming
        .as_mut()
        .ok_or(NativeCommandLiteralUnavailable::Context)?
        .authority = NativeCommandNamePrimingAuthority::CompilerSelected(Box::new(prerequisite));
    Ok(selected)
}

/// Select the native generic compiler's original command-head actions.
/// Dynamic heads return `Ok(None)`. An unknown lookup is an explicit residual,
/// rather than a fabricated cache-free registration. This query grants neither
/// dispatch permission nor a future command-cache hit.
///
/// # Errors
/// Returns unavailable actual compiler, name policy, namespace or lookup evidence.
pub fn native_compiled_command_literal(
    entry: &NativeCompilationEntry,
    words: &NativeCompilerWords<'_>,
) -> Result<Option<NativeCompiledCommandLiteral>, NativeCommandLiteralUnavailable> {
    let point = entry
        .execution_point
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let name = entry
        .name_protocol
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    if name.authority() != NamePolicyAuthority::Native
        || tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(name)
    {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let version = name
        .recipe()
        .tcl_version()
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    if words
        .shapes()
        .first()
        .and_then(|shape| shape.compiler_head(Some(crate::InvocationDialect::for_version(version))))
        != Some(true)
    {
        return Ok(None);
    }
    let bytes = words
        .literal(0)
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let mut selected = native_compiled_command_name_literal(entry, bytes)?;
    selected.hide = version == tcl_dialect::TclVersion::V8_5 && words.shapes().len() == 1;
    Ok(Some(selected))
}

/// Register a compiler-selected command name from the actual closed entry.
/// The caller must retain the original compiler selection that emits this name.
/// This receipt describes literal allocation and cache priming only; it grants
/// neither a handler, a compiler selection nor permission for future dispatch.
///
/// # Errors
/// Returns unavailable actual compiler, naming, namespace or closed lookup evidence.
pub fn native_compiled_command_name_literal(
    entry: &NativeCompilationEntry,
    bytes: &[u8],
) -> Result<NativeCompiledCommandLiteral, NativeCommandLiteralUnavailable> {
    let point = entry
        .execution_point
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    let name = entry
        .name_protocol
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    if name.authority() != NamePolicyAuthority::Native
        || tcl_syntax::naming::NamePolicyProtocol::for_native_point(point) != Some(name)
    {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let namespace = entry
        .namespaces
        .iter()
        .find(|row| row.token == entry.current_namespace)
        .ok_or(NativeCommandLiteralUnavailable::Context)?;
    if entry
        .namespaces
        .iter()
        .filter(|row| row.token == namespace.token)
        .count()
        != 1
    {
        return Err(NativeCommandLiteralUnavailable::Context);
    }
    let context = NativeLiteralContext {
        interpreter: entry.interpreter,
        namespace_token: entry.current_namespace,
        entry_epoch: entry.epoch,
        namespace_path: namespace.path.clone(),
    };
    let binding = entry
        .lookup_command_bytes(entry.current_namespace, bytes)
        .map_err(NativeCommandLiteralUnavailable::Lookup)?
        .cloned();
    native_compiled_command_name_literal_from_lookup(point, name, context, bytes, binding)
}

#[cfg(test)]
mod tests;
