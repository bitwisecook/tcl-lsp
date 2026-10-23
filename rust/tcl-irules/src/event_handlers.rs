// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-driven extraction of iRules event handlers and effective priorities.

use tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::OriginalIrulesSourceContext;

/// Conditional authored iRules handler and literal priority report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesEventHandler {
    /// Literal event written in the handler declaration.
    pub event: String,
    /// Literal inherited or per-handler priority.
    pub priority: u16,
}

/// Capture original hosted source before projecting handler priority advice.
#[must_use]
pub fn extract_irules_event_handlers(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesEventHandler> {
    OriginalIrulesSourceContext::capture(source, registry).map_or_else(Vec::new, |context| {
        extract_original_irules_event_handlers(source, &context)
    })
}

/// Project top-level handler reports from the same complete original source.
/// Dynamic setters and unavailable source selection withhold inherited priority;
/// a later literal setter restores it. Inline priority affects only its handler.
#[must_use]
pub fn extract_original_irules_event_handlers(
    source: &str,
    context: &OriginalIrulesSourceContext,
) -> Vec<IrulesEventHandler> {
    let mut inherited_priority = None;
    let mut inherited_unknown = false;
    let mut handlers = Vec::new();
    for (_, words) in context.top_level_source_vectors(source) {
        let Some(words) = words else {
            inherited_unknown = true;
            continue;
        };
        let Some((setter, policy, roles)) =
            words.with_source_schema(context.context_registry(), |schema| {
                (
                    schema
                        .semantics
                        .traits
                        .contains(Traits::SETS_EVENT_PRIORITY),
                    schema.authored_event_handler_priority(),
                    schema.arg_roles(),
                )
            })
        else {
            inherited_unknown = true;
            continue;
        };
        if setter {
            inherited_priority = literal_u16_arg(words, 0);
            inherited_unknown = inherited_priority.is_none();
            continue;
        }
        let Some(policy) = policy else {
            continue;
        };
        let Some(event) = literal_arg(words, 0) else {
            continue;
        };
        let Some(roles) = roles else {
            continue;
        };
        let body_index = roles
            .iter()
            .find_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
            .unwrap_or(words.arguments().len());
        let explicit = (1..body_index)
            .find(|&index| literal_arg(words, index) == Some(policy.keyword))
            .map(|index| literal_u16_arg(words, index + 1));
        let priority = explicit.unwrap_or_else(|| {
            (!inherited_unknown).then(|| inherited_priority.unwrap_or(policy.default_priority))
        });
        if let Some(priority) = priority {
            handlers.push(IrulesEventHandler {
                event: event.to_owned(),
                priority,
            });
        }
    }
    handlers
}

fn literal_u16_arg(words: &OriginalRegistryWords, index: usize) -> Option<u16> {
    literal_arg(words, index)?.parse().ok()
}

fn literal_arg(words: &OriginalRegistryWords, index: usize) -> Option<&str> {
    words.original_argument_word(index)?;
    std::str::from_utf8(words.arguments().get(index)?.literal_bytes()?).ok()
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
    #[test]
    fn original_source_priorities_decline_replaced_and_computed_commands() {
        for source in [
            "proc when args {}; when HTTP_REQUEST {}",
            "proc ::when args {}; ::when HTTP_REQUEST {}",
            "interp alias {} event {} when; event HTTP_REQUEST {}",
            "set command when; $command HTTP_REQUEST {}",
            "set tail en; \"wh${tail}\" HTTP_REQUEST {}",
        ] {
            assert!(extract(source).is_empty(), "{source}");
        }
        assert_eq!(
            extract("proc priority args {}; priority 100; when HTTP_REQUEST {}"),
            [] as [crate::event_handlers::IrulesEventHandler; 0]
        );
        assert_eq!(
            extract("proc priority args {}; priority 100; when HTTP_REQUEST priority 300 {}"),
            vec![IrulesEventHandler {
                event: "HTTP_REQUEST".into(),
                priority: 300
            }]
        );
    }

    #[test]
    fn original_source_priorities_keep_top_level_geometry_and_unknown_expansion() {
        assert_eq!(
            extract("set stored {priority 10; when HTTP_RESPONSE {}}; when HTTP_REQUEST {}"),
            vec![IrulesEventHandler {
                event: "HTTP_REQUEST".into(),
                priority: 500
            }]
        );
        assert_eq!(
            extract("proc helper {} {proc priority args {}; priority 10}; when HTTP_REQUEST {}"),
            vec![IrulesEventHandler {
                event: "HTTP_REQUEST".into(),
                priority: 500
            }]
        );
        assert_eq!(
            extract(
                "priority {*}$values; when HTTP_REQUEST {}; priority 200; when HTTP_RESPONSE {}"
            ),
            vec![IrulesEventHandler {
                event: "HTTP_RESPONSE".into(),
                priority: 200
            }]
        );
    }

    #[test]
    fn original_source_priorities_reject_changed_image_and_registry() {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "when HTTP_REQUEST {}";
        let context = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        assert_eq!(
            extract_original_irules_event_handlers("when HTTP_RESPONSE {}", &context),
            [] as [crate::event_handlers::IrulesEventHandler; 0]
        );
        let foreign = tcl_registry::model::ingress::static_context_for("f5-iapps");
        assert!(
            context
                .source_vectors()
                .iter()
                .all(|(_, words)| { words.with_source_schema(foreign, |_| ()).is_none() })
        );
    }
    #[test]
    fn original_source_inline_unknown_does_not_change_inherited_priority() {
        assert_eq!(
            extract("priority 100; when HTTP_REQUEST priority $value {}; when HTTP_RESPONSE {}"),
            vec![IrulesEventHandler {
                event: "HTTP_RESPONSE".into(),
                priority: 100
            }]
        );
        assert_eq!(
            extract(
                "priority 100; when HTTP_REQUEST priority {*}$values {}; when HTTP_RESPONSE {}"
            ),
            vec![IrulesEventHandler {
                event: "HTTP_RESPONSE".into(),
                priority: 100
            }]
        );
    }

    #[test]
    fn original_source_priority_context_declines_changed_lexer_configuration() {
        let source = "when HTTP_REQUEST {}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        assert!(OriginalIrulesSourceContext::from_source_analysis(source, &analysis).is_some());
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax ^= true;
        assert!(OriginalIrulesSourceContext::from_source_analysis(source, &analysis).is_none());
    }
}
