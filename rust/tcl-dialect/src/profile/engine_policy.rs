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

//! The execution-engine policies a profile selects: variable lookup and storage, parameters, packages, indices, interpreters and level grammars.

use super::DialectProfile;
use crate::model::{CorePoints, Family, SpecProvider, SurfaceQuery};
use crate::version::{StringCharacterModel, TclVersion};

impl DialectProfile {
    /// Whether this dialect keeps the TIP 278 namespace-scope global
    /// variable fallback (Tcl 8.x yes, 9.0+ no).
    ///
    /// Follows the *runtime* base release, so a vendor shell inherits its
    /// embedded core's behaviour — an iRules script runs on a real 8.4. A
    /// profile with no documented base gets `false`: without evidence of an
    /// 8.x core, the stricter 9.0 reading avoids inventing cross-scope
    /// references.
    #[must_use]
    pub fn namespace_var_global_fallback(&self) -> bool {
        self.runtime_base
            .is_some_and(TclVersion::namespace_var_global_fallback)
    }

    /// The point this profile asks surface questions at — which core
    /// family and release it is, and which vendor package it carries.
    ///
    /// The two halves stay apart — which language, and which packages —
    /// because they answer different questions. A profile whose vendor
    /// surface *is* a core family — iRules — asks as that family and
    /// carries no Tcl release: a spec available across the Tcl ladder is
    /// not thereby an iRules spec.
    ///
    /// A family whose ladder is not keyed by version (the iRules `tmos`
    /// line) asks about its whole ladder: there is no release to name.
    #[must_use]
    pub fn surface_query(&self) -> SurfaceQuery<'static> {
        if let Some(point) = self.core_point {
            let family = point.family();
            let core = family.ancestry().map_or_else(
                || CorePoints::one(family, Some(point.release().as_str())),
                |ancestry| {
                    CorePoints::two(
                        (family, Some(point.release().as_str())),
                        (ancestry.parent, Some(ancestry.anchor)),
                    )
                },
            );
            return SurfaceQuery {
                realm: crate::model::InvocationRealm::RuleLoader,
                core,
                packages: crate::model::PackageFloors::Borrowed(&[]),
            };
        }
        if let Some(SpecProvider::Core(family)) = self.vendor_surface {
            return SurfaceQuery::any_release(family);
        }
        SurfaceQuery {
            realm: crate::model::InvocationRealm::RuleLoader,
            core: match self.signature_base {
                Some(version) => CorePoints::one(Family::Tcl, Some(version.version_string())),
                // No pinned release, but still a Tcl surface: the permissive
                // `tcl` sink and the `tk` ingress profile ask about the whole
                // ladder. A profile whose grammar names no core family
                // (`f5-bigip`) has no Tcl surface to ask about.
                None if self
                    .grammar_union
                    .contains(&SpecProvider::Core(Family::Tcl)) =>
                {
                    CorePoints::one(Family::Tcl, None)
                }
                None => CorePoints::NONE,
            },
            packages: crate::model::PackageFloors::Borrowed(self.surface_packages),
        }
    }

    /// The release this profile's *runtime* behaviour follows, if it names one.
    ///
    /// A thin name over [`Self::runtime_base`], but the name is the point: it
    /// is the sanctioned way to get from a profile to a [`TclVersion`], so the
    /// step reads the same everywhere and a future rule (a vendor surface
    /// that overrides the base, say) has one place to land.
    #[must_use]
    pub fn runtime_version(&self) -> Option<TclVersion> {
        self.runtime_base
    }

    /// Variable namespace/activation lookup semantics of the execution engine.
    #[must_use]
    pub fn variable_lookup_policy(&self) -> Option<crate::VariableLookupPolicy> {
        if self.runtime_version().is_some() {
            Some(crate::VariableLookupPolicy::Tcl)
        } else if self
            .grammar_union
            .contains(&SpecProvider::Core(Family::Jim))
        {
            Some(crate::VariableLookupPolicy::Jim)
        } else {
            None
        }
    }

    /// Native root-variable storage, independent of name and alias selection.
    ///
    /// ```
    /// use tcl_dialect::{DialectProfile, VariableContainerModel};
    /// let profile = DialectProfile::find("tcl8.6").unwrap();
    /// assert_eq!(profile.variable_container_model(), Some(VariableContainerModel::DistinctArray));
    /// ```
    #[must_use]
    pub fn variable_container_model(&self) -> Option<crate::VariableContainerModel> {
        self.variable_lookup_policy()
            .map(crate::VariableLookupPolicy::container_model)
    }

    /// Variable alias identity, separate from qualified-name lookup grammar.
    ///
    /// ```
    /// use tcl_dialect::{DialectProfile, VariableLinkBinding};
    /// let profile = DialectProfile::find("tcl8.6").unwrap();
    /// assert_eq!(profile.variable_link_binding(), Some(VariableLinkBinding::StableCell));
    /// ```
    #[must_use]
    pub fn variable_link_binding(&self) -> Option<crate::VariableLinkBinding> {
        self.variable_lookup_policy()
            .map(crate::VariableLookupPolicy::link_binding)
    }

    /// Native lazy double-string policy, independently of source numeral grammar.
    #[must_use]
    pub fn double_string_policy(&self) -> Option<crate::DoubleStringPolicy> {
        self.runtime_version()
            .map(crate::DoubleStringPolicy::for_tcl_version)
            .or_else(|| {
                self.core_point
                    .filter(|point| point.release() == crate::model::Release::JIM_0_84)
                    .map(|_| crate::DoubleStringPolicy::JimTwelve)
            })
    }

    /// Native procedure/apply formal parameter and activation grammar.
    #[must_use]
    pub fn parameter_grammar(&self) -> Option<crate::ParameterGrammar> {
        self.variable_lookup_policy().map(|policy| match policy {
            crate::VariableLookupPolicy::Tcl => crate::ParameterGrammar::Tcl,
            crate::VariableLookupPolicy::Jim => crate::ParameterGrammar::Jim,
        })
    }

    /// Package discovery protocol, separate from C Tcl release grammar.
    #[must_use]
    pub fn package_protocol(&self) -> Option<crate::PackageProtocol> {
        self.variable_lookup_policy().map(|policy| match policy {
            crate::VariableLookupPolicy::Tcl => crate::PackageProtocol::Tcl,
            crate::VariableLookupPolicy::Jim => crate::PackageProtocol::Jim,
        })
    }

    /// Native concat representation protocol proved for this engine.
    #[must_use]
    pub fn concat_policy(&self) -> Option<crate::ConcatPolicy> {
        if let Some(version) = self.runtime_version() {
            Some(crate::ConcatPolicy::Tcl(version))
        } else {
            self.core_point
                .is_some_and(|point| point.release() == crate::model::Release::JIM_0_84)
                .then_some(crate::ConcatPolicy::JimRepresentationSensitive)
        }
    }

    /// Complete container index policy proved for this native release.
    #[must_use]
    pub fn index_syntax(&self) -> Option<crate::IndexSyntax> {
        self.runtime_version()
            .map(crate::IndexSyntax::for_version)
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == crate::model::Release::JIM_0_84)
                    .then_some(crate::IndexSyntax {
                        numbers: self.grammar.numbers,
                        grammar: crate::IndexGrammar::Jim,
                        width: crate::IndexIntegerWidth::Jim32,
                        end_abbreviations: false,
                    })
            })
    }

    /// List replacement bounds proved for this runtime release.
    #[must_use]
    pub fn list_set_bounds(&self) -> Option<crate::ListSetBounds> {
        self.runtime_version()
            .map(TclVersion::list_set_bounds)
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == crate::model::Release::JIM_0_84)
                    .then_some(crate::ListSetBounds::ExistingElement)
            })
    }

    /// Exit-status conversion, independently of numeral spelling grammar.
    #[must_use]
    pub fn process_exit_conversion(&self) -> Option<crate::ProcessExitConversion> {
        if let Some(version) = self.runtime_version() {
            Some(if version < TclVersion::V9_0 {
                crate::ProcessExitConversion::Narrow32
            } else {
                crate::ProcessExitConversion::WholeInteger
            })
        } else if self.variable_lookup_policy() == Some(crate::VariableLookupPolicy::Jim) {
            Some(crate::ProcessExitConversion::SaturatingWide)
        } else {
            None
        }
    }

    /// Interpreter factory and handle protocol of this execution engine.
    #[must_use]
    pub fn interpreter_protocol(&self) -> Option<crate::InterpreterProtocol> {
        self.variable_lookup_policy().map(|policy| match policy {
            crate::VariableLookupPolicy::Tcl => crate::InterpreterProtocol::Tcl,
            crate::VariableLookupPolicy::Jim => crate::InterpreterProtocol::JimHandles,
        })
    }

    /// How namespace imports bind their source commands.
    #[must_use]
    pub fn namespace_import_binding(&self) -> Option<crate::NamespaceImportBinding> {
        if self.runtime_version().is_some() {
            Some(crate::NamespaceImportBinding::CommandToken)
        } else if self
            .grammar_union
            .contains(&SpecProvider::Core(Family::Jim))
        {
            Some(crate::NamespaceImportBinding::SourceName)
        } else {
            None
        }
    }

    /// The `upvar` optional-level grammar, independently of numeral syntax.
    /// Jim uses count parity; an environment without a runtime abstains.
    #[must_use]
    pub fn upvar_level_presence(&self) -> Option<crate::FrameLevelPresence> {
        self.runtime_version()
            .map(TclVersion::upvar_level_presence)
            .or_else(|| {
                self.grammar_union
                    .contains(&SpecProvider::Core(Family::Jim))
                    .then_some(crate::FrameLevelPresence::ArgumentParity)
            })
    }

    /// The `uplevel` optional-level grammar, independently of numeral syntax.
    /// Jim's digit/`#` probe does not accept signed or padded integers.
    #[must_use]
    pub fn uplevel_level_presence(&self) -> Option<crate::FrameLevelPresence> {
        self.runtime_version()
            .map(TclVersion::uplevel_level_presence)
            .or_else(|| {
                self.grammar_union
                    .contains(&SpecProvider::Core(Family::Jim))
                    .then_some(crate::FrameLevelPresence::DigitOrHash)
            })
    }

    /// The selected character recipe, separately from appliance evidence.
    #[must_use]
    pub fn character_model(&self) -> Option<StringCharacterModel> {
        // Keep the F5 compatibility projection aligned with CoreProfileId;
        // its selected recipe supplies no measured appliance character count.
        if matches!(
            self.runtime_core_family(),
            Some(Family::F5Irules | Family::F5Tcl)
        ) {
            return Some(StringCharacterModel::Utf16CodeUnits);
        }
        if self
            .core_point
            .is_some_and(|point| point.release() == crate::model::Release::JIM_0_84)
        {
            return Some(StringCharacterModel::Jim084Utf8);
        }
        self.runtime_version()
            .map(TclVersion::string_character_model)
    }
}
