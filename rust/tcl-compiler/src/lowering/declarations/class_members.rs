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

//! The post-pass that lifts methods, constructors and declared variables out of class definitions into the module.

use super::super::{Lowerer, join_namespace, seg_word_is_static_braced};
use super::DefinerCall;
use super::procedures::{parse_formal_param_names, qualify_proc_name};
use crate::ir::{CommandTokens, MethodDef, MethodKind, Statement};
use crate::segmenter::SegmentedCommand;
use std::collections::{HashMap, HashSet};
use tcl_lexer::SourceSegmentationPurpose;
use tcl_registry::definer::{
    DefinitionBodyGrammar, InitTiming, MemberCurrentNamespace, MemberEffect, MemberReceiver,
    MemberRow,
};
use tcl_registry::{ArgRole, InvocationArguments, InvocationWord};
use tcl_syntax::word_rules::WordValueRules;

impl Lowerer<'_> {
    pub(in super::super) fn advisory_method_instance_names(
        &self,
        statement: &Statement,
    ) -> Vec<String> {
        let Statement::Call { args, tokens, .. } = statement else {
            return Vec::new();
        };
        let possible = tokens.as_ref().and_then(|tokens| {
            let context = self.invocation_metadata_context()?;
            let advice = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                self.registry, Some(context), tokens,
            )?;
            if advice.unknown_residual || advice.may_be_absent || advice.candidates.is_empty() {
                return None;
            }
            let mut names = Vec::new();
            for candidate in &advice.candidates {
                names.extend(candidate.possible_method_instance_variable_names(
                    self.registry,
                    context,
                    tokens,
                )?);
            }
            Some(names)
        });
        // Unavailable source advice only widens the textual protection inventory.
        possible.unwrap_or_else(|| args.clone())
    }

    // TclOO method-body lowering (method purity for O126)

    /// Populate `module.methods` from the fully-assembled IR.
    ///
    /// Runs as a cache-independent post-pass (mirroring
    /// [`populate_trace_facts`]): clone the top-level script and proc
    /// bodies, then walk them — plus any `namespace eval` blocks — for
    /// `oo::class create` / `oo::define` barriers and lift each
    /// `method` / `constructor` / `destructor` body to an
    /// [`MethodDef`]. Method bodies are lowered for analysis only —
    /// codegen never reads `module.methods`, and the barrier emitted
    /// for the class command is unchanged, so bytecode is
    /// byte-identical.
    pub fn extract_oo_methods_pass(&mut self) {
        let retained = self.module_source_bindings.take();
        let enclosing = self.exchange_boxed_source_bindings(retained);
        // The walk borrows each script out of the module rather than cloning
        // it: the derived `Clone` of cap-deep structured IR recurses several
        // frames per nesting level, which overflowed a 2 MiB thread at the
        // structured-lowering depth cap. No member extraction reads these
        // scripts back from the module.
        let top_level = std::mem::take(&mut self.module.top_level);
        let top_namespace = if self.module.top_level_namespace.is_empty() {
            "::".to_owned()
        } else {
            self.module.top_level_namespace.clone()
        };
        self.walk_for_oo_methods(&top_level.statements, &top_namespace);
        self.module.top_level = top_level;
        let procs: Vec<(crate::ir::ProcedureId, String)> = self
            .module
            .procedures
            .iter()
            .filter_map(|(id, p)| Some((*id, p.body.namespace_context.as_deref()?.advisory_key()?)))
            .collect();
        for (id, namespace) in &procs {
            let Some(body) = self
                .module
                .procedures
                .get_mut(id)
                .map(|procedure| std::mem::take(&mut procedure.body))
            else {
                continue;
            };
            self.walk_for_oo_methods(&body.statements, namespace);
            if let Some(procedure) = self.module.procedures.get_mut(id) {
                procedure.body = body;
            }
        }
        // Every definition block for every class has now been walked, so the
        // per-class instance-variable union is complete.  Merge it into each
        // method: a method extracted from an early block must still see state
        // a later `oo::define` block declared for its class, or a consumer
        // reading `MethodDef::instance_vars` (the optimiser's method-body
        // propagation gate, `elimination.rs`'s dead-store protection) would
        // treat object state as a private local.  Order-free by construction —
        // the union does not depend on which block was walked first.
        for method in self.module.methods.values_mut() {
            if method.body.executed_source.is_none()
                && let Some(class_vars) = self.class_instance_vars.get(&method.class_name)
            {
                method.instance_vars.extend(class_vars.iter().cloned());
            }
        }
        self.module_source_bindings = self.exchange_boxed_source_bindings(enclosing);
    }

    /// Recursive walk for `oo::class` / `oo::define` definition
    /// barriers. Descends `namespace eval` blocks (tracking the active
    /// namespace) and extracts methods from any class/define command
    /// carrying a static braced body. The lowerer evaluates a
    /// `namespace eval` body inline and discards it (emitting a
    /// `Barrier`), so rather than descending a preserved block this
    /// walk re-segments the barrier's body token instead (see
    /// [`Self::walk_segments_for_oo`]).
    fn walk_for_oo_methods(&mut self, statements: &[Statement], namespace: &str) {
        for stmt in statements {
            match stmt {
                // Folded `eval` / `uplevel` bodies survive as Blocks.
                Statement::Block {
                    body,
                    namespace: block_ns,
                    ..
                } => {
                    let ns = if block_ns.is_empty() {
                        namespace
                    } else {
                        block_ns.as_str()
                    };
                    self.walk_for_oo_methods(&body.statements, ns);
                }
                Statement::Call {
                    tokens: Some(ct), ..
                }
                | Statement::Barrier {
                    tokens: Some(ct), ..
                } => {
                    if let Some(call) = self.classify_definer_call(ct) {
                        self.extract_definer_members(
                            call,
                            ct,
                            ct.argv[call.body_idx].start() + 1,
                            namespace,
                        );
                    } else if self.definer_invocation_may_supply_body(ct) {
                        self.module.oo_evidence.unretained_executable_roots = true;
                    } else if self.is_namespace_eval_shape(ct) {
                        // The body was lowered inline and discarded;
                        // re-segment it to find classes defined directly
                        // inside the namespace.
                        let child_ns = join_namespace(namespace, &ct.argv_texts[2]);
                        let body_off = ct.argv[3].start() + 1;
                        let Some(segments) = self.segment_source(
                            &ct.argv_texts[3],
                            body_off,
                            SourceSegmentationPurpose::NestedScript,
                        ) else {
                            self.module.oo_evidence.unretained_executable_roots = true;
                            continue;
                        };
                        self.in_word_space(&ct.argv_texts[3], body_off, |this| {
                            this.walk_segments_for_oo(&segments, &child_ns);
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Segment-level counterpart of [`Self::walk_for_oo_methods`] used
    /// for `namespace eval` bodies (which the lowerer discards). Walks
    /// re-segmented commands, recursing through nested `namespace eval`
    /// and extracting members from any registry-classified definer block.
    fn walk_segments_for_oo(&mut self, segments: &[SegmentedCommand], namespace: &str) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let tokens = self.cmd_tokens(seg);
            if seg.is_partial {
                if self.definer_invocation_may_supply_body(&tokens) {
                    self.module.oo_evidence.unretained_executable_roots = true;
                }
                continue;
            }
            if let Some(call) = self.classify_definer_call(&tokens) {
                let body = seg.argv[call.body_idx];
                let off = body.span.start() + u32::from(body.content_offset);
                self.extract_definer_members(call, &tokens, off, namespace);
            } else if self.definer_invocation_may_supply_body(&tokens) {
                self.module.oo_evidence.unretained_executable_roots = true;
            } else if self.is_namespace_eval_shape(&tokens) {
                let child_ns = join_namespace(namespace, &seg.texts[2]);
                let off = seg.argv[3].span.start() + u32::from(seg.argv[3].content_offset);
                let Some(sub) = self.segment_source(
                    &seg.texts[3],
                    off,
                    SourceSegmentationPurpose::NestedScript,
                ) else {
                    self.module.oo_evidence.unretained_executable_roots = true;
                    continue;
                };
                self.in_word_space(&seg.texts[3], off, |this| {
                    this.walk_segments_for_oo(&sub, &child_ns);
                });
            }
        }
    }

    /// Retain each selected class-relation member with its authentic outer
    /// definer. Value formation and target selection belong to the source owner;
    /// punctuation in the written operand never determines ancestry here.
    fn record_class_relation_member(
        &mut self,
        call: &DefinerCall,
        definer: &CommandTokens,
        seg: &SegmentedCommand,
        class_qname: &str,
    ) -> bool {
        let head = seg.texts[0].as_str();
        let Some(member) = self.source_definition_member(call.grammar, head) else {
            return false;
        };
        if member.all_args_ref != Some(tcl_registry::definer::MemberRefKind::Class) {
            return false;
        }
        self.module.class_relations.push(crate::ir::ClassRelation {
            declaring_class: class_qname.to_owned(),
            definer: std::sync::Arc::new(definer.clone()),
            member: std::sync::Arc::new(self.cmd_tokens(seg)),
            target_word: call.name_idx,
            body_word: call.body_idx,
        });
        true
    }

    /// Retain each member's actual body ID, typed kind and declaring source owner.
    /// The selected [`DefinerCall::grammar`] supplies recognition and operand layout.
    ///
    /// `texts` is the definer command's full per-word text array;
    /// `body_content_offset` is the absolute source offset of the first
    /// byte inside the body's braces.
    fn extract_definer_members(
        &mut self,
        call: DefinerCall,
        definer: &CommandTokens,
        body_content_offset: u32,
        namespace: &str,
    ) {
        let call = DefinerCall {
            definition_scope: Some(self.module.retain_definition_script_source(
                definer,
                &crate::ir::DefinitionScriptLayout {
                    body: call.body_idx,
                    name: call.name_idx,
                    per_object: call.per_object,
                    offset: body_content_offset,
                    grammar: call.grammar,
                    descriptor: call.descriptor,
                },
                self.registry,
            )),
            ..call
        };
        let texts = &definer.argv_texts;
        let body_text = texts[call.body_idx].as_str();
        let target = definer
            .words()
            .first()
            .and_then(|head| head.original_written_span(self.module.source.bytes()))
            .and_then(|span| {
                crate::registry_invocation::original_native_compiler_words(
                    &self.module.source,
                    definer.words(),
                    span.start(),
                    self.config,
                )
            })
            .and_then(|words| {
                words
                    .get(call.name_idx)
                    .and_then(crate::registry_invocation::original_source_receiver_name_operand)
            })
            .filter(|target| match target {
                crate::registry_invocation::SourceReceiverNameOperand::Literal(name)
                | crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name) => {
                    !name.as_bytes().contains(&0)
                }
            });
        let class_qname = match (call.per_object, target) {
            // This label reports per-object source units; definition IDs own their scope.
            (
                true,
                Some(
                    crate::registry_invocation::SourceReceiverNameOperand::Literal(name)
                    | crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name),
                ),
            ) => format!("::@objdefine@::{name}"),
            (false, Some(crate::registry_invocation::SourceReceiverNameOperand::Literal(name))) => {
                qualify_proc_name(namespace, &name)
            }
            _ => {
                self.module.oo_evidence.dynamic_target = true;
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
        };
        let Some(segments) = self.segment_source(
            body_text,
            body_content_offset,
            SourceSegmentationPurpose::NestedScript,
        ) else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };

        let surface = self
            .invocation_metadata_context()
            .map(|context| context.context().authoring_query());
        let class_ivars = declared_member_vars(call.grammar, &segments, surface, self.config);
        // This block sees only its own declarations; a sibling `oo::define`
        // block may declare more state for the same class, and may be walked
        // *after* the methods that use it.  Accumulate the whole-class union
        // for `extract_oo_methods_pass`'s merge — see
        // [`Self::class_instance_vars`].  A per-object block accumulates
        // under its own synthetic key, so per-object `variable` declarations
        // stay per-object by construction.
        if !class_ivars.is_empty() {
            self.class_instance_vars
                .entry(class_qname.clone())
                .or_default()
                .extend(class_ivars.iter().cloned());
        }

        self.in_word_space(body_text, body_content_offset, |this| {
            this.extract_members_from_segments(
                &call,
                definer,
                &segments,
                &class_qname,
                &class_ivars,
                namespace,
            );
        });
    }

    /// The per-segment member walk behind [`Self::extract_definer_members`],
    /// factored out so a wrapper's block form (`self { … }` /
    /// `private { … }`) can recurse into its nested definition script with
    /// the same class context.
    fn extract_members_from_segments(
        &mut self,
        call: &DefinerCall,
        definer: &CommandTokens,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let member_supplies_body = self.member_invocation_supplies_body(call.grammar, seg);
            if seg.is_partial {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            if self.record_class_relation_member(call, definer, seg, class_qname) {
                continue;
            }
            let Some(member) = self.source_definition_member(call.grammar, head) else {
                // Definition scripts can call ordinary absolute commands (or
                // dynamically installed definition helpers). Their execution
                // is not represented by a MethodDef root.
                self.module.oo_evidence.unretained_executable_roots = true;
                continue;
            };
            // A wrapper member (`self …`, `private …`, itcl's access
            // modifiers) either prefixes an inner member (shift one place
            // right) or — for `wrapper_block_body` wrappers — carries a
            // whole nested definition script to recurse into.
            let (member, kw, base) = match member.kind {
                tcl_registry::definer::MemberKind::Wrapper => match seg.texts.get(1) {
                    Some(inner) if self.source_definition_member(call.grammar, inner).is_some() => {
                        let inner_member = self
                            .source_definition_member(call.grammar, inner)
                            .expect("selected source member");
                        // No double wrapping (`self private method …` is not
                        // a real Tcl shape).
                        if inner_member.kind != tcl_registry::definer::MemberKind::Flat {
                            self.module.oo_evidence.unretained_executable_roots |=
                                member_supplies_body;
                            continue;
                        }
                        (inner_member, inner.as_str(), 2usize)
                    }
                    Some(_)
                        if member.wrapper_block_body
                            && seg.texts.len() >= 2
                            && seg_word_is_static_braced(seg, 1) =>
                    {
                        // `self { … }` / `private { … }` — a nested
                        // definition script with the same member grammar.
                        let block_tok = seg.argv[1];
                        let off = block_tok.span.start() + u32::from(block_tok.content_offset);
                        let Some(sub) = self.segment_source(
                            &seg.texts[1],
                            off,
                            SourceSegmentationPurpose::NestedScript,
                        ) else {
                            self.module.oo_evidence.unretained_executable_roots = true;
                            continue;
                        };
                        // A `self { variable v }` declares per-class-object
                        // state, not instance state; keep it out of the
                        // instance union — only the members are lifted.
                        let wrapped_call = DefinerCall { ..*call };
                        let empty = HashSet::new();
                        let moves_off_the_instances = member
                            .wrapper_shift
                            .and_then(|shift| shift.receiver)
                            .is_some_and(|receiver| receiver != MemberReceiver::Instance);
                        let ivars = if moves_off_the_instances {
                            &empty
                        } else {
                            class_ivars
                        };
                        self.in_word_space(&seg.texts[1], off, |this| {
                            this.extract_members_from_wrapper_block(
                                &wrapped_call,
                                &sub,
                                class_qname,
                                ivars,
                                namespace,
                                head,
                            );
                        });
                        continue;
                    }
                    _ => {
                        self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                        continue;
                    }
                },
                tcl_registry::definer::MemberKind::Flat => (member, head, 1usize),
                // Flag-keyed bodies (`property … -get/-set …`) are accessor
                // scripts, not method frames — no unit today (documented
                // limit).
                tcl_registry::definer::MemberKind::FlagKeyed => {
                    self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                    continue;
                }
            };
            let frame = self.member_frame_for_segment(call, seg);
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw,
                    base,
                    frame,
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    fn member_frame_for_segment(&self, call: &DefinerCall, seg: &SegmentedCommand) -> MemberFrame {
        let effective = statement_words(&self.cmd_tokens(seg), self.config);
        let words: Vec<_> = effective
            .iter()
            .map(|word| word.as_registry_word())
            .collect();
        MemberFrame::of_row(
            call.grammar,
            call.grammar
                .member_row(0, InvocationArguments::structured(&words), None)
                .as_ref(),
        )
    }

    /// Recurse into a wrapper's block form with the wrapper prefixed onto
    /// each member — `self { method m … }` records `m` as a class-object
    /// method, and `private { method m … }` as an instance method, exactly
    /// like their prefix spellings, because the registry answers the prefixed
    /// statement's row with the wrapper's shift applied.
    fn extract_members_from_wrapper_block(
        &mut self,
        call: &DefinerCall,
        segments: &[SegmentedCommand],
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
        wrapper: &str,
    ) {
        for seg in segments {
            if seg.texts.is_empty() {
                continue;
            }
            let head = seg.texts[0].as_str();
            let member_supplies_body = self.member_invocation_supplies_body(call.grammar, seg);
            if seg.is_partial {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            let Some(member) = self.source_definition_member(call.grammar, head) else {
                self.module.oo_evidence.unretained_executable_roots = true;
                continue;
            };
            if member.kind != tcl_registry::definer::MemberKind::Flat {
                self.module.oo_evidence.unretained_executable_roots |= member_supplies_body;
                continue;
            }
            let effective = statement_words(&self.cmd_tokens(seg), self.config);
            let words: Vec<_> = std::iter::once(InvocationWord::Literal(wrapper))
                .chain(effective.iter().map(|word| word.as_registry_word()))
                .collect();
            let frame = MemberFrame::of_row(
                call.grammar,
                call.grammar
                    .member_row(0, InvocationArguments::structured(&words), None)
                    .as_ref(),
            );
            self.extract_one_member(
                MemberExtraction {
                    call,
                    seg,
                    member,
                    kw: head,
                    base: 1,
                    frame,
                },
                class_qname,
                class_ivars,
                namespace,
            );
        }
    }

    /// Lift one body-bearing member call to a [`MethodDef`] unit, when its
    /// grammar layout and this consumer's kind routing say it opens a
    /// method frame.
    // One grammar-driven member extraction with shared validation paths.
    #[allow(clippy::too_many_lines)]
    fn extract_one_member(
        &mut self,
        ex: MemberExtraction<'_, '_>,
        class_qname: &str,
        class_ivars: &HashSet<String>,
        namespace: &str,
    ) {
        let MemberExtraction {
            call,
            seg,
            member,
            kw,
            base,
            frame,
        } = ex;
        let args = &seg.texts[base..];
        let Some(context) = self
            .invocation_metadata_context()
            .filter(|context| context.matches_registry(self.registry))
        else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        let surface = Some(context.context().authoring_query());
        // Argument layout comes from the grammar: which relative index (0-
        // based after the keyword) is the body / name / parameter list.
        let Some(body_rel) = self
            .source_member_body_indices(call.grammar, kw, seg, base)
            .and_then(|indices| indices.into_iter().find(|&index| index < args.len()))
        else {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        let kind = match frame {
            MemberFrame::Opens(kind) => kind,
            MemberFrame::NoFrame => {
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
            // The layout the body word sits in is unknowable (a computed word
            // where an optional one may stand): whichever method it defines,
            // no scan can read it, so the class abstains as a whole — the
            // same answer a computed name gives below.
            MemberFrame::Unreadable => {
                self.module
                    .oo_unanalysed_classes
                    .insert(class_qname.to_string());
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            }
        };
        // A member that also declares a variable (itcl `variable NAME ?init?
        // ?configbody?`, snit 1.x `onconfigure`) is a declaration whose
        // trailing script is not an ordinary method frame — skipped
        // (documented limit; `MemberFrame::of_row` already excludes them by
        // effect, this keeps the exclusion structural too).
        if member
            .indices_for_call_in(args, surface, ArgRole::VarWrite)
            .next()
            .is_some()
        {
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        }
        let b_idx = base + body_rel;
        let name_owned: String;
        let name: &str = if let Some(rel) = member
            .indices_for_call_in(args, surface, ArgRole::Name)
            .next()
        {
            let Some(n) = seg.texts.get(base + rel) else {
                self.module.oo_evidence.unretained_executable_roots = true;
                return;
            };
            n.as_str()
        } else {
            // Nameless members: the synthetic id is the keyword, matching
            // the established `<constructor>` / `<destructor>` scheme.
            name_owned = format!("<{kw}>");
            &name_owned
        };
        // Dynamic method names / non-static bodies are left un-lowered —
        // and recorded as an unanalysable member of the class: any method of
        // the class may have been (re)defined with a body no scan can read,
        // so per-class analysis must abstain for the whole class.
        if name.contains('$') || name.contains('[') || !seg_word_is_static_braced(seg, b_idx) {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        }
        // A parameter list Tcl would reject defines no method, and the class as
        // a whole becomes unanalysable — the same abstention the dynamic name /
        // body check above makes.
        let Some(params) = member_param_names(
            member,
            args,
            seg,
            base,
            WordValueRules::from_config(&self.config),
            self.invocation_dialect
                .and_then(tcl_registry::InvocationDialect::parameter_grammar),
            surface,
        ) else {
            self.module
                .oo_unanalysed_classes
                .insert(class_qname.to_string());
            self.module.oo_evidence.unretained_executable_roots = true;
            return;
        };
        // Lower the method body in its own frame (fresh const-map +
        // proc-depth) so the enclosing scope's tracked scalars
        // don't leak into the body's barrier-relaxation gate —
        // exactly as the `proc` case does — and suppress
        // nested-proc registration while doing so.
        let body_tok = seg.argv[b_idx];
        let body_off = body_tok.span.start() + u32::from(body_tok.content_offset);
        let execution_namespace = match call.grammar.member_current_namespace() {
            tcl_registry::definer::MemberCurrentNamespace::DefinedEntity => {
                crate::ir::ExecutionNamespace::exact(class_qname)
            }
            tcl_registry::definer::MemberCurrentNamespace::RuntimeReceiver => {
                crate::ir::ExecutionNamespace::RuntimeSelected
            }
        };
        // A runtime receiver namespace cannot be named during lowering. Keep
        // the lexical namespace for source reconstruction; the typed
        // execution context carried by MethodDef makes every proof consumer
        // abstain from relative command-resolution assumptions.
        let lowering_namespace = execution_namespace.for_head("").unwrap_or(namespace);
        self.proc_depth += 1;
        self.const_map_stack.push(HashMap::new());
        let prev_suppress = self.suppress_proc_register;
        self.suppress_proc_register = true;
        let retained = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.executed_script_for_word(body_tok.span).cloned());
        let body_script = if let Some(source) = retained {
            let tokens = self.cmd_tokens(seg);
            let namespace = b_idx
                .checked_sub(1)
                .and_then(|argument| {
                    self.original_body_namespace_context(&tokens, argument, &source)
                })
                .or_else(|| {
                    self.source_bindings
                        .as_ref()?
                        .executed_script_namespace_context(&source)
                });
            match namespace {
                Some(namespace) => self.lower_original_body(source, Some(&namespace)),
                // No receiver namespace is retained for a runtime-selected
                // body: report it through the same lexical approximation as an
                // unretained one, under the MethodDef's runtime-selected context.
                // It is not an original receiver frame, so it keeps no
                // executed source.
                None if execution_namespace == crate::ir::ExecutionNamespace::RuntimeSelected => {
                    let mut script =
                        self.lower_body(seg.texts[b_idx].as_str(), body_off, lowering_namespace);
                    script.executed_source = None;
                    script
                }
                None => self.lower_original_body(source, None),
            }
        } else {
            self.lower_body(seg.texts[b_idx].as_str(), body_off, lowering_namespace)
        };
        self.suppress_proc_register = prev_suppress;
        self.const_map_stack.pop();
        self.proc_depth -= 1;
        if self.method_body_may_install_definition(&body_script) {
            self.module.oo_evidence.unretained_executable_roots = true;
        }

        // Instance vars in scope: class-level decls (plus the grammar's
        // implicit member-body variables — snit's `self`/`selfns`/`type`/
        // `options`, a widget's `win`/`hull`, itcl's `this`) plus this
        // method's own top-level `variable` declarations. A write to any of
        // these mutates object state (impure for O126).
        let mut method_ivars = class_ivars.clone();
        method_ivars.extend(call.grammar.implicit_vars.iter().map(|s| (*s).to_string()));
        for statement in &body_script.statements {
            method_ivars.extend(self.advisory_method_instance_names(statement));
        }

        let original_receiver_context = body_script.executed_source.as_deref().and_then(|source| {
            self.source_bindings
                .as_ref()?
                .original_receiver_body_context(source)
        });
        let original_parameters = body_script.executed_source.as_deref().and_then(|source| {
            self.source_bindings
                .as_ref()?
                .original_script_formals(source)
        });
        if body_script.executed_source.is_some() {
            method_ivars.clear();
        }
        let def = MethodDef {
            definition_scope: call.definition_scope,
            class_name: class_qname.to_string(),
            method_name: name.to_string(),
            params,
            body: body_script,
            execution_namespace,
            kind,
            span: Some(seg.span),
            instance_vars: method_ivars,
            original_receiver_context,
            original_parameters,
        };
        self.module.retain_method(def);
    }
}

/// Formal-parameter names of a body-bearing member, read from the word its
/// grammar marks as the parameter list (`base + rel` in `seg.texts`).
///
/// An absent or empty word is the nullary member `method m {} {…}`, so it
/// yields no names; `None` is a list Tcl itself would reject.
fn member_param_names(
    member: &tcl_registry::definer::MemberSpec,
    args: &[String],
    seg: &SegmentedCommand,
    base: usize,
    rules: WordValueRules,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    surface: Option<tcl_dialect::model::SurfaceQuery<'_>>,
) -> Option<Vec<String>> {
    let param_text = member
        .indices_for_call_in(args, surface, ArgRole::ParamList)
        .next()
        .and_then(|rel| seg.texts.get(base + rel))
        .filter(|text| !text.is_empty());
    match param_text {
        Some(text) => parse_formal_param_names(text, rules, grammar),
        None => Some(Vec::new()),
    }
}

/// One body-bearing member call ready to lift — bundles the walk context so
/// [`Lowerer::extract_one_member`] stays under the argument limit.
#[derive(Clone, Copy)]
struct MemberExtraction<'a, 'b> {
    call: &'a DefinerCall,
    seg: &'a SegmentedCommand,
    member: &'b tcl_registry::definer::MemberSpec,
    /// The effective member keyword (the inner one for a wrapper prefix).
    kw: &'a str,
    /// Index of the member's first argument word in `seg.texts` (1, or 2
    /// past a wrapper prefix).
    pub(in super::super) base: usize,
    /// The frame the member's body opens, read off its row.
    pub(in super::super) frame: MemberFrame,
}

/// The frame a member statement's body opens, read off the statement's
/// [`MemberRow`] — its effect and the side it resolves to after every wrapper
/// shift — never its keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberFrame {
    /// A method frame of this shape ([`MethodKind::from_effect`]).
    Opens(MethodKind),
    /// A body that is not a method frame: an option accessor or mutator, a
    /// declaration's configuration script, a definition-time script whose
    /// namespace no name reaches.
    NoFrame,
    /// The registry cannot read the statement's layout (a computed word where
    /// an optional one may stand).
    Unreadable,
}

impl MemberFrame {
    /// The frame `row` opens under `grammar`.
    ///
    /// A callable opens the frame [`MethodKind::from_effect`] names. A script
    /// run once at definition opens a class-level frame when the family runs
    /// member bodies in the defined entity's own namespace
    /// ([`MemberCurrentNamespace::DefinedEntity`] — snit's `typeconstructor`
    /// is the proc `${type}::Snit_typeconstructor`, run with `type` bound);
    /// under [`MemberCurrentNamespace::RuntimeReceiver`] it runs in the class
    /// object's own namespace (`TclOO`'s `initialise`, in `::oo::ObjN`), which
    /// neither the class name nor a receiver names, so it opens none.
    fn of_row(grammar: &DefinitionBodyGrammar, row: Option<&MemberRow>) -> Self {
        let Some(row) = row else {
            return Self::Unreadable;
        };
        let kind = match row.effect {
            MemberEffect::Callable { role, .. } => MethodKind::from_effect(role, row.receiver),
            MemberEffect::InitScript {
                timing: InitTiming::AtDefinition,
                ..
            } if grammar.member_current_namespace() == MemberCurrentNamespace::DefinedEntity => {
                Some(MethodKind::ClassMethod)
            }
            _ => None,
        };
        kind.map_or(Self::NoFrame, Self::Opens)
    }
}

/// Value-conservative definition-member words from the original positioned
/// token owner, under the source's retained lexical axes. Wrapper prefixes
/// are composed by the caller after this projection.
pub(in super::super) fn statement_words(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
    tokens
        .words()
        .iter()
        .map(|word| {
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                WordValueRules::from_config(&config),
            )
        })
        .collect()
}

/// The instance variables one definition body declares at class level, per
/// the definer's grammar: every `all_args_var` member's names (the `TclOO`
/// `variable` slot — skipping a leading slot-operation word, which names no
/// variable) plus every `VarWrite`-role name (snit's `variable` /
/// `typevariable` / `component` / `typecomponent`, itcl's `variable` /
/// `common`), including itcl's access-modifier-wrapped spellings.
///
/// The result is the **union of every name ever declared** across the
/// block: slot removal operations (`variable -remove a`, `-set`, `-clear`)
/// are deliberately not folded here — the union is the conservative
/// direction for every consumer of [`crate::ir::MethodDef::instance_vars`]
/// (existence-fold abstention, W-family known-bound, O126 impurity), where
/// a stale extra name only widens abstention while a missed name produces a
/// false claim.  A *sibling* definition block may declare more, which is
/// why the caller accumulates a per-class union rather than using this
/// result directly.
pub(in super::super) fn declared_member_vars(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    segments: &[crate::segmenter::SegmentedCommand],
    surface: Option<tcl_dialect::model::SurfaceQuery<'_>>,
    config: tcl_lexer::LexerConfig,
) -> HashSet<String> {
    let mut out = HashSet::new();
    for seg in segments {
        if seg.is_partial || seg.texts.is_empty() {
            continue;
        }
        let head = seg.texts[0].as_str();
        let Some(member) = grammar.source_member_in(head.as_bytes(), surface) else {
            continue;
        };
        // Unwrap an access-modifier prefix (itcl `public variable x`,
        // TclOO 9's `private variable x`) one level.
        let (member, args, base): (&tcl_registry::definer::MemberSpec, &[String], usize) =
            if member.kind == tcl_registry::definer::MemberKind::Wrapper {
                match seg
                    .texts
                    .get(1)
                    .and_then(|inner| grammar.source_member_in(inner.as_bytes(), surface))
                {
                    Some(inner_member)
                        if inner_member.kind == tcl_registry::definer::MemberKind::Flat
                            && seg.texts.len() >= 3 =>
                    {
                        (inner_member, &seg.texts[2..], 2)
                    }
                    _ => continue,
                }
            } else {
                (member, &seg.texts[1..], 1)
            };
        let decoded: Vec<Option<String>> = args
            .iter()
            .enumerate()
            .map(|(relative, text)| {
                let index = base.checked_add(relative)?;
                if seg.single_token_word.get(index) != Some(&true) {
                    return None;
                }
                let token = seg.argv.get(index)?;
                crate::value_transfer::literal_token_value(text, token.kind, &config)
                    .map(std::borrow::Cow::into_owned)
            })
            .collect();
        let Some(values) = decoded.iter().cloned().collect::<Option<Vec<_>>>() else {
            // Unknown member operands widen the union of known literal names.
            out.extend(decoded.into_iter().flatten());
            continue;
        };
        if member.all_args_var {
            let names = match member.slot {
                Some(slot) => match slot.split_call(&values) {
                    Some((_, names)) => names,
                    None => continue,
                },
                None => &values,
            };
            out.extend(names.iter().cloned());
        } else {
            out.extend(
                member
                    .indices_for_call_in(&values, surface, ArgRole::VarWrite)
                    .filter_map(|relative| values.get(relative).cloned()),
            );
        }
    }
    out
}
