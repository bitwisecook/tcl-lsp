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

//! The class of each created instance: constructor calls, registry factories, definer grammars and metaclass chains.

use super::super::state::Analyser;
use tcl_lexer::Span;

impl Analyser {
    /// Retain class presentation from genuine original construction sites.
    /// Setter and direct named calls use their shared source receipts, which
    /// identify canonical declarations and selected family layouts. These
    /// labels are conditional source metadata, without allocation or lifetime.
    /// Per-item replay captures only original producer sites and later joins
    /// them to the grafted canonical declarations at the same source offsets.
    pub(crate) fn record_instance_creation(
        &mut self,
        cmd_name: &str,
        args: &[String],
        creation_ns: &str,
        site_offset: u32,
    ) {
        // Source class labels retain their genuine naming/setter declarations.
        // Successful publications and lifetime remain with the shared bindings.
        let bound_registry_factory = self.record_registry_factory_instance(site_offset);

        // Per-item bodies retain the exact source producer site until their
        // canonical declarations join the shell metadata during replay.
        if self.pending_instances.is_some() {
            let shape_a =
                crate::registry_invocation::source_structure::source_handle_construction_at(
                    &self.source,
                    &self.result,
                    site_offset,
                )
                .is_some();
            let shape_b = crate::registry_invocation::source_structure::source_constructor_call_at(
                &self.source,
                &self.result,
                site_offset,
            )
            .is_some();
            // A registry factory already bound above needs no user-class replay.
            if (shape_a || shape_b)
                && !bound_registry_factory
                && let Some(pending) = self.pending_instances.as_mut()
            {
                pending.push((
                    cmd_name.to_owned(),
                    args.to_vec(),
                    creation_ns.to_owned(),
                    site_offset,
                ));
            }
            return;
        }
        if bound_registry_factory {
            return;
        }
        // Genuine user-class setter syntax joins its source constructor.
        if let Some((variable, class_q)) = self
            .class_from_constructor_subst(site_offset)
            .or_else(|| self.jim_class_from_constructor_subst(site_offset))
        {
            self.result.instance_classes.insert(variable, class_q);
            return;
        }
        // Direct named construction retains its genuine original factory,
        // family layout and canonical class declaration. This is source class
        // presentation only; successful publication belongs to binding state.
        // A logical source input pins no interpreter: its class is the
        // retained logical declaration, and the label is conditional.
        let logical = self.result.allows_retained_logical_declaration_advice();
        if let Some(call) = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            site_offset,
        ) && let Some(shape) = if logical {
            call.logical_constructor_shape(&self.result)
        } else {
            call.constructor_shape(&self.result)
        } {
            let arguments = call.arguments();
            let name_at = match shape {
                crate::command_binding::OriginalSourceConstructorShape::Method(method) => {
                    method.names_instance_at.map(usize::from)
                }
                crate::command_binding::OriginalSourceConstructorShape::BareWord { .. } => Some(0),
            };
            if let Some(name_at) = name_at
                && let Some(name) = arguments
                    .get(name_at)
                    .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                && !name.is_empty()
                && let Some(class) = if logical {
                    call.class_declaration().logical_source_class(&self.result)
                } else {
                    call.class_declaration().source_class(&self.result).map(
                        crate::signature_scan::original_name::SourceDeclarationMetadata::metadata,
                    )
                }
            {
                let class_q = class.qualified_name.clone();
                self.result
                    .instance_classes
                    .insert(name.to_owned(), class_q.clone());
                let binding = super::super::types::InstanceCommandBinding {
                    qualified_name: crate::naming::qualify(creation_ns, name),
                    class_q,
                };
                if !self.result.instance_command_bindings.contains(&binding) {
                    self.result.instance_command_bindings.push(binding);
                }
            }
        }
    }

    /// Retain source class labels from genuine named-factory/setter receipts.
    /// This compatibility presentation supplies no successful creation or life.
    fn record_registry_factory_instance(&mut self, site_offset: u32) -> bool {
        use tcl_registry::AuthoredSourceCommandPublicationKind::Instance;
        let named = crate::registry_invocation::source_structure::source_command_publication_at(
            &self.source,
            &self.result,
            site_offset,
        )
        .and_then(|publication| {
            let Instance { class_name } = publication.kind() else {
                return None;
            };
            Self::factory_package_proved(&self.result, &publication)?;
            let name = std::str::from_utf8(publication.name_bytes()).ok()?;
            Some((name.to_owned(), class_name.to_owned()))
        });
        if let Some((name, class)) = named {
            self.bind_registry_instance_class(name, class);
            return true;
        }
        let captured = crate::registry_invocation::source_structure::source_handle_class_advice_at(
            &self.source,
            &self.result,
            site_offset,
        )
        .and_then(|binding| {
            let Instance { class_name } = binding.factory().kind() else {
                return None;
            };
            Self::factory_package_proved(&self.result, binding.factory())?;
            Some((
                std::str::from_utf8(binding.variable_bytes())
                    .ok()?
                    .to_owned(),
                class_name.to_owned(),
            ))
        });
        if let Some((name, class)) = captured {
            self.bind_registry_instance_class(name, class);
            return true;
        }
        false
    }

    /// A factory whose owning package is not proved present names no instance.
    fn factory_package_proved(
        analysis: &super::super::types::AnalysisResult,
        factory: &crate::registry_invocation::OriginalSourceCommandPublication,
    ) -> Option<()> {
        let context = analysis
            .resolved_input
            .as_ref()?
            .borrowed_context_registry();
        let descriptor = factory
            .original_invocation()
            .with_source_schema(context, |schema| {
                schema.authored_source_descriptors().command
            })?;
        analysis
            .retained_command_realm()?
            .source_bindings_ref()
            .package_factory_is_proved_at(factory.invocation_site(), descriptor)
            .then_some(())
    }

    /// Collision-safe registry instance reporting metadata. Differing class
    /// labels withdraw an ambiguous entry for the complete file. A remaining
    /// entry is presentation only; widget diagnostics independently require
    /// `source_registered_instance_words_at` and the genuine selected schema.
    /// The map supplies no Native receiver, handler or successful construction.
    fn bind_registry_instance_class(&mut self, name: String, class: String) {
        if self.result.ambiguous_instance_names.contains(&name) {
            return;
        }
        match self.result.instance_classes.get(&name) {
            Some(existing) if *existing != class => {
                self.result.instance_classes.remove(&name);
                self.result.ambiguous_instance_names.insert(name);
            }
            _ => {
                self.result.instance_classes.insert(name, class);
            }
        }
    }

    /// Logical source reporting compatibility; Native class identity requires
    /// an original class receipt from the genuine selected command or operand.
    pub(in super::super) fn resolve_user_class_in(
        &self,
        name: &str,
        scope_path: &[usize],
    ) -> Option<String> {
        super::super::class_hierarchy::resolve_retained_logical_class_name(
            &self.result,
            name,
            &self.command_resolution_namespace(scope_path),
        )
    }

    /// Logical reporting lookup. A propagated value label never issues an
    /// original Native class operand or a positioned class-cell selection.
    pub(super) fn resolve_user_class_at(&self, name: &str, offset: u32) -> Option<String> {
        super::super::class_hierarchy::resolve_retained_logical_class_name(
            &self.result,
            name,
            &super::super::scope::command_resolution_namespace_at(
                &self.result.global_scope,
                offset,
            ),
        )
    }

    /// Retain a source class label from the genuine setter and constructor.
    /// The canonical original declaration never comes from its printed head.
    fn class_from_constructor_subst(&self, site_offset: u32) -> Option<(String, String)> {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let construction =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                &self.source,
                &self.result,
                site_offset,
            )?;
        let offset = construction.construction().words.first()?.span().start();
        let call = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            offset,
        )?;
        call.constructor_shape(&self.result)?;
        let class = call.class_declaration().source_class(&self.result)?;
        Some((
            std::str::from_utf8(construction.variable_bytes())
                .ok()?
                .to_owned(),
            class.metadata().qualified_name.clone(),
        ))
    }

    /// Logical reporting compatibility for a possible constructor shape.
    /// Registry manufacturer layouts and bounded reported metaclass fallback
    /// metadata supply the model. The caller independently requires positive
    /// retained Logical input; this query supplies no Native class identity,
    /// callable manufacturer, allocation or successful constructor result.
    pub(in super::super) fn class_command_constructs_with(
        &self,
        class_q: &str,
        word: &str,
    ) -> bool {
        let Some(grammar) = self.class_definer_grammar(class_q) else {
            // Workspace report metadata supplies the separate Logical model
            // when no local class report determines its family grammar.
            return self.workspace_manufacturer_word(word)
                || (self.workspace_bare_word_classes.contains(class_q)
                    && is_plain_created_name(word));
        };
        if grammar.manufacturer(word).is_some() {
            return self.class_manufacturer_method(class_q, word).is_some();
        }
        if grammar.is_builtin_type_method(word) || !is_plain_created_name(word) {
            return false;
        }
        if self
            .result
            .all_classes
            .get(class_q)
            .is_some_and(|c| c.class_command_fallback.constructs_named_instance())
        {
            return true;
        }
        let Some(meta) = self.metaclass_def(class_q) else {
            return false;
        };
        meta.factory
            .as_ref()
            .is_some_and(|f| f.unknown_binds_instance)
            && !self.metaclass_chain_declares_method(meta, word)
    }

    /// Registry manufacturer layout under the reporting class/export model.
    /// This descriptor supplies source positions, independently of Native
    /// class identity, current manufacturer lookup or actual construction.
    pub(in super::super) fn class_manufacturer_method(
        &self,
        class_q: &str,
        word: &str,
    ) -> Option<&'static tcl_registry::definer::ManufacturerMethod> {
        let method = self.class_definer_grammar(class_q)?.manufacturer(word)?;
        let Some(class) = self.result.all_classes.get(class_q) else {
            return (method.visibility == tcl_registry::definer::MemberVisibility::Exported)
                .then_some(method);
        };
        (!class.class_unexports.contains(word)
            && (method.visibility == tcl_registry::definer::MemberVisibility::Exported
                || class.class_exports.contains(word)))
        .then_some(method)
    }

    /// Whether `word` is a manufacturer method of **any** definer family the
    /// registry models — the fallback for a workspace class whose own
    /// document this analysis has not seen, so its family is unknown here.
    ///
    /// Over-approximate by exactly the union of the families' manufacturer
    /// words (`create` / `new` / `createWithNamespace`), which is what the
    /// `subcmd == "new" || subcmd == "create"` literal this replaces already
    /// assumed — and still registry data, so a new family widens it without
    /// a walker edit.
    fn workspace_manufacturer_word(&self, word: &str) -> bool {
        self.analysis_context()
            .commands()
            .is_manufacturer_method(word)
    }

    /// The definition-body grammar governing `class_q`'s definer family.
    ///
    /// A registry metaclass (`oo::class`, `snit::type`) carries the grammar
    /// on its own spec.  A **user** metaclass has no spec, so the grammar is
    /// the registry metaclass at the root of its superclass chain — recorded
    /// on the factory when the metaclass was written, which is the only place
    /// it is provable.
    pub(in super::super) fn class_definer_grammar(
        &self,
        class_q: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.class_definer_grammar_with_provenance(class_q, false)
    }

    /// Positively retained Logical family advice from an observed class definer.
    /// The current source, lexer and availability inputs must still correspond.
    /// This descriptor supplies neither a Native class token nor construction,
    /// a formal-parameter grammar, method-table closure or dispatch activation.
    pub(in super::super) fn retained_logical_class_definer_grammar(
        &self,
        class_q: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        // naming.diagnostics.retained-logical-class-family
        // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
        if !self.result.allows_retained_logical_declaration_advice() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let context = self.analysis_context();
        if context.context() != input.availability_context()
            || context.commands().snapshot().semantic_key()
                != input
                    .context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
            || self.file_lexer_config() != input.lexer_config()
            || !self.result.matches_original_source_image(
                &self.command_document_source_image(),
                input.lexer_config(),
            )
        {
            return None;
        }
        self.class_definer_grammar_with_provenance(class_q, true)
    }

    fn class_definer_grammar_with_provenance(
        &self,
        class_q: &str,
        require_observed: bool,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let class = self.result.all_classes.get(class_q)?;
        if require_observed
            && class.metaclass_provenance != super::super::types::MetaclassProvenance::Observed
        {
            return None;
        }
        if let Some(grammar) = self.definition_grammar(&class.metaclass) {
            return Some(grammar);
        }
        let meta = self.metaclass_def(class_q)?;
        if require_observed
            && meta.metaclass_provenance != super::super::types::MetaclassProvenance::Observed
        {
            return None;
        }
        self.definition_grammar(&meta.factory.as_ref()?.root_metaclass)
    }

    /// The recorded `ClassDef` of `class_q`'s metaclass, when the metaclass
    /// is itself a class this document knows.
    ///
    /// Resolved through the shared owner-aware class-name resolver, so a
    /// bare metaclass word resolves exactly as a `superclass` word in the
    /// same position would — and abstains on an ambiguous tail rather than
    /// picking one.
    pub(in super::super) fn metaclass_def(
        &self,
        class_q: &str,
    ) -> Option<&super::super::types::ClassDef> {
        let class = self.result.all_classes.get(class_q)?;
        let resolved = super::super::class_hierarchy::resolve_class_lookup(
            class.metaclass_lookup.as_ref()?,
            &self.result.all_classes,
        )?;
        self.result.all_classes.get(&resolved)
    }

    /// Whether `method` is declared as an instance method anywhere in
    /// `meta`'s own superclass chain — the members a **class command**
    /// dispatch really finds, because a class is an instance of its
    /// metaclass.
    ///
    /// Bounded by the recorded class count: every class is visited once.
    fn metaclass_chain_declares_method(
        &self,
        meta: &super::super::types::ClassDef,
        method: &str,
    ) -> bool {
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: Vec<&super::super::types::ClassDef> = vec![meta];
        while let Some(class) = queue.pop() {
            if !visited.insert(class.qualified_name.clone()) {
                continue;
            }
            if class.methods.contains_key(method) {
                return true;
            }
            for parent in &class.superclasses {
                if let Some(resolved) = class
                    .relation_lookups
                    .get(parent)
                    .and_then(Option::as_ref)
                    .and_then(|lookup| {
                        super::super::class_hierarchy::resolve_class_lookup(
                            lookup,
                            &self.result.all_classes,
                        )
                    })
                    && let Some(def) = self.result.all_classes.get(&resolved)
                {
                    queue.push(def);
                }
            }
        }
        false
    }

    /// Retain the genuine setter and scalar-variable constructor geometry.
    /// The later value model can supply reporting labels for explicitly Logical
    /// input. Source geometry and constant values do not establish Native class
    /// selection, constructor execution or an allocated result.
    pub(super) fn record_pending_instance_class_site(&mut self, site_offset: u32) {
        if self.class_from_constructor_subst(site_offset).is_some() {
            return;
        }
        let Some(construction) =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                &self.source,
                &self.result,
                site_offset,
            )
        else {
            return;
        };
        let Some((class_var, manufacturer_word, span)) =
            class_var_head_constructor_subst(&construction)
        else {
            return;
        };
        let Ok(target_name) = std::str::from_utf8(construction.variable_bytes()) else {
            return;
        };
        self.pending_instance_class_sites
            .push(super::super::state::PendingInstanceClassSite {
                class_var,
                manufacturer_word,
                span,
                target_name: target_name.to_owned(),
            });
    }

    /// Retain class-variable constructor labels for explicitly Logical input.
    /// Constant contributors supply reporting values, not genuine original
    /// Native class operands, current source-cell occupancy or allocations.
    /// Native and hosted consumers require their own selected class-instance
    /// carrier and cannot recover it by parsing a propagated value label.
    pub(in crate::analyser) fn settle_pending_instance_class_sites(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.pending_instance_class_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.pending_instance_class_sites);
        if !self.result.resolved_input.as_ref().is_some_and(
            super::super::input::ResolvedAnalysisInput::has_logical_source_name_context,
        ) {
            return;
        }
        let config = self.file_lexer_config();
        for site in &sites {
            // A write trace can mutate the variable at any read — see
            // `settle_const_dispatches`'s identical guard.
            if cu.ir_module.has_dynamic_variable_trace
                || cu.ir_module.traced_variables.contains(&site.class_var)
            {
                continue;
            }
            let fu = cu.function_unit_at(site.span.start());
            let Some(contributors) = crate::value_provenance::const_contributors(
                fu,
                site.span.start(),
                &site.class_var,
                config,
            ) else {
                continue;
            };
            let mut resolved: Option<String> = None;
            for c in &contributors {
                let value = c.value.trim();
                if value.is_empty() || crate::naming::is_dynamic_word(value) {
                    resolved = None;
                    break;
                }
                let Some(qc) = self.resolve_user_class_at(value, site.span.start()) else {
                    resolved = None;
                    break;
                };
                if !self.class_command_constructs_with(&qc, &site.manufacturer_word) {
                    resolved = None;
                    break;
                }
                match &resolved {
                    None => resolved = Some(qc),
                    Some(existing) if *existing != qc => {
                        resolved = None;
                        break;
                    }
                    Some(_) => {}
                }
            }
            if let Some(class) = resolved {
                self.result
                    .instance_classes
                    .insert(site.target_name.clone(), class);
            }
        }
    }
}

/// Retain an exact scalar-variable constructor head and static selector from
/// the shared original setter/substitution carrier. Braced variable names and
/// escaped selectors follow their lexical/name owners. Composite and array
/// heads decline. The source span is the authentic reference extent; no class
/// identity, variable value or successful result is issued here.
pub(in crate::analyser) fn class_var_head_constructor_subst(
    construction: &crate::registry_invocation::OriginalSourceHandleConstruction,
) -> Option<(String, String, Span)> {
    let words = &construction.construction().words;
    let head = words.first()?;
    if head.group().expand {
        return None;
    }
    let arena = head.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Variable { name, index: None } = part.part else {
        return None;
    };
    let class_var = std::str::from_utf8(arena.bytes(name)?).ok()?.to_owned();
    let protocol = construction
        .setter()
        .dialect()?
        .native_source_string_protocol()?;
    let captured =
        tcl_registry::native_compiler_words::NativeCompilerWords::capture(words, protocol).ok()?;
    Some((
        class_var,
        std::str::from_utf8(captured.literal(1)?).ok()?.to_owned(),
        part.span,
    ))
}

/// Whether `name` is a concrete, bindable instance-command name in a
/// `CLASS create NAME` construct — a plain word the analyser can register.
/// Excludes the auto-name token (`%AUTO%`), computed names (`$v`, `[…]`), and
/// any word carrying list / substitution metacharacters.
fn is_plain_created_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('%')
        && !name.contains(['$', '[', ']', '{', '}', '(', ')', ' ', '"'])
}
