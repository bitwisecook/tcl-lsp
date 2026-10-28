// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fold requests selected for an owned application evaluator.
//!
//! Selection runs the ordinary retained-source gates of the O129 fold but not
//! the selected outer implementation. The result is request data only: no
//! rewrite, execution, cache entry or Native permission follows from it, and
//! its holder decides which evaluator, if any, answers it.

use tcl_registry::CommandRegistry;
use tcl_runtime_api::closed_native_consumer::ClosedSourceRefusal;

use crate::compilation_unit::CompilationUnit;
use crate::const_subst::{ConstSubstCtx, SelectedConstFold};
use crate::ir::WordExpr;

/// Maximum accepted UTF-8 source bytes for request selection.
pub const MAX_APPLICATION_SOURCE_BYTES: usize = 4096;

/// One whole-word command substitution of a top-level command, with the fold
/// request the ordinary optimiser would have evaluated for it.
#[derive(Debug, Clone)]
pub struct SelectedApplicationFold {
    span: std::ops::Range<usize>,
    request: SelectedConstFold,
    release: tcl_dialect::TclVersion,
}

impl SelectedApplicationFold {
    /// Byte range of the whole `[…]` in the selected source.
    #[must_use]
    pub fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    /// The checked request; it has not been evaluated.
    #[must_use]
    pub fn request(&self) -> &SelectedConstFold {
        &self.request
    }

    /// Whether `value` may stand for the substitution as the literal it is
    /// spelt, under the same release rule the ordinary fold applies.
    #[must_use]
    pub fn admits_answer(&self, value: &str) -> bool {
        !crate::const_subst::folded_literal_reads_apart(value, self.release)
    }
}

/// Select the fold request of every whole-word command substitution in the
/// top-level commands of `source`. Source declaring a procedure or other
/// lowered body is refused, and a command whose
/// retained substitution inventory is unavailable selects nothing.
///
/// # Errors
/// Refuses oversized or procedure-bearing source, a profile without an actual
/// Tcl execution point, and a module without retained source metadata.
pub fn select_application_folds(
    source: &str,
    registry: &CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<Vec<SelectedApplicationFold>, ClosedSourceRefusal> {
    if source.len() > MAX_APPLICATION_SOURCE_BYTES {
        return Err(ClosedSourceRefusal::Budget);
    }
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    if dialect.execution_point().is_none()
        || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
    {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    let cu = CompilationUnit::build_for_profile(source, registry, false, profile);
    let module = &cu.ir_module;
    if !module.procedures.is_empty()
        || module.body_lowerings().next().is_some()
        || module.source_metadata_input.is_none()
    {
        return Err(ClosedSourceRefusal::Source);
    }
    let retained = crate::registry_invocation::retained_module_metadata_context(registry, module)
        .ok_or(ClosedSourceRefusal::Source)?;
    let metadata: crate::registry_invocation::InvocationMetadataContext<'_> = (&*retained).into();
    let mut selected = Vec::new();
    for statement in &module.top_level.statements {
        let Some(tokens) = statement.tokens() else {
            continue;
        };
        let Some(calls) =
            crate::optimiser::propagation::retained_substitution_calls(tokens, module, registry)
        else {
            continue;
        };
        for word in tokens.words() {
            let WordExpr::CommandSubstitution { source: site, .. } = word else {
                continue;
            };
            let Some(call) =
                crate::optimiser::propagation::retained_call_at(&calls, site.span.start())
            else {
                continue;
            };
            let context = ConstSubstCtx {
                registry,
                resolution_namespace: "::",
                namespace_context: call
                    .tokens
                    .as_ref()
                    .and_then(crate::registry_invocation::compiled_namespace_context),
                version: None,
                defining_class: None,
                trusts: &|_| false,
                lookup_var: &|_| None,
            };
            if let Some(request) =
                context.select_retained_call_with_metadata_context(call, &calls, metadata)
            {
                selected.push(SelectedApplicationFold {
                    span: call.span.as_range(),
                    request,
                    release: profile.vm_runtime_version,
                });
            }
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .and_then(|environment| environment.catalogue_profile())
            .expect("tcl9.1 catalogue profile")
    }

    #[test]
    fn selection_names_the_request_without_running_it() {
        let profile = profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let source = "set s abc\nputs [string length $s]\n";
        let selected =
            select_application_folds(source, &registry, profile).expect("top-level source selects");
        let [fold] = selected.as_slice() else {
            panic!("one selected fold: {selected:?}");
        };
        assert_eq!(&source[fold.span()], "[string length $s]");
        assert_eq!(fold.request().facts().canonical_command, "string");
        assert_eq!(fold.request().arguments(), Some(vec!["abc"]));
        assert!(fold.admits_answer("3"));
    }

    #[test]
    fn selection_refuses_procedures_and_oversized_source() {
        let profile = profile();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        assert_eq!(
            select_application_folds("proc p {} {string length abc}\np\n", &registry, profile)
                .unwrap_err(),
            ClosedSourceRefusal::Source
        );
        let long = format!("set s {}\n", "x".repeat(MAX_APPLICATION_SOURCE_BYTES));
        assert_eq!(
            select_application_folds(&long, &registry, profile).unwrap_err(),
            ClosedSourceRefusal::Budget
        );
    }
}
