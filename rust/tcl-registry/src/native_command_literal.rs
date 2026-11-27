// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual generic C compiler command-literal registration and priming actions.

use tcl_runtime_api::native_command_name::{
    NativeCommandNamePriming, NativeCommandNamePrimingAuthority, NativeLiteralContext,
};
use tcl_runtime_api::native_compilation::NativeCompilationEntry;
use tcl_syntax::naming::{NamePolicyAuthority, NativeNameContext, NativeNameQualification};

use crate::native_compiler_words::NativeCompilerWords;
use tcl_native_model::native_command_literal::{
    NativeCommandLiteralUnavailable, NativeCompiledCommandLiteral,
};

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
    if words.shapes().first().and_then(|shape| {
        shape.compiler_head(Some(tcl_native_model::InvocationDialect::for_version(
            version,
        )))
    }) != Some(true)
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
    if words.shapes().first().and_then(|shape| {
        shape.compiler_head(Some(tcl_native_model::InvocationDialect::for_version(
            version,
        )))
    }) != Some(true)
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

#[cfg(test)]
mod original_deletion_epoch_tests {
    #[test]
    fn original_command_deletion_epoch_predicates_keep_outer_recursive_and_unlink_stages() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Pure source predicates only; no Native callbacks/cache observations.
        use tcl_dialect::TclVersion;
        for (version, outer, recursive, final_unlink, nested_unlink, namespace) in [
            (TclVersion::V8_4, true, false, false, false, false),
            (TclVersion::V8_5, true, true, false, false, true),
            (TclVersion::V8_6, true, true, true, false, true),
            (TclVersion::V9_0, false, false, true, true, true),
            (TclVersion::V9_1, false, false, true, true, true),
        ] {
            let protocol = tcl_native_model::InvocationDialect::for_version(version)
                .native_command_name_protocol()
                .unwrap();
            assert_eq!(protocol.deletion_epoch_before_trace(false), outer);
            assert_eq!(protocol.deletion_epoch_before_trace(true), recursive);
            assert_eq!(protocol.deletion_epoch_after_unlink(false), final_unlink);
            assert_eq!(protocol.deletion_epoch_after_unlink(true), nested_unlink);
            assert_eq!(
                protocol.deletion_invalidates_namespace_after_trace(),
                namespace
            );
        }
        let jim = tcl_native_model::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").analyser_profile(),
        );
        assert!(jim.native_command_name_protocol().is_none());
    }
}
