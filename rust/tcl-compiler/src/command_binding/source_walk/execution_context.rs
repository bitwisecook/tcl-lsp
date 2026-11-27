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

//! The execution context of a walk and the derived contexts built outside recursive frames.

use super::super::compiled_invocation::NativeCompilationSnapshot;
use super::super::variable_access::SourceVariableEvaluationOwner;
use super::super::{SourceArgumentReceipts, SourceExecutionContext, original_name_value};
use tcl_names::namespace::SourceNamespaceKey;

/// Source selection is independent of an actual entered-body observation.
/// Conditional descendants retain their selected source and analysis frame.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum SourceBodyEntryPurpose {
    Selected,
    Conditional,
}

impl<'a> SourceExecutionContext<'a> {
    pub(in super::super) fn compilation(
        &self,
    ) -> tcl_registry::native_compilation::NativeCompilationContext {
        self.control.compilation()
    }
    pub(in super::super) fn source_purpose(&self) -> tcl_lexer::SourceSegmentationPurpose {
        self.control.source_purpose()
    }
    pub(in super::super) fn body_entry(&self) -> SourceBodyEntryPurpose {
        self.control.body_entry()
    }
    pub(in super::super) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.control.realm()
    }
    pub(in super::super) fn conditional(self) -> Self {
        Self {
            control: self
                .control
                .with_body_entry(SourceBodyEntryPurpose::Conditional),
            ..self
        }
    }

    pub(in super::super) fn entered_frame(
        &self,
    ) -> Option<&'a tcl_names::variable::VariableExecutionFrame> {
        (self.body_entry() == SourceBodyEntryPurpose::Selected).then_some(self.frame)
    }

    pub(super) fn withdraw_observed_operand_facts(&mut self, observed: bool) {
        if observed {
            self.written_method_prefixes = None;
            self.written_name_values = None;
            self.written_representations = None;
            self.written_variable_reads = None;
        }
    }

    pub(in super::super) fn namespace_identity(&self) -> SourceNamespaceKey {
        self.namespace_key
            .cloned()
            .unwrap_or_else(|| SourceNamespaceKey::authored(self.namespace))
    }
}

/// The evaluator owns the updated original facade; transient Copy views
/// borrow it for no longer than this holder. No view points at a helper local.
pub(in super::super) struct SourceArgumentExecutionContext<'a> {
    pub(in super::super) context: SourceExecutionContext<'a>,
    pub(in super::super) compilation:
        Option<original_name_value::OriginalSourceVariableCompilation<'a>>,
}

impl SourceArgumentExecutionContext<'_> {
    pub(in super::super) fn context(&self) -> SourceExecutionContext<'_> {
        SourceExecutionContext {
            original_variable_compilation: self.compilation.as_ref(),
            ..self.context
        }
    }
}

/// Capture the original argument receipts without retaining construction
/// temporaries in the recursive command driver.
#[inline(never)]
pub(in super::super) fn source_argument_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    receipts: SourceArgumentReceipts<'a>,
) -> Box<SourceArgumentExecutionContext<'a>> {
    let SourceArgumentReceipts {
        arguments,
        values,
        name_values,
        representations,
        objects,
        prefixes,
        reads,
    } = receipts;
    let context = *context;
    let compilation = context
        .original_variable_compilation
        .and_then(|original| (*original).with_evaluated_words(name_values));
    Box::new(SourceArgumentExecutionContext {
        context: SourceExecutionContext {
            original_written_projection: None,
            written_arguments: Some(arguments),
            written_values: Some(values),
            written_name_values: Some(name_values),
            original_variable_compilation: None,
            written_representations: Some(representations),
            written_objects: Some(objects),
            written_method_prefixes: Some(prefixes),
            written_variable_reads: Some(reads),
            ..context
        },
        compilation,
    })
}

#[inline(never)]
pub(super) fn source_execution_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    owner: Option<&'a SourceVariableEvaluationOwner>,
    offset: u32,
) -> Box<SourceExecutionContext<'a>> {
    let context = *context;
    Box::new(SourceExecutionContext {
        invocation_offset: offset,
        variable_read_owner: owner,
        ..context
    })
}

#[inline(never)]
pub(in super::super) fn source_context_clone_boxed<'a>(
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(*context)
}

#[inline(never)]
pub(in super::super) fn source_context_conditional_boxed<'a>(
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(context.conditional())
}

/// Derived traversal contexts live on the heap at genuine entry changes.
/// Every unmodified field retains its exact original frame and source receipt.
#[inline(never)]
pub(super) fn source_chunk_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    snapshot: Option<&'a NativeCompilationSnapshot>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        expression_source: None,
        original_variable_compilation: None,
        compilation_snapshot: context.compilation_snapshot.or(snapshot),
        ..*context
    })
}

#[inline(never)]
pub(super) fn source_selected_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    selected: &'a tcl_registry::native_compilation::NativeCompilationSelection,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        selected_compilation: Some(selected),
        original_variable_compilation: context
            .original_variable_compilation
            .filter(|original| original.selection() == selected),
        ..*context
    })
}

#[inline(never)]
pub(in super::super) fn source_body_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    namespace: &'a str,
    frame: &'a tcl_names::variable::VariableExecutionFrame,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        control: context
            .control
            .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
        namespace,
        namespace_key: frame.namespace_identity().or_else(|| {
            (context.namespace == namespace)
                .then_some(context.namespace_key)
                .flatten()
        }),
        frame,
        depth: context.depth + 1,
        ..*context
    })
}

/// Construct frame copies and derived contexts before recursive descent.
#[inline(never)]
pub(super) fn source_selected_frame_boxed(
    context: &SourceExecutionContext<'_>,
    namespace_known: bool,
) -> Box<tcl_names::variable::VariableExecutionFrame> {
    Box::new(if namespace_known {
        context
            .frame
            .clone()
            .with_namespace_identity(context.namespace_identity())
    } else {
        context.frame.clone()
    })
}

#[inline(never)]
pub(super) fn source_parent_frame_boxed(
    frame: &tcl_names::variable::VariableExecutionFrame,
) -> Box<tcl_names::variable::VariableExecutionFrame> {
    Box::new(frame.clone())
}

#[inline(never)]
pub(super) fn source_frame_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    frame: &'a tcl_names::variable::VariableExecutionFrame,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext { frame, ..*context })
}

#[inline(never)]
pub(super) fn source_original_compilation_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    original: Option<&'a original_name_value::OriginalSourceVariableCompilation<'a>>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        original_variable_compilation: original,
        ..*context
    })
}

#[inline(never)]
pub(super) fn source_namespace_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    namespace: &'a SourceNamespaceKey,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        namespace_key: Some(namespace),
        ..*context
    })
}

#[inline(never)]
pub(super) fn source_loop_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        control: context.control.with_compilation(
            tcl_registry::native_compilation::NativeCompilationContext {
                loop_depth: context.compilation().loop_depth.saturating_add(1),
                ..context.compilation()
            },
        ),
        ..*context
    })
}
