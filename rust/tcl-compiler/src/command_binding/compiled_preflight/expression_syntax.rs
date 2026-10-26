// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-only child syntax from an actual retained C8.6 Expr recipe.

use super::{
    Arc, CommandAllocationSite, CompilerTraversal, SourceCommandBindings, SourceCompiledChild,
    SourceExecutionContext, WordExpr,
};
use crate::command_binding::compiled_invocation::{
    SourceNativeCompilerAdmission, SourceNativeStructuredPreparation,
};
use crate::command_binding::compiler_inventory::{SourceCompilerInvocation, SourceCompilerVisits};
use std::collections::BTreeMap;
use tcl_registry::native_expression_program::NativeExpressionProgram;
use tcl_registry::native_instruction_plan::NativeInstructionPlan;

/// This inventory never participates in Native selection or runtime entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::command_binding) struct SourceExpressionSyntaxChild {
    child: SourceCompiledChild,
    preparation: Arc<SourceNativeStructuredPreparation>,
    dependencies: Vec<crate::command_binding::SourceNativeCompilationDependency>,
}

fn expression_program(
    preparation: &SourceNativeStructuredPreparation,
) -> Option<&NativeExpressionProgram> {
    let NativeInstructionPlan::Expression(expression) = &preparation.recipe else {
        return None;
    };
    expression.program.as_ref()
}

fn current_parent(visits: &SourceCompilerVisits, child: &SourceExpressionSyntaxChild) -> bool {
    let SourceNativeCompilerAdmission::Inline(parent) = child.child.parent.as_ref() else {
        return false;
    };
    child.dependencies.iter().all(|dependency| {
        dependency.guard == tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry
    }) && parent.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
        && child.preparation.compilation_site == parent.compilation_site
        && visits.get(&parent.compilation_site).is_some_and(|parents| {
            !parents.is_empty()
                && parents.iter().all(|visit| {
                    visit.admitted.as_deref() == Some(child.child.parent.as_ref())
                        && visit.structured.as_deref() == Some(child.preparation.as_ref())
                        && visit.namespace_key == child.child.parent_namespace_key
                        && visit.table == child.child.table
                })
        })
}

fn exact_child(child: &SourceExpressionSyntaxChild, site: &CommandAllocationSite) -> bool {
    let Some(program) = expression_program(&child.preparation) else {
        return false;
    };
    let Some(scripts) = program.c86_eager_compiler_scripts() else {
        return false;
    };
    let image = child.preparation.compilation_site.source.source_image();
    if !Arc::ptr_eq(&child.child.script.origin, &site.source)
        || !Arc::ptr_eq(
            &child.child.script.origin,
            &child.preparation.compilation_site.source,
        )
        || !matches!(child.child.script.mapping, super::super::ExecutedScriptMapping::Contiguous { base } if base == site.offset)
        || child.child.script.base() != site.offset
        || image.bytes().get(program.span.as_range()) != Some(program.source.as_slice())
        || child.child.script.text.channel() != image.channel()
    {
        return false;
    }
    scripts.into_iter().any(|span| {
        let Some(base) = program
            .span
            .start()
            .checked_add(span.start())
            .and_then(|base| base.checked_add(1))
        else {
            return false;
        };
        let Some(bytes) = program.source.get(span.as_range()) else {
            return false;
        };
        base == site.offset
            && bytes.len() >= 2
            && bytes.first() == Some(&b'[')
            && bytes.last() == Some(&b']')
            && child.child.script.text.bytes() == &bytes[1..bytes.len() - 1]
    })
}

fn syntax_visits(scanner: &CompilerTraversal<'_>) -> SourceCompilerVisits {
    let mut visits = scanner.compiler_invocations.clone();
    for (site, children) in &scanner.expression_syntax_children {
        for child in children {
            if !exact_child(child, site) || !current_parent(&visits, child) {
                continue;
            }
            for (site, incoming) in child.child.compiler_visits.iter() {
                let retained = visits.entry(site.clone()).or_default();
                for visit in incoming {
                    if !retained.contains(visit) {
                        retained.push(visit.clone());
                    }
                }
            }
        }
    }
    visits
}

struct ExpressionParent {
    admission: Arc<SourceNativeCompilerAdmission>,
    preparation: Arc<SourceNativeStructuredPreparation>,
}

struct ExpressionObservation {
    sources: Vec<(u32, tcl_lexer::SourceImage)>,
    visits: Arc<SourceCompilerVisits>,
    dependencies: Vec<crate::command_binding::SourceNativeCompilationDependency>,
}

impl CompilerTraversal<'_> {
    fn expression_parent(
        &self,
        words: &[WordExpr],
        program: &NativeExpressionProgram,
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) -> Option<ExpressionParent> {
        if words.len() != 2 {
            return None;
        }
        let selected = super::super::compiled_invocation::select_invocation_boxed(
            words,
            self.state,
            &SourceExecutionContext {
                compilation_snapshot: Some(self.snapshot),
                ..context
            },
            offset,
        );
        let parent = selected.admitted?;
        let SourceNativeCompilerAdmission::Inline(proof) = parent.as_ref() else {
            return None;
        };
        let preparation = selected.structured?;
        let original = selected.original_words?;
        if proof.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
            || expression_program(&preparation) != Some(program)
            || proof.compilation_site != preparation.compilation_site
            || proof.compilation_site.offset != offset
            || !Arc::ptr_eq(&proof.compilation_site.source, &self.chunk.source)
            || original.site != proof.compilation_site
            || original.words.as_ref() != words
            || original.config != context.config
        {
            return None;
        }
        Some(ExpressionParent {
            admission: parent,
            preparation,
        })
    }

    #[inline(never)]
    fn observe_expression_syntax(
        &self,
        program: &NativeExpressionProgram,
        scripts: &[tcl_lexer::Span],
        context: SourceExecutionContext<'_>,
    ) -> Option<ExpressionObservation> {
        let image = self.chunk.source.source_image();
        if image.bytes().get(program.span.as_range()) != Some(program.source.as_slice()) {
            return None;
        }
        let mut snapshot = self.snapshot.clone();
        // This scratch compiler never mutates the caller's admission, locals,
        // failure order, snapshot or effect/Normal outcomes. Only the separate
        // source-syntax inventory can receive its immutable visited vectors.
        let mut scanner = CompilerTraversal {
            state: self.state,
            snapshot: &mut snapshot,
            chunk: self.chunk.clone(),
            possible_error: false,
            compiler_dependencies: Vec::new(),
            math_table_prerequisite: None,
            before_next_context: Vec::new(),
            compiler_invocations: BTreeMap::new(),
            compiled_children: BTreeMap::new(),
            expression_syntax_children: BTreeMap::new(),
            locals: None,
        };
        let mut sources = Vec::new();
        for span in scripts {
            let bytes = program.source.get(span.as_range()).filter(|bytes| {
                bytes.len() >= 2 && bytes.first() == Some(&b'[') && bytes.last() == Some(&b']')
            })?;
            let base = program
                .span
                .start()
                .checked_add(span.start())?
                .checked_add(1)?;
            let source =
                tcl_lexer::SourceImage::from_bytes(&bytes[1..bytes.len() - 1], image.channel());
            let child_context = SourceExecutionContext {
                control: context
                    .control
                    .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
                depth: context.depth + 1,
                ..context
            };
            if scanner.script(&source, base, child_context).is_some() || scanner.possible_error {
                return None;
            }
            sources.push((base, source));
        }
        let visits = syntax_visits(&scanner);
        // A nested expression remains a separate actual compiler boundary.
        // This initial correspondence never admits a lazy/function/recovery
        // program merely because its enclosing expression is eager.
        if visits.values().flatten().any(|visit| {
            visit
                .structured
                .as_ref()
                .and_then(|preparation| expression_program(preparation))
                .is_some_and(|program| program.c86_eager_compiler_scripts().is_none())
        }) {
            return None;
        }
        Some(ExpressionObservation {
            sources,
            visits: Arc::new(visits),
            dependencies: scanner.compiler_dependencies.clone(),
        })
    }

    pub(super) fn retain_expression_syntax(
        &mut self,
        words: &[WordExpr],
        program: &NativeExpressionProgram,
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) {
        let Some(scripts) = program
            .c86_eager_compiler_scripts()
            .filter(|scripts| !scripts.is_empty())
        else {
            return;
        };
        let Some(parent) = self.expression_parent(words, program, offset, context) else {
            return;
        };
        let Some(observation) = self.observe_expression_syntax(program, &scripts, context) else {
            return;
        };
        for (base, source) in observation.sources {
            let child = SourceExpressionSyntaxChild {
                child: SourceCompiledChild {
                    enclosing: self.chunk.clone(),
                    table: Arc::clone(&self.snapshot.table),
                    parent: Arc::clone(&parent.admission),
                    script: super::super::ExecutedScriptSource {
                        text: source,
                        origin: Arc::clone(&self.chunk.source),
                        mapping: super::super::ExecutedScriptMapping::Contiguous { base },
                    },
                    compilation: context.compilation(),
                    parent_namespace_key: context.namespace_identity(),
                    namespace_key: context.namespace_identity(),
                    compiler_visits: Arc::clone(&observation.visits),
                },
                preparation: Arc::clone(&parent.preparation),
                dependencies: observation.dependencies.clone(),
            };
            let site = CommandAllocationSite {
                source: Arc::clone(&self.chunk.source),
                offset: base,
            };
            self.expression_syntax_children
                .entry(site)
                .or_default()
                .push(child);
        }
    }
}

impl SourceCommandBindings {
    pub(in crate::command_binding) fn original_expression_compiler_syntax_visits(
        &self,
        site: &CommandAllocationSite,
    ) -> Vec<&SourceCompilerInvocation> {
        let mut visits = Vec::new();
        for (child_site, children) in &self.expression_syntax_children {
            if !Arc::ptr_eq(&child_site.source, &site.source) || child_site.offset > site.offset {
                continue;
            }
            for child in children {
                if !exact_child(child, child_site)
                    || !current_parent(&self.compiler_invocations, child)
                    || self
                        .compilation_provider_required
                        .contains(&child.child.enclosing)
                    || site
                        .offset
                        .checked_sub(child_site.offset)
                        .is_none_or(|offset| offset as usize >= child.child.script.text.len())
                {
                    continue;
                }
                if let Some(original) = child.child.compiler_visits.get(site) {
                    visits.extend(original);
                }
            }
        }
        visits
    }
}
