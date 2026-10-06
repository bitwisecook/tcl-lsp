// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Normal conversion of an independently proved ordinary container operand.

use super::{
    ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
};
use crate::{native_numeric::ClosedContainerRepresentations, place::Place};
use tcl_registry::representation::{OrdinaryContainerCoercion, OrdinaryContainerRepresentation};
use tcl_syntax::value::ValueRepresentation;

pub(super) fn target(coercion: OrdinaryContainerCoercion) -> ValueRepresentation {
    match coercion.target() {
        OrdinaryContainerRepresentation::List => ValueRepresentation::List,
        OrdinaryContainerRepresentation::Dictionary => ValueRepresentation::Dict,
    }
}

pub(super) struct CapturedContainerCoercion {
    receiver: Place,
    origin: crate::var_resolve::ContentsOrigin,
    source: Option<super::Arc<super::SourceOriginId>>,
    previous: Option<ClosedContainerRepresentations>,
    value: crate::native_numeric::StoredNativeRepresentation,
    stock_completion: Option<tcl_registry::completion_route::InvocationCompletionRoute>,
}

pub(super) fn capture(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<CapturedContainerCoercion> {
    if let Some(stock) = capture_stock_length(native, facts, state, context) {
        return Some(stock);
    }
    context.written_representations?;
    let arguments = native.invocation.arguments();
    let coercion = facts
        .representation_effect
        .successful_ordinary_container_coercion(arguments, facts.argument_offset)?;
    let receiver = capture_receiver(native, coercion.argument(), state, context)?;
    let variables = &state.source_variables;
    let previous = variables.container_representation_alternatives_at(&receiver)?;
    let target = target(coercion);
    let may_preserve = coercion
        .may_preserve_input_for_bytes(arguments.literal_at(coercion.argument()).map(str::as_bytes));
    let value = if let Some(object) =
        variables.contents_stock_literal_object_at(&receiver, context.registry)
    {
        crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(if may_preserve {
            object.possibly_converted(target)
        } else {
            object.converted(target)
        })
    } else if may_preserve {
        previous
            .joined(ClosedContainerRepresentations::of(target)?)
            .stored()
    } else {
        target.into()
    };
    Some(CapturedContainerCoercion {
        origin: variables.contents_origin(&receiver),
        source: variables.contents_source(&receiver).cloned(),
        receiver,
        previous: Some(previous),
        value,
        stock_completion: None,
    })
}

fn capture_receiver(
    native: SourceNativeInvocation<'_>,
    argument: usize,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Place> {
    // Enter observers or later substitutions may replace the frozen object.
    context.written_representations?;
    if native.invocation.arguments().exact_argv_len()? != argument.checked_add(1)? {
        return None;
    }
    let receiver = super::argument_reads::current_argument_place(native, argument, state, context)?;
    state
        .source_variables
        .read_produces_value(receiver, context.registry)
        .then(|| receiver.clone())
}

/// Stock hooks, actual cache class and successful conversion are separate
/// receipts. In particular Tcl 9 numeric Length preserves its original cache.
fn capture_stock_length(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<CapturedContainerCoercion> {
    use crate::native_numeric::StoredNativeRepresentation as Stored;
    use tcl_registry::native_stock_list::{
        NativeStockListCacheDisposition as Cache, NativeStockListInputClass as Class,
    };
    let arguments = native.invocation.arguments();
    let protocol = facts.stock_list_length_protocol(arguments)?;
    let receiver = capture_receiver(native, protocol.argument(), state, context)?;
    let variables = &state.source_variables;
    let key = crate::var_resolve::canonical_binding_value_key(&receiver)?;
    let stock = variables.contents_stock_literal_object_at(&receiver, context.registry);
    let numeric = variables
        .contents_already_native_numeric_at(&receiver, context.registry)
        .then(|| variables.value_representations.get(&key)?.numeric_shape())
        .flatten();
    let root_provider = variables
        .contents_list_method_provider_at(&receiver, context.registry)
        .filter(|provider| {
            *provider == tcl_registry::native_result::NativeListMethodProvider::EmptyListRoot
        });
    if stock.is_none() && numeric.is_none() && root_provider.is_none() {
        return None;
    }
    let class = if numeric.is_some() {
        Class::Numeric
    } else {
        match variables.contents_representation_at(&receiver) {
            ValueRepresentation::String => Class::String,
            ValueRepresentation::List => Class::List,
            ValueRepresentation::Dict => Class::Dictionary,
            ValueRepresentation::Unknown => Class::Unknown,
        }
    };
    let original = if let Some(object) = stock {
        Stored::StockLiteralObject(object)
    } else if let Some(shape) = numeric {
        Stored::NumericShape(shape)
    } else {
        Stored::ReadOnlyListMethodProvider(root_provider?)
    };
    let disposition = protocol.normal_cache_disposition(
        class,
        arguments.literal_at(protocol.argument()).map(str::as_bytes),
    )?;
    let value = match disposition {
        Cache::Preserved => original,
        Cache::List => stock.map_or_else(
            || ValueRepresentation::List.into(),
            |object| Stored::StockLiteralObject(object.converted(ValueRepresentation::List)),
        ),
        Cache::Unknown => stock.map_or_else(
            || {
                root_provider.map_or_else(
                    || ValueRepresentation::Unknown.into(),
                    Stored::ReadOnlyListMethodProvider,
                )
            },
            |object| Stored::StockLiteralObject(object.converted(ValueRepresentation::Unknown)),
        ),
    };
    Some(CapturedContainerCoercion {
        origin: variables.contents_origin(&receiver),
        source: variables.contents_source(&receiver).cloned(),
        receiver,
        previous: None,
        value,
        stock_completion: Some(protocol.completion()),
    })
}

/// Completion uses the same current original-operand witness as normal
/// conversion. A closed ordinary domain bounds codes without proving success.
pub(super) fn ordinary_list_length_completion(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<tcl_registry::completion_route::InvocationCompletionRoute> {
    if !matches!(
        facts.representation_effect,
        tcl_registry::representation::RepresentationEffect::CoerceOrdinaryList { .. }
    ) {
        return None;
    }
    let captured = capture(native, facts, state, context)?;
    if let Some(route) = captured.stock_completion {
        return Some(route);
    }
    let previous = captured.previous?;
    let arguments = native.invocation.arguments();
    let selected = facts
        .representation_effect
        .successful_ordinary_container_coercion(arguments, facts.argument_offset)?;
    let mut consensus = None;
    for (representation, ordinary) in [
        (
            ValueRepresentation::List,
            OrdinaryContainerRepresentation::List,
        ),
        (
            ValueRepresentation::Dict,
            OrdinaryContainerRepresentation::Dictionary,
        ),
    ] {
        if previous.contains(representation) {
            let route =
                facts.ordinary_list_length_completion(arguments, selected.argument(), ordinary)?;
            if consensus.is_some_and(|previous| previous != route) {
                return None;
            }
            consensus = Some(route);
        }
    }
    consensus
}

pub(super) fn finish(
    outcomes: &mut SourceOutcomes,
    captured: Option<CapturedContainerCoercion>,
    context: SourceExecutionContext<'_>,
) {
    let Some(captured) = captured else { return };
    let Some(normal) = &mut outcomes.normal else {
        return;
    };
    let variables = &normal.source_variables;
    let receiver = &captured.receiver;
    if !variables.read_produces_value(receiver, context.registry)
        || variables.contents_origin(receiver) != captured.origin
        || variables.contents_source(receiver) != captured.source.as_ref()
    {
        return;
    }
    let Some(key) = crate::var_resolve::canonical_binding_value_key(receiver) else {
        return;
    };
    super::Arc::make_mut(&mut normal.source_variables)
        .value_representations
        .insert(key, captured.value);
}
