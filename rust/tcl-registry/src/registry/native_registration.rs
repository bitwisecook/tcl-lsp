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

//! Native command registrations: compiler hooks, implementation slots, registration lookup and admission.

use super::effective_semantics::jim_fresh_command_names;
use super::invocation_resolution::{
    pick_form, resolve_semantic_subcommand, successful_handler_contracts,
};
use super::{CommandRegistry, PrerequisiteEntry, PrerequisiteIndex, RegistrationSlots};
use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::resolved_invocation::SubcommandResolution;
use crate::spec::CommandSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, PoisonError};
use tcl_dialect::model::{Family, SurfaceQuery, surface_admits};

impl CommandRegistry {
    /// Original private compiler slots in a trusted stock entry environment.
    /// This catalogue projection supplies descriptors and fresh-entry names;
    /// a live runtime snapshot must independently supply each actual identity.
    #[must_use]
    pub fn native_compiler_implementation_slots(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<crate::native_compilation::NativeCompilerImplementationLookup> {
        let Some(query) = dialect.authoring_query() else {
            return Vec::new();
        };
        let mut slots = BTreeMap::new();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            for compilation in
                spec.native_compilation
                    .into_iter()
                    .chain(spec.subcommands.iter().flat_map(|subcommand| {
                        subcommand.native_compilation.into_iter().chain(
                            subcommand
                                .sub_subcommands
                                .iter()
                                .filter_map(|worker| worker.native_compilation),
                        )
                    }))
            {
                for lookup in compilation
                    .implementation_prerequisites(dialect)
                    .into_iter()
                    .flatten()
                {
                    slots.insert(lookup.slot, lookup);
                }
            }
        }
        slots.into_values().collect()
    }

    /// Original private handler and compiler slots in an explicitly trusted
    /// stock entry. This bootstrap surface establishes neither live identity
    /// nor compiler-hook availability; those proofs use separate descriptors.
    #[must_use]
    pub fn stock_native_implementation_slots(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<crate::native_compilation::NativeCompilerImplementationLookup> {
        let mut slots: BTreeMap<_, _> = self
            .native_compiler_implementation_slots(dialect)
            .into_iter()
            .map(|lookup| (lookup.slot, lookup))
            .collect();
        if dialect.family() != Some(Family::Tcl) {
            return slots.into_values().collect();
        }
        let Some(query) = dialect.authoring_query() else {
            return slots.into_values().collect();
        };
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            for contract in successful_handler_contracts(spec) {
                for lookup in contract.stock_native_workers(dialect) {
                    slots.insert(lookup.slot, lookup);
                }
            }
        }
        slots.into_values().collect()
    }

    /// Native compiler descriptor of one actual stock registration identity.
    ///
    /// `identity` is an already constructed canonical command key. Private
    /// worker registrations reverse-project their authored parent and frozen
    /// prefix before selecting the member descriptor. Ambiguous mappings or
    /// unavailable descriptors retain uncertainty; this never proves that a
    /// live command has the stock implementation.
    #[must_use]
    pub fn native_compilation_for_registration(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let key = (dialect, identity.to_owned());
        if let Some(cached) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .compilers
            .get(&key)
        {
            return *cached;
        }
        let selected = self.native_compilation_for_registration_uncached(&key.1, dialect);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .compilers
            .insert(key, selected);
        selected
    }

    pub(super) fn native_compilation_for_registration_uncached(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let key = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
        let query = dialect.authoring_query()?;
        if let Some(spec) = self.native_registration_exact_descriptor(identity, dialect) {
            return spec
                .native_compilation
                .or_else(|| crate::native_tcloo_registration::compilation(identity, dialect));
        }
        if let Some(compilation) = crate::native_tcloo_registration::compilation(identity, dialect)
        {
            return Some(compilation);
        }
        let mut mapping = None;
        let mut selected = None;
        let mut found = false;
        let index = self.prerequisite_index(dialect, query);
        for entry in index.get(key).into_iter().flatten() {
            if mapping.is_some_and(|previous| previous != entry.lookup) {
                return None;
            }
            mapping = Some(entry.lookup);
            let candidate = if entry.terminal {
                // The terminal registration owns a compiler descriptor
                // even when its semantic operand offset cannot be rebased.
                entry.compilation
            } else {
                // Registration owns this descriptor independently of
                // invocation argc; selecting a member needs no fake argv.
                self.native_registration_compiler_descriptor(identity, dialect)?
            };
            if found && selected != Some(candidate) {
                return None;
            }
            selected = Some(candidate);
            found = true;
        }
        selected.or_else(|| self.native_registration_compiler_descriptor(identity, dialect))
    }

    /// Every implementation-path step of every visible spec, in registry scan
    /// order: the walk the selection did before the prerequisite index.
    ///
    /// Taken once per registry and dialect. The per-spec work (visibility,
    /// implementation paths, unrooted slot keys) does not depend on the
    /// identity being selected, so repeating it per identity only made the
    /// equivalence test quadratic.
    #[cfg(test)]
    pub(super) fn native_registration_full_scan(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<(&'static str, PrerequisiteEntry)> {
        let Some(query) = dialect.authoring_query() else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compilations =
                spec.native_compilation
                    .into_iter()
                    .chain(spec.subcommands.iter().flat_map(|member| {
                        member.native_compilation.into_iter().chain(
                            member
                                .sub_subcommands
                                .iter()
                                .filter(|worker| member.admits_sub_subcommand(worker, query))
                                .filter_map(|worker| worker.native_compilation),
                        )
                    }));
            for compilation in compilations {
                let Some(path) = compilation.implementation_prerequisites(dialect) else {
                    continue;
                };
                for lookup in &path {
                    let terminal = matches!(
                        compilation.grammar,
                        crate::native_compilation::NativeCompilationGrammar::WithImplementationPath { .. }
                    ) && path.last() == Some(lookup);
                    steps.push((
                        tcl_syntax::naming::unroot_rooted_key(lookup.slot).unwrap_or(lookup.slot),
                        PrerequisiteEntry {
                            compilation,
                            lookup: *lookup,
                            terminal,
                        },
                    ));
                }
            }
        }
        steps
    }

    /// The selection before the prerequisite index: filter the full scan's
    /// steps by slot instead of looking the slot up in the index.
    #[cfg(test)]
    pub(super) fn native_compilation_for_registration_full_scan(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
        scan: &[(&'static str, PrerequisiteEntry)],
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let key = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
        dialect.authoring_query()?;
        if let Some(spec) = self.native_registration_exact_descriptor(identity, dialect) {
            return spec
                .native_compilation
                .or_else(|| crate::native_tcloo_registration::compilation(identity, dialect));
        }
        if let Some(compilation) = crate::native_tcloo_registration::compilation(identity, dialect)
        {
            return Some(compilation);
        }
        let mut mapping = None;
        let mut selected = None;
        let mut found = false;
        for (_, step) in scan.iter().filter(|(slot, _)| *slot == key) {
            if mapping.is_some_and(|previous| previous != step.lookup) {
                return None;
            }
            mapping = Some(step.lookup);
            let candidate = if step.terminal {
                // The terminal registration owns a compiler descriptor
                // even when its semantic operand offset cannot be rebased.
                step.compilation
            } else {
                // Registration owns this descriptor independently of
                // invocation argc; selecting a member needs no fake argv.
                self.native_registration_compiler_descriptor(identity, dialect)?
            };
            if found && selected != Some(candidate) {
                return None;
            }
            selected = Some(candidate);
            found = true;
        }
        selected.or_else(|| self.native_registration_compiler_descriptor(identity, dialect))
    }

    /// The implementation-path entries of every visible compilation for
    /// `dialect`, built once per registry and dialect.
    fn prerequisite_index(
        &self,
        dialect: crate::InvocationDialect,
        query: SurfaceQuery<'_>,
    ) -> Arc<PrerequisiteIndex> {
        if let Some(index) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .prerequisites
            .get(&dialect)
        {
            return Arc::clone(index);
        }
        let mut index = PrerequisiteIndex::default();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compilations =
                spec.native_compilation
                    .into_iter()
                    .chain(spec.subcommands.iter().flat_map(|member| {
                        member.native_compilation.into_iter().chain(
                            member
                                .sub_subcommands
                                .iter()
                                .filter(|worker| member.admits_sub_subcommand(worker, query))
                                .filter_map(|worker| worker.native_compilation),
                        )
                    }));
            for compilation in compilations {
                let Some(path) = compilation.implementation_prerequisites(dialect) else {
                    continue;
                };
                let owns = matches!(
                    compilation.grammar,
                    crate::native_compilation::NativeCompilationGrammar::WithImplementationPath { .. }
                );
                for lookup in &path {
                    index
                        .entry(
                            tcl_syntax::naming::unroot_rooted_key(lookup.slot)
                                .unwrap_or(lookup.slot),
                        )
                        .or_default()
                        .push(PrerequisiteEntry {
                            compilation,
                            lookup: *lookup,
                            terminal: owns && path.last() == Some(lookup),
                        });
                }
            }
        }
        let index = Arc::new(index);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .prerequisites
            .insert(dialect, Arc::clone(&index));
        index
    }

    /// Select the exact authored registration key, without written-name fallback.
    pub(crate) fn native_registration_exact_descriptor(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<&'static CommandSpec> {
        let key = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
        let query = dialect.authoring_query()?;
        let rooted = tcl_syntax::naming::root_unrooted_key(key);
        let exact = |label: &str| {
            self.by_name
                .get(label)
                .and_then(|specs| self.best_visible(specs, Some(query)))
                .filter(|spec| {
                    tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name) == key
                })
        };
        exact(key).or_else(|| exact(&rooted))
    }

    /// A normal worker roster can publish a registration independently of a
    /// compiler implementation path. Select only its authored frozen member
    /// descriptor; operand cardinality and normal-handler facts play no part.
    fn native_registration_compiler_descriptor(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let lookup = self.native_registration_lookup(identity, dialect)?;
        let query = dialect.authoring_query()?;
        let spec = self.get_for_surface(lookup.command, Some(query))?;
        if spec.owning_package().is_some() {
            return None;
        }
        match lookup.prepended {
            [] => spec.native_compilation,
            [member] => {
                let selected = spec.resolve_subcommand_for_dialect(member, Some(query))?;
                (selected.name == *member)
                    .then_some(selected.native_compilation)
                    .flatten()
                    .map(|mut compiler| {
                        if matches!(compiler.grammar, crate::native_compilation::NativeCompilationGrammar::StringMatch(_)) {
                            compiler.grammar = crate::native_compilation::NativeCompilationGrammar::StringMatch(crate::native_string_compilation::NativeStringMatchScope::PrivateOperands);
                        }
                        compiler.grammar = match compiler.grammar {
                            crate::native_compilation::NativeCompilationGrammar::StringTrim { operation, .. } => crate::native_compilation::NativeCompilationGrammar::StringTrim { scope: crate::native_scalar_compilation::NativeScalarScope::PrivateOperands, operation },
                            crate::native_compilation::NativeCompilationGrammar::StringEqual(_) => crate::native_compilation::NativeCompilationGrammar::StringEqual(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringRange(_) => crate::native_compilation::NativeCompilationGrammar::StringRange(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringReplace(_) => crate::native_compilation::NativeCompilationGrammar::StringReplace(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringInsert(_) => crate::native_compilation::NativeCompilationGrammar::StringInsert(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringMap(_) => crate::native_compilation::NativeCompilationGrammar::StringMap(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringLength(_) => crate::native_compilation::NativeCompilationGrammar::StringLength(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringIsAscii(_) => crate::native_compilation::NativeCompilationGrammar::StringIsAscii(crate::native_scalar_compilation::NativeScalarScope::PrivateOperands),
                            crate::native_compilation::NativeCompilationGrammar::StringOperand { operation, .. } => crate::native_compilation::NativeCompilationGrammar::StringOperand { scope: crate::native_scalar_compilation::NativeScalarScope::PrivateOperands, operation },
                            grammar => grammar,
                        };
                        compiler
                    })
            }
            [member, operation] => {
                let selected = spec.resolve_subcommand_for_dialect(member, Some(query))?;
                if selected.name != *member {
                    return None;
                }
                let operation_spec = selected.resolve_sub_subcommand_gated(
                    operation,
                    Some(query),
                    self.package_floor_for_spec(spec),
                )?;
                (operation_spec.name == *operation)
                    .then_some(operation_spec.native_compilation)
                    .flatten()
            }
            _ => None,
        }
    }

    /// Backend metadata selected from typed original native compiler words.
    /// This query neither proves admission nor grants normal handler effects.
    #[must_use]
    pub fn native_compiler_hooks(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<NativeCompilerHooks> {
        let dialect = words.arguments().dialect()?;
        if self
            .native_registration_lookup(words.head_literal()?, dialect)
            .is_some()
        {
            return self.with_native_registration(words, |projected| {
                self.native_compiler_hooks(projected)
            });
        }
        let query = dialect.authoring_query()?;
        let spec = self.get_for_surface(words.head_literal()?, Some(query))?;
        let (subcommand, sub) = resolve_semantic_subcommand(
            spec,
            words.arguments(),
            Some(query),
            self.package_floor_for_spec(spec),
            Some(dialect.numbers),
        );
        if !spec.subcommands.is_empty()
            && !words.arguments().is_empty()
            && sub.is_none()
            && subcommand != SubcommandResolution::NotApplicable
        {
            return None;
        }
        let form = if let Some(sub) = sub {
            pick_form(
                spec,
                Some(sub),
                words.arguments(),
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(query),
                    package_version: self.package_floor_for_spec(spec),
                },
            )
        } else {
            pick_form(
                spec,
                None,
                words.arguments(),
                crate::resolved_invocation::InvocationAvailability {
                    query: Some(query),
                    package_version: self.package_floor_for_spec(spec),
                },
            )
        };
        Some(NativeCompilerHooks {
            lowering: form
                .and_then(|form| form.lowering_hook)
                .or(sub.and_then(|sub| sub.lowering_hook))
                .or(spec.lowering_hook),
            codegen: form
                .and_then(|form| form.codegen_hook)
                .or(sub.and_then(|sub| sub.codegen_hook))
                .or(spec.codegen_hook),
            inline: sub
                .and_then(|sub| sub.inline_codegen_hook)
                .or(spec.inline_codegen_hook),
        })
    }

    /// Logical member contract of an actual stock private registration.
    /// This metadata projection proves no live slot or implementation identity.
    #[must_use]
    pub fn native_registration_invocation_facts(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<crate::InvocationFacts> {
        self.native_registration_facts(words, false)
    }

    /// Inert selector operands used only by a private registration's backend
    /// metadata layout. These are never runtime arguments to that private slot.
    #[must_use]
    pub fn native_registration_operand_prefix(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<&'static [&'static str]> {
        Some(
            self.native_registration_lookup(identity, dialect)?
                .prepended,
        )
    }

    /// Original selected descriptor of a stock registration's logical source.
    /// Private registrations use the canonical registry-authored parent lookup;
    /// an explicit descriptor at the identity retains its own provenance. This
    /// supplies metadata without a live token, argument layout or completion.
    #[must_use]
    pub fn native_registration_source_descriptor(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Option<&'static CommandSpec> {
        let query = dialect.authoring_query()?.with_realm(realm);
        if let Some(spec) = self.get_for_surface(identity, Some(query)) {
            return Some(spec);
        }
        let lookup = self.native_registration_lookup(identity, dialect)?;
        self.get_for_surface(lookup.command, Some(query))
    }

    /// Intrinsic descriptors authored for one native registration. This is
    /// registration metadata, independent of invocation cardinality; the caller
    /// must separately prove the live stock token and any ensemble mapping.
    #[must_use]
    pub fn native_registration_intrinsic_ids(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<Vec<crate::IntrinsicId>> {
        let query = dialect.authoring_query()?;
        let lookup = self.native_registration_lookup(identity, dialect);
        let (command, prefix) = lookup.map_or((identity, &[][..]), |lookup| {
            (lookup.command, lookup.prepended)
        });
        let spec = self.get_for_surface(command, Some(query))?;
        if spec.owning_package().is_some() {
            return None;
        }
        let mut ids = BTreeSet::new();
        let mut add = |semantic, lowering, codegen, inline| {
            if let Some(id) =
                crate::IntrinsicId::from_descriptors(semantic, lowering, codegen, inline)
            {
                ids.insert(id);
            }
        };
        if prefix.is_empty() {
            add(
                spec.semantic_operation,
                spec.lowering_hook,
                spec.codegen_hook,
                spec.inline_codegen_hook,
            );
            for form in spec.command_forms.iter().filter(|form| {
                form.surface
                    .is_none_or(|gate| surface_admits(gate, Some(&query)))
            }) {
                add(
                    form.semantic_operation,
                    form.lowering_hook,
                    form.codegen_hook,
                    None,
                );
            }
        }
        for member in spec.subcommands.iter().filter(|member| {
            prefix.first().is_none_or(|name| *name == member.name)
                && member
                    .surface
                    .is_none_or(|gate| surface_admits(gate, Some(&query)))
        }) {
            if prefix.len() <= 1 {
                add(
                    member.semantic_operation,
                    member.lowering_hook,
                    member.codegen_hook,
                    member.inline_codegen_hook,
                );
                for form in member.subcommand_forms.iter().filter(|form| {
                    form.surface
                        .is_none_or(|gate| surface_admits(gate, Some(&query)))
                }) {
                    add(
                        form.semantic_operation,
                        form.lowering_hook,
                        form.codegen_hook,
                        None,
                    );
                }
            }
        }
        Some(ids.into_iter().collect())
    }

    /// Normal-edge facts for the same actual private registration.
    #[must_use]
    pub fn native_registration_success_facts(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Option<crate::InvocationFacts> {
        self.native_registration_facts(words, true)
    }

    fn native_registration_facts(
        &self,
        words: crate::InvocationWords<'_>,
        successful: bool,
    ) -> Option<crate::InvocationFacts> {
        let dialect = words.arguments().dialect()?;
        let lookup = self.native_registration_lookup(words.head_literal()?, dialect)?;
        let mut facts = self.with_native_registration(words, |projected| {
            let resolved = self.resolve_structured_invocation(projected, dialect.authoring_query());
            let invocation = resolved.resolved()?;
            Some(if successful {
                invocation.facts_after_success()
            } else {
                invocation.facts()
            })
        })?;
        facts.argument_offset = facts.argument_offset.checked_sub(lookup.prepended.len())?;
        if let crate::state_transition::StateTransitionKnowledge::Declared(transitions) =
            &mut facts.state_transitions
        {
            *transitions = transitions
                .project_argument_indices(|index| index.checked_sub(lookup.prepended.len()))?;
        }
        if let Some(compilation) = &mut facts.native_compilation
            && let crate::native_compilation::NativeCompilationGrammar::Dictionary {
                ensemble, ..
            } = &mut compilation.grammar
        {
            *ensemble = false;
        }
        Some(facts)
    }

    /// Completion contract of an actual private registration in its physical frame.
    /// Unknown argument values and expansions remain typed during projection.
    #[must_use]
    pub fn native_registration_completion_route(
        &self,
        words: crate::InvocationWords<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        let dialect = words.arguments().dialect()?;
        self.with_native_registration(words, |projected| {
            self.invocation_completion_route_in_frame(
                projected.head_literal()?,
                projected.arguments(),
                dialect.authoring_query(),
                frame,
            )
        })
    }

    fn with_native_registration<T>(
        &self,
        words: crate::InvocationWords<'_>,
        project: impl FnOnce(crate::InvocationWords<'_>) -> Option<T>,
    ) -> Option<T> {
        let dialect = words.arguments().dialect()?;
        let lookup = self.native_registration_lookup(words.head_literal()?, dialect)?;
        let mut arguments = lookup
            .prepended
            .iter()
            .copied()
            .map(crate::InvocationWord::Literal)
            .collect::<Vec<_>>();
        arguments
            .extend((0..words.arguments().len()).filter_map(|index| words.arguments().get(index)));
        project(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal(lookup.command),
                &arguments,
            )
            .with_dialect(dialect),
        )
    }

    pub(super) fn native_registration_lookup(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilerImplementationLookup> {
        let key = (dialect, identity.to_owned());
        if let Some(cached) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .lookups
            .get(&key)
        {
            return *cached;
        }
        let selected = self.native_registration_lookup_uncached(&key.1, dialect);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .lookups
            .insert(key, selected);
        selected
    }

    pub(super) fn native_registration_lookup_uncached(
        &self,
        identity: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::native_compilation::NativeCompilerImplementationLookup> {
        let key = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
        let query = dialect.authoring_query()?;
        self.registration_slots(dialect, query)
            .get(key)
            .copied()
            .flatten()
    }

    /// Every implementation-path and stock-worker slot a visible spec publishes
    /// for `dialect`, built once per registry and dialect.
    pub(super) fn registration_slots(
        &self,
        dialect: crate::InvocationDialect,
        query: SurfaceQuery<'_>,
    ) -> Arc<RegistrationSlots> {
        if let Some(slots) = self
            .native_registrations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .registration_slots
            .get(&dialect)
        {
            return Arc::clone(slots);
        }
        let mut slots = RegistrationSlots::default();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compiled = spec
                .native_compilation
                .into_iter()
                .chain(spec.subcommands.iter().flat_map(|member| {
                    member.native_compilation.into_iter().chain(
                        member
                            .sub_subcommands
                            .iter()
                            .filter_map(|worker| worker.native_compilation),
                    )
                }))
                .flat_map(|compilation| {
                    compilation
                        .implementation_prerequisites(dialect)
                        .into_iter()
                        .flatten()
                });
            let normal = successful_handler_contracts(spec)
                .flat_map(|contract| contract.stock_native_workers(dialect));
            for lookup in compiled.chain(normal) {
                let selected = slots
                    .entry(
                        tcl_syntax::naming::unroot_rooted_key(lookup.slot).unwrap_or(lookup.slot),
                    )
                    .or_insert(Some(lookup));
                // Two different lookups for one slot leave it ambiguous for good.
                if *selected != Some(lookup) {
                    *selected = None;
                }
            }
        }
        let slots = Arc::new(slots);
        self.native_registrations
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .registration_slots
            .insert(dialect, Arc::clone(&slots));
        slots
    }

    /// Every registration slot in registry scan order, as the selection walked
    /// them per identity before [`Self::registration_slots`]. Taken once per
    /// registry and dialect by the equivalence test.
    #[cfg(test)]
    pub(super) fn native_registration_lookup_scan(
        &self,
        dialect: crate::InvocationDialect,
    ) -> Vec<(
        &'static str,
        crate::native_compilation::NativeCompilerImplementationLookup,
    )> {
        let Some(query) = dialect.authoring_query() else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        for specs in self.by_name.values() {
            let Some(spec) = self.best_visible(specs, Some(query)) else {
                continue;
            };
            if spec.owning_package().is_some() {
                continue;
            }
            let compiled = spec
                .native_compilation
                .into_iter()
                .chain(spec.subcommands.iter().flat_map(|member| {
                    member.native_compilation.into_iter().chain(
                        member
                            .sub_subcommands
                            .iter()
                            .filter_map(|worker| worker.native_compilation),
                    )
                }))
                .flat_map(|compilation| {
                    compilation
                        .implementation_prerequisites(dialect)
                        .into_iter()
                        .flatten()
                });
            let normal = successful_handler_contracts(spec)
                .flat_map(|contract| contract.stock_native_workers(dialect));
            for lookup in compiled.chain(normal) {
                steps.push((
                    tcl_syntax::naming::unroot_rooted_key(lookup.slot).unwrap_or(lookup.slot),
                    lookup,
                ));
            }
        }
        steps
    }

    /// The registration lookup before [`Self::registration_slots`]: filter the
    /// scan's steps by slot.
    #[cfg(test)]
    pub(super) fn native_registration_lookup_full_scan(
        identity: &str,
        dialect: crate::InvocationDialect,
        scan: &[(
            &'static str,
            crate::native_compilation::NativeCompilerImplementationLookup,
        )],
    ) -> Option<crate::native_compilation::NativeCompilerImplementationLookup> {
        let key = tcl_syntax::naming::unroot_rooted_key(identity).unwrap_or(identity);
        dialect.authoring_query()?;
        let mut selected = None;
        for (_, lookup) in scan.iter().filter(|(slot, _)| *slot == key) {
            if selected.is_some_and(|previous| previous != *lookup) {
                return None;
            }
            selected = Some(*lookup);
        }
        selected
    }

    /// Measured native admission for a catalogued command. A custom binding
    /// outside this catalogue has no stock-command admission answer.
    #[must_use]
    pub fn native_command_admission(
        &self,
        name: &str,
        dialect: crate::InvocationDialect,
    ) -> Option<bool> {
        self.get(name)?;
        Self::native_stock_registration_admission(dialect, name)
    }

    /// Measured admission for an actual stock-command registration, independent
    /// of catalogue ancestry. Unknown releases/builds remain unknown.
    #[must_use]
    pub fn native_stock_registration_admission(
        dialect: crate::InvocationDialect,
        name: &str,
    ) -> Option<bool> {
        // This is a stock implementation admission query. A live custom
        // native binding is not constrained by the fresh engine's roster.
        // Catalogue absence is not permission for an original stock identity.
        // naming.mathop-binding-and-written-head-controls
        // docs/design/analysis/name-resolution-proofs/mathop-binding-and-written-head-controls.md
        (dialect.core_point
            == Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )))
        .then(|| jim_fresh_command_names().contains(&crate::rooted_command_descriptor_label(name)))
    }
}

/// Backend hooks of a typed native compiler invocation, without effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCompilerHooks {
    /// Structured lowering selected by the original compiler words.
    pub lowering: Option<LoweringHookId>,
    /// Statement emitter selected by the original compiler words.
    pub codegen: Option<CodegenHookId>,
    /// Value emitter selected by the original compiler words.
    pub inline: Option<InlineCodegenHookId>,
}
