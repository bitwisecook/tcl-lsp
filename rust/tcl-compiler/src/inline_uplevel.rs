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

//! Whole-callee uplevel-passthrough inlining.
//!
//! Detects procs whose entire body is a single
//! [`Statement::UpFrame`] with a *relative* `frame_shift == 1` (the
//! canonical "shift to caller's frame, evaluate body, restore frame" idiom)
//! and rewrites every callsite to splice the body inline.

use std::collections::HashMap;

use crate::ir::{CommandTokens, Module, Procedure, Script, Statement};
use crate::lowering::Lowerer;
use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};
use tcl_registry::{CommandRegistry, FRAME_REACH_TRAITS};

/// Recognised passthrough proc shape used by the rewriter.
#[derive(Debug, Clone)]
pub enum PassthroughShape {
    /// Zero-param proc whose body is exactly one
    /// `Statement::UpFrame { frame_shift: 1, absolute: false, body, .. }`
    /// — splice `body` directly at every callsite.
    Static {
        /// Pre-lowered body to inline at every callsite.
        body: Script,
    },
    /// Single-param proc whose body is a runtime ``uplevel 1 $param``
    /// call. The rewriter handles this per callsite by parsing the
    /// callsite's brace-literal argument; this enum carries no body
    /// because the body lives at the callsite.
    ParamBody {
        /// Name of the single proc parameter — sanity-checked at
        /// rewrite time (the callsite's literal is what gets inlined,
        /// not the param name).
        param_name: String,
        /// Actual selector operands from the dispatcher before its body.
        selector: Vec<String>,
        /// Authored frame grammar, revalidated with every concrete body.
        frame: tcl_registry::FrameEffectSpec,
    },
}

/// Walk every procedure in *module* and classify it as a static
/// passthrough candidate, a param-body passthrough candidate, or
/// not a passthrough at all.
///
/// Returns `{qualified_name -> shape}` for every passthrough proc.
#[must_use]
pub fn detect_passthrough_candidates(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<String, PassthroughShape> {
    let mut out = HashMap::new();
    for (qname, proc) in &module.procedures {
        if let Some(shape) =
            classify_passthrough(proc, registry, module.source_entry.invocation_dialect)
        {
            out.insert(qname.clone(), shape);
        }
    }
    out
}

/// Convenience: only the static (zero-param) shape, used by tests.
#[must_use]
pub fn detect_static_passthrough(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<String, Script> {
    detect_passthrough_candidates(module, registry)
        .into_iter()
        .filter_map(|(k, v)| match v {
            PassthroughShape::Static { body } => Some((k, body)),
            PassthroughShape::ParamBody { .. } => None,
        })
        .collect()
}

/// Classify a single procedure as a passthrough candidate or
/// return `None`.
fn classify_passthrough(
    proc: &Procedure,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<PassthroughShape> {
    if let Some(body) = static_passthrough_body(proc, registry) {
        return Some(PassthroughShape::Static { body });
    }
    if let Some((param_name, selector, frame)) =
        param_body_passthrough_param(proc, registry, dialect)
    {
        return Some(PassthroughShape::ParamBody {
            param_name,
            selector,
            frame,
        });
    }
    None
}

/// Return the inner body if *proc* is a zero-param `uplevel 1`
/// passthrough — body is exactly one [`Statement::UpFrame`] with a
/// *relative* `frame_shift == 1` and no nested frame-reaching
/// commands. The absolute `uplevel #1` names a frame counted down
/// from the global one, so it is not the same-frame idiom and is
/// rejected alongside `#0`.
fn static_passthrough_body(proc: &Procedure, registry: &CommandRegistry) -> Option<Script> {
    if !proc.params.is_empty() {
        return None;
    }
    if proc.body.statements.len() != 1 {
        return None;
    }
    match &proc.body.statements[0] {
        Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            ..
        } if !*absolute && *frame_shift == 1 => {
            if body_has_frame_reach(body, registry) || body_has_completion_escape(body) {
                None
            } else {
                Some(body.clone())
            }
        }
        _ => None,
    }
}

/// Return the param name if *proc* matches
/// `proc NAME {P} { uplevel ?1? $P }` — the single-body-param
/// passthrough shape. Recognises both the bare `uplevel
/// $body` (implicit level 1) form lowered as a `Statement::Call` /
/// `Statement::Barrier` for the outer dispatcher (since the body
/// token is `$var`, the lowering can't relax it to
/// [`Statement::UpFrame`]).
///
/// The actual frame-reach check still runs on the *callsite's*
/// inlined body inside the rewriter — at detector time we only
/// confirm the dispatcher's surface shape.
fn param_body_passthrough_param(
    proc: &Procedure,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<(String, Vec<String>, tcl_registry::FrameEffectSpec)> {
    if proc.params.len() != 1 {
        return None;
    }
    let param = &proc.params[0];
    if proc.body.statements.len() != 1 {
        return None;
    }
    let stmt = &proc.body.statements[0];

    // Two surface shapes both lower to a runtime call: ``uplevel
    // $body`` becomes ``Statement::Call`` (or ``Barrier`` when the
    // default lowering treats it as opaque); the explicit
    // ``uplevel 1 $body`` form goes through the same dispatch.
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return None;
    };

    // The dispatcher must be a command whose registry frame-effect
    // grammar runs a script in the frame the level word selects, and
    // the level word must resolve to the immediate caller. Reading the
    // grammar rather than the spelling covers every spelling of the
    // command (``::uplevel``) and every spelling of the level word
    // (``1``, ``+1``, ``0x1``, omitted) without enumerating either.
    let tokens = stmt.tokens()?;
    let target = tokens.source_binding.as_ref()?.proved_execution_target()?;
    if !target.registry_backed {
        return None;
    }
    let spec = registry.get(&target.command)?;
    let frame_effect = spec.frame_effect?;
    if frame_effect.layout != FrameArgLayout::ScriptInSelectedFrame {
        return None;
    }
    let source_words = tokens.words();
    let mut words: Vec<_> = target
        .prepended
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect();
    words.extend(
        source_words
            .iter()
            .skip(1)
            .map(crate::registry_invocation::invocation_word),
    );
    let mut arguments =
        tcl_registry::InvocationArguments::Structured(&words).with_profile(registry.profile());
    if let Some(dialect) = dialect {
        arguments = arguments.with_dialect(dialect);
    }
    let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
        level,
        level_word_len,
    } = frame_effect.successful_layout(arguments).layout
    else {
        return None;
    };
    // A dispatcher body must originate from its actual parameter source word;
    // a literal body inserted by an alias is a different procedure shape.
    if words.len() != level_word_len + 1 {
        return None;
    }
    let source_body_index = level_word_len.checked_sub(target.prepended.len())?;
    let body_arg = args.get(source_body_index)?;
    // Only the immediate caller's frame is the passthrough idiom;
    // a deeper shift, an absolute frame, or a runtime-computed level
    // can't be inlined the same way.
    if level != FrameLevel::Relative(1) {
        return None;
    }
    // Body word must be a pure ``$param`` reference to the sole
    // proc parameter.
    let referenced = body_arg.strip_prefix('$')?;
    let referenced = referenced
        .strip_prefix('{')
        .map_or(referenced, |s| s.strip_suffix('}').unwrap_or(referenced));
    if referenced != *param {
        return None;
    }
    let selector = words[..level_word_len]
        .iter()
        .map(|word| word.literal().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    Some((param.clone(), selector, frame_effect))
}

/// True if *script* contains a command that reaches a stack frame other
/// than the one it is written in, and would therefore change meaning
/// once the script is spliced into the caller.
///
/// After inlining the body runs in the caller's frame; if the body
/// itself does `uplevel 1 {...}`, that now references the caller's
/// *caller* — a frame the original author may not have anticipated.
/// Reject conservatively.
///
/// Which commands those are comes from the registry — a spec's
/// [`frame_effect`](tcl_registry::CommandSpec::frame_effect) grammar or any
/// of [`FRAME_REACH_TRAITS`], composed over the resolved subcommand — so
/// `argparse`, `tailcall`, `eval`, and `info level` are covered alongside
/// `uplevel` / `upvar` without this pass naming a command.
#[must_use]
pub fn body_has_frame_reach(script: &Script, registry: &CommandRegistry) -> bool {
    script
        .statements
        .iter()
        .any(|stmt| statement_has_frame_reach(stmt, registry))
}

fn statement_has_frame_reach(stmt: &Statement, registry: &CommandRegistry) -> bool {
    let reaches = |script: &Script| body_has_frame_reach(script, registry);
    match stmt {
        Statement::UpFrame { .. } => true,
        Statement::Barrier { .. } | Statement::Call { .. } => {
            invocation_reaches_frame(stmt, registry)
        }
        Statement::If {
            clauses, else_body, ..
        } => clauses.iter().any(|c| reaches(&c.body)) || else_body.as_ref().is_some_and(reaches),
        Statement::For {
            init, next, body, ..
        } => reaches(init) || reaches(next) || reaches(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. } => reaches(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            reaches(body)
                || handlers.iter().any(|h| reaches(&h.body))
                || finally_body.as_ref().is_some_and(reaches)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().any(|a| a.body.as_ref().is_some_and(reaches))
                || default_body.as_ref().is_some_and(reaches)
        }
        _ => false,
    }
}

/// The registry's answer for one `Call` / `Barrier`: does this invocation
/// reach a frame other than the one it is written in?
///
/// The retained source proof selects the actual registry implementation and
/// composes frozen alias argv before asking its structured trait descriptor.
/// Traits such as `info level` and `info frame` belong to the selected
/// subcommand. Missing implementation, frame, or argument evidence declines
/// inlining; written command spelling does not supply execution proof.
fn invocation_reaches_frame(stmt: &Statement, registry: &CommandRegistry) -> bool {
    let Some(tokens) = stmt.tokens() else {
        return true;
    };
    let Some(target) = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::proved_target)
    else {
        return true;
    };
    if !target.registry_backed {
        return true;
    }
    let Some(spec) = registry.get(&target.command) else {
        return true;
    };
    if spec.frame_effect.is_some() {
        return true;
    }
    let source_words = tokens.words();
    let mut arguments: Vec<_> = target
        .prepended
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect();
    arguments.extend(
        source_words
            .iter()
            .skip(1)
            .map(crate::registry_invocation::invocation_word),
    );
    let invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        &arguments,
    )
    .with_profile(registry.profile());
    registry
        .resolve_structured_invocation(invocation, registry.own_surface_query())
        .resolved()
        .is_none_or(|resolved| resolved.facts().traits.intersects(FRAME_REACH_TRAITS))
}

/// True if *script* can complete with a `return` / `break` / `continue`
/// that escapes the script's own top level.
///
/// The passthrough proc we are about to erase is
/// `proc P {…} { uplevel 1 $body }`: `uplevel` is transparent to every
/// completion code, so the body's code flows to *P*'s proc boundary,
/// which (a) decrements a `return`'s level — absorbing `return 5` so the
/// caller carries on — and (b) turns a raw `break`/`continue` into an
/// `invoked "…" outside of a loop` error. Splicing the body directly
/// into the caller removes that boundary: a spliced `return` now returns
/// the *caller's* proc, and a spliced `break`/`continue` now drives the
/// caller's enclosing loop instead of erroring. Both change observable
/// behaviour, so decline the inline when the body can escape this way.
///
/// `in_loop` tracks whether a loop *within the body* would absorb a bare
/// `break`/`continue`; `catch` absorbs every non-`OK` code, so its body
/// can never contribute an escape.
#[must_use]
pub fn body_has_completion_escape(script: &Script) -> bool {
    script
        .statements
        .iter()
        .any(|s| statement_has_completion_escape(s, false))
}

fn statement_has_completion_escape(stmt: &Statement, in_loop: bool) -> bool {
    match stmt {
        // `return` propagates to the proc boundary regardless of any
        // enclosing loop, so it always escapes the body.
        Statement::Return { .. } => true,
        // A bare `break`/`continue` escapes unless a loop *inside the
        // body* absorbs it.
        Statement::Call { command, .. } | Statement::Barrier { command, .. }
            if matches!(command.as_str(), "break" | "continue") =>
        {
            !in_loop
        }
        // Loop bodies absorb `break`/`continue`; their init/next scripts
        // (for `for`) run in the enclosing context and do not.
        Statement::For {
            init, next, body, ..
        } => statement_scripts_escape(&[init, next], in_loop) || body_iter_escape(body, true),
        Statement::While { body, .. } | Statement::Foreach { body, .. } => {
            body_iter_escape(body, true)
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|c| body_iter_escape(&c.body, in_loop))
                || else_body
                    .as_ref()
                    .is_some_and(|b| body_iter_escape(b, in_loop))
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().any(|a| {
                a.body
                    .as_ref()
                    .is_some_and(|b| body_iter_escape(b, in_loop))
            }) || default_body
                .as_ref()
                .is_some_and(|b| body_iter_escape(b, in_loop))
        }
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
            body_iter_escape(body, in_loop)
        }
        // `try` may re-raise a `return`/`break`/`continue` from its body,
        // handlers, or finally clause; conservatively treat any as an escape.
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            body_iter_escape(body, in_loop)
                || handlers.iter().any(|h| body_iter_escape(&h.body, in_loop))
                || finally_body
                    .as_ref()
                    .is_some_and(|b| body_iter_escape(b, in_loop))
        }
        // Everything else — including `catch`, which intercepts every non-`OK`
        // completion code and turns it into a value so nothing inside it can
        // escape — contributes no escape. `catch` is deliberately NOT recursed
        // into for that reason.
        _ => false,
    }
}

fn body_iter_escape(script: &Script, in_loop: bool) -> bool {
    script
        .statements
        .iter()
        .any(|s| statement_has_completion_escape(s, in_loop))
}

fn statement_scripts_escape(scripts: &[&Script], in_loop: bool) -> bool {
    scripts.iter().any(|s| body_iter_escape(s, in_loop))
}

/// Rewrite every passthrough callsite in *module* to splice the
/// callee's body inline. Mutates the module in place.
///
/// Detects passthrough candidates via
/// [`detect_passthrough_candidates`] and walks every script
/// (top-level + each procedure body) replacing matching
/// [`Statement::Call`] / [`Statement::Barrier`] nodes with
/// [`Statement::Block`] (static shape) or with the inlined
/// callsite-body literal (param-body shape).
///
/// Safe to call multiple times — already-inlined callsites no
/// longer match the pattern.
pub fn inline_uplevel_passthrough(module: &mut Module, registry: &CommandRegistry) {
    let source_image = module.source.clone();
    let Ok(source) = source_image.try_text() else {
        return;
    };
    let candidates = detect_passthrough_candidates(module, registry);
    if candidates.is_empty() {
        return;
    }
    let generations: HashMap<_, _> = module
        .procedures
        .iter()
        .map(|(name, proc)| (name.clone(), proc.span.start()))
        .collect();
    let config = module.lexer_config;
    let bindings = crate::command_binding::SourceCommandBindings::analyse_in_namespace_with_options(
        source,
        &module.top_level_namespace,
        config,
        registry,
        module.source_entry.options(),
    );
    let context = InlineContext {
        candidates: &candidates,
        generations: &generations,
        registry,
        bindings: &bindings,
        config,
        entry: &module.source_entry,
        source,
    };
    let mut top = std::mem::take(&mut module.top_level);
    rewrite_script_in_place(&mut top, "::", &context);
    module.top_level = top;
    let proc_qnames: Vec<String> = module.procedures.keys().cloned().collect();
    for qname in proc_qnames {
        let caller_ns = namespace_of(&qname);
        if let Some(proc) = module.procedures.get_mut(&qname) {
            let mut body = std::mem::take(&mut proc.body);
            rewrite_script_in_place(&mut body, &caller_ns, &context);
            proc.body = body;
        }
    }
}

fn namespace_of(qname: &str) -> String {
    if let Some(idx) = qname.rfind("::") {
        if idx == 0 {
            return "::".to_string();
        }
        let prefix = &qname[..idx];
        return if prefix.starts_with("::") {
            prefix.to_string()
        } else {
            format!("::{prefix}")
        };
    }
    "::".to_string()
}

struct InlineContext<'a> {
    candidates: &'a HashMap<String, PassthroughShape>,
    generations: &'a HashMap<String, u32>,
    registry: &'a CommandRegistry,
    bindings: &'a crate::command_binding::SourceCommandBindings,
    config: tcl_lexer::LexerConfig,
    entry: &'a crate::command_binding::SourceAnalysisEntry,
    source: &'a str,
}

fn rewrite_script_in_place(script: &mut Script, namespace: &str, context: &InlineContext<'_>) {
    for stmt in &mut script.statements {
        rewrite_statement_in_place(stmt, namespace, context);
    }
}

fn rewrite_statement_in_place(stmt: &mut Statement, namespace: &str, context: &InlineContext<'_>) {
    walk_nested_scripts(
        stmt,
        |body, ns| rewrite_script_in_place(body, ns, context),
        namespace,
    );
    if let Some(replacement) = try_inline_callsite(stmt, namespace, context) {
        *stmt = replacement;
    }
}

fn try_inline_callsite(
    stmt: &Statement,
    namespace: &str,
    context: &InlineContext<'_>,
) -> Option<Statement> {
    let InlineContext {
        candidates,
        generations,
        registry,
        bindings,
        config,
        entry,
        source,
    } = context;
    let config = *config;
    let (_command, args, span, tokens) = match stmt {
        Statement::Call {
            command,
            args,
            span,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            span,
            tokens,
            ..
        } if !command.is_empty() => (command.as_str(), args.as_slice(), *span, tokens),
        _ => return None,
    };

    let target = tokens
        .as_ref()?
        .source_binding
        .as_ref()?
        .proved_execution_target()?;
    if target.kind != crate::command_binding::BindingKind::Proc {
        return None;
    }
    let identity = target.identity.as_ref()?;
    let shape = candidates.get(&identity.origin)?;
    let declaration = generations.get(&identity.origin).copied()?;
    if !target.matches_authored_implementation(source, declaration) {
        return None;
    }
    // Retain effective argv: an alias prefix counts towards procedure arity.
    if !target.prepended.is_empty() {
        return None;
    }

    match shape {
        PassthroughShape::Static { body } => {
            if !args.is_empty() {
                return None;
            }
            Some(Statement::Block {
                span,
                body: body.clone(),
                namespace: namespace.to_string(),
                tokens: tokens.clone(),
                error_context: None,
            })
        }
        PassthroughShape::ParamBody {
            selector, frame, ..
        } => {
            // ParamBody: the dispatcher proc is `proc D {body}
            // { uplevel ?1? $body }`. Rewrite a callsite when:
            //   * exactly one argument,
            //   * that argument is a single brace-string token
            //     (`TokenType::Str`) — i.e. the source wrote
            //     ``D {literal-body}``,
            //   * no `{*}`-expansion on any word,
            //   * the materialised body lowers cleanly and
            //     contains no nested frame-reaching commands.
            let tk = tokens.as_ref()?;
            let (literal, base) = literal_body_argument(args, tk)?;
            if !concrete_body_selects_caller(*frame, selector, literal, entry.invocation_dialect) {
                return None;
            }
            let call_proof = tk.source_binding.as_ref()?;
            let selected = bindings.analyse_script_at_site(
                literal,
                base,
                span.start(),
                &call_proof.variable_frame,
                config,
                registry,
            )?;
            let mut lowerer = Lowerer::with_config(registry, config);
            lowerer.set_source_analysis_options(entry.options());
            let inlined =
                lowerer.lower_into_script_with_bindings(literal, base, namespace, selected);
            if body_has_frame_reach(&inlined, registry) || body_has_completion_escape(&inlined) {
                return None;
            }
            Some(Statement::Block {
                span,
                body: inlined,
                namespace: namespace.to_string(),
                tokens: tokens.clone(),
                error_context: None,
            })
        }
    }
}

fn literal_body_argument<'a>(args: &'a [String], tokens: &CommandTokens) -> Option<(&'a str, u32)> {
    let [literal] = args else {
        return None;
    };
    if tokens.argv_kinds.get(1) != Some(&tcl_lexer::TokenType::Str)
        || tokens
            .expand_word
            .iter()
            .flatten()
            .any(|&expanded| expanded)
    {
        return None;
    }
    Some((literal, tokens.argv.get(1)?.start().checked_add(1)?))
}

fn concrete_body_selects_caller(
    frame: tcl_registry::FrameEffectSpec,
    selector: &[String],
    literal: &str,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    let mut concrete: Vec<_> = selector
        .iter()
        .map(|word| tcl_registry::InvocationWord::Literal(word))
        .collect();
    concrete.push(tcl_registry::InvocationWord::Literal(literal));
    let mut arguments = tcl_registry::InvocationArguments::structured(&concrete);
    if let Some(dialect) = dialect {
        arguments = arguments.with_dialect(dialect);
    }
    matches!(frame.resolve_arguments(arguments),
        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
            level: FrameLevel::Relative(1), level_word_len,
        } if level_word_len == selector.len())
}

/// Visit every nested [`Script`] field of *stmt* with *visitor*
/// (mutable). Used by the rewriter to recurse into structured
/// statements without enumerating every variant's fields at the
/// call site.
fn walk_nested_scripts<F>(stmt: &mut Statement, mut visitor: F, namespace: &str)
where
    F: FnMut(&mut Script, &str),
{
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses.iter_mut() {
                visitor(&mut c.body, namespace);
            }
            if let Some(b) = else_body.as_mut() {
                visitor(b, namespace);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            visitor(init, namespace);
            visitor(next, namespace);
            visitor(body, namespace);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::UpFrame { body, .. } => {
            visitor(body, namespace);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            visitor(body, namespace);
            for h in handlers.iter_mut() {
                visitor(&mut h.body, namespace);
            }
            if let Some(f) = finally_body.as_mut() {
                visitor(f, namespace);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms.iter_mut() {
                if let Some(b) = a.body.as_mut() {
                    visitor(b, namespace);
                }
            }
            if let Some(d) = default_body.as_mut() {
                visitor(d, namespace);
            }
        }
        Statement::Block {
            body,
            namespace: ns,
            ..
        } => {
            // Block carries its own namespace; recurse with that.
            let inner_ns = ns.clone();
            visitor(body, &inner_ns);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir_with;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default().project_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.6").expect("C Tcl 8.6 profile"),
        )
    }

    fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
        let profile = registry.profile().expect("selected test interpreter");
        let mut lowerer =
            Lowerer::with_config(registry, tcl_lexer::LexerConfig::for_profile(Some(profile)));
        lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
            unknown_entry: false,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..crate::command_binding::SourceAnalysisOptions::default()
        });
        lower_to_ir_with(lowerer, source)
    }

    #[test]
    fn zero_param_static_passthrough_detected() {
        let m = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        let candidates = detect_static_passthrough(&m, &reg());
        assert_eq!(candidates.len(), 1);
        assert!(candidates.contains_key("::reset"));
    }

    #[test]
    fn zero_param_with_extra_statement_is_not_passthrough() {
        let m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0}\n puts done }",
            &reg(),
        );
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn proc_with_params_is_not_static_passthrough() {
        let m = lower_to_ir("proc reset {x} { uplevel 1 {set counter 0} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn nested_uplevel_blocks_static_passthrough() {
        let m = lower_to_ir(
            "proc reset {} { uplevel 1 {uplevel 1 {set counter 0}} }",
            &reg(),
        );
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn nested_upvar_call_blocks_static_passthrough() {
        let m = lower_to_ir("proc bind {} { uplevel 1 {upvar foo bar} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn frame_shift_zero_is_not_static_passthrough() {
        // ``uplevel #0`` shifts to absolute global frame — can't be
        // expressed as a same-frame inline.
        let m = lower_to_ir("proc reset {} { uplevel #0 {set counter 0} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn absolute_level_one_is_not_static_passthrough() {
        // `uplevel #1` counts *down* from the global frame, so it is not the
        // caller's-frame idiom `uplevel 1` denotes. Both carry the magnitude
        // 1; only the `absolute` flag separates them.
        let m = lower_to_ir("proc reset {} { uplevel #1 {set counter 0} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
        let relative = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        assert!(
            !detect_static_passthrough(&relative, &reg()).is_empty(),
            "the relative form is still recognised",
        );
    }

    #[test]
    fn static_passthrough_with_return_rejected() {
        // Splicing a `return` into the caller would return from the CALLER's
        // proc, since the erased passthrough boundary no longer absorbs it.
        let m = lower_to_ir("proc run {} { uplevel 1 {return 5} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn static_passthrough_with_bare_break_rejected() {
        let m = lower_to_ir("proc run {} { uplevel 1 {break} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn static_passthrough_with_bare_continue_rejected() {
        let m = lower_to_ir("proc run {} { uplevel 1 {continue} }", &reg());
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn static_passthrough_with_loop_absorbed_break_allowed() {
        // A `break` fully contained in a loop within the body is absorbed by
        // that loop and never escapes, so the inline stays safe.
        let m = lower_to_ir(
            "proc run {} { uplevel 1 {foreach x {1 2} { break }} }\nrun",
            &reg(),
        );
        assert_eq!(detect_static_passthrough(&m, &reg()).len(), 1);
    }

    #[test]
    fn static_passthrough_with_catch_absorbed_return_allowed() {
        // `catch` intercepts every non-OK completion code, so a `return`
        // inside it cannot escape the body.
        let m = lower_to_ir("proc run {} { uplevel 1 {catch {return 5}} }\nrun", &reg());
        assert_eq!(detect_static_passthrough(&m, &reg()).len(), 1);
    }

    #[test]
    fn static_passthrough_return_inside_loop_still_rejected() {
        // A loop absorbs break/continue but NOT return, so a return nested in
        // a loop still escapes to the proc boundary.
        let m = lower_to_ir(
            "proc run {} { uplevel 1 {foreach x {1 2} { return $x }} }",
            &reg(),
        );
        assert!(detect_static_passthrough(&m, &reg()).is_empty());
    }

    #[test]
    fn param_body_passthrough_return_callsite_not_inlined() {
        // The dispatcher shape is still a candidate, but a callsite whose
        // literal body escapes with `return` must not be spliced.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher { return 5 }",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn param_body_passthrough_plain_callsite_still_inlined() {
        // Control: a non-escaping literal body is inlined as before, so the
        // completion-escape gate hasn't disabled the optimisation wholesale.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher { set counter 0 }",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 1);
    }

    /// Inline `body` through `proc dispatcher {body} { uplevel 1 $body }` and
    /// report how many callsites collapsed to a spliced `Block`.
    fn dispatched_blocks(body: &str) -> usize {
        let src = format!("proc dispatcher {{body}} {{ uplevel 1 $body }}\ndispatcher {{{body}}}");
        let mut m = lower_to_ir(&src, &reg());
        inline_uplevel_passthrough(&mut m, &reg());
        count_blocks(&m.top_level)
    }

    #[test]
    fn param_body_callsite_calling_argparse_not_inlined() {
        // `argparse` injects locals into the frame of *its own* caller
        // (`FrameArgLayout::OpaqueCallerVars`), so splicing the body one frame
        // down redirects every injected name.
        assert_eq!(dispatched_blocks("argparse {a b}"), 0);
    }

    #[test]
    fn param_body_callsite_calling_tailcall_not_inlined() {
        // `tailcall` (`Traits::REPLACES_FRAME`) replaces the frame it runs in;
        // spliced into the caller it would replace the *caller's* frame.
        assert_eq!(dispatched_blocks("tailcall other"), 0);
    }

    #[test]
    fn param_body_callsite_calling_info_level_not_inlined() {
        // `info level` reads the current frame's depth and words
        // (`Traits::CURRENT_FRAME_INTROSPECTION`, carried on the *subcommand*
        // — a parent-only trait test misses it).
        assert_eq!(dispatched_blocks("info level"), 0);
    }

    #[test]
    fn param_body_callsite_calling_qualified_uplevel_not_inlined() {
        // The globally-qualified spelling is the same command; the old
        // two-name surface test only knew the bare `uplevel`. A `$`-body
        // keeps the call on the generic dispatch path, so the guard sees a
        // `Call` whose surface word is `::uplevel` rather than a lowered
        // `Statement::UpFrame`.
        assert_eq!(dispatched_blocks("::uplevel 1 $script"), 0);
        assert_eq!(dispatched_blocks("::upvar 1 outer inner"), 0);
    }

    #[test]
    fn param_body_callsite_with_benign_body_still_inlined() {
        // Control: a body that reaches no other frame still inlines, so the
        // widened gate has not disabled the optimisation wholesale.
        assert_eq!(dispatched_blocks("set counter 0"), 1);
    }

    #[test]
    fn param_body_level_word_is_read_as_a_frame_not_a_literal() {
        // The level word goes through the registry's frame-effect
        // resolution, so every spelling of "the immediate caller" is
        // recognised and nothing else is.
        for (src, expected) in [
            ("proc dispatcher {body} { uplevel 0x1 $body }", true),
            ("proc dispatcher {body} { uplevel +1 $body }", true),
            ("proc dispatcher {body} { uplevel 0 $body }", false),
            ("proc dispatcher {body} { uplevel #1 $body }", false),
            ("proc dispatcher {body} { uplevel #0 $body }", false),
            ("proc dispatcher {body} { uplevel $lvl $body }", false),
        ] {
            let m = lower_to_ir(src, &reg());
            let found = matches!(
                detect_passthrough_candidates(&m, &reg()).get("::dispatcher"),
                Some(PassthroughShape::ParamBody { .. })
            );
            assert_eq!(found, expected, "{src}");
        }
    }

    #[test]
    fn implicit_selector_candidate_revalidates_the_concrete_body() {
        let mut accepted = lower_to_ir(
            "proc dispatcher {body} {uplevel $body}\ndispatcher {set x 1}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut accepted, &reg());
        assert!(
            accepted
                .top_level
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::Block { .. }))
        );
        let mut rejected = lower_to_ir(
            "proc dispatcher {body} {uplevel $body}\ndispatcher {1}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut rejected, &reg());
        assert!(
            !rejected
                .top_level
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::Block { .. }))
        );
    }

    #[test]
    fn qualified_uplevel_dispatcher_is_still_a_candidate() {
        // The dispatcher's own head resolves through the registry too, so
        // `proc D {b} { ::uplevel 1 $b }` is the same passthrough shape.
        let m = lower_to_ir("proc dispatcher {body} { ::uplevel 1 $body }", &reg());
        assert!(matches!(
            detect_passthrough_candidates(&m, &reg()).get("::dispatcher"),
            Some(PassthroughShape::ParamBody { .. })
        ));
    }

    #[test]
    fn param_body_passthrough_detected() {
        let m = lower_to_ir("proc dispatcher {body} { uplevel 1 $body }", &reg());
        let candidates = detect_passthrough_candidates(&m, &reg());
        let shape = candidates.get("::dispatcher").expect("expected candidate");
        match shape {
            PassthroughShape::ParamBody { param_name, .. } => assert_eq!(param_name, "body"),
            PassthroughShape::Static { .. } => panic!("expected ParamBody, got Static"),
        }
    }

    #[test]
    fn param_body_passthrough_implicit_level_one() {
        // ``uplevel $body`` (no explicit level) defaults to 1 so
        // it matches the same shape.
        let m = lower_to_ir("proc dispatcher {body} { uplevel $body }", &reg());
        let candidates = detect_passthrough_candidates(&m, &reg());
        assert!(matches!(
            candidates.get("::dispatcher"),
            Some(PassthroughShape::ParamBody { .. })
        ));
    }

    #[test]
    fn param_body_passthrough_two_params_rejected() {
        let m = lower_to_ir("proc dispatcher {body extra} { uplevel 1 $body }", &reg());
        assert!(detect_passthrough_candidates(&m, &reg()).is_empty());
    }

    #[test]
    fn param_body_passthrough_wrong_param_rejected() {
        // ``$other`` isn't the proc's parameter — mismatch.
        let m = lower_to_ir("proc dispatcher {body} { uplevel 1 $other }", &reg());
        assert!(detect_passthrough_candidates(&m, &reg()).is_empty());
    }

    #[test]
    fn body_with_only_assignment_has_no_frame_reach() {
        let m = lower_to_ir("set x 1", &reg());
        assert!(!body_has_frame_reach(&m.top_level, &reg()));
    }

    fn count_blocks(script: &Script) -> usize {
        let mut n = 0;
        for stmt in &script.statements {
            if let Statement::Block { .. } = stmt {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn rewriter_inlines_zero_param_passthrough_callsite() {
        let mut m = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        inline_uplevel_passthrough(&mut m, &reg());
        // The callsite ``reset`` should now be a Statement::Block
        // splicing in the body.
        assert_eq!(count_blocks(&m.top_level), 1);
    }

    #[test]
    fn rewriter_requires_the_retained_authored_implementation_allocation() {
        use crate::command_binding::{AllocationIncarnation, SourceOriginId};
        use std::sync::Arc;

        let registry = reg();
        let original = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0} }\nreset",
            &registry,
        );
        for different_source in [false, true] {
            let mut module = original.clone();
            let (Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. }) =
                module.top_level.statements.last_mut().unwrap()
            else {
                panic!("expected procedure call");
            };
            let target = &mut tokens
                .as_mut()
                .unwrap()
                .source_binding
                .as_mut()
                .unwrap()
                .targets[0];
            let allocation = target.implementation_allocation.as_mut().unwrap();
            if different_source {
                allocation.site.source = Arc::new(SourceOriginId::authored(&Arc::from(
                    "a different source instance",
                )));
            } else {
                allocation.incarnation = AllocationIncarnation::RepeatedFresh;
            }
            inline_uplevel_passthrough(&mut module, &registry);
            assert_eq!(count_blocks(&module.top_level), 0);
        }
    }

    #[test]
    fn rewriter_skips_callsites_with_args() {
        // Static-shape candidates are zero-param; a call passing
        // an argument should not be rewritten (would change
        // semantics).
        let mut m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0} }\nreset extra",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_unknown_callees() {
        // Calling a non-passthrough proc shouldn't be touched.
        let mut m = lower_to_ir("proc helper {} { puts hi }\nhelper", &reg());
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_recurses_into_if_body() {
        let mut m = lower_to_ir(
            "proc reset {} { uplevel 1 {set counter 0} }\nif {1} { reset }",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        // ``proc`` itself emits a ``Call`` at top level (registers
        // the proc but also keeps a ``proc`` invocation statement);
        // the ``if`` follows. Find the ``If`` statement and confirm
        // the inner block contains the inlined ``Block``.
        let if_stmt = m
            .top_level
            .statements
            .iter()
            .find(|s| matches!(s, Statement::If { .. }))
            .expect("expected an If statement");
        match if_stmt {
            Statement::If { clauses, .. } => {
                assert_eq!(count_blocks(&clauses[0].body), 1);
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn rewriter_is_idempotent() {
        // Running the pass twice yields the same module — once
        // inlined, the callsite no longer matches the pattern.
        let mut m1 = lower_to_ir("proc reset {} { uplevel 1 {set counter 0} }\nreset", &reg());
        inline_uplevel_passthrough(&mut m1, &reg());
        let after_first = m1.clone();
        inline_uplevel_passthrough(&mut m1, &reg());
        assert_eq!(
            after_first.top_level.statements.len(),
            m1.top_level.statements.len()
        );
    }

    #[test]
    fn rewriter_with_no_candidates_is_noop() {
        let mut m = lower_to_ir("set x 1\nputs $x", &reg());
        let before = m.clone();
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(
            before.top_level.statements.len(),
            m.top_level.statements.len()
        );
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_inlines_param_body_brace_literal() {
        // ``proc dispatcher {body} { uplevel 1 $body }``
        // followed by ``dispatcher {set counter 0}`` rewrites the
        // callsite to a Block containing the parsed literal body.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {set counter 0}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        // Find the rewritten block — top-level has [proc-call,
        // dispatcher-call→Block, …].
        let block = m
            .top_level
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Block { .. }))
            .expect("expected a Block from the ParamBody rewrite");
        if let Statement::Block { body, .. } = block {
            assert!(!body.statements.is_empty(), "block body should be lowered");
        }
    }

    #[test]
    fn rewriter_skips_param_body_with_dynamic_arg() {
        // ``dispatcher $dyn`` — the arg is a $var, not a brace
        // literal. Stay on the dispatch path.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher $dyn",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_with_command_subst_arg() {
        // ``dispatcher [build_body]`` — Cmd token, not a brace
        // literal. Stay on the dispatch path.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher [build_body]",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_when_inner_has_frame_reach() {
        // The inlined body contains an ``uplevel 1`` — semantics
        // would change after inlining, refuse.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {uplevel 1 {set x 1}}",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }

    #[test]
    fn rewriter_skips_param_body_with_expand_word() {
        // ``dispatcher {*}$args`` — expansion defeats the
        // single-arg analysis.
        let mut m = lower_to_ir(
            "proc dispatcher {body} { uplevel 1 $body }\ndispatcher {*}$args",
            &reg(),
        );
        inline_uplevel_passthrough(&mut m, &reg());
        assert_eq!(count_blocks(&m.top_level), 0);
    }
}
