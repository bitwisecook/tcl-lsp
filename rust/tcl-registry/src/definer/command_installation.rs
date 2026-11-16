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

//! How a class command installs and names what it manufactures: dispatchers, manufacturer methods and factory initialisers.

use super::member_effect::MemberVisibility;
use super::{DefinerFamily, DefinitionBodyGrammar, MemberSpec};
use tcl_dialect::model::SpecSurface;

/// Authored finite source presentation hints for an instance-name operand.
/// These values select only source construction candidates, never an entered
/// constructor, object identity or Native execution guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BareWordConstructionSourceHint {
    /// Complete literal source values recognised by this authored recipe.
    pub values: &'static [&'static str],
    /// Non-empty source-value prefixes recognised by this authored recipe.
    pub prefixes: &'static [&'static str],
}

impl BareWordConstructionSourceHint {
    /// Project this exact authored recipe, without identifying a class family.
    #[must_use]
    pub fn recognises(self, word: &str) -> bool {
        self.values.contains(&word)
            || self
                .prefixes
                .iter()
                .any(|prefix| !prefix.is_empty() && word.starts_with(*prefix))
    }
}

/// One two-word command slot installed by the selected source class factory.
/// Its descriptor withdraws older source declarations; it supplies neither
/// the installed implementation nor an actual receiver or execution result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceTwoWordFactoryInitialiser {
    /// Member units appended to the original class-name value by the provider.
    pub name: &'static str,
    /// Provider-selected publication protocol; Jim aliases do not inherit
    /// procedure namespace qualification. Shared table comparison is separate.
    pub publication: tcl_syntax::naming::NativeNamePurpose,
    /// Whether the original factory base list controls this installation.
    pub presence: SourceFactoryInitialiserPresence,
}

/// Original source operand condition for a factory-generated command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFactoryInitialiserPresence {
    /// Installed by every successful selected factory recipe.
    Always,
    /// Installed only when the original base list contains at least one name.
    NonemptyBaseList,
}

impl SourceFactoryInitialiserPresence {
    /// Conditional source replacement, independently of successful execution.
    /// An unavailable base count remains unavailable.
    #[must_use]
    pub fn replaces_source_slot(self, base_count: Option<usize>) -> Option<bool> {
        match self {
            Self::Always => Some(true),
            Self::NonemptyBaseList => base_count.map(|count| count != 0),
        }
    }
}

/// Conditional command installation described by an actual definer recipe.
/// This does not prove callback closure, a live receiver, class methods,
/// native compiler hooks or executable specialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionCommandInstallation {
    /// Original argument naming the newly defined dispatcher.
    pub name_at: u8,
    /// Original argument evaluated as its definition.
    pub body_at: u8,
    /// Dispatcher implementation installed by the provider recipe.
    pub dispatcher: DefinitionDispatcher,
    /// Returned-name protocol of its admitted manufacturing methods.
    pub construction_result: ConstructionNameResult,
}

/// Implementation protocol of an installed definition dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefinitionDispatcher {
    /// Snit type-command dispatch; distinct from a native `TclOO` class object.
    SnitType,
    /// Itcl's class-name dispatcher, independently installed by its provider.
    /// This is not a native `TclOO` class or instance receipt.
    ItclClass,
}

impl DefinitionDispatcher {
    /// Native engine axes audited for this installed provider recipe. A live
    /// attested installation and unchanged dependencies are still required;
    /// this query grants neither an installed token nor a compiler hook alone.
    #[must_use]
    pub fn native_installation_is_audited(self, dialect: crate::InvocationDialect) -> bool {
        use tcl_dialect::TclVersion;
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
            return false;
        }
        match self {
            Self::SnitType => dialect
                .tcl_version
                .is_some_and(|version| version >= TclVersion::V8_5),
            Self::ItclClass => matches!(
                dialect.tcl_version,
                Some(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            ),
        }
    }
}

/// Name contents returned by a successful manufacturing recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionNameResult {
    /// The qualified instance name, including any provider-generated suffix.
    /// The return value alone proves neither continued command existence nor
    /// object class identity after arbitrary callbacks.
    QualifiedInstanceName,
    /// The requested instance spelling, replacing `#auto` with a generated
    /// suffix. Itcl can return a relative name even when the actual installed
    /// command is qualified. This proves no continued command existence.
    WrittenInstanceName,
}

/// One method of a class system's **class command** that manufactures an
/// instance — see [`DefinitionBodyGrammar::manufacturers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManufacturerMethod {
    /// The method word as invoked on the class command (`create`).
    pub keyword: &'static str,
    /// Whether ordinary dispatch through the class command can reach this
    /// method without an explicit export. `createWithNamespace` is a real
    /// `TclOO` method but is unexported in C Tcl 9.0.4 and 8.6.
    pub visibility: MemberVisibility,
    /// Index — among the call's arguments, argument 0 being the manufacturer
    /// keyword itself — of the word naming the new instance, or `None` when
    /// the manufacturer generates the name itself (`new`).
    pub names_instance_at: Option<u8>,
    /// Index — in the same argument coordinate system — of the definition
    /// body for a class-manufacturing call. `None` when this method creates
    /// an ordinary instance and has no class-definition body.
    pub definition_body_at: Option<u8>,
    /// Index of the first argument passed to the newly created object's
    /// constructor. This keeps constructor parameter flow independent of the
    /// spelling and of structural words such as an explicit object name or
    /// namespace name.
    pub constructor_args_from: u8,
}

impl DefinitionBodyGrammar {
    /// Original two-word slots overwritten by the selected Jim class recipe.
    /// Common names come from the selected family descriptors. Constructor
    /// support follows its independently pinned provider window; an unknown
    /// release cannot select it. No native installation is certified here.
    #[must_use]
    pub fn source_two_word_factory_initialisers(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Option<Vec<SourceTwoWordFactoryInitialiser>> {
        use tcl_dialect::model::Family;
        let query = dialect.authoring_query()?;
        let (Family::Jim, Some(_)) = query.core.nearest()? else {
            return None;
        };
        if self.family != DefinerFamily::JimClass
            || !SpecSurface::core_in(Family::Jim, &[("0.76", None)]).admits(&query)
        {
            return None;
        }
        let mut names: Vec<_> = self
            .builtin_object_methods
            .iter()
            .map(|method| method.name)
            .chain(self.manufacturers.iter().map(|method| method.keyword))
            .collect();
        if SpecSurface::core_in(Family::Jim, &[("0.82", None)]).admits(&query) {
            names.extend(["constructor", "defaultconstructor"]);
        }
        names.sort_unstable();
        names.dedup();
        let mut initialisers: Vec<_> = names
            .into_iter()
            .map(|name| SourceTwoWordFactoryInitialiser {
                name,
                publication: if name == "constructor" {
                    tcl_syntax::naming::NativeNamePurpose::AliasPublication
                } else {
                    tcl_syntax::naming::NativeNamePurpose::CommandPublication
                },
                presence: SourceFactoryInitialiserPresence::Always,
            })
            .collect();
        initialisers.push(SourceTwoWordFactoryInitialiser {
            name: "baseclass",
            publication: tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            presence: SourceFactoryInitialiserPresence::NonemptyBaseList,
        });
        Some(initialisers)
    }

    /// Provider-authored dispatcher installation and returned-name protocol.
    /// Snit 2.3.4 and Itcl 4.3.2 evaluate their definitions before installing
    /// provider dispatchers. Immediate definition members can retire that
    /// command even on normal completion, so this is conditional metadata.
    /// Actual handler/provider identity and callback effects remain required.
    #[must_use]
    pub const fn command_installation(&self) -> Option<DefinitionCommandInstallation> {
        match self.family {
            DefinerFamily::Snit => Some(DefinitionCommandInstallation {
                name_at: 0,
                body_at: 1,
                dispatcher: DefinitionDispatcher::SnitType,
                construction_result: ConstructionNameResult::QualifiedInstanceName,
            }),
            DefinerFamily::Itcl => Some(DefinitionCommandInstallation {
                name_at: 0,
                body_at: 1,
                dispatcher: DefinitionDispatcher::ItclClass,
                construction_result: ConstructionNameResult::WrittenInstanceName,
            }),
            _ => None,
        }
    }

    /// Members that install a procedure body without running it during the
    /// selected provider definition. Immediate initialization and declaration initializers
    /// remain unknown. This grants no instance-constructor closure.
    #[must_use]
    pub fn installation_member_is_deferred(&self, member: &MemberSpec) -> bool {
        matches!(self.family, DefinerFamily::Snit | DefinerFamily::Itcl)
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
            && match self.family {
                DefinerFamily::Itcl => {
                    matches!(
                        member.keyword,
                        "method" | "proc" | "constructor" | "destructor"
                    )
                }
                DefinerFamily::Snit => matches!(
                    member.keyword,
                    "method"
                        | "typemethod"
                        | "proc"
                        | "constructor"
                        | "destructor"
                        | "onconfigure"
                        | "oncget"
                ),
                _ => false,
            }
    }

    /// Name operand of a deferred declaration that changes type dispatch.
    /// The selected member must belong to this exact installation grammar.
    #[must_use]
    pub fn installation_dispatch_method_name_at(&self, member: &MemberSpec) -> Option<u8> {
        (matches!(self.family, DefinerFamily::Snit | DefinerFamily::Itcl)
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
            && match self.family {
                DefinerFamily::Snit => member.keyword == "typemethod",
                DefinerFamily::Itcl => member.keyword == "proc",
                _ => false,
            })
        .then_some(0)
    }

    /// Normal returned-name operand of a selected manufacture call. Bare-word
    /// Snit construction excludes generated and declared type methods. Itcl's
    /// class dispatcher always treats its first word as the instance name.
    /// This grants no receiver existence.
    #[must_use]
    pub fn conditional_construction_name_at<'a>(
        &self,
        first: &str,
        declared_type_methods: impl IntoIterator<Item = &'a str>,
    ) -> Option<usize> {
        // Itcl class commands construct even when the first word also names
        // a class-scoped proc; that proc lives at Class::procName instead.
        if self.family == DefinerFamily::Itcl {
            return Some(0);
        }
        if let Some(method) = self.manufacturer(first) {
            return method.names_instance_at.map(usize::from);
        }
        (self.bare_word_construction
            && !self.builtin_type_methods.contains(&first)
            && !declared_type_methods.into_iter().any(|name| name == first))
        .then_some(0)
    }

    /// The manufacturer method `keyword` names, if this family's class
    /// command has one (see [`Self::manufacturers`]).
    #[must_use]
    pub fn manufacturer(&self, keyword: &str) -> Option<&'static ManufacturerMethod> {
        let idx = self
            .manufacturers
            .iter()
            .position(|m| m.keyword == keyword)?;
        Some(&self.manufacturers[idx])
    }
}
