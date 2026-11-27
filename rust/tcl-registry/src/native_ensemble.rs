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

//! Compiler-only selection from an actual ensemble configuration.

use crate::native_compilation::NativeCompilationWordShape;
use tcl_core_types::NameBytes;
use tcl_dialect::{TclVersion, model::Family};
use tcl_native_model::InvocationDialect;
use tcl_native_model::native_ensemble::{
    NativeEnsembleMemberSelection, NativeEnsembleWorkerSelection,
};
use tcl_runtime_api::native_compilation::NativeEnsembleCompiler;

/// Original parser-selected ensemble member, retaining literal expansion geometry.
/// The selector addresses the actual C compiler's flattened original vector;
/// source word indices and mapped worker reporting names are not substituted.
///
/// # Errors
/// Returns unavailable original parser geometry or an unknown native compiler.
pub fn original_ensemble_selector(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    dialect: InvocationDialect,
) -> Result<
    Option<crate::native_compiler_word_projection::NativeProjectedCompilerWord>,
    crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable,
> {
    original_ensemble_selector_at(words, 1, dialect)
}

/// Original flattened member at one reached nested ensemble compiler depth.
/// This preserves the parser's source operand and literal expansion geometry.
///
/// # Errors
/// Returns unavailable original parser geometry or an unknown native compiler.
pub fn original_ensemble_selector_at(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand: usize,
    dialect: InvocationDialect,
) -> Result<
    Option<crate::native_compiler_word_projection::NativeProjectedCompilerWord>,
    crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable,
> {
    let version = dialect.tcl_version.filter(|_| dialect.family() == Some(Family::Tcl))
        .ok_or(crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable::SourceGeometry)?;
    let projected =
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)?;
    Ok(projected.get(operand).cloned())
}

/// Resolve a configured worker through its original namespace lookup owner.
/// The callback consumes the configured name once, before original operands;
/// the resulting binding must be retained independently of its reporting name.
#[must_use]
pub fn select_worker(
    configuration: &NativeEnsembleCompiler,
    member: Option<&[u8]>,
    shape: Option<NativeCompilationWordShape>,
    dialect: Option<InvocationDialect>,
    mut lookup: impl FnMut(
        u64,
        &[u8],
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
) -> NativeEnsembleWorkerSelection {
    match select_member(configuration, member, shape, dialect) {
        NativeEnsembleMemberSelection::Generic => NativeEnsembleWorkerSelection::Generic,
        NativeEnsembleMemberSelection::Unknown => NativeEnsembleWorkerSelection::Unknown,
        NativeEnsembleMemberSelection::Worker { member, command } => {
            lookup(configuration.namespace_token, command.as_bytes()).map_or(
                NativeEnsembleWorkerSelection::Unknown,
                |binding| NativeEnsembleWorkerSelection::Worker {
                    member,
                    binding: Box::new(binding),
                },
            )
        }
    }
}

/// Select a canonical member through its original map-target object owner.
/// The callback receives source-independent map coordinates, never a reported
/// worker name. It must resolve the original head in the retained namespace.
#[must_use]
pub fn select_worker_from_original_target(
    configuration: &NativeEnsembleCompiler,
    member: Option<&[u8]>,
    shape: Option<NativeCompilationWordShape>,
    dialect: Option<InvocationDialect>,
    mut lookup: impl FnMut(
        u64,
        &NameBytes,
        usize,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
) -> NativeEnsembleWorkerSelection {
    match select_mapping(configuration, member, shape, dialect) {
        MappingSelection::Generic => NativeEnsembleWorkerSelection::Generic,
        MappingSelection::Unknown => NativeEnsembleWorkerSelection::Unknown,
        MappingSelection::Worker { member, .. } => lookup(configuration.namespace_token, member, 0)
            .map_or(NativeEnsembleWorkerSelection::Unknown, |binding| {
                NativeEnsembleWorkerSelection::Worker {
                    member: member.clone(),
                    binding: Box::new(binding),
                }
            }),
    }
}

/// Select the configured worker from the same immutable native entry.
/// An open, conflicting or unavailable namespace lookup remains unknown.
#[must_use]
pub fn select_worker_in_entry(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    ensemble_token: u64,
    configuration: &NativeEnsembleCompiler,
    member: Option<&[u8]>,
    shape: Option<NativeCompilationWordShape>,
    dialect: Option<InvocationDialect>,
) -> NativeEnsembleWorkerSelection {
    use NativeEnsembleWorkerSelection as Result;
    match select_mapping(configuration, member, shape, dialect) {
        MappingSelection::Generic => Result::Generic,
        MappingSelection::Unknown => Result::Unknown,
        MappingSelection::Worker { member, prefix } => original_worker_in_entry(
            entry,
            ensemble_token,
            configuration,
            member,
            prefix[0].as_ref(),
            dialect,
        )
        .map_or(Result::Unknown, |binding| Result::Worker {
            member: member.clone(),
            binding: Box::new(binding.clone()),
        }),
    }
}

fn original_worker_in_entry<'a>(
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
    ensemble_token: u64,
    configuration: &NativeEnsembleCompiler,
    member: &NameBytes,
    metadata_name: Option<&NameBytes>,
    dialect: Option<InvocationDialect>,
) -> Option<&'a tcl_runtime_api::native_compilation::NativeCompilationBinding> {
    use tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary;
    let rows = entry
        .ensemble_target_objects
        .as_ref()?
        .iter()
        .filter(|row| {
            row.ensemble_token == ensemble_token && &row.member == member && row.prefix_index == 0
        })
        .collect::<Vec<_>>();
    let [observation] = rows.as_slice() else {
        return None;
    };
    if observation.object_identity == 0 || observation.resident_name.as_ref() != metadata_name {
        return None;
    }
    match &observation.primary {
        NativeEnsembleTargetPrimary::Unavailable => return None,
        NativeEnsembleTargetPrimary::StockString => {}
        NativeEnsembleTargetPrimary::CommandName {
            origin,
            cache,
            lookup,
        } => {
            let protocol = dialect?.native_command_name_protocol()?;
            if !protocol.accepts_cache_origin(*origin) {
                return None;
            }
            if let Some(cache) = cache {
                let lookup = lookup.as_ref()?;
                if lookup.interpreter != entry.interpreter
                    || lookup.reference.namespace_token != configuration.namespace_token
                {
                    return None;
                }
                if protocol.cache_is_current(cache, lookup) {
                    let mut nodes = entry.commands.iter().filter(|node| {
                        node.token == cache.token
                            && node.implementation_generation == cache.implementation_generation
                            && lookup.target.as_ref().is_some_and(|target| {
                                node.namespace_token == target.namespace_token
                            })
                    });
                    let binding = nodes.next()?;
                    return nodes.next().is_none().then_some(binding);
                }
            }
        }
    }
    entry
        .lookup_command_bytes(
            configuration.namespace_token,
            observation.resident_name.as_ref()?.as_bytes(),
        )
        .ok()
        .flatten()
}

/// Select an actual configured member using the native ensemble compiler's rules.
#[must_use]
pub fn select_member(
    configuration: &NativeEnsembleCompiler,
    member: Option<&[u8]>,
    shape: Option<NativeCompilationWordShape>,
    dialect: Option<InvocationDialect>,
) -> NativeEnsembleMemberSelection {
    match select_mapping(configuration, member, shape, dialect) {
        MappingSelection::Generic => NativeEnsembleMemberSelection::Generic,
        MappingSelection::Unknown => NativeEnsembleMemberSelection::Unknown,
        MappingSelection::Worker { member, prefix } => match &prefix[0] {
            Some(name) => NativeEnsembleMemberSelection::Worker {
                member: member.clone(),
                command: name.clone(),
            },
            None => NativeEnsembleMemberSelection::Unknown,
        },
    }
}

enum MappingSelection<'a> {
    Generic,
    Worker {
        member: &'a NameBytes,
        prefix: &'a [Option<NameBytes>],
    },
    Unknown,
}

fn select_mapping<'a>(
    configuration: &'a NativeEnsembleCompiler,
    member: Option<&[u8]>,
    shape: Option<NativeCompilationWordShape>,
    dialect: Option<InvocationDialect>,
) -> MappingSelection<'a> {
    use MappingSelection as Selection;
    let Some(dialect) = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl)) else {
        return Selection::Unknown;
    };
    let Some(version) = dialect
        .tcl_version
        .filter(|version| *version >= TclVersion::V8_5)
    else {
        return Selection::Unknown;
    };
    if !matches!(
        shape,
        Some(
            NativeCompilationWordShape::Literal
                | NativeCompilationWordShape::QuotedLiteral
                | NativeCompilationWordShape::BracedLiteral
        )
    ) || !configuration.parameters.is_empty()
    {
        return Selection::Generic;
    }
    let Some(member) = member else {
        return Selection::Generic;
    };
    let allowed = |name: &NameBytes| {
        configuration
            .subcommands
            .as_ref()
            .is_none_or(|names| names.iter().any(|item| item.as_bytes() == name.as_bytes()))
    };
    let exact = configuration
        .map
        .iter()
        .filter(|(name, _)| name.as_bytes() == member && allowed(name))
        .collect::<Vec<_>>();
    let rows = if exact.is_empty() && configuration.prefixes {
        configuration
            .map
            .iter()
            .filter(|(name, _)| name.as_bytes().starts_with(member) && allowed(name))
            .collect::<Vec<_>>()
    } else {
        exact
    };
    let [(member, prefix)] = rows.as_slice() else {
        return Selection::Generic;
    };
    if prefix.len() != 1 {
        return if version >= TclVersion::V8_6 {
            Selection::Generic
        } else {
            Selection::Unknown
        };
    }
    Selection::Worker { member, prefix }
}

/// Whether an actual worker without a compiler is emitted as a captured-name call.
#[must_use]
pub fn no_hook_worker_is_named(dialect: Option<InvocationDialect>) -> Option<bool> {
    let dialect = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl))?;
    let version = dialect
        .tcl_version
        .filter(|version| *version >= TclVersion::V8_5)?;
    Some(version >= TclVersion::V8_6)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_byte_members_preserve_exact_identity_and_prefix_ambiguity() {
        let first = NameBytes::from(&b"m\0\xff"[..]);
        let second = NameBytes::from(&b"m\0\xfe"[..]);
        let worker = NameBytes::from(&b"::w\0\xff"[..]);
        let mut configuration = NativeEnsembleCompiler {
            namespace_token: 7,
            map: vec![
                (first.clone(), vec![Some(worker.clone())]),
                (second.clone(), vec![Some(worker.clone())]),
            ],
            subcommands: None,
            prefixes: true,
            parameters: Vec::new(),
            unknown_handler: None,
        };
        let dialect = Some(InvocationDialect::for_version(TclVersion::V9_0));
        let shape = Some(NativeCompilationWordShape::Literal);
        assert_eq!(
            select_member(&configuration, Some(first.as_bytes()), shape, dialect),
            NativeEnsembleMemberSelection::Worker {
                member: first.clone(),
                command: worker.clone()
            }
        );
        assert_eq!(
            select_member(&configuration, Some(b"m\0"), shape, dialect),
            NativeEnsembleMemberSelection::Generic
        );
        configuration.subcommands = Some(vec![first.clone()]);
        assert_eq!(
            select_member(&configuration, Some(b"m\0"), shape, dialect),
            NativeEnsembleMemberSelection::Worker {
                member: first,
                command: worker
            }
        );
        assert_eq!(
            select_member(&configuration, Some(second.as_bytes()), shape, dialect),
            NativeEnsembleMemberSelection::Generic
        );
        assert_eq!(
            select_member(&configuration, Some(b"m"), None, dialect),
            NativeEnsembleMemberSelection::Generic
        );
        assert_eq!(
            select_member(&configuration, Some(b"m"), shape, None),
            NativeEnsembleMemberSelection::Unknown
        );
    }
}
