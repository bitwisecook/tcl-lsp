// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-driven extraction of iRules event handlers and effective priorities.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// A statically-resolved iRules event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesEventHandler {
    /// Event handled by the `when` command.
    pub event: String,
    /// Effective priority after top-level and per-handler overrides.
    pub priority: u16,
}

/// Extract handlers whose event and effective priority are statically known.
///
/// BIG-IP begins at the handler command's registry-declared default. A
/// top-level command carrying [`Traits::SETS_EVENT_PRIORITY`] changes the
/// inherited value for subsequent handlers; an inline priority clause wins
/// for that handler only. Dynamic and malformed priorities make the inherited
/// value unknown until a later literal setter, avoiding speculative conflicts.
#[must_use]
pub fn extract_irules_event_handlers(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesEventHandler> {
    let config = LexerConfig::for_dialect("f5-irules");
    let mut inherited_priority = registry
        .event_handler_spec()
        .and_then(|spec| spec.event_handler_priority)
        .map(|policy| policy.default_priority);
    let mut handlers = Vec::new();

    for cmd in segment_commands_with_offset_and_config(source, 0, config) {
        let Some(spec) = registry.get(cmd.name()) else {
            continue;
        };
        let args = cmd.args();

        if spec.traits.contains(Traits::SETS_EVENT_PRIORITY) {
            inherited_priority = literal_u16_arg(&cmd, 0);
            continue;
        }

        let Some(policy) = spec.event_handler_priority else {
            continue;
        };
        let Some(event) = args.first() else {
            continue;
        };
        if !literal_arg(&cmd, 0) {
            continue;
        }

        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let body_index = registry
            .arg_indices_for_role(cmd.name(), &arg_refs, ArgRole::Body)
            .into_iter()
            .next()
            .unwrap_or(args.len());
        let explicit = args
            .get(1..body_index)
            .unwrap_or(&[])
            .iter()
            .position(|arg| arg == policy.keyword)
            .map(|relative_index| relative_index + 1)
            .map(|keyword_index| literal_u16_arg(&cmd, keyword_index + 1));
        let priority = match explicit {
            Some(priority) => priority,
            None => inherited_priority,
        };
        if let Some(priority) = priority {
            handlers.push(IrulesEventHandler {
                event: event.clone(),
                priority,
            });
        }
    }
    handlers
}

fn literal_u16_arg(cmd: &tcl_compiler::segmenter::SegmentedCommand, index: usize) -> Option<u16> {
    if !literal_arg(cmd, index) {
        return None;
    }
    cmd.args().get(index)?.parse().ok()
}

fn literal_arg(cmd: &tcl_compiler::segmenter::SegmentedCommand, index: usize) -> bool {
    cmd.arg_single_token().get(index).copied() == Some(true)
        && cmd
            .arg_tokens()
            .get(index)
            .is_some_and(|token| !matches!(token.kind, TokenType::Var | TokenType::Cmd))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(source: &str) -> Vec<IrulesEventHandler> {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        extract_irules_event_handlers(source, registry)
    }

    #[test]
    fn top_level_priority_affects_only_subsequent_handlers() {
        assert_eq!(
            extract(
                "when CLIENT_ACCEPTED { one }\npriority 200\n\
                 when HTTP_REQUEST { two }\n\
                 when HTTP_RESPONSE priority 300 { three }\n\
                 when SERVER_CONNECTED { four }\n"
            ),
            vec![
                IrulesEventHandler {
                    event: "CLIENT_ACCEPTED".into(),
                    priority: 500
                },
                IrulesEventHandler {
                    event: "HTTP_REQUEST".into(),
                    priority: 200
                },
                IrulesEventHandler {
                    event: "HTTP_RESPONSE".into(),
                    priority: 300
                },
                IrulesEventHandler {
                    event: "SERVER_CONNECTED".into(),
                    priority: 200
                },
            ]
        );
    }

    #[test]
    fn comments_and_nested_priority_commands_do_not_change_inheritance() {
        assert_eq!(
            extract("# priority 10\nwhen HTTP_REQUEST { priority 20 }\nwhen HTTP_RESPONSE { ok }"),
            vec![
                IrulesEventHandler {
                    event: "HTTP_REQUEST".into(),
                    priority: 500
                },
                IrulesEventHandler {
                    event: "HTTP_RESPONSE".into(),
                    priority: 500
                },
            ]
        );
    }

    #[test]
    fn body_literal_named_priority_is_not_an_inline_option() {
        assert_eq!(
            extract("when HTTP_REQUEST priority"),
            vec![IrulesEventHandler {
                event: "HTTP_REQUEST".into(),
                priority: 500,
            }]
        );
    }

    #[test]
    fn dynamic_compound_events_and_priorities_are_not_treated_as_literals() {
        assert_eq!(
            extract(
                "when HTTP_$suffix { dynamic-event }\n\
                 priority 2$zero\n\
                 when HTTP_REQUEST { unknown-inherited-priority }\n\
                 priority 200\n\
                 when HTTP_RESPONSE priority 3$zero { dynamic-priority }\n\
                 when SERVER_CONNECTED { literal }"
            ),
            vec![IrulesEventHandler {
                event: "SERVER_CONNECTED".into(),
                priority: 200,
            }]
        );
    }
}
