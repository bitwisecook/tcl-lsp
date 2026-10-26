// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected append data correspondence, independent of its effect receipt.

use super::{ModuleCommandBindings, SourceExecutionContext, SourceOutcomes};
use crate::command_binding::original_name_value::OriginalProducedNameValue;
use crate::var_resolve::OriginalNameValueRead;
use std::sync::Arc;
use tcl_syntax::native_string::NativeStringProtocol;

pub(super) struct CapturedAppendValue {
    receiver: OriginalNameValueRead,
    value: OriginalProducedNameValue,
    protocol: NativeStringProtocol,
}

impl CapturedAppendValue {
    // Called only within the same selected Native append capture, before its
    // unchanged preparation. This reads existing frozen values, never source
    // spelling or the string-effect receipt's owners as a value substitute.
    pub(super) fn capture(
        before: &ModuleCommandBindings,
        receiver: &crate::place::Place,
        context: SourceExecutionContext<'_>,
        from: usize,
        count: usize,
    ) -> Option<Self> {
        let protocol = before.baseline.dialect?.native_string_protocol()?;
        protocol.tcl_version()?;
        let receiver = before
            .source_variables
            .original_name_read_result(receiver, context.registry)?;
        if receiver.value().policy().string_protocol() != protocol {
            return None;
        }
        let inputs = context.written_name_values?;
        if inputs.len() != count.checked_add(1)? {
            return None;
        }
        let mut value = receiver.value().clone();
        for argument in from.checked_add(1)?..count {
            let input = inputs.get(argument.checked_add(1)?)?.as_deref()?;
            if !input.is_current(&before.source_variables)
                || input.policy().string_protocol() != protocol
            {
                return None;
            }
            // This central release-aware data recipe independently refuses
            // representation-sensitive non-ASCII/NUL cases. It grants no
            // Native object, getter, effect, String class or Normal outcome.
            value = value.concatenated(input)?;
        }
        Some(Self {
            receiver,
            value,
            protocol,
        })
    }

    pub(super) fn before_write_is_current(
        &self,
        state: &ModuleCommandBindings,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.receiver.is_current(&state.source_variables, registry)
            && self.value.is_current(&state.source_variables)
            && state
                .baseline
                .dialect
                .and_then(tcl_registry::InvocationDialect::native_string_protocol)
                == Some(self.protocol)
    }

    pub(super) fn publish(
        self,
        outcomes: &mut SourceOutcomes,
        before: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        after: &crate::place::Place,
    ) {
        let Some(normal) = &mut outcomes.normal else {
            return;
        };
        if normal.current_source_origin != before.current_source_origin
            || normal.original_command_world != before.original_command_world
            || normal.variable_frame != before.variable_frame
            || normal
                .baseline
                .dialect
                .and_then(tcl_registry::InvocationDialect::native_string_protocol)
                != Some(self.protocol)
            || !self.value.is_current(&normal.source_variables)
        {
            return;
        }
        let value = if let Some(old) = &outcomes.normal_name_value {
            let Some(joined) = old.joined(&self.value) else {
                return;
            };
            joined
        } else {
            self.value
        };
        if !value.is_current(&normal.source_variables) {
            return;
        }
        let Ok(text) = std::str::from_utf8(value.bytes()) else {
            return;
        };
        if outcomes
            .normal_value
            .as_ref()
            .is_some_and(|old| old.text != text)
        {
            return;
        }
        let Some(key) = crate::var_resolve::canonical_binding_value_key(after) else {
            return;
        };
        let variables = Arc::make_mut(&mut normal.source_variables);
        if !variables.retain_original_name_value(after, &value, context.registry) {
            return;
        }
        // The caller has already checked this same completed operation's
        // Normal WriteAt/source/current cell. This adds only its data facet;
        // it neither performs another store nor publishes representation.
        variables.constant_values.insert(key, text.to_owned());
        outcomes.normal_value.get_or_insert_with(|| {
            Arc::new(
                crate::command_binding::native_result::EvaluatedSourceValue {
                    text: text.to_owned(),
                    representation: tcl_syntax::value::ValueRepresentation::Unknown,
                    numeric: None,
                },
            )
        });
        outcomes.normal_name_value = Some(Arc::new(value));
    }
}
