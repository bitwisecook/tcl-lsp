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

//! Native ensemble configuration tables, compiler attachment and member
//! selection results, independent of the compiler-side selection.

use crate::InvocationDialect;
use tcl_core_types::NameBytes;
use tcl_dialect::{TclVersion, model::Family};

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

impl InvocationDialect {
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
                let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
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
                crate::test_profiles::resolve_environment(engine).unit_profile(),
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
            assert_eq!(plan.displaced_mappings.len(), 0);
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
