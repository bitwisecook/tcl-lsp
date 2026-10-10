// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional self-method source templates, separately from entered dispatch.

use super::{CommandAllocationSite, SourceDeclaredReceiverBodyEntry};
use crate::analyser::types::{
    MemberSide, OriginalLexicalMemberContext, OriginalSourceMethodMetadata,
};
use crate::analyser::{AnalysisResult, Scope, ScopeKind};
use crate::registry_invocation::InvocationWordOrigin;
use crate::registry_invocation::source_structure::{OriginalRegistryWords, source_registry_words};
use crate::signature_scan::{
    formal_parameters::SignatureSourceFormalParameters, scope::SignatureSourceNameKey,
};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage};
use tcl_syntax::word_rules::WordValueRules;
type SourceClassMetadata =
    crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ClassDef>;

/// Original source declaration selected for conditional caller-name advice.
/// This retains source MRO premises and both body owners, but supplies no
/// entered receiver, method implementation, caller cells or completed write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclaredSelfMethodTemplate {
    call: CommandAllocationSite,
    context: OriginalLexicalMemberContext,
    invocation: OriginalRegistryWords,
    caller: Arc<SourceDeclaredReceiverBodyEntry>,
    callee: Arc<SourceDeclaredReceiverBodyEntry>,
    method: Box<OriginalSourceMethodMetadata>,
    formals: SignatureSourceFormalParameters,
    display: String,
    parameter_arguments: Vec<Option<(usize, String)>>,
}
impl OriginalDeclaredSelfMethodTemplate {
    /// Exact original call source instruction; this is not an allocation result.
    #[must_use]
    pub const fn call(&self) -> &CommandAllocationSite {
        &self.call
    }
    /// Full unchanged Registry source vector, including conditional premises.
    #[must_use]
    pub const fn invocation(&self) -> &OriginalRegistryWords {
        &self.invocation
    }
    /// Authentic lexical caller declaration, independently of runtime entry.
    #[must_use]
    pub const fn lexical_context(&self) -> &OriginalLexicalMemberContext {
        &self.context
    }
    /// The actual original declaration-body recipe, with no entered frame grant.
    #[must_use]
    pub fn caller_body(&self) -> &SourceDeclaredReceiverBodyEntry {
        &self.caller
    }
    /// Exact selected source callee body; retaining it does not execute it.
    #[must_use]
    pub fn callee_body(&self) -> &SourceDeclaredReceiverBodyEntry {
        &self.callee
    }
    /// Canonical source member metadata, separate from any allocated method.
    #[must_use]
    pub fn source_method(&self) -> &OriginalSourceMethodMetadata {
        &self.method
    }
    /// Original parameter-list value under its independently selected grammar.
    #[must_use]
    pub const fn formals(&self) -> &SignatureSourceFormalParameters {
        &self.formals
    }
    /// Reporting label only; no identity or selection follows from this string.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display
    }
    /// A literal written operand and its original post-head index. Dynamic,
    /// inserted and expanded operands supply no editable literal projection.
    #[must_use]
    pub fn literal_parameter_argument(&self, parameter: usize) -> Option<(usize, &str)> {
        self.parameter_arguments
            .get(parameter)?
            .as_ref()
            .map(|(index, value)| (*index, value.as_str()))
    }
}

impl AnalysisResult {
    /// Source-only self-dispatch template in the same genuine receiver-body
    /// declaration as the later read. Unknown source MRO, a different frame,
    /// known head shadow or missing complete source correspondence abstains.
    #[must_use]
    pub fn original_declared_self_method_template(
        &self,
        source: &str,
        command: &crate::segmenter::SegmentedCommand,
        read: u32,
    ) -> Option<OriginalDeclaredSelfMethodTemplate> {
        // naming.tcloo.original-declared-self-method-caller-template
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md
        let (context, caller) = caller_context(self, source, command, read)?;
        let image = SourceImage::document(source);
        let config = context.resolved_input().lexer_config();
        let root = context.source_class(self)?;
        let bindings = self.retained_command_realm()?.source_bindings_ref();
        let origin = bindings.source_origin()?;
        let offset = command.argv.first()?.span.start();
        let invocation = source_registry_words(source, self, command)?;
        if !invocation.matches_source(&image, config) {
            return None;
        }
        let registry = context.resolved_input().context_registry();
        let count = invocation.with_source_schema(&registry, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::traits::Traits::TCLOO_SELF_DISPATCH)
                .then(|| schema.words.arguments().exact_argv_len())
                .flatten()
        })??;
        let selector = invocation.operands().first()?.as_ref()?.input()?;
        if selector.policy() != context.class_publication().policy() {
            return None;
        }
        let providers =
            crate::analyser::class_hierarchy::original_metadata::original_instance_metadata_order(
                self, root,
            )?;
        let (provider, method) = source_method(&providers, selector)?;
        if method.name_purpose() != tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod
            || method.native_class_delegate()
            || method.forward_prefix().is_some()
        {
            return None;
        }
        let body = method.body_word()?;
        let selected_body =
            bindings.declared_receiver_body_entry_at(origin, body.content_span().ok()?.start())?;
        if selected_body.declaration() != method.declaration().site()
            || !body_matches(body, &selected_body)
        {
            return None;
        }
        let parameters = method.parameters_word()?;
        let dialect = method.source_dialect()?;
        let original = SignatureSourceNameKey::from_original_native_word(
            parameters,
            WordValueRules::from_config(&parameters.config()),
            selector.policy(),
        )?;
        let formals = SignatureSourceFormalParameters::from_original_input(&original, dialect)?;
        if selected_body
            .original_formal_topology()
            .is_some_and(|topology| topology.original_input() != formals.original_input())
        {
            return None;
        }
        let plan = formals.bindings(count.checked_sub(1)?).ok()?;
        let parameter_arguments = literal_arguments(&invocation, &formals, plan);
        Some(OriginalDeclaredSelfMethodTemplate {
            call: CommandAllocationSite {
                source: origin.clone(),
                offset,
            },
            context: context.clone(),
            invocation,
            caller,
            callee: selected_body,
            method: Box::new(method.clone()),
            formals,
            display: format!(
                "{}::{}",
                provider.name().reported_full_name()?,
                method.metadata().name
            ),
            parameter_arguments,
        })
    }
}

fn caller_context<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    read: u32,
) -> Option<(
    &'a OriginalLexicalMemberContext,
    Arc<SourceDeclaredReceiverBodyEntry>,
)> {
    let offset = command.argv.first()?.span.start();
    if offset >= read {
        return None;
    }
    let context = lexical_context_at(&analysis.global_scope, offset, None)?;
    if lexical_context_at(&analysis.global_scope, read, None)? != context {
        return None;
    }
    let image = SourceImage::document(source);
    let config = context.resolved_input().lexer_config();
    if context.side() != MemberSide::Instance || !context.matches_source(&image, config) {
        return None;
    }
    context.source_class(analysis)?;
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(context.resolved_input())
        || realm.original_source_image()? != &image
    {
        return None;
    }
    let bindings = realm.source_bindings_ref();
    let origin = bindings.source_origin()?;
    let caller = bindings.declared_receiver_body_entry_at(origin, offset)?;
    if bindings.declared_receiver_body_entry_at(origin, read)? != caller
        || caller.declaration() != context.declaration_site()
        || !body_matches(context.body_word(), &caller)
    {
        return None;
    }
    Some((context, caller))
}

fn lexical_context_at<'a>(
    scope: &'a Scope,
    offset: u32,
    incoming: Option<&'a OriginalLexicalMemberContext>,
) -> Option<&'a OriginalLexicalMemberContext> {
    let current = match scope.kind {
        ScopeKind::Method => scope.original_member_context.as_deref(),
        ScopeKind::Proc => None,
        ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel => incoming,
    };
    let mut children = scope.children.iter().filter(|child| {
        child
            .body_span
            .is_some_and(|span| span.start() <= offset && offset < span.end())
    });
    if let Some(child) = children.next() {
        if children.next().is_some() {
            return None;
        }
        return lexical_context_at(child, offset, current);
    }
    current
}

fn body_matches(word: &NativeWord, entry: &SourceDeclaredReceiverBodyEntry) -> bool {
    let Ok(span) = word.content_span() else {
        return false;
    };
    let script = entry.source();
    script.origin.source_image() == word.image()
        && script.base() == span.start()
        && word.image().bytes().get(span.as_range()) == Some(script.text.bytes())
}

fn source_method<'a>(
    providers: &[&'a SourceClassMetadata],
    selector: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<(&'a SourceClassMetadata, OriginalSourceMethodMetadata)> {
    for provider in providers {
        let methods = provider
            .metadata()
            .original_members
            .methods(MemberSide::Instance)?;
        if let Some(method) = methods.into_iter().find(|method| {
            method.original_name_input().policy() == selector.policy()
                && method.original_name_input().bytes() == selector.bytes()
        }) {
            return Some((*provider, method));
        }
    }
    None
}

fn literal_arguments(
    invocation: &OriginalRegistryWords,
    formals: &SignatureSourceFormalParameters,
    plan: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
) -> Vec<Option<(usize, String)>> {
    let mut output = vec![None; formals.parameters().len()];
    for binding in plan {
        let tcl_syntax::formal_params::FormalByteArgumentBinding::Value {
            parameter,
            argument,
        } = binding
        else {
            continue;
        };
        let operand = argument + 1;
        let Some(value) = invocation
            .arguments()
            .get(operand)
            .and_then(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
        else {
            continue;
        };
        let Some(InvocationWordOrigin::Written(written)) = invocation.origins().get(operand + 1)
        else {
            continue;
        };
        let Some(original) = invocation
            .operands()
            .get(operand)
            .and_then(Option::as_ref)
            .and_then(|operand| operand.input())
        else {
            continue;
        };
        if original.bytes() == value
            && let Ok(label) = core::str::from_utf8(value)
        {
            output[parameter] = written
                .checked_sub(1)
                .map(|written| (written, label.to_owned()));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    fn query(
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<OriginalDeclaredSelfMethodTemplate> {
        let offset = u32::try_from(source.rfind("my ")?).ok()?;
        let context = lexical_context_at(&analysis.global_scope, offset, None)?;
        let range = context.body_word().content_span().ok()?;
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            &source[range.as_range()],
            range.start(),
            context.resolved_input().lexer_config(),
        )
        .into_iter()
        .find(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span.start() == offset)
        })?;
        analysis.original_declared_self_method_template(
            source,
            &command,
            u32::try_from(source.rfind("puts")?).ok()?,
        )
    }
    #[test]
    fn original_declared_self_template_uses_source_mixin_without_entering_a_receiver() {
        // naming.tcloo.original-declared-self-method-caller-template
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md
        let source = "oo::class create M {method pick {name} {upvar name local; set local VALUE}}
oo::class create C {mixin M; constructor {} {my pick name; puts $name}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_classes.clear();
            let template = query(source, &analysis).unwrap();
            assert_eq!(
                template.source_method().original_name_input().bytes(),
                b"pick"
            );
            assert_eq!(template.literal_parameter_argument(0), Some((1, "name")));
            assert_ne!(
                template.caller_body().declaration(),
                template.callee_body().declaration()
            );
            let realm = analysis.retained_command_realm().unwrap();
            assert!(
                realm
                    .source_bindings_ref()
                    .invocation_at_source("", template.call().offset)
                    .receiver_self_method_entry(context_registry(&template).commands())
                    .is_none()
            );
        }
    }
    fn context_registry(
        template: &OriginalDeclaredSelfMethodTemplate,
    ) -> Arc<tcl_registry::model::ContextRegistry> {
        template
            .lexical_context()
            .resolved_input()
            .context_registry()
    }
    #[test]
    fn original_declared_self_template_declines_unknown_relations_shadow_and_foreign_source() {
        // naming.tcloo.original-declared-self-method-caller-template
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md
        for source in [
            "oo::class create C {mixin Missing; constructor {} {my pick name; puts $name}}",
            "oo::class create M {method pick {name} {}}; oo::class create C {mixin M; constructor {} {proc nested {} {my pick name; puts $name}}}",
            "rename ::oo::class {}; proc ::oo::class {args} {}; oo::class create C {method pick {name} {}; constructor {} {my pick name; puts $name}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(query(source, &analysis).is_none(), "{source}");
        }
        let source =
            "oo::class create C {method pick {name} {}; constructor {} {my pick name; puts $name}}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(query(&(source.to_owned() + " "), &analysis).is_none());
    }
    #[test]
    fn original_declared_receiver_traits_keep_caller_names_without_runtime_entry() {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        let source = "oo::class create M {method pick {input} {upvar name local; set local VALUE; upvar $input alias; set alias VALUE}}
oo::class create C {mixin M; constructor {} {my pick name; puts $name}}";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let template = query(source, &analysis).unwrap();
            let registry = context_registry(&template);
            let script = template.callee_body().source();
            let realm = analysis.retained_command_realm().unwrap();
            let env = crate::analyser::param_traits::TraitScanEnv {
                surface: tcl_registry::model::DocumentCommandSurface::new(
                    registry.commands(),
                    None,
                ),
                config: analysis.body_lexer_config.unwrap(),
                identities: realm,
                executed_source: Some(script),
            };
            let body = script.try_text().unwrap();
            assert_eq!(
                crate::analyser::param_traits::caller_frame_literal_targets(body, env).get("name"),
                Some(&true),
                "{dialect}"
            );
            assert!(
                crate::analyser::param_traits::caller_frame_upvar_params(&["input"], body, env)
                    .contains("input"),
                "{dialect}"
            );
            let traits = crate::analyser::param_traits::infer_param_traits(&["input"], body, env);
            assert!(
                traits["input"].contains(&crate::analyser::types::ProcArgTrait::VarWrite),
                "{dialect}"
            );
            let source_bindings = realm.source_bindings_ref();
            let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
            let binding = source_bindings.invocation_at_source("", offset);
            assert!(
                binding
                    .receiver_self_method_entry(registry.commands())
                    .is_none()
            );
            assert!(binding.proved_execution_target().is_none());
        }
    }

    #[test]
    fn original_declared_receiver_traits_refuse_shadowed_and_foreign_layouts() {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        for prelude in [
            "proc upvar {args} {}\n",
            "trace add execution upvar enter {apply {{args} {}}}\n",
        ] {
            let source = format!(
                "{prelude}oo::class create C {{method pick {{}} {{upvar name local; set local VALUE}}; constructor {{}} {{my pick; puts $name}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            if let Some(template) = query(&source, &analysis) {
                let registry = context_registry(&template);
                let script = template.callee_body().source();
                let env = crate::analyser::param_traits::TraitScanEnv {
                    surface: tcl_registry::model::DocumentCommandSurface::new(
                        registry.commands(),
                        None,
                    ),
                    config: analysis.body_lexer_config.unwrap(),
                    identities: analysis.retained_command_realm().unwrap(),
                    executed_source: Some(script),
                };
                assert!(
                    crate::analyser::param_traits::caller_frame_literal_targets(
                        script.try_text().unwrap(),
                        env
                    )
                    .is_empty(),
                    "{prelude}"
                );
                assert!(
                    crate::analyser::param_traits::caller_frame_literal_targets(
                        "upvar other local",
                        env
                    )
                    .is_empty()
                );
            }
        }
    }
}
