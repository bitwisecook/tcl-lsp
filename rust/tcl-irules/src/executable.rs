//! Registry-aware inventory of commands that can execute in an iRule.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

#[cfg(feature = "test-instrumentation")]
use std::cell::Cell;

use tcl_compiler::realm::{CommandBindingRealm, document_realm_bindings_with_config};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, Token, TokenType};
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::expr_surface::RuntimeExprSurface;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// The one resolved iRules grammar drives both script and expression lexing.
///
/// A caller normally supplies a profile-stamped iRules registry.  The
/// fallback keeps the public inventory API correct for legacy registries that
/// loaded the iRules pack without retaining its profile.
#[derive(Clone, Copy)]
struct InventoryLexing {
    /// The resolved profile is the single source for expression grammar and
    /// runtime-surface selection.  Keep it beside the script lexer config so
    /// an inventory never mixes an iRules script parse with another release's
    /// expression rules.
    profile: &'static tcl_dialect::DialectProfile,
    config: LexerConfig,
    expr_surface: RuntimeExprSurface,
}

impl InventoryLexing {
    fn for_registry(registry: &CommandRegistry) -> Self {
        let profile = registry
            .profile()
            .filter(|profile| profile.is_irules())
            .unwrap_or_else(tcl_dialect::DialectProfile::irules);
        Self {
            profile,
            config: LexerConfig::for_file_grammar(profile.grammar),
            expr_surface: RuntimeExprSurface::for_profile(profile),
        }
    }
}

/// Immutable inventory-wide services threaded through recursive script walks.
struct InventoryContext<'a> {
    registry: &'a CommandRegistry,
    identities: &'a CommandBindingRealm,
    lexing: InventoryLexing,
}

/// One executable command, after static command-head resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesExecutableCommand {
    pub span: tcl_lexer::Span,
    pub command: String,
    pub args: Vec<String>,
    pub variable_names: Vec<String>,
    pub event: Option<String>,
}

#[cfg(feature = "test-instrumentation")]
thread_local! {
    static EXECUTABLE_CLOSURE_BUILDS: Cell<usize> = const { Cell::new(0) };
}

/// Reset this thread's executable-closure build count.
///
/// Available only to the integration test feature. Production consumers never
/// carry this instrumentation.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
pub fn reset_executable_closure_builds_for_tests() {
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(0));
}

/// Return this thread's executable-closure build count.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
#[must_use]
pub fn executable_closure_builds_for_tests() -> usize {
    EXECUTABLE_CLOSURE_BUILDS.with(Cell::get)
}

/// Commands that can execute from valid top-level event and procedure
/// declarations. Invalid top-level executable statements and nested
/// declarations are excluded; registry-declared bodies and command
/// substitutions are followed recursively.
#[must_use]
pub fn irules_executable_commands(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    #[cfg(feature = "test-instrumentation")]
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(builds.get() + 1));
    let lexing = InventoryLexing::for_registry(registry);
    let identities = document_realm_bindings_with_config(source, lexing.config, registry);
    let ctx = InventoryContext {
        registry,
        identities: &identities,
        lexing,
    };
    let mut event_bodies = Vec::new();
    let mut procedures = HashMap::<String, Token>::new();
    collect_top_level_regions(source, &ctx, &mut event_bodies, &mut procedures);
    event_rooted_closure(source, &ctx, event_bodies, &procedures)
}

/// Return the event-rooted executable closure for every valid top-level
/// `when EVENT { … }` handler matching `event`.
///
/// The closure includes all matching handlers, not merely the first source
/// occurrence, followed by procedures reached through registry-declared
/// [`Traits::INVOKES_USER_PROC`] edges.  Ordinary Tcl-looking direct calls do
/// not make a procedure reachable.  Procedure traversal is cycle-safe and
/// each returned command keeps its exact source span.
#[must_use]
pub fn irules_event_executable_closure(
    source: &str,
    event: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    let lexing = InventoryLexing::for_registry(registry);
    let identities = document_realm_bindings_with_config(source, lexing.config, registry);
    let ctx = InventoryContext {
        registry,
        identities: &identities,
        lexing,
    };
    let mut event_bodies = Vec::new();
    let mut procedures = HashMap::<String, Token>::new();
    collect_top_level_regions(source, &ctx, &mut event_bodies, &mut procedures);
    event_rooted_closure(
        source,
        &ctx,
        event_bodies
            .into_iter()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(event))
            .collect(),
        &procedures,
    )
}

/// Build an executable closure from already-proven event roots.
fn event_rooted_closure(
    source: &str,
    ctx: &InventoryContext<'_>,
    event_bodies: Vec<(String, Token)>,
    procedures: &HashMap<String, Token>,
) -> Vec<IrulesExecutableCommand> {
    // Events are the only execution roots in an iRule. Procedure bodies enter
    // the inventory only through a statically resolved registry-declared
    // user-proc invocation (`call`), recursively and cycle-safely.
    let mut out = Vec::new();
    let mut pending = VecDeque::<(String, String)>::new();
    for (event, body) in event_bodies {
        let before = out.len();
        recurse_token(source, &body, ctx, Context::Event(event), &mut out, 1);
        enqueue_proc_calls(&out[before..], ctx.registry, &mut pending);
    }
    let mut reached = HashSet::new();
    while let Some((name, event)) = pending.pop_front() {
        if !reached.insert(name.clone()) {
            continue;
        }
        let Some(body) = procedures.get(&name) else {
            continue;
        };
        let before = out.len();
        // A procedure executes on behalf of the event that reached it. Keep
        // that provenance on every command in the closure so consumers that
        // classify event-sensitive state do not mistake a called helper for
        // dormant code.
        recurse_token(source, body, ctx, Context::Procedure(event), &mut out, 1);
        enqueue_proc_calls(&out[before..], ctx.registry, &mut pending);
    }
    // A local proc spelling is not an invocation form in iRules; only a
    // registry-declared `INVOKES_USER_PROC` edge (`call`) can dispatch it.
    out.retain(|command| !procedures.contains_key(&procedure_key(&command.command)));
    out
}

/// iRules user procedures live in the global command namespace.  The absolute
/// marker is spelling, not a distinct procedure identity, so `call helper`
/// and `call ::helper` must reach the same top-level declaration.
fn procedure_key(name: &str) -> String {
    tcl_syntax::naming::canonical_written_command(name)
        .trim_start_matches("::")
        .to_owned()
}

fn enqueue_proc_calls(
    commands: &[IrulesExecutableCommand],
    registry: &CommandRegistry,
    pending: &mut VecDeque<(String, String)>,
) {
    for command in commands {
        let Some(spec) = registry.get(&command.command) else {
            continue;
        };
        if !spec.traits.contains(Traits::INVOKES_USER_PROC) {
            continue;
        }
        let args: Vec<&str> = command.args.iter().map(String::as_str).collect();
        for index in registry.arg_indices_for_role(&command.command, &args, ArgRole::Name) {
            if let Some(name) = command.args.get(index)
                && !name.contains(['$', '[', ']', ';'])
                && let Some(event) = command.event.as_ref()
            {
                pending.push_back((procedure_key(name), event.clone()));
            }
        }
    }
}

fn collect_top_level_regions(
    source: &str,
    ctx: &InventoryContext<'_>,
    events: &mut Vec<(String, Token)>,
    procedures: &mut HashMap<String, Token>,
) {
    for cmd in segment_commands_with_offset_and_config(source, 0, ctx.lexing.config) {
        let at = cmd.argv.first().map_or(0, |token| token.span.start());
        let resolved = ctx.identities.head_words(cmd.name(), at).resolved;
        let canonical = tcl_syntax::naming::canonical_written_command(resolved);
        let head = if ctx.registry.get_exact(&canonical).is_some() {
            canonical
        } else {
            canonical.trim_start_matches("::").to_owned()
        };
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        let Some(closed) = tcl_registry::events::closed_braced_argument_words(
            source,
            cmd.arg_tokens(),
            cmd.arg_single_token(),
        ) else {
            continue;
        };
        let Some(arguments) = tcl_registry::events::IrulesDeclarationArguments::new(
            &args,
            cmd.arg_tokens(),
            cmd.arg_single_token(),
            &closed,
        ) else {
            continue;
        };
        match ctx
            .registry
            .irules_top_level_declaration(&head, arguments, event_registry())
        {
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Event {
                event,
                body_index,
                ..
            }) => {
                if let Some(body) = cmd
                    .argv
                    .get(body_index + 1)
                    .copied()
                    .filter(|body| body.kind == TokenType::Str)
                {
                    events.push((event, body));
                }
            }
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure {
                name_index,
                body_index,
            }) => {
                let (Some(name), Some(body)) =
                    (args.get(name_index), cmd.argv.get(body_index + 1).copied())
                else {
                    continue;
                };
                if !name.contains(['$', '[', ']', ';']) {
                    procedures.insert(procedure_key(name), body);
                }
            }
            Some(
                tcl_registry::events::IrulesTopLevelDeclaration::Priority { .. }
                | tcl_registry::events::IrulesTopLevelDeclaration::Timing { .. },
            )
            | None => {}
        }
    }
}

#[derive(Clone)]
enum Context {
    Event(String),
    Procedure(String),
}

fn event_registry() -> &'static tcl_registry::events::EventRegistry {
    static EVENTS: OnceLock<tcl_registry::events::EventRegistry> = OnceLock::new();
    EVENTS.get_or_init(tcl_registry::events::EventRegistry::build)
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]
fn walk(
    full: &str,
    slice: &str,
    base: u32,
    ctx: &InventoryContext<'_>,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    if depth >= 256 {
        return;
    }
    let registry = ctx.registry;
    let identities = ctx.identities;
    let lexing = ctx.lexing;
    for cmd in segment_commands_with_offset_and_config(slice, base, lexing.config.nested()) {
        let at = cmd.argv.first().map_or(0, |token| token.span.start());
        let resolved = identities.head_words(cmd.name(), at).resolved;
        let canonical = tcl_syntax::naming::canonical_written_command(resolved);
        let head = if registry.get_exact(&canonical).is_some() {
            canonical
        } else {
            canonical.trim_start_matches("::").to_owned()
        };
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();

        let nested_context = match context {
            Context::Event(_) => IrulesExecutionContext::EventBody,
            Context::Procedure(_) => IrulesExecutionContext::ProcedureBody,
        };
        if registry.irules_command_placement(&head, nested_context)
            == IrulesCommandPlacement::RequiresTopLevel
        {
            continue;
        }

        let mut variable_names = Vec::new();
        for token in &cmd.all_tokens {
            if token.kind == TokenType::Var {
                let start = token.span.start() as usize + token.content_offset as usize;
                let end = token.span.end() as usize;
                if let Some(raw) = full.get(start..end) {
                    variable_names.push(variable_name(raw));
                }
            }
        }
        let owned_spans: HashSet<_> = registry
            .arg_indices_for_role(&head, &args, ArgRole::Body)
            .into_iter()
            .filter_map(|idx| cmd.argv.get(idx + 1))
            .map(|tok| (tok.span.start(), tok.span.end()))
            .collect();
        let mut expression_command_spans = Vec::new();
        for index in registry.arg_indices_for_role(&head, &args, ArgRole::Expr) {
            let Some(token) = cmd.argv.get(index + 1) else {
                continue;
            };
            // Tcl substitutes an unbraced or quoted argument *before* `if`
            // asks `expr` to parse it.  Therefore its complete lexer-owned
            // `Cmd` tokens must flow through the generic substitution pass
            // below even if the resulting expression is invalid. A braced
            // word is opaque to the script lexer and is evaluated by `expr`
            // itself, so only that form needs the syntax/runtime-surface gate
            // to recover its live command substitutions.
            let token_start = token.span.start() as usize;
            if full.as_bytes().get(token_start) != Some(&b'{') || token.content_offset != 1 {
                continue;
            }
            let expression_start = token_start + token.content_offset as usize;
            let expression_end = token.span.end() as usize;
            let Some(expression) = full.get(expression_start..expression_end) else {
                continue;
            };
            let substitutions = tcl_syntax::expr::live_expression_substitutions(
                expression,
                lexing.profile,
                lexing.config,
                |parsed| lexing.expr_surface.validate(parsed).is_ok(),
            );
            for span in substitutions.variables {
                let variable_start = expression_start + span.start() as usize;
                let variable_end = expression_start + span.end() as usize;
                if let Some(raw) = full.get(variable_start..variable_end) {
                    variable_names.push(variable_name(raw));
                }
            }
            for span in substitutions.commands {
                let command_start = expression_start + span.start() as usize;
                let command_end = expression_start + span.end() as usize;
                let Some(interior_start) = command_start.checked_add(1) else {
                    continue;
                };
                let Some(interior_end) = command_end.checked_sub(1) else {
                    continue;
                };
                if interior_start >= interior_end {
                    continue;
                }
                expression_command_spans.push(interior_start..interior_end);
            }
        }
        out.push(IrulesExecutableCommand {
            span: cmd.span,
            command: head.clone(),
            args: cmd.args().to_vec(),
            variable_names,
            event: match &context {
                Context::Event(event) | Context::Procedure(event) => Some(event.clone()),
            },
        });

        recurse_bodies(full, &cmd, &head, &args, ctx, context.clone(), out, depth);
        recurse_case_bodies(full, &cmd, &head, &args, ctx, context.clone(), out, depth);
        for command_range in expression_command_spans {
            let Some(interior) = full.get(command_range.clone()) else {
                continue;
            };
            walk(
                full,
                interior,
                u32::try_from(command_range.start).unwrap_or(0),
                ctx,
                context.clone(),
                out,
                depth + 1,
            );
        }
        for token in &cmd.all_tokens {
            if token.kind == TokenType::Cmd
                && !owned_spans.contains(&(token.span.start(), token.span.end()))
            {
                recurse_token(full, token, ctx, context.clone(), out, depth + 1);
            }
        }
    }
}

/// Convert a lexer-owned variable spelling into the public inventory name.
///
/// Script tokens arrive with their `$` / `${` introducer stripped through
/// `content_offset`; expression spans intentionally include it so their exact
/// source range remains available to every consumer. Normalise only those
/// delimiters here, preserving array keys and namespace spelling exactly like
/// the pre-existing script-token path.
fn variable_name(raw: &str) -> String {
    let raw = raw.strip_prefix('$').unwrap_or(raw);
    raw.strip_prefix('{')
        .and_then(|name| name.strip_suffix('}'))
        .unwrap_or(raw)
        .to_owned()
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn recurse_case_bodies(
    full: &str,
    cmd: &SegmentedCommand,
    head: &str,
    args: &[&str],
    ctx: &InventoryContext<'_>,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    let registry = ctx.registry;
    let dialect = registry
        .profile()
        .map_or_else(tcl_dialect::DialectSet::empty, |profile| {
            profile.availability_mask
        });
    let Some((spec, invocation)) = registry.case_invocation(head, args, dialect) else {
        return;
    };
    let Some(index) = invocation.clause_list_index else {
        return;
    };
    let Some(token) = cmd
        .argv
        .get(index + 1)
        .filter(|token| token.kind == TokenType::Str)
    else {
        return;
    };
    let start = token.span.start() as usize + token.content_offset as usize;
    let end = token.span.end() as usize;
    let Some(inner) = full.get(start..end) else {
        return;
    };
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };
    for body in tcl_syntax::case_list::split_case_list(inner, &shape)
        .into_iter()
        .filter_map(|clause| clause.body)
        .filter(|body| body.braced)
    {
        // The case-list owner alone defines the braced arm's interior. Its
        // end is already exclusive (at the closing brace), so subtracting
        // from it would shave the final byte of the nested command and make
        // this closure disagree with the reference walker.
        let body_range = body.content_range();
        let body_start = start + body_range.start;
        let body_end = start + body_range.end;
        if let Some(script) = full.get(body_start..body_end) {
            walk(
                full,
                script,
                u32::try_from(body_start).unwrap_or(0),
                ctx,
                context.clone(),
                out,
                depth + 1,
            );
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn recurse_bodies(
    full: &str,
    cmd: &SegmentedCommand,
    head: &str,
    args: &[&str],
    ctx: &InventoryContext<'_>,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    let registry = ctx.registry;
    for idx in registry.arg_indices_for_role(head, args, ArgRole::Body) {
        if let Some(token) = cmd.argv.get(idx + 1) {
            recurse_token(full, token, ctx, context.clone(), out, depth + 1);
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn recurse_token(
    full: &str,
    token: &Token,
    ctx: &InventoryContext<'_>,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    // The recovery lexer preserves a `Cmd` token for an unterminated `[` so
    // editor features can still colour the fragment.  It is not an executable
    // command substitution, however: only the lexer range owner can prove a
    // closing `]` through nested Tcl syntax and comments.
    if token.kind == TokenType::Cmd
        && tcl_lexer::command_substitution_end(full, token.span.start() as usize).is_none()
    {
        return;
    }
    let start = token.span.start() as usize + token.content_offset as usize;
    let end = token.span.end() as usize;
    if let Some(inner) = full.get(start..end) {
        walk(
            full,
            inner,
            u32::try_from(start).unwrap_or(0),
            ctx,
            context,
            out,
            depth,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(source: &str) -> Vec<IrulesExecutableCommand> {
        irules_executable_commands(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_dialect::DialectProfile::irules(),
            )
            .commands(),
        )
    }

    #[test]
    fn inventory_obeys_irules_execution_boundaries() {
        let facts = commands(concat!(
            "pool invalid_top\n",
            "proc helper {} { pool from_proc }\n",
            "when HTTP_REQUEST { call helper; switch -- x { x { pool from_event } }; when CLIENT_DATA { pool invalid_nested } }\n",
        ));
        let pools: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(pools, ["from_event", "from_proc"]);
    }

    #[test]
    fn event_closure_selects_every_matching_handler_and_only_registry_call_edges() {
        let source = concat!(
            "proc first_helper {} { pool helper_one; call second_helper }\n",
            "proc second_helper {} { pool helper_two; call first_helper }\n",
            "proc dormant {} { pool dormant_pool }\n",
            "when HTTP_REQUEST { pool first_pool; call first_helper; dormant; apply {{} { pool lambda_pool }} }\n",
            "when http_request { pool second_pool; call second_helper }\n",
            "when CLIENT_DATA { pool other_event_pool; call dormant }\n",
        );
        let registry = tcl_registry::model::ingress::static_context_for_profile(
            tcl_dialect::DialectProfile::irules(),
        )
        .commands();
        let closure = irules_event_executable_closure(source, "HTTP_REQUEST", registry);
        let pools: Vec<_> = closure
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(
            pools,
            ["first_pool", "second_pool", "helper_one", "helper_two"],
            "both roots and the cycle-safe call closure are present exactly once"
        );
        assert!(
            closure.iter().all(|fact| {
                !matches!(
                    fact.args.first().map(String::as_str),
                    Some("dormant_pool" | "other_event_pool" | "lambda_pool")
                )
            }),
            "cross-event, dormant, direct-call, and lambda body regions are not in the closure"
        );
        let first_span = closure
            .iter()
            .find(|fact| fact.command == "pool" && fact.args == ["first_pool"])
            .map(|fact| fact.span)
            .expect("first matching handler pool");
        assert_eq!(
            &source[first_span.as_range()],
            "pool first_pool",
            "closure spans slice the original source exactly"
        );
    }

    #[test]
    fn inventory_follows_live_substitutions_not_tcl_data() {
        let facts = commands(concat!(
            "when HTTP_REQUEST {\n",
            " # HTTP::respond 500; set static::comment 1\n",
            " set inert {HTTP::respond 501; set static::data 1}\n",
            " set live [HTTP::uri]\n",
            "}\n",
        ));
        assert!(facts.iter().any(|fact| fact.command == "HTTP::uri"));
        assert!(!facts.iter().any(|fact| fact.command == "HTTP::respond"));
        assert!(facts.iter().all(|fact| {
            !fact
                .variable_names
                .iter()
                .any(|name| name.starts_with("static::"))
                && fact
                    .args
                    .first()
                    .is_none_or(|arg| !arg.starts_with("static::"))
        }));
    }

    #[test]
    fn inventory_treats_expr_literals_as_data_but_follows_live_substitutions() {
        let facts = commands(concat!(
            "when HTTP_REQUEST {\n",
            "  expr { pool expr_literal; HTTP::respond 500 }\n",
            "  expr [HTTP::uri]\n",
            "}\n",
        ));
        assert!(
            facts.iter().any(|fact| fact.command == "HTTP::uri"),
            "a real TokenType::Cmd expression substitution executes Tcl"
        );
        for inert in ["pool", "HTTP::respond"] {
            assert!(
                facts.iter().all(|fact| fact.command != inert),
                "expression literal text must not become an executable {inert} command"
            );
        }
    }

    #[test]
    fn inventory_uses_complete_live_expression_spans_without_walking_expr_data() {
        let source = concat!(
            "when HTTP_REQUEST {\n",
            "  set marker \"☃\"\n",
            "  if {[class match [HTTP::host] equals /Common/braced_dg]} { set hit 1 }\n",
            "  if {\"[class match ignored equals /Common/inert_dg]\"} { set inert 1 }\n",
            "  if [class match [HTTP::uri] equals /Common/bare_dg] { set bare 1 }\n",
            "  if \"[class match [HTTP::path] equals /Common/quoted_dg]\" { set quoted 1 }\n",
            "  if {[class match [HTTP::method] equals /Common/recovered_dg} { set recovered 1 }\n",
            "}\n",
        );
        let facts = commands(source);
        let got: Vec<_> = facts
            .iter()
            .filter(|fact| {
                matches!(
                    fact.command.as_str(),
                    "class" | "HTTP::host" | "HTTP::uri" | "HTTP::path" | "HTTP::method"
                )
            })
            .map(|fact| (fact.command.as_str(), fact.span.start(), fact.span.end()))
            .collect();
        assert_eq!(
            got,
            [
                ("class", 46, 95),
                ("HTTP::host", 59, 69),
                ("class", 120, 163),
                ("class", 189, 235),
                ("HTTP::uri", 202, 211),
                ("class", 259, 308),
                ("HTTP::path", 272, 282),
            ],
            "spans are absolute bytes, include each command's final body byte, include quoted expression substitutions, and never include malformed brackets"
        );
        assert!(
            facts
                .iter()
                .any(|fact| { fact.args.iter().any(|arg| arg == "/Common/inert_dg") }),
            "a substitution inside an expression double quote is executable"
        );
    }

    #[test]
    fn inventory_keeps_complete_case_arm_spans_for_reference_consumers() {
        let source = concat!(
            "when HTTP_REQUEST {\n",
            "  switch $route {\n",
            "    first { pool /Common/\u{2603} }\n",
            "    nested { if {$enabled} { pool /Common/nested } }\n",
            "    default { pool /Common/final }\n",
            "  }\n",
            "}\n",
        );
        let registry = tcl_registry::model::ingress::static_context_for_profile(
            tcl_dialect::DialectProfile::irules(),
        )
        .commands();
        let facts = irules_executable_commands(source, registry);
        let pools: Vec<_> = facts.iter().filter(|fact| fact.command == "pool").collect();
        assert_eq!(
            pools
                .iter()
                .map(|fact| fact.args[0].as_str())
                .collect::<Vec<_>>(),
            ["/Common/\u{2603}", "/Common/nested", "/Common/final"],
        );
        for fact in pools {
            assert_eq!(
                &source[fact.span.as_range()],
                format!("pool {}", fact.args[0]),
                "an executable span includes its final body byte"
            );
        }

        let references = crate::extract_irules_object_references(source, None, registry);
        assert_eq!(
            references
                .iter()
                .map(|reference| reference.name.as_str())
                .collect::<Vec<_>>(),
            ["/Common/\u{2603}", "/Common/nested", "/Common/final"],
            "the shared reference walk accepts every complete closure span",
        );

        let malformed =
            commands("when HTTP_REQUEST { switch x { first { pool /Common/live } odd } }");
        assert!(
            malformed.iter().all(|fact| fact.command != "pool"),
            "a malformed clause list errors before any arm runs: {malformed:?}"
        );
    }

    #[test]
    fn inventory_collects_live_braced_expr_variables_and_keeps_command_ownership() {
        let source = concat!(
            "when HTTP_REQUEST {\n",
            "  set marker \"\u{2603}\"\n",
            "  if {$static::maintenance && $static::table($static::slot) && ${static::braced} && [set static::from_command 1]} {}\n",
            "  if {\"$static::quoted\" eq {${static::braced_data}}} {}\n",
            "  if \"$static::word_timed +\" {}\n",
            "  if {$static::broken +} {}\n",
            "}\n",
        );
        let facts = commands(source);
        let live = facts
            .iter()
            .find(|fact| {
                fact.command == "if"
                    && fact
                        .args
                        .first()
                        .is_some_and(|arg| arg.contains("static::maintenance"))
            })
            .expect("live braced expression command");
        assert_eq!(
            live.variable_names,
            [
                "static::maintenance",
                "static::table($static::slot)",
                "static::slot",
                "static::braced",
            ],
        );
        assert!(
            !live
                .variable_names
                .iter()
                .any(|name| name.contains("from_command")),
            "the nested command owns its script variable facts"
        );
        assert!(facts.iter().any(|fact| {
            fact.command == "set"
                && fact
                    .args
                    .first()
                    .is_some_and(|arg| arg == "static::from_command")
        }));
        let quoted_in_braced_expr = facts
            .iter()
            .find(|fact| {
                fact.command == "if"
                    && fact
                        .args
                        .first()
                        .is_some_and(|arg| arg.contains("static::quoted"))
            })
            .expect("quoted operand inside braced expression");
        assert_eq!(quoted_in_braced_expr.variable_names, ["static::quoted"]);
        assert!(facts.iter().all(|fact| {
            !(fact.command == "if"
                && fact
                    .variable_names
                    .iter()
                    .any(|name| matches!(name.as_str(), "static::braced_data" | "static::broken")))
        }));
        let quoted = facts
            .iter()
            .find(|fact| {
                fact.command == "if"
                    && fact
                        .args
                        .first()
                        .is_some_and(|arg| arg.contains("word_timed"))
            })
            .expect("quoted expression command");
        assert_eq!(
            quoted.variable_names,
            ["static::word_timed"],
            "a quoted word substitutes before expr rejects its trailing operator"
        );
    }

    #[test]
    fn inventory_drops_unterminated_unbraced_expression_substitutions() {
        let facts =
            commands("when HTTP_REQUEST { if [class match [HTTP::host] equals malformed_dg }");
        assert!(
            facts
                .iter()
                .all(|fact| !matches!(fact.command.as_str(), "class" | "HTTP::host")),
            "a recovery Cmd token without a closing bracket is not executable: {facts:?}"
        );
    }

    #[test]
    fn inventory_matches_tcl86_and_tcl9_word_substitution_before_expr_errors() {
        // C Tcl 8.6 and 9 both execute the complete substitutions in the
        // quoted and bare words before `if` reports their trailing `+` as an
        // expression error. A braced word reaches expr without Tcl word
        // substitution, so its malformed expression executes neither command.
        let facts = commands(concat!(
            "when HTTP_REQUEST {\n",
            "  if \"[HTTP::uri] +\" {}\n",
            "  if [HTTP::host] + {}\n",
            "  if {[HTTP::method] +} {}\n",
            "}\n",
        ));
        let commands: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command.starts_with("HTTP::"))
            .map(|fact| fact.command.as_str())
            .collect();
        assert_eq!(
            commands,
            ["HTTP::uri", "HTTP::host"],
            "quoted and bare word substitutions precede expression parsing; braced ones do not"
        );
    }

    #[test]
    fn inventory_requires_braced_declaration_bodies() {
        let facts = commands(concat!(
            "when BOGUS_EVENT { pool bogus; set static::bogus 1; table incr bogus }\n",
            "when CLIENT_DATA pool\n",
            "when SERVER_DATA \"pool quoted_event\"\n",
            "proc missing {}\n",
            "proc bare_proc {} pool\n",
            "proc quoted_proc {} \"pool quoted_proc\"\n",
            "proc extra {} { pool malformed } trailing\n",
            "proc valid {} { pool valid_proc }\n",
            "when HTTP_REQUEST { call valid; pool valid_event }\n",
        ));
        let pools: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(pools, ["valid_event", "valid_proc"]);
        assert!(facts.iter().all(|fact| fact.command != "table"));
        assert!(
            facts
                .iter()
                .all(|fact| fact.args.first().is_none_or(|arg| arg != "static::bogus"))
        );
    }

    #[test]
    fn procedure_inventory_is_reachable_from_events_through_call_edges() {
        let facts = commands(concat!(
            "proc dormant {} { pool dormant }\n",
            "proc leaf {} { pool leaf; call cycle_a }\n",
            "proc cycle_a {} { pool cycle_a; call cycle_b }\n",
            "proc cycle_b {} { pool cycle_b; call cycle_a }\n",
            "when HTTP_REQUEST { call leaf; pool event }\n",
        ));
        let pools: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(pools, ["event", "leaf", "cycle_a", "cycle_b"]);
    }

    #[test]
    fn direct_proc_spelling_is_not_an_execution_edge() {
        let facts = commands(concat!(
            "proc helper {} { pool forbidden }\n",
            "when HTTP_REQUEST { helper }\n",
        ));
        assert!(facts.iter().all(|fact| fact.command != "helper"));
        assert!(facts.iter().all(|fact| fact.command != "pool"));
    }

    #[test]
    fn call_edges_normalise_the_global_procedure_marker() {
        let facts = commands(concat!(
            "proc ::helper {} { pool rooted_helper }\n",
            "when HTTP_REQUEST { call helper }\n",
        ));
        assert!(
            facts
                .iter()
                .any(|fact| fact.command == "pool" && fact.args == ["rooted_helper"]),
            "the absolute marker is not a distinct user-procedure identity: {facts:?}"
        );
    }
}
