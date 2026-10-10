// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-written hosted barriers remain separate from native publications.

use crate::signature_scan::vendor_name::VendorSourceNameInput;
use std::collections::BTreeSet;
use tcl_core_types::NameBytes;
use tcl_lexer::NativeWord;
use tcl_registry::model::ContextRegistry;
use tcl_registry::{CommandBindingTransition, StateTransitionDomain, TransitionSubject};
use tcl_syntax::naming::VendorSourceNamePurpose;

/// Authored root-script transition evidence before one original hosted head.
/// These records can withdraw nominal source advice. They supply no command
/// existence, runtime table, Native name recipe, successful commit or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSourceCatalogueBarriers {
    head: VendorSourceNameInput,
    context: tcl_registry::model::ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    original: Vec<Vec<NativeWord>>,
    schemas: Vec<super::VendorRegistryInvocationShape>,
    unknown_transitions: bool,
}
impl VendorSourceCatalogueBarriers {
    /// Actual original head whose authored applicability was checked.
    #[must_use]
    pub const fn original_head(&self) -> &VendorSourceNameInput {
        &self.head
    }
    /// Complete original root commands preceding the selected head.
    /// Bodies are never treated as executed merely because they were written.
    #[must_use]
    pub fn original_commands(&self) -> &[Vec<NativeWord>] {
        &self.original
    }
    /// Independently selected source descriptors at the retained prior vectors.
    #[must_use]
    pub fn source_schemas(&self) -> &[super::VendorRegistryInvocationShape] {
        &self.schemas
    }
    /// Unknown mutators preserve alternative applicability, rather than making
    /// a definite source barrier or supplying a quiet-world proof.
    #[must_use]
    pub const fn has_unknown_transitions(&self) -> bool {
        self.unknown_transitions
    }
    /// Same complete source image, channel and semantic lexer configuration.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.head.matches_source(image, config)
            && self
                .original
                .iter()
                .flatten()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// Same retained Registry store and complete availability context.
    #[must_use]
    pub fn matches_context(&self, context: &ContextRegistry) -> bool {
        self.registry == context.commands().snapshot().semantic_key()
            && &self.context == context.context()
    }
}

/// Readonly authored-source ordering; neither runtime transfer nor a lookup.
pub(super) fn capture(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
) -> Option<VendorSourceCatalogueBarriers> {
    let region = tcl_lexer::Span::new(0, u32::try_from(head.source_image().bytes().len()).ok()?);
    capture_region(context, head, selected_prefixes, region)
}

/// The same authored barrier classifier over sealed original script ancestry.
/// Parent source bodies supply regions only; no entered frame or table follows.
pub(super) fn capture_in_script_body(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
    body: &super::OriginalSourceScriptBodyOrigin,
) -> Option<VendorSourceCatalogueBarriers> {
    let mut receipt = capture(context, head, selected_prefixes)?;
    for region in body.source_regions() {
        let local = capture_region(context, head, selected_prefixes, *region)?;
        receipt.original.extend(local.original);
        receipt.schemas.extend(local.schemas);
        receipt.unknown_transitions |= local.unknown_transitions;
    }
    Some(receipt)
}

fn capture_region(
    context: &ContextRegistry,
    head: &VendorSourceNameInput,
    selected_prefixes: &[Vec<NameBytes>],
    region: tcl_lexer::Span,
) -> Option<VendorSourceCatalogueBarriers> {
    let image = head.source_image();
    let config = head.lexer_config();
    let plan = tcl_lexer::native_script_words_in(image.clone(), region, config).ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let mut receipt = VendorSourceCatalogueBarriers {
        head: head.clone(),
        context: context.context().clone(),
        registry: context.commands().snapshot().semantic_key(),
        original: Vec::new(),
        schemas: Vec::new(),
        unknown_transitions: false,
    };
    let mut barriers = BTreeSet::new();
    for command in plan.commands {
        let Some(first) = command.words.first() else {
            continue;
        };
        let Some(last) = command.words.last() else {
            continue;
        };
        // A body containing this head is not a preceding source operation.
        if last.span().end() > head.span().start() {
            break;
        }
        let input = VendorSourceNameInput::from_original_word(first, head.policy());
        receipt.original.push(command.words.clone());
        let root_paths = input
            .literal_units(VendorSourceNamePurpose::CommandHead)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|name| {
                tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                    "::",
                    &[] as &[&str],
                    name,
                )
            });
        if root_paths.as_ref().is_some_and(|paths| {
            paths
                .iter()
                .any(|name| barriers.contains(&NameBytes::from(name.as_bytes())))
        }) {
            // A custom preceding handler may itself mutate arbitrary names.
            receipt.unknown_transitions = true;
            barriers.clear();
            continue;
        }
        let Some(shape) = super::vendor_registry_invocation_shape(context, &input, &command.words)
        else {
            receipt.unknown_transitions = true;
            barriers.clear();
            continue;
        };
        let transitions = shape.source_transitions();
        let deferred = shape
            .possible_traits()
            .contains(tcl_registry::Traits::DEFERS_BODY);
        let immediate_body = shape
            .roles()
            .iter()
            .any(|(_, role)| matches!(role, tcl_registry::ArgRole::Body))
            && !deferred;
        if immediate_body
            || transitions.is_none_or(|transitions| {
                transitions.widens(StateTransitionDomain::CommandBindings)
                    || transitions.widens(StateTransitionDomain::CommandResolution)
            })
        {
            receipt.unknown_transitions = true;
            barriers.clear();
        } else if !receipt.unknown_transitions
            && let Some(transitions) = transitions
        {
            for transition in transitions.command_bindings() {
                if apply_source_transition(&mut barriers, &shape, transition).is_none() {
                    receipt.unknown_transitions = true;
                    barriers.clear();
                    break;
                }
            }
        }
        receipt.schemas.push(shape);
    }
    if selected_prefixes
        .iter()
        .any(|path| path.iter().any(|coordinate| barriers.contains(coordinate)))
    {
        return None;
    }
    Some(receipt)
}

fn source_subject<'a>(
    shape: &'a super::VendorRegistryInvocationShape,
    subject: &TransitionSubject,
) -> Option<&'a [u8]> {
    let ordinal = subject.argument_index()?.checked_add(1)?;
    let value = tcl_syntax::naming::vendor_source_literal_units(
        shape.original_head().policy(),
        shape.original_words().get(ordinal)?,
        VendorSourceNamePurpose::SourceName,
    )?;
    let selected = subject
        .literal()
        .map(str::as_bytes)
        .or_else(|| subject.native_bytes())?;
    (selected == value).then_some(value)
}
fn source_coordinate(
    shape: &super::VendorRegistryInvocationShape,
    subject: &TransitionSubject,
) -> Option<NameBytes> {
    Some(NameBytes::from(tcl_syntax::naming::qualify_bytes(
        b"::",
        source_subject(shape, subject)?,
    )))
}
fn apply_source_transition(
    barriers: &mut BTreeSet<NameBytes>,
    shape: &super::VendorRegistryInvocationShape,
    transition: &CommandBindingTransition,
) -> Option<()> {
    match transition {
        CommandBindingTransition::Define { name, .. } => {
            barriers.insert(source_coordinate(shape, name)?);
        }
        CommandBindingTransition::Move { from, to } => {
            barriers.insert(source_coordinate(shape, from)?);
            if !source_subject(shape, to)?.is_empty() {
                barriers.insert(source_coordinate(shape, to)?);
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            if interpreter.as_ref().is_some_and(|interpreter| {
                source_subject(shape, interpreter).is_none_or(|bytes| !bytes.is_empty())
            }) {
                return None;
            }
            barriers.insert(source_coordinate(shape, name)?);
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            ..
        } => {
            if !source_subject(shape, source_interpreter)?.is_empty() {
                return None;
            }
            barriers.insert(source_coordinate(shape, alias)?);
        }
        CommandBindingTransition::Unknown { .. } => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    fn candidate(
        source: &str,
        marker: &str,
    ) -> Option<super::super::OriginalConditionalVendorRegistryMetadata> {
        candidate_in(source, marker, "f5-irules")
    }
    fn candidate_in(
        source: &str,
        marker: &str,
        environment: &str,
    ) -> Option<super::super::OriginalConditionalVendorRegistryMetadata> {
        let analysis = crate::analyser::Analyser::new().analyse(source, environment);
        super::super::source_structure::selected_vendor_registry_words_at(
            source,
            &analysis,
            u32::try_from(source.rfind(marker).unwrap()).unwrap(),
        )
        .map(|(metadata, _)| metadata)
    }
    #[test]
    fn original_hosted_source_barriers_keep_declarations_moves_and_unknowns_separate() {
        // naming.compiler.conditional-registry-source-metadata
        // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
        for (source, marker) in [
            (
                "proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}",
                "call Lib",
            ),
            (
                "proc pool args {}\nwhen HTTP_REQUEST {pool selected}",
                "pool selected",
            ),
            ("proc when args {}\nwhen HTTP_REQUEST {}", "when HTTP"),
            (
                "proc HTTP::respond args {return custom}\nwhen HTTP_REQUEST {HTTP::respond 200}",
                "HTTP::respond 200",
            ),
        ] {
            assert!(candidate(source, marker).is_none(), "{source}");
        }
        // The iApp context independently admits this transition descriptor.
        assert!(
            candidate_in(
                "rename source moved; source selected.tcl",
                "source selected",
                "f5-iapps",
            )
            .is_none()
        );
        assert!(
            candidate_in(
                "rename source {}; source selected.tcl",
                "source selected",
                "f5-iapps",
            )
            .is_none()
        );
        // TMM excludes rename: its spelling supplies no selected transition.
        let unavailable = candidate(
            "rename pool moved\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(unavailable.authored_barriers().has_unknown_transitions());
        let known = candidate("when HTTP_REQUEST {pool selected}", "pool selected").unwrap();
        assert!(!known.authored_barriers().has_unknown_transitions());
        let unknown = candidate(
            "rename $which moved\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(unknown.authored_barriers().has_unknown_transitions());
        // A declaration written inside a deferred procedure has not run as a root statement.
        assert!(
            candidate(
                "proc define {} {proc pool args {}}\nwhen HTTP_REQUEST {pool selected}",
                "pool selected"
            )
            .is_some()
        );
        // A possible arbitrary mutation withdraws definite source-written shadowing.
        let uncertain = candidate(
            "proc pool args {}\nunknown_mutator\nwhen HTTP_REQUEST {pool selected}",
            "pool selected",
        )
        .unwrap();
        assert!(uncertain.authored_barriers().has_unknown_transitions());
        let image = tcl_lexer::SourceImage::document("when HTTP_REQUEST {pool selected}");
        let config = known.authored_barriers().original_head().lexer_config();
        assert!(known.authored_barriers().matches_source(&image, config));
        assert!(!known.authored_barriers().matches_source(
            &tcl_lexer::SourceImage::document("when CLIENT_DATA {}"),
            config
        ));
        assert!(known.authored_barriers().matches_context(
            tcl_registry::model::ingress::static_context_for("f5-irules")
        ));
        assert!(
            !known
                .authored_barriers()
                .matches_context(tcl_registry::model::ingress::static_context_for("f5-iapps"))
        );
    }
}
