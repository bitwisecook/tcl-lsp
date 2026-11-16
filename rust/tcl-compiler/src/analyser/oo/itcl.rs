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

//! `itcl::class` definitions.

use super::super::scope::scope_at_mut;
use super::super::state::Analyser;
use super::super::types::ClassDef;
use super::member_effects::apply_slot_member;
use super::{
    ClassBodyCtx, MemberForm, MemberLanding, MemberStatement, member_label, unwrap_wrapper_member,
};
use tcl_lexer::{Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, RelationSlot};

impl Analyser {
    /// Handle an [incr Tcl] `itcl::class Name { body }` (family `Itcl`),
    /// modelling it as a [`ClassDef`] with method scopes exactly like
    /// `oo::class` / `snit::type`, so `$this method` dispatch and reads of the
    /// instance / `common` variables inside method bodies don't false-fire.
    /// The access modifiers `public` / `protected` / `private` are prefix
    /// wrappers (registry `MemberKind::Wrapper`) and are unwrapped here.
    pub(in super::super) fn handle_itcl_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_itcl_class_source(super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: None,
            original: None,
        })
    }

    pub(in super::super) fn handle_itcl_class_source(
        &mut self,
        call: super::super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            original,
            ..
        } = call;
        let Some(grammar) = original
            .map(super::super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| self.definition_grammar(cmd_name))
            .filter(|g| g.family == tcl_registry::definer::DefinerFamily::Itcl)
        else {
            return false;
        };
        if args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        let raw_name = &args[0];
        let body = &args[1];
        // As for snit: a relative itcl class name homes to the namespace
        // current at the call, not the lexical one.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // Constructed key in, construction-inverse tail out: a colon
        // trim or `rsplit("::")` would collapse a lone-colon name.
        let qualified = super::super::handlers::qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[0].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok = arg_tokens[1];
        let doc = std::mem::take(&mut self.last_comment);
        let original_name = original
            .and_then(super::super::original_definer::OriginalClassDefinerSource::original_name)
            .or_else(|| self.original_provider_class_name(name_span));
        let source_name = original_name.as_ref().and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::provider_advice_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let mut class = ClassDef {
            source_name,
            name: simple.clone(),
            qualified_name: qualified.clone(),
            name_span,
            body_span: body_tok.span,
            metaclass: cmd_name.to_string(),
            metaclass_provenance: super::super::types::MetaclassProvenance::Observed,
            doc,
            ..Default::default()
        };
        if !body.is_empty() {
            self.parse_itcl_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, grammar,
            );
        }
        // For `my`-dispatch resolution — see
        // `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.retain_original_provider_class(original_name.as_ref(), &class);
        self.result
            .retain_class_declaration(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple, class);
        }
        true
    }

    /// Parse an itcl class body through each statement's registry row: a
    /// superclass relation (`inherit`) → superclasses, declared state
    /// (`variable` / `common`) → the variables in scope, a callable (`method` /
    /// `proc` / `constructor` / `destructor`) → a method scope in the table its
    /// row lands in.  Two passes (so a method can reference any variable
    /// regardless of declaration order); an access modifier is a wrapper whose
    /// shift the row already carries — its member's visibility.
    fn parse_itcl_definition_body(
        &mut self,
        body: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        grammar: &'static DefinitionBodyGrammar,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = crate::segmenter::segment_commands_with_offset_and_config(
            body,
            base_offset,
            self.file_lexer_config().nested(),
        );

        // itcl injects `this` (the object's own command) into every method body.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();

        // Pass 1: declared state names (`variable` / `common`, under any
        // access modifier) + the ancestry an `inherit` names — each read off
        // the statement's row.
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        for cmd in &cmds {
            if cmd.is_partial || cmd.texts.is_empty() {
                continue;
            }
            // A base class an `inherit` names is a command reference (the same
            // registry-driven path TclOO's `superclass`/`mixin` use), so
            // find-references / go-to-definition / rename reach it across files.
            self.record_member_command_references(grammar, &cmd.texts, &cmd.argv, scope_path);
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let args = statement.args();
            match statement.landing() {
                // Per-instance and per-type (`common`) state are both in scope
                // in an itcl method body.
                MemberLanding::State(_) => {
                    if let Some(name) = statement
                        .member
                        .indices_for_call_in(args, dialect, ArgRole::VarWrite)
                        .next()
                        .and_then(|i| args.get(i))
                    {
                        instance_vars.push(name.clone());
                    }
                }
                MemberLanding::Relation(RelationSlot::Superclass) => {
                    apply_slot_member(Some(statement.member), args, &mut class_def.superclasses);
                }
                _ => {}
            }
        }
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !implicit_vars.contains(&v.as_str()))
            .cloned()
            .collect();

        // Pass 2: method-bearing members in their own method scopes.
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
            if let Some((keyword, kw_args, kw_toks)) =
                unwrap_wrapper_member(grammar, &cmd.texts, &cmd.argv)
                && let Some(member) = grammar.member(keyword)
            {
                let parameter_indices: Vec<usize> = member
                    .indices_for_call_in(kw_args, dialect, ArgRole::ParamList)
                    .collect();
                super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                    self,
                    kw_args,
                    kw_toks,
                    &parameter_indices,
                );
            }
            // Only a body the row lands in a method table is a method scope:
            // `variable` / `common` are declarations (handled in pass 1) even
            // though `variable`'s optional config body carries an
            // `ArgRole::Body` (that body is highlighted by the token walker,
            // not recorded as a method here), and `inherit` carries no body.
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let MemberLanding::Method(kind) = statement.landing() else {
                continue;
            };
            let label = member_label(statement.member, statement.keyword(), statement.args());
            let mut ctx = ClassBodyCtx {
                grammar,
                class_def,
                class_qualified,
                scope_path,
            };
            let form = MemberForm {
                member: statement.member,
                kind,
                label: &label,
                visibility: statement.row.visibility.as_str(),
            };
            self.extract_class_member(
                statement.args(),
                statement.arg_tokens(),
                &mut ctx,
                &instance_vars,
                &form,
            );
        }
    }
}
