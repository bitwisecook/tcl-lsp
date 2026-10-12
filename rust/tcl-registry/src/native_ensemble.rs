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

use crate::InvocationDialect;
use crate::native_compilation::NativeCompilationWordShape;
use tcl_core_types::NameBytes;
use tcl_dialect::{TclVersion, model::Family};
use tcl_runtime_api::native_compilation::NativeEnsembleCompiler;

/// Selected C ensemble configuration tables and native member-name extent.
/// This recipe describes the issuer, not catalogue command availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEnsembleConfigurationProtocol {
    version: TclVersion,
    parameters: bool,
    dying_namespace: bool,
}

impl InvocationDialect {
    /// Select authentic Tcl 8.5+ ensemble object configuration semantics.
    #[must_use]
    pub fn native_ensemble_configuration_protocol(
        self,
    ) -> Option<NativeEnsembleConfigurationProtocol> {
        let version = self.native_command_name_protocol()?.version();
        (version >= TclVersion::V8_5).then_some(NativeEnsembleConfigurationProtocol {
            version,
            parameters: version >= TclVersion::V8_6,
            dying_namespace: version >= TclVersion::V9_0,
        })
    }
}

/// Native string-key table entry with an independent counted Dict-key position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEnsembleTableEntry {
    /// `CString` table member, independent of the configured original key object.
    pub member: Vec<u8>,
    /// Counted original mapping key selected by the native build algorithm.
    pub mapping: Option<usize>,
}

/// Native table construction and displaced map-only reference ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEnsembleTablePlan {
    /// Actual table entries, sorted by native byte member name.
    pub entries: Vec<NativeEnsembleTableEntry>,
    /// Prior map prefixes overwritten without a native decrement.
    pub displaced_mappings: Vec<usize>,
}

impl NativeEnsembleConfigurationProtocol {
    /// Missing-selector suffix of the actual C ensemble dispatcher. This pure
    /// release selection supplies no original invocation or ensemble identity.
    #[must_use]
    pub const fn missing_selector_usage(self) -> &'static [u8] {
        match self.version {
            TclVersion::V8_5 => b"subcommand ?argument ...?",
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1 => b"subcommand ?arg ...?",
            TclVersion::V8_4 => unreachable!(),
        }
    }

    /// Original ensemble entry accepts only the selected actual namespace
    /// lifetime. C8.5/8.6 test `NS_DYING`; C9 tests `NS_DEAD`.
    #[must_use]
    pub const fn permits_namespace_lifecycle(
        self,
        lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
    ) -> bool {
        use tcl_syntax::native_namespace_name::NativeNamespaceLifecycle;
        match lifecycle {
            NativeNamespaceLifecycle::Live => true,
            NativeNamespaceLifecycle::Dying => self.dying_namespace,
            NativeNamespaceLifecycle::Dead => false,
        }
    }

    /// Original-object create option table, including its native index order.
    #[must_use]
    pub fn create_options(self) -> &'static [&'static str] {
        if self.parameters {
            tcl_cmd_core::ensemble::CREATE_OPTIONS.names()
        } else {
            tcl_cmd_core::ensemble::CREATE_OPTIONS_85.names()
        }
    }
    /// Original-object configure option table, including its native index order.
    #[must_use]
    pub fn configure_options(self) -> &'static [&'static str] {
        if self.parameters {
            tcl_cmd_core::ensemble::CONFIG_OPTIONS.names()
        } else {
            tcl_cmd_core::ensemble::CONFIG_OPTIONS_85.names()
        }
    }
    /// Decode an index from this recipe's create table, not another release's table.
    #[must_use]
    pub fn create_option(self, index: usize) -> tcl_cmd_core::ensemble::CreateOption {
        tcl_cmd_core::ensemble::CreateOption::from_index(if !self.parameters && index >= 2 {
            index + 1
        } else {
            index
        })
    }
    /// Decode an index from this recipe's configure table.
    #[must_use]
    pub fn configure_option(self, index: usize) -> tcl_cmd_core::ensemble::ConfigOption {
        tcl_cmd_core::ensemble::ConfigOption::from_index(if !self.parameters && index >= 2 {
            index + 1
        } else {
            index
        })
    }
    /// Enumerate only options actually present in the selected native table.
    #[must_use]
    pub fn configuration_options(self) -> Vec<tcl_cmd_core::ensemble::ConfigOption> {
        (0..self.configure_options().len())
            .map(|index| self.configure_option(index))
            .collect()
    }
    /// Build the native `CString` table independently of counted configuration data.
    /// `same_list_and_map` is actual header identity, never byte equality.
    #[must_use]
    pub fn table_plan(
        self,
        subcommands: Option<&[&[u8]]>,
        keys: &[&[u8]],
        exports: &[&[u8]],
        same_list_and_map: bool,
    ) -> NativeEnsembleTablePlan {
        let mut rows = std::collections::BTreeMap::<Vec<u8>, Option<usize>>::new();
        let mut displaced_mappings = Vec::new();
        if let Some(names) = subcommands {
            if same_list_and_map {
                for pair in names.as_chunks::<2>().0 {
                    let key = self.member_name(pair[0]).to_vec();
                    let mapping = keys.iter().position(|name| *name == pair[0]);
                    rows.insert(key, mapping);
                    rows.entry(self.member_name(pair[1]).to_vec())
                        .or_insert(None);
                }
            } else {
                for name in names {
                    let mapping = keys.iter().position(|key| key == name);
                    rows.entry(self.member_name(name).to_vec())
                        .or_insert(mapping);
                }
            }
        } else if !keys.is_empty() {
            for (index, key) in keys.iter().enumerate() {
                if let Some(Some(previous)) =
                    rows.insert(self.member_name(key).to_vec(), Some(index))
                {
                    displaced_mappings.push(previous);
                }
            }
        } else {
            for name in exports {
                rows.entry(self.member_name(name).to_vec()).or_insert(None);
            }
        }
        NativeEnsembleTablePlan {
            entries: rows
                .into_iter()
                .map(|(member, mapping)| NativeEnsembleTableEntry { member, mapping })
                .collect(),
            displaced_mappings,
        }
    }
    /// Resolve the native exact `CString` hash probe before counted prefix matching.
    #[must_use]
    pub fn resolve_member<T: AsRef<[u8]>>(
        self,
        members: &[T],
        original: &[u8],
        prefixes: bool,
    ) -> Option<usize> {
        members
            .iter()
            .position(|member| member.as_ref() == self.member_name(original))
            .or_else(|| tcl_cmd_core::ensemble::resolve_subcommand(members, original, prefixes))
    }
    /// Native string-key table extent. Original Dict keys remain counted objects;
    /// this projection must never replace their retained identity or lookup key.
    #[must_use]
    pub fn member_name(self, original: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(original)
    }
}

/// Actual native API that installs or removes the ensemble compiler function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEnsembleCompilerAttachmentProtocol {
    version: TclVersion,
}

impl crate::InvocationDialect {
    /// Select `Tcl_SetEnsembleFlags` compiler attachment without granting a
    /// runtime implementation identity to the configured ensemble.
    #[must_use]
    pub fn native_ensemble_compiler_attachment_protocol(
        self,
    ) -> Option<NativeEnsembleCompilerAttachmentProtocol> {
        let version = self.tcl_version?;
        (self.family()? == tcl_dialect::model::Family::Tcl && version >= TclVersion::V8_5)
            .then_some(NativeEnsembleCompilerAttachmentProtocol { version })
    }
}

impl NativeEnsembleCompilerAttachmentProtocol {
    /// Raw compiler function presence selected by the native compile flag.
    #[must_use]
    pub const fn hook(
        self,
        compile: bool,
    ) -> tcl_runtime_api::native_compilation::NativeCompilerHookPresence {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
        if matches!(self.version, TclVersion::V8_4) {
            NativeCompilerHookPresence::Unknown
        } else if compile {
            NativeCompilerHookPresence::Present
        } else {
            NativeCompilerHookPresence::Absent
        }
    }
}

/// Number of compiler-affecting native setter calls made by one successfully
/// parsed `namespace ensemble configure` update. The unknown-handler and prefix
/// setters do not bump the compiler epoch; the subcommand and mapping setters do,
/// and Tcl 8.6+ also writes the parameter list. Each call checks the actual raw
/// configuration token's compiler attachment, even when the value is unchanged.
/// Read-only queries and rejected updates do not invoke this transaction.
#[must_use]
pub fn configuration_compiler_mutations(dialect: InvocationDialect) -> Option<usize> {
    if dialect.family() != Some(Family::Tcl) {
        return None;
    }
    match dialect.tcl_version? {
        TclVersion::V8_4 => None,
        TclVersion::V8_5 => Some(2),
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1 => Some(3),
    }
}

/// Actual ensemble member compiler selection, without runtime handler semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeEnsembleMemberSelection {
    /// The compiler invokes the public ensemble normally.
    Generic,
    /// A single configured worker can supply its own compiler or a named invocation.
    Worker {
        /// Canonical selected map member, retained for ensemble rewriting.
        member: NameBytes,
        /// Actual configured command name.
        command: NameBytes,
    },
    /// Compiler inputs or a delegated worker protocol remain unproved.
    Unknown,
}

/// Actual configured worker selected at the original compiler boundary.
/// Reporting names and catalogue implementation identities cannot replace the
/// retained command registration supplied by the lookup owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeEnsembleWorkerSelection {
    /// The actual ensemble compiler declines to ordinary public dispatch.
    Generic,
    /// One configured member selects this exact named command registration.
    Worker {
        /// Canonical member retained for the original usage rewrite.
        member: NameBytes,
        /// Original selected command node, independent of callable lifetime.
        binding: Box<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
    },
    /// Actual configuration, naming context or worker registration is missing.
    Unknown,
}

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

#[cfg(test)]
mod configuration_epoch_tests {
    use super::*;

    #[test]
    fn completed_configuration_matches_original_native_epoch_windows() {
        for (version, rows) in [
            (
                TclVersion::V8_4,
                include_str!("../../../runtime/rust/tests/data/native_ensemble_epochs/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../../../runtime/rust/tests/data/native_ensemble_epochs/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../../../runtime/rust/tests/data/native_ensemble_epochs/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../../../runtime/rust/tests/data/native_ensemble_epochs/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../../../runtime/rust/tests/data/native_ensemble_epochs/9.1.0.tsv"),
            ),
        ] {
            let recipe = configuration_compiler_mutations(InvocationDialect::for_version(version));
            let mut previous = None;
            for row in rows.lines().skip(1) {
                let fields: Vec<_> = row.split('\t').collect();
                let compiler = fields[2].parse::<usize>().unwrap();
                if let Some(previous) = previous {
                    assert_eq!(fields[3], "1", "{version:?}: actual original compiler hook");
                    let expected = if fields[1] == "0" { recipe.unwrap() } else { 0 };
                    assert_eq!(compiler - previous, expected, "{version:?} {}", fields[0]);
                }
                previous = Some(compiler);
            }
            assert_eq!(recipe.is_none(), version == TclVersion::V8_4);
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(configuration_compiler_mutations(jim), None);
    }
}

#[cfg(test)]
mod configuration_surface_tests {
    use super::*;

    #[test]
    fn actual_ensemble_missing_selector_usage_is_selected_independently_of_info() {
        // naming.ensemble.original-missing-selector-source-usage
        // docs/design/analysis/name-resolution-proofs/ensemble-original-missing-selector-source-usage.md
        // The separate public originals exercise stock info; this pure API control
        // independently checks the generic dispatcher recipe. Pinned source:
        // C8.5 tclNamesp.c:6050; C8.6 tclEnsemble.c:1717; C9.0:1787; C9.1:1788.
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let recipe = dialect.native_ensemble_configuration_protocol();
            if version == TclVersion::V8_4 {
                assert!(recipe.is_none());
                continue;
            }
            let expected: &[u8] = if version == TclVersion::V8_5 {
                b"subcommand ?argument ...?"
            } else {
                b"subcommand ?arg ...?"
            };
            assert_eq!(recipe.unwrap().missing_selector_usage(), expected);
            assert_eq!(
                dialect.native_info_original_missing_selector_usage(),
                Some(expected)
            );
        }
        for engine in ["jim", "irules"] {
            let dialect = InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(engine).unit_profile(),
            );
            assert!(dialect.native_ensemble_configuration_protocol().is_none());
        }
        let mut unknown = InvocationDialect::for_version(TclVersion::V8_6);
        unknown.core_point = None;
        unknown.native_family = None;
        assert!(unknown.native_ensemble_configuration_protocol().is_none());
    }

    #[test]
    fn actual_namespace_lifetime_policy_matches_pinned_ensemble_entry() {
        // Native proof: naming.ensemble.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/ensemble-original-command-holder-routing.md
        // Pinned C source entry checks complement the original public source
        // controls; no namespace lifetime is inferred from a reporting name.
        use tcl_syntax::native_namespace_name::NativeNamespaceLifecycle as State;
        for version in TclVersion::ALL {
            let recipe =
                InvocationDialect::for_version(version).native_ensemble_configuration_protocol();
            if version == TclVersion::V8_4 {
                assert!(recipe.is_none());
                continue;
            }
            let recipe = recipe.unwrap();
            assert!(recipe.permits_namespace_lifecycle(State::Live));
            assert_eq!(
                recipe.permits_namespace_lifecycle(State::Dying),
                version >= TclVersion::V9_0
            );
            assert!(!recipe.permits_namespace_lifecycle(State::Dead));
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(jim.native_ensemble_configuration_protocol().is_none());
    }

    #[test]
    fn actual_release_tables_and_counted_mapping_keys_remain_separate() {
        for version in TclVersion::ALL {
            let recipe =
                InvocationDialect::for_version(version).native_ensemble_configuration_protocol();
            if version == TclVersion::V8_4 {
                assert!(recipe.is_none());
                continue;
            }
            let recipe = recipe.unwrap();
            let names = recipe.configure_options();
            assert_eq!(names.contains(&"-parameters"), version >= TclVersion::V8_6);
            let index = recipe
                .create_options()
                .iter()
                .position(|name| *name == "-prefixes")
                .unwrap();
            assert_eq!(
                recipe.create_option(index),
                tcl_cmd_core::ensemble::CreateOption::Prefixes
            );
            let keys: &[&[u8]] = &[b"m\0left", b"m\0right"];
            let plan = recipe.table_plan(None, keys, &[], false);
            assert_eq!(
                plan.entries,
                vec![NativeEnsembleTableEntry {
                    member: b"m".to_vec(),
                    mapping: Some(1)
                }]
            );
            assert_eq!(plan.displaced_mappings, vec![0]);
            let plan = recipe.table_plan(Some(keys), keys, &[], false);
            assert_eq!(
                plan.entries,
                vec![NativeEnsembleTableEntry {
                    member: b"m".to_vec(),
                    mapping: Some(0)
                }]
            );
            assert!(plan.displaced_mappings.is_empty());
            assert_eq!(
                recipe.resolve_member(&[b"moon".as_slice()], b"m\0tail", true),
                None
            );
            assert_eq!(recipe.member_name(b"m\xc0\x80tail"), b"m\xc0\x80tail");
            let same: &[&[u8]] = &[b"key", b"::first", b"key\0other", b"::second"];
            let plan = recipe.table_plan(Some(same), &[same[0], same[2]], &[], true);
            assert_eq!(
                plan.entries
                    .iter()
                    .map(|row| (row.member.as_slice(), row.mapping))
                    .collect::<Vec<_>>(),
                vec![
                    (b"::first".as_slice(), None),
                    (b"::second".as_slice(), None),
                    (b"key".as_slice(), Some(1))
                ]
            );
        }
    }
}
