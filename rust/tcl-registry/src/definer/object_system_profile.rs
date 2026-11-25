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

//! Independent definition, dispatch and body-environment facts for authored grammars.

use super::{
    ConstructionNameResult, DefinitionCommandInstallation, DefinitionDispatcher,
    DefinitionMemberNamePurpose, MemberCurrentNamespace,
};

/// How the source declaration presents its members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionShape {
    /// A metaclass manufacture call with an optional member script.
    MetaclassScript,
    /// A name followed by a script of declarations.
    NamedScript,
    /// An optional base list followed by initial variable values.
    VariableDictionary,
    /// A source provider may create or extend a definition.
    ConditionalScript,
    /// Structured data that installs no runtime commands.
    Declaration,
}

/// Storage and invocation shape of declared members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStorage {
    /// Receiver method tables with native method lookup.
    ReceiverMethods,
    /// Namespace-qualified procedures for class-scoped members.
    QualifiedProcedures,
    /// A procedure key is the two-element list of class and member.
    TwoWordCommands,
    /// A provider dispatcher selects its own member implementation.
    ProviderMethods,
    /// Declaration data has no callable member storage.
    Declarations,
}

/// Visibility of an unmodified member declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberExportRule {
    /// Every declared name is exported unless explicitly restricted.
    All,
    /// Only names beginning with an ASCII lowercase byte are exported.
    AsciiLowercase,
}

/// Source declaration and body facts; these grant no native handler authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectSystemProfile {
    /// Source operand layout used to collect declarations.
    pub definition_shape: DefinitionShape,
    /// How callable members are stored and selected.
    pub member_storage: MemberStorage,
    /// Current namespace of an executable member body.
    pub current_namespace: MemberCurrentNamespace,
    /// Commands initially present in a runtime receiver namespace.
    pub receiver_namespace_commands: &'static [&'static str],
    /// Visibility of names before explicit access changes.
    pub export_rule: MemberExportRule,
    /// Naming protocol for ordinary source member declarations.
    pub member_name_purpose: DefinitionMemberNamePurpose,
    /// Naming protocol for source configuration declarations.
    pub configuration_name_purpose: DefinitionMemberNamePurpose,
    /// Native worker namespace, independently of handler identity proof.
    pub native_definition_namespace: Option<&'static str>,
    /// Provider installation layout and returned-name protocol.
    pub installation: Option<DefinitionCommandInstallation>,
    /// Members that store callable bodies during provider installation.
    pub deferred_installation_members: &'static [&'static str],
    /// Deferred member that changes type-level dispatch.
    pub installation_dispatch_member: Option<&'static str>,
    /// Exact operand count and base-list index of a root declaration.
    pub source_base_class_list: Option<(usize, usize)>,
    /// Root command dependencies of native default construction.
    pub default_construction_dependencies: &'static [&'static str],
}

impl ObjectSystemProfile {
    /// Name of an equal shipped profile for lossless source rendering.
    #[must_use]
    pub fn shipped_constant_name(self) -> Option<&'static str> {
        [
            ("TCLOO_PROFILE", TCLOO_PROFILE),
            ("SNIT_PROFILE", SNIT_PROFILE),
            ("ITCL_PROFILE", ITCL_PROFILE),
            ("CLAY_PROFILE", CLAY_PROFILE),
            ("JIM_PROFILE", JIM_PROFILE),
            ("DECLARATION_PROFILE", DECLARATION_PROFILE),
        ]
        .into_iter()
        .find_map(|(name, profile)| (self == profile).then_some(name))
    }

    /// Whether declarations describe runtime command manufacturers.
    #[must_use]
    pub const fn manufactures_runtime_commands(self) -> bool {
        !matches!(self.definition_shape, DefinitionShape::Declaration)
    }

    /// Whether member dispatch uses receiver method tables.
    #[must_use]
    pub const fn members_use_receiver_methods(self) -> bool {
        matches!(self.member_storage, MemberStorage::ReceiverMethods)
    }

    /// Whether each member occupies a two-word procedure key.
    #[must_use]
    pub const fn members_are_two_word_commands(self) -> bool {
        matches!(self.member_storage, MemberStorage::TwoWordCommands)
    }

    /// Whether class-scoped members use qualified procedure names.
    #[must_use]
    pub const fn class_procedures_are_qualified(self) -> bool {
        matches!(self.member_storage, MemberStorage::QualifiedProcedures)
    }

    /// Whether native definition workers are described by this profile.
    #[must_use]
    pub const fn has_native_definition_workers(self) -> bool {
        self.native_definition_namespace.is_some()
    }

    /// Whether source declarations are conditional provider inventories.
    #[must_use]
    pub const fn conditional_source_inventory(self) -> bool {
        matches!(self.definition_shape, DefinitionShape::ConditionalScript)
    }
}

/// Structured declaration data with no runtime manufacturers.
pub const DECLARATION_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::Declaration,
    member_storage: MemberStorage::Declarations,
    current_namespace: MemberCurrentNamespace::DefinedEntity,
    receiver_namespace_commands: &[],
    export_rule: MemberExportRule::All,
    member_name_purpose: DefinitionMemberNamePurpose::SourceValue,
    configuration_name_purpose: DefinitionMemberNamePurpose::SourceValue,
    native_definition_namespace: None,
    installation: None,
    deferred_installation_members: &[],
    installation_dispatch_member: None,
    source_base_class_list: None,
    default_construction_dependencies: &[],
};

/// Native receiver methods and metaclass definition scripts.
pub const TCLOO_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::MetaclassScript,
    member_storage: MemberStorage::ReceiverMethods,
    current_namespace: MemberCurrentNamespace::RuntimeReceiver,
    receiver_namespace_commands: super::TCLOO_RECEIVER_NAMESPACE_COMMANDS,
    export_rule: MemberExportRule::AsciiLowercase,
    member_name_purpose: DefinitionMemberNamePurpose::TclOoMethod,
    configuration_name_purpose: DefinitionMemberNamePurpose::TclOoMethod,
    native_definition_namespace: Some("::oo::define"),
    default_construction_dependencies: &["::oo::class", "::oo::object"],
    ..DECLARATION_PROFILE
};

/// Conditional provider inventory backed by receiver method tables.
pub const CLAY_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::ConditionalScript,
    native_definition_namespace: None,
    configuration_name_purpose: DefinitionMemberNamePurpose::SourceValue,
    default_construction_dependencies: &[],
    ..TCLOO_PROFILE
};

/// Named script definitions with provider-owned method dispatch.
pub const SNIT_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::NamedScript,
    member_storage: MemberStorage::ProviderMethods,
    installation: Some(DefinitionCommandInstallation {
        name_at: 0,
        body_at: 1,
        dispatcher: DefinitionDispatcher::SnitType,
        construction_result: ConstructionNameResult::QualifiedInstanceName,
    }),
    deferred_installation_members: &[
        "method",
        "typemethod",
        "proc",
        "constructor",
        "destructor",
        "onconfigure",
        "oncget",
    ],
    installation_dispatch_member: Some("typemethod"),
    ..DECLARATION_PROFILE
};

/// Named script definitions with qualified class procedures.
pub const ITCL_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::NamedScript,
    member_storage: MemberStorage::QualifiedProcedures,
    installation: Some(DefinitionCommandInstallation {
        name_at: 0,
        body_at: 1,
        dispatcher: DefinitionDispatcher::ItclClass,
        construction_result: ConstructionNameResult::WrittenInstanceName,
    }),
    deferred_installation_members: &["method", "proc", "constructor", "destructor"],
    installation_dispatch_member: Some("proc"),
    ..DECLARATION_PROFILE
};

/// Variable dictionaries with two-word member procedure keys.
pub const JIM_PROFILE: ObjectSystemProfile = ObjectSystemProfile {
    definition_shape: DefinitionShape::VariableDictionary,
    member_storage: MemberStorage::TwoWordCommands,
    source_base_class_list: Some((3, 1)),
    ..DECLARATION_PROFILE
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definer::{DefinerFamily, DefinitionBodyGrammar, TCLOO_GRAMMAR};

    #[test]
    fn identity_does_not_select_namespace_export_or_native_worker_metadata() {
        let renamed = DefinitionBodyGrammar {
            family: DefinerFamily::SpecTcl,
            ..TCLOO_GRAMMAR
        };
        assert_eq!(
            renamed.member_current_namespace(),
            MemberCurrentNamespace::RuntimeReceiver
        );
        assert!(renamed.member_default_exported("lower"));
        assert!(!renamed.member_default_exported("Upper"));
        assert_eq!(
            renamed.default_construction_lookup_dependencies(),
            Some(&["::oo::class", "::oo::object"][..])
        );
        assert!(
            renamed
                .definition_member_lookup(renamed.member("method").unwrap(), None)
                .is_some()
        );
    }

    #[test]
    fn independent_profile_facts_override_the_same_family_identity() {
        let custom = DefinitionBodyGrammar {
            profile: ObjectSystemProfile {
                current_namespace: MemberCurrentNamespace::DefinedEntity,
                export_rule: MemberExportRule::All,
                native_definition_namespace: None,
                default_construction_dependencies: &[],
                ..TCLOO_PROFILE
            },
            ..TCLOO_GRAMMAR
        };
        assert_eq!(
            custom.member_current_namespace(),
            MemberCurrentNamespace::DefinedEntity
        );
        assert!(custom.member_default_exported("Upper"));
        assert_eq!(custom.default_construction_lookup_dependencies(), None);
        assert!(
            custom
                .definition_member_lookup(custom.member("method").unwrap(), None)
                .is_none()
        );
        assert!(custom.profile.manufactures_runtime_commands());
    }

    #[test]
    fn shipped_profiles_keep_distinct_orthogonal_storage_and_body_facts() {
        assert_eq!(CLAY_PROFILE.member_storage, TCLOO_PROFILE.member_storage);
        assert!(CLAY_PROFILE.conditional_source_inventory());
        assert!(!CLAY_PROFILE.has_native_definition_workers());
        assert_eq!(SNIT_PROFILE.definition_shape, ITCL_PROFILE.definition_shape);
        assert!(!SNIT_PROFILE.class_procedures_are_qualified());
        assert!(ITCL_PROFILE.class_procedures_are_qualified());
        assert!(JIM_PROFILE.members_are_two_word_commands());
        assert_eq!(JIM_PROFILE.source_base_class_list, Some((3, 1)));
        assert!(!DECLARATION_PROFILE.manufactures_runtime_commands());
    }
}
