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

//! `oo::objdefine`: the receiver, the per-object members one block declares and its definition-abort diagnostics.

use super::super::super::state::Analyser;
use tcl_core_types::DiagCode;
use tcl_lexer::Token;

/// Where a class-manufacturing call's words sit, and what its manufacturer
/// contributes on top of the written body.
///
/// See [`Analyser::manufacturer_layout`].
#[derive(Clone, Copy)]
struct SelectedObjdefineBodyLayout {
    pub(in super::super) grammar: Option<&'static tcl_registry::definer::DefinitionBodyGrammar>,
    body_argument: Option<usize>,
    definer_disabled: bool,
}

impl Analyser {
    /// Handle `oo::objdefine $obj …` — record the object variable
    /// (so W308 can suppress unknown-method false positives from
    /// per-instance extensions) **and** walk the per-object
    /// definition so its method bodies are analysed exactly like an
    /// `oo::define` class's.
    ///
    /// `oo::objdefine` shares the `oo::define` member grammar, so the
    /// body / inline forms are parsed with the same helpers.  The
    /// members are collected into a *throwaway* `ClassDef` whose only
    /// purpose is to drive [`Self::parse_oo_definition_body`]'s
    /// method-body walk (variable / command resolution and in-body
    /// diagnostics light up as a side effect).  The `ClassDef` is
    /// deliberately **not** registered in `all_classes`: a per-object
    /// extension is not a class and must never surface in class
    /// listings, hover, rename, or completion.  Its method bodies
    /// home under a private synthetic name so the duplicate detector
    /// never confuses them with the object's real class methods.
    ///
    /// Returns `true` when it owns the command's body walk (an object
    /// name is present), mirroring [`Self::handle_oo_define_command`],
    /// so the generic body recursion does not also descend the body.
    ///
    /// A receiver written as a substitution keeps its **variable** name as
    /// the `objdefined_vars` / `object_methods` key — that is what a
    /// `$obj method` dispatch site is keyed by — but when the word also
    /// folds to a dominating constant (`foreach o {::a ::b} { oo::objdefine
    /// $o { … } }`) the per-object facts are recorded under the *object's*
    /// own name as well, so a bare `::a probe` call sees them too.  tclsh
    /// 9.0.4 / 8.6.16 both confirm every literal element really gains the
    /// method.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::OoObjdefine`].
    pub fn handle_oo_objdefine_at(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let Some(words) = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            cmd_tok.span.start(),
        ) else {
            return true;
        };
        let context = self.analysis_context();
        let Some((grammar, definer_disabled)) = words
            .with_source_schema(&context, |schema| {
                let command = schema.authored_source_descriptors().command;
                // A script operand enters the definition vocabulary only through
                // the selected configure layout. The inline member form instead
                // names a member of the selected descriptor's own grammar under
                // its complete selected layout; that member's body stays Tcl.
                let grammar = schema
                    .authored_source_definition_body_grammar()
                    .or_else(|| {
                        let declared = command.definition_body?;
                        (schema.authored_source_argument_roles().1
                            && args
                                .get(1)
                                .is_some_and(|keyword| declared.is_member(keyword)))
                        .then_some(declared)
                    })?;
                Some((grammar, !context.context().spec_available(command)))
            })
            .flatten()
        else {
            return true;
        };
        let body_argument = if args
            .get(1)
            .is_some_and(|subcommand| grammar.is_member(subcommand))
        {
            None
        } else {
            let body_arguments: Vec<_> = words
                .written_argument_roles()
                .into_iter()
                .filter_map(|(argument, role)| {
                    (role == tcl_registry::ArgRole::Body).then_some(argument)
                })
                .collect();
            let [body_argument] = body_arguments.as_slice() else {
                return true;
            };
            let Some(source_body) = words.original_argument_word(*body_argument) else {
                return true;
            };
            let Some(body_token) = arg_tokens.get(*body_argument) else {
                return true;
            };
            if !words.matches_source(source_body.image(), source_body.config())
                || !source_body
                    .tokens()
                    .iter()
                    .any(|token| token.span == body_token.span)
            {
                return true;
            }
            Some(*body_argument)
        };
        self.handle_oo_objdefine_with_layout(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            SelectedObjdefineBodyLayout {
                grammar: Some(grammar),
                body_argument,
                definer_disabled,
            },
        )
    }

    /// Apply the standalone object-definition layout used by direct test fixtures.
    #[cfg(test)]
    pub fn handle_oo_objdefine(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        let grammar = self.definition_grammar("oo::objdefine");
        self.handle_oo_objdefine_with_layout(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            SelectedObjdefineBodyLayout {
                grammar,
                body_argument: Some(1),
                definer_disabled: false,
            },
        )
    }

    fn handle_oo_objdefine_with_layout(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        layout: SelectedObjdefineBodyLayout,
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let (obj_name, resolved_obj) =
            self.oo_objdefine_receiver(args, arg_tokens, arg_single, scope_path);
        if let Some(name) = &obj_name {
            self.objdefined_vars.insert(name.clone());
        }
        // A receiver that resolves to no static name (`$objs($i)`, `[pick]`)
        // may define members on *any* object, so the per-object W315 gates
        // abstain file-wide.
        if obj_name.is_none() && resolved_obj.is_none() {
            self.objdefine_unresolved_receiver = true;
            for state in self.result.object_member_state.values_mut().flatten() {
                state.logical_source_members = None;
            }
        }

        // `oo::objdefine $obj` with no definition script — the object variable
        // is recorded above; there is nothing more to walk.
        if args.len() < 2 {
            return true;
        }

        if let Some(target) = arg_tokens.first() {
            self.retain_original_class_configuration_metadata(target.span);
        }

        // A throwaway holder for the walked members.  The method bodies home
        // under a private synthetic name (`@objdefine@…`, unrepresentable in
        // real Tcl) so a per-object `greet` never collides with the same-named
        // class method in the duplicate detector or the scope-name key.
        let synthetic = obj_name.as_ref().map_or_else(
            || "::@objdefine@".to_string(),
            |name| format!("::@objdefine@::{name}"),
        );
        let mut object_class = super::super::super::types::ClassDef {
            name: obj_name.clone().unwrap_or_default(),
            qualified_name: synthetic,
            ..Default::default()
        };

        // The receiver keys this block records under, and the binding the
        // block belongs to: the innermost proc/method frame is
        // the walk-time spelling of the binding identity consumers re-derive
        // from `ObjectMemberState::anchor_offset`.
        let keys: Vec<String> = [obj_name.clone(), resolved_obj.clone()]
            .into_iter()
            .flatten()
            .collect();
        let frame = self.innermost_frame_extent(scope_path);
        let objdefine_offset = arg_tokens.first().map_or(0, |t| t.span.start());
        // Seed the holder with the binding's accumulated per-object table, so
        // this block's retractions and renames run against the state the
        // interpreter would hold — a second block retracting what the first
        // declared removes it silently instead of recording a false abort,
        // which is exactly the hazard that kept W315 out of `oo::objdefine`.
        let (seed, prior_state_conditional) = keys
            .first()
            .and_then(|k| self.objdefine_binding_state(k, frame))
            .map_or_default(|st| (st.methods.clone(), st.conditional));
        object_class.methods = seed;
        if let Some(source) = self.logical_object_member_source_input()
            && let Some(previous) = keys
                .first()
                .and_then(|key| self.objdefine_binding_state(key, frame))
            && let Some(owner) = previous.logical_source_members.as_ref()
            && owner.matches_state(previous, &source.0, &source.1)
        {
            object_class
                .original_members
                .absorb(owner.original_members(), true);
        }

        self.walk_oo_objdefine_form(args, arg_tokens, scope_path, &mut object_class, layout);

        // **W315** candidates. `oo::objdefine` aborts the same
        // way a class definition does (`oo::objdefine $o { deletemethod m }`
        // naming a class-provided `m` errors `method m does not exist` on
        // 9.0.4 and 8.6.14 alike — a per-object retraction reaches only the
        // object's *own* table, never an inherited member).  The seeded walk
        // above judged this block against the binding's accumulated table, so
        // its aborts carry cross-block state; whether each one is *emittable*
        // needs document-wide facts (per-object declarations under any key,
        // receiver creations), so they are held for
        // `flush_objdefine_abort_diagnostics` rather than reported here.
        for abort in std::mem::take(&mut object_class.definition_aborts) {
            self.objdefine_abort_candidates.push(
                super::super::super::state::ObjdefineAbortCandidate {
                    abort,
                    receiver: keys.first().cloned().unwrap_or_default(),
                    prior_state_conditional,
                },
            );
        }

        self.fold_objdefine_block(&keys, frame, objdefine_offset, &object_class);

        true
    }

    /// Resolve the source spelling and any statically-known object target of
    /// an `oo::objdefine` receiver. The source spelling remains the dispatch
    /// key, while a resolved loop literal contributes an additional key.
    fn oo_objdefine_receiver(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> (Option<String>, Option<String>) {
        let Some(token) = arg_tokens.first() else {
            return (None, None);
        };
        let config = self.file_lexer_config();
        let Some(realm) = self.result.retained_command_realm() else {
            return (None, None);
        };
        let Some(image) = realm
            .original_source_image()
            .filter(|image| image.bytes() == self.source.as_bytes())
        else {
            return (None, None);
        };
        let Some((word, _)) =
            realm.original_written_word_at_span_in_source(image, token.span, config)
        else {
            return (None, None);
        };
        let operand = crate::registry_invocation::original_source_receiver_name_operand(&word);
        let written = match operand {
            Some(crate::registry_invocation::SourceReceiverNameOperand::Literal(value)) => {
                if self.selected_logical_declaration_advice() && value.as_bytes().contains(&0) {
                    return (None, None);
                }
                return (Some(value), None);
            }
            Some(crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name)) => {
                name
            }
            None => return (None, None),
        };
        if self.selected_logical_declaration_advice() && written.as_bytes().contains(&0) {
            return (None, None);
        }
        let resolved = self
            .resolve_dynamic_word(
                &args[0],
                Some(*token),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .filter(|name| {
                !name.is_empty() && *name != written && !crate::naming::is_dynamic_word(name)
            });
        (Some(written), resolved)
    }

    /// Walk an `oo::objdefine` inline member or its braced definition body
    /// through the same registry grammar used by `oo::define`.
    fn walk_oo_objdefine_form(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        object_class: &mut super::super::super::types::ClassDef,
        layout: SelectedObjdefineBodyLayout,
    ) {
        let Some(grammar) = layout.grammar else {
            return;
        };
        let inline_form = args
            .get(1)
            .is_some_and(|subcommand| grammar.is_member(subcommand));
        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            {
                if let Some(member) = inline_args
                    .first()
                    .and_then(|subcmd| grammar.member(subcmd))
                {
                    self.emit_w002_oo_member_option_disabled(
                        member,
                        &inline_args[1..],
                        &inline_tokens[1..],
                        layout.definer_disabled,
                    );
                }
                super::super::super::oo::parse_oo_define_inline_in(
                    self,
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    object_class,
                    Some(self.analysis_context().context().authoring_query()),
                    scope_path,
                );
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let (Some(body), Some(body_tok)) = (
            layout.body_argument.and_then(|argument| args.get(argument)),
            layout
                .body_argument
                .and_then(|argument| arg_tokens.get(argument).copied()),
        ) {
            self.parse_oo_definition_body(
                body,
                body_tok,
                object_class,
                scope_path,
                Some(grammar),
                layout.definer_disabled,
            );
        }
    }

    /// Record one walked `oo::objdefine` block's member declarations and fold
    /// it into each receiver key's durable [`super::super::super::types::ObjectMemberState`]
    /// — the durable home for per-object visibility.
    ///
    /// Declarations feed `object_methods` so `$obj m` navigation resolves the
    /// per-object override ahead of a same-named class method, keyed by the
    /// objdefine site's receiver offset — the receiver's *binding identity*,
    /// never the textual tail alone.  The diff runs per
    /// key against *that key's* own accumulated state: members another block
    /// already recorded for the key are carry-over, not new declarations,
    /// while a `foreach` re-dispatch's per-element key (empty state) still
    /// gains the block's members.  The fold then stores the block's effective
    /// table plus the explicit export/unexport flips, which may name
    /// *class*-provided members the object masks or revives.
    fn fold_objdefine_block(
        &mut self,
        keys: &[String],
        frame: Option<(u32, u32)>,
        objdefine_offset: u32,
        object_class: &super::super::super::types::ClassDef,
    ) {
        let block_conditional = self.conditional_depth > 0;
        let source = self.logical_object_member_source_input();
        for key in keys {
            let previous_source_complete =
                self.objdefine_binding_state(key, frame)
                    .is_none_or(|previous| {
                        source.as_ref().is_some_and(|(image, input)| {
                            previous
                                .logical_source_members
                                .as_ref()
                                .is_some_and(|owner| owner.matches_state(previous, image, input))
                        })
                    });
            let key_seed = self
                .objdefine_binding_state(key, frame)
                .map_or_default(|st| st.methods.clone());
            let mut new_methods: Vec<super::super::super::types::MethodDef> = object_class
                .methods
                .values()
                .filter(|m| key_seed.get(&m.name) != Some(*m))
                .cloned()
                .collect();
            // Source-ordered, so the recorded sequence is the same on every
            // run (and the same under each key) rather than the hash map's
            // iteration order.
            new_methods.sort_by_key(|def| def.name_span.start());
            if !new_methods.is_empty() {
                self.record_object_methods(key.clone(), &new_methods, objdefine_offset);
            }
            let state = self.objdefine_binding_state_mut(key, frame, objdefine_offset);
            state.methods.clone_from(&object_class.methods);
            for name in &object_class.exports {
                state.exports.insert(name.clone());
                state.unexports.remove(name);
            }
            for name in &object_class.unexports {
                state.unexports.insert(name.clone());
                state.exports.remove(name);
            }
            state.conditional |= block_conditional;
            state.logical_source_members = source
                .as_ref()
                .filter(|_| previous_source_complete && !object_class.member_set_incomplete)
                .and_then(|(image, input)| {
                    super::super::super::types::LogicalSourceObjectMemberState::capture(
                        image.clone(),
                        input.clone(),
                        state,
                        &object_class.original_members,
                    )
                });
        }
    }

    fn logical_object_member_source_input(
        &self,
    ) -> Option<(
        tcl_lexer::SourceImage,
        super::super::super::ResolvedAnalysisInput,
    )> {
        (self.selected_logical_declaration_advice() && !self.objdefine_unresolved_receiver)
            .then_some(())?;
        let input = self.resolved_analysis_input();
        let realm = self.result.retained_command_realm()?;
        realm
            .matches_resolved_analysis_input(&input)
            .then_some(())?;
        let image = realm.original_source_image()?;
        (image.bytes() == self.source.as_bytes()).then_some((image.clone(), input))
    }

    /// The accumulated [`super::super::super::types::ObjectMemberState`] for `key`'s
    /// binding in `frame`, if any `oo::objdefine` block was walked for it.
    fn objdefine_binding_state(
        &self,
        key: &str,
        frame: Option<(u32, u32)>,
    ) -> Option<&super::super::super::types::ObjectMemberState> {
        let idx = *self.objdefine_bindings.get(&(key.to_owned(), frame))?;
        self.result.object_member_state.get(key)?.get(idx)
    }

    /// [`Self::objdefine_binding_state`], creating the binding's entry on
    /// first use with `anchor` as its binding-identity anchor.
    fn objdefine_binding_state_mut(
        &mut self,
        key: &str,
        frame: Option<(u32, u32)>,
        anchor: u32,
    ) -> &mut super::super::super::types::ObjectMemberState {
        let states = self
            .result
            .object_member_state
            .entry(key.to_owned())
            .or_default();
        let idx = *self
            .objdefine_bindings
            .entry((key.to_owned(), frame))
            .or_insert_with(|| {
                states.push(super::super::super::types::ObjectMemberState {
                    anchor_offset: anchor,
                    ..Default::default()
                });
                states.len() - 1
            });
        &mut states[idx]
    }

    /// The byte extent of the innermost proc / method body scope along
    /// `scope_path` — the walk-time spelling of the binding identity the
    /// LSP side re-derives from a record's anchor offset (the innermost
    /// `Proc`/`Method` frame, or `None` at the top level).
    fn innermost_frame_extent(&self, scope_path: &[usize]) -> Option<(u32, u32)> {
        let mut scope = &self.result.global_scope;
        let mut extent = None;
        for &idx in scope_path {
            let Some(child) = scope.children.get(idx) else {
                break;
            };
            if matches!(
                child.kind,
                super::super::super::types::ScopeKind::Proc
                    | super::super::super::types::ScopeKind::Method
            ) && let Some(span) = child.body_span
            {
                extent = Some((span.start(), span.end()));
            }
            scope = child;
        }
        extent
    }

    /// Logical source compatibility for per-object definition-abort advice.
    /// Candidates retain reporting member/receiver labels, without a genuine
    /// selected operation or a closed current Native own-method table. Native,
    /// hosted and unknown input therefore supplies no diagnosis from these rows.
    /// Original member ledgers describe authored source order; constructed
    /// instance source carriers supply neither allocation nor absence closure.
    pub(in super::super::super) fn flush_objdefine_abort_diagnostics(&mut self) {
        let candidates = std::mem::take(&mut self.objdefine_abort_candidates);
        // naming.diagnostics.original-object-abort-admission
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
        if !self.result.allows_retained_logical_declaration_advice()
            || candidates.is_empty()
            || self.objdefine_unresolved_receiver
        {
            return;
        }
        let declared_anywhere: std::collections::HashSet<&str> = self
            .result
            .object_methods
            .values()
            .flatten()
            .map(|m| m.def.name.as_str())
            .collect();
        let mut diagnostics = Vec::new();
        for cand in &candidates {
            let emit = match cand.abort.kind {
                super::super::super::types::DefinitionAbortKind::RenameToItself => true,
                super::super::super::types::DefinitionAbortKind::DestinationExists => {
                    !cand.prior_state_conditional
                }
                super::super::super::types::DefinitionAbortKind::MissingMember => {
                    !cand.prior_state_conditional
                        && !declared_anywhere.contains(cand.abort.member.as_str())
                        && (self.result.instance_classes.contains_key(&cand.receiver)
                            || self
                                .result
                                .created_instance_commands
                                .contains(&cand.receiver))
                }
            };
            if emit {
                diagnostics.push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W315,
                    cand.abort.span,
                    cand.abort.object_message(),
                    super::super::super::types::Severity::Warning,
                ));
            }
        }
        self.result.diagnostics.extend(diagnostics);
    }

    /// Add one `oo::objdefine` site's per-object methods under `key`,
    /// skipping any the same site already contributed.
    ///
    /// The `foreach`-literal simulation re-dispatches an installer once per
    /// remaining element, and the ordinary walk already covered the first
    /// one, so the first element's site is visited twice under the
    /// *variable*'s key.  A single source site can never legitimately
    /// declare the same member twice, so `(objdefine_offset, name)` is an
    /// exact identity for "already recorded" — no iteration count is
    /// needed, and a genuinely separate `oo::objdefine` block on the same
    /// object (a different offset) still accumulates.
    fn record_object_methods(
        &mut self,
        key: String,
        methods: &[super::super::super::types::MethodDef],
        objdefine_offset: u32,
    ) {
        let recorded = self.result.object_methods.entry(key).or_default();
        for def in methods {
            if recorded
                .iter()
                .any(|m| m.objdefine_offset == objdefine_offset && m.def.name == def.name)
            {
                continue;
            }
            recorded.push(super::super::super::types::ObjectMethodDef {
                def: def.clone(),
                objdefine_offset,
            });
        }
    }
}
