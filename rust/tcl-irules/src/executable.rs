//! Registry-aware inventory of commands that can execute in an iRule.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use tcl_compiler::head_identity::{HeadIdentityMap, command_head_identities_with_config};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{ExprTokenType, LexerConfig, Token, TokenType, tokenise_expr};
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// The one resolved iRules grammar drives both script and expression lexing.
///
/// A caller normally supplies a profile-stamped iRules registry.  The
/// fallback keeps the public inventory API correct for legacy registries that
/// loaded the iRules pack without retaining its profile.
#[derive(Clone, Copy)]
struct InventoryLexing {
    config: LexerConfig,
    expr_dialect: &'static str,
}

impl InventoryLexing {
    fn for_registry(registry: &CommandRegistry) -> Self {
        let profile = registry
            .profile()
            .filter(|profile| profile.is_irules())
            .unwrap_or_else(tcl_dialect::DialectProfile::irules);
        Self {
            config: LexerConfig::for_file_grammar(profile.grammar),
            expr_dialect: profile.name,
        }
    }
}

/// Immutable inventory-wide services threaded through recursive script walks.
struct InventoryContext<'a> {
    registry: &'a CommandRegistry,
    identities: &'a HeadIdentityMap,
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

/// Commands that can execute from valid top-level event and procedure
/// declarations. Invalid top-level executable statements and nested
/// declarations are excluded; registry-declared bodies and command
/// substitutions are followed recursively.
#[must_use]
pub fn irules_executable_commands(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    let lexing = InventoryLexing::for_registry(registry);
    let identities = command_head_identities_with_config(source, lexing.config, registry);
    let ctx = InventoryContext {
        registry,
        identities: &identities,
        lexing,
    };
    let mut event_bodies = Vec::new();
    let mut procedures = HashMap::<String, Token>::new();
    collect_top_level_regions(source, &ctx, &mut event_bodies, &mut procedures);

    // Events are the only execution roots in an iRule. Procedure bodies enter
    // the inventory only through a statically resolved registry-declared
    // user-proc invocation (`call`), recursively and cycle-safely.
    let mut out = Vec::new();
    let mut pending = VecDeque::new();
    for (event, body) in event_bodies {
        let before = out.len();
        recurse_token(source, &body, &ctx, Context::Event(event), &mut out, 1);
        enqueue_proc_calls(&out[before..], registry, &mut pending);
    }
    let mut reached = HashSet::new();
    while let Some(name) = pending.pop_front() {
        if !reached.insert(name.clone()) {
            continue;
        }
        let Some(body) = procedures.get(&name) else {
            continue;
        };
        let before = out.len();
        recurse_token(source, body, &ctx, Context::Procedure, &mut out, 1);
        enqueue_proc_calls(&out[before..], registry, &mut pending);
    }
    // A local proc spelling is not an invocation form in iRules; only a
    // registry-declared `INVOKES_USER_PROC` edge (`call`) can dispatch it.
    out.retain(|command| !procedures.contains_key(&command.command));
    out
}

fn enqueue_proc_calls(
    commands: &[IrulesExecutableCommand],
    registry: &CommandRegistry,
    pending: &mut VecDeque<String>,
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
            {
                pending.push_back(name.clone());
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
                    procedures.insert((*name).to_owned(), body);
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
    Procedure,
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
            Context::Procedure => IrulesExecutionContext::ProcedureBody,
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
                    variable_names.push(raw.trim_matches(['{', '}']).to_owned());
                }
            }
        }
        out.push(IrulesExecutableCommand {
            span: cmd.span,
            command: head.clone(),
            args: cmd.args().to_vec(),
            variable_names,
            event: match &context {
                Context::Event(event) => Some(event.clone()),
                Context::Procedure => None,
            },
        });

        recurse_bodies(full, &cmd, &head, &args, ctx, context.clone(), out, depth);
        recurse_case_bodies(full, &cmd, &head, &args, ctx, context.clone(), out, depth);
        let owned_spans: HashSet<_> = registry
            .arg_indices_for_role(&head, &args, ArgRole::Body)
            .into_iter()
            .filter_map(|idx| cmd.argv.get(idx + 1))
            .map(|tok| (tok.span.start(), tok.span.end()))
            .collect();
        for index in registry.arg_indices_for_role(&head, &args, ArgRole::Expr) {
            let Some(token) = cmd.argv.get(index + 1) else {
                continue;
            };
            // The main Tcl lexer deliberately leaves a braced word opaque, so
            // its `all_tokens` contains no `Cmd` token for `if {[class match
            // ...]}`. The expression lexer owns that nested language. Its
            // command end is inclusive; preserve that contract while stripping
            // only the actual `[` and optional closing `]` before handing the
            // interior back to the script walker. Everything else in an
            // expression, including quoted strings and malformed non-command
            // data, remains inert.
            if token.kind != TokenType::Str || token.content_offset != 1 {
                continue;
            }
            let expression_start = token.span.start() as usize + token.content_offset as usize;
            let expression_end = token.span.end() as usize;
            let Some(expression) = full.get(expression_start..expression_end) else {
                continue;
            };
            for expr_token in tokenise_expr(expression, Some(lexing.expr_dialect)) {
                if expr_token.kind != ExprTokenType::Command {
                    continue;
                }
                let command_start = expression_start + expr_token.start as usize;
                let command_end = expression_start + expr_token.end as usize + 1;
                let Some(command) = full.get(command_start..command_end) else {
                    continue;
                };
                let Some(interior) = command.strip_prefix('[') else {
                    continue;
                };
                let interior = interior.strip_suffix(']').unwrap_or(interior);
                if interior.is_empty() {
                    continue;
                }
                walk(
                    full,
                    interior,
                    u32::try_from(command_start + 1).unwrap_or(0),
                    ctx,
                    context.clone(),
                    out,
                    depth + 1,
                );
            }
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
        let body_start = start + body.start + 1;
        let body_end = start + body.end.saturating_sub(1);
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
            tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules()),
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
    fn inventory_uses_expr_command_token_spans_without_walking_expr_data() {
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
                ("class", 189, 235),
                ("HTTP::uri", 202, 211),
                ("class", 259, 308),
                ("HTTP::path", 272, 282),
                ("class", 335, 389),
                ("HTTP::method", 348, 360),
            ],
            "spans are absolute bytes, include each command's final body byte, and never include the expression brackets"
        );
        assert!(
            facts
                .iter()
                .all(|fact| { !fact.args.iter().any(|arg| arg == "/Common/inert_dg") })
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
}
