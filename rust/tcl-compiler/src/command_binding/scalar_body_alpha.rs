// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional scalar formal renaming without changing frames or values.

use super::declaration_layout::original_declaration_layouts;
use super::{CommandAllocationSite, SourceCommandBindings, SourceInvocationBinding};
use crate::signature_scan::variable_symbol::{
    SignatureSourceVariableSlot, SignatureSourceVariableSymbol,
};
use tcl_lexer::{ExecutablePart, LexerConfig, SourceImage};
use tcl_registry::native_result::{NativeResultContract, NativeResultSelection};

/// Semantic equivalence for the sole scalar formal of a closed source whose
/// sole procedure body returns that same whole object. The original and
/// renamed executions retain the same activation, argv and release order.
/// Source edits still require independently authenticated formal/root geometry.
/// Arbitrary external procedure introspection is outside this closed-source
/// contract; this is neither a reached call nor a physical native frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalScalarBodyAlphaRename {
    symbol: SignatureSourceVariableSymbol,
    proposed: tcl_core_types::NameBytes,
    declaration: CommandAllocationSite,
    body: super::declaration_preview::DeclaredProcedureBody,
    entry: std::sync::Arc<super::declaration_layout::OriginalDiagnosticFrameEntry>,
    target: super::SourceCommandTarget,
    config: LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
}

impl OriginalScalarBodyAlphaRename {
    /// Exact source-local symbol whose complete formal/read coverage was proved.
    #[must_use]
    pub const fn selected_symbol(&self) -> &SignatureSourceVariableSymbol {
        &self.symbol
    }

    /// Proposed scalar formal bytes under the same independently selected policy.
    #[must_use]
    pub fn new_native_tail(&self) -> &[u8] {
        self.proposed.as_bytes()
    }

    /// Exact original declaration, without claiming a runtime installation.
    #[must_use]
    pub const fn declaration_site(&self) -> &CommandAllocationSite {
        &self.declaration
    }

    /// Whole consumer image, source channel and full parser correspondence.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.declaration.source.source_image() == image && self.config == config
    }

    /// Exact semantic Registry selection retained by this equivalence proof.
    #[must_use]
    pub fn matches_registry(&self, registry: &tcl_registry::CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

fn trace_alpha(stage: &str, offset: u32) {
    if std::env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some() {
        eprintln!("SCALAR_ALPHA offset={offset} stage={stage}");
    }
}

struct ScalarAlphaRowContext<'a> {
    entry: &'a std::sync::Arc<super::declaration_layout::OriginalDiagnosticFrameEntry>,
    topology: &'a super::formal_topology::OriginalFormalTopology,
    symbol: &'a SignatureSourceVariableSymbol,
    name: &'a tcl_core_types::NameBytes,
    image: &'a SourceImage,
    config: LexerConfig,
    registry: &'a tcl_registry::CommandRegistry,
}

fn validate_alpha_formal<'a>(
    body: &'a super::declaration_preview::DeclaredProcedureBody,
    entry: &std::sync::Arc<super::declaration_layout::OriginalDiagnosticFrameEntry>,
    symbol: &SignatureSourceVariableSymbol,
    proposed: &[u8],
    config: LexerConfig,
    site: &CommandAllocationSite,
) -> Option<(
    &'a super::formal_topology::OriginalFormalTopology,
    tcl_core_types::NameBytes,
)> {
    trace_alpha("formal-topology", site.offset);
    let topology = body.original_parameters.as_ref()?;
    let [parameter] = topology.parameters() else {
        return None;
    };
    if parameter.default.is_some() {
        return None;
    }
    let names = topology.fixed_scalar_binding_names(1)?;
    let [name] = names.as_slice() else {
        return None;
    };
    // A scalar spelling can change the ParamList grammar, such as the
    // final rest name or Jim's caller-link prefix. Reuse its own recipe.
    trace_alpha("formal-rename-grammar", site.offset);
    let fields = topology.original_name_fields()?;
    let [field] = fields.as_slice() else {
        return None;
    };
    if field.name != *name || field.recipe.renamed_input(proposed)?.as_slice() != proposed {
        return None;
    }
    trace_alpha("symbol-frame", site.offset);
    let SignatureSourceVariableSlot::Local { frame, simple } = symbol.slot() else {
        return None;
    };
    if std::env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some() {
        eprintln!(
            "SCALAR_ALPHA offset={} frame_source={} frame_identity={} frame_config={} formal_key={} formal_policy={}",
            site.offset,
            frame.source() == entry.source(),
            frame.frame() == entry.frame(),
            frame.lexer_config() == config,
            simple == name,
            symbol.policy() == topology.original_input().policy()
        );
    }
    if frame.source() != entry.source()
        || frame.frame() != entry.frame()
        || frame.lexer_config() != config
        || simple != name
        || symbol.policy() != topology.original_input().policy()
    {
        return None;
    }
    let renamed = symbol.renamed(proposed)?;
    trace_alpha("renamed-symbol", site.offset);
    let SignatureSourceVariableSlot::Local {
        frame: new_frame,
        simple: new_name,
    } = renamed.slot()
    else {
        return None;
    };
    if new_frame != frame || new_name.as_bytes() != proposed {
        return None;
    }

    Some((topology, name.clone()))
}

fn scalar_alpha_body_site(
    body: &super::declaration_preview::DeclaredProcedureBody,
    site: &CommandAllocationSite,
    config: LexerConfig,
) -> Option<CommandAllocationSite> {
    trace_alpha("body-geometry", site.offset);
    let super::ExecutedScriptMapping::Contiguous { base } = body.source.mapping else {
        return None;
    };
    let body_commands = crate::segmenter::segment_commands_image_with_offset_and_config(
        &body.source.text,
        base,
        config,
    )?;
    let [body_command] = body_commands.as_slice() else {
        return None;
    };
    if body_command.is_partial {
        return None;
    }
    let body_site = CommandAllocationSite {
        source: site.source.clone(),
        offset: body_command.span.start(),
    };
    Some(body_site)
}

fn validate_alpha_return_contract(
    body_site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    target: &super::SourceCommandTarget,
    config: LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Option<()> {
    trace_alpha("return-contract", body_site.offset);
    let advice = super::original_site_operand_layout_advice(
        body_site,
        tokens,
        &row.snapshot,
        &row.namespace,
        config,
    )?;
    let dialect = advice.dialect();
    let effective = crate::registry_invocation::effective_words_for_target(tokens, target)?;
    let values = effective
        .words
        .iter()
        .map(|word| {
            crate::registry_invocation::effective_invocation_word(
                word,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            )
        })
        .collect::<Vec<_>>();
    let arguments = values
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
        crate::registry_invocation::resolve_registry_words_in_realm(
            registry,
            None,
            &arguments,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
    else {
        return None;
    };
    let args =
        tcl_registry::InvocationArguments::structured(arguments.get(1..)?).with_dialect(dialect);
    if facts.native_result != Some(NativeResultContract::ReturnResult)
        || facts.argument_offset != 0
        || NativeResultContract::ReturnResult.select(args, 0) != NativeResultSelection::Argument(0)
    {
        return None;
    }
    Some(())
}

impl SourceCommandBindings {
    pub(crate) fn original_scalar_body_alpha_rename(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
        symbol: &SignatureSourceVariableSymbol,
        proposed: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalScalarBodyAlphaRename> {
        let site = declaration.declaration_site();
        trace_alpha("consumer-correspondence", site.offset);
        let config = declaration.name_input().lexer_config();
        let image = site.source.source_image();
        self.matches_original_source_image(image, config)
            .then_some(())?;

        // A body-only proof cannot close callers observing info args/body or
        // registering an execution callback elsewhere in the same source.
        let commands =
            crate::segmenter::segment_commands_image_with_offset_and_config(image, 0, config)?;
        let [command] = commands.as_slice() else {
            return None;
        };
        if command.is_partial || command.span.start() != site.offset {
            return None;
        }
        trace_alpha("completed-world", site.offset);
        let world = self.original_completed_command_world()?;
        let publication = world.declaration_at(declaration.name().slot(), symbol.policy())?;
        if publication.declaration_site() != site
            || publication.kind() != super::OriginalCommandPublicationKind::Procedure
        {
            return None;
        }

        trace_alpha("declared-body", site.offset);
        let body = self.original_declared_procedure_at(site, registry)?;
        trace_alpha("body-frame-owner", site.offset);
        let entry = if let Some(entry) =
            self.conditional_body_entry_at(&body.source.origin, body.source.base())
        {
            if entry.source() != &body.source
                || entry.original_formal_topology() != body.original_parameters.as_ref()
                || entry.namespace_context() != Some(&body.namespace)
                || !publication.has_implementation_allocation(entry.allocation())
                || entry.allocation().site != *site
            {
                return None;
            }
            super::declaration_layout::OriginalDiagnosticFrameEntry::Body(entry)
        } else {
            super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(
                std::sync::Arc::new(body.clone()),
            )
        };
        let entry = std::sync::Arc::new(entry);
        let (topology, name) =
            validate_alpha_formal(&body, &entry, symbol, proposed, config, site)?;
        let body_site = scalar_alpha_body_site(&body, site, config)?;
        trace_alpha("body-observations", body_site.offset);
        let rows = original_declaration_layouts(self.declaration_layouts.get(&body_site)?)?;
        let context = ScalarAlphaRowContext {
            entry: &entry,
            topology,
            symbol,
            name: &name,
            image,
            config,
            registry,
        };
        let mut selected = None;
        for row in rows {
            let target = self.scalar_alpha_row_target(&body_site, row, &context)?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &target)
            {
                return None;
            }
            selected = Some(target);
        }
        trace_alpha("admitted", site.offset);
        Some(OriginalScalarBodyAlphaRename {
            symbol: symbol.clone(),
            proposed: tcl_core_types::NameBytes::from(proposed),
            declaration: site.clone(),
            body,
            entry,
            target: selected?,
            config,
            registry: registry.snapshot().semantic_key(),
        })
    }
    fn scalar_alpha_row_target(
        &self,
        body_site: &CommandAllocationSite,
        row: &super::declaration_layout::DeclarationLayoutObservation,
        context: &ScalarAlphaRowContext<'_>,
    ) -> Option<super::SourceCommandTarget> {
        let ScalarAlphaRowContext {
            entry,
            topology,
            symbol,
            name,
            image,
            config,
            registry,
        } = *context;
        trace_alpha("body-entry", body_site.offset);
        if &row.entry != entry
            || row.entry.original_formal_topology() != Some(topology)
            || row.config != config
        {
            return None;
        }
        trace_alpha("body-observers", body_site.offset);
        let state = &row.snapshot.state;
        let context = &state.source_variables;
        if state.has_opaque_domain()
            || state.source_step_observed()
            || context.dynamic_bindings
            || !context.alias_bindings.is_empty()
            || !context.upvar_aliases.is_empty()
            || !context.traced.is_empty()
            || !context.untracked_traces.is_empty()
            || !context.trace_registrations.is_empty()
            || !context.possible_trace_registrations.is_empty()
        {
            return None;
        }
        trace_alpha("body-words", body_site.offset);
        let tokens = super::declaration_preview::declaration_tokens(body_site, row)?;
        let words = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            body_site.offset,
            config,
        )?;
        let [head, argument] = words.as_slice() else {
            return None;
        };
        if head.group().expand || argument.group().expand {
            return None;
        }
        let arena = argument.executable_parts();
        let [component] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Variable {
            name: root,
            index: None,
        } = component.part
        else {
            return None;
        };
        let root = tcl_syntax::backslash::native_source_literal_bytes(
            arena.bytes(root)?,
            arena.image().channel(),
            topology.source_string_protocol(),
        )
        .ok()?;
        if root.as_ref() != name.as_bytes() {
            return None;
        }
        trace_alpha("quiet-formal-read", body_site.offset);
        let binding = SourceInvocationBinding {
            dispatch_site: Some(body_site.clone()),
            declaration_layout_observations: Some(vec![row.clone()].into()),
            lookup_state: Some(row.snapshot.clone()),
            lookup_namespace_key: row.namespace.clone(),
            variable_context: std::sync::Arc::clone(&state.source_variables),
            variable_frame: entry.frame().clone(),
            ..Default::default()
        };
        if !binding.original_operands_preserve_lookup(&tokens, registry, row, 0) {
            return None;
        }
        let head = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            head,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            symbol.policy(),
        )?;
        trace_alpha("selected-return-target", body_site.offset);
        let target = binding.original_registry_target_for_name(&head)?;
        let final_state = self.original_completed_root_state.as_ref()?;
        if target.kind != super::BindingKind::Builtin
            || !final_state.retained_target_is_current(&target)
            || state.source_execution_observed(target.identity.as_ref())
            || state.runtime_execution_observed(target.identity.as_ref())
        {
            return None;
        }
        validate_alpha_return_contract(body_site, &tokens, row, &target, config, registry)?;
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    fn receipt(
        source: &str,
        profile: &str,
        proposed: &[u8],
    ) -> Option<super::OriginalScalarBodyAlphaRename> {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile);
        let declaration = analysis.original_procedure_declarations().next()?;
        let owner = tcl_registry::model::ingress::static_context_for(profile);
        analysis.original_scalar_body_alpha_rename(declaration, proposed, owner.commands())
    }

    #[test]
    // Implementation contract: naming.variable.scalar-formal-body-alpha-equivalence
    // docs/design/analysis/name-resolution-proofs/scalar-formal-body-alpha-equivalence.md
    fn original_scalar_body_alpha_preserves_frame_argv_and_whole_object_return() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source = "proc p {original} {return $original}";
            let proof = receipt(source, profile, b"x")
                .unwrap_or_else(|| panic!("closed scalar body proof missing for {profile}"));
            assert_eq!(proof.new_native_tail(), b"x");
            assert_eq!(
                proof.selected_symbol().rename_tail(),
                Some(b"original".as_slice())
            );
            let config = proof.config;
            assert!(proof.matches_source(&tcl_lexer::SourceImage::document(source), config));
            assert!(!proof.matches_source(
                &tcl_lexer::SourceImage::document(&format!("{source}\nlist changed")),
                config,
            ));
            assert!(
                !proof.matches_source(&tcl_lexer::SourceImage::native(source.as_bytes()), config)
            );
            let mut other = config;
            other.strict_quoting = !other.strict_quoting;
            assert!(!proof.matches_source(&tcl_lexer::SourceImage::document(source), other));
            let owner = tcl_registry::model::ingress::static_context_for(profile);
            assert!(proof.matches_registry(owner.commands()));
            assert_eq!(proof.entry.frame(), match proof.selected_symbol().slot() {
                crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local { frame, .. } => frame.frame(),
                _ => panic!("alpha proof must retain the actual original local frame"),
            });
        }
    }

    #[test]
    // Native proof: naming.formal.raw-zero-list-and-binding-renaming
    // docs/design/analysis/name-resolution-proofs/formal-raw-zero-list-and-binding-renaming.md
    fn original_scalar_alpha_raw_zero_requires_selected_formal_and_body_correspondence() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap();
            let dialect = tcl_registry::InvocationDialect::of_point(point);
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let source = "proc p {long\0tail} {return ${long\0tail}}";
            for image in [
                tcl_lexer::SourceImage::native(source.as_bytes()),
                tcl_lexer::SourceImage::document(source),
            ] {
                let plan = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                    config,
                )
                .unwrap();
                let words = &plan.commands[0].words;
                let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(&words[2],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config), dialect.authored_name_policy().unwrap()).unwrap();
                let topology =
                    super::super::formal_topology::OriginalFormalTopology::from_original_key(
                        key, dialect,
                    )
                    .unwrap();
                let body = tcl_lexer::native_script_words_in(
                    image.clone(),
                    words[3].content_span().unwrap(),
                    config,
                )
                .unwrap();
                let argument = &body.commands[0].words[1];
                let arena = argument.executable_parts();
                let [part] = arena.list(arena.root()) else {
                    panic!("authentic sole formal read");
                };
                let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(argument,
                    part.span, tcl_syntax::word_rules::WordValueRules::from_config(&config), dialect.authored_name_policy().unwrap()).unwrap();
                let matches = topology.fixed_scalar_binding_names(1).is_some_and(|names| {
                    names
                        .as_slice()
                        .first()
                        .is_some_and(|name| name.as_bytes() == root.bytes())
                });
                let native_old = matches!(profile, "tcl8.4" | "tcl8.5")
                    && image.channel() == tcl_lexer::SourceChannel::NativeValue;
                assert_eq!(matches, !native_old, "{profile}: {:?}", image.channel());
                if image.channel() == tcl_lexer::SourceChannel::Document {
                    assert_eq!(
                        root.bytes(),
                        if profile == "jimtcl" {
                            b"long\0tail".as_slice()
                        } else {
                            b"long\xc0\x80tail".as_slice()
                        },
                        "{profile}: Document zero source units"
                    );
                } else {
                    assert_eq!(root.bytes(), b"long\0tail");
                }
            }
        }
    }

    #[test]
    // Implementation contract: naming.variable.scalar-formal-body-alpha-equivalence
    // docs/design/analysis/name-resolution-proofs/scalar-formal-body-alpha-equivalence.md
    fn original_scalar_body_alpha_declines_observers_introspection_and_changed_topology() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            for source in [
                "proc p {original} {return [info locals]}",
                "proc p {original} {return prefix$original}",
                "proc p {original} {return $original(k)}",
                "proc p {original} {set other $original; return $other}",
                "proc p {{original DEFAULT}} {return $original}",
                "proc p {args} {return $args}",
                "proc p {original other} {return $original}",
                "proc p {original} {return $original}\ninfo args p",
                "proc p {original} {return $original}\ntrace add execution p enter callback",
                "rename return gone\nproc p {original} {return $original}",
            ] {
                assert!(
                    receipt(source, profile, b"x").is_none(),
                    "{profile}: {source}"
                );
            }
            for proposed in [b"::qualified".as_slice(), b"x(k)", b"", b"x::y", b"args"] {
                assert!(
                    receipt("proc p {original} {return $original}", profile, proposed).is_none()
                );
            }
        }
        assert!(receipt("proc p {&original} {return $original}", "jimtcl", b"x").is_none());
        assert!(receipt("proc p {original} {return $original}", "jimtcl", b"&x").is_none());
    }
}
