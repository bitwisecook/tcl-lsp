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

//! The replacement for one call site: the expansion, the renamed body and the local names it writes.

use super::parameter_bindings::build_param_bindings;
use super::return_routing::{
    Stored, capture_implicit_return_value, has_nested_irreturn, wrap_with_irreturn_loop,
};
use super::rewrite::{Site, Tail};
use super::{InlineSpec, MAX_INLINING_WALK_DEPTH, heads, rename};
use std::collections::{HashMap, HashSet};
use tcl_compiler::ir::{CommandBindingSite, Procedure, Script, Statement};
use tcl_lexer::{LexerConfig, Span};
use tcl_syntax::word_rules::WordValueRules;

// per-call-site splice

/// One inlined call-site replacement plus the structured-command dependencies
/// which move into the caller script with it.
pub(super) struct InlineExpansion {
    pub(super) statements: Vec<Statement>,
    pub(super) command_binding_sites: Vec<CommandBindingSite>,
    pub(super) procedure_bindings: Vec<tcl_runtime_api::ProcedureBindingIdentity>,
}

impl InlineExpansion {
    pub(super) fn without_bindings(statements: Vec<Statement>) -> Self {
        Self {
            statements,
            command_binding_sites: Vec::new(),
            procedure_bindings: Vec::new(),
        }
    }

    fn from_script(script: Script) -> Self {
        Self {
            statements: script.statements,
            command_binding_sites: script.command_binding_sites.iter().cloned().collect(),
            procedure_bindings: script
                .procedure_binding_requirements
                .iter()
                .cloned()
                .collect(),
        }
    }
}

/// The body of a parameterised definition with a site's renames applied, where
/// the splice cannot rename the IR alone: `None` keeps the call.
pub(super) type RenamedBody<'a> = dyn Fn(&HashMap<String, String>) -> Option<Script> + 'a;

/// Produce the inlined statements and their script-owned metadata for a single
/// call site, or `None` to decline (keep the call).
///
/// `resolution_namespace` is the namespace the call resolved its command in,
/// and the one the spliced statements now run in.
pub(super) fn splice_call_site(
    call: &Statement,
    spec: &InlineSpec,
    resolution_namespace: &str,
    counter: &mut usize,
    site: Site,
    live: bool,
    renamed: Option<&RenamedBody<'_>>,
) -> Option<InlineExpansion> {
    let Statement::Call {
        command,
        args,
        span,
        tokens,
        ..
    } = call
    else {
        return None;
    };
    let span = *span;
    let definition = spec.definition();
    let defined_in = definition.namespace.clone();
    let foreign = resolution_namespace != defined_in;
    // A definition in a namespace of its own resolved each command it names there
    // first and in the global namespace after it. No spelling says that to a
    // caller in another namespace, so such a body is spliced only where it names
    // no command the IR keeps.
    if foreign
        && defined_in != "::"
        && heads::names_a_command(spec.body_statements(), spec.lexer_config())
    {
        return None;
    }
    // A live compile lowered the body under the caller's own entry, which
    // resolves its commands where the caller runs.
    if foreign && live {
        return None;
    }
    let mut expansion = match spec {
        InlineSpec::Empty(_, _) => {
            if !args.is_empty() {
                return None;
            }
            // An empty body's value is the empty string. Where the value is
            // read, the call can vanish only when nothing stands before it to
            // be mistaken for it: `set a 5; noop` is "", and `set a 5` is not.
            if site.tail != Tail::Dropped && !site.first {
                return None;
            }
            Some(InlineExpansion::without_bindings(Vec::new()))
        }
        InlineSpec::Verbatim(body, _, _) => {
            if !args.is_empty() {
                return None;
            }
            // Re-attribute each spliced call to the call site for diagnostics.
            let statements = body
                .statements
                .iter()
                .map(|inner| match inner {
                    Statement::Call { .. } => with_span(inner, span),
                    other => other.clone(),
                })
                .collect();
            Some(InlineExpansion {
                statements,
                command_binding_sites: body.command_binding_sites.iter().cloned().collect(),
                procedure_bindings: body
                    .procedure_binding_requirements
                    .iter()
                    .cloned()
                    .collect(),
            })
        }
        InlineSpec::Parameterised(proc, rules, config) => {
            splice_v3(call, proc, *rules, *config, counter, site.tail, renamed)
        }
    }?;
    // A definition in the global namespace resolved each command it names there. A
    // caller in another namespace would resolve the same spelling from its own
    // first, and a command of that name defined there would answer in its place.
    if foreign && defined_in == "::" {
        expansion.statements = heads::root(
            std::mem::take(&mut expansion.statements),
            spec.lexer_config(),
        )?;
    }
    expansion.procedure_bindings.push(
        tcl_runtime_api::ProcedureBindingIdentity::in_rooted_namespace(
            resolution_namespace,
            command,
            definition.name,
            definition.parameters,
            definition.body,
        )
        .with_namespace_context(
            tokens
                .as_ref()
                .and_then(tcl_compiler::registry_invocation::compiled_namespace_context),
        ),
    );
    Some(expansion)
}

/// v3 splice — build parameter bindings, α-rename the body, and handle
/// the `return`-disposition strategy.
fn splice_v3(
    call: &Statement,
    proc: &Procedure,
    rules: WordValueRules,
    config: LexerConfig,
    counter: &mut usize,
    tail: Tail,
    renamed: Option<&RenamedBody<'_>>,
) -> Option<InlineExpansion> {
    let span = call.span();
    let body_stmts = &proc.body.statements;
    let body_has_trailing_return = body_stmts
        .last()
        .is_some_and(|s| matches!(s, Statement::Return { .. }));
    let body_has_non_trailing_return = body_stmts
        .iter()
        .rev()
        .skip(1)
        .any(|s| matches!(s, Statement::Return { .. }))
        || has_nested_irreturn(&proc.body);

    // A `catch` or a `try` reads the value of the call and a `return` in its
    // place would not end the procedure, so only a body with no `return` — whose
    // last command's value is the body's — can stand there.
    if tail == Tail::Captured && (body_has_trailing_return || body_has_non_trailing_return) {
        return None;
    }

    // A live splice evaluates the call's words once, into anonymous slots,
    // before anything of the body runs, and binds the formals from them: the
    // runtime invokes those very words if the splice has to be abandoned.
    let (words, replaced) = match renamed {
        Some(_) => {
            let (words, replaced) = evaluated_words(call, *counter + 1)?;
            (words, Some(replaced))
        }
        None => (Vec::new(), None),
    };
    let (cid, mut rename, bindings) = build_param_bindings(
        replaced.as_ref().unwrap_or(call),
        proc,
        rules,
        config,
        counter,
    )?;

    // Mangle every locally-written variable too.
    for name in collect_local_names(&proc.body) {
        rename
            .entry(name.clone())
            .or_insert_with(|| format!("{}{name}", super::local_prefix(cid)));
    }

    let renamed_body = match renamed {
        Some(renamed) => renamed(&rename)?,
        None => rename::rewrite_script(&proc.body, &rename, config),
    };

    let mut out = words;
    out.extend(bindings);
    if body_has_non_trailing_return {
        // Capture the implicit-trailing-return value when the body falls
        // off the end without an explicit `return`.
        let mut implicit: Option<Stored> = None;
        if !body_has_trailing_return && let Some(trailing) = renamed_body.statements.last() {
            implicit = Some(capture_implicit_return_value(trailing)?);
        }
        let result_var = format!("{}RESULT", super::local_prefix(cid));
        let mut wrapped = wrap_with_irreturn_loop(&renamed_body, &result_var, span, implicit);
        if tail == Tail::Returned {
            // The value is the result's, which is read as the lowering spells a
            // whole-word variable reference: the emitter substitutes `${name}`
            // and pushes `$name` as the text it is.
            wrapped.push(Statement::Return {
                expr_base: None,
                tokens: None,
                span,
                value: Some(format!("${{{result_var}}}")),
                value_word: None,
                expr: None,
                command_binding: None,
                braced: false,
            });
        }
        out.extend(wrapped);
        return Some(wrapped_expansion(out, &renamed_body, renamed.is_some()));
    }

    if body_has_trailing_return && tail != Tail::Returned {
        // Trailing return where the value is not the procedure's — wrap so it
        // doesn't short-circuit the rest of the caller's body.
        let result_var = format!("{}RESULT", super::local_prefix(cid));
        out.extend(wrap_with_irreturn_loop(
            &renamed_body,
            &result_var,
            span,
            None,
        ));
        return Some(wrapped_expansion(out, &renamed_body, renamed.is_some()));
    }

    // No return, or a trailing return where the value is the procedure's — flat
    // splice.
    let expansion = InlineExpansion::from_script(renamed_body);
    out.extend(expansion.statements);
    Some(InlineExpansion {
        statements: out,
        command_binding_sites: expansion.command_binding_sites,
        procedure_bindings: expansion.procedure_bindings,
    })
}

/// The name of the anonymous slot that holds word `index` of splice `cid`'s
/// call, its command word first.
pub(super) fn word_slot(cid: usize, index: usize) -> String {
    format!("{}w{index}", super::local_prefix(cid))
}

/// The statements that evaluate `call`'s words, once and in order, into the
/// slots of splice `cid`, and the call with each argument read back from its
/// slot. `None` for a call whose command word is not a literal, or that
/// expands a word.
fn evaluated_words(call: &Statement, cid: usize) -> Option<(Vec<Statement>, Statement)> {
    let Statement::Call {
        span,
        command,
        canonical_command,
        args,
        defs,
        reads,
        reads_own_defs,
        safe_on_uninit,
        tokens,
        foreach_groups,
    } = call
    else {
        return None;
    };
    let tokens = tokens.as_ref()?;
    if tokens
        .expand_word
        .as_ref()
        .is_some_and(|expand| expand.iter().any(|&word| word))
    {
        return None;
    }
    let tcl_compiler::ir::WordExpr::Literal { text: head, .. } = tokens.words().first()? else {
        return None;
    };
    let mut words = vec![Statement::AssignConst {
        span: *span,
        name: word_slot(cid, 0),
        name_braced: false,
        value: head.clone(),
        value_span: None,
    }];
    let mut read_back = Vec::with_capacity(args.len());
    for (index, arg) in args.iter().enumerate() {
        let name = word_slot(cid, index + 1);
        words.push(if tokens.arg_is_braced_literal(index) {
            Statement::AssignConst {
                span: *span,
                name: name.clone(),
                name_braced: false,
                value: arg.clone(),
                value_span: None,
            }
        } else {
            Statement::AssignValue {
                span: *span,
                name: name.clone(),
                name_braced: false,
                value: arg.clone(),
                value_needs_backsubst: arg.contains('\\'),
                tokens: None,
            }
        });
        read_back.push(format!("${{{name}}}"));
    }
    let replaced = Statement::Call {
        span: *span,
        command: command.clone(),
        canonical_command: canonical_command.clone(),
        args: read_back,
        defs: defs.clone(),
        reads: reads.clone(),
        reads_own_defs: *reads_own_defs,
        safe_on_uninit: *safe_on_uninit,
        tokens: None,
        foreach_groups: foreach_groups.clone(),
    };
    Some((words, replaced))
}

/// The statements of a wrapped body. A body lowered under a live entry keeps
/// its binding sites: the commands its typed operations were folded from are
/// held to the live command through them.
fn wrapped_expansion(statements: Vec<Statement>, body: &Script, live: bool) -> InlineExpansion {
    if !live {
        return InlineExpansion::without_bindings(statements);
    }
    let mut expansion = InlineExpansion::from_script(body.clone());
    expansion.statements = statements;
    expansion
}

/// Clone `stmt` (a [`Statement::Call`]) with its span replaced. Used to
/// re-attribute spliced statements to the call site.
fn with_span(stmt: &Statement, span: Span) -> Statement {
    match stmt {
        Statement::Call {
            command,
            canonical_command,
            args,
            defs,
            reads,
            reads_own_defs,
            safe_on_uninit,
            tokens,
            foreach_groups,
            ..
        } => Statement::Call {
            span,
            command: command.clone(),
            canonical_command: canonical_command.clone(),
            args: args.clone(),
            defs: defs.clone(),
            reads: reads.clone(),
            reads_own_defs: *reads_own_defs,
            safe_on_uninit: *safe_on_uninit,
            tokens: tokens.clone(),
            foreach_groups: foreach_groups.clone(),
        },
        other => other.clone(),
    }
}

// local-name collection

/// Return the union of every local-variable name written inside `script`.
/// Array-element writes contribute the array base name.
pub(super) fn collect_local_names(script: &Script) -> HashSet<String> {
    let mut names = HashSet::new();
    walk_local_writes(script, &mut names, 0);
    names
}

pub(super) fn record_local_base(name: &str, names: &mut HashSet<String>) {
    if tcl_syntax::naming::is_qualified(name.as_bytes()) {
        return;
    }
    let base = tcl_syntax::naming::split_element_ref(name).map_or(name, |(base, _)| base);
    if !base.is_empty() {
        names.insert(base.to_owned());
    }
}

/// `depth` is the nesting level of `script` — see
/// [`MAX_INLINING_WALK_DEPTH`].
pub(super) fn walk_local_writes(script: &Script, names: &mut HashSet<String>, depth: u32) {
    if MAX_INLINING_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => record_local_base(name, names),
            Statement::Call { defs, .. } => {
                for d in defs {
                    record_local_base(d, names);
                }
            }
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                walk_local_writes(body, names, depth + 1);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_local_writes(&c.body, names, depth + 1);
                }
                if let Some(b) = else_body {
                    walk_local_writes(b, names, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_local_writes(init, names, depth + 1);
                walk_local_writes(next, names, depth + 1);
                walk_local_writes(body, names, depth + 1);
            }
            Statement::While { body, .. } => walk_local_writes(body, names, depth + 1),
            Statement::Foreach {
                iterators, body, ..
            } => {
                for it in iterators {
                    for v in &it.vars {
                        record_local_base(v, names);
                    }
                }
                walk_local_writes(body, names, depth + 1);
            }
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                walk_local_writes(body, names, depth + 1);
                if let Some(v) = result_var {
                    names.insert(v.clone());
                }
                if let Some(v) = options_var {
                    names.insert(v.clone());
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_local_writes(body, names, depth + 1);
                for h in handlers {
                    walk_local_writes(&h.body, names, depth + 1);
                    if let Some(v) = &h.var_name {
                        names.insert(v.clone());
                    }
                    if let Some(v) = &h.options_var {
                        names.insert(v.clone());
                    }
                }
                if let Some(fb) = finally_body {
                    walk_local_writes(fb, names, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_local_writes(b, names, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_local_writes(b, names, depth + 1);
                }
            }
            _ => {}
        }
    }
}
