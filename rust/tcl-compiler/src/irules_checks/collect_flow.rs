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

//! IRULE1005 to IRULE1008: collect, release and payload pairing across events and connection sides.

use super::{
    CollectFlowState, CollectionCall, CollectionSourceObligationKind,
    ConditionalCollectionSourcePairing, ConditionalCollectionSourcePairingAdvice,
    ConditionalIruleSourceOwner, ConditionalSourcePairingAdvice, IrulesCheckPayload,
    IrulesCheckWarning, IrulesSourceQuery, PayloadCall, conditional_source_pairing_message,
    is_irules,
};
use crate::cfg_builder::build_cfg_with_registry_and_config;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::lowering::lower_to_ir_with_config;
use crate::sccp::cfg_order;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::events::{
    CollectionReleaseRequirement, DataCollectionAction, EventRegistry, PayloadCollectionRequirement,
};
use tcl_registry::side_effects::ConnectionSide;

/// Record a registry-declared data-collection operation.
///
/// This deliberately does not infer semantics from a `::payload` suffix:
/// `UDP::payload` and `ASM::payload` are immediately available, while TCP,
/// HTTP, and SSL payloads require collection.  New protocol commands join the
/// analysis by declaring [`tcl_registry::DataCollectionOperation`] in their
/// command specs.
fn classify_collect_operation(
    operation: tcl_registry::events::DataCollectionOperation,
    args: &[String],
    event: &str,
    side: &str,
    span: Span,
    state: &mut CollectFlowState,
) {
    let arg_words: Vec<&str> = args.iter().map(String::as_str).collect();
    let protocol = operation.protocol.name.to_string();
    match operation.action {
        DataCollectionAction::Collect => {
            state
                .collected
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.collect_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Release => {
            state
                .released
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.release_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Payload => {
            let Some(collection_requirement) = operation.payload_requirement_for_args(&arg_words)
            else {
                // Dynamic/unknown call form: it may select a payload form
                // whose availability differs, so abstain rather than guess.
                return;
            };
            state.payload_calls.push(PayloadCall {
                protocol,
                side: side.to_string(),
                span,
                collection_requirement,
            });
        }
    }
}

fn scan_when_body_for_collect_flow(
    fu: &FunctionUnit,
    event: &str,
    side: &str,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
) -> Option<()> {
    let config = source.function_config(fu)?;
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(
                stmt,
                CollectionEventSide { event, side },
                state,
                source,
                fu,
                config,
                fu.base_offset,
            )?;
        }
    }
    Some(())
}

fn retain_collection_source_obligations(
    possible: &crate::registry_invocation::RegistryInvocationAssistance,
    state: &mut CollectFlowState,
) {
    if possible.unknown_residual {
        state
            .obligations
            .insert(CollectionSourceObligationKind::UnknownTarget);
    }
    if possible.may_be_absent {
        state
            .obligations
            .insert(CollectionSourceObligationKind::MayBeAbsent);
    }
}

#[derive(Clone, Copy)]
struct CollectionEventSide<'a> {
    event: &'a str,
    side: &'a str,
}

/// Follow registry-declared side transitions with each original child owner.
/// Missing supplied child source withdraws the whole pairing query: it is not
/// evidence that a collect or release is absent.
fn scan_selected_collection_side_body(
    parent: &crate::ir::CommandTokens,
    written: usize,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide {
        event,
        side: inner_side,
    } = event_side;
    let registry = source.registry;
    if source.standalone {
        let binding = parent.source_binding.as_ref()?;
        let image = binding.invocation_site()?.source.source_image();
        let parent_config = binding.original_lexer_config_for_tokens(parent)?;
        let words = crate::registry_invocation::original_native_compiler_words(
            image,
            parent.words(),
            parent.argv.first()?.start(),
            parent_config,
        )?;
        let word = words.get(written.checked_add(1)?)?;
        let content = word.content_span().ok()?;
        let body = std::str::from_utf8(image.bytes().get(content.as_range())?).ok()?;
        scan_standalone_side_switch_body(
            body,
            CollectionEventSide {
                event,
                side: inner_side,
            },
            state,
            source,
            original_function,
            word.config(),
            base_offset + i64::from(content.start()),
        )?;
    } else {
        let body_unit = source.unit.conditional_original_body_unit(
            registry,
            original_function,
            parent,
            written,
        )?;
        scan_secondary_collection_body(
            &body_unit,
            CollectionEventSide {
                event,
                side: inner_side,
            },
            state,
            source,
            original_function,
            body_unit.source_lexer_config(),
            0,
        )?;
    }
    Some(())
}

fn classify_stmt_for_collect_flow(
    stmt: &Statement,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    let registry = source.registry;
    if let crate::ir::OriginalArgvInventoryDisposition::EffectOnly(marker) =
        stmt.original_argv_inventory_disposition()
    {
        state
            .obligations
            .insert(CollectionSourceObligationKind::EffectOnly(marker));
        return Some(());
    }
    if let Some(tokens) = stmt.tokens() {
        tokens
            .source_binding
            .as_ref()?
            .original_lexer_config_for_tokens(tokens)?;
        scan_nested_collection_words(
            tokens,
            CollectionEventSide { event, side },
            state,
            source,
            original_function.body_id,
            config,
            base_offset,
        )?;
    }
    let abs = |sp: Span| -> Span {
        if base_offset == 0 {
            return sp;
        }
        let s = (i64::from(sp.start()) + base_offset).max(0);
        let e = (i64::from(sp.end()) + base_offset).max(0);
        Span::new(
            u32::try_from(s).unwrap_or(u32::MAX),
            u32::try_from(e).unwrap_or(u32::MAX),
        )
    };
    let (span, parent) = match stmt {
        Statement::Call { span, tokens, .. } | Statement::Barrier { span, tokens, .. } => {
            (*span, tokens.as_ref()?)
        }
        Statement::AssignValue { tokens: None, .. } => return None,
        _ => return Some(()),
    };
    let possible =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            source.metadata(),
            parent,
        )?;
    retain_collection_source_obligations(&possible, state);
    for (ordinal, shape) in possible.candidates.iter().enumerate() {
        if possible.candidates[..ordinal].contains(shape) {
            continue;
        }
        if let Some(target) = shape.side_switch_target {
            if !shape.roles_complete {
                return None;
            }
            let mut bodies = shape.roles.iter().filter_map(|&(argument, role)| {
                (role == tcl_registry::ArgRole::Body)
                    .then_some(shape.argument_offset + usize::from(argument))
            });
            let effective = bodies.next()?;
            if bodies.next().is_some() {
                return None;
            }
            let written = shape.effective.written_argument(effective)?;
            if !parent.arg_is_braced_literal(written) {
                return None;
            }
            let inner_side = target.execution_side(side);
            scan_selected_collection_side_body(
                parent,
                written,
                CollectionEventSide {
                    event,
                    side: inner_side,
                },
                state,
                source,
                original_function,
                base_offset,
            )?;
        } else if let Some(operation) = shape.data_collection {
            let Some(arguments) = collection_source_arguments(parent, shape, operation, source)
            else {
                continue;
            };
            classify_collect_operation(operation, &arguments, event, side, abs(span), state);
        }
    }
    Some(())
}

/// Nested argv retains the function's full policy while the original binding
/// may select its execution grammar. Metadata and dispatch/effect admissions
/// remain independent of this syntax purpose.
fn scan_nested_collection_words(
    parent: &crate::ir::CommandTokens,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    body: Option<crate::ir::ProcedureId>,
    parent_config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    let children = source.argument_calls(parent, parent_config)?;
    if !source.standalone {
        state
            .source_argv
            .push((body?, source.original_argv(parent)?));
    }
    for call in children {
        let tokens = call.tokens.as_ref()?;
        if !source.standalone {
            state
                .source_argv
                .push((body?, source.original_argv(tokens)?));
        }
        let possible =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        retain_collection_source_obligations(&possible, state);
        for (ordinal, shape) in possible.candidates.iter().enumerate() {
            if possible.candidates[..ordinal].contains(shape) {
                continue;
            }
            let Some(operation) = shape.data_collection else {
                continue;
            };
            let Some(arguments) = collection_source_arguments(tokens, shape, operation, source)
            else {
                continue;
            };
            let span = Span::new(
                u32::try_from((base_offset + i64::from(tokens.argv.first()?.start())).max(0))
                    .unwrap_or(u32::MAX),
                u32::try_from((base_offset + i64::from(tokens.argv.last()?.end())).max(0))
                    .unwrap_or(u32::MAX),
            );
            classify_collect_operation(operation, &arguments, event, side, span, state);
        }
    }
    Some(())
}

fn collection_source_arguments(
    tokens: &crate::ir::CommandTokens,
    shape: &crate::registry_invocation::RegistryInvocationShape,
    operation: tcl_registry::events::DataCollectionOperation,
    source: &IrulesSourceQuery<'_>,
) -> Option<Vec<String>> {
    if operation.action != DataCollectionAction::Payload {
        return Some(Vec::new());
    }
    shape
        .source_argument_words(source.registry, source.metadata(), tokens)?
        .iter()
        .map(|word| Some(std::str::from_utf8(word.literal_bytes()?).ok()?.to_owned()))
        .collect()
}

/// Explicit standalone assistance is a separate fresh body ingress. It retains
/// the caller's complete config and never substitutes a profile's grammar.
fn scan_standalone_side_switch_body(
    body_text: &str,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    parent_config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    if !source.standalone {
        return None;
    }
    let config = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(parent_config);
    let module = lower_to_ir_with_config(body_text, source.registry, config);
    let cfg_module = build_cfg_with_registry_and_config(&module, false, source.registry, config);
    let unit = FunctionUnit::build(
        "::side_switch",
        cfg_module.top_level,
        &[],
        source.registry,
        config,
    );
    scan_secondary_collection_body(
        &unit,
        CollectionEventSide { event, side },
        state,
        source,
        original_function,
        config,
        base_offset,
    )
}

fn scan_secondary_collection_body(
    unit: &FunctionUnit,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    for bn in cfg_order(&unit.cfg) {
        if !unit.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = unit.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(
                stmt,
                CollectionEventSide { event, side },
                state,
                source,
                original_function,
                config,
                base_offset,
            )?;
        }
    }
    Some(())
}

/// Definite collect/release diagnostics require an independent runtime lifecycle owner.
/// Retained source inventories alone cannot supply that absence proof.
#[must_use]
pub fn find_collect_flow_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    if !is_irules(dialect) {
        return Vec::new();
    }
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return Vec::new();
    };
    let Some((state, _)) = collect_flow_state(&source) else {
        return Vec::new();
    };
    if !state.obligations.is_empty() {
        return Vec::new();
    }
    // This source-only interface has no complete runtime collection owner.
    Vec::new()
}

/// Conditional source pairing under genuine full input and handler-body owners.
/// Missing lexical phases or unsupported Body/Expr/callback inventories decline.
#[must_use]
pub fn find_conditional_collection_source_pairing_advice<'unit>(
    cu: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ConditionalCollectionSourcePairingAdvice<'unit>> {
    if !is_irules(dialect) {
        return None;
    }
    let source = IrulesSourceQuery::capture(cu, registry)?;
    let (state, events_seen) = collect_flow_state(&source)?;
    let advice = source.conditional_pairing_advice(&state)?;
    let events = EventRegistry::build();
    let mut out = Vec::new();
    emit_missing_data_collect_warnings(
        cu,
        registry,
        &events,
        &events_seen,
        &state,
        &advice,
        &mut out,
    );
    emit_payload_without_collect_warnings(&state, &advice, &mut out);
    emit_collect_without_release_warnings(&events, &events_seen, &state, &advice, &mut out);
    emit_release_without_collect_warnings(&state, &advice, &mut out);
    let mut obligations = state.obligations.clone();
    obligations.insert(CollectionSourceObligationKind::EventActivation);
    obligations.insert(CollectionSourceObligationKind::SourceSchemaApplicability);
    Some(ConditionalCollectionSourcePairingAdvice {
        owner: ConditionalIruleSourceOwner {
            unit: cu,
            registry,
            _handlers: advice.handlers,
            _schemas: advice.schemas,
            obligations,
            _completions: advice.completions,
            flow_commands: advice.flow_commands,
        },
        pairings: out
            .into_iter()
            .map(|warning| ConditionalCollectionSourcePairing { warning })
            .collect(),
    })
}

/// Existing event-table side selection remains independent of body admission.
pub(super) fn collect_flow_state(
    source: &IrulesSourceQuery<'_>,
) -> Option<(CollectFlowState, Vec<String>)> {
    let mut state = CollectFlowState::default();
    let mut events_seen = Vec::new();
    let events = EventRegistry::build();
    for fu in source.unit.analysable_functions() {
        let tcl_core_types::Knowledge::Proven(event) = source.event(fu)? else {
            continue;
        };
        let event = event.body.event();
        events_seen.push(event.to_owned());
        let Some(side) = events
            .get_props(event)
            .and_then(tcl_registry::events::EventProps::command_side)
            .and_then(|side| match side {
                ConnectionSide::Client => Some("client"),
                ConnectionSide::Server => Some("server"),
                _ => None,
            })
        else {
            retain_handler_source_vectors(fu, &mut state, source)?;
            continue;
        };
        scan_when_body_for_collect_flow(fu, event, side, &mut state, source)?;
    }
    Some((state, events_seen))
}

/// An event with no single connection side has no collection pairing to
/// scan, but its handler still owns its source vectors: the flow advice
/// accepts a handler's commands only from that census.
fn retain_handler_source_vectors(
    fu: &FunctionUnit,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
) -> Option<()> {
    if source.standalone {
        return Some(());
    }
    let config = source.function_config(fu)?;
    let body = fu.body_id?;
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            if matches!(
                stmt.original_argv_inventory_disposition(),
                crate::ir::OriginalArgvInventoryDisposition::EffectOnly(_)
            ) {
                continue;
            }
            let Some(tokens) = stmt.tokens() else {
                continue;
            };
            tokens
                .source_binding
                .as_ref()?
                .original_lexer_config_for_tokens(tokens)?;
            let children = source.argument_calls(tokens, config)?;
            state
                .source_argv
                .push((body, source.original_argv(tokens)?));
            for call in children {
                state
                    .source_argv
                    .push((body, source.original_argv(call.tokens.as_ref()?)?));
            }
        }
    }
    Some(())
}

/// Project missing source collection candidates within the certified handler census.
fn emit_missing_data_collect_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    // The event table owns protocol/side requirements; the retained declaration
    // owns its original event operand. Body statements supply no header geometry.
    let mut emitted_events: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for event in events_seen {
        let Some((protocols, required_side)) = events.data_collect_requirement(event) else {
            continue;
        };
        if !emitted_events.insert(event.as_str()) {
            continue;
        }
        let satisfied = protocols.iter().any(|protocol_name| {
            let Some(protocol) = registry.data_collection_protocol(protocol_name) else {
                // The event table must not be treated as proof for a protocol
                // whose command surface is absent from this registry profile.
                return false;
            };
            protocol.payload_requirement == PayloadCollectionRequirement::NotRequired
                || state
                    .collected
                    .get(protocol.name)
                    .is_some_and(|s| s.contains(required_side))
        });
        if satisfied {
            continue;
        }
        let proto_hint: Vec<&str> = protocols
            .iter()
            .filter_map(|protocol| registry.data_collection_collect_command(protocol))
            .map(|spec| spec.name)
            .collect();
        if proto_hint.is_empty() {
            // The registry says each applicable protocol supplies payload
            // without an explicit collect, so the source cannot be proved to
            // miss one. This is intentionally an abstention, not a warning.
            continue;
        }
        let Some(span) = event_anchor_span(cu, registry, event) else {
            continue;
        };
        out.push(IrulesCheckWarning {
            span,
            code: DiagCode::Irule1005,
            message: conditional_source_pairing_message(advice, &format!(
                "'{event}' has no matching source {required_side} {} candidate in the retained handlers.",
                proto_hint.join(" or "),
            )),
            payload: IrulesCheckPayload::EventWithoutCollect {
                event: event.clone(),
                side: required_side.to_string(),
                collect_commands: proto_hint.iter().map(|name| (*name).to_owned()).collect(),
            },
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Original event operand from the authentic selected declaration generation.
pub(super) fn event_anchor_span(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    event: &str,
) -> Option<Span> {
    let source = IrulesSourceQuery::capture(cu, registry)?;
    source
        .unit
        .functions()
        .filter_map(|fu| {
            source
                .event(fu)
                .and_then(tcl_core_types::Knowledge::into_proven)
        })
        .filter(|owned| owned.body.event() == event)
        .filter_map(|owned| {
            owned
                .body
                .event_word(&cu.ir_module.source, cu.ir_module.lexer_config, registry)
        })
        .map(tcl_lexer::NativeWord::span)
        .min_by_key(|span| span.start())
}

/// Emit IRULE1006 for payload commands that explicitly require collection.
fn emit_payload_without_collect_warnings(
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for payload in &state.payload_calls {
        let matched = state
            .collected
            .get(&payload.protocol)
            .is_some_and(|s| s.contains(&payload.side));
        if payload.collection_requirement == PayloadCollectionRequirement::ExplicitCollect
            && !matched
        {
            out.push(IrulesCheckWarning {
                span: payload.span,
                code: DiagCode::Irule1006,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible '{}::payload' without a matching source {} {}::collect candidate.",
                    payload.protocol, payload.side, payload.protocol,
                )),
                payload: IrulesCheckPayload::PayloadWithoutCollect {
                    protocol: payload.protocol.clone(),
                    side: payload.side.clone(),
                },
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Emit IRULE1007 for registered collection lifecycles not released by code or event.
fn emit_collect_without_release_warnings(
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for collect in &state.collect_calls {
        let needs_release =
            collect.release_requirement != CollectionReleaseRequirement::NotApplicable;
        let released = state
            .released
            .get(&collect.protocol)
            .is_some_and(|s| s.contains(&collect.side));
        let implicitly_released = collect.release_requirement
            == CollectionReleaseRequirement::ImplicitInDataEvent
            && events_seen.iter().any(|event| {
                let side = match collect.side.as_str() {
                    "client" => ConnectionSide::Client,
                    "server" => ConnectionSide::Server,
                    _ => return false,
                };
                events.data_event_releases_collection(
                    event,
                    &collect.event,
                    &collect.protocol,
                    side,
                )
            });
        if needs_release && !released && !implicitly_released {
            out.push(IrulesCheckWarning {
                span: collect.span,
                code: DiagCode::Irule1007,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible {}::collect without a matching source {}::release candidate on the {} side.",
                    collect.protocol, collect.protocol, collect.side,
                )),
                payload: IrulesCheckPayload::CollectWithoutRelease {
                    protocol: collect.protocol.clone(),
                    side: collect.side.clone(),
                },
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Project source release candidates without a matching source collection.
fn emit_release_without_collect_warnings(
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for release in &state.release_calls {
        let matched = state
            .collected
            .get(&release.protocol)
            .is_some_and(|s| s.contains(&release.side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: release.span,
                code: DiagCode::Irule1008,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible {}::release without a matching source {}::collect candidate on the {} side.",
                    release.protocol, release.protocol, release.side,
                )),
                payload: IrulesCheckPayload::ReleaseWithoutCollect {
                    protocol: release.protocol.clone(),
                    side: release.side.clone(),
                },
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}
