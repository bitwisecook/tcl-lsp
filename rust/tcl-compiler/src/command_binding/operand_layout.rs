// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Operand layout selected by an actual compiler registration.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceInvocationBinding, SourceNativeCompilationDependency,
};
use tcl_registry::native_compilation::{
    NativeCompilationOperandLayout, NativeCompilationSpec, NativeCompilationWordShape,
};

/// Compiler operand coordinates without an opcode or completion licence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeOperandLayoutProof {
    target: SourceCommandTarget,
    layout: NativeCompilationOperandLayout,
    compilation_site: CommandAllocationSite,
    lookup_dependencies: Vec<SourceNativeCompilationDependency>,
}

impl SourceNativeOperandLayoutProof {
    /// Registry-owned coordinates in the original effective argv.
    #[must_use]
    pub fn layout(&self) -> NativeCompilationOperandLayout {
        self.layout
    }

    /// Original compiler registration, independent of the later handler.
    #[must_use]
    pub fn target(&self) -> &SourceCommandTarget {
        &self.target
    }

    /// Exact source instruction at which this layout was selected.
    #[must_use]
    pub fn compilation_site(&self) -> &CommandAllocationSite {
        &self.compilation_site
    }

    /// Original registration guards; no reached-handler proof is implied.
    #[must_use]
    pub fn lookup_dependencies(&self) -> &[SourceNativeCompilationDependency] {
        &self.lookup_dependencies
    }
}

impl SourceInvocationBinding {
    /// Independently retained compiler operand layout, never an opcode proof.
    #[must_use]
    pub fn native_operand_layout(&self) -> Option<&SourceNativeOperandLayoutProof> {
        self.native_operand_layout.as_deref()
    }
}

#[derive(Clone, Copy)]
pub(super) struct LayoutInvocation<'a> {
    pub invocation: tcl_registry::InvocationWords<'a>,
    pub shapes: &'a [NativeCompilationWordShape],
    pub target: &'a SourceCommandTarget,
    pub head: &'a str,
    pub offset: u32,
}

pub(super) fn retain_layout(
    spec: NativeCompilationSpec,
    selected: LayoutInvocation<'_>,
    dependency_closed: bool,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Arc<SourceNativeOperandLayoutProof>> {
    if !dependency_closed {
        return None;
    }
    let LayoutInvocation {
        invocation,
        shapes,
        target,
        head,
        offset,
    } = selected;
    let layout = spec.operand_layout(
        invocation,
        shapes,
        state.baseline.compilation_dialect(),
        context.compilation,
    )?;
    let NativeCompilationOperandLayout::Pattern { guard, .. } = layout;
    Some(Arc::new(SourceNativeOperandLayoutProof {
        target: target.clone(),
        layout,
        compilation_site: CommandAllocationSite {
            source: Arc::clone(state.current_source_origin.as_ref()?),
            offset,
        },
        lookup_dependencies: vec![SourceNativeCompilationDependency {
            compiler_prerequisite: state
                .runtime_command_compiler_prerequisite(
                    head,
                    &context.namespace_identity(),
                    crate::registry_invocation::native_command_binding_guard(guard),
                )
                .map(Arc::new),
            target: target.clone(),
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            head: head.to_owned(),
            guard,
        }],
    }))
}
