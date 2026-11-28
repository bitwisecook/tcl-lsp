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

#[cfg(test)]
mod tests {
    use tcl_irules::*;

    fn extract(source: &str) -> Vec<IrulesEventHandler> {
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        extract_irules_event_handlers(source, registry)
    }

    #[test]
    fn top_level_priority_affects_only_subsequent_handlers() {
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
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
            [] as [tcl_irules::IrulesEventHandler; 0]
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "when HTTP_REQUEST {}";
        let context = OriginalIrulesSourceContext::capture(source, registry).unwrap();
        assert_eq!(
            extract_original_irules_event_handlers("when HTTP_RESPONSE {}", &context),
            [] as [tcl_irules::IrulesEventHandler; 0]
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
        crate::support::install_surfaces();
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
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        assert!(OriginalIrulesSourceContext::from_source_analysis(source, &analysis).is_some());
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax ^= true;
        assert!(OriginalIrulesSourceContext::from_source_analysis(source, &analysis).is_none());
    }
}
