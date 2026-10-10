//! Presentation projections of the shared conditional iRules source context.

#[cfg(feature = "test-instrumentation")]
use std::cell::Cell;
use std::collections::{HashSet, VecDeque};
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventEmissionCertainty;

/// Presentation-only source-candidate fields. Command/argument/event strings
/// grant no identity, runtime effects, entered frame or worker reachability.
/// Semantic consumers use `OriginalIrulesSourceContext`'s sealed words instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesExecutableCommand {
    /// Actual independently retained source command extent.
    pub span: tcl_lexer::Span,
    /// Selected source-schema label, without a live handler grant.
    pub command: String,
    /// Original literal values or written unknown operands for display.
    pub args: Vec<String>,
    /// Source variable-token labels, without cell or frame authority.
    pub variable_names: Vec<String>,
    /// Actual event-body source label, without event reachability.
    pub event: Option<String>,
}

#[cfg(feature = "test-instrumentation")]
thread_local! {
    static EXECUTABLE_CLOSURE_BUILDS: Cell<usize> = const { Cell::new(0) };
}
/// Reset this thread's source-closure capture count.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
pub fn reset_executable_closure_builds_for_tests() {
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(0));
}
/// Number of whole source-context captures at this compatibility ingress.
#[cfg(feature = "test-instrumentation")]
#[doc(hidden)]
#[must_use]
pub fn executable_closure_builds_for_tests() -> usize {
    EXECUTABLE_CLOSURE_BUILDS.with(Cell::get)
}

pub(crate) fn record_source_capture() {
    #[cfg(feature = "test-instrumentation")]
    EXECUTABLE_CLOSURE_BUILDS.with(|builds| builds.set(builds.get() + 1));
}

/// Conditional event-rooted source candidates. Applicability and procedure
/// binding remain unproved; this compatibility return is presentation only.
#[must_use]
pub fn irules_executable_commands(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    crate::OriginalIrulesSourceContext::capture(source, registry)
        .map_or_else(Vec::new, |context| context.presentation_commands(source))
}

/// Source candidates associated with an actual retained event-body descriptor.
/// An event label is a readonly filter, not worker or handler entry authority.
#[must_use]
pub fn irules_event_executable_closure(
    source: &str,
    event: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    irules_executable_commands(source, registry)
        .into_iter()
        .filter(|command| {
            command
                .event
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(event))
        })
        .collect()
}

/// Possible authored event-emission relation in the current source schema.
/// The Registry certainty describes the selected schema, without execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesEventEmissionEdge {
    /// Actual emitting source command extent.
    pub span: tcl_lexer::Span,
    /// Source event-body descriptor label.
    pub from_event: String,
    /// Actual selected source-schema label.
    pub command: String,
    /// Possible target event from the authored Registry schema.
    pub to_event: &'static str,
    /// Schema certainty, without observed execution.
    pub certainty: EventEmissionCertainty,
}

fn source_emission_edges(
    context: &crate::OriginalIrulesSourceContext,
) -> Vec<IrulesEventEmissionEdge> {
    let actual = context.context_registry();
    let mut out = Vec::new();
    for command in context.commands() {
        let Some(arguments) = command
            .words()
            .arguments()
            .iter()
            .map(|word| std::str::from_utf8(word.literal_bytes()?).ok())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let emission = command
            .words()
            .with_source_schema(actual, |selected| {
                if !selected
                    .argument_count_for_arity()
                    .is_some_and(|count| selected.semantics.arity.accepts(count))
                {
                    return None;
                }
                actual
                    .commands()
                    .get_exact(selected.canonical_command)?
                    .event_emission_for_args(&arguments)
            })
            .flatten();
        let Some(emission) = emission else {
            continue;
        };
        for to_event in emission.events {
            out.push(IrulesEventEmissionEdge {
                span: command.span(),
                from_event: command.event_source().event().to_owned(),
                command: command.words().command().to_owned(),
                to_event,
                certainty: emission.certainty,
            });
        }
    }
    out
}

/// Current conditional event-emission source relations, through authentic argv.
#[must_use]
pub fn irules_event_emission_edges(
    source: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesEventEmissionEdge> {
    crate::OriginalIrulesSourceContext::capture(source, registry)
        .map_or_else(Vec::new, |context| source_emission_edges(&context))
}

/// Conditional source paths widened by actual authored event-emission schema.
/// This union supplies neither an entered event nor complete runtime coverage.
#[must_use]
pub fn irules_event_reachable_closure(
    source: &str,
    event: &str,
    registry: &CommandRegistry,
) -> Vec<IrulesExecutableCommand> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let edges = source_emission_edges(&context);
    let mut pending = VecDeque::from([event.to_owned()]);
    let mut visited = HashSet::new();
    while let Some(event) = pending.pop_front() {
        if !visited.insert(event.to_ascii_uppercase()) {
            continue;
        }
        pending.extend(
            edges
                .iter()
                .filter(|edge| edge.from_event.eq_ignore_ascii_case(&event))
                .map(|edge| edge.to_event.to_owned()),
        );
    }
    context
        .presentation_commands(source)
        .into_iter()
        .filter(|command| {
            command
                .event
                .as_deref()
                .is_some_and(|event| visited.contains(&event.to_ascii_uppercase()))
        })
        .collect()
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
        assert_eq!(pools, ["from_proc", "from_event"]);
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
            ["helper_one", "helper_two", "first_pool", "second_pool"],
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
        assert_eq!(pools, ["valid_proc", "valid_event"]);
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
        assert_eq!(pools, ["leaf", "cycle_a", "cycle_b", "event"]);
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
    fn source_calls_do_not_invent_global_marker_equivalence() {
        // Implementation contract: naming.consumer.original-irules-source-context
        // docs/design/analysis/name-resolution-proofs/original-irules-source-context.md
        let source = "proc ::helper {} { pool rooted_helper }\nwhen HTTP_REQUEST { call helper }\n";
        let facts = commands(source);
        assert!(facts.iter().all(|fact| fact.command != "pool"));
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let context = crate::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        assert!(context.commands().iter().any(|command| {
            command
                .obligations()
                .contains(&crate::IrulesSourceObligation::ProcedureTargetUnavailable)
        }));
    }
}
