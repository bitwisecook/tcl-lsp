//! Registry-aware inventory of commands that can execute in an iRule.

use std::collections::HashSet;
use std::sync::OnceLock;

use tcl_compiler::head_identity::{HeadIdentityMap, command_head_identities_with_config};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, Token, TokenType};
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

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
    let config = LexerConfig::for_file_dialect("f5-irules");
    let identities = command_head_identities_with_config(source, config, registry);
    let mut out = Vec::new();
    walk(
        source,
        source,
        0,
        registry,
        &identities,
        Context::TopLevel,
        &mut out,
        0,
    );
    out
}

#[derive(Clone)]
enum Context {
    TopLevel,
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
    registry: &CommandRegistry,
    identities: &HeadIdentityMap,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    if depth >= 256 {
        return;
    }
    for cmd in segment_commands_with_offset_and_config(
        slice,
        base,
        LexerConfig::for_file_dialect("f5-irules"),
    ) {
        let at = cmd.argv.first().map_or(0, |token| token.span.start());
        let resolved = identities.head_words(cmd.name(), at).resolved;
        let canonical = tcl_syntax::naming::canonical_written_command(resolved);
        let head = if registry.get_exact(&canonical).is_some() {
            canonical
        } else {
            canonical.trim_start_matches("::").to_owned()
        };
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();

        if matches!(context, Context::TopLevel) {
            let Some(spec) = registry.get(&head) else {
                continue;
            };
            if registry.irules_command_placement(&head, IrulesExecutionContext::TopLevel)
                != IrulesCommandPlacement::Allowed
            {
                continue;
            }
            let next = if spec.traits.contains(Traits::IS_EVENT_HANDLER) {
                args.first()
                    .map(|event| event.to_uppercase())
                    .filter(|event| event_registry().is_known(event))
                    .map(Context::Event)
            } else if spec.traits.contains(Traits::DEFINES_PROCEDURE) {
                u16::try_from(args.len())
                    .ok()
                    .filter(|count| spec.arity.accepts(*count))
                    .and_then(|_| {
                        registry
                            .arg_indices_for_role(&head, &args, ArgRole::Body)
                            .into_iter()
                            .all(|index| cmd.argv.get(index + 1).is_some())
                            .then_some(Context::Procedure)
                    })
            } else {
                None
            };
            if let Some(next) = next {
                recurse_bodies(
                    full, &cmd, &head, &args, registry, identities, next, out, depth,
                );
            }
            continue;
        }

        let nested_context = match context {
            Context::Event(_) => IrulesExecutionContext::EventBody,
            Context::Procedure => IrulesExecutionContext::ProcedureBody,
            Context::TopLevel => unreachable!(),
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
                _ => None,
            },
        });

        recurse_bodies(
            full,
            &cmd,
            &head,
            &args,
            registry,
            identities,
            context.clone(),
            out,
            depth,
        );
        recurse_case_bodies(
            full,
            &cmd,
            &head,
            &args,
            registry,
            identities,
            context.clone(),
            out,
            depth,
        );
        let body_spans: HashSet<_> = registry
            .arg_indices_for_role(&head, &args, ArgRole::Body)
            .into_iter()
            .filter_map(|idx| cmd.argv.get(idx + 1))
            .map(|tok| (tok.span.start(), tok.span.end()))
            .collect();
        for token in &cmd.all_tokens {
            if token.kind == TokenType::Cmd
                && !body_spans.contains(&(token.span.start(), token.span.end()))
            {
                recurse_token(
                    full,
                    token,
                    registry,
                    identities,
                    context.clone(),
                    out,
                    depth + 1,
                );
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
    registry: &CommandRegistry,
    identities: &HeadIdentityMap,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
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
                registry,
                identities,
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
    registry: &CommandRegistry,
    identities: &HeadIdentityMap,
    context: Context,
    out: &mut Vec<IrulesExecutableCommand>,
    depth: u16,
) {
    for idx in registry.arg_indices_for_role(head, args, ArgRole::Body) {
        if let Some(token) = cmd.argv.get(idx + 1) {
            recurse_token(
                full,
                token,
                registry,
                identities,
                context.clone(),
                out,
                depth + 1,
            );
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn recurse_token(
    full: &str,
    token: &Token,
    registry: &CommandRegistry,
    identities: &HeadIdentityMap,
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
            registry,
            identities,
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
            "when HTTP_REQUEST { switch -- x { x { pool from_event } }; when CLIENT_DATA { pool invalid_nested } }\n",
        ));
        let pools: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(pools, ["from_proc", "from_event"]);
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
    fn unknown_events_and_malformed_procs_do_not_open_execution_regions() {
        let facts = commands(concat!(
            "when BOGUS_EVENT { pool bogus; set static::bogus 1; table incr bogus }\n",
            "proc missing {}\n",
            "proc extra {} { pool malformed } trailing\n",
            "proc valid {} { pool valid_proc }\n",
            "when HTTP_REQUEST { call valid; pool valid_event }\n",
        ));
        let pools: Vec<_> = facts
            .iter()
            .filter(|fact| fact.command == "pool")
            .map(|fact| fact.args[0].as_str())
            .collect();
        assert_eq!(pools, ["valid_proc", "valid_event"]);
        assert!(facts.iter().all(|fact| fact.command != "table"));
        assert!(
            facts
                .iter()
                .all(|fact| fact.args.first().is_none_or(|arg| arg != "static::bogus"))
        );
    }
}
