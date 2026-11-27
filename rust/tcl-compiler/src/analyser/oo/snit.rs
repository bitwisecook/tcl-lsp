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

//! snit type and widget definitions.

use super::super::scope::scope_at_mut;
use super::super::state::Analyser;
use super::super::types::ClassDef;
use super::{ClassBodyCtx, MemberForm, MemberLanding, MemberStatement, member_label};
use tcl_lexer::{Token, TokenType};
use tcl_registry::arg_role::ArgRole;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberReceiver, StateScope};

/// Implicit variable snit injects into `typemethod` / `typeconstructor` bodies.
///
/// The instance-body implicits (`self` / `selfns` / `type` / `options`) come
/// from the definer's registry grammar (`implicit_vars`); this is only the
/// narrower *type*-body subset, which the flat grammar list doesn't
/// distinguish — a `typemethod` body sees `type` but not `self`.
// registry-axis-ok: special_vars — snit's implicit type variable in a typemethod body; until the snit definition grammar declares its implicit variables
const SNIT_TYPE_IMPLICIT: &[&str] = &["type"];

/// The snit definer being parsed.  Which implicit variables its member
/// bodies see — including the widget-only `win` / `hull` pair — comes from
/// the grammar's own `implicit_vars` (`snit::widget` / `widgetadaptor` carry
/// `SNIT_WIDGET_GRAMMAR`), so no consumer matches the definer's name suffix.
struct SnitDefiner {
    pub(super) grammar: &'static DefinitionBodyGrammar,
}

impl Analyser {
    /// Handle a snit (tcllib) type/widget definition —
    /// ``snit::type``/``snit::widget``/``snit::widgetadaptor Name { body }``
    /// (and their `::`-qualified forms).  Snit reinterprets its body as a
    /// class description, exactly like an `oo::class` body, so we model it as
    /// a real [`ClassDef`] with method scopes — object dispatch inside method
    /// bodies (`$self foo`, `$component bar`) is then recognised as in-method
    /// dispatch (no false W307) and snit's implicit instance variables don't
    /// surface as read-before-set / unused.
    pub(in super::super) fn handle_snit_type_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_snit_type_source(super::super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: None,
            original: None,
        })
    }

    pub(in super::super) fn handle_snit_type_source(
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
        // Which commands are snit definers, their member sub-keywords + argument
        // layout, and the variables snit injects into member bodies are all
        // registry data (a `Snit`-family definition-body grammar) — not a
        // hardcoded name / member / implicit-var list.  A new snit-like definer
        // is picked up automatically once its spec carries the grammar.  The
        // grammar is `&'static`, so it outlives the immutable registry borrow
        // released here before the `&mut self` work below.
        let Some(grammar) = original
            .map(super::super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| self.definition_grammar(cmd_name))
            .filter(|g| {
                g.profile.installation.is_some_and(|installation| {
                    installation.dispatcher == tcl_registry::definer::DefinitionDispatcher::SnitType
                })
            })
        else {
            return false;
        };
        if args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        let raw_name = &args[0];
        let body = &args[1];
        // A relative type name homes to the namespace current at the call —
        // the command-resolution namespace, so a `snit::type` created inside
        // `proc ::ns::p {}` becomes `::ns::T`.
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
            let definer = SnitDefiner { grammar };
            self.parse_snit_definition_body(
                body, body_tok, &mut class, &qualified, scope_path, &definer,
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

    /// Parse a snit type/widget body into methods + variable declarations, in
    /// two passes (so a method can reference any instance/type variable
    /// regardless of declaration order).
    fn parse_snit_definition_body(
        &mut self,
        body: &str,
        body_tok: Token,
        class_def: &mut ClassDef,
        class_qualified: &str,
        scope_path: &[usize],
        definer: &SnitDefiner,
    ) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        let grammar = definer.grammar;
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        let cmds = tcl_source::segment::segment_commands_with_offset_and_config(
            body,
            base_offset,
            self.file_lexer_config().nested(),
        );

        // The variables snit injects into every member body come from the
        // definer's registry grammar (`implicit_vars`); the widget grammar's
        // list already includes the widget-only `win` / `hull` pair.
        let implicit_vars = grammar.implicit_vars;
        let mut instance_vars: Vec<String> =
            implicit_vars.iter().map(|s| (*s).to_string()).collect();
        let mut type_vars: Vec<String> = SNIT_TYPE_IMPLICIT
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        // First pass: collect declared instance / type variable + component
        // names.  Which members declare state, and whose, is each statement's
        // row (`StateDeclaration`'s scope: per-type state is the type
        // variables', per-instance the instance variables'); which argument
        // holds the name is the member's registry layout.
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        for cmd in &cmds {
            if cmd.is_partial {
                continue;
            }
            let Some(statement) = MemberStatement::read(grammar, &cmd.texts, &cmd.argv, dialect)
            else {
                continue;
            };
            let MemberLanding::State(scope) = statement.landing() else {
                continue;
            };
            let args = statement.args();
            let Some(name) = statement
                .member
                .indices_for_call_in(args, dialect, ArgRole::VarWrite)
                .next()
                .and_then(|i| args.get(i))
            else {
                continue;
            };
            match scope {
                StateScope::PerType => type_vars.push(name.clone()),
                StateScope::PerInstance => instance_vars.push(name.clone()),
                // An option is configured through the instance, not read as
                // a variable of its own.
                StateScope::Option => {}
            }
        }

        // Record the *explicit* instance + type variables on the class —
        // method-scope seeding and the W307 dispatch-source suppression both
        // read `ClassDef::variables`.  The base grammar's implicit scalars
        // (`self`/`selfns`/`type`/`options` — `SNIT_GRAMMAR.implicit_vars`,
        // shared by every snit definer) and the type-implicit `type` are
        // filtered; a widget's injected `win`/`hull` (the extra names its
        // own grammar adds on top of the base set) are kept — they are real
        // per-instance variables worth surfacing on the class record.
        let base_implicits = tcl_registry::definer::SNIT_GRAMMAR.implicit_vars;
        class_def.variables = instance_vars
            .iter()
            .filter(|v| !base_implicits.contains(&v.as_str()))
            .chain(
                type_vars
                    .iter()
                    .filter(|v| !SNIT_TYPE_IMPLICIT.contains(&v.as_str())),
            )
            .cloned()
            .collect();

        // Second pass: analyse method-bearing declarations in method scopes.
        for cmd in &cmds {
            if cmd.is_partial || cmd.texts.is_empty() {
                continue;
            }
            self.retain_original_member_metadata(grammar, cmd, class_def, scope_path);
            let mut ctx = ClassBodyCtx {
                grammar,
                class_def,
                class_qualified,
                scope_path,
            };
            self.dispatch_snit_member(cmd, &mut ctx, &instance_vars, &type_vars);
        }
    }

    /// Dispatch one snit body statement by its row: a callable or a
    /// definition-time script to the method extractor, a namespace procedure
    /// to the ordinary proc handler.  Split out of
    /// [`Self::parse_snit_definition_body`] so the two-pass walk stays small.
    ///
    /// Recognition, argument layout (which words are the name / parameter list
    /// / value var / body) and what the statement declares all come from its
    /// registry row ([`member_landing`], the routing the `TclOO` fold shares):
    /// a type-object row's body sees the type variables and lands among the
    /// class methods, an instance row's sees the instance variables.  Only the
    /// synthetic name of the nameless forms is decided here.
    fn dispatch_snit_member(
        &mut self,
        cmd: &tcl_source::segment::SegmentedCommand,
        ctx: &mut ClassBodyCtx<'_>,
        instance_vars: &[String],
        type_vars: &[String],
    ) {
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        let (sub_args, sub_tokens) = (&cmd.texts[1..], cmd.argv.get(1..).unwrap_or(&[]));
        if let Some(member) = ctx.grammar.member(&cmd.texts[0]) {
            let parameter_indices = self.member_formal_parameter_indices(
                ctx.grammar,
                member,
                sub_args,
                sub_tokens,
                ctx.class_def,
            );
            super::super::diagnostics::emit_invalid_formal_parameter_list_diagnostics(
                self,
                sub_args,
                sub_tokens,
                &parameter_indices,
            );
        }
        let Some(statement) = MemberStatement::read(ctx.grammar, &cmd.texts, &cmd.argv, dialect)
        else {
            return;
        };
        match statement.landing() {
            // A type-private `proc name args body` is a procedure in the
            // type's namespace — analyse it as an ordinary proc in the
            // enclosing scope, not a method.
            MemberLanding::Procedure => {
                // No per-argument single-token info is threaded this deep into
                // the snit member dispatcher; `&[]` is the same safe default
                // `resolve_dynamic_word` already falls back to elsewhere (a
                // dynamic type-private proc name still gets a chance to resolve
                // via `fold_interpolation_single`, just not the single-`$var`
                // fast path).
                self.handle_proc_command(
                    statement.keyword(),
                    statement.args(),
                    statement.arg_tokens(),
                    &[],
                    ctx.scope_path,
                );
            }
            MemberLanding::Method(kind) => {
                let seed_vars = if statement.row.receiver == MemberReceiver::TypeObject {
                    type_vars
                } else {
                    instance_vars
                };
                let label = member_label(statement.member, statement.keyword(), statement.args());
                let form = MemberForm {
                    member: statement.member,
                    kind,
                    label: &label,
                    visibility: statement.row.visibility.as_str(),
                };
                self.extract_class_member(
                    statement.args(),
                    statement.arg_tokens(),
                    ctx,
                    seed_vars,
                    &form,
                );
            }
            _ => {}
        }
    }
}
