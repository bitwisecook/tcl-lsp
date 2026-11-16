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

//! Instance methods of an object class: providers, selection in context and resolving an instance invocation.

use super::CommandRegistry;
use super::argument_roles::sub_command_prefixes;
use super::invocation_resolution::pick_form;
use crate::InvocationWords;
use crate::abbrev::{Keyword, KeywordTable};
use crate::arg_role::AppendedArity;
use crate::invocation_words::{CommandPrefixArguments, InvocationWord};
use crate::resolved_invocation::{ResolvedInvocation, ResolvedSubcommand, SubcommandResolution};
use crate::spec::{CommandSpec, SubCommand};
use tcl_dialect::model::{SurfaceQuery, surface_admits};

/// Unavailable inherited source method context, distinct from an unknown method.
/// No variant proves method absence, executed failure or Native eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceMethodContextUnavailable {
    /// The supplied full context owns a different command store.
    RegistryMismatch,
    /// The retained factory is not the actual context-selected descriptor.
    FactorySelection,
    /// An inherited provider is unavailable under actual context resolution.
    ProviderSelection(&'static str),
    /// A declared superclass cannot select a descriptor under this surface.
    SuperclassSelection(&'static str),
    /// An owner has no independently declared instance method table.
    MethodTable(&'static str),
    /// Receiver and method options require distinct unrepresented version axes.
    ReceiverOptionAxes,
}

impl CommandRegistry {
    /// The [`ObjectClassSpec`] for a `TclOO` / megawidget class named
    /// `class_name`, or `None` when it is not a registry-modelled class.
    ///
    /// For a `TclOO` class the class name is the factory command name
    /// (`oo::class create Foo` binds command `Foo`), so this resolves through
    /// the ordinary command table — no separate index.  A leading `::` falls
    /// back to the bare name, as with [`Self::get`].
    #[must_use]
    pub fn object_class(&self, class_name: &str) -> Option<&crate::spec::ObjectClassSpec> {
        self.get(class_name).and_then(|s| s.object_class)
    }

    /// Assemble the visible instance-method table for `class_name`, walking
    /// declared superclasses breadth-first.
    ///
    /// The owning *class command* supplies the lifecycle axis for each row,
    /// rather than assuming every inherited method belongs to the receiver's
    /// package. This is the object-method counterpart of normal subcommand
    /// filtering: a Tk 9.1-only widget operation must not become a hover,
    /// semantic-token, taint, or completion fact in a profile that resolves
    /// Tk 9.0. The filter is entirely descriptor-driven, so the registry knows
    /// no widget or method spellings.
    #[must_use]
    pub fn instance_methods(&self, class_name: &str) -> Vec<&'static SubCommand> {
        self.instance_methods_at(class_name, None, self.own_surface_query())
    }

    /// [`Self::instance_methods`] with a document-resolved floor for the
    /// owning package and an explicit surface point.
    ///
    /// A request-time caller obtains `package_version` from its document's
    /// package-require facts for the class command, which can only raise the
    /// profile/ambient floor. Passing `None` retains the registry's static
    /// profile answer. `dialect` similarly lets an unprofiled, hand-assembled
    /// registry honour a caller's resolved command surface.
    #[must_use]
    pub fn instance_methods_at(
        &self,
        class_name: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<&'static SubCommand> {
        let Some(class_spec) = self.get_for_surface(class_name, dialect) else {
            return Vec::new();
        };
        self.instance_methods_for_descriptor_at(class_spec, package_version, dialect)
    }

    /// Visible method metadata from an independently selected factory descriptor.
    /// The descriptor must belong to this exact Registry and surface. This
    /// source projection supplies no instance allocation or runtime dispatch.
    #[must_use]
    pub fn instance_methods_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<&'static SubCommand> {
        self.instance_method_providers_for_descriptor_at(class_spec, dialect, |_| {
            Some(package_version)
        })
        .into_iter()
        .map(|(_, method)| method)
        .collect()
    }

    /// Visible method rows with their actual owning class descriptors.
    ///
    /// The receiver factory must be the exact descriptor selected by this
    /// Registry and surface. Each inherited owner retains its own package:
    /// `provider_floor` returns `None` for an unavailable owner, `Some(None)`
    /// for an admitted owner with no version floor, or `Some(Some(version))`
    /// for its document-resolved package floor. The Registry combines that
    /// floor with the same owner's static floor before filtering its rows.
    /// Unavailable owners contribute neither methods nor superclass traversal.
    ///
    /// This source catalogue projection grants no package activation, instance
    /// allocation, executed method, physical receiver or Native permission.
    #[must_use]
    pub fn instance_method_providers_for_descriptor_at<'v>(
        &self,
        class_spec: &CommandSpec,
        dialect: Option<SurfaceQuery<'_>>,
        mut provider_floor: impl FnMut(&CommandSpec) -> Option<Option<&'v str>>,
    ) -> Vec<(&'static CommandSpec, &'static SubCommand)> {
        self.instance_method_providers_with_context_status(
            class_spec,
            dialect,
            &mut provider_floor,
            false,
        )
        .unwrap_or_default()
    }

    fn instance_method_providers_with_context_status<'v>(
        &self,
        class_spec: &CommandSpec,
        dialect: Option<SurfaceQuery<'_>>,
        mut provider_floor: impl FnMut(&CommandSpec) -> Option<Option<&'v str>>,
        require_complete: bool,
    ) -> Result<Vec<(&'static CommandSpec, &'static SubCommand)>, InstanceMethodContextUnavailable>
    {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let Some(selected) = self.get_for_surface(class_spec.name, dialect) else {
            return Err(InstanceMethodContextUnavailable::FactorySelection);
        };
        if !std::ptr::eq(selected, class_spec) {
            return Err(InstanceMethodContextUnavailable::FactorySelection);
        }
        let mut queue = std::collections::VecDeque::from([selected]);
        let mut seen = std::collections::HashSet::new();
        let mut method_names = std::collections::HashSet::new();
        let mut visible_methods = Vec::new();
        while let Some(owner) = queue.pop_front() {
            if !seen.insert(owner.name) {
                continue;
            }
            let Some(class) = owner.object_class else {
                if require_complete {
                    return Err(InstanceMethodContextUnavailable::MethodTable(owner.name));
                }
                continue;
            };
            let Some(package_version) = provider_floor(owner) else {
                if require_complete {
                    return Err(InstanceMethodContextUnavailable::ProviderSelection(
                        owner.name,
                    ));
                }
                continue;
            };
            let package_floor = self.package_floor_for_spec_at(owner, package_version);
            for candidate in class.instance_methods {
                let method_dialects = candidate.surface.or(owner.surface);
                if !candidate.available_for_version(package_floor)
                    || !method_dialects.is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
                {
                    continue;
                }
                if method_names.insert(candidate.name) {
                    visible_methods.push((owner, candidate));
                }
            }
            for superclass in class.superclasses {
                if let Some(owner) = self.get_for_surface(superclass, dialect) {
                    queue.push_back(owner);
                } else if require_complete {
                    return Err(InstanceMethodContextUnavailable::SuperclassSelection(
                        superclass,
                    ));
                }
            }
        }
        Ok(visible_methods)
    }

    /// Complete inherited source catalogue under the exact retained context/store.
    /// Missing provider/table correspondence remains typed unavailable, never an
    /// empty method table or an unknown-method verdict.
    pub fn instance_method_providers_for_descriptor_in_context(
        &self,
        class_spec: &CommandSpec,
        context: &crate::model::ContextRegistry,
    ) -> Result<Vec<(&'static CommandSpec, &'static SubCommand)>, InstanceMethodContextUnavailable>
    {
        if !std::ptr::eq(self, context.commands().as_ref()) {
            return Err(InstanceMethodContextUnavailable::RegistryMismatch);
        }
        let availability = context.context();
        if !availability
            .resolve_spec(self, class_spec.name)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
        {
            return Err(InstanceMethodContextUnavailable::FactorySelection);
        }
        self.instance_method_providers_with_context_status(
            class_spec,
            Some(availability.authoring_query()),
            |owner| {
                availability
                    .resolve_spec(self, owner.name)
                    .filter(|selected| std::ptr::eq(*selected, owner))
                    .map(|selected| Self::instance_method_provider_floor(selected, context))
            },
            true,
        )
    }

    fn instance_method_provider_floor<'c>(
        owner: &CommandSpec,
        context: &'c crate::model::ContextRegistry,
    ) -> Option<&'c str> {
        let availability = context.context();
        let package = availability
            .keyed_ambient_placement(owner)
            .map(|placement| placement.package.as_ref())
            .or_else(|| owner.owning_package())?;
        availability
            .placement_floor(package)
            .map(tcl_dialect::model::Version::as_str)
    }

    /// Keyword classification from one complete actual inherited provider table.
    /// Unavailable context remains separate from a genuine `KeywordMatch::Unknown`.
    pub fn instance_method_selection_for_descriptor_in_context(
        &self,
        class_spec: &CommandSpec,
        method: &str,
        context: &crate::model::ContextRegistry,
    ) -> Result<crate::abbrev::KeywordMatch<'static>, InstanceMethodContextUnavailable> {
        let providers =
            self.instance_method_providers_for_descriptor_in_context(class_spec, context)?;
        Self::instance_method_provider_keyword(class_spec, &providers, method)
    }

    fn instance_method_provider_keyword(
        class_spec: &CommandSpec,
        providers: &[(&'static CommandSpec, &'static SubCommand)],
        method: &str,
    ) -> Result<crate::abbrev::KeywordMatch<'static>, InstanceMethodContextUnavailable> {
        let class =
            class_spec
                .object_class
                .ok_or(InstanceMethodContextUnavailable::MethodTable(
                    class_spec.name,
                ))?;
        Ok(KeywordTable::from_keywords(
            providers.iter().map(|(_, method)| Keyword {
                name: method.name,
                min_abbrev: method.min_abbrev,
            }),
            class.method_prefix_matching,
        )
        .resolve(method))
    }

    /// Resolve an instance method `method` on class `class_name`, walking
    /// declared superclasses breadth-first. Lifecycle-gated methods are
    /// filtered against their owning class command's resolved package floor.
    /// Returns the owning class's [`SubCommand`] method spec, or `None` when
    /// unresolved.
    #[must_use]
    pub fn instance_method(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<&crate::spec::SubCommand> {
        self.instance_method_at(class_name, method, None, self.own_surface_query())
    }

    /// [`Self::instance_method`] with a document-resolved owning-package
    /// floor and explicit surface point. See [`Self::instance_methods_at`]
    /// for the two axes' composition.
    #[must_use]
    pub fn instance_method_at(
        &self,
        class_name: &str,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&crate::spec::SubCommand> {
        self.instance_method_for_descriptor_at(
            self.get_for_surface(class_name, dialect)?,
            method,
            package_version,
            dialect,
        )
    }

    /// Select an authored instance method from the same retained factory
    /// descriptor, with the shared lifecycle and prefix-ambiguity rules.
    #[must_use]
    pub fn instance_method_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static SubCommand> {
        let canonical = self
            .instance_method_selection_for_descriptor_at(
                class_spec,
                method,
                package_version,
                dialect,
            )?
            .unique()?;
        self.instance_methods_for_descriptor_at(class_spec, package_version, dialect)
            .into_iter()
            .find(|candidate| candidate.name == canonical)
    }

    /// Classify a source method spelling from its independently retained
    /// descriptor, selected lifecycle and shared abbreviation owner.
    #[must_use]
    pub fn instance_method_selection_for_descriptor_at(
        &self,
        class_spec: &CommandSpec,
        method: &str,
        package_version: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::abbrev::KeywordMatch<'static>> {
        self.get_for_surface(class_spec.name, dialect)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
            .then_some(())?;
        let methods = self.instance_methods_for_descriptor_at(class_spec, package_version, dialect);
        let matching = class_spec.object_class?.method_prefix_matching;
        Some(
            KeywordTable::from_keywords(
                methods.iter().map(|method| Keyword {
                    name: method.name,
                    min_abbrev: method.min_abbrev,
                }),
                matching,
            )
            .resolve(method),
        )
    }

    /// Resolve a concrete object-instance invocation to the same
    /// target-neutral semantic projection used for ordinary commands.
    ///
    /// `args` starts with the method word and is followed by its arguments.
    /// The class/factory command supplies the registry-owned method table, but
    /// constructor-only command traits and effects are deliberately not
    /// inherited by the instance call. A matching [`SubCommandForm`] may
    /// replace the method row's coarse traits, mutator flag, and legacy side
    /// effects for the concrete argument shape.
    ///
    /// [`SubCommandForm`]: crate::forms::SubCommandForm
    #[must_use]
    pub fn resolve_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        receiver: &'w str,
        args: &'w [&'w str],
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        self.resolve_structured_instance_invocation(
            class_name,
            InvocationWords::literals(receiver, args),
            dialect,
        )
    }

    /// Source-aware companion to [`Self::resolve_instance_invocation`].
    ///
    /// The receiver is the invocation head. Only a literal method word can
    /// select the class method table. Form-level literal selectors likewise
    /// inspect only known literal words; a substituted operation word keeps
    /// the conservative method-row semantics rather than guessing a form.
    #[must_use]
    pub fn resolve_structured_instance_invocation<'r, 'w>(
        &'r self,
        class_name: &str,
        words: InvocationWords<'w>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        let class_spec = if dialect.is_none() {
            self.get(class_name)?
        } else {
            self.get_for_surface(class_name, dialect)?
        };
        if !self.effect_provider_is_admitted(class_spec, dialect) {
            return None;
        }
        self.resolve_structured_instance_invocation_for_descriptor(class_spec, words, None, dialect)
    }

    /// Source-aware instance schema from an independently selected factory
    /// descriptor and its exact original effective argv. The Registry identity,
    /// lifecycle and method prefix rules remain shared with the named API.
    #[must_use]
    pub fn resolve_structured_instance_invocation_for_descriptor<'r, 'w>(
        &'r self,
        class_spec: &'r CommandSpec,
        words: InvocationWords<'w>,
        package_version: Option<&'w str>,
        dialect: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let words = words.with_profile(self.profile());
        let arguments = words.arguments();
        let dialect = dialect.or_else(|| {
            arguments
                .dialect()
                .and_then(crate::InvocationDialect::authoring_query)
        });
        let method_spelling = arguments.literal_at(0)?;
        self.get_for_surface(class_spec.name, dialect)
            .is_some_and(|selected| std::ptr::eq(selected, class_spec))
            .then_some(())?;
        class_spec.object_class?;
        // This is the selected source schema. Package activation, allocation
        // and actual method execution are independent of its readonly roles.
        let method = self.instance_method_for_descriptor_at(
            class_spec,
            method_spelling,
            package_version,
            dialect,
        )?;
        let availability = crate::resolved_invocation::InvocationAvailability {
            query: dialect.or_else(|| {
                words
                    .arguments()
                    .dialect()
                    .and_then(crate::InvocationDialect::authoring_query)
            }),
            package_version: self.package_floor_for_spec_at(class_spec, package_version),
        };
        let form = pick_form(class_spec, Some(method), arguments, availability);
        let resolved_method = ResolvedSubcommand {
            spelling: method_spelling,
            canonical_name: method.name,
        };
        let subcommand = if method_spelling == method.name {
            SubcommandResolution::Exact(resolved_method)
        } else {
            SubcommandResolution::UniquePrefix(resolved_method)
        };
        Some(ResolvedInvocation::new_instance(
            words,
            class_spec,
            method,
            form,
            subcommand,
            availability,
        ))
    }

    /// Original instance schema with exact full-context inherited provider ownership.
    /// Method/form/options use the selected provider's own package placement.
    /// Unknown or ambiguous keywords are `Ok(None)`; missing owner/context or
    /// unrepresented option axes are typed unavailable and grant no absence.
    pub fn resolve_structured_instance_invocation_for_descriptor_in_context<'r, 'w>(
        &'r self,
        class_spec: &'r CommandSpec,
        words: InvocationWords<'w>,
        context: &'w crate::model::ContextRegistry,
    ) -> Result<Option<ResolvedInvocation<'r, 'w>>, InstanceMethodContextUnavailable> {
        self.resolve_structured_instance_with_method_provider(class_spec, words, context)
    }

    // The selected provider, receiver axes and constructor remain one producer.
    fn resolve_structured_instance_with_method_provider<'r, 'w>(
        &'r self,
        class_spec: &'r CommandSpec,
        words: InvocationWords<'w>,
        context: &'w crate::model::ContextRegistry,
    ) -> Result<Option<ResolvedInvocation<'r, 'w>>, InstanceMethodContextUnavailable> {
        let words = words.with_profile(self.profile());
        let arguments = words.arguments();
        let providers =
            self.instance_method_providers_for_descriptor_in_context(class_spec, context)?;
        let Some(spelling) = arguments.literal_at(0) else {
            return Ok(None);
        };
        let Some(canonical) =
            Self::instance_method_provider_keyword(class_spec, &providers, spelling)?.unique()
        else {
            return Ok(None);
        };
        let Some((owner, method)) = providers
            .into_iter()
            .find(|(_, method)| method.name == canonical)
        else {
            return Ok(None);
        };
        let query = Some(context.context().authoring_query());
        let availability = crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec_at(
                owner,
                Self::instance_method_provider_floor(owner, context),
            ),
        };
        let receiver_availability = crate::resolved_invocation::InvocationAvailability {
            query,
            package_version: self.package_floor_for_spec_at(
                class_spec,
                Self::instance_method_provider_floor(class_spec, context),
            ),
        };
        let form = pick_form(owner, Some(method), arguments, availability);
        let resolved_method = ResolvedSubcommand {
            spelling,
            canonical_name: method.name,
        };
        let subcommand = if spelling == method.name {
            SubcommandResolution::Exact(resolved_method)
        } else {
            SubcommandResolution::UniquePrefix(resolved_method)
        };
        ResolvedInvocation::new_instance_with_method_provider(
            words,
            class_spec,
            owner,
            method,
            form,
            subcommand,
            crate::resolved_invocation::InstanceMethodAvailability {
                provider: availability,
                receiver: receiver_availability,
            },
        )
        .map(Some)
    }

    /// Resolve the [`ArgRole::CommandPrefix`] positions and appended arities for
    /// an instance-method dispatch `$obj method method_args…`.
    ///
    /// `method_args` are the words *after* the method name.  Mirrors the
    /// subcommand arm of [`Self::command_prefixes`] (static
    /// [`SubCommand::command_prefixes`] table ∪ `command_prefix_resolver` ∪
    /// command-prefix options), but keyed on the object's class + method rather
    /// than a top-level command — so `$g walk … -command cb` (option value) and
    /// `$t walkproc … cb` (trailing positional, resolver) light up the same
    /// references / call-graph / W123 / arity substrate.  Returned indices are
    /// relative to `method_args` (0 = first word after the method name).
    ///
    /// [`SubCommand::command_prefixes`]: crate::spec::SubCommand::command_prefixes
    #[must_use]
    pub fn instance_method_command_prefixes(
        &self,
        class_name: &str,
        method: &str,
        method_args: &[&str],
    ) -> Vec<(usize, AppendedArity)> {
        self.instance_method_command_prefixes_with_arguments(
            class_name,
            method,
            CommandPrefixArguments::literals(method_args),
        )
    }

    /// Source-aware companion to [`Self::instance_method_command_prefixes`].
    /// Literal-sensitive resolvers can distinguish runtime substitutions from
    /// values proved at the call site.
    #[must_use]
    pub fn instance_method_command_prefixes_structured<'w>(
        &self,
        class_name: &str,
        method: &str,
        spellings: &'w [&'w str],
        words: &'w [InvocationWord<'w>],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(arguments) = CommandPrefixArguments::structured(spellings, words) else {
            return Vec::new();
        };
        self.instance_method_command_prefixes_with_arguments(class_name, method, arguments)
    }

    fn instance_method_command_prefixes_with_arguments(
        &self,
        class_name: &str,
        method: &str,
        method_args: CommandPrefixArguments<'_>,
    ) -> Vec<(usize, AppendedArity)> {
        self.instance_method(class_name, method)
            .map_or_else(Vec::new, |m| sub_command_prefixes(m, method_args))
    }
}
