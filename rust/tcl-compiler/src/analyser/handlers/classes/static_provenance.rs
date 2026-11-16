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

//! The literal environment of a dominating statement prefix and the static values of words, commands and procedure results under it.

use super::super::super::state::Analyser;
use super::super::super::types::ProcDef;
use super::MAX_UNKNOWN_BODY_DEPTH;
use crate::segmenter::SegmentedCommand;
use crate::signature_scan::params::bind_proc_formals;
use tcl_lexer::Span;

enum StaticScriptOutcome {
    FallsThrough(String),
    Returns(String),
}

/// `word` with each `$name` / `${name}` reading one of `params` replaced by
/// the call site's argument at that parameter's position.
///
/// `None` — abstain — when the word carries a command substitution or a
/// backslash escape (whose value this cannot compute), when an interpolation
/// names something other than a parameter, or when the call site supplied no
/// argument for the parameter it does name.
pub(in super::super) fn substitute_bound_words(
    word: &str,
    params: &[String],
    args: &[String],
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    if word.contains('\\') {
        return None;
    }
    let length = u32::try_from(word.len()).ok()?;
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::document(word),
        Span::new(0, length),
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    )
    .ok()?;
    let mut out = String::with_capacity(word.len());
    for part in arena.list(arena.root()) {
        match part.part {
            tcl_lexer::ExecutablePart::Text(_) => {
                let text = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
                if text.contains('$') {
                    return None;
                }
                out.push_str(text);
            }
            tcl_lexer::ExecutablePart::Variable {
                name, index: None, ..
            } => {
                let name = std::str::from_utf8(arena.bytes(name)?).ok()?;
                let index = params.iter().position(|parameter| parameter == name)?;
                let value = args.get(index)?;
                if tcl_syntax::naming::is_dynamic_word(value) {
                    return None;
                }
                out.push_str(value);
            }
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
            | tcl_lexer::ExecutablePart::Command { .. }
            | tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => return None,
        }
    }
    Some(out)
}

impl Analyser {
    /// Execute the direct, dominating prefix of one selected script body.
    /// A Tcl result completion ends the path; statements after it cannot
    /// contribute provenance to a later creation.
    pub(super) fn apply_static_path_prefix(
        &mut self,
        proc_qname: &str,
        body_span: Span,
        stop_off: u32,
        env: &mut std::collections::HashMap<String, String>,
    ) -> Option<()> {
        let previous = self.static_eval_site;
        let mut applied = Some(());
        for seg in self.direct_statements_in_span(body_span)? {
            if seg.span.start() >= stop_off {
                break;
            }
            self.static_eval_site = Some(seg.span);
            if !self.static_statement_falls_through(&seg, 0) {
                applied = None;
                break;
            }
            if self
                .apply_static_provenance_statement(proc_qname, &seg, env, 0)
                .is_none()
            {
                applied = None;
                break;
            }
        }
        self.static_eval_site = previous;
        applied
    }

    /// Apply one dominating statement to the literal environment.  Binding
    /// layout comes from the registry's `set` handle/value descriptor; branch
    /// conditionality and expression/body positions come from traits and
    /// argument roles. Registry-declared variable/unknown writes invalidate
    /// the affected facts; an unsupported command abstains.
    fn apply_static_provenance_statement(
        &mut self,
        proc_qname: &str,
        seg: &SegmentedCommand,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let spec = registry.get(seg.name())?;

        if spec
            .traits
            .contains(tcl_registry::Traits::BRANCH_SELECTED_BODY)
        {
            let exprs =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Expr);
            let bodies =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            if let ([expr_idx], [body_idx]) = (exprs.as_slice(), bodies.as_slice())
                && let Some(condition) = self.static_expr_value(
                    seg.args().get(*expr_idx)?,
                    proc_qname,
                    env,
                    &mut Vec::new(),
                    depth,
                )
            {
                if condition {
                    self.invalidate_static_facts_written_by_script(
                        seg.args().get(*body_idx)?,
                        env,
                        depth + 1,
                    )?;
                }
                return Some(());
            }

            // For a multi-arm or unresolved branch, keep only facts no arm
            // can change. Argument layout and same-frame body identity both
            // come from the registry. An explicit/dynamic variable write or
            // a registry-declared unknown write invalidates the affected
            // fact; unrelated locals survive without selecting an arm.
            for body_idx in bodies {
                self.invalidate_static_facts_written_by_script(
                    seg.args().get(body_idx)?,
                    env,
                    depth + 1,
                )?;
            }
            return Some(());
        }

        if let Some(binding) = spec.binds_handle
            && matches!(
                binding.class_from,
                tcl_registry::handle_binding::HandleClassSource::ConstructionValue(_)
            )
            && let Some(bound) = binding.resolve(&args)
        {
            let value =
                self.static_word_value(bound.class_word, proc_qname, env, &mut Vec::new(), depth);
            if let Some(value) = value {
                env.insert(bound.name.to_owned(), value);
            } else {
                env.remove(bound.name);
            }
            return Some(());
        }

        for idx in registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::VarWrite)
        {
            if let Some(name) = seg.args().get(idx) {
                env.remove(name);
            }
        }
        self.invalidate_static_facts_written_by_segment(seg, env, depth + 1)
    }

    /// Invalidate literal facts a same-frame script may write.
    ///
    /// Variable positions, nested body positions, body-frame ownership, and
    /// unknown write effects are all registry declarations. A dynamic write
    /// target or an unknown command/effect can name any local and therefore
    /// clears the environment; a literal target kills only that fact.
    fn invalidate_static_facts_written_by_script(
        &self,
        body: &str,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            env.clear();
            return Some(());
        }
        for seg in crate::segmenter::segment_commands_with_offset_and_config(
            body,
            0,
            self.file_lexer_config().nested(),
        ) {
            self.invalidate_static_facts_written_by_segment(&seg, env, depth + 1)?;
        }
        Some(())
    }

    /// Invalidate facts written by one command whose body framing and
    /// variable layout are described by the registry.
    pub(in super::super) fn invalidate_static_facts_written_by_segment(
        &self,
        seg: &SegmentedCommand,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let Some(spec) = registry.get(seg.name()) else {
            env.clear();
            return Some(());
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let resolved_sub = args
            .first()
            .and_then(|first| spec.resolve_subcommand(first));

        if resolved_sub.is_some_and(|sub| sub.creates_scope_alias) {
            env.clear();
            return Some(());
        }

        for role in [
            tcl_registry::ArgRole::VarWrite,
            tcl_registry::ArgRole::LoopVarList,
        ] {
            for idx in registry.arg_indices_for_role(seg.name(), &args, role) {
                let Some(word) = seg.args().get(idx) else {
                    continue;
                };
                if tcl_syntax::naming::is_dynamic_word(word) {
                    env.clear();
                    return Some(());
                }
                let names: Vec<String> = if role == tcl_registry::ArgRole::LoopVarList {
                    let Ok(names) = self.word_rules().split_list(word) else {
                        env.clear();
                        return Some(());
                    };
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                } else {
                    vec![word.clone()]
                };
                for name in names {
                    let base = tcl_syntax::naming::split_element_ref(&name)
                        .map_or(name.as_str(), |(base, _)| base);
                    env.remove(base);
                }
            }
        }

        let plain_bodies = registry.plain_body_arg_indices(seg.name(), &args);
        for idx in &plain_bodies {
            if let Some(nested) = seg.args().get(*idx) {
                self.invalidate_static_facts_written_by_script(nested, env, depth + 1)?;
            }
        }

        // Control commands summarise their nested scripts as an unknown
        // effect. Same-frame bodies were inspected above, while structural
        // bodies execute in another frame. For a direct command with no
        // script argument, an unknown/variable write lacking a more precise
        // VarWrite role invalidates every remaining fact.
        let has_script_argument = tcl_registry::ArgRole::ALL
            .iter()
            .filter(|role| role.carries_script())
            .any(|&role| {
                !registry
                    .arg_indices_for_role(seg.name(), &args, role)
                    .is_empty()
            });
        if !has_script_argument {
            let effects = resolved_sub
                .filter(|sub| !sub.side_effects.lower_bound().is_empty())
                .map_or(spec.side_effects, |sub| sub.side_effects);
            if effects.lower_bound().iter().any(|effect| {
                effect.writes
                    && matches!(
                        effect.target,
                        tcl_registry::prelude::SideEffectTarget::Unknown
                            | tcl_registry::prelude::SideEffectTarget::Variable
                    )
            }) && registry
                .arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::VarWrite)
                .is_empty()
            {
                env.clear();
            }
        }
        Some(())
    }

    /// Evaluate a static word under `env`. Braced words are literal; a whole
    /// command substitution dispatches through registry folds or a proven
    /// user-procedure result; composite variable words use the same strict
    /// substitution routine as direct parameter names.
    pub(super) fn static_word_value(
        &mut self,
        word: &str,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let trimmed = word.trim();
        if let Some(inner) = trimmed.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            return Some(inner.to_owned());
        }
        let unquoted = trimmed
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(trimmed);
        if let Some((head, args)) = crate::value_shapes::parse_command_substitution_with_config(
            unquoted,
            self.file_lexer_config().nested(),
        ) {
            if !self.result.allows_lexical_declaration_advice() {
                let seg = self.static_substitution_segment(unquoted)?;
                return self.static_segment_value(&seg, proc_qname, env, stack, depth);
            }
            let values: Vec<Option<String>> = args
                .iter()
                .map(|arg| self.static_word_value(arg, proc_qname, env, stack, depth))
                .collect();
            return self.static_command_value(&head, &values, proc_qname, env, stack, depth);
        }
        let mut names: Vec<String> = env.keys().cloned().collect();
        names.sort();
        let values: Vec<String> = names
            .iter()
            .filter_map(|name| env.get(name).cloned())
            .collect();
        substitute_bound_words(unquoted, &names, &values, self.file_lexer_config())
    }

    /// Evaluate a registry-foldable command or a statically-resolved user
    /// procedure. Unknown arguments are allowed into a user procedure because
    /// a selected path may not read them; registry folds require every input.
    pub(in super::super) fn static_command_value(
        &mut self,
        head: &str,
        args: &[Option<String>],
        proc_qname: &str,
        _env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        // A materialised String command has no genuine original call point,
        // occupied implementation or captured prefix. Registry folds here
        // belong only to the explicit Logical assistance domain.
        if !self.result.allows_lexical_declaration_advice() || depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        if registry.get(head).is_some() {
            if !self.static_provenance_command_is_trusted(&crate::naming::qualify("::", head)) {
                return None;
            }
            return self.static_registry_fold(head, args);
        }

        let qname = self.resolve_static_proc_name(proc_qname, head)?;
        if !self.static_provenance_command_is_trusted(&qname)
            || stack.iter().any(|seen| seen == &qname)
        {
            return None;
        }
        stack.push(qname.clone());
        let result = self.static_proc_result(&qname, args, stack, depth + 1);
        stack.pop();
        result
    }

    /// Fold a registry command whose occupied implementation the caller has
    /// already established, lexically or at its source position.
    fn static_registry_fold(&mut self, head: &str, args: &[Option<String>]) -> Option<String> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let spec = context.context().resolve_spec(registry, head)?;
        let values: Vec<&str> = args
            .iter()
            .map(|value| value.as_deref())
            .collect::<Option<_>>()?;
        let dialect = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .map(|mut dialect| {
                dialect.lexer_grammar = self.grammar();
                dialect.word_values = self.word_rules();
                dialect
            });
        if spec.subcommands.is_empty() {
            return match dialect {
                Some(dialect) => spec.run_const_fold_in(&values, dialect),
                None => spec.run_const_fold(&values, None),
            };
        }
        let (sub, rest) = values.split_first()?;
        let subcommand = spec.resolve_subcommand(sub)?;
        match dialect {
            Some(dialect) => subcommand.run_const_fold_in(rest, dialect),
            None => subcommand.run_const_fold(rest, None),
        }
    }

    /// The one command of a `[…]` substitution written inside the statement
    /// under evaluation, at its own source position. Two identical
    /// substitutions in one statement abstain.
    fn static_substitution_segment(&self, substitution: &str) -> Option<SegmentedCommand> {
        let site = self.static_eval_site?;
        let statement = self
            .source
            .get(site.start() as usize..site.end() as usize)?;
        let mut found = statement.match_indices(substitution);
        let (at, _) = found.next()?;
        if found.next().is_some() {
            return None;
        }
        let start = site
            .start()
            .checked_add(u32::try_from(at).ok()?)?
            .checked_add(1)?;
        let end = start.checked_add(u32::try_from(substitution.len()).ok()?.checked_sub(2)?)?;
        let mut commands = self.direct_statements_in_span(Span::new(start, end))?;
        if commands.len() != 1 {
            return None;
        }
        commands.pop()
    }

    /// Evaluate one command at its source position. The head is the
    /// procedure or registry builtin that position is proved to dispatch to;
    /// its spelling selects nothing.
    fn static_segment_value(
        &mut self,
        seg: &SegmentedCommand,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let previous = self.static_eval_site.replace(seg.span);
        let values: Vec<Option<String>> = seg
            .args()
            .iter()
            .map(|arg| self.static_word_value(arg, proc_qname, env, stack, depth))
            .collect();
        let result = self.static_positioned_command_value(seg, &values, stack, depth);
        self.static_eval_site = previous;
        result
    }

    fn static_positioned_command_value(
        &mut self,
        seg: &SegmentedCommand,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if let Some(proc_def) = self.original_static_procedure_at(seg) {
            // The position's own lookup proves the implementation. The
            // head must still be that procedure's own name (an alias does
            // not lend it), and a command-table mutation that may name it
            // anywhere in the document blocks the fold, as on the lexical
            // path.
            let qname = proc_def.qualified_name.clone();
            if tcl_syntax::naming::key_tail(seg.name()) != tcl_syntax::naming::key_tail(&qname)
                || !self.static_provenance_command_is_trusted(&qname)
                || stack.iter().any(|seen| seen == &qname)
            {
                return None;
            }
            stack.push(qname);
            let result = self.static_proc_def_result(&proc_def, args, stack, depth + 1);
            stack.pop();
            return result;
        }
        (self.original_static_builtin_at(seg)
            && self.static_provenance_command_is_trusted(&crate::naming::qualify("::", seg.name())))
        .then(|| self.static_registry_fold(seg.name(), args))?
    }

    /// Whether this position is proved to dispatch to the registry builtin
    /// its head spells, with no alias prefix.
    fn original_static_builtin_at(&self, segment: &SegmentedCommand) -> bool {
        let config = self.file_lexer_config();
        if !self
            .result
            .matches_original_source_image(&tcl_lexer::SourceImage::document(&self.source), config)
        {
            return false;
        }
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            config,
            segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.proved_execution_target())
            .is_some_and(|target| {
                target.registry_backed
                    && target.kind == crate::command_binding::BindingKind::Builtin
                    && target.prepended.is_empty()
                    && crate::naming::qualify("::", segment.name()) == target.command
            })
    }

    /// On a release profile a call of a command the registry does not
    /// describe has no schema to read; its body, if the document declares
    /// one, is a definition body the trust scan skips.
    fn release_call_has_no_schema(
        &self,
        registry: &tcl_registry::CommandRegistry,
        seg: &SegmentedCommand,
    ) -> bool {
        self.result
            .resolved_input
            .as_ref()
            .is_some_and(|input| !input.has_logical_source_name_context())
            && registry.get(seg.name()).is_none()
    }

    /// Whether `query` still denotes its declared builtin/procedure for the
    /// narrow static-provenance proof.
    ///
    /// Command-table mutators are discovered only through the registry's
    /// [`CommandTableEffect`](tcl_registry::CommandTableEffect). Dynamic
    /// target words retain their literal fragments instead of collapsing the
    /// entire command table: `${ns}::define::$method` cannot name `::string`,
    /// whereas `$name` can name anything and therefore abstains. This scan is
    /// flow-insensitive, so a compatible mutation before or after the proof
    /// site blocks it equally.
    pub(in super::super) fn static_provenance_command_is_trusted(&self, query: &str) -> bool {
        let Some(context) = self.retained_static_source_context() else {
            return false;
        };
        let registry = context.commands().as_ref();
        let Some(statements) = self.statements_in_span(Span::new(
            0,
            u32::try_from(self.source.len()).unwrap_or(u32::MAX),
        )) else {
            return false;
        };
        let expected_declarations = usize::from(self.result.all_procs.contains_key(query));
        let mut matching_declarations = 0_usize;

        for seg in statements {
            // A declaration body is dormant until its command is invoked.
            // Its dynamic `proc`/`rename`/`interp alias` words therefore do
            // not mutate the load-time command table merely because the body
            // is present in the source. Invoked bodies are evaluated through
            // `static_proc_result`, whose unsupported/mutating statements
            // still make the proof abstain.
            if self
                .result
                .offset_is_inside_any_definition_body(seg.span.start())
            {
                continue;
            }
            let Some(original) =
                crate::registry_invocation::source_structure::source_registry_words(
                    &self.source,
                    &self.result,
                    &seg,
                )
            else {
                if self.release_call_has_no_schema(registry, &seg) {
                    continue;
                }
                return false;
            };
            let Some(transitions) = original.with_source_schema(&context, |schema| {
                schema
                    .semantics
                    .state_transitions
                    .resolve_after_success_with_effect_coverage(schema.words.arguments())
                    .0
            }) else {
                return false;
            };
            let input = self
                .result
                .resolved_input
                .as_ref()
                .expect("checked original source input");
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                input,
                self.file_lexer_config(),
                Some(input.unit_profile()),
            );
            let overlap = |subject: &tcl_registry::TransitionSubject| {
                let word = original.original_argument_word(subject.argument_index()?)?;
                crate::dynamic_names::original_command_name_overlap(word, query, metadata).or_else(
                    || {
                        // Outside the Logical domain no fragment of a
                        // computed word is read: it may name anything.
                        (!input.has_logical_source_name_context()).then(|| {
                            crate::dynamic_names::release_command_name_overlap(word, query)
                        })
                    },
                )
            };
            for transition in transitions.command_bindings() {
                match transition {
                    tcl_registry::CommandBindingTransition::Define { name, .. } => {
                        let Some(name) = overlap(name) else {
                            return false;
                        };
                        if name.may_equal {
                            if name.dynamic {
                                return false;
                            }
                            matching_declarations += 1;
                            if matching_declarations > expected_declarations {
                                return false;
                            }
                        }
                    }
                    tcl_registry::CommandBindingTransition::Move { from, to } => {
                        let (Some(old), Some(new)) = (overlap(from), overlap(to)) else {
                            return false;
                        };
                        if old.may_equal || (!new.empty && new.may_equal) {
                            return false;
                        }
                    }
                    tcl_registry::CommandBindingTransition::Delete { name, .. }
                    | tcl_registry::CommandBindingTransition::Alias { alias: name, .. } => {
                        let Some(name) = overlap(name) else {
                            return false;
                        };
                        if name.may_equal {
                            return false;
                        }
                    }
                    // A mutation whose very shape is unknown could name
                    // anything.
                    tcl_registry::CommandBindingTransition::Unknown { .. } => return false,
                }
            }
        }
        matching_declarations == expected_declarations
    }

    /// A genuine positioned occupied implementation for static continuation.
    /// Metadata joins its original allocation; a final report name cannot
    /// supply a replacement, captured prefix or current command identity.
    pub(super) fn original_static_procedure_at(
        &self,
        segment: &SegmentedCommand,
    ) -> Option<ProcDef> {
        let config = self.file_lexer_config();
        self.result
            .matches_original_source_image(&tcl_lexer::SourceImage::document(&self.source), config)
            .then_some(())?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            config,
            segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref()?;
        let target = binding.proved_execution_target()?;
        if target.registry_backed
            || target.kind != crate::command_binding::BindingKind::Proc
            || !target.prepended.is_empty()
        {
            return None;
        }
        let reference = binding.evaluated_command_reference()?;
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())?;
        if Some(definition.allocation()) != target.implementation_allocation.as_ref() {
            return None;
        }
        self.result
            .proc_for_definition(definition, &self.source)
            .cloned()
    }

    /// Explicit Logical assistance only. This unpositioned String interface
    /// has no original call/allocation point from which Native or hosted
    /// occupied implementation could be proved.
    pub(in super::super) fn resolve_static_proc_name(
        &self,
        caller: &str,
        head: &str,
    ) -> Option<String> {
        if !self.result.allows_lexical_declaration_advice() {
            return None;
        }
        let absolute = if tcl_syntax::naming::is_absolute(head) {
            crate::naming::qualify("", head)
        } else {
            let (ns, _) = crate::naming::key_holder_and_tail(caller);
            crate::naming::qualify(ns, head)
        };
        if self.result.all_procs.contains_key(&absolute) {
            return Some(absolute);
        }
        let global = crate::naming::qualify("", head);
        self.result
            .all_procs
            .contains_key(&global)
            .then_some(global)
    }

    /// Evaluate the return of a small user procedure from registry command
    /// folds. Every unsupported control-flow or command result abstains.
    pub(in super::super) fn static_proc_result(
        &mut self,
        qname: &str,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        // Materialised substitution strings have no original operation point.
        // A positioned caller must use original_static_procedure_at instead.
        if !self.result.allows_lexical_declaration_advice() {
            return None;
        }
        let proc_def = self.result.all_procs.get(qname)?.clone();
        self.static_proc_def_result(&proc_def, args, stack, depth)
    }

    fn static_proc_def_result(
        &mut self,
        proc_def: &ProcDef,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        self.retained_static_source_context()?;
        if proc_def.params_computed {
            return None;
        }
        let mut env = std::collections::HashMap::new();
        for (name, value) in bind_proc_formals(&proc_def.params, args, self.word_rules())? {
            if let Some(value) = value {
                env.insert(name, value);
            }
        }
        let qname = proc_def.qualified_name.as_str();
        match self.static_script_result(qname, proc_def.body_span, &mut env, stack, depth)? {
            StaticScriptOutcome::FallsThrough(value) | StaticScriptOutcome::Returns(value) => {
                Some(value)
            }
        }
    }

    fn static_script_result(
        &mut self,
        qname: &str,
        body_span: Span,
        env: &mut std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<StaticScriptOutcome> {
        let previous = self.static_eval_site;
        let outcome = self.static_script_statements(qname, body_span, env, stack, depth);
        self.static_eval_site = previous;
        outcome
    }

    fn static_script_statements(
        &mut self,
        qname: &str,
        body_span: Span,
        env: &mut std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<StaticScriptOutcome> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let statements = self.direct_statements_in_span(body_span)?;
        let mut result = String::new();
        for seg in statements {
            self.static_eval_site = Some(seg.span);
            let raw_args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            match self.segment_invocation_completion(&seg) {
                tcl_registry::registry::InvocationCompletion::ReturnsResult(Some(idx)) => {
                    let value =
                        self.static_word_value(seg.args().get(idx)?, qname, env, stack, depth)?;
                    return Some(StaticScriptOutcome::Returns(value));
                }
                tcl_registry::registry::InvocationCompletion::ReturnsResult(None) => {
                    return Some(StaticScriptOutcome::Returns(String::new()));
                }
                tcl_registry::registry::InvocationCompletion::Terminates
                | tcl_registry::registry::InvocationCompletion::Unknown => return None,
                tcl_registry::registry::InvocationCompletion::FallsThrough => {}
            }
            let spec = registry.get(seg.name())?;
            if spec
                .traits
                .contains(tcl_registry::Traits::BRANCH_SELECTED_BODY)
            {
                if registry.control_invocation_valid(
                    seg.name(),
                    &raw_args,
                    Some(context.context().authoring_query()),
                ) != Some(true)
                {
                    return None;
                }
                if !matches!(
                    spec.lowering_hook,
                    Some(
                        tcl_registry::hooks::LoweringHookId::If
                            | tcl_registry::hooks::LoweringHookId::Switch
                    )
                ) {
                    return None;
                }
                let mut selected = None;
                let controls = self.control_arms_for_segment(&seg);
                if !controls.complete {
                    return None;
                }
                for arm in controls.arms {
                    match self.control_body_is_selected(qname, &arm, env, depth) {
                        Some(true) if selected.is_none() => selected = Some(arm.body_span),
                        Some(true) | None => return None,
                        Some(false) => {}
                    }
                }
                if let Some(span) = selected {
                    match self.static_script_result(qname, span, env, stack, depth + 1)? {
                        StaticScriptOutcome::Returns(value) => {
                            return Some(StaticScriptOutcome::Returns(value));
                        }
                        StaticScriptOutcome::FallsThrough(value) => result = value,
                    }
                } else {
                    result.clear();
                }
                continue;
            }
            if let Some(binding) = spec.binds_handle
                && matches!(
                    binding.class_from,
                    tcl_registry::handle_binding::HandleClassSource::ConstructionValue(_)
                )
                && let Some(bound) = binding.resolve(&raw_args)
            {
                result = self.static_word_value(bound.class_word, qname, env, stack, depth)?;
                env.insert(bound.name.to_owned(), result.clone());
                continue;
            }
            let values: Vec<Option<String>> = seg
                .args()
                .iter()
                .map(|arg| self.static_word_value(arg, qname, env, stack, depth))
                .collect();
            result = if self.result.allows_lexical_declaration_advice() {
                self.static_command_value(seg.name(), &values, qname, env, stack, depth)?
            } else {
                self.static_positioned_command_value(&seg, &values, stack, depth)?
            };
        }
        Some(StaticScriptOutcome::FallsThrough(result))
    }

    pub(super) fn static_expr_value(
        &mut self,
        word: &str,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let mut expr = word
            .trim()
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or(word.trim())
            .to_owned();
        while let Some(start) = expr.find('[') {
            let mut nesting = 0_u32;
            let mut end = None;
            for (idx, ch) in expr[start..].char_indices() {
                match ch {
                    '[' => nesting += 1,
                    ']' => {
                        nesting = nesting.saturating_sub(1);
                        if nesting == 0 {
                            end = Some(start + idx + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let end = end?;
            let substitution = expr.get(start..end)?;
            let folded = self.static_word_value(substitution, proc_qname, env, stack, depth)?;
            expr.replace_range(start..end, &folded);
        }
        // Each value is its text, which the expression engine reads as Tcl
        // reads a literal operand.
        let constants: crate::tcl_expr_eval::Env = env
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    crate::tcl_expr_eval::EnvValue::Str(value.clone()),
                )
            })
            .collect();
        crate::tcl_expr_eval::evaluate_expr_with_constants(
            &crate::parse_expr_for_profile(&expr, Some(self.profile)),
            &constants,
            crate::tcl_expr_eval::FoldPolicy::from_registry(context.commands()),
        )
        .map(|value| value != 0)
    }
}
