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

//! Jim `class` definers, their member calls and the two-word `proc {CLASS member}` form.

use super::super::state::Analyser;
use super::super::types::{ClassDef, MethodDef};
use super::{ClassBodyCtx, MemberForm};
use crate::ir::MethodKind;
use tcl_lexer::Token;
use tcl_registry::definer::DefinitionBodyGrammar;

/// A definer's grammar plus the class being populated and where its members
/// live — bundled so the snit / itcl member dispatch + extraction helpers stay
/// under the argument limit.  Shared by both families (the extraction is
/// grammar-driven; only the per-family dispatch differs).
struct OriginalJimClassLists {
    bases_dynamic: bool,
    pub(super) superclasses: Vec<String>,
    variable_words: Vec<String>,
}

fn original_two_word_proc_name(
    declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<
        super::super::types::ProcDef,
    >,
) -> Option<(
    crate::signature_scan::scope::SignatureSourceNameInput,
    crate::signature_scan::scope::SignatureSourceNameInput,
)> {
    let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declaration.name_input().clone(),
    );
    let children = input.original_list_elements()?;
    let [class, member] = children.as_slice() else {
        return None;
    };
    Some((class.clone(), member.clone()))
}

fn insert_original_two_word_member(
    class_def: &mut ClassDef,
    member: &str,
    proc: &super::super::types::ProcDef,
) {
    class_def.methods.insert(
        member.to_string(),
        MethodDef {
            name: member.to_string(),
            params: proc.params.clone(),
            params_computed: proc.params_computed,
            formal_count: proc.formal_count.clone(),
            name_span: proc.name_span,
            body_span: proc.body_span,
            kind: "method".to_string(),
            is_self_method: false,
            visibility: "public".to_string(),
            doc: proc.doc.clone(),
            forward_target: None,
        },
    );
}

impl Analyser {
    /// Handle a `JimClass` definer — `class NAME ?BASES? VARS` — recording the
    /// class as a [`ClassDef`] with its base classes and the instance
    /// variables `VARS` names.
    ///
    /// `VARS` is a dictionary of every instance variable and its initial
    /// value, not a script, so it is read for its keys and never walked as a
    /// body. The class defines no member itself: methods arrive later, through
    /// [`Self::handle_jim_class_member_call`] or a two-word `proc`
    /// ([`Self::record_two_word_proc_member`]).
    pub(in super::super) fn handle_jim_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> bool {
        self.handle_jim_class_source(super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: Some(cmd_tok),
            original: None,
        })
    }

    pub(in super::super) fn handle_jim_class_source(
        &mut self,
        call: super::super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            original: _,
            cmd_tok,
        } = call;
        let Some(authored) = cmd_tok
            .and_then(|token| {
                crate::registry_invocation::source_structure::source_authored_jim_class_at(
                    &self.source,
                    &self.result,
                    token.span.start(),
                )
            })
            .filter(|source| {
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "ingress.arguments",
                    source.matches_arguments(args, arg_tokens).then_some(()),
                )
                .is_some()
            })
        else {
            return false;
        };
        // The definer's own arity is 2 or 3; any other shape is left to the
        // arity check.
        if !(2..=3).contains(&args.len()) || arg_tokens.len() != args.len() {
            return false;
        }
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = super::super::handlers::qualify(&ns_prefix, &args[0]);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(&args[0], name_span);

        let vars_at = args.len() - 1;
        let OriginalJimClassLists {
            bases_dynamic,
            superclasses,
            variable_words,
        } = Self::original_jim_class_lists(&authored);
        self.report_original_jim_dictionary_shape(cmd_name, &variable_words, arg_tokens[vars_at]);
        let own_variables: Vec<String> = variable_words.into_iter().step_by(2).collect();
        // A derived class's variable dictionary is its bases' merged with its
        // own, so a method body sees every inherited instance variable too.
        let mut variables = authored.inherited_variables(&self.result);
        for variable in own_variables {
            if !variables.contains(&variable) {
                variables.push(variable);
            }
        }

        let doc = std::mem::take(&mut self.last_comment);
        let mut class = ClassDef {
            // The distinct authored source receipt supplies no Native class name.
            source_name: None,
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span: arg_tokens[vars_at].span,
            metaclass: cmd_name.to_string(),
            metaclass_provenance: super::super::types::MetaclassProvenance::Observed,
            superclasses,
            variables,
            inheritance_unknown: bases_dynamic,
            doc,
            ..Default::default()
        };
        // Source-only base relations never donate a Native naming lookup.
        for base in &class.superclasses {
            class.relation_lookups.insert(base.clone(), None);
        }
        self.register_defined_class(qualified.clone(), class, scope_path);
        self.record_authored_jim_procedure_members_at(
            authored.factory().site().offset,
            None,
            scope_path,
        );
        self.adopt_recorded_two_word_procs(&qualified, scope_path, None);
        true
    }

    pub(in super::super) fn retained_source_advice_purpose(
        &self,
    ) -> Option<crate::command_binding::OriginalInvocationSourcePurpose> {
        let input = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref())?;
        let config = self.file_lexer_config();
        if input.lexer_config() != config {
            return None;
        }
        let context = self.analysis_context();
        let registry = context.commands();
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, input,
        )?;
        self.head_identities
            .source_bindings_ref()
            .original_source_entry_purpose(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
                metadata,
                registry,
            )
    }

    fn original_jim_class_lists(
        original: &crate::command_binding::OriginalAuthoredJimClass,
    ) -> OriginalJimClassLists {
        OriginalJimClassLists {
            bases_dynamic: original.bases_unknown(),
            superclasses: original
                .base_fields()
                .unwrap_or_default()
                .into_iter()
                .map(|field| field.value().to_owned())
                .collect(),
            variable_words: original
                .variable_fields()
                .unwrap_or_default()
                .iter()
                .map(|field| field.value().to_owned())
                .collect(),
        }
    }

    fn report_original_jim_dictionary_shape(
        &mut self,
        command: &str,
        words: &[String],
        token: Token,
    ) {
        if words.len().is_multiple_of(2) {
            return;
        }
        let span = super::super::utils::full_word_span(token, &self.source);
        self.result.diagnostics.push(super::super::types::Diagnostic::new(
            tcl_core_types::DiagCode::E005,
            span,
            format!("Wrong argument-count shape for '{command}': the variable dictionary has an odd number of elements, so '{}' has no value", words[words.len() - 1]),
            super::super::types::Severity::Error,
        ));
    }

    /// Attach genuine two-word procedure declarations to this source class.
    /// Native list children retain their own producer and naming scope; only
    /// explicit Logical compatibility may read reporting procedure names.
    fn adopt_recorded_two_word_procs(
        &mut self,
        class_q: &str,
        scope_path: &[usize],
        original: Option<&super::super::original_definer::OriginalClassDefinerSource>,
    ) {
        if let Some(original) = original {
            let calls: Vec<_> = self
                .result
                .original_procedure_declarations()
                .filter_map(|procedure| {
                    let (input, member) = original_two_word_proc_name(procedure)?;
                    let reference =
                        crate::registry_invocation::source_structure::source_class_reference_at(
                            &self.source,
                            &self.result,
                            original.declaration().factory().site().offset,
                            &input,
                        )?;
                    (reference.class_declaration().factory().site()
                        == original.declaration().factory().site())
                    .then(|| (reference, member, procedure.clone()))
                })
                .collect();
            for (reference, member, procedure) in calls {
                self.add_original_two_word_member(&reference, &member, &procedure, scope_path);
            }
            return;
        }
        if self.retained_source_advice_purpose()
            != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        {
            return;
        }
        let rules = self.word_rules();
        let calls: Vec<_> = self
            .result
            .all_procs
            .values()
            .filter_map(|procedure| {
                let words = rules.split_list(&procedure.name).ok()?;
                let [class, member] = words.as_slice() else {
                    return None;
                };
                (self.resolve_user_class_in(class, scope_path).as_deref() == Some(class_q))
                    .then(|| (member.to_string(), procedure.clone()))
            })
            .collect();
        for (member, procedure) in calls {
            self.add_two_word_member(class_q, &member, &procedure, scope_path);
        }
    }

    /// Class presentation for a setter whose single substitution is a selected
    /// authored Jim class call naming one of its definer grammar's
    /// manufacturers (`set p [Point new]`). Conditional source metadata only:
    /// no allocation, successful construction or receiver lifetime.
    pub(in super::super) fn jim_class_from_constructor_subst(
        &self,
        site_offset: u32,
    ) -> Option<(String, String)> {
        use crate::registry_invocation::source_structure as structure;
        let construction =
            structure::source_handle_construction_at(&self.source, &self.result, site_offset)?;
        let offset = construction.construction().words.first()?.span().start();
        let call = structure::source_authored_jim_call_at(&self.source, &self.result, offset)?;
        let member = call.original_words().get(1)?.bytes();
        let context = self.result.resolved_input.as_ref()?.context_registry();
        let grammar = context
            .context()
            .resolve_spec(context.commands(), call.class().factory().command())?
            .definition_body?;
        (grammar.family == tcl_registry::definer::DefinerFamily::JimClass
            && std::str::from_utf8(member)
                .ok()
                .and_then(|member| grammar.manufacturer(member))
                .is_some_and(|manufacturer| manufacturer.names_instance_at.is_none()))
        .then_some(())?;
        let class = call.metadata(&self.result)?;
        Some((
            std::str::from_utf8(construction.variable_bytes())
                .ok()?
                .to_owned(),
            class.qualified_name.clone(),
        ))
    }

    /// Handle `CLASS member …` where `CLASS` is a `JimClass` class and `member`
    /// is one of its definer's own members (`CLASS method NAME ARGS BODY`),
    /// recording the member on the class and walking its body with the class
    /// variables and the grammar's implicit variables (`self`) bound.
    pub(in super::super) fn handle_jim_class_member_call(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        command_token: Token,
    ) -> bool {
        if let Some(call) =
            crate::registry_invocation::source_structure::source_authored_jim_call_at(
                &self.source,
                &self.result,
                command_token.span.start(),
            )
        {
            let Some(mut class) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.member.metadata",
                    call.metadata(&self.result).cloned(),
                )
            else {
                return false;
            };
            let context = self
                .result
                .resolved_input
                .as_ref()
                .map(super::super::input::ResolvedAnalysisInput::context_registry);
            let Some(context) = context else {
                return false;
            };
            let Some(grammar) = context
                .context()
                .resolve_spec(context.commands(), call.class().factory().command())
                .and_then(|spec| spec.definition_body)
            else {
                return false;
            };
            if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
                return false;
            }
            if crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.member.arguments",
                super::super::original_definer::logical_arguments_match(
                    call.original_words(),
                    args,
                    arg_tokens,
                )
                .then_some(()),
            )
            .is_none()
            {
                return false;
            }
            if !self.analyse_jim_inline_member(grammar, args, arg_tokens, &mut class, scope_path) {
                return false;
            }
            self.register_defined_class(class.qualified_name.clone(), class, scope_path);
            return true;
        }
        let original = super::super::original_definer::OriginalClassCallSource::capture(
            &self.source,
            &self.result,
            command_token.span.start(),
        );
        let (args, arg_tokens, mut class_def, grammar) = if let Some(original) = &original {
            let Some(class) = original.call.class_declaration().source_class(&self.result) else {
                return false;
            };
            (
                original.arguments.as_slice(),
                original.tokens.as_slice(),
                class.metadata().clone(),
                original.grammar,
            )
        } else {
            let Some(class_q) = self.resolve_user_class_in(cmd_name, scope_path) else {
                return false;
            };
            let Some(class) = self.result.all_classes.get(&class_q).cloned() else {
                return false;
            };
            let Some(grammar) = self.class_definer_grammar(&class_q) else {
                return false;
            };
            (args, arg_tokens, class, grammar)
        };
        if grammar.family != tcl_registry::definer::DefinerFamily::JimClass {
            return false;
        }
        if !self.analyse_jim_inline_member(grammar, args, arg_tokens, &mut class_def, scope_path) {
            return false;
        }
        let class_q = class_def.qualified_name.clone();
        if let Some(original) = &original {
            self.retain_original_source_class_update(original.call.class_declaration(), &class_def);
        }
        self.register_defined_class(class_q, class_def, scope_path);
        true
    }

    fn analyse_jim_inline_member(
        &mut self,
        grammar: &'static DefinitionBodyGrammar,
        args: &[String],
        arg_tokens: &[Token],
        class_def: &mut ClassDef,
        scope_path: &[usize],
    ) -> bool {
        let Some(keyword) = args.first() else {
            return false;
        };
        if arg_tokens.len() != args.len() {
            return false;
        }
        let Some(member) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.member.row",
            grammar.member(keyword),
        ) else {
            return false;
        };
        self.retain_original_inline_member_metadata(
            grammar, args, arg_tokens, class_def, scope_path,
        );
        let class_q = class_def.qualified_name.clone();
        let member_args = &args[1..];
        let member_tokens = &arg_tokens[1..];
        let parameter_indices = self.member_formal_parameter_indices(
            grammar,
            member,
            member_args,
            member_tokens,
            class_def,
        );
        super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
            self,
            member_args,
            member_tokens,
            &parameter_indices,
        );
        let mut seed_vars = class_def.variables.clone();
        seed_vars.extend(grammar.implicit_vars.iter().map(ToString::to_string));
        let mut ctx = ClassBodyCtx {
            grammar,
            class_def,
            class_qualified: &class_q,
            scope_path,
        };
        let form = MemberForm {
            member,
            kind: MethodKind::Method,
            label: "",
            visibility: "public",
        };
        self.extract_class_member(member_args, member_tokens, &mut ctx, &seed_vars, &form);
        true
    }

    /// Record a `proc` whose name is the two-element list `{CLASS member}` as a
    /// method of `CLASS` when `CLASS` is a class of a family that stores its
    /// members as two-word commands
    /// ([`tcl_registry::definer::DefinerFamily::members_are_two_word_commands`]).
    ///
    /// The procedure is itself the two-word command `{CLASS member}` — every
    /// dialect defines that — so this only adds the class-side fact. The
    /// method has no class variables and no `self` in its body: a raw
    /// definition is not wrapped by the class command's `method`. A class
    /// written after the `proc` adopts it when the class is defined
    /// ([`Self::adopt_recorded_two_word_procs`]).
    pub(in super::super) fn record_two_word_proc_member(
        &mut self,
        proc_name: &str,
        proc: &super::super::types::ProcDef,
        scope_path: &[usize],
    ) {
        let Some(purpose) = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.raw_member.purpose",
            self.retained_source_advice_purpose(),
        ) else {
            return;
        };
        if purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            let original_offset =
                self.result
                    .original_procedure_declarations()
                    .find_map(|original| {
                        (original.metadata() == proc).then_some(original.declaration_site().offset)
                    });
            if let Some(offset) = original_offset
                && self.record_authored_jim_procedure_members_at(offset, Some(proc), scope_path)
            {
                return;
            }
            let selected = self
                .result
                .original_procedure_declarations()
                .find_map(|original| {
                    (original.metadata().name_span == proc.name_span
                        && original.metadata().body_span == proc.body_span
                        && original.metadata().source_name == proc.source_name)
                        .then_some(())?;
                    let (input, member) =
                        crate::registry_invocation::source_structure::trace_jim_source_option(
                            "jim.raw_member.name_children",
                            original_two_word_proc_name(original),
                        )?;
                    let reference =
                        crate::registry_invocation::source_structure::trace_jim_source_option(
                            "jim.raw_member.class_reference",
                            crate::registry_invocation::source_structure::source_class_reference_at(
                                &self.source,
                                &self.result,
                                original.declaration_site().offset,
                                &input,
                            ),
                        )?;
                    Some((reference, member, original.clone()))
                });
            if let Some((reference, member, procedure)) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.raw_member.selected",
                    selected,
                )
            {
                self.add_original_two_word_member(&reference, &member, &procedure, scope_path);
            }
            return;
        }
        let Ok(words) = self.word_rules().split_list(proc_name) else {
            return;
        };
        let [class_word, member_word] = words.as_slice() else {
            return;
        };
        let Some(class_q) = self.resolve_user_class_in(class_word, scope_path) else {
            return;
        };
        self.add_two_word_member(&class_q, member_word, proc, scope_path);
    }

    fn record_authored_jim_procedure_members_at(
        &mut self,
        offset: u32,
        expected: Option<&super::super::types::ProcDef>,
        scope_path: &[usize],
    ) -> bool {
        let Some(references) =
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.own_member.recipes",
                crate::registry_invocation::source_structure::source_authored_jim_procedure_members_at(
                    &self.source, &self.result, offset,
                ),
            )
        else {
            return false;
        };
        let _ = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.own_member.recipes_nonempty",
            (!references.is_empty()).then_some(()),
        );
        let mut recorded = false;
        for reference in references {
            let Some(procedure) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.procedure",
                    reference.source_procedure(&self.result),
                )
            else {
                continue;
            };
            if expected.is_some_and(|expected| procedure.metadata() != expected) {
                continue;
            }
            let Some(canonical) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.class",
                    reference.class_metadata(&self.result),
                )
            else {
                continue;
            };
            let mut class = canonical.clone();
            let Some(metadata) =
                crate::registry_invocation::source_structure::trace_jim_source_option(
                    "jim.own_member.association",
                    self.original_authored_jim_procedure_member(
                        &reference,
                        procedure,
                        reference.member(),
                        &class,
                    ),
                )
            else {
                continue;
            };
            let Ok(label) = std::str::from_utf8(reference.member().bytes()) else {
                continue;
            };
            insert_original_two_word_member(&mut class, label, procedure.metadata());
            class.original_members.declare(metadata);
            self.register_defined_class(class.qualified_name.clone(), class, scope_path);
            recorded = true;
        }
        recorded
    }

    fn add_original_two_word_member(
        &mut self,
        reference: &crate::command_binding::OriginalSourceClassReference,
        member: &crate::signature_scan::scope::SignatureSourceNameInput,
        procedure: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            super::super::types::ProcDef,
        >,
        scope_path: &[usize],
    ) {
        let declaration = reference.class_declaration();
        let context = self.analysis_context();
        if !declaration
            .grammar(&context)
            .is_some_and(|grammar| grammar.family.members_are_two_word_commands())
        {
            return;
        }
        let Some(original) = declaration.source_class(&self.result) else {
            return;
        };
        let mut class = original.metadata().clone();
        let Ok(label) = std::str::from_utf8(member.bytes()) else {
            return;
        };
        insert_original_two_word_member(&mut class, label, procedure.metadata());
        if !self.retain_original_two_word_source_member(reference, procedure, member, &mut class) {
            return;
        }
        self.retain_original_source_class_update(declaration, &class);
        self.register_defined_class(class.qualified_name.clone(), class, scope_path);
    }

    /// Add `proc` to `class_q` as its method `member` when the class's family
    /// stores its members as two-word commands.
    fn add_two_word_member(
        &mut self,
        class_q: &str,
        member: &str,
        proc: &super::super::types::ProcDef,
        scope_path: &[usize],
    ) {
        if !self
            .class_definer_grammar(class_q)
            .is_some_and(|grammar| grammar.family.members_are_two_word_commands())
        {
            return;
        }
        let Some(mut class_def) = self.result.all_classes.get(class_q).cloned() else {
            return;
        };
        insert_original_two_word_member(&mut class_def, member, proc);
        self.register_defined_class(class_q.to_string(), class_def, scope_path);
    }
}
